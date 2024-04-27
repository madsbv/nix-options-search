// Ratatui docs logging example, modified to use OnceLock instead of lazy_static
// https://ratatui.rs/how-to/develop-apps/log-with-tracing/
use std::path::PathBuf;
use std::sync::OnceLock;

use color_eyre::eyre::Result;
use directories::ProjectDirs;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt, Layer};

pub static PROJECT_NAME: OnceLock<String> = OnceLock::new();
pub static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
pub static LOG_ENV: OnceLock<String> = OnceLock::new();
pub static LOG_FILE: OnceLock<String> = OnceLock::new();

pub fn project_name() -> &'static str {
    PROJECT_NAME.get_or_init(|| env!("CARGO_CRATE_NAME").to_uppercase().to_string())
}
pub fn data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = std::env::var(format!("{}_DATA", project_name()))
            .ok()
            .map(PathBuf::from);
        if let Some(s) = dir {
            s
        } else if let Some(proj_dirs) = project_directory() {
            proj_dirs.data_local_dir().to_path_buf()
        } else {
            PathBuf::from(".").join(".data")
        }
    })
}
pub fn log_env() -> &'static str {
    LOG_ENV.get_or_init(|| format!("{}_LOGLEVEL", project_name()))
}
pub fn log_file() -> &'static str {
    LOG_FILE.get_or_init(|| format!("{}.log", env!("CARGO_PKG_NAME")))
}

fn project_directory() -> Option<ProjectDirs> {
    ProjectDirs::from("dev", "mvil", env!("CARGO_PKG_NAME"))
}

pub fn initialize() -> Result<()> {
    let directory = data_dir();
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(log_file());
    let log_file = std::fs::File::create(log_path)?;
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
