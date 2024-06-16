use anyhow::{Error, Result};
use kube;

use crate::configparser::{config, CONFIG};

/// kubernetes access checks
pub fn check(profile: &config::ProfileConfig) -> Result<()> {
    // we need to make sure that:
    // a) can talk to the cluster
    // b) have the right permissions (a la `kubectl auth can-i`)

    // build a client
    let client = client(profile);

    return Ok(());
}

/// Returns K8S Client for selected profile
async fn client(profile: &config::ProfileConfig) -> Result<kube::Client> {
    // make sure the profile exists

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
