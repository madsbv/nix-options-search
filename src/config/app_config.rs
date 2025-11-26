use std::path::PathBuf;

use crate::search::Source;

use super::user_config::UserConfig;

#[derive(Debug)]
pub(crate) struct AppConfig {
    pub(crate) sources: Vec<Source>,
    pub(crate) cache_duration: Option<std::time::Duration>,
    pub(crate) cache_dir: Option<PathBuf>,
    pub(crate) log_level: String,
    pub(crate) log_file: Option<PathBuf>,
}

impl From<UserConfig> for AppConfig {
    fn from(value: UserConfig) -> Self {
        Self {
            sources: value.sources,
            cache_duration: if value.auto_refresh_cache {
                Some(value.cache_duration)
            } else {
                None
            },
            cache_dir: if value.use_cache {
                Some(value.cache_dir)
            } else {
                None
            },
            log_level: value.log_level,
            log_file: if value.enable_logging {
                Some(value.log_file)
            } else {
                None
            },
        }
    }
}
