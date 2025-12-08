use clap::Parser;
use color_eyre::eyre::Result;
use tracing::debug;

mod app;
mod project_paths;
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
    color_eyre::install()?;
    config::Config::set(None::<figment::providers::Serialized<()>>)?;
    logging::initialize()?;
    cache::initialize()?;

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        cmd.run()?;
    } else {
        debug!("Application started");
        let mut terminal = tui::init()?;
        App::new().run(&mut terminal)?;
    }

    Ok(())
}
