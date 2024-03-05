use crate::opt_display::OptDisplay;
use crate::search::{new_searcher, search_for, Source};
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Block, Position, Title},
        Borders, Paragraph,
    },
};
use std::cell::{self, RefCell};
use std::io;

// TODO: Implement tabs and tab switching.
pub struct App {
    search_string: String,
    // We need `RefCell` because `Nucleo` holds the pattern to search for as internal state, and doing a search requires `&mut Nucleo`. Using RefCell allows us to do the search at render-time, when we know how many results we'll need to populate the window.
    // Alternative: Split the searching step up into the reparse step and a finish step that actually outputs the results.
    pages: Vec<SearchPage>,
    active_page: usize,
    exit: bool,
}

impl App {
    pub fn new() -> App {
        App {
            search_string: String::new(),
            pages: vec![
                SearchPage::new(Source::NixDarwin),
                SearchPage::new(Source::NixOS),
                SearchPage::new(Source::HomeManager),
                SearchPage::new(Source::HomeManagerNixOS),
                SearchPage::new(Source::HomeManagerNixDarwin),
            ],
            active_page: 0,
            exit: false,
        }
    }

    fn get_matcher(&self) -> cell::RefMut<'_, nucleo::Nucleo<Vec<String>>> {
        assert!(self.active_page < self.pages.len());
        self.pages[self.active_page].get_matcher()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: Figure out how to make matchers lazy load
struct SearchPage {
    source: Source,
    matcher: RefCell<nucleo::Nucleo<Vec<String>>>,
}

impl SearchPage {
    fn new(source: Source) -> Self {
        SearchPage {
            source,
            matcher: new_searcher(source, true).into(),
        }
    }

    fn get_matcher(&self) -> cell::RefMut<'_, nucleo::Nucleo<Vec<String>>> {
        // Idea for lazy loading: Make self.matcher a RefCell<Option<...>>. On entering this method, check whether self.matcher is &None. If so, compute a matcher, and do a RefCell::Replace to set self.matcher to Some<matcher>. Then do borrow_mut().
        // Problem when I first tried that: How do you unwrap an Option behind a RefMut? I couldn't satisfy Rust.
        self.matcher.borrow_mut()
    }
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

        let results_outer_area = chunks[0];
        let results_inner_area = results_block.inner(results_outer_area);

        // Since `Layout` doesn't have a `block` method, we render it manually
        results_block.render(results_outer_area, buf);

        // TODO: Don't hard-code the height of an OptDisplay
        let opt_display_height = 4;
        // Also decide whether to round up or down
        let n_opts = results_inner_area.height as usize / opt_display_height;

        let results = search_for(&self.search_string, &mut self.get_matcher())
            .take(n_opts)
            .map(|v| OptDisplay::from_vec(v.clone()))
            .collect::<Vec<_>>();

        // TODO: Do something with the spacers?
        #[allow(clippy::cast_possible_truncation)]
        let (results_layout, _) = Layout::default()
            .direction(Direction::Vertical)
            .constraints(results.iter().map(|_| opt_display_height as u16))
            .margin(1)
            .split_with_spacers(results_inner_area); // Constraint implements from<u16>

        for (&rect, opt) in std::iter::zip(results_layout.iter(), results) {
            opt.render(rect, buf);
        }

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::THICK);

        let search_par = Paragraph::new(Text::from(self.search_string.clone().red()))
            .centered()
            .block(search_block);
        search_par.render(chunks[1], buf);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_key_event() {
        let mut app = App::new();
        // TODO: Get all the different matchers to make sure they're constructed correctly.

        app.handle_key_event(KeyCode::Char('w').into());
        assert_eq!(app.search_string, "w".to_string());

        assert!(!app.exit);
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }
}
