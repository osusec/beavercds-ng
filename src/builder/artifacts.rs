use anyhow::{anyhow, Context, Error, Result};
use futures::future::try_join_all;
use itertools::Itertools;
use simplelog::{debug, trace};
use std::path::PathBuf;

use crate::builder::docker::{client, copy_file, create_container};
use crate::configparser::challenge::ProvideConfig;

use super::docker;

/// extract assets from given container name and provide config to challenge directory, return file path(s) extracted
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn extract_asset(provide: &ProvideConfig, container: &str) -> Result<Vec<String>> {
    debug!("extracting assets from container {}", container);
    // This needs to handle three cases:
    // - single or multiple files without renaming (no as: field)
    // - single file with rename (one item with as:)
    // - multiple files as archive (multiple items with as:)

    // TODO: since this puts artifacts in the repo source folder, this should
    // try to not overwrite any existing files.

    match &provide.as_file {
        // no renaming, copy out all as-is
        None => extract_files(container, &provide.include).await,
        // (as is keyword, so add underscore)
        Some(as_) => {
            if provide.include.len() == 1 {
                // single file, rename
                extract_rename(container, &provide.include[0], as_).await
            } else {
                // multiple files, zip as archive
                extract_archive(container, &provide.include, as_).await
            }
        }
    }
}

/// Extract multiple files from container
async fn extract_files(container: &str, files: &Vec<String>) -> Result<Vec<String>> {
    trace!(
        "extracting {} files without renaming: {:?}",
        files.len(),
        files
    );

    try_join_all(
        files
            .iter()
            .enumerate() // need index to avoid copy collisions
            .map(|(i, f)| docker::copy_file(container, f, None)),
    )
    .await

    // files
    //     .iter()
    //     .map(|f| docker::copy_file(container, f, None))
    //     .collect::<Result<Vec<_>>>()
}

/// Extract one file from container and rename
async fn extract_rename(container: &str, file: &str, new_name: &str) -> Result<Vec<String>> {
    trace!("extracting file and renaming it");

    Ok(vec!["todo rename".to_string()])
}

/// Extract one or more file from container as archive
async fn extract_archive(
    container: &str,
    files: &Vec<String>,
    archive_name: &str,
) -> Result<Vec<String>> {
    trace!("extracting mutliple files into archive");

    Ok(vec!["todo archive".to_string()])
}
