use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use itertools::Itertools;
use simplelog::*;

use crate::builder::BuildResult;
use crate::clients::kube_client;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{enabled_challenges, get_config, get_profile_config, ChallengeConfig};

/// Render challenge manifest templates and apply to cluster
pub async fn deploy_challenges(
    profile_name: &str,
    build_results: &[(&ChallengeConfig, BuildResult)],
) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    let enabled_challenges = enabled_challenges(profile_name)?;

    todo!()
}
