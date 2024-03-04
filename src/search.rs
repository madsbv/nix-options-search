use color_eyre::eyre::{eyre, Result};
use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};

use crate::opt_data::{parse_options, OptData};

const NIX_DARWIN_URL: &str = "https://daiderd.com/nix-darwin/manual/index.html";
flate!(static NIX_DARWIN_CACHED_HTML: str from "data/nix-darwin-index.html");

pub fn nix_darwin_searcher() -> Result<Nucleo<Vec<String>>> {
    let body: String = ureq::get(NIX_DARWIN_URL).call()?.into_string()?;
    searcher_from_html(&body)
}

pub fn nix_darwin_searcher_from_cache() -> Result<Nucleo<Vec<String>>> {
    searcher_from_html(&NIX_DARWIN_CACHED_HTML)
}

fn searcher_from_html(html: &str) -> Result<Nucleo<Vec<String>>> {
    let dom = tl::parse(html, tl::ParserOptions::default())?;
    let opts = parse_options(&dom)?;

    init_nuc(&opts)
}

/// Take a non-empty vector of `OptData` as input. The number of columns is determined by the length of `OptData::fields_as_strings()`
fn init_nuc(data: &[OptData]) -> Result<Nucleo<Vec<String>>> {
    let columns = data
        .first()
        .ok_or(eyre!(
            "the collection of data injected to the searcher should be non-empty"
        ))?
        .fields_as_strings()
        .len();
    let mut nuc = Nucleo::<Vec<String>>::new(
        Config::DEFAULT,
        std::sync::Arc::new(|| ()),
        None,
        u32::try_from(columns)?,
    );
    let inj = nuc.injector();
    for d in data {
        let mut d_strings = d.fields_as_strings();
        debug_assert_eq!(columns, d_strings.len());

        let d_strings_clone = d_strings.clone();
        let f = |fill: &mut [Utf32String]| {
            (0..columns).rev().for_each(|i| {
                fill[i] = d_strings
                    .pop()
                    .expect("all d_strings have the same number of fields")
                    .into();
            });
        };
        // NOTE: First argument is the "data" part of matched items; use it to store the data you want to get out at the end (e.g. the entire object you're searching for, or an index to it).
        // The second argument is a closure that outputs the text that should be displayed as the user, and which Nucleo matches a given pattern against. For us, that could be the contents of the various fields of OptData in different columns
        inj.push(d_strings_clone, f);
    }
    nuc.tick(1);
    Ok(nuc)
}

/// Convenience function for doing a blocking search on nuc. The best match is first in the output.
#[allow(clippy::module_name_repetitions)]
pub fn search_for<'a, T: Sync + Send + Clone>(
    pattern: &str,
    nuc: &'a mut Nucleo<T>,
) -> impl Iterator<Item = &'a T> + 'a {
    nuc.pattern.reparse(
        0,
        pattern,
        CaseMatching::Ignore,
        Normalization::Smart,
        false,
    );

    // Blocks until finished
    while nuc.tick(10).running {}

    let snap = nuc.snapshot();
    let n = snap.matched_item_count();

    snap.matched_items(0..n).map(|item| item.data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check that we can parse the valid data, generate a matcher, and that the cached data actually yields roughly the expected number of items.
    #[test]
    fn parse_cached_darwin() {
        let matcher = nix_darwin_searcher_from_cache()
            .expect("cached data should be parsable and generate a matcher");
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 100);
    }
}
