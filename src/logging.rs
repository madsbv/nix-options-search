// Ratatui docs logging example, modified to use OnceLock instead of lazy_static
// https://ratatui.rs/how-to/develop-apps/log-with-tracing/
use crate::config::AppConfig;
use color_eyre::eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub fn initialize(config: &AppConfig) -> Result<()> {
    let Some(ref log_file_path) = config.log_file else {
        return Ok(());
    };
    if let Some(log_dir) = log_file_path.parent() {
        std::fs::create_dir_all(log_dir)?;
    }
    let log_file = std::fs::File::create(log_file_path)?;

    // Build an EnvFilter from the environment variable RUST_LOG if set, else from the loaded configuration.
    // The directives syntax: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_filter(filter);
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}
