use crate::search::Source;
use color_eyre::eyre::Result;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use super::project_paths::{self, project_env_name};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UserConfig {
    pub(super) sources: Vec<Source>,
    pub(super) use_cache: bool,
    pub(super) auto_refresh_cache: bool,
    pub(super) cache_duration: std::time::Duration,
    pub(super) cache_dir: PathBuf,
    pub(super) enable_logging: bool,
    pub(super) log_level: String,
    pub(super) log_file: PathBuf,
}

impl Default for UserConfig {
    fn default() -> Self {
        // TODO: Set default sources
        Self {
            sources: Vec::default(),
            use_cache: true,
            auto_refresh_cache: true,
            cache_duration: Duration::from_secs(7 * 24 * 60 * 60),
            cache_dir: project_paths::default_cache_dir().clone(),
            enable_logging: true,
            log_level: String::from("error"),
            log_file: project_paths::default_log_file().clone(),
        }
    }
}
impl UserConfig {
    fn figment(config_file: &PathBuf) -> Figment {
        Figment::from(Serialized::defaults(UserConfig::default()))
            .merge(Toml::file(config_file))
            .merge(Env::prefixed(format!("{}_", project_env_name()).as_str()))
    }

    pub(super) fn build(custom_config_location: Option<PathBuf>) -> Result<Self> {
        let config_file =
            custom_config_location.unwrap_or_else(project_paths::default_config_file);
        Ok(Self::figment(&config_file).extract()?)
    }

    pub(crate) fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
