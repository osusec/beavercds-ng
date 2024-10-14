use simplelog::*;
use std::process::exit;

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};

pub fn run(profile_name: &str, push: &bool) {
    info!("building images...");

    build_challenges(profile_name);
}
