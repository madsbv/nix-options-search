#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

use anyhow::Result;

mod app;
mod opt_data;
mod opt_display;
mod search;
mod tui;

fn main() -> Result<()> {
    let mut terminal = tui::init()?;

    let mut app = app::darwin()?;

    let _ = app.run(&mut terminal);

    tui::restore()
}
