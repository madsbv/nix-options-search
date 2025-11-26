use crate::config::CONFIG;
use color_eyre::eyre::Result;

pub(crate) fn initialize() -> Result<()> {
    if let Some(dir) = &CONFIG.wait().cache_dir {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub(crate) fn delete_cache_dir() -> Result<()> {
    if let Some(dir) = &CONFIG.wait().cache_dir {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}
