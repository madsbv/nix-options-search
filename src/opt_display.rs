use std::borrow::{BorrowMut, Cow};

use crate::opt_data::OptText;
use ratatui::{prelude::*, widgets::Paragraph};
use textwrap::{wrap, Options};

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
            .constraints([Constraint::Length(1), Constraint::Length(2)])
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

        let name = wrapped_paragraph_with_title(&self.name, "Name: ", title_style, inner_top[0]);
        let var_type =
            wrapped_paragraph_with_title(&self.var_type, "Type: ", title_style, inner_top[1]);
        let default =
            wrapped_paragraph_with_title(&self.default, "Default: ", title_style, inner_top[2]);
        name.render(inner_top[0], buf);
        var_type.render(inner_top[1], buf);
        default.render(inner_top[2], buf);

        let description = wrapped_paragraph_with_title(
            &self.description,
            "Description: ",
            title_style,
            inner_bottom[0],
        );
        let example =
            wrapped_paragraph_with_title(&self.example, "Example: ", title_style, inner_bottom[1]);
        description.render(inner_bottom[0], buf);
        example.render(inner_bottom[1], buf);
    }
}

fn wrapped_paragraph_with_title<'a>(
    content: &'a str,
    title: &'a str,
    title_style: Style,
    area: Rect,
) -> Paragraph<'a> {
    let title_span = Span::styled(title, title_style);

    if area.height <= 1 {
        // Avoid running text wrapping algorithm if not needed.
        return Paragraph::new(Line::from(vec![title_span, content.to_string().into()]));
    }

    let mut wrapped = wrap_text(content, title, area.width as usize);
    let mut lines = vec![];

    lines.push(Line::from(vec![
        title_span,
        std::mem::take(&mut wrapped[0]).into(),
    ]));

    for w in wrapped.into_iter().skip(1).take(area.height as usize - 1) {
        lines.push(Line::from(w.to_string()));
    }
    Paragraph::new(Text::from(lines))
}

fn wrap_text<'a>(content: &'a str, title: &'a str, width: usize) -> Vec<Cow<'a, str>> {
    let options = Options::new(width).initial_indent(title);

    let mut wrapped = wrap(content, options);

    wrapped[0] = Cow::Owned(
        wrapped[0]
            .borrow_mut()
            .strip_prefix(title)
            .expect("wrapping with initial_indent `title` prefixes `wrapped[0]` with `title`")
            .into(),
    );
    wrapped
}

impl OptText {
    pub const HEIGHT: usize = 3;
}

#[cfg(test)]
mod tests {
    

    use super::*;

    #[test]
    fn test_wrapping() {
        let title = "Test: ";
        let content = "This is a string of length 74. The text to be wrapped has total length 80.";
        let wrapped = wrap_text(content, title, 40);

        assert_eq!(wrapped[0].as_ref(), "This is a string of length 74. The");
        assert_eq!(
            wrapped[1].as_ref(),
            "text to be wrapped has total length 80."
        );
    }
}
