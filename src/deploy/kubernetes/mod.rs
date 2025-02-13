use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use itertools::Itertools;
use minijinja;
use simplelog::*;

use crate::builder::BuildResult;
use crate::clients::kube_client;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{get_config, get_profile_config, ChallengeConfig};
use crate::utils::TryJoinAll;

pub mod templates;

/// Render challenge manifest templates and apply to cluster
pub async fn deploy_challenges(
    profile_name: &str,
    build_results: &[(&ChallengeConfig, BuildResult)],
) -> Result<Vec<()>> {
    let profile = get_profile_config(profile_name)?;

    // Kubernetes deployment needs to:
    // 1. render manifests
    //   - namespace
    //   - challenge pod deployment(s)
    //   - service
    //   - ingress
    //
    // 2. update ingress controller tcp ports
    //
    // 3. wait for all challenges to become ready (?)

    build_results
        .iter()
        .map(|(chal, _)| deploy_single_challenge(profile_name, chal))
        .try_join_all()
        .await
}

async fn deploy_single_challenge(profile_name: &str, chal: &ChallengeConfig) -> Result<()> {
    // render templates

    let ns_manifest = minijinja::render!(templates::CHALLENGE_NAMESPACE, slug => chal.slugify());
    trace!("NAMESPACE:\n{:#?}", ns_manifest);

    for pod in &chal.pods {
        let depl_manifest = minijinja::render!(
            templates::CHALLENGE_DEPLOYMENT,
            chal, pod, profile_name, slug => chal.slugify(),
        );

        trace!("DEPLOYMENT:\n{:#?}", depl_manifest);
    }
    Ok(())
}
