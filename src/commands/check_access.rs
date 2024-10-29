use anyhow::{Context, Error, Result};
use itertools::Itertools;
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

    let results: Result<(), Vec<_>> = to_check.into_iter().try_for_each(|p| {
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
        Err(errs) => {
            error!("Error checking profile {profile}:");
            errs.iter().for_each(|e| error!("{e:?}\n"));
            exit(1)
        }
    }
}

/// checks a single profile (`profile`) for the given accesses
fn check_profile(
    name: &str,
    kubernetes: bool,
    frontend: bool,
    registry: bool,
) -> Result<(), Vec<Error>> {
    info!("checking profile {name}...");

    let mut results = vec![];

    if kubernetes {
        results.push(access::kube::check(name).context("could not access kubernetes cluster"));
    }
    if frontend {
        results.push(access::frontend::check(name).context("could not access frontend"));
    }
    if registry {
        results.push(access::docker::check(name).context("could not access container registry"));
    }

    let (ok, errs): (Vec<_>, Vec<_>) = results.into_iter().partition_result();

    if !errs.is_empty() {
        Err(errs)
    } else {
        Ok(())
    }
}
