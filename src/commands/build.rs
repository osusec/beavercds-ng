use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};

pub fn run(profile_name: &str, push: &bool, extract: &bool) {
    info!("building images...");

    let results = match build_challenges(profile_name, *push, *extract) {
        Ok(results) => results,
        Err(e) => {
            error!("{e:?}");
            exit(1)
        }
    };
    info!("images built successfully!");
}
