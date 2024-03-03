use ratatui::{prelude::*, widgets::Paragraph, widgets::Wrap};

#[derive(Clone, Debug)]
pub struct OptDisplay {
    name: String,
    description: String,
    var_type: String,
    default: String,
    example: String,
}

/// A widget to display a single option parsed from nix-darwin/nixos/home-manager.
/// Layout:
/// ######################################################
/// # Name: ...          Type: ...          Default: ... #
/// # Description: ...............          Example: ... #
/// #     ........................              ........ #
/// ######################################################
///
/// This widget only handles text layout and has no border built in.
impl Widget for &OptDisplay {
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

        // TODO: Colors on the identifiers
        // TODO: Better text wrapping
        let name_line = Line::from(format!("Name: {}", self.name));
        let type_line = Line::from(format!("Type: {}", self.var_type));
        let default_line = Line::from(format!("Default: {}", self.default));
        name_line.render(inner_top[0], buf);
        type_line.render(inner_top[1], buf);
        default_line.render(inner_top[2], buf);

        let description_text =
            Paragraph::new(format!("Description: {}", self.description)).wrap(Wrap { trim: false });
        let example_text =
            Paragraph::new(format!("Example: {}", self.example)).wrap(Wrap { trim: false });
        description_text.render(inner_bottom[0], buf);
        example_text.render(inner_bottom[1], buf);
    }
}

impl OptDisplay {
    /// Create an `OptDisplay` from a vector of Strings, assumed to be in the order `name, description, var_type, default, example`. Defaults to empty strings for any missing entries.
    pub fn from_vec(mut opt: Vec<String>) -> Self {
        let mut opt = opt.drain(..);
        Self {
            name: opt.next().unwrap_or_default(),
            description: opt.next().unwrap_or_default(),
            var_type: opt.next().unwrap_or_default(),
            default: opt.next().unwrap_or_default(),
            example: opt.next().unwrap_or_default(),
        }
    }
}

// TODO: Create an OptDisplayList or something
