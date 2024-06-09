use anyhow::{Error, Result};

use crate::configparser;

/// kubernetes access checks
pub fn check() -> Result<()> {
    // Ok(())
    Err(Error::msg("bad kube!"))
}
