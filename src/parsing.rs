use bitcode::{Decode, Encode};
use color_eyre::eyre::{ensure, Result};
use html2text::from_read_with_decorator;
use html2text::render::TrivialDecorator;
use lazy_regex::regex_find;
use std::borrow::Cow;
use tl::{HTMLTag, NodeHandle, Parser, ParserOptions, VDom};
use tracing::{trace, warn};

/// A fully parsed option entity with fields formatted as raw text ready to print
#[derive(Clone, Debug, Encode, Decode, PartialEq)]
pub(crate) struct OptText {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) var_type: String,
    pub(crate) default: String,
    pub(crate) example: String,
    pub(crate) declared_by: String,
    pub(crate) declared_by_urls: Vec<String>,
}

/// Structure of data/index.html (nix-darwin): Each option header is in a `<dt>`, associated description, type, default, example and link to docs is in a `<dd>`.
/// This method assumes that there's an equal number of `<dt>` and `<dd>` tags, and that they come paired up one after the other. If the number of `<dt>` and `<dd>` tags don't match, this panics. If they are out of order, we have no way of catching it, so the output will just be meaningless.
pub(crate) fn parse_options(html: &str) -> Result<Vec<OptText>> {
    let dom = tl::parse(html, ParserOptions::default())?;
    let p = dom.parser();
    // TODO: To parse the Nixpkgs reference manual ("https://nixos.org/manual/nixpkgs/stable/"), would it help to pull out dl lists first and then parse dt/dd tags pairwise in each list?
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
        "Should have {} dt tags = {} dd tags",
        dt_tags.len(),
        dd_tags.len()
    );

    Ok(std::iter::zip(dt_tags, dd_tags)
        .filter_map(|(dt, dd)| OptParser::new(dt, dd, p).parse())
        .map(std::convert::Into::into)
        .collect())
}

/// Different data sources expose version information in different ways, so we try multiple methods in hopes of eventually succeeding.
pub(crate) fn parse_version(html: &str) -> Result<Option<String>> {
    let dom = tl::parse(html, ParserOptions::default())?;
    let sub = parse_version_by_subtitle(&dom);
    if sub.is_some() {
        Ok(sub)
    } else {
        Ok(parse_version_by_title(&dom))
    }
}

/// Try to find a version in the title of the page, using an "official" semver regex.
/// This applies to the Nix Built-ins source in the defaults.
fn parse_version_by_title<'dom>(dom: &'dom VDom<'dom>) -> Option<String> {
    let p = dom.parser();
    let title = dom.query_selector("title")?.next()?.get(p)?.inner_html(p);

    // https://semver.org/#is-there-a-suggested-regular-expression-regex-to-check-a-semver-string
    let semver = regex_find!(
        r#"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#,
        &title
    )?;
    Some(format!("Version {semver}"))
}

/// Use the first `<subtitle>` contents, if such a field exists.
/// Used for most of the default sources.
fn parse_version_by_subtitle<'dom>(dom: &'dom VDom<'dom>) -> Option<String> {
    let p = dom.parser();
    Some(
        dom.query_selector(".subtitle")?
            .next()?
            .get(p)?
            .inner_html(p)
            .to_string(),
    )
}

/// A term and each section of its description, as a list of the top level html elements in each section.
#[derive(Clone, Debug)]
struct OptData<'a> {
    term: Vec<HTMLTag<'a>>,
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

    fn term_id(&self) -> String {
        self.term
            .iter()
            .filter_map(|t| t.query_selector(self.p, "a"))
            .flatten()
            .filter_map(|nh| nh.get(self.p))
            .filter_map(tl::Node::as_tag)
            .map(HTMLTag::attributes)
            .filter_map(move |a| a.id())
            .map(tl::Bytes::as_utf8_str)
            .collect()
    }

    /// NOTE: Options can be declared in multiple places, hence returning a Vec here.
    /// They can also be declared in *no* places, e.g. `Background` for Nix-Darwin.
    fn extract_urls<'a>(
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
        self.extract_urls(&self.declared_by, Some("filename"))
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }
}

#[derive(Clone, Debug)]
struct OptRawHTML {
    id: String,
    name: String,
    description: String,
    var_type: String,
    default: String,
    example: String,
    declared_by: String,
    declared_by_urls: Vec<String>,
}

impl From<OptData<'_>> for OptRawHTML {
    fn from(value: OptData<'_>) -> Self {
        let declared_by_urls = value.declared_by_urls();

        trace!(name: "Convert OptData to OptRawHTML", declared_by = format!("{:?}", value.declared_by), declared_by = format!("{declared_by_urls:?}"));
        Self {
            id: value.term_id(),
            name: value.field_to_raw_html(&value.term),
            description: value.field_to_raw_html(&value.description),
            var_type: value.field_to_raw_html(&value.var_type),
            default: value.field_to_raw_html(&value.default),
            example: value.field_to_raw_html(&value.example),
            declared_by: value.field_to_raw_html(&value.declared_by),
            declared_by_urls,
        }
    }
}

