use crate::opt_display::OptDisplay;
use crate::search::{nix_darwin_searcher, nix_darwin_searcher_from_cache, search_for};
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
use std::cell::RefCell;
use std::io;

// This will probably be renamed to SearchPage whenever we add nixOS/home-manager support as well.
// Actually, only the matcher will be switched out, search_string and the search box should remain.
pub struct App {
    search_string: String,
    // We need `RefCell` because `Nucleo` holds the pattern to search for as internal state, and doing a search requires `&mut Nucleo`. Using RefCell allows us to do the search at render-time, when we know how many results we'll need to populate the window.
    // Alternative: Split the searching step up into the reparse step and a finish step that actually outputs the results.
    matcher: RefCell<nucleo::Nucleo<Vec<String>>>,
    exit: bool,
}

/// The nix-darwin options searcher
pub fn darwin() -> Result<App> {
    let matcher = nix_darwin_searcher()
        .unwrap_or(nix_darwin_searcher_from_cache()?)
        .into();
    Ok(App {
        search_string: String::new(),
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

        let results = search_for(&self.search_string, &mut *self.matcher.borrow_mut())
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
        let mut app =
            darwin().expect("we can initialize an app from the cached index.html at least");

        app.handle_key_event(KeyCode::Char('w').into());
        assert_eq!(app.search_string, "w".to_string());

        assert!(!app.exit);
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }
}
