use crate::opt_data::OptText;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use tui_widget_list::{ListableWidget, ScrollAxis};

/// A widget to display a single option parsed from nix-darwin/nixos/home-manager.
/// Layout:
/// ######################################################
/// # Name: ...          Type: ...          Default: ... #
/// # Description: ...............          Example: ... #
/// #     ........................              ........ #
/// ######################################################
///
/// This widget only handles text layout and has no border built in.
impl Widget for &OptText {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
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

        let name = wrapped_paragraph_with_title(&self.name, "Name: ", title_style);
        let var_type = wrapped_paragraph_with_title(&self.var_type, "Type: ", title_style);
        let default = wrapped_paragraph_with_title(&self.default, "Default: ", title_style);
        name.render(inner_top[0], buf);
        var_type.render(inner_top[1], buf);
        default.render(inner_top[2], buf);

        let description =
            wrapped_paragraph_with_title(&self.description, "Description: ", title_style);
        let example = wrapped_paragraph_with_title(&self.example, "Example: ", title_style);
        description.render(inner_bottom[0], buf);
        example.render(inner_bottom[1], buf);
    }
}

fn wrapped_paragraph_with_title<'a>(
    content: &'a str,
    title: &'a str,
    title_style: Style,
) -> Paragraph<'a> {
    let title_span = Span::styled(title, title_style);

    return Paragraph::new(Line::from(vec![title_span, content.to_string().into()]))
        .wrap(Wrap { trim: true });
}

impl OptText {
    pub const HEIGHT: usize = 3;
}

pub struct ListableOptWidget {
    content: OptText,
    height: usize,
}

impl ListableOptWidget {
    pub const HEIGHT: usize = 4;
    pub const HIGHLIGHTED_HEIGHT: usize = 7;
}

impl From<OptText> for ListableOptWidget {
    fn from(value: OptText) -> Self {
        ListableOptWidget {
            content: value,
            height: ListableOptWidget::HEIGHT,
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
        OptText::from(self).render(area, buf);
    }
}

impl ListableWidget for ListableOptWidget {
    fn highlight(self) -> Self
    where
        Self: Sized,
    {
        Self {
            height: ListableOptWidget::HIGHLIGHTED_HEIGHT,
            ..self
        }
    }

    fn size(&self, _: &ScrollAxis) -> usize {
        self.height
    }
}
