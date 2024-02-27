
#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools
)]
use anyhow::Result;
use tl::{NodeHandle, Parser, VDom};

fn main() -> Result<()> {
    // let body: String = ureq::get("https://daiderd.com/nix-darwin/manual/index.html")
    //     .call()?
    //     .into_string()?;
    let body = std::fs::read_to_string("data/index-short.html").unwrap();
    let dom = tl::parse(&body, tl::ParserOptions::default())?;
    let opts = parse_options(&dom);
    println!("{:#?}", opts.unwrap());
    Ok(())
}

// Structure of data/index.html (nix-darwin): Each option header is in a <dt>, associated description, type, default and link to docs is in a <dd>.

fn parse_options(dom: &VDom<'_>) -> Option<Vec<OptData>> {
    let p = dom.parser();
    // NodeHandles to all dt and dd tags, in order
    let varlist: Vec<_> = dom.query_selector("dt, dd")?.collect();

    // Entries of varlist should be pairs of dt followed by dd
    assert!(varlist.len() % 2 == 0);

    // Pair up dt and dd tags, parse and collect
    let mut opts = vec![];
    let mut index = 0;
    while index + 1 < varlist.len() {
        let parser = OptDataParser::new(varlist.get(index)?, varlist.get(index + 1)?, p);
        opts.push(parser.parse());
        index += 2;
    }
    Some(opts)
}

#[derive(Debug)]
struct OptDataParser<'a> {
    dt: &'a NodeHandle,
    dd: &'a NodeHandle,
    p: &'a Parser<'a>,
}

impl OptDataParser<'_> {
    fn new<'a>(dt: &'a NodeHandle, dd: &'a NodeHandle, p: &'a Parser) -> OptDataParser<'a> {
        OptDataParser { dt, dd, p }
    }

    fn parse(&self) -> OptData {
        let tag_slices = self.split_tags();
        let name = self.get_name();
        // TODO: We'll make the fields of OptData Option and remove a bunch of these unwrap_or_else
        // TODO: It might be better to have get_field_by_separator consume the sections it uses, and debug_assert that there's one left, which we'll feed into description at the end
        let description = tag_slices
            .first()
            .map(|s| self.get_field(s))
            .unwrap_or_default();
        let var_type = self
            .get_field_by_separator(&tag_slices, OptDataParser::separator_tags()[0])
            .unwrap_or_default();
        let default = self
            .get_field_by_separator(&tag_slices, OptDataParser::separator_tags()[1])
            .unwrap_or_default();
        let example = self
            .get_field_by_separator(&tag_slices, OptDataParser::separator_tags()[2])
            .unwrap_or_default();
        let declared_by = self
            .get_field_by_separator(&tag_slices, OptDataParser::separator_tags()[3])
            .unwrap_or_default();

        OptData {
            name,
            description,
            var_type,
            default,
            example,
            declared_by,
        }
    }

    // TODO: Is there a better way to do this? Just keep it inline in self.parse()?
    fn separator_tags() -> [&'static str; 4] {
        [
            r#"<span class="emphasis"><em>Type:</em></span>"#,
            r#"<span class="emphasis"><em>Default:</em></span>"#,
            r#"<span class="emphasis"><em>Example:</em></span>"#,
            r#"<span class="emphasis"><em>Declared by:</em></span>"#,
        ]
    }

    // TODO: We might want to make this into a slightly more refined HTML flattener, in particular treating different HTML tags differently.
    fn get_field(&self, section: &[tl::HTMLTag]) -> String {
        section
            .iter()
            .map(|t| t.inner_text(self.p))
            .fold(String::new(), |acc, e| acc + &e)
    }

    fn get_field_by_separator(
        &self,
        split_tags: &Vec<Vec<tl::HTMLTag>>,
        tag: &str,
    ) -> Option<String> {
        for section in split_tags {
            if let Some(t) = section.first() {
                if t.inner_html(self.p).contains(tag) {
                    return Some(self.get_field(section));
                }
            }
        }
        None
    }

    fn split_tags(&self, // OMG the TYPES
    ) -> Vec<Vec<tl::HTMLTag<'_>>> {
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
        let rev_tags: Vec<_> = dd_tags.iter().rev().collect();
        // Iterator of iterators
        let split_iter = rev_tags
            .split_inclusive(|t| {
                OptDataParser::separator_tags()
                    .iter()
                    .any(|a| t.inner_html(self.p).contains(a))
            })
            .rev()
            .map(|s| s.iter().rev().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        // TODO: Fix all of the type nonsense down here
        split_iter
            .into_iter()
            .map(|v| v.into_iter().map(|&&h| h.clone()).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    }

    fn get_name(&self) -> String {
        self.dt
            .get(self.p)
            .unwrap()
            .inner_text(self.p)
            .trim()
            .to_string()
    }
}

// TODO: Not all of these always show up. Make them optional, or just encode the lack as empty strings?
#[allow(dead_code)]
#[derive(Debug)]
struct OptData {
    name: String,
    description: String,
    var_type: String,
    default: String,
    example: String,
    declared_by: String,
}
