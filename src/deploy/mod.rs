use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Error, Result};
use itertools::Itertools;
use simplelog::*;

use crate::builder::BuildResult;
use crate::clients::{bucket_client, kube_client};
use crate::cluster_setup as setup;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{enabled_challenges, get_config, get_profile_config};

/// Render challenge manifest templates and apply to cluster
pub async fn deploy_challenges(profile_name: &str, build_results: &[BuildResult]) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    todo!()
}

/// Upload files to frontend asset bucket
/// Returns urls of upload files.
pub async fn upload_assets(
    profile_name: &str,
    build_results: &[BuildResult],
) -> Result<Vec<String>> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    let bucket = bucket_client(&profile.s3)?;

    todo!()

    // TODO: should uploaded URLs be a (generated) part of the challenge config
    // struct?
}

/// Sync deployed challenges with rCTF frontend
pub async fn update_frontend(profile_name: &str, build_results: &[BuildResult]) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    todo!()
}
