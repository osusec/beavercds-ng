use crate::configparser::{validate_challenges, validate_config};
use simplelog::*;
use std::process::exit;

pub fn run() {
    info!("validating config...");
    match validate_config() {
        Ok(_) => info!("  config ok!"),
        Err(err) => {
            error!("{err:#}");
            exit(1);
        }
    }

    info!("validating challenges...");
    match validate_challenges() {
        Ok(_) => info!("  challenges ok!"),
        Err(errors) => {
            for e in errors.iter() {
                error!("{e:#}");
            }
            exit(1);
        }
    }
}
