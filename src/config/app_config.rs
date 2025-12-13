use super::{
    project_paths::{default_cache_dir, default_log_file},
    user_config::UserConfig,
    SourceConfig,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct AppConfig {
    #[allow(dead_code)]
    pub(crate) sources: Vec<SourceConfig>,
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

impl From<AppConfig> for UserConfig {
    fn from(value: AppConfig) -> Self {
        Self {
            sources: value.sources,
            use_cache: value.cache_dir.is_some(),
            auto_refresh_cache: value.cache_duration.is_some(),
            cache_duration: value.cache_duration.unwrap_or_default(),
            cache_dir: value.cache_dir.unwrap_or_else(default_cache_dir),
            enable_logging: value.log_file.is_some(),
            log_level: value.log_level,
            log_file: value.log_file.unwrap_or_else(default_log_file),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        UserConfig::default().into()
    }
}
