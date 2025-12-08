use crate::project_paths::cache_dir;
use color_eyre::eyre::Result;

pub(crate) fn initialize() -> Result<()> {
    std::fs::create_dir_all(cache_dir())?;
    Ok(())
}

pub(crate) fn delete_cache_dir() -> Result<()> {
    std::fs::remove_dir_all(cache_dir())?;
    Ok(())
}
