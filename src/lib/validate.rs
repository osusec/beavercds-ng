use std::process::exit;

use crate::lib::configparser::*;
use itertools::Itertools;
use simplelog::*;

pub fn run() {
    debug!("running validate");

    let config: RCDSConfig = match parse_rcds_config() {
        Ok(contents) => contents,
        Err(err) => {
            error!("{err:?}");
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
            error!("{err:?}");
            println!();
        }
        exit(1);
    }
}
