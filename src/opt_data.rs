use std::borrow::Cow;

use color_eyre::eyre::{ensure, Result};
use html2text::from_read_with_decorator;
use html2text::render::text_renderer::TrivialDecorator;
use tl::{HTMLTag, NodeHandle, Parser, VDom};
use tracing::debug;

/// Structure of data/index.html (nix-darwin): Each option header is in a `<dt>`, associated description, type, default, example and link to docs is in a `<dd>`.
/// This method assumes that there's an equal number of `<dt>` and `<dd>` tags, and that they come paired up one after the other. If the number of `<dt>` and `<dd>` tags don't match, this panics. If they are out of order, we have no way of catching it, so the output will just be meaningless.
pub fn parse_options<'dom>(dom: &'dom VDom<'dom>) -> Result<Vec<OptData<'dom>>> {
    let p = dom.parser();
    let dt_tags = dom
        .query_selector("dt")
        .expect("dt is a valid CSS selector")
        .collect::<Vec<_>>();
    let dd_tags = dom
        .query_selector("dd")
        .expect("dd is a valid CSS selector")
        .collect::<Vec<_>>();

    ensure!(
        dt_tags.len() == dd_tags.len(),
        "there should be an equal number of dt and dd tags"
    );

    Ok(std::iter::zip(dt_tags, dd_tags)
        .map(|(dt, dd)| OptParser::new(dt, dd, p).parse())
        .collect())
}

#[derive(Clone, Debug)]
pub struct OptData<'a> {
    name: Vec<HTMLTag<'a>>,
    description: Vec<HTMLTag<'a>>,
    var_type: Vec<HTMLTag<'a>>,
    default: Vec<HTMLTag<'a>>,
    example: Vec<HTMLTag<'a>>,
    declared_by: Vec<HTMLTag<'a>>,
    p: &'a Parser<'a>,
}

impl OptData<'_> {
    // NOTE: All conversion of HTMLTags to plaintext goes through this function.
    fn field_to_raw_html(&self, section: &[HTMLTag]) -> String {
        section
            .iter()
            .map(|t| t.outer_html(self.p))
            .fold(String::new(), |acc, e| acc + "\n" + &e)
            .trim()
            .to_string()
    }

    fn extract_urls_from_section<'a>(
        &'a self,
        section: &'a [HTMLTag],
        class: Option<&str>,
    ) -> Vec<Cow<'a, str>> {
        // Surely there's a better way...
        section
            .iter()
            .filter_map(|t| t.query_selector(self.p, "a"))
            .flatten()
            .filter_map(|nh| nh.get(self.p))
            .filter_map(tl::Node::as_tag)
            .map(HTMLTag::attributes)
            .filter_map(move |a| match class {
                Some(class) if a.is_class_member(class) => a.get("href"),
                None => a.get("href"),
                _ => None,
            })
            .flatten()
            .map(tl::Bytes::as_utf8_str)
            .collect()
    }

    fn declared_by_urls(&self) -> Vec<String> {
        self.extract_urls_from_section(&self.declared_by, Some("filename"))
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct OptRawHTML {
    pub name: String,
    pub description: String,
    pub var_type: String,
    pub default: String,
    pub example: String,
    pub declared_by: String,
    pub declared_by_urls: Vec<String>,
}

impl From<OptData<'_>> for OptRawHTML {
    fn from(value: OptData<'_>) -> Self {
        // This seems to do the right thing on first glance.
        // TODO: Add the declared_by_link/declared_by_url field to OptRawHTML and OptText, and integrate with UI
        // TODO: Write unit test for this function with a couple of example HTML snippets

        let declared_by_urls = value.declared_by_urls();

        debug!(name: "Convert OptData to OptRawHTML", declared_by = format!("{:?}", value.declared_by), declared_by = format!("{declared_by_urls:?}"));
        Self {
            name: value.field_to_raw_html(&value.name),
            description: value.field_to_raw_html(&value.description),
            var_type: value.field_to_raw_html(&value.var_type),
            default: value.field_to_raw_html(&value.default),
            example: value.field_to_raw_html(&value.example),
            declared_by: value.field_to_raw_html(&value.declared_by),
            declared_by_urls,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OptText {
    pub name: String,
    pub description: String,
    pub var_type: String,
    pub default: String,
    pub example: String,
    pub declared_by: String,
    pub declared_by_urls: Vec<String>,
}

impl From<OptRawHTML> for OptText {
    fn from(html: OptRawHTML) -> Self {
        let dec = TrivialDecorator::new();
        let name = from_read_with_decorator(html.name.as_bytes(), 10000, dec.clone());
        let description = from_read_with_decorator(html.description.as_bytes(), 10000, dec.clone());
        let var_type = from_read_with_decorator(html.var_type.as_bytes(), 10000, dec.clone())
            .trim_start_matches("Type:")
            .trim()
            .to_string();
        let default = from_read_with_decorator(html.default.as_bytes(), 10000, dec.clone())
            .trim_start_matches("Default:")
            .trim()
            .to_string();
        let example = from_read_with_decorator(html.example.as_bytes(), 10000, dec.clone())
            .trim_start_matches("Example:")
            .trim()
            .to_string();
        let declared_by = from_read_with_decorator(html.declared_by.as_bytes(), 10000, dec.clone())
            .trim_start_matches("Declared By:")
            .trim()
            .to_string();
        Self {
            name,
            description,
            var_type,
            default,
            example,
            declared_by,
            declared_by_urls: html.declared_by_urls,
        }
    }
}

impl From<OptData<'_>> for OptText {
    fn from(value: OptData<'_>) -> Self {
        let html: OptRawHTML = value.into();
        html.into()
    }
}

impl std::fmt::Display for OptData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let html: OptRawHTML = self.clone().into();
        write!(f, "{html}")
    }
}

