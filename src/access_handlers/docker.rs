use anyhow::{Error, Result};

use crate::configparser::{config, CONFIG};

/// container registry / daemon access checks
pub fn check(profile: &config::ProfileConfig) -> Result<()> {
    Ok(())
}
