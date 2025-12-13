use std::sync::OnceLock;

use clap::Parser;
use color_eyre::eyre::Result;

mod app;
mod cli;
use cli::Cli;
mod cache;
mod config;
mod logging;
mod opt_display;
mod parsing;
mod search;
mod tui;

use config::AppConfig;

fn main() {
    let res = init_and_run();
    if let Err(e) = tui::restore() {
        eprintln!("{e:#?}");
    }
    if let Err(e) = res {
        eprintln!("{e:#?}");
    }
}

fn init_and_run() -> Result<()> {
    static CONFIG: OnceLock<AppConfig> = OnceLock::new();

    // This should essentially never error, but if it does, it's a non-critical error to the end user so we ignore it in release builds.
    let res = color_eyre::install();
    debug_assert!(matches!(res, Ok(())));

    let cli = Cli::parse();

    // Get a static config object to pass around references to. This is needed e.g. in search::new_searcher, where stuff like the cache configuration is used to construct a data_fn, a closure that gets passed to a new thread eventually
    let config = config::initialize(&cli)?;
    CONFIG.set(config).expect("Can set OnceCell once");
    let config = CONFIG.get().expect("Can get value of just-set OnceCell");

    logging::initialize(config)?;
    cache::initialize(config)?;

    cli.run(config)?;

    Ok(())
}
