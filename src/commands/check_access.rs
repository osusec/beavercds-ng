use anyhow::{Context, Error, Result};
use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::access_handlers as access;
use crate::configparser::{get_config, get_profile_config};

pub fn run(profile: &str, kubernetes: &bool, frontend: &bool, registry: &bool, bucket: &bool) {
    // if user did not give a specific check, check all of them
    let check_all = !kubernetes && !frontend && !registry && !bucket;

    let config = get_config().unwrap();

    let profiles_to_check: Vec<_> = match profile {
        "all" => config.profiles.keys().cloned().collect(),
        p => vec![String::from(p)],
    };

    let results: Vec<_> = profiles_to_check
        .iter()
        .map(|profile_name| {
            (
                profile_name, // associate profile name to results
                check_profile(
                    profile_name,
                    *kubernetes || check_all,
                    *frontend || check_all,
                    *registry || check_all,
                    *bucket || check_all,
                ),
            )
        })
        .collect();

    debug!("access results: {results:?}");

    // die if there were any errors
    let mut should_exit = false;
    for (profile, result) in results.iter() {
        match result {
            Ok(_) => info!("  all good!"),
            Err(errs) => {
                error!("{} errors checking profile {profile}:", errs.len());
                errs.iter().for_each(|e| error!("{e:?}\n"));
                should_exit = true
            }
        }
    }
    if should_exit {
        exit(1);
    }
}

/// checks a single profile (`profile`) for the given accesses
fn check_profile(
    name: &str,
    kubernetes: bool,
    frontend: bool,
    registry: bool,
    bucket: bool,
) -> Result<(), Vec<Error>> {
    info!("checking profile {name}...");

    let mut errs = vec![];

    if kubernetes {
        match access::kube::check(name).context("could not access kubernetes cluster") {
            Err(e) => errs.push(e),
            Ok(_) => info!("  kubernetes ok!"),
        };
    }
    if frontend {
        match access::frontend::check(name).context("could not access frontend") {
            Err(e) => errs.push(e),
            Ok(_) => info!("  frontend ok!"),
        };
    }
    if registry {
        match access::docker::check(name).context("could not access container registry") {
            Err(e) => errs.push(e),
            Ok(_) => info!("  registry ok!"),
        };
    }
    if bucket {
        match access::s3::check(name).context("could not access asset bucket") {
            Err(e) => errs.push(e),
            Ok(_) => info!("  bucket ok!"),
        };
    }

    if !errs.is_empty() {
        Err(errs)
    } else {
        Ok(())
    }
}
