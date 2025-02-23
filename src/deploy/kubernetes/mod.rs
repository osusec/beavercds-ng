use std::path::PathBuf;

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use itertools::Itertools;
use minijinja;
use simplelog::*;

use crate::builder::BuildResult;
use crate::clients::{apply_manifest_yaml, kube_client};
use crate::configparser::challenge::ExposeType;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{get_config, get_profile_config, ChallengeConfig};
use crate::utils::TryJoinAll;

pub mod templates;

/// How and where a challenge was deployed/exposed at
pub struct DeployResult {
    // challenges could have multiple exposed services
    pub exposed: Vec<PodDeployResult>,
}

pub enum PodDeployResult {
    Http { domain: String },
    Tcp { port: usize },
}

/// Render challenge manifest templates and apply to cluster
pub async fn deploy_challenges(
    profile_name: &str,
    build_results: &[(&ChallengeConfig, BuildResult)],
) -> Result<Vec<DeployResult>> {
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
    // 3. wait for all challenges to become ready
    //
    // 4. record domains and IPs of challenges to pass to frontend (?)

    let results = build_results
        .iter()
        .map(|(chal, _)| deploy_single_challenge(profile_name, chal))
        .try_join_all()
        .await?;

    update_ingress_tcp().await?;

    Ok(results)
}

// Deploy all K8S resources for a single challenge `chal`.
//
// Creates the challenge namespace, deployments, services, and ingresses needed
// to deploy and expose the challenge.
async fn deploy_single_challenge(
    profile_name: &str,
    chal: &ChallengeConfig,
) -> Result<DeployResult> {
    info!("  deploying chal {:?}...", chal.directory);
    // render templates

    let profile = get_profile_config(profile_name)?;

    let kube = kube_client(profile).await?;

    let ns_manifest = minijinja::render!(
        templates::CHALLENGE_NAMESPACE,
        chal, slug => chal.slugify()
    );
    trace!("NAMESPACE:\n{}", ns_manifest);

    debug!("applying namespace for chal {:?}", chal.directory);
    apply_manifest_yaml(&kube, &ns_manifest).await?;

    let expose_results = DeployResult { exposed: vec![] };

    for pod in &chal.pods {
        let pod_image = chal.container_tag_for_pod(profile_name, &pod.name)?;
        let depl_manifest = minijinja::render!(
            templates::CHALLENGE_DEPLOYMENT,
            chal, pod, pod_image, profile_name, slug => chal.slugify(),
        );
        trace!("DEPLOYMENT:\n{}", depl_manifest);

        debug!(
            "applying deployment for chal {:?} pod {:?}",
            chal.directory, pod.name
        );
        apply_manifest_yaml(&kube, &depl_manifest).await?;

        // tcp and http exposes need to he handled separately, so separate them by type
        let (tcp_ports, http_ports): (Vec<_>, Vec<_>) = pod
            .ports
            .iter()
            .partition(|p| matches!(p.expose, ExposeType::Tcp(_)));

        if !tcp_ports.is_empty() {
            let tcp_manifest = minijinja::render!(
                templates::CHALLENGE_SERVICE_TCP,
                chal, pod, tcp_ports, slug => chal.slugify(), domain => profile.challenges_domain
            );
            trace!("TCP SERVICE:\n{}", tcp_manifest);

            debug!(
                "applying tcp service for chal {:?} pod {:?}",
                chal.directory, pod.name
            );
            apply_manifest_yaml(&kube, &tcp_manifest).await?;

            // TODO:
            // expose_results.exposed.push(PodDeployResult::Tcp { port: tcp_ports[0]. });
        }

        if !http_ports.is_empty() {
            let http_manifest = minijinja::render!(
                templates::CHALLENGE_SERVICE_HTTP,
                chal, pod, http_ports, slug => chal.slugify(), domain => profile.challenges_domain
            );
            trace!("HTTP INGRESS:\n{}", http_manifest);

            debug!(
                "applying http service and ingress for chal {:?} pod {:?}",
                chal.directory, pod.name
            );
            apply_manifest_yaml(&kube, &http_manifest).await?;
        }
    }

    Ok(expose_results)
}

// Updates the current ingress controller chart with the current set of TCP
// ports needed for challenges.
// TODO: move to Gateway to avoid needing to redeploy ingress?
async fn update_ingress_tcp() -> Result<()> {
    Ok(())
}
