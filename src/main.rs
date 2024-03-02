#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::similar_names
)]

#[allow(unused_imports)]
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::*, Borders, Paragraph},
};
use std::io;

mod opt_data;
mod search;
#[allow(unused_imports)]
use search::{nix_darwin_searcher, nix_darwin_searcher_from_cache, search_for};

mod tui;

fn main() -> Result<()> {
    let mut terminal = tui::init()?;

    let mut app = init_darwin_app(false)?;

    let _ = app.run(&mut terminal);

    tui::restore()
}

// TODO: This will probably be renamed to SearchPage whenever we add nixOS/home-manager support as well.
// TODO: Lifetimes so search_results can just point to the contents of the matcher
pub struct App {
    search_string: String,
    search_results: Vec<Vec<String>>,
    matcher: nucleo::Nucleo<Vec<String>>,
    exit: bool,
}

fn init_darwin_app(use_cache: bool) -> Result<App> {
    let matcher = if use_cache {
        nix_darwin_searcher_from_cache()?
    } else {
        nix_darwin_searcher()?
    };
    Ok(App {
        search_string: String::new(),
        search_results: vec![],
        matcher,
        exit: false,
    })
}

impl App {
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.render_frame(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.size());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // Blocks until a key is pressed
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key_event(key),
            _ => {}
        };
        // TODO: Test whether we need to have concurrency involved here.
        self.search_results = search_for(&self.search_string, &mut self.matcher, 1000)
            .into_iter()
            .map(|item| item.data.clone()) // TODO: Eliminate clone?
            .collect();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char(c) => self.search_string.push(c),
            // TODO: Handle things like backspace, and some way to quit
            _ => {}
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title = Title::from(" Nix-darwin options search ".bold());
        let instructions = Title::from(Line::from(vec![" Quit ".into(), "<Q>".blue().bold()]));
        let block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK);

        let results = Text::from(vec![Line::from(
            self.search_results
                .iter()
                .map(|s| s.clone().join(" ").yellow()) // TODO: Better presentation; have the fields next to each other in blocks
                .collect::<Vec<_>>(),
        )]);

        Paragraph::new(results)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_key_event() {
        let mut app =
            init_darwin_app(true).expect("we can initialize an app from the cached index.html");

        app.handle_key_event(KeyCode::Char('w').into());
        assert_eq!(app.search_string, "w".to_string());

        assert!(!app.exit);
        app.handle_key_event(KeyCode::Char('q').into());
        assert!(app.exit);
    }
}
