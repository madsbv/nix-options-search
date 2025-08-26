use bitcode::{Decode, Encode};
use color_eyre::eyre::{eyre, Result};
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::logging::cache_dir;
use crate::opt_data::{parse_options, parse_version, OptText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InputStatus {
    Unchanged,
    Append,
    Change,
}

pub struct Finder {
    source: Source,
    version: Arc<OnceLock<String>>,
    searcher: Nucleo<OptText>,
    injection_handle: Option<JoinHandle<()>>,
    pub(crate) results_waiting: Arc<AtomicBool>,
}

impl Finder {
    pub fn new(source: Source) -> Self {
        Self::new_with_data_fn(source, None)
    }

    // Allows for overriding the data source, namely for tests that specifically want to acquire data online or from cache.
    fn new_with_data_fn(
        source: Source,
        data_fn: Option<Box<dyn Fn() -> Result<SourceData> + Send>>,
    ) -> Self {
        let data_fn = data_fn.unwrap_or(Box::new(move || source.get_data()));

        let results_waiting = Arc::new(AtomicBool::new(false));
        let results_sender = Arc::clone(&results_waiting);
        let notify = Arc::new(move || {
            results_sender.store(true, Ordering::Relaxed);
        });
        let version = Arc::new(OnceLock::new());
        let (searcher, handle) = new_searcher(data_fn, version.clone(), notify);
        Finder {
            source,
            version,
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

    pub fn version(&self) -> &str {
        self.version
            .get()
            .map_or("Version number not found (yet)", |s| s)
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

    pub fn source(&self) -> Source {
        self.source
    }

    // For testing purposes
    pub fn find_blocking(
        &mut self,
        pattern: &str,
        max: Option<usize>,
    ) -> std::result::Result<Vec<OptText>, Box<dyn std::any::Any + Send + 'static>> {
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

/// Create a searcher with concurrent parsing and injection of data. Getting data (either through HTTP or cached HTML) and injecting it into Nucleo is done in a separate thread, so we can return the searcher quickly instead of blocking.
fn new_searcher(
    data_fn: Box<dyn Fn() -> Result<SourceData> + Send>,
    version: Arc<OnceLock<String>>,
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
        let opts = if let Ok(data) = data_fn() {
            version.get_or_init(|| data.version);
            data.opts
        } else {
            version.get_or_init(|| "Failed to get data".to_string());
            vec![]
        };

        for d in opts {
            // TODO: Add the right data to search string
            // NOTE: First argument is the "data" part of matched items; use it to store the data you want to get out at the end (e.g. the entire object you're searching for, or an index to it).
            // The second argument is a closure that outputs the text that should be displayed as the user, and which Nucleo matches a given pattern against. For us, that could be the contents of the various fields of OptData in different columns
            inj.push(d, |data, col| col[0] = data.name.clone().into());
        }
    });
    nuc.tick(0);
    (nuc, handle)
}

#[derive(Debug, Copy, Clone, Encode, Decode, PartialEq, strum::VariantArray)]
pub enum Source {
    NixDarwin,
    NixOS,
    NixOSUnstable,
    HomeManager,
    HomeManagerNixOS,
    HomeManagerNixDarwin,
}

impl Source {
    const CACHE_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
    // From docs: Compression level 0 means "use zstd default compression level", currently 3
    const ZSTD_COMPRESSION_LEVEL: i32 = 0;

    fn url(self) -> &'static str {
        match self {
            Self::NixDarwin => "https://nix-darwin.github.io/nix-darwin/manual/index.html",
            Self::NixOS => "https://nixos.org/manual/nixos/stable/options",
            Self::NixOSUnstable => "https://nixos.org/manual/nixos/unstable/options",
            Self::HomeManager => "https://nix-community.github.io/home-manager/options.xhtml",
            Self::HomeManagerNixOS => {
                "https://nix-community.github.io/home-manager/nixos-options.xhtml"
            }
            Self::HomeManagerNixDarwin => {
                "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml"
            }
        }
    }

    fn version_url(self) -> &'static str {
        match self {
            Self::NixDarwin => self.url(),
            Self::NixOS => "https://nixos.org/manual/nixos/stable/",
            Self::NixOSUnstable => "https://nixos.org/manual/nixos/unstable/",
            Self::HomeManager | Self::HomeManagerNixOS | Self::HomeManagerNixDarwin => {
                "https://nix-community.github.io/home-manager/"
            }
        }
    }

    fn url_to(self, opt: &OptText) -> String {
        let tag = match self {
            Self::NixDarwin | Self::NixOS | Self::NixOSUnstable | Self::HomeManager => "opt",
            Self::HomeManagerNixOS => "nixos-opt",
            Self::HomeManagerNixDarwin => "nix-darwin-opt",
        };
        format!("{}#{}-{}", self.url(), tag, opt.name.trim())
    }

    fn cache_path(self) -> PathBuf {
        cache_dir().clone().join(format!("{self}.zst"))
    }

    fn store_cache_to(data: &SourceData, path: &PathBuf) -> Result<()> {
        let bitdata = bitcode::encode(data);
        let zstddata =
            zstd::stream::encode_all(bitdata.as_slice(), Source::ZSTD_COMPRESSION_LEVEL)?;
        std::fs::write(path, zstddata)?;
        Ok(())
    }

    fn store_cache(self, data: &SourceData) -> Result<()> {
        Source::store_cache_to(data, &self.cache_path())
    }

    fn load_cache_from(path: &PathBuf) -> Result<SourceData> {
        let zstddata = std::fs::read(path)?;
        let bitdata = zstd::stream::decode_all(zstddata.as_slice())?;
        let data = bitcode::decode(&bitdata)?;
        Ok(data)
    }

    fn load_cache(self) -> Result<SourceData> {
        Source::load_cache_from(&self.cache_path())
    }

    /// Returns Ok(bool) if and only if there is a readable cache file. The value of the bool depends on the last modified time of the file, as reported by the file system.
    fn cache_is_current(self) -> Result<bool> {
        let f = std::fs::File::open(self.cache_path())?;
        let last_modified = f.metadata()?.modified()?;
        let age = last_modified.elapsed()?;
        Ok(age < Source::CACHE_MAX_AGE)
    }

    fn get_version(self) -> Result<String> {
        let html = ureq::get(self.version_url())
            .call()?
            .body_mut()
            .read_to_string()?;

        let dom = tl::parse(&html, tl::ParserOptions::default())?;

        parse_version(&dom).ok_or(eyre!(
            "Parsing version from html failed. Length of html document: {}",
            html.len()
        ))
    }

    fn get_online_data(self) -> Result<SourceData> {
        let html = ureq::get(self.url())
            .call()?
            .body_mut()
            .with_config()
            // 30 MB reading limit.
            // The default is 10MB, but the nixos docs are 20-21MB, at least uncompressed.
            .limit(30 * 1024 * 1024)
            .read_to_string()?;
        let dom = tl::parse(&html, tl::ParserOptions::default())?;

        Ok(SourceData {
            source: self,
            opts: parse_options(&dom)
                .map(|ok| ok.into_iter().map(std::convert::Into::into).collect())?,
            version: self.get_version()?,
        })
    }

    // We could return a Result or Option to account for possible failure modes, but currently I'm not sure what I'd use it for.
    // Maybe if we return a semantically meaningful error, we can retry HTTP requests occassionally on failure? Exponential backoff
    fn get_data(self) -> Result<SourceData> {
        let cache_validity = self.cache_is_current();
        if let Ok(true) = cache_validity {
            if let Ok(data) = self.load_cache() {
                // Happy path: Just use existing cache
                return Ok(data);
            }
        }
        // Cache is outdated or there was a reading error
        if let Ok(data) = self.get_online_data() {
            // We can get the opts from the web and update the cache
            let _ = self.store_cache(&data);
            return Ok(data);
        }
        // Cache is outdated or broken and we're effectively offline
        if let Ok(false) = cache_validity {
            // If the cache was just outdated, returning the outdated cache is better than nothing
            // If loading the cache fails, other Sources might still work, so avoid crashing the program by unwrapping.
            if let Ok(data) = self.load_cache() {
                return Ok(data);
            }
        }
        // TODO: Embed precomputed cache at compile time to use as a last ditch fallback?
        Err(eyre!("Failed to get data for {self}"))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::NixDarwin => "Nix-Darwin",
            Self::NixOS => "NixOS",
            Self::NixOSUnstable => "NixOS Unstable",
            Self::HomeManager => "Home Manager",
            Self::HomeManagerNixOS => "Home Manager NixOS",
            Self::HomeManagerNixDarwin => "Home Manager Nix-Darwin",
        };
        write!(f, "{s}")
    }
}

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
struct SourceData {
    source: Source,
    opts: Vec<OptText>,
    version: String,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[cfg(feature = "online-tests")]
    use tempfile::tempdir;

