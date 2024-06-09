use anyhow::{Error, Result};

/// kubernetes access checks
pub fn check() -> Result<()> {
    // Ok(())
    Err(Error::msg("bad kube!"))
}
