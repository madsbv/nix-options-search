#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::similar_names
)]

use anyhow::Result;
use nucleo::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo::Config;

mod opt_data;
use opt_data::{parse_options, OptData};

fn main() -> Result<()> {
    // let body: String = ureq::get("https://daiderd.com/nix-darwin/manual/index.html")
    //     .call()?
    //     .into_string()?;
    // let body = std::fs::read_to_string("data/index.html").unwrap();
    let body = std::fs::read_to_string("data/index-short.html").unwrap();
    let dom = tl::parse(&body, tl::ParserOptions::default())?;
    let opts = parse_options(&dom);

    let opt_strings = opts.iter().map(OptData::to_string);

    let pattern = "documentation";

    let mut nuc =
        nucleo::Nucleo::<String>::new(Config::DEFAULT, std::sync::Arc::new(|| ()), None, 1);
    let inj = nuc.injector();
    for s in opt_strings {
        inj.push(s.clone(), |fill| fill[0] = s.into());
    }
    eprintln!("{}", inj.injected_items());

    nuc.pattern
        .reparse(0, pattern, CaseMatching::Smart, Normalization::Smart, false);

    nuc.tick(10);

    let snap = nuc.snapshot();
    let n = snap.matched_item_count();
    eprintln!("{n}");
    for m in snap.matched_items(0..n) {
        eprintln!("{}", m.data);
    }
    // let matches = fuzzy_match(pattern, opt_strings);

    // for m in matches.iter().take(10) {
    //     eprintln!("{} \n Score: {}\n", m.0, m.1);
    // }

    Ok(())
}

/// convenience function to easily fuzzy match
/// on a (relatively small list of inputs). This is not recommended for building a full tui
/// application that can match large numbers of matches as all matching is done on the current
/// thread, effectively blocking the UI
// Taken from https://github.com/helix-editor/helix/blob/d0bb77447138f5f70f96b174a8f29045a956c8c4/helix-core/src/fuzzy.rs#L4
// Look into optimizations/keeping a global matcher around as a static, like helix does
pub fn fuzzy_match<T: AsRef<str>>(
    pattern: &str,
    items: impl IntoIterator<Item = T>,
) -> Vec<(T, u16)> {
    let mut matcher = nucleo::Matcher::default();
    matcher.config = Config::DEFAULT;
    let pattern = Atom::new(
        pattern,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
        false,
    );
    pattern.match_list(items, &mut matcher)
}
