use anyhow::{anyhow, Context, Error, Result};
use futures::future::try_join_all;
use futures::FutureExt;
use itertools::Itertools;
use simplelog::{debug, trace};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::tempdir_in;
use zip;

use crate::builder::docker;
use crate::configparser::challenge::{ChallengeConfig, ProvideConfig};

/// extract assets from given container name and provide config to challenge directory, return file path(s) extracted
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn extract_asset(
    chal: &ChallengeConfig,
    provide: &ProvideConfig,
    container: &docker::ContainerInfo,
) -> Result<Vec<PathBuf>> {
    debug!("extracting assets from container {}", &container.name);
    // This needs to handle three cases:
    // - single or multiple files without renaming (no as: field)
    // - single file with rename (one item with as:)
    // - multiple files as archive (multiple items with as:)

    // TODO: since this puts artifacts in the repo source folder, this should
    // try to not overwrite any existing files.

    match &provide.as_file {
        // no renaming, copy out all as-is
        None => extract_files(chal, container, &provide.include).await,
        // (as is keyword, so add underscore)
        Some(as_) => {
            if provide.include.len() == 1 {
                // single file, rename
                extract_rename(chal, container, &provide.include[0], as_).await
            } else {
                // multiple files, zip as archive
                extract_archive(chal, container, &provide.include, as_).await
            }
        }
    }
}

/// Extract multiple files from container
async fn extract_files(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    files: &Vec<String>,
) -> Result<Vec<PathBuf>> {
    debug!(
        "extracting {} files without renaming: {:?}",
        files.len(),
        files
    );

    try_join_all(files.iter().map(|f| {
        let from = PathBuf::from(f);
        // use basename of source file as target name
        let to = chal.directory.join(from.file_name().unwrap());

        docker::copy_file(container, from, to)
    }))
    .await
}

/// Extract one file from container and rename
async fn extract_rename(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    file: &str,
    new_name: &str,
) -> Result<Vec<PathBuf>> {
    debug!("extracting file {:?} renamed to {:?}", file, new_name);

    let new_file =
        docker::copy_file(container, PathBuf::from(file), PathBuf::from(new_name)).await?;

    Ok(vec![new_file])
}

/// Extract one or more file from container as archive
async fn extract_archive(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    files: &Vec<String>,
    archive_name: &str,
) -> Result<Vec<PathBuf>> {
    debug!(
        "extracting {} files {:?} into archive {:?}",
        files.len(),
        files,
        archive_name
    );

    // copy all listed files to tempdir
    let tempdir = tempdir_in(".")?;
    let copied_files = try_join_all(files.iter().map(|f| {
        let from = PathBuf::from(f);
        let to = tempdir.path().join(from.file_name().unwrap());

        docker::copy_file(container, from, to)
    }))
    .await?;

    // write them all to new zip
    let zipfile = File::create(chal.directory.join(archive_name))?;
    let mut z = zip::ZipWriter::new(zipfile);
    let opts = zip::write::SimpleFileOptions::default();

    let mut buf = vec![];
    for path in copied_files.into_iter() {
        trace!("adding {:?} to zip", &path);
        File::open(&path)?.read_to_end(&mut buf)?;
        z.start_file(path.to_string_lossy(), opts)?;
        z.write_all(&buf)?;
        buf.clear();
    }

    z.finish();

    Ok(vec![chal.directory.join(archive_name)])
}