impl From<OptRawHTML> for OptText {
    fn from(html: OptRawHTML) -> Self {
        let name = read_html_strip_prefix(&html.name, None);
        let description = read_html_strip_prefix(&html.description, None);
        let var_type = read_html_strip_prefix(&html.var_type, Some("Type:"));
        let default = read_html_strip_prefix(&html.default, Some("Default:"));
        let example = read_html_strip_prefix(&html.example, Some("Example:"));
        let declared_by = read_html_strip_prefix(&html.declared_by, Some("Declared By:"));
        Self {
            id: html.id,
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

fn read_html_strip_prefix(s: &str, pre: Option<&str>) -> String {
    let dec = TrivialDecorator::new();
    let pre = pre.unwrap_or("");
    from_read_with_decorator(s.as_bytes(), 10000, dec)
        .unwrap_or(s.to_string())
        .trim_start_matches(pre)
        .trim()
        .to_string()
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
            "Name: {}\nDescription: {}\n{}\n{}\n{}\n{}\n{:?}\n--------------",
            self.name,
            self.description,
            self.var_type,
            self.default,
            self.example,
            self.declared_by,
            self.declared_by_urls
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
struct OptParser<'a> {
    dt: NodeHandle,
    dd: NodeHandle,
    p: &'a Parser<'a>,
}

impl<'dom> OptParser<'dom> {
    fn new(dt: NodeHandle, dd: NodeHandle, p: &'dom Parser) -> OptParser<'dom> {
        OptParser { dt, dd, p }
    }

    fn parse(self) -> Option<OptData<'dom>> {
        let Some(mut tag_slices) = self.split_tags() else {
            warn!("Failed to split dd tags on a node");
            return None;
        };
        let term = self.get_term().unwrap_or_else(|| {
            warn!("Failed to parse option name");
            vec![]
        });
        let var_type = self.get_field_by_separator(&mut tag_slices, Self::TYPE_TAG);
        let default = self.get_field_by_separator(&mut tag_slices, Self::DEFAULT_TAG);
        let example = self.get_field_by_separator(&mut tag_slices, Self::EXAMPLE_TAG);
        let declared_by = self.get_field_by_separator(&mut tag_slices, Self::DECLAREDBY_TAG);
        let description = tag_slices
            .into_iter()
            .fold(vec![], |acc, e| [acc, e].concat());

        Some(OptData {
            term,
            description,
            var_type,
            default,
            example,
            declared_by,
            p: self.p,
        })
    }

    const TYPE_TAG: &'static str = r#"<span class="emphasis"><em>Type:</em></span>"#;
    const DEFAULT_TAG: &'static str = r#"<span class="emphasis"><em>Default:</em></span>"#;
    const EXAMPLE_TAG: &'static str = r#"<span class="emphasis"><em>Example:</em></span>"#;
    const DECLAREDBY_TAG: &'static str = r#"<span class="emphasis"><em>Declared by:</em></span>"#;
    const SEPARATOR_TAGS: [&'static str; 4] = [
        Self::TYPE_TAG,
        Self::DEFAULT_TAG,
        Self::EXAMPLE_TAG,
        Self::DECLAREDBY_TAG,
    ];

    fn get_term(&'_ self) -> Option<Vec<HTMLTag<'dom>>> {
        Some(
            self.dt
                .get(self.p)?
                .children()? // <- Creates owned struct tl::Children
                .top()
                .iter()
                .filter_map(|n| n.get(self.p)?.as_tag().cloned())
                .collect::<Vec<_>>(),
        )
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

    fn split_tags(&self) -> Option<Vec<Vec<HTMLTag<'dom>>>> {
        // Can we simplify this unholy incantation?
        let dd_tags = self
            .dd
            .get(self.p)?
            .children()? // <- Creates owned struct tl::Children
            .top()
            .iter()
            .filter_map(|n| n.get(self.p)?.as_tag())
            .collect::<Vec<_>>();

        // split_inclusive puts the matched element at the end of the previous slice. We want the matched element at the beginning of the slice. To fix, we reverse the entire list, split_inclusive, then reverse both the outer list and each of the inner lists created by split_inclusive.
        let rev_tags: Vec<_> = dd_tags.into_iter().rev().collect();
        // vector of vectors, in the right order
        Some(
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
                .collect::<Vec<_>>(),
        )
    }
}
