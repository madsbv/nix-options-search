// Ratatui docs logging example, modified to use OnceLock instead of lazy_static
// https://ratatui.rs/how-to/develop-apps/log-with-tracing/
use color_eyre::eyre::Result;
use directories::ProjectDirs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub static PROJECT_NAME: OnceLock<String> = OnceLock::new();
pub static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static LOG_ENV: OnceLock<String> = OnceLock::new();
pub static LOG_FILE: OnceLock<PathBuf> = OnceLock::new();

pub fn project_name() -> &'static str {
    PROJECT_NAME.get_or_init(|| env!("CARGO_CRATE_NAME").to_uppercase().to_string())
}

/// Cache location is determined by the first found option of:
/// - the environment variable `NIX_OPTIONS_SEARCH_CACHE`,
/// - The OS standard cache directory (usually in `$HOME/.cache/` on Linux, `$HOME/Library/Caches/` on Mac),
///
/// If neither is found, the cache will be placed in the subdirectory `.cache` in the current directory, which will be created if it does not exist.
pub fn cache_dir() -> &'static PathBuf {
    CACHE_DIR.get_or_init(|| {
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
    })
}
pub fn log_env() -> &'static str {
    LOG_ENV.get_or_init(|| format!("{}_LOGLEVEL", project_name()))
}

/// Log location is determined by the first found option of:
/// - the environment variable `NIX_OPTIONS_SEARCH_LOG`,
/// - The OS standard data directory (usually in `$HOME/.local/share/` on Linux, `$HOME/Library/Application Support/` on Mac),
///
/// If neither is found, the log will be placed in the subdirectory `.log` in the current directory, which will be created if it does not exist.
pub fn log_file_path() -> &'static PathBuf {
    LOG_FILE.get_or_init(|| {
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
    })
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

    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG")
            .or_else(|_| std::env::var(log_env()))
            .unwrap_or_else(|_| format!("{}=info", env!("CARGO_CRATE_NAME"))),
    );

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
