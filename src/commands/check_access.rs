use anyhow::{Context, Error, Result};
use itertools::{self, Itertools};
use simplelog::info;
use simplelog::*;
use std::process::exit;

use crate::access_handlers as access;
use crate::configparser::{get_config, get_profile_config};

pub fn run(profile: &str, kubernetes: &bool, frontend: &bool, registry: &bool) {
    // if user did not give a specific check, check all of them
    let check_all = !kubernetes && !frontend && !registry;

    let config = get_config().unwrap();

    let to_check: Vec<_> = match profile {
        "all" => config.profiles.keys().cloned().collect(),
        p => vec![String::from(p)],
    };

    let results: Result<()> = to_check.into_iter().try_for_each(|p| {
        check_profile(
            &p,
            *kubernetes || check_all,
            *frontend || check_all,
            *registry || check_all,
        )
    });

    // die if there were any errors
    match results {
        Ok(_) => info!("  all good!"),
        Err(err) => {
            error!("{err:#}");
            exit(1)
        }
    }
}

/// checks a single profile (`profile`) for the given accesses
fn check_profile(
    profile_name: &str,
    kubernetes: bool,
    frontend: bool,
    registry: bool,
) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    info!("checking profile {profile_name}...");

    // todo: this works but ehhh
    let mut results = vec![];

    if kubernetes {
        results.push(access::kube::check(profile));
    }
    if frontend {
        results.push(access::frontend::check(profile));
    }
    if registry {
        results.push(access::docker::check(profile));
    }

    // takes first Err in vec as Result() return
    results
        .into_iter()
        .collect::<Result<_>>()
        .with_context(|| format!("error in profile {profile_name}"))
}
