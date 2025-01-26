// Builders for the various client structs for Docker/Kube etc.

use anyhow::{anyhow, bail, Context, Error, Result};
use bollard;
use futures::TryFutureExt;
use kube::{
    self,
    api::{DynamicObject, GroupVersionKind, TypeMeta},
    core::ResourceExt,
    discovery::{ApiCapabilities, ApiResource, Discovery, Scope},
};
use s3;
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
// S3 stuff
//

/// create bucket client for passed profile config
pub fn bucket_client(config: &config::S3Config) -> Result<Box<s3::Bucket>> {
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
pub fn bucket_client_anonymous(config: &config::S3Config) -> Result<Box<s3::Bucket>> {
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
            kube::Config::from_custom_kubeconfig(kc, &options).await
        }
        None => kube::Config::from_kubeconfig(&options).await,
    }?;

    let client = kube::Client::try_from(client_config)?;

    // check kube api readiness endpoint to make sure its reachable
    let ready_req = http::Request::get("/readyz").body(vec![]).unwrap();
    let ready_resp = client
        .request_text(ready_req)
        .await
        // change 'Connection refused' error into something more helpful
        .map_err(|e| {
            anyhow!("could not connect to Kubernetes (is KUBECONFIG or KUBECONTEXT correct?)")
        })?;

    if ready_resp != "ok" {
        bail!("Kubernetes is not ready")
    };

    Ok(client)
}

pub async fn kube_resource_for(
    kube_object: &DynamicObject,
    client: &kube::Client,
) -> Result<(ApiResource, ApiCapabilities)> {
    let gvk = if let Some(tm) = &kube_object.types {
        GroupVersionKind::try_from(tm)?
    } else {
        bail!(
            "cannot apply object without valid TypeMeta {:?}",
            kube_object
        );
    };

    let name = kube_object.name_any();

    kube::discovery::pinned_kind(client, &gvk)
        .await
        .with_context(|| {
            format!(
                "could not find resource type {:?} on cluster",
                kube_object.types.clone().unwrap_or_default()
            )
        })
}

/// Create a Kube API client for the passed object's resource type
pub async fn kube_api_for(
    kube_object: &DynamicObject,
    client: kube::Client,
) -> Result<kube::Api<DynamicObject>> {
    let ns = kube_object.metadata.namespace.as_deref();

    let (resource, caps) = kube_resource_for(kube_object, &client).await?;

    if caps.scope == kube::discovery::Scope::Cluster {
        Ok(kube::Api::all_with(client, &resource))
    } else if let Some(namespace) = ns {
        Ok(kube::Api::namespaced_with(client, namespace, &resource))
    } else {
        Ok(kube::Api::default_namespaced_with(client, &resource))
    }
}
