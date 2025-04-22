use std::path::Path;
use std::process::exit;
use tracing::{debug, error, info, trace, warn};

use crate::configparser::{get_challenges, get_config, get_profile_deploy};

pub fn run() {
    info!("validating config...");
    let config = match get_config() {
        Ok(c) => c,
        Err(err) => {
            error!("{err:#}");
            exit(1);
        }
    };
    info!("  config ok!");

    info!("validating challenges...");
    let chals = match get_challenges() {
        Ok(c) => c,
        Err(errors) => {
            for e in errors.iter() {
                error!("{e:#}");
            }
            exit(1);
        }
    };
    info!("  challenges ok!");

    // check global deploy settings for invalid challenges
    info!("validating deploy config...");
    for (profile_name, _pconfig) in config.profiles.iter() {
        // fetch from config
        let deploy_challenges = match get_profile_deploy(profile_name) {
            Ok(d) => &d.challenges,
            Err(err) => {
                error!("{err:#}");
                exit(1);
            }
        };

        // check for missing
        let missing: Vec<_> = deploy_challenges
            .keys()
            .filter(
                // try to find any challenge paths in deploy config that do not exist
                |path| !chals.iter().any(|c| c.directory == Path::new(path)),
            )
            .collect();
        if !missing.is_empty() {
            error!(
                "Deploy settings for profile '{profile_name}' has challenges that do not exist:"
            );
            missing.iter().for_each(|path| error!("  - {path}"));
            exit(1)
        }
    }
    info!("  deploy ok!")
}
