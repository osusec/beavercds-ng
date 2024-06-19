use anyhow::{anyhow, Error, Result};
use k8s_openapi::api::{
    authentication::v1::SelfSubjectReview, authorization::v1::SelfSubjectAccessReview,
};
use k8s_openapi::serde_json::{from_value, json, to_string};
use kube;
use simplelog::*;
use tokio;

use crate::configparser::{config, CONFIG};

/// kubernetes access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile: &config::ProfileConfig) -> Result<()> {
    // we need to make sure that:
    // a) can talk to the cluster
    // b) have the right permissions (a la `kubectl auth can-i`)

    // build a client
    let client = client(profile).await?;

    // try to get cluster info (whoami)
    let reviewapi: kube::Api<SelfSubjectReview> = kube::Api::all(client);
    let resp = reviewapi
        .create(
            &kube::api::PostParams::default(),
            &from_value(json!({
                "apiVersion": "authentication.k8s.io/v1",
                "kind": "SelfSubjectReview"
            }))?,
        )
        .await?;
    let status = resp.status.ok_or(anyhow!("could not access cluster"))?;

    // todo: Is it safe to unwrap here? Does this always return a username?
    // Seems like it but need to test more... dont want to panic
    debug!(
        "authenticated as {:?}",
        status
            .user_info
            .unwrap()
            .username
            .unwrap_or("(no username)".into())
    );

    // todo:? check what permissions we have and error if we are missing any

    return Ok(());
}

/// Returns K8S Client for selected profile
async fn client(profile: &config::ProfileConfig) -> Result<kube::Client> {
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
    return Ok(client);
}
