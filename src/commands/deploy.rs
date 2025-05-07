use anyhow::{Context, Result};
use itertools::Itertools;
use std::process::exit;
use tracing::{debug, error, info, trace, warn};

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};
use crate::deploy;

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str, no_build: &bool, _dry_run: &bool) -> Result<()> {
    let profile = get_profile_config(profile_name).unwrap();

    // has the cluster been setup?
    deploy::check_setup(profile).await?;

    // build before deploying
    if *no_build {
        warn!("");
        warn!("Not building before deploying! are you sure this is a good idea?");
        warn!("");
    }

    info!("building challenges...");
    let build_results = build_challenges(profile_name, true, true).await?;

    trace!(
        "got built results: {:#?}",
        build_results.iter().map(|b| &b.1).collect_vec()
    );

    deploy::deploy_challenges(profile_name, &build_results)
        .await
        .context("could not deploy challenges")?;

    Ok(())
}
