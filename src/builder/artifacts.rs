use anyhow::{anyhow, Context, Error, Result};
use futures::future::try_join_all;
use itertools::Itertools;
use simplelog::{debug, trace};
use std::path::{Path, PathBuf};

use crate::builder::docker::{client, copy_file, create_container};
use crate::configparser::challenge::{ChallengeConfig, ProvideConfig};

/// extract assets from given container name and provide config to challenge directory, return file path(s) extracted
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn extract_asset(
    chal: &ChallengeConfig,
    provide: &ProvideConfig,
    container: &str,
) -> Result<Vec<PathBuf>> {
    debug!("extracting assets from container {}", container);
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
    container: &str,
    files: &Vec<String>,
) -> Result<Vec<PathBuf>> {
    trace!(
        "extracting {} files without renaming: {:?}",
        files.len(),
        files
    );

    // try_join_all(
    //     files
    //         .iter()
    //         .enumerate() // need index to avoid copy collisions
    //         .map(|(i, f)| docker::copy_file(container, f, None)),
    // )
    // .await

    let mut results = vec![];

    for f in files.iter() {
        let from = Path::new(f);
        // if no rename is given, use basename of `from` as target path
        // these files should go in chal directory, so pass it in
        let to = chal
            .directory
            .join(from.file_name().unwrap().to_str().unwrap());

        results.push(copy_file(container, from, &to).await?);
    }

    Ok(results)
}

/// Extract one file from container and rename
async fn extract_rename(
    chal: &ChallengeConfig,
    container: &str,
    file: &str,
    new_name: &str,
) -> Result<Vec<PathBuf>> {
    trace!("extracting file and renaming it");

    Ok(vec![])
}

/// Extract one or more file from container as archive
async fn extract_archive(
    chal: &ChallengeConfig,
    container: &str,
    files: &Vec<String>,
    archive_name: &str,
) -> Result<Vec<PathBuf>> {
    trace!("extracting mutliple files into archive");

    Ok(vec![])
}
