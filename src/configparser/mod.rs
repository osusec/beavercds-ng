pub mod challenge;
pub mod config;
pub mod field_coersion;

use anyhow::{anyhow, Error, Result};
pub use challenge::ChallengeConfig; // reexport
pub use config::UserPass; // reexport
use itertools::Itertools;
use std::path::Path;
use std::sync::OnceLock;
use tracing::{debug, error, info, trace, warn};

// todo: replace with std::LazyLock once v1.80 is out?
pub static CONFIG: OnceLock<config::RcdsConfig> = OnceLock::new();
pub static CHALLENGES: OnceLock<Vec<challenge::ChallengeConfig>> = OnceLock::new();
// type aliases for above's lifetimes
pub type RcdsConfig = &'static config::RcdsConfig;
pub type ChallengeConfigs = &'static Vec<challenge::ChallengeConfig>;

/// get config from global, or load from file if not parsed yet
pub fn get_config() -> Result<RcdsConfig> {
    // return already parsed value
    if let Some(existing) = CONFIG.get() {
        return Ok(existing);
    }

    let config = config::parse();

    // if config parsed OK, set global and return that
    // otherwise pass through the errors from parsing
    config.map(|c| CONFIG.get_or_init(|| c))
}

/// Get config struct for the passed profile name
pub fn get_profile_config(profile_name: &str) -> Result<&config::ProfileConfig> {
    get_config()?
        .profiles
        .get(profile_name)
        .ok_or(anyhow!("profile {profile_name} not found in config"))
}
/// Get challenge deploy config struct for the passed profile name
pub fn get_profile_deploy(profile_name: &str) -> Result<&config::ProfileDeploy> {
    get_config()?
        .deploy
        .get(profile_name)
        .ok_or(anyhow!("profile {profile_name} not found in deploy config"))
}

/// get challenges from global, or load from files if not parsed yet
pub fn get_challenges() -> Result<ChallengeConfigs, Vec<Error>> {
    // return already parsed value
    if let Some(existing) = CHALLENGES.get() {
        return Ok(existing);
    }

    let chals = challenge::parse_all();

    chals.map(|c| CHALLENGES.get_or_init(|| c))
}

/// Get all enabled challenges for profile
pub fn enabled_challenges(profile_name: &str) -> Result<Vec<&ChallengeConfig>> {
    let config = get_config()?;
    let challenges = get_challenges().unwrap();
    let deploy = &get_profile_deploy(profile_name)?.challenges;

    let enabled = deploy
        .iter()
        .filter_map(|(chal_path, enabled)| match enabled {
            true => challenges
                .iter()
                .find(|c| c.directory == Path::new(chal_path)),
            false => None,
        })
        .collect();

    Ok(enabled)
}
