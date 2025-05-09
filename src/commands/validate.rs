use anyhow::{bail, Result};
use std::path::Path;
use std::process::exit;
use tracing::{debug, error, info, trace, warn};

use crate::configparser::{get_challenges, get_config, get_profile_deploy};

pub fn run() -> Result<()> {
    info!("validating config...");

    let config = get_config()?;
    info!("  config ok!");

    info!("validating challenges...");
    // print these errors here instead of returning, since its a vec of them
    // TODO: figure out how to return this error directly
    let chals = match get_challenges() {
        Ok(c) => c,
        Err(errors) => {
            for e in errors.iter() {
                error!("{e:#}");
            }
            bail!("failed to validate challenges");
        }
    };
    info!("  challenges ok!");

    // check global deploy settings for invalid challenges
    info!("validating deploy config...");
    for (profile_name, _pconfig) in config.profiles.iter() {
        // fetch from config
        let deploy_challenges = get_profile_deploy(profile_name)?;

        // check for missing
        let missing: Vec<_> = deploy_challenges
            .challenges
            .keys()
            .filter(
                // try to find any challenge paths in deploy config that do not exist
                |path| !chals.iter().any(|c| c.directory == Path::new(path)),
            )
            .collect();

        // TODO: figure out how to return this error directly
        if !missing.is_empty() {
            error!(
                "Deploy settings for profile '{profile_name}' has challenges that do not exist:"
            );
            missing.iter().for_each(|path| error!("  - {path}"));
            bail!("failed to validate deploy config");
        }
    }
    info!("  deploy ok!");

    Ok(())
}
