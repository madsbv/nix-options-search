use color_eyre::eyre::Result;
use include_flate::flate;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::opt_data::{parse_options, OptText};

flate!(static NIX_DARWIN_CACHED_HTML: str from "data/nix-darwin-index.html");
flate!(static NIXOS_CACHED_HTML: str from "data/nixos-index.html");
flate!(static HOME_MANAGER_CACHED_HTML: str from "data/home-manager-index.html");
flate!(static HOME_MANAGER_NIXOS_CACHED_HTML: str from "data/home-manager-nixos-index.html");
flate!(static HOME_MANAGER_NIX_DARWIN_CACHED_HTML: str from "data/home-manager-nix-darwin-index.html");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputStatus {
    Unchanged,
    Append,
    Change,
}

pub struct Finder {
    source: Source,
    // TODO: Can we optimize memory usage by making this take Cows?
    searcher: Nucleo<OptText>,
    injection_handle: Option<JoinHandle<()>>,
    pub(crate) results_waiting: Arc<AtomicBool>,
}

impl Finder {
    pub fn new(source: Source) -> Self {
        let results_waiting = Arc::new(AtomicBool::new(false));
        let results_sender = Arc::clone(&results_waiting);
        let notify = Arc::new(move || {
            results_sender.store(true, Ordering::Relaxed);
        });
        let (searcher, handle) = new_searcher(source, true, notify);
        Finder {
            source,
            searcher,
            injection_handle: Some(handle),
            results_waiting,
        }
    }

    pub fn name(&self) -> String {
        self.source.to_string()
    }

    pub fn init_search(&mut self, pattern: &str, input_status: InputStatus) {
        if input_status != InputStatus::Unchanged {
            self.searcher.pattern.reparse(
                0,
                pattern,
                CaseMatching::Ignore,
                Normalization::Smart,
                // NOTE: As far as I can tell, the optimization that this enables is that if we append to the search string, then any item that had score 0 before will still have score 0, so we don't have to rerun scoring against those items. We still run scoring as usual against all other items.
                input_status == InputStatus::Append,
            );
        }
        self.searcher.tick(10);
    }

    pub fn get_results(&self, max: Option<usize>) -> Vec<OptText> {
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
    ) -> std::result::Result<Vec<OptText>, Box<(dyn std::any::Any + Send + 'static)>> {
        if let Some(handle) = std::mem::take(&mut self.injection_handle) {
            handle.join()?;
        }
        self.init_search(pattern, InputStatus::Change);
        while self.searcher.tick(1000).running {}
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

    pub(crate) fn cache(self) -> &'static str {
        match self {
            Self::NixDarwin => &NIX_DARWIN_CACHED_HTML,
            Self::NixOS => &NIXOS_CACHED_HTML,
            Self::HomeManager => &HOME_MANAGER_CACHED_HTML,
            Self::HomeManagerNixOS => &HOME_MANAGER_NIXOS_CACHED_HTML,
            Self::HomeManagerNixDarwin => &HOME_MANAGER_NIX_DARWIN_CACHED_HTML,
        }
    }

    pub(crate) fn opt_text_from_cache(self) -> Vec<OptText> {
        let dom = tl::parse(self.cache(), tl::ParserOptions::default()).expect("cache should work");
        parse_options(&dom)
            .expect("cache should work")
            .into_iter()
            .map(std::convert::Into::into)
            .collect()
    }

    pub(crate) fn opt_text_from_url(self) -> Result<Vec<OptText>> {
        let res = ureq::get(self.url()).call()?;
        let mut html = String::new();
        res.into_reader().read_to_string(&mut html)?;
        let dom = tl::parse(&html, tl::ParserOptions::default())?;

        parse_options(&dom).map(|ok| ok.into_iter().map(std::convert::Into::into).collect())
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
fn new_searcher(
    source: Source,
    try_http: bool,
    notify: Arc<dyn Fn() + Sync + Send>,
) -> (Nucleo<OptText>, JoinHandle<()>) {
    let opts = move || {
        if try_http {
            source
                .opt_text_from_url()
                .unwrap_or(source.opt_text_from_cache())
        } else {
            source.opt_text_from_cache()
        }
    };

    let mut nuc = Nucleo::<OptText>::new(
        Config::DEFAULT,
        notify,
        // NOTE: There might be room for some optimization in thread allocation here, either by capping the number of threads for each Nucleo instance, or using the multi-column capabilities to merge the instances together.
        None,
        1,
    );
    let inj = nuc.injector();

    let handle = std::thread::spawn(move || {
        let data = opts();
        for d in data {
            // TODO: Add the right data to search string
            // NOTE: First argument is the "data" part of matched items; use it to store the data you want to get out at the end (e.g. the entire object you're searching for, or an index to it).
            // The second argument is a closure that outputs the text that should be displayed as the user, and which Nucleo matches a given pattern against. For us, that could be the contents of the various fields of OptData in different columns
            inj.push(d, |data, col| col[0] = data.name.clone().into());
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
    fn parse_caches() {
        let mut handles = vec![];
        for s in [
            Source::NixDarwin,
            Source::NixOS,
            Source::HomeManager,
            Source::HomeManagerNixOS,
            Source::HomeManagerNixDarwin,
        ] {
            handles.push((s, std::thread::spawn(move || parse_source_from_cache(s))));
        }
        for h in handles {
            assert!(h.1.join().is_ok(), "Parsing cache for {} failed", h.0);
        }
    }

    fn parse_source_from_cache(source: Source) {
        let (mut searcher, handle) = new_searcher(source, false, Arc::new(|| {}));
        handle
            .join()
            .expect("parsing cached data should be infallible");
        while searcher.tick(1000).running {}
        let snap = searcher.snapshot();

        // TODO: Do some actual search comparisons instead
        assert!(snap.item_count() > 5, "Parsing from {source} failed");
    }

    /// Check that we can parse the valid data, generate a matcher, and that the cached data actually yields roughly the expected number of items.
    #[test]
    fn test_finders() {
        let mut handles = vec![];
        for s in [
            Source::NixDarwin,
            Source::NixOS, // While slow, running this is necessary to sufficiently test the find_blocking method
            Source::HomeManager,
            Source::HomeManagerNixOS,
            Source::HomeManagerNixDarwin,
        ] {
            handles.push((s, std::thread::spawn(move || finder(s))));
        }
        for h in handles {
            assert!(
                h.1.join().is_ok(),
                "Searching with Finder for {} failed",
                h.0
            );
        }
    }

    fn finder(source: Source) {
        let mut f = Finder::new(source);
        assert_ne!(
            f.find_blocking("s", Some(5))
                .expect("find_blocking should not fail")
                .len(),
            0,
            "Searching with finder from {source} failed"
        );
    }
}
