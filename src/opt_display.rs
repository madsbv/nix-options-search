use crate::opt_data::OptText;
use ratatui::{
    prelude::*,
    widgets::{Block, Padding, Paragraph, Wrap},
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

pub struct ListableOptWidget {
    pub content: OptText,
    height: usize,
    width: usize,
    style: Style,
}

impl ListableOptWidget {
    const DEFAULT_HEIGHT: usize = 4;

    pub fn new(value: OptText, width: usize, index: usize) -> Self {
        ListableOptWidget {
            content: value,
            height: ListableOptWidget::DEFAULT_HEIGHT,
            width,
            style: if index % 2 == 0 {
                Style::default()
            } else {
                Style::default().bg(Color::Indexed(236))
            },
        }
    }
}

impl Widget for ListableOptWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let title_style = Style::new().blue();

        let name = Paragraph::new(Line::from(vec![
            Span::styled("Name: ", title_style),
            self.content.name.to_string().into(),
        ]));
        let var_type = Paragraph::new(Line::from(vec![
            Span::styled("Type: ", title_style),
            self.content.var_type.to_string().into(),
        ]));
        let default = Paragraph::new(Line::from(vec![
            Span::styled("Default: ", title_style),
            self.content.default.to_string().into(),
        ]));
        let description = Paragraph::new(Line::from(vec![
            Span::styled("Description: ", title_style),
            self.content.description.to_string().into(),
        ]))
        .wrap(Wrap { trim: true });
        let example = Paragraph::new(Line::from(vec![
            Span::styled("Example: ", title_style),
            self.content.example.to_string().into(),
        ]))
        .wrap(Wrap { trim: true });

        let block = Block::default()
            .style(self.style)
            .padding(Padding::bottom(1));
        let inner = block.inner(area);
        block.render(area, buf);

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Percentage(100)])
            .split(inner);

        let inner_top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(1, 3); 3])
            .split(outer[0]);
        let inner_bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)])
            .split(outer[1]);

        name.render(inner_top[0], buf);
        var_type.render(inner_top[1], buf);
        default.render(inner_top[2], buf);

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