    #[test]
    #[cfg(feature = "online-tests")]
    fn test_cache_roundtrip() {
        let s = Source::NixDarwin;
        let Ok(opts) = s.get_online_data() else {
            panic!(
                "Can get and parse options for {s} from the web (tests require network connection)"
            )
        };

        let tmpdir = tempdir().expect("Can create temporary directory");
        let path = tmpdir.path().join(PathBuf::from(format!("{s}.zst")));

        Source::store_cache_to(&opts, &path)
            .expect("Can encode, compress and store cache to local testing directory");
        let roundtrip_opts = Source::load_cache_from(&path).expect(
            "Can read, decompress and decode stored cache data from local testing directory",
        );

        assert_eq!(opts, roundtrip_opts);
    }

    /// Check that we can get, parse and query all online data sources with at least some results.
    #[test]
    #[cfg(feature = "online-tests")]
    fn test_finders() {
        let mut handles = vec![];
        for s in [
            Source::NixDarwin,
            Source::NixOS, // While slow, running this is necessary to sufficiently test the find_blocking method
            Source::HomeManager,
            Source::HomeManagerNixOS,
            Source::HomeManagerNixDarwin,
        ] {
            handles.push((s, std::thread::spawn(move || test_finder(s))));
        }
        for h in handles {
            assert!(
                h.1.join().is_ok(),
                "Searching with Finder for {} failed",
                h.0
            );
        }
    }

