use std::fmt::Debug;

use anyhow::{anyhow, Context, Error, Result};
use k8s_openapi::{
    api::apps::v1::Deployment,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::api::{DynamicObject, Patch, PatchParams};
use kube::{Api, ResourceExt};
use serde;
use serde_yml;
use simplelog::*;

use crate::clients::{kube_api_for, kube_client};
use crate::configparser::{config, get_config, get_profile_config};

// Deploy cluster resources needed for challenges to work.
//
// Some components can or must be deployed and configured ahead of time, like
// the ingress controller, cert-manager, external-dns, and helm controller.

/// Install k3s-io/helm-controller to manage Helm charts as custom resources.
// The native Helm SDK/API is only available for Golang, and there are no
// native wrappers around it. That leaves us with two options:
//   - shell out to the Helm CLI
//   - use an in-cluster operator/controller to manage Helm deployments as CRDs
//
// This uses the latter option, as it should be more reliable than shelling out
// and parsing the output, and does not require Helm to be installed.
pub async fn deploy_helm_controller(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying Helm controller...");
    let client = kube_client(profile).await?;

    // // download manifest from Github release artifacts
    // debug!("downloading manifest from github release");
    // const CONTROLLER_VERSION: &str = "v0.15.15"; // current latest release
    // let manifest = reqwest::get(format!("https://github.com/k3s-io/helm-controller/releases/download/{VER}/deploy-cluster-scoped.yaml", VER = CONTROLLER_VERSION))
    //     .await?
    //     .text()
    //     .await?;

    // // the upstream manifest uses an unqualified image, so we need to add the registry to it
    // let manifest = manifest.replace(
    //     "image: rancher/helm-controller",
    //     "image: docker.io/rancher/helm-controller",
    // );

    // nevermind that, upstream manifest is missing RBAC
    // use vendored copy with changes
    let manifest = include_str!("helm-controller-cluster-scoped.deployment.yaml");

    let pp = PatchParams::apply("kubectl-light").force();

    // this manifest has multiple documents (crds, deployment)
    for yaml in multidoc_deserialize(&manifest)? {
        let obj: DynamicObject = serde_yml::from_value(yaml)?;
        debug!(
            "applying resource {} {}",
            obj.types.clone().unwrap_or_default().kind,
            obj.name_any()
        );

        let obj_api = kube_api_for(&obj, client.clone()).await?;
        match obj_api
            // patch is idempotent and will create if not present
            .patch(&obj.name_any(), &pp, &Patch::Apply(&obj))
            .await
        {
            Ok(d) => Ok(()),
            // if error is from cluster api, mark it as such
            Err(kube::Error::Api(ae)) => {
                // Err(kube::Error::Api(ae).into())
                Err(anyhow!(ae).context("error from cluster when deploying"))
            }
            // other errors could be anything
            Err(e) => Err(anyhow!(e)).context("unknown error when deploying"),
        }?;
    }

    Ok(())
}

pub fn install_ingress(profile: &config::ProfileConfig) -> Result<()> {
    Ok(())
}

pub fn install_certmanager(profile: &config::ProfileConfig) -> Result<()> {
    Ok(())
}

pub fn install_extdns(profile: &config::ProfileConfig) -> Result<()> {
    Ok(())
}

/// Deserialize multi-document yaml string into a Vec of the documents
fn multidoc_deserialize(data: &str) -> Result<Vec<serde_yml::Value>> {
    use serde::Deserialize;
    let mut docs = vec![];
    for de in serde_yml::Deserializer::from_str(data) {
        match serde_yml::Value::deserialize(de)? {
            serde_yml::Value::Null => (),
            not_null => docs.push(not_null),
        };
    }
    Ok(docs)
}
