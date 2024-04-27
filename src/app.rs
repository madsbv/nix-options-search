use crate::opt_data::OptText;
use crate::opt_display::ListableOptWidget;
use crate::search::{Finder, InputStatus, Source};
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::Padding;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Block, Position, Title},
        Borders, Paragraph, Tabs,
    },
};
use std::io;
use std::time::Duration;
use tui_widget_list::{List, ListState};

// XXX: Optimization idea: Have a "results cache stack" where, each time search_string is appended to, we push the current search results; and when Backspace is pressed, instead of re-searching we just pop the stack. On tab change, we have to clear the stack. Might not be worth it.
pub struct App {
    search_string: String,
    pages: Vec<Finder>,
    // An integer in `0..self.pages.len()`
    active_page: usize,
    // To use Nucleo's append optimization and avoid reparsing if pattern hasn't changed
    input_status: InputStatus,
    exit: bool,
}

impl App {
    pub fn new() -> App {
        App {
            search_string: String::new(),
            pages: vec![
                Finder::new(Source::NixDarwin),
                Finder::new(Source::NixOS),
                Finder::new(Source::HomeManager),
                Finder::new(Source::HomeManagerNixOS),
                Finder::new(Source::HomeManagerNixDarwin),
            ],
            active_page: 0,
            input_status: InputStatus::Change,
            exit: false,
        }
    }

    fn init_search(&mut self) {
        assert!(self.active_page < self.pages.len());
        self.pages[self.active_page].init_search(&self.search_string, self.input_status);
        self.input_status = InputStatus::Unchanged;
    }

    fn get_results(&self, max: Option<usize>) -> Vec<OptText> {
        assert!(self.active_page < self.pages.len());
        self.pages[self.active_page].get_results(max)
    }

    // For testing
    #[allow(dead_code)]
    fn search_blocking(
        &mut self,
        max: Option<usize>,
    ) -> std::result::Result<Vec<OptText>, Box<(dyn std::any::Any + Send + 'static)>> {
        assert!(self.active_page < self.pages.len());
        self.pages[self.active_page].find_blocking(&self.search_string, max)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
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
        // If recv sees that results are waiting while we're waiting for user input, return early and render the pending results.
        // NOTE: Semantically, this should really be a `select!` statement in async context.
        // This polling does take an appreciable amount of CPU time.
        while let Ok(false) = event::poll(Duration::from_millis(500)) {
            if self.pages[self.active_page]
                .results_waiting
                .try_recv()
                .is_ok()
            {
                self.init_search();
                return Ok(());
            }
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_key_event(key),
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c) => {
                self.search_string.push(c);
                self.input_status = InputStatus::Append;
            }
            KeyCode::Backspace => {
                self.search_string.pop();
                self.input_status = InputStatus::Change;
            }
            KeyCode::Right => {
                if self.active_page + 1 < self.pages.len() {
                    self.active_page += 1;
                    self.input_status = InputStatus::Change;
                }
            }
            KeyCode::Left => {
                if self.active_page > 0 {
                    self.active_page -= 1;
                    self.input_status = InputStatus::Change;
                }
            }
            KeyCode::Esc => self.exit = true,
            _ => {}
        }
        self.init_search();
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
        // TODO: Consider splitting out the rendering of each section in individual functions/widgets, or just organize code better
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        // TODO: Styling
        let width_of_tabs_widget: usize =
            self.pages.iter().map(|p| p.name().len()).sum::<usize>() + self.pages.len() * 3 + 1;
        let tabs_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                #[allow(clippy::cast_possible_truncation)]
                Constraint::Length(width_of_tabs_widget as u16),
                Constraint::Min(0),
            ])
            .split(chunks[0]);
        let tabs = Tabs::new(self.pages.iter().map(Finder::name).collect::<Vec<_>>())
            .block(Block::default().title("Tabs").borders(Borders::ALL))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(self.active_page)
            // .divider(symbols::DOT)
            .padding(" ", " ");

        tabs.render(tabs_layout[1], buf);

        let title = Title::from(format!(" {} ", self.pages[self.active_page].name()).bold());
        let instructions = Title::from(Line::from(vec![
            " Change tabs: ".into(),
            "<Left>/<Right>, ".yellow().bold(),
            "Quit: ".into(),
            "<Esc> ".yellow().bold(),
        ]));

        let results_block = Block::default()
            .title(title.alignment(Alignment::Center))
            .title(
                instructions
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::ALL)
            .border_set(border::THICK)
            .padding(Padding::horizontal(1));

        let results_area = chunks[1];

        let results_list = List::new(
            self.get_results(None)
                .into_iter()
                .map(ListableOptWidget::from)
                .collect(),
        )
        .block(results_block);
        // I probably have to keep this around somewhere.
        let mut state = ListState::default();
        results_list.render(results_area, buf, &mut state);

        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::THICK);

        let search_par = Paragraph::new(Text::from(self.search_string.clone().red()))
            .centered()
            .block(search_block);
        search_par.render(chunks[2], buf);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_search_string() {
        let mut app = App::new();

        app.handle_key_event(KeyCode::Char('w').into());
        assert_eq!(app.search_string, "w".to_string());

        assert!(!app.exit);
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }

    #[test]
    fn switch_tabs() {
        let mut app = App::new();
        for _ in 0..app.active_page {
            app.handle_key_event(KeyCode::Left.into());
        }
        assert_eq!(app.active_page, 0);
        app.handle_key_event(KeyCode::Left.into());
        assert_eq!(app.active_page, 0);

        for i in 1..app.pages.len() - 1 {
            app.handle_key_event(KeyCode::Right.into());
            assert_eq!(app.active_page, i);
        }

        app.handle_key_event(KeyCode::Right.into());
        assert_eq!(app.active_page, app.pages.len() - 1);
    }

    #[test]
    fn quit() {
        let mut app = App::new();
        assert!(!app.exit);
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }

    // Tests against internet-acquired HTML if possible
    #[test]
    fn search_each_tab() {
        let mut app = App::new();
        // Make sure we start at the first tab
        for _ in 0..app.active_page {
            app.handle_key_event(KeyCode::Left.into());
        }
        app.handle_key_event(KeyCode::Char('s').into());
        for i in 0..app.pages.len() {
            assert_eq!(app.active_page, i);
            assert_ne!(
                app.search_blocking(Some(10))
                    .expect("search should work")
                    .len(),
                0,
                "on page {}: {}",
                app.active_page,
                app.pages[app.active_page].name()
            );
            app.handle_key_event(KeyCode::Right.into());
        }
    }
}
