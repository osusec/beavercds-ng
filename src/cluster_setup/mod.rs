use std::fmt::Debug;
use std::io::prelude::*;
use std::io::BufReader;

use anyhow::{anyhow, bail, Context, Error, Result};
use duct;
use http::header::VARY;
use itertools::Itertools;
use k8s_openapi::{
    api::apps::v1::Deployment,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::api::{DynamicObject, Patch, PatchParams};
use kube::runtime::WatchStreamExt;
use kube::{Api, ResourceExt};
use minijinja;
use serde;
use serde_yml;
use simplelog::*;
use tempfile;

use crate::clients::{kube_api_for, kube_client, kube_resource_for};
use crate::configparser::{config, get_config, get_profile_config};

// Deploy cluster resources needed for challenges to work.
//
// Some components can or must be deployed and configured ahead of time, like
// the ingress controller, cert-manager, and external-dns

pub async fn install_ingress(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying ingress-nginx chart...");

    const VALUES: &str = include_str!("../asset_files/setup_manifests/ingress-nginx.helm.yaml");
    trace!("values:\n{}", VALUES);

    install_helm_chart(
        profile,
        "ingress-nginx",
        Some("https://kubernetes.github.io/ingress-nginx"),
        "ingress-nginx",
        "ingress",
        VALUES,
    )
    .context("failed to install ingress-nginx helm chart")
}

pub async fn install_certmanager(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying cert-manager chart...");

    const VALUES: &str = include_str!("../asset_files/setup_manifests/cert-manager.helm.yaml");
    trace!("values:\n{}", VALUES);

    install_helm_chart(
        profile,
        "cert-manager",
        Some("https://charts.jetstack.io"),
        "cert-manager",
        "ingress",
        VALUES,
    )?;

    info!("deploying cert-manager issuers...");
    let client = kube_client(profile).await?;

    // letsencrypt and letsencrypt-staging
    const ISSUERS_YAML: &str =
        include_str!("../asset_files/setup_manifests/letsencrypt.issuers.yaml");
    apply_manifest_yaml(client, ISSUERS_YAML).await
}

pub async fn install_extdns(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying external-dns chart...");

    const VALUES_TEMPLATE: &str =
        include_str!("../asset_files/setup_manifests/external-dns.helm.yaml.j2");

    // add profile dns: field directly to chart values
    let values = minijinja::render!(
        VALUES_TEMPLATE,
        provider_credentials => serde_yml::to_string(&profile.dns)?,
        chal_domain => profile.challenges_domain
    );
    trace!("deploying templated external-dns values:\n{}", values);

    install_helm_chart(
        profile,
        "oci://registry-1.docker.io/bitnamicharts/external-dns",
        None,
        "external-dns",
        "ingress",
        &values,
    )
}

//
// install helpers
//

/// Install the chart via shelling out to Helm cli
fn install_helm_chart(
    profile: &config::ProfileConfig,
    chart: &str,
    repo: Option<&str>,
    release_name: &str,
    namespace: &str,
    values: &str,
) -> Result<()> {
    // write values to tempfile
    let mut temp_values = tempfile::Builder::new()
        .prefix(release_name)
        .suffix(".values.yaml")
        .tempfile()?;
    temp_values.write_all(values.as_bytes())?;

    let repo_arg = match repo {
        Some(r) => format!("--repo {r}"),
        None => "".to_string(),
    };

    // build args as string/split instead of direct vec to make interpolating
    // conditional repo_arg easier. there is not weird whitespace etc. that
    // would mess up interpolation; all of the values here are constants
    // elsewhere, no user input.

    // use `upgrade --install` instead of `install` so subsequent runs dont
    // error when the release already exists
    let args = format!(
        r#"
        upgrade --install
            {release_name}
            {chart} {repo_arg}
            --namespace {namespace} --create-namespace
            --values {}
            --wait --timeout 1m
            --debug
            --kube-context {}
        "#,
        temp_values.path().to_string_lossy(),
        profile.kubecontext
    );

    let mut helm_cmd = duct::cmd("helm", args.split_whitespace())
        // capture stdout and stderr for our logging
        .stderr_to_stdout()
        .stdout_capture();

    // set kubeconfig if there is one in the profile
    if let Some(kc) = profile.kubeconfig.as_ref() {
        // TODO: normalize ~/ in path
        helm_cmd = helm_cmd.env("KUBECONFIG", kc)
    }

    // stream output to stdout
    let reader = helm_cmd.reader()?;
    let mut lines = BufReader::new(reader).lines();

    while let Some(item) = lines.next() {
        match item {
            Ok(line) => debug!("helm: <bright-black>{line}</>"),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

/// Apply multi-document manifest file
async fn apply_manifest_yaml(client: kube::Client, manifest: &str) -> Result<()> {
    // set ourself as the owner for managed fields
    // https://kubernetes.io/docs/reference/using-api/server-side-apply/#managers
    let pp = PatchParams::apply("beavercds").force();

    // this manifest has multiple documents (crds, deployment)
    for yaml in multidoc_deserialize(manifest)? {
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
