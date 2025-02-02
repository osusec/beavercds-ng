use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str, push: &bool, extract: &bool) {
    info!("building images...");

    let results = match build_challenges(profile_name, *push, *extract).await {
        Ok(results) => results,
        Err(e) => {
            error!("{e:?}");
            exit(1)
        }
    };
    info!("images built successfully!");
}
