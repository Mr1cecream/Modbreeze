use crate::{
    errors::BreezeError,
    structs::{Mod, ModId, ModSide, Pack},
};
use anyhow::Result;
use async_recursion::async_recursion;
use ferinth::{structures::version::DependencyType, Ferinth};
use fs_extra::file::{move_file, CopyOptions as FileCopyOptions};
use furse::{structures::file_structs::FileRelationType, Furse};
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
use libium::{
    config::structs::ModLoader,
    upgrade::{mod_downloadable, Downloadable},
};
use log::{error, info};
use rayon::prelude::*;
use reqwest::Client;
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    fs::{create_dir_all, remove_file},
    spawn,
    sync::{RwLock, Semaphore},
    task::JoinSet,
};

/// Get the `Downloadable`s for the mods in a `Pack`
/// Returns a `Vec` of the `Downloadable`s
pub async fn get_downloadables(
    side: ModSide,
    resourcepacks: bool,
    shaderpacks: bool,
    pack: Pack,
) -> Result<Vec<Downloadable>> {
    let api_key = env!("CF_API_KEY");
    let furse = Furse::new(api_key);
    let ferinth = Ferinth::new(
        "modbreeze",
        option_env!("CARGO_PKG_VERSION"),
        Some("Mr. Icecream#9624"),
        None,
    )?;
    let mods = if side == ModSide::All {
        pack.mods
    } else {
        pack.mods
            .into_par_iter()
            .filter(|mod_| mod_.side == side || mod_.side == ModSide::All)
            .collect()
    };

    #[async_recursion(?Send)]
    async fn inner(
        mods: Vec<Mod>,
        furse: &Furse,
        ferinth: &Ferinth,
        mc_version: String,
        loader: ModLoader,
        to_download: Arc<RwLock<Vec<Downloadable>>>,
        output: Arc<String>,
    ) -> Result<()> {
        let dependencies: Arc<Mutex<Vec<Mod>>> = Arc::new(Mutex::new(Vec::new()));
        let mut tasks = JoinSet::new();
        let semaphore = Arc::new(Semaphore::new(75));
        for mod_ in mods.iter() {
            let permit = semaphore.clone().acquire_owned().await?;
            let furse = furse.clone();
            let ferinth = ferinth.clone();
            let mc_version = mc_version.clone();
            let loader = loader.clone();
            let to_download = to_download.clone();
            let output = output.clone();
            let mod_ = mod_.clone();
            let dependencies = dependencies.clone();
            tasks.spawn(async move {
                let mc_version_to_check = if mod_.ignore_version {
                    None
                } else {
                    Some(mc_version)
                };
                let loader_to_check = if mod_.ignore_loader {
                    None
                } else {
                    Some(loader)
                };
                let _permit = permit;
                let downloadable = match mod_.id.clone() {
                    ModId::CurseForgeId(id) => match mod_downloadable::get_latest_compatible_file(
                        furse.get_mod_files(id.try_into()?).await?,
                        mc_version_to_check.as_deref(),
                        loader_to_check.as_ref(),
                    ) {
                        None => Err(BreezeError::NoCompatFile(
                            mod_.name.clone(),
                            mod_.id.clone(),
                        )),
                        Some(ok) => {
                            info!("Got file for mod {}, id: {}", mod_.name, mod_.id);
                            let dependencies_: Vec<Mod> = ok
                                .0
                                .dependencies
                                .into_iter()
                                .filter(|d| d.relation_type == FileRelationType::RequiredDependency)
                                .map(|d| Mod {
                                    name: format!("Dependency of {}", &mod_.name),
                                    id: ModId::CurseForgeId(d.mod_id as u32),
                                    ignore_loader: mod_.ignore_loader,
                                    ignore_version: mod_.ignore_version,
                                    side: mod_.side, // doesn't matter in this situation
                                })
                                .collect();
                            dependencies_.into_iter().for_each(|d| {
                                let mut dependencies = dependencies.lock().expect("Mutex poisoned");
                                if !dependencies.contains(&d) {
                                    info!("Adding dependency: {}, id: {}", d.name, d.id);
                                    dependencies.push(d);
                                }
                            });

                            Ok(Downloadable {
                                download_url: ok.0.download_url.ok_or(
                                    BreezeError::DistributionDenied(
                                        mod_.name.clone(),
                                        mod_.id.clone(),
                                    ),
                                )?,
                                output: PathBuf::from(if ok.0.file_name.ends_with(".jar") {
                                    "mods"
                                } else {
                                    &output
                                })
                                .join(ok.0.file_name),
                                length: ok.0.file_length as u64,
                            })
                        }
                    },
                    ModId::ModrinthId(id) => match mod_downloadable::get_latest_compatible_version(
                        &ferinth.list_versions(&id.to_string()).await?,
                        mc_version_to_check.as_deref(),
                        loader_to_check.as_ref(),
                    ) {
                        None => Err(BreezeError::NoCompatFile(
                            mod_.name.clone(),
                            mod_.id.clone(),
                        )),
                        Some(ok) => {
                            info!("Got version file for mod {}, id: {}", mod_.name, mod_.id);
                            for d in ok.1.dependencies.into_iter().filter(|d| {
                                d.dependency_type == DependencyType::Required
                                    && (d.project_id.is_some() || d.version_id.is_some())
                            }) {
                                let project_id = match d.project_id {
                                    Some(project_id) => project_id,
                                    None => {
                                        if let Ok(ok) =
                                            ferinth.get_version(&d.version_id.unwrap()).await
                                        {
                                            ok.project_id
                                        } else {
                                            continue;
                                        }
                                    }
                                };
                                let d = Mod {
                                    name: format!("Dependency of {}", &mod_.name),
                                    id: ModId::ModrinthId(project_id),
                                    ignore_loader: mod_.ignore_loader,
                                    ignore_version: mod_.ignore_version,
                                    side: mod_.side, // doesn't matter in this situation
                                };
                                let mut dependencies = dependencies.lock().expect("Mutex poisoned");
                                if !dependencies.contains(&d) {
                                    info!("Adding dependency: {}, id: {}", d.name, d.id);
                                    dependencies.push(d);
                                }
                            }

                            Ok(Downloadable {
                                download_url: ok.0.url,
                                output: PathBuf::from(if ok.0.filename.ends_with(".jar") {
                                    "mods"
                                } else {
                                    &output
                                })
                                .join(ok.0.filename),
                                length: ok.0.size as u64,
                            })
                        }
                    },
                };
                match downloadable {
                    Ok(ok) => to_download.write().await.push(ok),
                    Err(err) => error!("{}", err),
                }
                Ok::<(), anyhow::Error>(())
            });
        }
        while let Some(res) = tasks.join_next().await {
            let _res = res??;
        }
        let dependencies = dependencies.lock().expect("Mutex poisoned");
        if !(dependencies.is_empty()) {
            inner(
                dependencies.clone(),
                furse,
                ferinth,
                mc_version,
                loader,
                to_download,
                output,
            )
            .await?;
        }
        Ok(())
    }
    let to_download = Arc::new(RwLock::new(Vec::new()));
    let mut futures = Vec::new();
    let mc_version = pack.mc_version;
    let loader = pack.loader;
    futures.push(inner(
        mods,
        &furse,
        &ferinth,
        mc_version.clone(),
        loader.clone(),
        to_download.clone(),
        Arc::new(String::from("mods")),
    ));
    if resourcepacks {
        futures.push(inner(
            pack.resourcepacks,
            &furse,
            &ferinth,
            mc_version.clone(),
            loader.clone(),
            to_download.clone(),
            Arc::new(String::from("resourcepacks")),
        ));
    }
    if shaderpacks {
        futures.push(inner(
            pack.shaderpacks,
            &furse,
            &ferinth,
            mc_version.clone(),
            loader.clone(),
            to_download.clone(),
            Arc::new(String::from("shaderpacks")),
        ));
    }
    for res in futures::future::join_all(futures).await {
        res?
    }
    Ok(Arc::try_unwrap(to_download)
        .map_err(|_| anyhow::anyhow!("Failed to run threads to completion"))?
        .into_inner())
}

