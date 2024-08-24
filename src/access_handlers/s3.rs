use anyhow::{Error, Result};

use crate::configparser::{get_config, get_profile_config};

/// s3 bucket access checks
pub fn check(profile_name: &str) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    Ok(())
}
