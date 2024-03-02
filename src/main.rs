#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(clippy::missing_errors_doc, clippy::similar_names)]

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Block, Position, Title},
        Borders, Paragraph, Row, Table,
    },
};
use std::io;

mod opt_data;
mod search;
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
    // The best matching result is first in the list
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
        // TODO: Arguably we should try to limit the number of matches we take. Can we access the size of the frame somehow, or should we just choose some reasonably small number?
        self.search_results = search_for(&self.search_string, &mut self.matcher, 10)
            .into_iter()
            .map(|item| item.data.clone()) // TODO: Eliminate clone?
            .collect();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.search_string.push(c),
            KeyCode::Backspace => {
                self.search_string.pop();
            }
            KeyCode::Esc => self.exit = true,
            _ => {}
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);

        let title = Title::from(" Nix-darwin options search ".bold());
        let instructions = Title::from(Line::from(vec![" Quit ".into(), "<Esc> ".yellow().bold()]));
        let results_block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK);

        // TODO: Split each result into multiple rows, or maybe construct a custom layout entirely
        let rows = self.search_results.clone().into_iter().map(Row::new);

        let widths = [
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
        ];

        let results_table = Table::new(rows, widths)
            .column_spacing(1)
            .header(
                Row::new(vec![
                    "Name",
                    "Description",
                    "Type",
                    "Default",
                    "Example",
                    "Declared by",
                ])
                .bottom_margin(1),
            )
            .block(results_block);

        // Table is also a StatefulWidget, so results_table.render() is ambiguous
        ratatui::widgets::Widget::render(results_table, chunks[0], buf);

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::THICK);

        let search_par =
            Paragraph::new(Text::from(self.search_string.clone().red())).block(search_block);
        search_par.render(chunks[1], buf);
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
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }
}