pub async fn download(output_dir: Arc<PathBuf>, to_download: Vec<Downloadable>) -> Result<()> {
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
    let client = Arc::new(Client::new());
    for downloadable in to_download {
        let permit = semaphore.clone().acquire_owned().await?;
        let output_dir = output_dir.clone();
        let progress_bar = progress_bar.clone();
        let folder = &downloadable.output.parent();
        let client = client.clone();
        if let Some(folder) = folder {
            create_dir_all(folder).await?;
        }
        tasks.push(spawn(async move {
            let _permit = permit;
            downloadable
                .download(
                    &client,
                    &output_dir,
                    |addition| progress_bar.inc(addition.try_into().unwrap()), // increase progress on download update
                )
                .await?;
            Ok::<(), anyhow::Error>(())
        }));
    }
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
        total += downloadable.length;
    }
    total
}

/// Check the `directory`
/// If there are files that are not in `to_download`, they will be removed
/// If a file in `to_download` is already there, it will be removed from the Vec
/// If a file is a `.part` file or the move failed, the file will be deleted
pub async fn clean(
    directory: &Path,
    to_download: &mut Vec<Downloadable>,
    remove: bool,
) -> Result<()> {
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
                || (remove
                    && move_file(
                        file.path(),
                        directory.join(".old").join(filename),
                        &FileCopyOptions::new(),
                    )
                    .is_err())
            {
                remove_file(file.path()).await?;
            }
        }
    }
    Ok(())
}

/// Find duplicates of the items in `slice` using a value obtained by the `key` closure
/// Returns the indices of duplicate items in reverse order for easy removal
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
