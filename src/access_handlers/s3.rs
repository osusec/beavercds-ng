use anyhow::{anyhow, bail, Context, Error, Result};
use s3;
use simplelog::*;
use tokio;

use crate::configparser::{
    config::{ProfileConfig, S3Config},
    get_config, get_profile_config,
};

/// s3 bucket access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile_name: &str) -> Result<()> {
    let profile = get_profile_config(profile_name)?;

    let bucket = bucket_client(&profile.s3)?;

    if !bucket.exists().await? {
        bail!("bucket {} does not exist!", profile.s3.bucket_name);
    }

    // try uploading file to bucket
    debug!("uploading test file to bucket");
    let test_file = ("/beavercds-test-file", "access test file!");
    bucket
        .put_object_with_content_type(test_file.0, test_file.1.as_bytes(), "text/plain")
        .await
        .with_context(|| {
            format!(
                "could not upload to asset bucket {:?}",
                profile.s3.bucket_name
            )
        })?;

    // download it to check
    debug!("downloading test file");
    let from_bucket = bucket.get_object(test_file.0).await?;
    if from_bucket.bytes() != test_file.1 {
        bail!("uploaded test file contents do not match, somehow!?");
    }

    // download as anonymous to check public access
    debug!("downloading test file as public user");
    let public_bucket = bucket_client_anonymous(&profile.s3)?;
    let from_public = public_bucket
        .get_object(test_file.0)
        .await
        .with_context(|| {
            anyhow!(
                "public download from asset bucket {:?} failed",
                profile.s3.bucket_name
            )
        })?;
    if from_public.bytes() != test_file.1 {
        bail!("contents of public bucket do not match uploaded file");
    }

    // clean up test file after checks
    bucket.delete_object(test_file.0).await?;

    Ok(())
}

/// create bucket client for passed profile config
pub fn bucket_client(config: &S3Config) -> Result<Box<s3::Bucket>> {
    trace!("creating bucket client");
    // TODO: once_cell this so it reuses the same bucket?
    let region = s3::Region::Custom {
        region: config.region.clone(),
        endpoint: config.endpoint.clone(),
    };
    let creds = s3::creds::Credentials::new(
        Some(&config.access_key),
        Some(&config.secret_key),
        None,
        None,
        None,
    )?;
    let bucket = s3::Bucket::new(&config.bucket_name, region, creds)?.with_path_style();

    Ok(bucket)
}

/// create public/anonymous bucket client for passed profile config
pub fn bucket_client_anonymous(config: &S3Config) -> Result<Box<s3::Bucket>> {
    trace!("creating anon bucket client");
    // TODO: once_cell this so it reuses the same bucket?
    let region = s3::Region::Custom {
        region: config.region.clone(),
        endpoint: config.endpoint.clone(),
    };
    let creds = s3::creds::Credentials::anonymous()?;
    let bucket = s3::Bucket::new(&config.bucket_name, region, creds)?.with_path_style();

    Ok(bucket)
}
