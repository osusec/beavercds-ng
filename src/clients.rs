// Builders for the various client structs for Docker/Kube etc.

use anyhow::{anyhow, bail, Ok, Result};
use bollard;
use kube;
use simplelog::*;

use crate::configparser::config;

//
// Docker stuff
//
pub async fn docker() -> Result<bollard::Docker> {
    debug!("connecting to docker...");
    let client = bollard::Docker::connect_with_defaults()?;
    client
        .ping()
        .await
        // truncate error chain with new error (returned error is way too verbose)
        .map_err(|_| anyhow!("could not talk to Docker daemon (is DOCKER_HOST correct?)"))?;

    Ok(client)
}

#[derive(Debug)]
pub enum EngineType {
    Docker,
    Podman,
}
pub async fn engine_type() -> EngineType {
    let c = docker().await.unwrap();
    let version = c.version().await.unwrap();

    if version
        .components
        .unwrap()
        .iter()
        .any(|c| c.name == "Podman Engine")
    {
        EngineType::Podman
    } else {
        EngineType::Docker
    }
}

//
// Kubernetes stuff
//

/// Returns Kubernetes Client for selected profile
pub async fn kube_client(profile: &config::ProfileConfig) -> Result<kube::Client> {
    debug!("building kube client");

    // read in kubeconfig from given kubeconfig (or default)
    // (use kube::Config to specify context)
    let options = kube::config::KubeConfigOptions {
        context: Some(profile.kubecontext.to_owned()),
        cluster: None,
        user: None,
    };

    let client_config = match &profile.kubeconfig {
        Some(kc_path) => {
            let kc = kube::config::Kubeconfig::read_from(kc_path)?;
            kube::Config::from_custom_kubeconfig(kc, &options).await?
        }
        None => kube::Config::from_kubeconfig(&options).await?,
    };

    // client::try_from returns a Result, but the Error is not compatible
    // with anyhow::Error, so assign this with ? and return Ok() separately
    let client = kube::Client::try_from(client_config)?;
    Ok(client)
}
