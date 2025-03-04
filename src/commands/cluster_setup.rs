use anyhow::{Context, Error, Result};
use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::cluster_setup as setup;
use crate::configparser::{get_config, get_profile_config};

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str) -> Result<()> {
    info!("setting up cluster...");
    let config = get_profile_config(profile_name).unwrap();

    setup::install_ingress(config).await?;
    setup::install_certmanager(config).await?;
    setup::install_extdns(config).await?;

    info!("charts deployed!");

    Ok(())
}
