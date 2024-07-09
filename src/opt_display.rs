use crate::opt_data::OptText;
use ratatui::{
    prelude::*,
    widgets::{Block, Padding, Paragraph, Wrap},
};
use tui_widget_list::PreRender;

/// A widget to display a single option parsed from nix-darwin/nixos/home-manager.
/// Layout:
/// ######################################################
/// # Name: ...          Type: ...          Default: ... #
/// # Description: ...............          Example: ... #
/// #     ........................              ........ #
/// ######################################################

pub struct ListableOptWidget {
    pub content: OptText,
    style: Style,
}

impl ListableOptWidget {
    const DEFAULT_HEIGHT: u16 = 4;

    pub fn new(value: OptText) -> Self {
        ListableOptWidget {
            content: value,
            style: Style::default(),
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

impl PreRender for ListableOptWidget {
    fn pre_render(&mut self, context: &tui_widget_list::PreRenderContext) -> u16 {
        self.style = if context.is_selected {
            Style::default().bg(Color::DarkGray)
        } else if context.index % 2 == 0 {
            Style::default()
        } else {
            Style::default().bg(Color::Indexed(236))
        };
        self.full_height(context.cross_axis_size)
    }
}

impl ListableOptWidget {
    fn full_height(&self, width: u16) -> u16 {
        // Description and example fields are laid out next to each other at a 2:1 ratio.

        #[allow(clippy::cast_possible_truncation)]
        let description_height = (self.content.description.len() as u16 * 3) / (width * 2);
        #[allow(clippy::cast_possible_truncation)]
        let example_height = (self.content.example.len() as u16 * 3) / width;

        // Integer division truncates decimals
        (description_height.max(example_height) + 3).max(ListableOptWidget::DEFAULT_HEIGHT)
    }
}
