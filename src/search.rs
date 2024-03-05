use color_eyre::eyre::{eyre, Result};
use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};

use crate::opt_data::{parse_options, OptData};

// TODO: Arguably the SearchPage struct from crate::app should instead be something like a Finder struct here which we then import there.

// TODO: Remove once searchers have been added and integrated.
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Source {
    NixDarwin,
    NixOS,
    HomeManager,
    HomeManagerNixOS,
    HomeManagerNixDarwin,
}

impl Source {
    fn url(self) -> &'static str {
        match self {
            Self::NixDarwin => "https://daiderd.com/nix-darwin/manual/index.html",
            Self::NixOS => "https://nixos.org/manual/nixos/stable/options",
            Self::HomeManager => "https://nix-community.github.io/home-manager/options.xhtml",
            Self::HomeManagerNixOS => {
                "https://nix-community.github.io/home-manager/nixos-options.xhtml"
            }
            Self::HomeManagerNixDarwin => {
                "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml"
            }
        }
    }

    fn cache(self) -> &'static str {
        match self {
            Self::NixDarwin => &NIX_DARWIN_CACHED_HTML,
            Self::NixOS => &NIXOS_CACHED_HTML,
            Self::HomeManager => &HOME_MANAGER_CACHED_HTML,
            Self::HomeManagerNixOS => &HOME_MANAGER_NIXOS_CACHED_HTML,
            Self::HomeManagerNixDarwin => &HOME_MANAGER_NIX_DARWIN_CACHED_HTML,
        }
    }
}

flate!(static NIX_DARWIN_CACHED_HTML: str from "data/nix-darwin-index.html");
flate!(static NIXOS_CACHED_HTML: str from "data/nixos-index.html");
flate!(static HOME_MANAGER_CACHED_HTML: str from "data/home-manager-index.html");
flate!(static HOME_MANAGER_NIXOS_CACHED_HTML: str from "data/home-manager-nixos-index.html");
flate!(static HOME_MANAGER_NIX_DARWIN_CACHED_HTML: str from "data/home-manager-nix-darwin-index.html");

// TODO: Should we rather compress these on disk and take a libflate/zstd dependency (so the source of this package takes up less space)?
// We could have a 'raw assets' folder in git for version management, and a 'compressed assets' folder that's generated fro the raw assets, and included in the crates.io package and the binary.

pub fn new_searcher(source: Source, try_http: bool) -> Nucleo<Vec<String>> {
    if try_http {
        if let Ok(res) = ureq::get(source.url()).call() {
            let mut html = String::new();
            if res.into_reader().read_to_string(&mut html).is_ok() {
                if let Ok(searcher) = searcher_from_html(&html) {
                    return searcher;
                }
            }
        }
    }
    searcher_from_html(source.cache()).expect("searcher from cache should always work")
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
        let mut matcher = new_searcher(Source::NixDarwin, false);
        // Make sure the matcher is fully initialized before taking a snapshot
        while matcher.tick(1000).running {}
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 100);
    }

    #[test]
    fn parse_cached_nixos() {
        let mut matcher = new_searcher(Source::NixOS, false);
        // Make sure the matcher is fully initialized before taking a snapshot
        while matcher.tick(1000).running {}
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 10000);
    }

    #[test]
    fn parse_cached_home_manager() {
        let mut matcher = new_searcher(Source::HomeManager, false);
        // Make sure the matcher is fully initialized before taking a snapshot
        while matcher.tick(1000).running {}
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 100);
    }

    #[test]
    fn parse_cached_home_manager_nixos() {
        let mut matcher = new_searcher(Source::HomeManagerNixOS, false);
        // Make sure the matcher is fully initialized before taking a snapshot
        while matcher.tick(1000).running {}
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 5);
    }

    #[test]
    fn parse_cached_home_manager_darwin() {
        let mut matcher = new_searcher(Source::HomeManagerNixDarwin, false);
        // Make sure the matcher is fully initialized before taking a snapshot
        while matcher.tick(1000).running {}
        let snap = matcher.snapshot();
        assert!(snap.item_count() > 5);
    }
}
