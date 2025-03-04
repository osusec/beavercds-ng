use anyhow::Result;
use simplelog::*;
use std::process::exit;

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

    // deploy needs to:
    // A) render kubernetes manifests
    //    - namespace, deployment, service, ingress
    //    - upgrade ingress config with new listen ports
    //
    // B) upload asset files to bucket
    //
    // C) update frontend with new state of challenges

    // A)
    info!("deploying challenges...");
    deploy::kubernetes::deploy_challenges(profile_name, &build_results).await?;

    // B)
    info!("deploying challenges...");
    deploy::s3::upload_assets(profile_name, &build_results).await?;

    // A)
    info!("deploying challenges...");
    deploy::frontend::update_frontend(profile_name, &build_results).await?;

    Ok(())
}
