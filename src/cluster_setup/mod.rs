use std::fmt::Debug;

use anyhow::{anyhow, Context, Error, Result};
use k8s_openapi::{
    api::apps::v1::Deployment,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::api::{DynamicObject, Patch, PatchParams};
use kube::runtime::WatchStreamExt;
use kube::{Api, ResourceExt};
use serde;
use serde_yml;
use simplelog::*;
use ureq;

use crate::clients::{kube_api_for, kube_client, kube_resource_for};
use crate::configparser::{config, get_config, get_profile_config};

// De../asset_files/setup_ploy cluster resources needed for challenges to work.
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

    // // download manifest from Github re../asset_files/setup_lease artifacts
    // debug!("downloading manifest from github release");
    // const CONTROLLER_VERSION: &str = "v0.15.15"; // current latest release
    // let manifest = reqwest::get(format!("https://github.com/k3s-io/helm-controller/releases/download/{VER}/deploy-cluster-scoped.yaml", VER = CONTROLLER_VERSION))
    //     .await?
    //     .text()
    //     .await?;

    // nevermind that, upstream manifest is missing RBAC
    // use vendored copy with changes
    const MANIFEST: &str = include_str!(
        "../asset_files/setup_manifests/helm-controller-cluster-scoped.deployment.yaml"
    );
    apply_manifest_yaml(client, MANIFEST).await
}

pub async fn install_ingress(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying nginx-ingress chart...");

    let client = kube_client(profile).await?;

    const CHART_YAML: &str = include_str!("../asset_files/setup_manifests/ingress-nginx.helm.yaml");

    // TODO: watch for helm chart manifest to apply correctly
    apply_helm_crd(client, CHART_YAML).await
}

pub async fn install_certmanager(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying cert-manager chart...");

    let client = kube_client(profile).await?;

    const CHART_YAML: &str = include_str!("../asset_files/setup_manifests/cert-manager.helm.yaml");
    apply_helm_crd(client.clone(), CHART_YAML).await?;

    // install cert-manager from upstream static manifest instead of helm chart.
    // the helm install gets confused when its CRDs already exist, so this is
    // more reliable against running cluster-setup multiple times.
    // EDIT: this is not the case when `crds.keep=false` so :shrug:

    // let certmanager_manifest = ureq::get(
    //     "https://github.com/cert-manager/cert-manager/releases/download/v1.16.2/cert-manager.yaml",
    // )
    //   .call()
    //   .context("could not download cert-manager manifest from Github release")?
    //   .into_string()?
    //   // deploy this into ingress namespace with other resources
    //   .replace("namespace: cert-manager", "namespace: ingress");
    // apply_manifest_yaml(client.clone(), &certmanager_manifest);

    // letsencrypt and letsencrypt-staging
    const ISSUERS_YAML: &str =
        include_str!("../asset_files/setup_manifests/letsencrypt.issuers.yaml");
    apply_manifest_yaml(client, ISSUERS_YAML).await
}

pub async fn install_extdns(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying external-dns chart...");

    let client = kube_client(profile).await?;

    const CHART_YAML: &str = include_str!("../asset_files/setup_manifests/external-dns.helm.yaml");
    apply_helm_crd(client, CHART_YAML).await
}

//
// install helpers
//

// Apply helm chart manifest and wait for deployment status
async fn apply_helm_crd(client: kube::Client, manifest: &str) -> Result<()> {
    apply_manifest_yaml(client.clone(), manifest).await?;

    // now wait for the deploy job to run and update status

    // pull out name and namespace from yaml string
    // this will only get a single manifest from the install_* functions,
    // so this unwrapping is safe:tm:
    let chart_crd: DynamicObject = serde_yml::from_str(manifest)?;
    debug!(
        "waiting for chart deployment {} {}",
        chart_crd
            .namespace()
            .unwrap_or("[default namespace]".to_string()),
        chart_crd.name_any()
    );

    let (chart_resource, caps) = kube_resource_for(&chart_crd, &client).await?;
    let chart_api = kube_api_for(&chart_crd, client).await?;

    let watch_conf = kube::runtime::watcher::Config::default();
    let watcher = kube::runtime::metadata_watcher(chart_api, watch_conf);

    // TODO: actually wait for chart status to change...
    // need to get job name from chart resource `.status.jobName`, and look at
    // status of job. helm-controller does not update status of chart CRD :/

    Ok(())
}

/// Apply multi-document manifest file
async fn apply_manifest_yaml(client: kube::Client, manifest: &str) -> Result<()> {
    // set ourself as the owner for managed fields
    // https://kubernetes.io/docs/reference/using-api/server-side-apply/#managers
    let pp = PatchParams::apply("beavercds").force();

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

/// Deserialize multi-document yaml string into a Vec of the documents
fn multidoc_deserialize(data: &str) -> Result<Vec<serde_yml::Value>> {
    use serde::Deserialize;

    let mut docs = vec![];
    for de in serde_yml::Deserializer::from_str(data) {
        match serde_yml::Value::deserialize(de)? {
            // discard any empty documents (e.g. from trailing ---)
            serde_yml::Value::Null => (),
            not_null => docs.push(not_null),
        };
    }
    Ok(docs)

    // // deserialize all chunks
    // serde_yml::Deserializer::from_str(data)
    //     .map(serde_yml::Value::deserialize)
    //     // discard any empty documents (e.g. from trailing ---)
    //     .filter_ok(|val| val != &serde_yml::Value::Null)
    //     // coerce errors to Anyhow
    //     .map(|r| r.map_err(|e| e.into()))
    //     .collect()
}