    #[cfg(feature = "online-tests")]
    fn test_finder(source: Source) {
        let mut f = Finder::new_with_data_fn(source, Some(Box::new(move || source.get_data())));
        assert_ne!(
            f.find_blocking("s", Some(5))
                .expect("find_blocking should not fail")
                .len(),
            0,
            "Searching with finder from {source} failed"
        );
    }

    #[test]
    #[cfg(feature = "online-tests")]
    fn test_doc_urls_trimmed() {
        // Previously, Source::url_to returned urls with a trailing newline. Still not sure where the newline originates.
        let s = Source::NixDarwin;
        let urls = s
            .get_data()
            .expect("Can get data")
            .opts
            .into_iter()
            .map(|opt| s.url_to(&opt));
        for url in urls {
            assert_eq!(url, url.trim());
            assert_ne!(url.chars().last(), Some('\n'));
        }
    }

    #[test]
    fn test_empty_search() {
        let mut f = Finder::new(Source::NixDarwin);
        assert_eq!(
            f.find_blocking("asdfasdfasdf", Some(5))
                .expect("find blocking should not fail")
                .len(),
            0,
            "Either empty searches crash or a search term that was thought to yield no results now does."
        );
    }

    #[test]
    #[cfg(feature = "online-tests")]
    fn test_get_version() {
        use strum::VariantArray;

        for s in Source::VARIANTS {
            let version = s.get_version().expect("Can get version");
            assert!(version.contains("Version"), "Version string: {version}");
        }
    }
}
