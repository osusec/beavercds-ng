use anyhow::{Error, Result};

use crate::configparser::{get_config, get_profile_config};

/// frontend dashbard access checks
pub fn check(profile_name: &str) -> Result<()> {
    let profile = get_profile_config(profile_name)?;
    Ok(())
}
