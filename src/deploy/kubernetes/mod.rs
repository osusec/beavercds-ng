use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Error, Ok, Result};
use base64ct::{Base64, Encoding};
use bollard::auth::DockerCredentials;
use itertools::Itertools;
use minijinja;
use tokio::time::timeout;
use tracing::{debug, error, info, trace, warn};

use crate::builder::BuildResult;
use crate::clients::{apply_manifest_yaml, kube_client, wait_for_status};
use crate::configparser::challenge::ExposeType;
use crate::configparser::config::ProfileConfig;
use crate::configparser::{get_config, get_profile_config, ChallengeConfig};
use crate::utils::{render_strict, TryJoinAll};

pub mod templates;

/// How and where a challenge was deployed/exposed at
pub struct KubeDeployResult {
    // challenges could have multiple exposed services
    pub exposed: Vec<PodDeployResult>,
}

pub enum PodDeployResult {
    Http { domain: String },
    Tcp { port: usize },
}

// Deploy all K8S resources for a single challenge `chal`.
//
// Creates the challenge namespace, deployments, services, and ingresses needed
// to deploy and expose the challenge.
pub async fn apply_challenge_resources(
    profile_name: &str,
    chal: &ChallengeConfig,
) -> Result<KubeDeployResult> {
    info!(
        "  deploying kube resources for chal {:?}...",
        chal.directory
    );
    // render templates

    let profile = get_profile_config(profile_name)?;

    let kube = kube_client(profile).await?;

    let ns_manifest = render_strict(
        templates::CHALLENGE_NAMESPACE,
        minijinja::context! { chal, slug => chal.slugify() },
    )?;
    trace!("NAMESPACE:\n{}", ns_manifest);

    debug!("applying namespace for chal {:?}", chal.directory);

    // apply namespace manifest
    apply_manifest_yaml(&kube, &ns_manifest)
        .await?
        .iter()
        // and then wait for it to be ready
        .map(|object| wait_for_status(&kube, object))
        .try_join_all()
        .await?;

    // add image pull credentials to the new namespace
    debug!(
        "applying namespace pull credentials for chal {:?}",
        chal.directory
    );

    let registry = &get_config()?.registry;
    let creds_manifest = render_strict(
        templates::IMAGE_PULL_CREDS_SECRET,
        minijinja::context! {
            slug => chal.slugify(),
            registry_domain => registry.domain,
            creds_b64 => Base64::encode_string(format!("{}:{}",
                registry.cluster.user,
                registry.cluster.pass,
            ).as_bytes()),
        },
    )?;
    apply_manifest_yaml(&kube, &creds_manifest).await?;

    // namespace boilerplate over, deploy actual challenge pods

    let results = KubeDeployResult { exposed: vec![] };

    for pod in &chal.pods {
        let pod_image = chal.container_tag_for_pod(profile_name, &pod.name)?;
        let depl_manifest = render_strict(
            templates::CHALLENGE_DEPLOYMENT,
            minijinja::context! {
                chal, pod, pod_image, profile_name,
                slug => chal.slugify(),
            },
        )?;
        trace!("DEPLOYMENT:\n{}", depl_manifest);

        trace!(
            "applying deployment for chal {:?} pod {:?}",
            chal.directory,
            pod.name
        );
        let depl = apply_manifest_yaml(&kube, &depl_manifest).await?;
        for object in depl {
            // wait for objects to be ready, with 5m timeout
            timeout(Duration::from_secs(5 * 60), wait_for_status(&kube, &object))
                .await
                // timeout wraps with another Result
                .with_context(|| {
                    format!(
                        "timed out waiting for chal {:?} pod {:?} deployment to become ready",
                        chal.directory, pod.name
                    )
                })?
                // inner result from wait_for_status
                .with_context(|| {
                    format!(
                        "failed to get status for chal {:?} pod {:?} deployment",
                        chal.directory, pod.name
                    )
                })?;
        }

        // tcp and http exposes need to he handled separately, so separate them by type
        let (tcp_ports, http_ports): (Vec<_>, Vec<_>) = pod
            .ports
            .iter()
            .partition(|p| matches!(p.expose, ExposeType::Tcp(_)));

        if !tcp_ports.is_empty() {
            let tcp_manifest = render_strict(
                templates::CHALLENGE_SERVICE_TCP,
                minijinja::context! {
                    chal, pod, tcp_ports,
                    slug => chal.slugify(), name_slug => chal.slugify_name(), domain => profile.challenges_domain
                },
            )?;
            trace!("TCP SERVICE:\n{}", tcp_manifest);

            debug!(
                "applying tcp service for chal {:?} pod {:?}",
                chal.directory, pod.name
            );
            let tcp = apply_manifest_yaml(&kube, &tcp_manifest).await?;
            for object in tcp {
                // wait for objects to be ready, with 5m timeout
                timeout(Duration::from_secs(5 * 60), wait_for_status(&kube, &object))
                    .await
                    // timeout wraps with another Result
                    .with_context(|| {
                        format!(
                            "timed out waiting for chal {:?} pod {:?} exposed TCP service to become ready",
                            chal.directory, pod.name
                        )
                    })?
                    // inner result from wait_for_status
                    .with_context(|| {
                        format!(
                            "failed to get status for chal {:?} pod {:?} exposed TCP service",
                            chal.directory, pod.name
                        )
                    })?;
            }

            // TODO:
            // expose_results.exposed.push(PodDeployResult::Tcp { port: tcp_ports[0]. });
        }

        if !http_ports.is_empty() {
            let http_manifest = render_strict(
                templates::CHALLENGE_SERVICE_HTTP,
                minijinja::context! {
                    chal, pod, http_ports,
                    slug => chal.slugify(), domain => profile.challenges_domain
                },
            )?;
            trace!("HTTP INGRESS:\n{}", http_manifest);

            debug!(
                "applying http service and ingress for chal {:?} pod {:?}",
                chal.directory, pod.name
            );
            let ingress = apply_manifest_yaml(&kube, &http_manifest).await?;
            for object in ingress {
                // wait for objects to be ready, with 5m timeout
                timeout(Duration::from_secs(5 * 60), wait_for_status(&kube, &object))
                    .await
                    // timeout wraps with another Result
                    .with_context(|| {
                        format!(
                            "timed out waiting for chal {:?} pod {:?} ingress to become ready",
                            chal.directory, pod.name
                        )
                    })?
                    // inner result from wait_for_status
                    .with_context(|| {
                        format!(
                            "failed to get status for chal {:?} pod {:?} ingress",
                            chal.directory, pod.name
                        )
                    })?;
            }
        }
    }

    Ok(results)
}

// Updates the current ingress controller chart with the current set of TCP
// ports needed for challenges.
// TODO: move to Gateway to avoid needing to redeploy ingress?
// TODO: is this needed? currently TCP challenges are separate LoadBalancer svcs
// async fn update_ingress_tcp() -> Result<()> {
//     Ok(())
// }
