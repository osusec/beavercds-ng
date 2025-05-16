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
use owo_colors::OwoColorize;
use serde;
use serde_yml;
use tempfile;
use tracing::{debug, error, info, trace, warn};

use crate::clients::{apply_manifest_yaml, kube_client};
use crate::configparser::{config, get_config, get_profile_config};
use crate::utils::render_strict;

// Deploy cluster resources needed for challenges to work.
//
// Some components can or must be deployed and configured ahead of time, like
// the ingress controller, cert-manager, and external-dns

// install these charts into this namespace
pub const INGRESS_NAMESPACE: &str = "ingress";

pub async fn install_ingress(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying ingress-nginx chart...");

    const VALUES: &str = include_str!("../asset_files/setup_manifests/ingress-nginx.helm.yaml");
    trace!("values:\n{}", VALUES);

    install_helm_chart(
        profile,
        "ingress-nginx",
        "https://kubernetes.github.io/ingress-nginx",
        None,
        "ingress-nginx",
        INGRESS_NAMESPACE,
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
        "https://charts.jetstack.io",
        None,
        "cert-manager",
        INGRESS_NAMESPACE,
        VALUES,
    )?;

    info!("deploying cert-manager issuers...");
    let client = kube_client(profile).await?;

    // letsencrypt and letsencrypt-staging
    const ISSUERS_TEMPLATE: &str =
        include_str!("../asset_files/setup_manifests/letsencrypt.issuers.yaml");

    let issuers_yaml = render_strict(
        ISSUERS_TEMPLATE,
        minijinja::context! {
            chal_domain => profile.challenges_domain
        },
    )?;

    apply_manifest_yaml(&client, &issuers_yaml).await?;

    Ok(())
}

pub async fn install_extdns(profile: &config::ProfileConfig) -> Result<()> {
    info!("deploying external-dns chart...");

    const VALUES_TEMPLATE: &str =
        include_str!("../asset_files/setup_manifests/external-dns.helm.yaml.j2");

    // add profile dns: field directly to chart values
    let values = render_strict(
        VALUES_TEMPLATE,
        minijinja::context! {
            provider_credentials => serde_yml::to_string(&profile.dns)?,
            chal_domain => profile.challenges_domain
        },
    )?;

    trace!("deploying templated external-dns values:\n{}", values);

    install_helm_chart(
        profile,
        "external-dns",
        "https://kubernetes-sigs.github.io/external-dns",
        None,
        "external-dns",
        INGRESS_NAMESPACE,
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
    repo: &str,
    version: Option<&str>,
    release_name: &str,
    namespace: &str,
    values: &str,
) -> Result<()> {
    // make sure `helm` is available to run
    duct::cmd!("helm", "version")
        .read()
        .context("helm binary is not available")?;

    // write values to tempfile
    let mut temp_values = tempfile::Builder::new()
        .prefix(release_name)
        .suffix(".values.yaml")
        .tempfile()?;
    temp_values.write_all(values.as_bytes())?;

    let version_arg = match version {
        Some(v) => format!("--version {v}"),
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
            {chart} --repo {repo} {version_arg}
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
    let lines = BufReader::new(reader).lines();

    for item in lines {
        match item {
            Ok(line) => {
                debug!("helm: {}", line.bright_black());
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}
