use anyhow::{anyhow, Error, Result};
use k8s_openapi::api::{
    authentication::v1::SelfSubjectReview, authorization::v1::SelfSubjectAccessReview,
};
use k8s_openapi::serde_json::{from_value, json, to_string};
use kube;
use simplelog::*;
use tokio;

use crate::clients::kube_client;
use crate::configparser::{config, get_config, get_profile_config};

/// kubernetes access checks
#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn check(profile_name: &str) -> Result<()> {
    let profile = get_profile_config(profile_name)?;

    // we need to make sure that:
    // a) can talk to the cluster
    // b) have the right permissions (a la `kubectl auth can-i`)

    // build a client
    let client = kube_client(profile).await?;

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
    let status = resp.status.ok_or(anyhow!("Could not access cluster"))?;

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

    Ok(())
}
