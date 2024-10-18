use simplelog::*;
use std::process::exit;

use crate::builder::{build_challenges, push_tags};
use crate::configparser::{get_config, get_profile_config};

pub fn run(profile_name: &str, push: &bool) {
    info!("building images...");

    let tags = match build_challenges(profile_name) {
        Ok(tags) => tags,
        Err(e) => {
            error!("{e:?}");
            exit(1)
        }
    };
    info!("images built successfully!");

    if *push {
        info!("pushing images...");

        match push_tags(tags) {
            Ok(_) => info!("images pushed successfully!"),
            Err(e) => {
                error!("{e:?}");
                exit(1)
            }
        }
    };
}
