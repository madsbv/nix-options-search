#![warn(clippy::all, clippy::pedantic)]
#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::similar_names)]

use color_eyre::eyre::Result;

mod app;
use app::App;
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
    let mut terminal = tui::init()?;

    App::new().run(&mut terminal)?;
    Ok(())
}
