use anyhow::{Context, Error, Result};
use itertools::Itertools;
use std::process::exit;
use tracing::{debug, error, info, trace, warn};

use crate::cluster_setup as setup;
use crate::configparser::{get_config, get_profile_config};

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str) {
    info!("setting up cluster...");
    let config = get_profile_config(profile_name).unwrap();

    if let Err(e) = setup::install_ingress(config).await {
        error!("{e:?}");
        exit(1);
    }
    if let Err(e) = setup::install_certmanager(config).await {
        error!("{e:?}");
        exit(1);
    }
    if let Err(e) = setup::install_extdns(config).await {
        error!("{e:?}");
        exit(1);
    }

    info!("charts deployed!")
}
