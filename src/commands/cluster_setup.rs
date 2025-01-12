use anyhow::{Context, Error, Result};
use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::cluster_setup as setup;
use crate::configparser::{get_config, get_profile_config};

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str) {
    info!("setting up cluster...");
    let config = get_profile_config(profile_name).unwrap();

    if let Err(e) = setup::deploy_helm_controller(config).await {
        error!("{e:?}");
        exit(1);
    }

    // once helm types are installed, install the other charts
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
}
