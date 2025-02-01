use std::fs::File;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use itertools::Itertools;
use simplelog::*;

use crate::builder::BuildResult;
use crate::clients::bucket_client;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{enabled_challenges, get_config, get_profile_config, ChallengeConfig};

/// Upload files to frontend asset bucket
/// Returns urls of upload files.
pub async fn upload_assets(
    profile_name: &str,
    build_results: &[(&ChallengeConfig, BuildResult)],
) -> Result<Vec<String>> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    let bucket = bucket_client(&profile.s3)?;

    todo!();
}
