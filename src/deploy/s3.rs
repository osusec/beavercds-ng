use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use futures::future::try_join_all;
use itertools::Itertools;
use s3::Bucket;
use tokio;
use tracing::{debug, error, info, trace, warn};

use crate::builder::BuildResult;
use crate::clients::bucket_client;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{enabled_challenges, get_config, get_profile_config, ChallengeConfig};
use crate::utils::TryJoinAll;

/// Artifacts and information about a deployed challenges.
pub struct S3DeployResult {
    pub uploaded_asset_urls: Vec<String>,
}

/// Upload files to frontend asset bucket
/// Returns urls of upload files.
pub async fn upload_challenge_assets(
    profile_name: &str,
    chal: &ChallengeConfig,
    build_result: &BuildResult,
) -> Result<S3DeployResult> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    let bucket = bucket_client(&profile.s3)?;

    info!("uploading assets for chal {:?}...", chal.directory);

    let uploaded = build_result
        .assets
        .iter()
        .map(|asset_file| async move {
            debug!("uploading file {:?}", asset_file);
            // upload to bucket
            let bucket_path = upload_single_file(bucket, chal, asset_file)
                .await
                .with_context(|| format!("failed to upload file {asset_file:?}"))?;

            // return link to the uploaded file
            // TODO: only works for AWS rn! support other providers
            let url = format!(
                "https://{bucket}.s3.{region}.amazonaws.com/{path}",
                bucket = &profile.s3.bucket_name,
                region = &profile.s3.region,
                path = bucket_path.to_string_lossy(),
            );
            Ok(url)
        })
        .try_join_all()
        .await
        .with_context(|| format!("failed to upload asset files for chal {:?}", chal.directory))?;

    // return new BuildResult with assets as bucket path
    Ok(S3DeployResult {
        uploaded_asset_urls: uploaded,
    })
}

async fn upload_single_file(
    bucket: &Bucket,
    chal: &ChallengeConfig,
    file: &Path,
) -> Result<PathBuf> {
    // e.g. s3.example.domain/assets/misc/foo/stuff.zip
    let path_in_bucket = format!(
        "assets/{chal_slug}/{file}",
        chal_slug = chal.directory.to_string_lossy(),
        file = file.file_name().unwrap().to_string_lossy()
    );

    trace!("uploading {:?} to bucket path {:?}", file, &path_in_bucket);

    // TODO: move to async/streaming to better handle large files and report progress
    let mut asset_file = tokio::fs::File::open(file).await?;
    let r = bucket
        .put_object_stream(&mut asset_file, &path_in_bucket)
        .await?;
    trace!("uploaded {} bytes for file {:?}", r.uploaded_bytes(), file);

    Ok(PathBuf::from(path_in_bucket))
}
