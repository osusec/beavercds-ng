use std::process::exit;

use crate::configparser::*;
use itertools::Itertools;
use simplelog::*;

pub fn run() {
    info!("validating config...");

    // attempt to parse configs but don't do anything with the results
    validate_and_return();

    info!("config is ok!")
}

pub fn validate_and_return() -> (RcdsConfig, Vec<ChallengeConfig>) {
    let config = match parse_rcds_config() {
        Ok(contents) => contents,
        Err(err) => {
            error!("{err:?}");
            // this should really be a Result imo :P - Zane
            exit(1);
        }
    };
    debug!("rcds config loaded");

    let (challenges, parse_errors): (Vec<_>, Vec<_>) =
        parse_all_challenges().into_iter().partition_result();

    debug!(
        "parsed {} chals, {} others failed parsing",
        challenges.len(),
        parse_errors.len()
    );

    if !parse_errors.is_empty() {
        for err in parse_errors.iter() {
            error!("{err:?}\n");
        }
        exit(1);
    }

    (config, challenges)
}
