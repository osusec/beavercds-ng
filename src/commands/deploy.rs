use itertools::Itertools;
use std::process::exit;
use tracing::{debug, error, info, trace, warn};

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
    let build_results = match build_challenges(profile_name, true, true).await {
        Ok(result) => result,
        Err(e) => {
            error!("{e:?}");
            exit(1);
        }
    };

    trace!(
        "got built results: {:#?}",
        build_results.iter().map(|b| &b.1).collect_vec()
    );

    // deploy needs to:
    // A) render kubernetes manifests
    //    - namespace, deployment, service, ingress
    //    - upgrade ingress config with new listen ports
    //
    // B) upload asset files to bucket
    //
    // C) update frontend with new state of challenges

    // A)
    if let Err(e) = deploy::kubernetes::deploy_challenges(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }

    // B)
    if let Err(e) = deploy::s3::upload_assets(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }

    // C)
    if let Err(e) = deploy::frontend::update_frontend(profile_name, &build_results).await {
        error!("{e:?}");
        exit(1);
    }
}
