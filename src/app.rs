use crate::opt_data::OptText;
use crate::opt_display::OptListItem;
use crate::search::{Finder, InputStatus, Source};
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::Padding;
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{block::Block, Borders, Paragraph, Tabs},
};
use std::io;
use std::time::Duration;
use tracing::debug;
use tui_widget_list::{ListBuilder, ListState, ListView};

// XXX: Optimization idea: Have a "results cache stack" where, each time search_string is appended to, we push the current search results; and when Backspace is pressed, instead of re-searching we just pop the stack. On tab change, we have to clear the stack. Might not be worth it.
pub struct App {
    search_string: String,
    pages: Vec<Finder>,
    // An integer in `0..self.pages.len()`
    active_page: usize,
    // To use Nucleo's append optimization and avoid reparsing if pattern hasn't changed
    input_status: InputStatus,
    result_list_state: ListState,
    selected_item: Option<OptText>,
    exit: bool,
}

impl App {
    pub fn new() -> App {
        App {
            search_string: String::new(),
            pages: vec![
                Finder::new(Source::NixDarwin),
                Finder::new(Source::NixOS),
                Finder::new(Source::NixOSUnstable),
                Finder::new(Source::HomeManager),
                Finder::new(Source::HomeManagerNixOS),
                Finder::new(Source::HomeManagerNixDarwin),
            ],
            active_page: 0,
            input_status: InputStatus::Change,
            result_list_state: ListState::default(),
            selected_item: None,
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

    fn render_frame(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        // If recv sees that results are waiting while we're waiting for user input, return early and render the pending results.
        // NOTE: Semantically, this should really be a `select!` statement in async context.
        // This polling does take an appreciable amount of CPU time.
        while let Ok(false) = event::poll(Duration::from_millis(500)) {
            if self.pages[self.active_page]
                .results_waiting
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                self.pages[self.active_page]
                    .results_waiting
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                self.init_search();
                debug!("Found waiting search results, rendering them");
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
        debug!(name: "Handling a key event", key = format!("{key:?}"));
        match (key.code, key.modifiers) {
            (KeyCode::Right, _) | (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                if self.active_page + 1 < self.pages.len() {
                    self.active_page += 1;
                    self.input_status = InputStatus::Change;
                    self.result_list_state.select(None);
                }
            }
            (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if self.active_page > 0 {
                    self.active_page -= 1;
                    self.input_status = InputStatus::Change;
                    self.result_list_state.select(None);
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.result_list_state.next();
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.result_list_state.previous();
            }
            (KeyCode::Esc, _) => self.exit = true,
            (KeyCode::Backspace, KeyModifiers::ALT) => {
                // Clear the search field
                // KeyModifier CTRL gets picked up as C-h instead
                self.search_string.clear();
                self.input_status = InputStatus::Change;
                self.result_list_state.select(Some(0));
            }
            (KeyCode::Backspace, _) => {
                self.search_string.pop();
                self.input_status = InputStatus::Change;
            }
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                let source = &self.pages[self.active_page];
                if let Some(ref o) = self.selected_item {
                    open_url(&source.url_to(o));
                } else {
                    open_url(source.url());
                };
            }
            (KeyCode::Enter, _) => {
                if let Some(ref o) = self.selected_item {
                    for u in &o.declared_by_urls {
                        open_url(u);
                    }
                }
                // TODO: Default behaviour if there's no url? Pop up an error message somehow?
                // TODO: When there's multiple urls, can we pop up a selection box?
            }
            (KeyCode::Char(c), m) if m == KeyModifiers::NONE || m == KeyModifiers::SHIFT => {
                self.search_string.push(c);
                self.input_status = InputStatus::Append;
                self.result_list_state.select(Some(0));
            }
            _ => {}
        }
        self.init_search();
    }
}

fn open_url(url: &str) {
    let res = open::that_detached(url);
    debug!(name: "Open url", "{url}, {res:?}");
}

impl App {
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
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
            .split(area);
        let tabs = Tabs::new(self.pages.iter().map(Finder::name).collect::<Vec<_>>())
            .block(Block::default().title("Tabs").borders(Borders::ALL))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(self.active_page)
            .padding(" ", " ");

        tabs.render(tabs_layout[1], buf);
    }

    fn render_results(&mut self, area: Rect, buf: &mut Buffer) {
        let title_text = format!(" {} ", self.pages[self.active_page].name());
        let version = format!(" {} ", self.pages[self.active_page].version());
        let instructions = Line::from(vec![
            " Navigation ".into(),
            "Arrows/C-[hjkl], ".yellow().bold(),
            "Quit ".into(),
            "<Esc>, ".yellow().bold(),
            "Open in browser: Source ".into(),
            "<Enter>, ".yellow().bold(),
            "Docs ".into(),
            "<C-o> ".yellow().bold(),
        ]);

        let results_block = Block::default()
            .title_top(Line::from(title_text).bold().centered())
            .title_top(Line::from(version).right_aligned())
            .title_bottom(instructions.centered())
            .borders(Borders::ALL)
            .border_set(border::THICK)
            .padding(Padding::horizontal(1));

        let results: Vec<OptListItem> = self
            .get_results(None)
            .into_iter()
            .map(OptListItem::new)
            .collect();

        let results_list_builder = ListBuilder::new(|context| {
            let mut item = results[context.index].clone();
            let height = item.pre_render(context);
            (item, height)
        });

        let results_list = ListView::new(results_list_builder, results.len()).block(results_block);

        self.selected_item = if let Some(i) = self.result_list_state.selected {
            results
                .get(i)
                // If the .get(i) call returns None, it's because we used to have more search results
                // before the search term was changed, and now the selection index is out of bounds.
                .or(results.last())
                .map(|s| s.content.clone())
        } else {
            None
        };

        results_list.render(area, buf, &mut self.result_list_state);
    }

    fn render_search_field(&self, area: Rect, buf: &mut Buffer) {
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::THICK);

        let search_par = Paragraph::new(Text::from(self.search_string.clone().red()))
            .centered()
            .block(search_block);
        search_par.render(area, buf);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        debug!("Rendering app");
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

        self.render_tabs(chunks[0], buf);
        self.render_results(chunks[1], buf);
        self.render_search_field(chunks[2], buf);
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
