use crate::cli::Cli;
use color_eyre::eyre::{eyre, Result};
use std::sync::OnceLock;

mod app_config;
pub(crate) mod consts;
mod project_paths;
mod user_config;
pub(crate) use app_config::AppConfig;
pub(crate) use project_paths::default_config_file;
pub(crate) use user_config::{SourceConfig, UserConfig};

/// The final source of truth on configurable aspects of the program.
pub(crate) static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub(crate) fn initialize(cli: &Cli) -> Result<()> {
    // Build user config from config file and possible environment variables
    let mut user_config = UserConfig::build(cli.config.clone())?;

    // Override config with any given cli flags
    if let Some(log_file) = &cli.log_file {
        user_config.log_file.clone_from(log_file);
    }

    // Set AppConfig
    CONFIG.set(AppConfig::from(user_config)).map_err(|err| {
        eyre!(
            "Loading configuration failed because the following AppConfig was already set: {err:?}"
        )
    })
}
