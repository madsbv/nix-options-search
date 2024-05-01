use bitcode::{decode, encode};
use color_eyre::eyre::Result;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crate::logging::data_dir;
use crate::opt_data::{parse_options, OptText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputStatus {
    Unchanged,
    Append,
    Change,
}

pub struct Finder {
    source: Source,
    // TODO: Can we optimize memory usage by making this take Cows?
    // If ListableWidgets receive list geometry as input, switch this to storing precomputed ListableOptWidgets instead
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
        let (searcher, handle) = new_searcher(Box::new(move || source.opt_text()), notify);
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

    pub fn url(&self) -> &'static str {
        self.source.url()
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

    pub fn url_to(&self, opt: &OptText) -> String {
        self.source.url_to(opt)
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
    const CACHE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
    const ZSTD_COMPRESSION_LEVEL: i32 = 0;

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

    fn url_to(self, opt: &OptText) -> String {
        let tag = match self {
            Self::NixDarwin | Self::NixOS | Self::HomeManager => "opt",
            Self::HomeManagerNixOS => "nixos-opt",
            Self::HomeManagerNixDarwin => "nix-darwin-opt",
        };
        format!("{}#{}-{}", self.url(), tag, opt.name)
    }

    fn cache_path(self) -> PathBuf {
        let mut path = data_dir().clone();
        path.push(format!("{self}.zst"));
        path
    }

    fn store_cache_to(opts: &[OptText], path: &PathBuf) -> Result<()> {
        let bitdata = encode(opts);
        let zstddata =
            zstd::stream::encode_all(bitdata.as_slice(), Source::ZSTD_COMPRESSION_LEVEL)?;
        std::fs::write(path, zstddata)?;
        Ok(())
    }

    fn store_cache(self, opts: &[OptText]) -> Result<()> {
        Source::store_cache_to(opts, &self.cache_path())
    }

    fn load_cache_from(path: &PathBuf) -> Result<Vec<OptText>> {
        let zstddata = std::fs::read(path)?;
        let bitdata = zstd::stream::decode_all(zstddata.as_slice())?;
        let opts = decode(&bitdata)?;
        Ok(opts)
    }

    fn load_cache(self) -> Result<Vec<OptText>> {
        Source::load_cache_from(&self.cache_path())
    }

    /// Returns Ok(bool) if and only if there is a readable cache file. The value of the bool depends on the last modified time of the file, as reported by the file system.
    fn cache_is_current(self) -> Result<bool> {
        let f = std::fs::File::open(self.cache_path())?;
        let last_modified = f.metadata()?.modified()?;
        let age = last_modified.elapsed()?;
        Ok(age < Source::CACHE_MAX_AGE)
    }

    fn opt_text_from_web(self) -> Result<Vec<OptText>> {
        let res = ureq::get(self.url()).call()?;
        let mut html = String::new();
        res.into_reader().read_to_string(&mut html)?;
        let dom = tl::parse(&html, tl::ParserOptions::default())?;

        parse_options(&dom).map(|ok| ok.into_iter().map(std::convert::Into::into).collect())
    }

    // We could return a Result or Option to account for possible failure modes, but currently I'm not sure what I'd use it for.
    // Maybe if we return a semantically meaningful error, we can retry HTTP requests occassionally on failure? Exponential backoff
    fn opt_text(self) -> Vec<OptText> {
        let cache_age = self.cache_is_current();
        if let Ok(true) = cache_age {
            if let Ok(opts) = self.load_cache() {
                // Happy path: Just use existing cache
                return opts;
            }
        }
        // Cache is outdated or there was a reading error
        if let Ok(opts) = self.opt_text_from_web() {
            // We can get the opts from the web and update the cache
            let _ = self.store_cache(&opts);
            return opts;
        }
        // Cache is outdated or broken and we're effectively offline
        if let Ok(false) = cache_age {
            // If the cache was just outdated, returning the outdated cache is better than nothing
            // If loading the cache fails, other Sources might still work, so avoid crashing the program by unwrapping.
            if let Ok(opts) = self.load_cache() {
                return opts;
            }
        }
        // We have nothing useful to return
        // TODO: Push out an "error" OptText for the user to see?
        // TODO: Embed precomputed cache at compile time to use as a last ditch fallback?
        vec![]
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
    opts_fn: Box<dyn Fn() -> Vec<OptText> + Send>,
    notify: Arc<dyn Fn() + Sync + Send>,
) -> (Nucleo<OptText>, JoinHandle<()>) {
    let mut nuc = Nucleo::<OptText>::new(
        Config::DEFAULT,
        notify,
        // NOTE: There might be room for some optimization in thread allocation here, either by capping the number of threads for each Nucleo instance, or using the multi-column capabilities to merge the instances together.
        None,
        1,
    );
    let inj = nuc.injector();

    let handle = std::thread::spawn(move || {
        let data = opts_fn();
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
    use tempfile::tempdir;

    #[test]
    fn test_cache_roundtrip() {
        let s = Source::NixDarwin;
        let opts = s.opt_text_from_web().expect(
            "Can get and parse options for {s} from the web (tests require network connection)",
        );

        let tmpdir = tempdir().expect("Can create temporary directory");
        let path = tmpdir.path().join(PathBuf::from(format!("{s}.zst")));

        Source::store_cache_to(&opts, &path)
            .expect("Can encode, compress and store cache to local testing directory");
        let roundtrip_opts = Source::load_cache_from(&path).expect(
            "Can read, decompress and decode stored cache data from local testing directory",
        );

        assert_eq!(opts, roundtrip_opts);
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
