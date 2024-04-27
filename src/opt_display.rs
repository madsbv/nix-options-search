use crate::opt_data::OptText;
use ratatui::{prelude::*, widgets::Paragraph};
use textwrap::{wrap, Options};
use tracing::debug;

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

// Factor out text wrapping logic from Ratatui element construction
fn wrapped_paragraph_with_title<'a>(
    content: &'a str,
    title: &'a str,
    title_style: Style,
    area: Rect,
) -> Paragraph<'a> {
    let title_span = Span::styled(title, title_style);
    // Necessary check to avoid `Vec::remove` panicking later.
    if content.is_empty() {
        return Paragraph::new(title_span);
    }

    // Skip text wrapping calculations if there's only one line of space.
    if area.height == 1 {
        return Paragraph::new(Line::from(vec![title_span, content.into()]));
    }

    let options = Options::new(area.width as usize).initial_indent(title);

    let mut wrapped = wrap(content, options)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect::<Vec<_>>();

    wrapped[0] = wrapped[0]
        .strip_prefix(title)
        .expect("wrapping with initial_indent `title` prefixes `wrapped[0]` with `title`")
        .into();

    let mut lines = vec![];

    lines.push(Line::from(vec![title_span, wrapped.remove(0).into()]));

    for w in wrapped.into_iter().skip(1).take(area.height as usize - 1) {
        lines.push(Line::from(w));
    }
    Paragraph::new(Text::from(lines))
}

impl OptText {
    pub const HEIGHT: usize = 3;
}
