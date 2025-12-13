use crate::cli::Cli;
use color_eyre::eyre::Result;

mod app_config;
pub(crate) mod consts;
mod project_paths;
mod user_config;
pub(crate) use app_config::AppConfig;
pub(crate) use project_paths::default_config_file;
pub(crate) use user_config::{default_config_toml, SourceConfig, UserConfig};

pub(crate) fn initialize(cli: &Cli) -> Result<AppConfig> {
    // Build user config from config file and possible environment variables
    let config_file = cli
        .config
        .clone()
        .unwrap_or_else(project_paths::default_config_file);
    let mut user_config = UserConfig::build(&config_file)?;

    // Override config with any given cli flags
    if let Some(log_file) = &cli.log_file {
        user_config.log_file.clone_from(log_file);
    }

    Ok(AppConfig::from(user_config))
}
