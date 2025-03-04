use anyhow::Result;
use itertools::Itertools;
use simplelog::*;
use std::process::exit;

use crate::builder::build_challenges;
use crate::configparser::{get_config, get_profile_config};

#[tokio::main(flavor = "current_thread")] // make this a sync function
pub async fn run(profile_name: &str, push: bool, extract: bool) -> Result<()> {
    info!("building images...");

    let results = build_challenges(profile_name, push, extract).await?;

    info!("images built successfully!");

    Ok(())
}
