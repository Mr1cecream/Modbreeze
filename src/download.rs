use crate::{errors::BreezeError, toml::Pack, Mod, ModSide};
use anyhow::Result;
use async_recursion::async_recursion;
use fs_extra::file::{move_file, CopyOptions as FileCopyOptions};
use furse::{
    structures::file_structs::{File, FileRelationType},
    Furse,
};
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use libium::{
    config::structs::ModLoader,
    upgrade::{mod_downloadable, Downloadable},
};
use log::{error, info};
use rayon::prelude::*;
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::{create_dir_all, remove_file},
    spawn,
    sync::Semaphore,
};

/// Get the `Downloadable`s for the mods in a `Pack`
/// Returns a `Vec` of the `Downloadable`s
pub async fn get_downloadables(side: ModSide, pack: Pack) -> Result<Vec<Downloadable>> {
    let api_key = env!("CF_API_KEY");
    let furse = Furse::new(api_key);
    let mods = if side == ModSide::All {
        pack.mods
    } else {
        pack.mods
            .into_par_iter()
            .filter(|mod_| mod_.side == side || mod_.side == ModSide::All)
            .collect()
    };

    let mut to_download: Vec<Downloadable> = Vec::new();
    #[async_recursion]
    async fn inner(
        mods: Vec<Mod>,
        furse: &Furse,
        mc_version: &str,
        loader: &ModLoader,
        to_download: &mut Vec<Downloadable>,
    ) -> Result<()> {
        let mut futures = Vec::new();
        for mod_ in mods.iter() {
            futures.push(furse.get_mod_files(mod_.id.try_into()?));
        }
        let files = futures::future::join_all(futures).await;
        files.par_iter().for_each(|files| match files {
            Err(err) => error!("{}", err),
            _ => (),
        });
        let files: Vec<Vec<File>> = files
            .into_iter()
            .filter(|files| files.is_ok())
            .map(|files| files.unwrap())
            .collect();
        let mut dependencies: Vec<Mod> = Vec::new();
        for i in 0..mods.len() {
            let mod_ = &mods[i];
            let files = &files[i];
            let downloadable = mod_downloadable::get_latest_compatible_file(
                files.to_vec(),
                mc_version,
                loader,
                Some(!mod_.ignore_version),
                Some(!mod_.ignore_loader),
            )
            .map_or_else(
                || {
                    Err(BreezeError::NoCompatFile(
                        mod_.name.clone(),
                        mod_.id.clone(),
                    ))
                },
                |ok| {
                    let dependencies_: Vec<Mod> =
                        ok.0.dependencies
                            .into_iter()
                            .filter(|d| d.relation_type == FileRelationType::RequiredDependency)
                            .map(|d| Mod {
                                name: format!("Dependency of {}", &mod_.name),
                                id: d.mod_id as u32,
                                ignore_loader: false,
                                ignore_version: false,
                                side: ModSide::All,
                            })
                            .collect();
                    dependencies_.into_iter().for_each(|d| {
                        if !dependencies.contains(&d) {
                            dependencies.push(d);
                        }
                    });
                    Ok(Downloadable {
                        download_url: ok
                            .0
                            .download_url
                            .ok_or(BreezeError::DistributionDenied(mod_.name.clone(), mod_.id))?,
                        output: PathBuf::from(if ok.0.file_name.ends_with(".zip") {
                            "resourcepacks"
                        } else {
                            "mods"
                        })
                        .join(ok.0.file_name),
                        size: Some(ok.0.file_length as u64),
                    })
                },
            );
            match downloadable {
                Ok(ok) => to_download.push(ok),
                Err(err) => error!("{}", err),
            }
        }
        if !dependencies.is_empty() {
            inner(dependencies, furse, mc_version, loader, to_download).await?;
        }
        Ok(())
    }
    inner(
        mods,
        &furse,
        &pack.mc_version,
        &pack.loader,
        &mut to_download,
    )
    .await?;
    Ok(to_download)
}

pub async fn download(output_dir: Arc<PathBuf>, to_download: Vec<Downloadable>) -> Result<()> {
    create_dir_all(&*output_dir.join("mods")).await?;
    let mut tasks = Vec::new();
    let semaphore = Arc::new(Semaphore::new(75));
    let progress_bar = ProgressBar::new(count_bytes(&to_download)).with_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] ({bytes}/{total_bytes}) ({percent}%)",
        )?
        .progress_chars("=> ")
        .tick_strings(&["Downloading.  ", "Downloading.. ", "Downloading...", "Finished."])
    );
    progress_bar.enable_steady_tick(Duration::from_millis(300));
    for downloadable in to_download {
        let permit = semaphore.clone().acquire_owned().await?;
        let output_dir = output_dir.clone();
        let progress_bar = progress_bar.clone();
        tasks.push(spawn(async move {
            let _permit = permit;
            downloadable
                .download(
                    &output_dir,
                    |_| {},
                    |x| progress_bar.inc(x.try_into().unwrap()), // increase progress on download update
                )
                .await?;
            Ok::<(), anyhow::Error>(())
        }));
    }
    progress_bar.tick(); // tick progress bar to start drawing
    for handle in tasks {
        handle.await??;
    }
    progress_bar.finish();
    Ok(())
}

/// Count the total size in bytes of the downloadables
fn count_bytes(downloadables: &[Downloadable]) -> u64 {
    let mut total = 0_u64;
    for downloadable in downloadables {
        total += downloadable.size.unwrap_or(0);
    }
    total
}

/// Check the `directory`
/// If there are files that are not in `to_download`, they will be removed
/// If a file in `to_download` is already there, it will be removed from the Vec
/// If a file is a `.part` file or the move failed, the file will be deleted
pub async fn clean(directory: &Path, to_download: &mut Vec<Downloadable>) -> Result<()> {
    let dupes = find_dupes_by_key(to_download, Downloadable::filename);
    if !dupes.is_empty() {
        info!(
            "{}",
            format!(
                "{} duplicate files were found: {}",
                dupes.len(),
                dupes
                    .into_iter()
                    .map(|i| to_download.swap_remove(i).filename())
                    .format(", ")
            )
        );
    }
    create_dir_all(directory.join(".old")).await?;
    for file in read_dir(&directory)? {
        let file = file?;
        if file.file_type()?.is_file() {
            let filename = file.file_name();
            let filename = filename.to_str().unwrap();
            if let Some(index) = to_download
                .iter()
                .position(|thing| filename == thing.filename())
            {
                to_download.swap_remove(index);
            } else if filename.ends_with("part")
                || move_file(
                    file.path(),
                    directory.join(".old").join(filename),
                    &FileCopyOptions::new(),
                )
                .is_err()
            {
                remove_file(file.path()).await?;
            }
        }
    }
    Ok(())
}

/// Find duplicates of the items in `slice` using a value obtained by the `key` closure
/// Returns the indices of duplicate items in reverse order for easy removal
// Source: https://github.com/gorilla-devs/ferium
fn find_dupes_by_key<T, V, F>(slice: &mut [T], key: F) -> Vec<usize>
where
    V: Eq + Ord,
    F: Fn(&T) -> V,
{
    let mut indices = Vec::new();
    if slice.len() < 2 {
        return indices;
    }
    slice.sort_unstable_by_key(&key);
    for i in 0..(slice.len() - 1) {
        if key(&slice[i]) == key(&slice[i + 1]) {
            indices.push(i);
        }
    }
    indices.reverse();
    indices
}
