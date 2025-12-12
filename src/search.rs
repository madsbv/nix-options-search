use crate::config::SourceConfig;
use crate::parsing::{parse_options, parse_version, OptText};
use bitcode::{Decode, Encode};
use color_eyre::eyre::{eyre, Context, Result};
use lazy_regex::regex_replace_all;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use tl::VDom;
use tracing::{debug, error, instrument};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InputStatus {
    Unchanged,
    Append,
    Change,
}

pub(crate) struct Finder {
    source: Source,
    version: Arc<OnceLock<String>>,
    searcher: Nucleo<OptText>,
    injection_handle: Option<JoinHandle<()>>,
    pub(crate) results_waiting: Arc<AtomicBool>,
}

impl Finder {
    pub(crate) fn new(
        source: Source,
        cache_dir: Option<&'static Path>,
        cache_duration: Option<Duration>,
    ) -> Self {
        Self::new_with_data_fn(source, None, cache_dir, cache_duration)
    }

    // Allows for overriding the data source, namely for tests that specifically want to acquire data online or from cache.
    fn new_with_data_fn(
        source: Source,
        data_fn: Option<Box<dyn Fn() -> Result<SourceData> + Send>>,
        cache_dir: Option<&'static Path>,
        cache_duration: Option<Duration>,
    ) -> Self {
        let source_clone = source.clone();
        let data_fn = data_fn.unwrap_or(Box::new(move || {
            let res = source_clone.get_data(cache_dir, cache_duration);
            if res.is_err() {
                debug!(?res);
            }
            res
        }));

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

    pub(crate) fn name(&self) -> String {
        self.source.to_string()
    }

    pub(crate) fn url(&self) -> &str {
        self.source.url()
    }

    pub(crate) fn version(&self) -> &str {
        self.version
            .get()
            .map_or("Version number not found (yet)", |s| s)
    }

    pub(crate) fn init_search(&mut self, pattern: &str, input_status: InputStatus) {
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

    pub(crate) fn get_results(&self, max: Option<usize>) -> Vec<OptText> {
        let snap = self.searcher.snapshot();
        let n = snap.matched_item_count();

        let res = snap.matched_items(0..n).map(|item| item.data).cloned();
        match max {
            Some(n) => res.take(n).collect(),
            None => res.collect(),
        }
    }

    // For testing purposes
    pub(crate) fn find_blocking(
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

    pub(crate) fn doc_url_to(&self, opt: &OptText) -> String {
        self.source.doc_url_to(opt)
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

#[derive(Debug, Clone, Encode, Decode, PartialEq, Deserialize, Serialize)]
// #[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct Source {
    inner: SourceConfig,
    // cache_duration: Option<Duration>,
    // cache_dir: Option<PathBuf>,
}

impl Source {
    // From docs: Compression level 0 means "use zstd default compression level", currently 3
    const ZSTD_COMPRESSION_LEVEL: i32 = 0;
    pub(crate) fn from(
        source: &SourceConfig,
        // cache_duration: Option<&Duration>,
        // cache_dir: Option<&PathBuf>,
    ) -> Self {
        Self {
            inner: source.clone(),
            // cache_duration: cache_duration.copied(),
            // cache_dir: cache_dir.cloned(),
        }
    }

    fn url(&self) -> &str {
        &self.inner.url
    }

    fn version_url(&self) -> &str {
        self.inner.version_url.as_ref().unwrap_or(&self.inner.url)
    }

    fn doc_url_to(&self, opt: &OptText) -> String {
        format!("{}#{}", self.url(), opt.id)
    }

    /// Returns the path to the cache file for this source if a cache directory has been configured, otherwise None
    fn cache_path(&self, cache_dir: &Path) -> PathBuf {
        cache_dir.join(format!("{self}.zst"))
    }

    #[instrument(err, level = "warn")]
    fn store_cache_to(data: &SourceData, path: &Path) -> Result<()> {
        let bitdata = bitcode::encode(data);
        let zstddata =
            zstd::stream::encode_all(bitdata.as_slice(), Source::ZSTD_COMPRESSION_LEVEL)?;
        std::fs::write(path, zstddata)?;
        Ok(())
    }

    fn store_cache(&self, data: &SourceData, cache_dir: &Path) -> Result<()> {
        Source::store_cache_to(data, &self.cache_path(cache_dir))
    }

    #[instrument(err, level = "warn")]
    fn load_cache_from(path: &Path) -> Result<SourceData> {
        let zstddata = std::fs::read(path)?;
        let bitdata = zstd::stream::decode_all(zstddata.as_slice())?;
        let data = bitcode::decode(&bitdata)?;
        Ok(data)
    }

    fn load_cache(&self, cache_dir: &Path) -> Result<SourceData> {
        Source::load_cache_from(&self.cache_path(cache_dir))
    }

    /// Returns Ok(bool) if and only if there is a readable cache file. The value of the bool depends on the last modified time of the file, as reported by the file system.
    fn cache_is_current(&self, cache_dir: &Path, cache_duration: Option<Duration>) -> Result<bool> {
        let f = std::fs::File::open(self.cache_path(cache_dir))?;
        if let Some(ref max_age) = cache_duration {
            let last_modified = f.metadata()?.modified()?;
            let age = last_modified.elapsed()?;
            Ok(age < *max_age)
        } else {
            Ok(true)
        }
    }

    #[instrument(err, level = "debug")]
    fn get_version(&self, dom: Option<&VDom>) -> Result<String> {
        if let (Some(dom), true) = (dom, self.url() == self.version_url()) {
            parse_version(dom)
        } else {
            let html = ureq::get(self.version_url())
                .call()?
                .body_mut()
                .read_to_string()?;

            let dom = tl::parse(&html, tl::ParserOptions::default())?;
            parse_version(&dom)
        }
        .ok_or(eyre!("Parsing version from html failed"))
    }

    #[instrument(err, level = "debug")]
    fn get_online_data(&self) -> Result<SourceData> {
        let html = ureq::get(self.url())
            .call()?
            .body_mut()
            .with_config()
            // 30 MB reading limit.
            // The default is 10MB, but the nixos docs are 20-21MB, at least uncompressed.
            .limit(30 * 1024 * 1024)
            .read_to_string()?;
        let dom = tl::parse(&html, tl::ParserOptions::default())?;
        let opts =
            parse_options(&dom).map(|ok| ok.into_iter().map(std::convert::Into::into).collect())?;

        let version = self.get_version(Some(&dom)).unwrap_or_else(|err| {
            // Log error on failed version parsing, but keep running
            error!(
                "Parsing version number failed for {}: {err}",
                self.inner.name
            );
            "No version number found".to_string()
        });

        let mut data = SourceData {
            source: self.clone(),
            opts,
            version,
        };
        data.nixos_unstable_declared_by_hack();
        Ok(data)
    }

    #[instrument(err, level = "debug")]
    fn get_data(
        &self,
        cache_dir: Option<&Path>,
        cache_duration: Option<Duration>,
    ) -> Result<SourceData> {
        // 0. No local cache is specified
        let Some(cache_dir) = cache_dir else {
            return self.get_online_data();
        };

        // 1. Try to load a fresh, matching cache
        if let Ok(data) = self.try_load_valid_cache(cache_dir, cache_duration) {
            return Ok(data);
        }

        // 2. Cache is outdated, doesn't match the current SourceConfig, or there was an IO error, get fresh data online
        if let Ok(data) = self.get_online_data() {
            // Update the cache, ignoring any errors
            drop(self.store_cache(&data, cache_dir));
            return Ok(data);
        }

        // 3. Cache is outdated or broken and we're effectively offline
        self.try_load_outdated_cache_by_url_only(cache_dir)
            .wrap_err(format!("Failed to get data for {self}"))
    }

    fn try_load_valid_cache(
        &self,
        cache_dir: &Path,
        cache_duration: Option<Duration>,
    ) -> Result<SourceData> {
        if self.cache_is_current(cache_dir, cache_duration)? {
            let data = self.load_cache(cache_dir)?;
            if data.source == *self {
                // The cached data is for the correct Source and is fresh
                return Ok(data);
            }
        }
        Err(eyre!("Cache is outdated or for the wrong Source"))
    }

    fn try_load_outdated_cache_by_url_only(&self, cache_dir: &Path) -> Result<SourceData> {
        let data = self.load_cache(cache_dir)?;
        if data.source.inner.url == *self.inner.url {
            // The cached data at least comes from the right place, but may be outdated, and version_urls might not match
            return Ok(data);
        }
        Err(eyre!("Cache doesn't exist or is for the wrong Source url"))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.name)
    }
}

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
struct SourceData {
    source: Source,
    opts: Vec<OptText>,
    version: String,
}

impl SourceData {
    fn nixos_unstable_declared_by_hack(&mut self) {
        if self.source.inner.name == "NixOS Unstable" {
            for opt in &mut self.opts {
                opt.declared_by_urls = opt
                    .declared_by_urls
                    .iter()
                    .map(|url| {
                        regex_replace_all!(r#"release-\d{2}\.\d{2}"#, url, "nixos-unstable")
                            .to_string()
                    })
                    .collect();
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "online-tests")]
    use super::*;
    #[cfg(feature = "online-tests")]
    use crate::config::consts;

    #[cfg(feature = "online-tests")]
    use tempfile::tempdir;

    #[test]
    #[cfg(feature = "online-tests")]
    fn test_cache_roundtrip() {
        let s = Source::from(&consts::NIX_DARWIN);
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
        for s in consts::BUILTIN_SOURCES.iter() {
            let s = Source {
                inner: (*s).clone(),
            };
            handles.push((
                s.clone(),
                std::thread::spawn(move || test_finder(&s, None, None)),
            ));
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
    fn test_finder(
        source: &Source,
        cache_dir: Option<&'static Path>,
        cache_duration: Option<Duration>,
    ) {
        let mut f = Finder::new(source.clone(), cache_dir, cache_duration);
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
        let s = Source::from(&consts::NIX_DARWIN);
        let urls = s
            .get_data(None, None)
            .expect("Can get data")
            .opts
            .into_iter()
            .map(|opt| s.doc_url_to(&opt));
        for url in urls {
            assert_eq!(url, url.trim());
            assert_ne!(url.chars().last(), Some('\n'));
        }
    }

    #[test]
    #[cfg(feature = "online-tests")]
    fn test_empty_search() {
        let mut f = Finder::new(Source::from(&consts::NIX_DARWIN), None, None);
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
        use crate::config::consts;

        for s in consts::BUILTIN_SOURCES.iter() {
            let s = Source::from(s);
            let version = s.get_version(None).expect("Can get version");
            assert!(version.contains("Version"), "Version string: {version}");
        }
    }
}
