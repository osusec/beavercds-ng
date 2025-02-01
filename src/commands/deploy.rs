use simplelog::*;
use std::process::exit;

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};
use crate::deploy;

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str, no_build: &bool, _dry_run: &bool) {
    let profile = get_profile_config(profile_name).unwrap();

    // has the cluster been setup?
    if let Err(e) = deploy::check_setup(profile).await {
        error!("{e:?}");
        exit(1);
    }

    // build before deploying
    if *no_build {
        warn!("");
        warn!("Not building before deploying! are you sure this is a good idea?");
        warn!("");
    }

    info!("building challenges...");
    let build_results = match build_challenges(profile_name, true, true) {
        Ok(result) => result,
        Err(e) => {
            error!("{e:?}");
            exit(1);
        }
    };

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
    if let Err(e) = deploy::kubernetes::deploy_challenges(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }

    // B)
    info!("deploying challenges...");
    if let Err(e) = deploy::s3::upload_assets(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }

    // A)
    info!("deploying challenges...");
    if let Err(e) = deploy::frontend::update_frontend(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }
}
