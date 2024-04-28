use crate::opt_data::OptText;
use ratatui::{
    prelude::*,
    widgets::{Block, Paragraph, Wrap},
};
use tracing::debug;
use tui_widget_list::{ListableWidget, ScrollAxis};

/// A widget to display a single option parsed from nix-darwin/nixos/home-manager.
/// Layout:
/// ######################################################
/// # Name: ...          Type: ...          Default: ... #
/// # Description: ...............          Example: ... #
/// #     ........................              ........ #
/// ######################################################
///

fn wrapped_paragraph_with_title<'a>(
    content: &'a str,
    title: &'a str,
    title_style: Style,
) -> Paragraph<'a> {
    let title_span = Span::styled(title, title_style);

    return Paragraph::new(Line::from(vec![title_span, content.to_string().into()]))
        .wrap(Wrap { trim: true });
}

pub struct ListableOptWidget {
    content: OptText,
    height: usize,
    width: usize,
    style: Style,
}

impl ListableOptWidget {
    const DEFAULT_HEIGHT: usize = 4;

    pub fn new(value: OptText, width: usize) -> Self {
        ListableOptWidget {
            content: value,
            height: ListableOptWidget::DEFAULT_HEIGHT,
            width,
            style: Style::default(),
        }
    }
}

impl From<ListableOptWidget> for OptText {
    fn from(value: ListableOptWidget) -> Self {
        value.content
    }
}

impl Widget for ListableOptWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::default().style(self.style);
        let inner = block.inner(area);
        block.render(area, buf);

        let area = inner;
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Percentage(100),
                // Padding between list elements
                Constraint::Length(1),
            ])
            .split(area);

        let inner_top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3); 3])
            .split(outer[0]);
        let inner_bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
            .split(outer[1]);

        let title_style = Style::new().blue();

        let name = wrapped_paragraph_with_title(&self.content.name, "Name: ", title_style);
        let var_type = wrapped_paragraph_with_title(&self.content.var_type, "Type: ", title_style);
        let default = wrapped_paragraph_with_title(&self.content.default, "Default: ", title_style);
        name.render(inner_top[0], buf);
        var_type.render(inner_top[1], buf);
        default.render(inner_top[2], buf);

        let description =
            wrapped_paragraph_with_title(&self.content.description, "Description: ", title_style);
        let example = wrapped_paragraph_with_title(&self.content.example, "Example: ", title_style);
        description.render(inner_bottom[0], buf);
        example.render(inner_bottom[1], buf);
    }
}

impl ListableWidget for ListableOptWidget {
    fn highlight(self) -> Self
    where
        Self: Sized,
    {
        // Description and example fields are laid out next to each other at a 2:1 ratio.
        let description_height = (self.content.description.len() * 3) / (self.width * 2);
        let example_height = (self.content.example.len() * 3) / self.width;

        // Integer division truncates decimals
        let height =
            (description_height.max(example_height) + 3).max(ListableOptWidget::DEFAULT_HEIGHT);

        debug!(
            name: "Compute highlighted item",
            description = self.content.description,
            example = self.content.example,
            width = self.width,
            description_height = description_height,
            example_height = example_height,
            height = height,
        );

        Self {
            height,
            style: Style::default().bg(Color::DarkGray),
            ..self
        }
    }

    fn size(&self, _: &ScrollAxis) -> usize {
        self.height
    }
}
