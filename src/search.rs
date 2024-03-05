use color_eyre::eyre::{eyre, Result};
use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};
use std::cell::{self, RefCell};

use crate::opt_data::{parse_options, OptData};

// TODO: init_nuc is actually kind of concurrent, in that it injects everything into the matcher, but doesn't block while the matcher actually does its thing. So maybe creating the matcher immediately, and letting it do this matching in the background, is the way to go. We could even double down on this, by getting the HTML concurrently with creating the matcher, then doing the injection and tick in a separate thread while returning the matcher immediately. For this we should probably make ureq give a Reader. Can we make tl take and give a reader?
// Alternatively, we can make the html parsing step return a function that returns the parsed html instead, thus deferring the parsing. We can embed the cache fallback in this function if we do it rigiht.
// How do we handle fallback to cache with this approach?
// Maybe we need to do some OnceCell/OnceLock stuff. Would help with concurrency as well.
// Once implemented, we do want to find a way to make sure that get_matcher blocks until initialization is done.

flate!(static NIX_DARWIN_CACHED_HTML: str from "data/nix-darwin-index.html");
flate!(static NIXOS_CACHED_HTML: str from "data/nixos-index.html");
flate!(static HOME_MANAGER_CACHED_HTML: str from "data/home-manager-index.html");
flate!(static HOME_MANAGER_NIXOS_CACHED_HTML: str from "data/home-manager-nixos-index.html");
flate!(static HOME_MANAGER_NIX_DARWIN_CACHED_HTML: str from "data/home-manager-nix-darwin-index.html");

// TODO: Figure out how to make matchers lazy load
pub struct Finder {
    source: Source,
    searcher: RefCell<Nucleo<Vec<String>>>,
}

impl Finder {
    pub fn new(source: Source) -> Self {
        Finder {
            source,
            searcher: new_searcher(source, true).into(),
        }
    }

    pub fn get_searcher(&self) -> cell::RefMut<'_, Nucleo<Vec<String>>> {
        self.searcher.borrow_mut()
    }

    pub fn name(&self) -> &'static str {
        self.source.name()
    }

    pub fn find(&self, pattern: &str, max: Option<usize>) -> Vec<Vec<String>> {
        let mut nuc = self.get_searcher();
        // TODO: This should not clone
        let res = find(pattern, &mut nuc).cloned();
        match max {
            Some(n) => res.take(n).collect(),
            None => res.collect(),
        }
    }
}

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

    // Todo: We can just make this an implementation of `std::fmt::Display`
    pub fn name(self) -> &'static str {
        match self {
            Self::NixDarwin => "nix-darwin",
            Self::NixOS => "nixOS",
            Self::HomeManager => "home-manager",
            Self::HomeManagerNixOS => "home-manager-nixOS",
            Self::HomeManagerNixDarwin => "home-manager-nix-darwin",
        }
    }
}

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
pub fn find<'a, T: Sync + Send + Clone>(
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
