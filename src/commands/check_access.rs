use anyhow::Result;
use simplelog::info;
use simplelog::*;
use std::process::exit;

use crate::access_handlers as access;
use crate::configparser::get_config;

pub fn run(kubernetes: bool, frontend: bool, registry: bool) {
    // if user did not give a specific check, check all of them
    let check_all = !kubernetes && !frontend && !registry;

    let config = get_config().unwrap();

    let mut problem = false;

    for (profile, prof_config) in config.profiles.iter() {
        info!("checking profile {profile}:");

        if kubernetes || check_all {
            info!("  kubernetes...");

            // access::kube::check().unwrap_or_else(|err| error!("{err:?}"));
            match access::kube::check() {
                Ok(_) => info!("  ok!"),
                Err(err) => {
                    error!("{err:?}");
                    problem = true;
                }
            }
            // exit(1);
        }

        if frontend || check_all {
            info!("  frontend...");
            match check_frontend() {
                Ok(_) => info!("  ok!"),
                Err(err) => {
                    error!("{err:?}");
                    problem = true;
                }
            }
        }

        if registry || check_all {
            info!("  container registry...");
            match check_registry() {
                Ok(_) => info!("  ok!"),
                Err(err) => {
                    error!("{err:?}");
                    problem = true;
                }
            }
        }
    }

    if problem {
        exit(1);
    }
}

fn check_frontend() -> Result<()> {
    Ok(())
}

fn check_registry() -> Result<()> {
    Ok(())
}
