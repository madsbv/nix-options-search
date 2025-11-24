use crate::{
    project_paths::{self, project_name},
    search::Source,
};
use color_eyre::eyre::{OptionExt, Result};
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Config {
    pub(crate) sources: Vec<Source>,
    pub(crate) use_cache: bool,
    pub(crate) cache_duration: std::time::Duration,
    pub(crate) cache_dir: Option<PathBuf>,
    pub(crate) log_level: String, // Probably not the right type, will dig that out later
    pub(crate) log_file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        // TODO: Set defaults
        Self {
            sources: Vec::default(),
            use_cache: true,
            cache_duration: Duration::default(),
            cache_dir: None,
            log_level: String::default(),
            log_file: Some(project_paths::log_file().clone()),
        }
    }
}
impl Config {
    fn figment() -> Figment {
        Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(project_paths::config_file()))
            .merge(Env::prefixed(format!("{}_", project_name()).as_str()))
    }

    pub(crate) fn get() -> Result<&'static Config> {
        CONFIG.get().ok_or_eyre("Config not defined")
    }

    pub(crate) fn set(extra: Option<impl figment::Provider>) -> Result<()> {
        CONFIG
            .set(
                match extra {
                    Some(extra) => Figment::from(Config::figment()).merge(extra),
                    None => Figment::from(Config::figment()),
                }
                .extract()?,
            )
            .map_err(|config| color_eyre::eyre::eyre!("Unable to set config: {config:?}"))
    }

    pub(crate) fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
