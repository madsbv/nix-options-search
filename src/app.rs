use crate::opt_display::OptDisplay;
use crate::search::{nix_darwin_searcher, nix_darwin_searcher_from_cache, search_for};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        block::{Block, Position, Title},
        Borders, Paragraph,
    },
};
use std::io;

// TODO: This will probably be renamed to SearchPage whenever we add nixOS/home-manager support as well.
// Actually, only search_results and matcher will be switched out, search_string and the search box should remain.
// TODO: Lifetimes so search_results can just point to the contents of the matcher? Doesn't seem like it's needed.
pub struct App {
    search_string: String,
    // The best matching result is first in the list
    search_results: Vec<Vec<String>>,
    matcher: nucleo::Nucleo<Vec<String>>,
    exit: bool,
}

pub fn init_darwin_app(use_cache: bool) -> Result<App> {
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
        // Actually, maybe we can store search_results as an iterator and just have it lazily evaluate the number of results we need when rendering.
        self.search_results = search_for(&self.search_string, &mut self.matcher, 100)
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

        let results_outer_area = chunks[0];
        let results_inner_area = results_block.inner(results_outer_area);

        // Since `Layout` doesn't have a `block` method, we render it manually
        results_block.render(results_outer_area, buf);

        // TODO: Don't hard-code the height of an OptDisplay
        let opt_display_height = 4;
        // Also decide whether to round up or down
        let n_opts = results_inner_area.height as usize / opt_display_height;

        let results = self
            .search_results
            .iter()
            .take(n_opts)
            .map(|v| OptDisplay::from_vec(v.clone()));

        // TODO: Do something with the spacers?
        let (results_layout, _) = Layout::default()
            .direction(Direction::Vertical)
            .constraints(results.clone().map(|_| opt_display_height as u16))
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
            init_darwin_app(true).expect("we can initialize an app from the cached index.html");

        app.handle_key_event(KeyCode::Char('w').into());
        assert_eq!(app.search_string, "w".to_string());

        assert!(!app.exit);
        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.exit);
    }
}
