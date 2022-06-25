use crate::{errors::BreezeError, toml::Pack, Mod, ModSide};
use anyhow::Result;
use fs_extra::file::{move_file, CopyOptions as FileCopyOptions};
use furse::Furse;
use itertools::Itertools;
use libium::upgrade::{mod_downloadable, Downloadable};
use log::warn;
use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    sync::Arc,
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
    let mods: Vec<&Mod> = pack
        .mods
        .iter()
        .filter(|mod_| {
            if side == ModSide::All {
                true
            } else {
                mod_.side == side || mod_.side == ModSide::All
            }
        })
        .collect();
    let mut to_download: Vec<Downloadable> = Vec::new();
    for mod_ in mods.iter() {
        let files = furse.get_mod_files(mod_.id.try_into()?).await?;
        let downloadable: Downloadable = mod_downloadable::get_latest_compatible_file(
            files,
            &pack.mc_version,
            &pack.loader,
            Some(true),
            Some(true),
        )
        .map_or_else(
            || Err(BreezeError::NoCompatFile),
            |ok| {
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
        )?;
        to_download.push(downloadable);
    }
    Ok(to_download)
}

pub async fn download(output_dir: Arc<PathBuf>, to_download: Vec<Downloadable>) -> Result<()> {
    create_dir_all(&*output_dir).await?;
    let mut tasks = Vec::new();
    let semaphore = Arc::new(Semaphore::new(75));
    for downloadable in to_download {
        let permit = semaphore.clone().acquire_owned().await?;
        let output_dir = output_dir.clone();
        tasks.push(spawn(async move {
            let _permit = permit;
            downloadable.download(&output_dir, |_x| {}, |_x| {}).await?;
            Ok::<(), anyhow::Error>(())
        }));
    }
    for handle in tasks {
        handle.await??;
    }
    Ok(())
}

/// Check the `directory`
/// If there are files that are not in `to_download`, they will be removed
/// If a file in `to_download` is already there, it will be removed from the Vec
/// If a file is a `.part` file or the move failed, the file will be deleted
async fn clean(directory: &Path, to_download: &mut Vec<Downloadable>) -> Result<()> {
    let dupes = find_dupes_by_key(to_download, Downloadable::filename);
    if !dupes.is_empty() {
        warn!(
            "{} duplicate files were found: {}",
            dupes.len(),
            dupes
                .into_iter()
                .map(|i| to_download.swap_remove(i).filename())
                .format(", ")
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
