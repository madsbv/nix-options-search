use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo, Utf32String};
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
    searcher: Nucleo<Vec<String>>,
    handle: Option<JoinHandle<()>>,
}

impl Finder {
    pub fn new(source: Source) -> Self {
        let (searcher, handle) = new_searcher(source, true);
        Finder {
            source,
            searcher,
            handle: Some(handle),
        }
    }

    pub fn name(&self) -> String {
        self.source.to_string()
    }

    pub fn init_search(&mut self, pattern: &str) {
        self.searcher.pattern.reparse(
            0,
            pattern,
            CaseMatching::Ignore,
            Normalization::Smart,
            false,
        );
        // TODO: Can we avoid this blocking? E.g. by redrawing the app on a timer?
        while self.searcher.tick(100).running {}
    }

    pub fn get_results(&self, max: Option<usize>) -> Vec<Vec<String>> {
        let snap = self.searcher.snapshot();
        let n = snap.matched_item_count();

        let res = snap.matched_items(0..n).map(|item| item.data).cloned();
        match max {
            Some(n) => res.take(n).collect(),
            None => res.collect(),
        }
    }

    // For testing purposes
    pub fn find_blocking(
        &mut self,
        pattern: &str,
        max: Option<usize>,
    ) -> std::result::Result<Vec<Vec<String>>, Box<(dyn std::any::Any + Send + 'static)>> {
        if let Some(handle) = std::mem::take(&mut self.handle) {
            handle.join()?;
        }
        self.init_search(pattern);
        Ok(self.get_results(max))
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
fn new_searcher(source: Source, try_http: bool) -> (Nucleo<Vec<String>>, JoinHandle<()>) {
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

#[cfg(test)]
mod tests {

    use super::*;

    /// Check that we can parse the valid data, generate a matcher, and that the cached data actually yields roughly the expected number of items.
    #[test]
    fn parse_cached() {
        for s in [
            Source::NixDarwin,
            // Source::NixOS, // Very slow to run, and validity of the cache file is also checked by the app tests.
            Source::HomeManager,
            Source::HomeManagerNixOS,
            Source::HomeManagerNixDarwin,
        ] {
            parse_source_from_cache(s);
        }
    }

    fn parse_source_from_cache(source: Source) {
        let (mut searcher, handle) = new_searcher(source, false);
        handle
            .join()
            .expect("parsing cached data should be infallible");
        while searcher.tick(1000).running {}
        let snap = searcher.snapshot();

        // TODO: Do some actual search comparisons instead
        assert!(snap.item_count() > 5, "Parsing from {source} failed");
    }
}
