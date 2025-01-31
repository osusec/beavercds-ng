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
use crate::clients::docker;
use crate::configparser::challenge::{ChallengeConfig, ProvideConfig};

/// extract assets from provide config and possible container to challenge directory, return file path(s) extracted
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn extract_asset(
    chal: &ChallengeConfig,
    provide: &ProvideConfig,
    // pod_containers:
) -> Result<Vec<PathBuf>> {
    // This needs to handle three cases * 2 sources:
    //   - single or multiple files without renaming (no as: field)
    //   - single file with rename (one item with as:)
    //   - multiple files as archive (multiple items with as:)
    // and whether the file is coming from
    //   - the repo
    //   - or a container

    // TODO: since this puts artifacts in the repo source folder, this should
    // try to not overwrite any existing files.

    // debug!("extracting assets from container {}", &container.name);

    let docker = docker().await?;

    match provide {
        // No action necessary, return path as-is
        ProvideConfig::FromRepo { files } => Ok(files.clone()),
        ProvideConfig::FromRepoRename { from, to } => {
            std::fs::rename(from, to)?;
            Ok(vec![to.clone()])
        }
        ProvideConfig::FromRepoArchive {
            files,
            archive_name,
        } => {
            zip_files(archive_name, files)?;
            Ok(vec![archive_name.clone()])
        }

        ProvideConfig::FromContainer { container, files } => extract_files(chal, container, files),
        ProvideConfig::FromContainerRename {
            container,
            from,
            to,
        } => extract_rename(chal, container, from, to),
        ProvideConfig::FromContainerArchive {
            container,
            files,
            archive_name,
        } => extract_archive(chal, container, files, archive_name),
    }
}

/// Extract multiple files from container
async fn extract_files(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    files: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    debug!(
        "extracting {} files without renaming: {:?}",
        files.len(),
        files
    );

    try_join_all(files.iter().map(|from| async {
        // use basename of source file as target name
        let to = chal.directory.join(from.file_name().unwrap());
        docker::copy_file(container, from, &to).await
    }))
    .await
}

/// Extract one file from container and rename
async fn extract_rename(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    file: &Path,
    new_name: &Path,
) -> Result<Vec<PathBuf>> {
    debug!("extracting file {:?} renamed to {:?}", file, new_name);

    let new_file = docker::copy_file(container, file, new_name).await?;

    Ok(vec![new_file])
}

/// Extract one or more file from container as archive
async fn extract_archive(
    chal: &ChallengeConfig,
    container: &docker::ContainerInfo,
    // files: &Vec<PathBuf>,
    files: &[PathBuf],
    archive_name: &Path,
) -> Result<Vec<PathBuf>> {
    debug!(
        "extracting {} files {:?} into archive {:?}",
        files.len(),
        files,
        archive_name
    );

    // copy all listed files to tempdir
    let tempdir = tempdir_in(".")?;
    let copied_files = try_join_all(files.iter().map(|from| async {
        let to = tempdir.path().join(from.file_name().unwrap());
        docker::copy_file(container, from, &to).await
    }))
    .await?;

    zip_files(&chal.directory.join(archive_name), &copied_files)?;

    Ok(vec![chal.directory.join(archive_name)])
}

/// Add multiple local `files` to a zipfile at `zip_name`
pub fn zip_files(archive_name: &Path, files: &[PathBuf]) -> Result<PathBuf> {
    debug!("creating zip at {:?}", archive_name);
    let zipfile = File::create(archive_name)?;
    let mut z = zip::ZipWriter::new(zipfile);
    let opts = zip::write::SimpleFileOptions::default();

    let mut buf = vec![];
    for path in files.iter() {
        trace!("adding {:?} to zip", path);
        // TODO: dont read entire file into memory
        File::open(path)?.read_to_end(&mut buf)?;
        // TODO: should this always do basename? some chals might need specific
        // file structure but including dirs should work fine
        z.start_file(path.file_name().unwrap().to_string_lossy(), opts)?;
        z.write_all(&buf)?;
        buf.clear();
    }

    z.finish();

    Ok(archive_name.to_path_buf())
}
