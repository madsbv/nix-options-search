use color_eyre::eyre::{eyre, Result};
use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};
use std::cell::{self, RefCell};
use std::fmt;
use std::thread::JoinHandle;

use crate::opt_data::{parse_options, OptData};

flate!(static NIX_DARWIN_CACHED_HTML: str from "data/nix-darwin-index.html");
flate!(static NIXOS_CACHED_HTML: str from "data/nixos-index.html");
flate!(static HOME_MANAGER_CACHED_HTML: str from "data/home-manager-index.html");
flate!(static HOME_MANAGER_NIXOS_CACHED_HTML: str from "data/home-manager-nixos-index.html");
flate!(static HOME_MANAGER_NIX_DARWIN_CACHED_HTML: str from "data/home-manager-nix-darwin-index.html");

pub struct Finder {
    source: Source,
    searcher: RefCell<Nucleo<Vec<String>>>,
    handle: Option<JoinHandle<()>>,
}

impl Finder {
    pub fn new(source: Source) -> Self {
        let (searcher, handle) = new_searcher_concurrent(source, true);
        Finder {
            source,
            searcher: searcher.into(),
            handle: Some(handle),
        }
    }

    pub fn get_searcher(&self) -> cell::RefMut<'_, Nucleo<Vec<String>>> {
        self.searcher.borrow_mut()
    }

    pub fn name(&self) -> String {
        self.source.to_string()
    }

    // TODO: How to avoid collecting here and returning iterator directly?
    pub fn find(&self, pattern: &str, max: Option<usize>) -> Vec<Vec<String>> {
        let mut nuc = self.get_searcher();
        // TODO: This should not clone
        let res = find(pattern, &mut nuc).cloned();
        match max {
            Some(n) => res.take(n).collect(),
            None => res.collect(),
        }
    }

    // For testing purposes
    pub fn find_blocking(&mut self, pattern: &str, max: Option<usize>) -> std::result::Result<Vec<Vec<String>>, Box<(dyn std::any::Any + Send + 'static)>> {
        if let Some(handle) = std::mem::take(&mut self.handle) {
           handle.join()?;
        }
        Ok(self.find(pattern, max))
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
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::NixDarwin => "Nix-Darwin",
            Self::NixOS => "NixOS",
            Self::HomeManager => "Home Manager",
            Self::HomeManagerNixOS => "Home Manager NixOS",
            Self::HomeManagerNixDarwin => "Home Manager Nix-Darwin",
        };
        write!(f, "{s}")
    }
}

/// Create a searcher with concurrent parsing and injection of data. Getting data (either through HTTP or cached HTML) and injecting it into Nucleo is done in a separate thread, so we can return the searcher quickly instead of blocking.
fn new_searcher_concurrent(
    source: Source,
    try_http: bool,
) -> (Nucleo<Vec<String>>, JoinHandle<()>) {
    let opts = move || {
        if try_http {
            if let Ok(res) = ureq::get(source.url()).call() {
                let mut html = String::new();
                if res.into_reader().read_to_string(&mut html).is_ok() {
                    if let Ok(dom) = tl::parse(&html, tl::ParserOptions::default()) {
                        if let Ok(opts) = parse_options(&dom) {
                            return opts
                                .into_iter()
                                .map(|o| o.fields_as_strings())
                                .collect::<Vec<Vec<String>>>();
                        }
                    }
                }
            }
        }
        let dom =
            tl::parse(source.cache(), tl::ParserOptions::default()).expect("cache should work");
        let opts = parse_options(&dom).expect("cache should work");
        opts.into_iter().map(|o| o.fields_as_strings()).collect()
    };

    // I think we have to hard-code this with concurrent injection
    let columns = OptData::num_fields();

    let mut nuc = Nucleo::<Vec<String>>::new(
        Config::DEFAULT,
        std::sync::Arc::new(|| ()),
        None,
        u32::try_from(columns).expect("number of columns fits in a u32"),
    );
    let inj = nuc.injector();

    let handle = std::thread::spawn(move || {
        let data = opts();
        for mut d in data {
            debug_assert_eq!(columns, d.len());

            let d_strings_clone = d.clone();
            let f = |fill: &mut [Utf32String]| {
                (0..columns).rev().for_each(|i| {
                    fill[i] = d
                        .pop()
                        .expect("all d_strings have the same number of fields")
                        .into();
                });
            };
            // NOTE: First argument is the "data" part of matched items; use it to store the data you want to get out at the end (e.g. the entire object you're searching for, or an index to it).
            // The second argument is a closure that outputs the text that should be displayed as the user, and which Nucleo matches a given pattern against. For us, that could be the contents of the various fields of OptData in different columns
            inj.push(d_strings_clone, f);
        }
    });

    nuc.tick(0);
    (nuc, handle)
}

/// Create a new searcher for `source`, with the option to try getting the live html for `source` or going straight to cache.
/// Blocks until the matcher is completely initialized and has parsed all data, which can take multiple seconds especially for the nixOS data.
// We keep this around for testing
#[allow(dead_code)]
fn new_searcher(source: Source, try_http: bool) -> Nucleo<Vec<String>> {
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

/// Get a searcher from raw HTML string.
/// Blocks while parsing HTML and injecting data to searcher, but doesn't wait for the data to be processed.
fn searcher_from_html(html: &str) -> Result<Nucleo<Vec<String>>> {
    let dom = tl::parse(html, tl::ParserOptions::default())?;
    let opts = parse_options(&dom)?;

    init_nuc(&opts)
}

/// Take a non-empty vector of `OptData` as input. The number of columns is determined by the length of `OptData::fields_as_strings()`
/// Blocks while injecting data into Nucleo, but doesn't wait for the data to be processed.
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
    nuc.tick(0);
    Ok(nuc)
}

/// Convenience function for doing a blocking search on nuc. The best match is first in the output.
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

    // TODO: Duplicate the tests above for new_searcher_concurrent.

    /// Test that the concurrently created searcher agrees with one created using blocking methods (easier to reason about). Helps verify that we inject data into the concurrently created searcher correctly.
    #[test]
    fn new_searcher_concurrent_correct() {
        let mut searcher = new_searcher(Source::NixDarwin, false);
        let (mut searcher_concurrent, handle) = new_searcher_concurrent(Source::NixDarwin, false);
        searcher.tick(0);
        handle
            .join()
            .expect("parsing cached data should be infallible");
        while searcher.tick(1000).running || searcher_concurrent.tick(1000).running {}
        let snap = searcher.snapshot();
        let snap_concurrent = searcher_concurrent.snapshot();

        // TODO: Do some actual search comparisons instead
        assert_eq!(snap_concurrent.item_count(), snap.item_count());
    }
}
