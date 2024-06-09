use anyhow::{Context, Error, Result};
use itertools::{self, Itertools};
use simplelog::info;
use simplelog::*;
use std::process::exit;

use crate::access_handlers as access;
use crate::configparser::get_config;

pub fn run(profile: &str, kubernetes: &bool, frontend: &bool, registry: &bool) {
    // if user did not give a specific check, check all of them
    let check_all = !kubernetes && !frontend && !registry;

    let config = get_config().unwrap();

    let to_check: Vec<_> = match profile {
        "all" => config.profiles.keys().cloned().collect(),
        p => vec![String::from(p)],
    };

    let results: Result<()> = to_check
        .into_iter()
        .map(|p| {
            check_profile(
                &p,
                *kubernetes || check_all,
                *frontend || check_all,
                *registry || check_all,
            )
        })
        .collect();

    // die if there were any errors
    match results {
        Ok(_) => info!("  all good!"),
        Err(err) => error!("{err:#}"),
    }
}

/// checks a single profile (`profile`) for the given accesses
fn check_profile(profile: &str, kubernetes: bool, frontend: bool, registry: bool) -> Result<()> {
    info!("checking profile {profile}...");

    // todo: this works but ehhh
    let mut results = vec![];

    if kubernetes {
        results.push(access::kube::check());
    }

    if frontend {
        results.push(access::frontend::check());
    }

    if registry {
        results.push(access::docker::check());
    }

    // takes first Err in vec as Result() return
    results
        .into_iter()
        .collect::<Result<()>>()
        .with_context(|| format!("bad config for profile"))
}
