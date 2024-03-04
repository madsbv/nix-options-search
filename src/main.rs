#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

use color_eyre::eyre::Result;

mod app;
mod opt_data;
mod opt_display;
mod search;
mod tui;

fn main() -> Result<()> {
    color_eyre::install()?;
    let mut terminal = tui::init()?;

    let mut app = app::darwin()?;

    let _ = app.run(&mut terminal);

    tui::restore()
}
