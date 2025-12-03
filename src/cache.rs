use crate::config::AppConfig;
use color_eyre::eyre::Result;

pub(crate) fn initialize(config: &AppConfig) -> Result<()> {
    if let Some(dir) = &config.cache_dir {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
