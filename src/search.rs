use anyhow::{anyhow, Result};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};

use crate::opt_data::{parse_options, OptData};

#[allow(dead_code)]
const NIX_DARWIN_URL: &str = "https://daiderd.com/nix-darwin/manual/index.html";
#[allow(dead_code)]
const NIX_DARWIN_CACHE_PATH: &str = "data/index.html";

#[allow(dead_code)]
pub fn nix_darwin_searcher() -> Result<Nucleo<Vec<String>>> {
    let body: String = ureq::get(NIX_DARWIN_URL).call()?.into_string()?;
    searcher_from_html(&body)
}

#[allow(dead_code)]
pub fn nix_darwin_searcher_from_cache() -> Result<Nucleo<Vec<String>>> {
    let body = std::fs::read_to_string("data/index.html")?;
    searcher_from_html(&body)
}

fn searcher_from_html(html: &str) -> Result<Nucleo<Vec<String>>> {
    let dom = tl::parse(html, tl::ParserOptions::default())?;
    let opts = parse_options(&dom);

    init_nuc(&opts)
}

/// Take a non-empty vector of `OptData` as input. The number of columns is determined by the length of `OptData::fields_as_strings()`
fn init_nuc(data: &[OptData]) -> Result<Nucleo<Vec<String>>> {
    let columns = data
        .first()
        .ok_or(anyhow!(
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

/// Convenience function for doing a blocking search on nuc
#[allow(clippy::module_name_repetitions)]
pub fn search_for<'a, T: Sync + Send + 'a>(
    pattern: &str,
    nuc: &'a mut Nucleo<T>,
    max_results: u32,
) -> Vec<nucleo::Item<'a, T>> {
    nuc.pattern.reparse(
        0,
        dbg!(pattern),
        CaseMatching::Ignore,
        Normalization::Smart,
        false,
    );

    // Blocks until finished
    while nuc.tick(10).running {}

    let snap = nuc.snapshot();
    let n = snap.matched_item_count().min(max_results);

    snap.matched_items(0..n).collect()
}
