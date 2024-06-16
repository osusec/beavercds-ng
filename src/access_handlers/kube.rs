use anyhow::{Error, Result};

use crate::configparser::{config, CONFIG};

/// kubernetes access checks
pub fn check(profile: &config::ProfileConfig) -> Result<()> {
    // Ok(())
    Err(Error::msg("bad kube!"))
}
