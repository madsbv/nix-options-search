// Ratatui docs logging example, modified to use OnceLock instead of lazy_static
// https://ratatui.rs/how-to/develop-apps/log-with-tracing/
use color_eyre::eyre::Result;
use directories::ProjectDirs;
use std::path::PathBuf;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, Layer, layer::SubscriberExt, util::SubscriberInitExt};

pub fn project_name() -> String {
    env!("CARGO_CRATE_NAME").to_uppercase()
}

/// Cache location is determined by the first found option of:
/// - the environment variable `NIX_OPTIONS_SEARCH_CACHE`,
/// - The OS standard cache directory (usually in `$HOME/.cache/` on Linux, `$HOME/Library/Caches/` on Mac),
///
/// If neither is found, the cache will be placed in the subdirectory `.cache` in the current directory, which will be created if it does not exist.
pub fn cache_dir() -> PathBuf {
    let dir = std::env::var(format!("{}_CACHE", project_name()))
        .ok()
        .map(PathBuf::from);
    if let Some(s) = dir {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.cache_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".cache")
    }
}

/// Log location is determined by the first found option of:
/// - the environment variable `NIX_OPTIONS_SEARCH_LOG`,
/// - The OS standard data directory (usually in `$HOME/.local/share/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, the log will be placed in the subdirectory `.log` in the current directory, which will be created if it does not exist.
pub fn log_file_path() -> PathBuf {
    let dir = std::env::var(format!("{}_LOG", project_name()))
        .ok()
        .map(PathBuf::from);
    if let Some(s) = dir {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.data_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".log")
    }
    .join(format!("{}.log", env!("CARGO_PKG_NAME")))
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "mvil", env!("CARGO_PKG_NAME"))
}

pub fn initialize() -> Result<()> {
    std::fs::create_dir_all(cache_dir())?;

    if let Some(log_dir) = log_file_path().parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    let log_file = std::fs::File::create(log_file_path())?;

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}