impl std::fmt::Display for OptText {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Name: {}\nDescription: {}\n{}\n{}\n{}\n{}\n--------------",
            self.name,
            self.description,
            self.var_type,
            self.default,
            self.example,
            self.declared_by,
        )
    }
}

impl std::fmt::Display for OptRawHTML {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Name: {}\nDescription: {}\n{}\n{}\n{}\n{}\n--------------",
            self.name,
            self.description,
            self.var_type,
            self.default,
            self.example,
            self.declared_by,
        )
    }
}

#[derive(Debug)]
pub struct OptParser<'a> {
    dt: NodeHandle,
    dd: NodeHandle,
    p: &'a Parser<'a>,
}

impl<'dom> OptParser<'dom> {
    pub fn new(dt: NodeHandle, dd: NodeHandle, p: &'dom Parser) -> OptParser<'dom> {
        OptParser { dt, dd, p }
    }

    pub fn parse(self) -> OptData<'dom> {
        let mut tag_slices = self.split_tags();
        let name = self.get_name();
        let var_type = self.get_field_by_separator(&mut tag_slices, OptParser::SEPARATOR_TAGS[0]);
        let default = self.get_field_by_separator(&mut tag_slices, OptParser::SEPARATOR_TAGS[1]);
        let example = self.get_field_by_separator(&mut tag_slices, OptParser::SEPARATOR_TAGS[2]);
        let declared_by =
            self.get_field_by_separator(&mut tag_slices, OptParser::SEPARATOR_TAGS[3]);
        let description = tag_slices
            .into_iter()
            .fold(vec![], |acc, e| [acc, e].concat());

        OptData {
            name,
            description,
            var_type,
            default,
            example,
            declared_by,
            p: self.p,
        }
    }

    const SEPARATOR_TAGS: [&'static str; 4] = [
        r#"<span class="emphasis"><em>Type:</em></span>"#,
        r#"<span class="emphasis"><em>Default:</em></span>"#,
        r#"<span class="emphasis"><em>Example:</em></span>"#,
        r#"<span class="emphasis"><em>Declared by:</em></span>"#,
    ];

    // Might want to unify this with self.get_field().
    fn get_name(&'_ self) -> Vec<HTMLTag<'dom>> {
        self.dt
            .get(self.p)
            .unwrap()
            .children() // <- Creates owned struct tl::Children
            .unwrap()
            .top()
            .iter()
            .filter_map(|n| n.get(self.p)?.as_tag().cloned())
            .collect::<Vec<_>>()
    }

    fn get_field_by_separator(
        &'_ self,
        split_tags: &'_ mut Vec<Vec<HTMLTag<'dom>>>,
        tag: &str,
    ) -> Vec<HTMLTag<'dom>> {
        for i in 0..split_tags.len() {
            if let Some(t) = split_tags[i].first() {
                if t.inner_html(self.p).contains(tag) {
                    return split_tags.swap_remove(i);
                }
            }
        }
        vec![]
    }

    fn split_tags(&self) -> Vec<Vec<HTMLTag<'dom>>> {
        // Can we simplify this unholy incantation?
        let dd_tags = self
            .dd
            .get(self.p)
            .unwrap()
            .children() // <- Creates owned struct tl::Children
            .unwrap()
            .top()
            .iter()
            .filter_map(|n| n.get(self.p)?.as_tag())
            .collect::<Vec<_>>();

        // split_inclusive puts the matched element at the end of the previous slice. We want the matched element at the beginning of the slice. To fix, we reverse the entire list, split_inclusive, then reverse both the outer list and each of the inner lists created by split_inclusive.
        let rev_tags: Vec<_> = dd_tags.into_iter().rev().collect();
        // vector of vectors, in the right order
        rev_tags
            // Split slice into slice of slices with separator_tags as terminators
            .split_inclusive(|t| {
                OptParser::SEPARATOR_TAGS
                    .iter()
                    .any(|a| t.inner_html(self.p).contains(a))
            })
            // Reverse outer
            .rev()
            // Reverse each inner and clone
            .map(|s| s.iter().rev().copied().cloned().collect::<Vec<_>>())
            .collect::<Vec<_>>()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: This is a costly test, and is already covered by tests in crate::search and crate::app.
    #[ignore]
    #[test]
    fn parse_caches_to_opts() {
        use crate::search::Source;
        for source in [
            Source::NixDarwin,
            Source::NixOS,
            Source::HomeManager,
            Source::HomeManagerNixOS,
            Source::HomeManagerNixDarwin,
        ] {
            let dom =
                tl::parse(source.cache(), tl::ParserOptions::default()).expect("cache should work");
            let opts = parse_options(&dom).expect("cache should work");
            assert!(!opts.is_empty());
        }
    }
}
