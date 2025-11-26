use clap::Parser;
use color_eyre::eyre::Result;
use tracing::debug;

mod app;
use app::App;
mod cli;
use cli::Cli;
mod cache;
mod config;
mod logging;
mod opt_data;
mod opt_display;
mod search;
mod tui;

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
    // This should essentially never error, but if it does, it's a non-critical error to the end user so we ignore it in release builds.
    let res = color_eyre::install();
    debug_assert!(matches!(res, Ok(())));

    let cli = Cli::parse();

    config::initialize(&cli)?;
    logging::initialize()?;
    cache::initialize()?;

    if let Some(ref cmd) = cli.command {
        cmd.run(&cli)?;
    } else {
        debug!("Application started");
        let mut terminal = tui::init()?;
        App::new().run(&mut terminal)?;
    }

    Ok(())
}
