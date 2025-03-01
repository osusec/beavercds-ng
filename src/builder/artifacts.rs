use anyhow::{anyhow, Context, Error, Result};
use futures::FutureExt;
use itertools::Itertools;
use simplelog::{debug, trace};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::iter::repeat_with;
use std::path::{Path, PathBuf};
use tempfile::tempdir_in;
use zip;

use crate::builder::docker;
use crate::clients::docker;
use crate::configparser::challenge::{ChallengeConfig, ProvideConfig};
use crate::utils::TryJoinAll;

/// extract assets from provide config and possible container to challenge directory, return file path(s) extracted
pub async fn extract_asset(
    chal: &ChallengeConfig,
    provide: &ProvideConfig,
    profile_name: &str,
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

    debug!(
        "extracting assets for challenge {:?} provide {:?}",
        chal.directory, &provide
    );

    let docker = docker().await?;

    match provide {
        // Repo file paths are relative to the challenge directory, so prepend chal dir

        // No action necessary, return path as-is
        ProvideConfig::FromRepo { files } => {
            Ok(files.iter().map(|f| chal.directory.join(f)).collect_vec())
        }
        ProvideConfig::FromRepoRename { from, to } => {
            std::fs::copy(chal.directory.join(from), chal.directory.join(to))
                .with_context(|| format!("could not copy repo file {from:?} to {to:?}"))?;
            Ok(vec![to.clone()])
        }
        ProvideConfig::FromRepoArchive {
            files,
            archive_name,
        } => {
            zip_files(
                &chal.directory.join(archive_name),
                &files.iter().map(|f| chal.directory.join(f)).collect_vec(),
            )
            .with_context(|| format!("could not create archive {archive_name:?}"))?;
            Ok(vec![archive_name.clone()])
        }

        // handle all container events together to manage container, then match again
        ProvideConfig::FromContainer {
            container: container_name,
            ..
        }
        | ProvideConfig::FromContainerRename {
            container: container_name,
            ..
        }
        | ProvideConfig::FromContainerArchive {
            container: container_name,
            ..
        } => {
            let tag = chal.container_tag_for_pod(profile_name, container_name)?;

            let name = format!(
                "asset-container-{}-{}-{}",
                chal.slugify(),
                container_name,
                // include random discriminator to avoid name collisions
                repeat_with(fastrand::alphanumeric)
                    .take(6)
                    .collect::<String>()
            );

            let container = docker::create_container(&tag, &name).await?;

            // match on `provide` enum again to handle each container type
            let files = match provide {
                ProvideConfig::FromContainer {
                    container: container_name,
                    files,
                } => extract_files(chal, &container, files)
                    .await
                    .with_context(|| {
                        format!("could not copy files {files:?} from container {container_name}")
                    }),

                ProvideConfig::FromContainerRename {
                    container: container_name,
                    from,
                    to,
                } => extract_rename(chal, &container, from, &chal.directory.join(to))
                    .await
                    .with_context(|| {
                        format!("could not copy file {from:?} from container {container_name}")
                    }),

                ProvideConfig::FromContainerArchive {
                    container: container_name,
                    files,
                    archive_name,
                } => extract_archive(chal, &container, files, &chal.directory.join(archive_name))
                    .await
                    .with_context(|| {
                        // rustfmt chokes silently if these format args are inlined... ???
                        format!(
                            "could not create archive {:?} with files {:?} from container {}",
                            archive_name, files, container_name
                        )
                    }),

                // non-container variants handled by outer match
                _ => unreachable!(),
            };

            docker::remove_container(container).await?;

            files
        }
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

    files
        .iter()
        .map(|from| async {
            // use basename of source file as target name
            let to = chal.directory.join(from.file_name().unwrap());
            docker::copy_file(container, from, &to).await
        })
        .try_join_all()
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
    let tempdir = tempfile::Builder::new()
        .prefix(".beavercds-archive-")
        .tempdir_in(".")?;
    let copied_files = files
        .iter()
        .map(|from| async {
            let to = tempdir.path().join(from.file_name().unwrap());
            docker::copy_file(container, from, &to).await
        })
        .try_join_all()
        .await?;

    // archive_name already has the chal dir prepended
    zip_files(archive_name, &copied_files)?;

    Ok(vec![archive_name.to_path_buf()])
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

    z.finish()?;

    Ok(archive_name.to_path_buf())
}
