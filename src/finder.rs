use crate::cache::{CacheConfig, Cacheable};
use crate::parsing::OptText;
use crate::source::{Source, SourceData};
use color_eyre::eyre::Result;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::debug;

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
    #[cfg(test)]
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
    pub(crate) fn new_with_data_fn(
        source: Source,
        data_fn: Option<Box<dyn FnOnce() -> Result<SourceData> + Send>>,
        cache_dir: Option<&'static Path>,
        cache_duration: Option<Duration>,
    ) -> Self {
        let source_clone = source.clone();
        let data_fn = data_fn.unwrap_or(Box::new(move || {
            let res = source_clone.get_data(&CacheConfig {
                file: cache_dir.map(|p| p.join(format!("{source_clone}.zst"))),
                duration: cache_duration,
            });
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
        let (searcher, _handle) = new_searcher(data_fn, version.clone(), notify);
        Finder {
            source,
            version,
            searcher,
            #[cfg(test)]
            #[allow(clippy::used_underscore_binding)]
            injection_handle: Some(_handle),
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

    #[cfg(test)]
    fn finish_injection_blocking(
        &mut self,
    ) -> std::result::Result<(), Box<dyn std::any::Any + Send + 'static>> {
        if let Some(handle) = std::mem::take(&mut self.injection_handle) {
            handle.join()?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn find_blocking(
        &mut self,
        pattern: &str,
        max: Option<usize>,
    ) -> std::result::Result<Vec<OptText>, Box<dyn std::any::Any + Send + 'static>> {
        self.finish_injection_blocking()?;
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
    data_fn: Box<dyn FnOnce() -> Result<SourceData> + Send>,
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test_utils::{create_test_finders, BUILTIN_SOURCES_WITH_HTML};

    /// Check that we can get, parse and query all online data sources with at least some results.
    #[test]
    fn test_finders() {
        for mut finder in create_test_finders() {
            assert_ne!(
                finder
                    .find_blocking("s", Some(5))
                    .expect("find_blocking should not fail")
                    .len(),
                0,
                "Searching with finder from {} failed",
                finder.source
            );
        }
    }

    #[test]
    fn test_empty_search() {
        for swh in BUILTIN_SOURCES_WITH_HTML.iter() {
            // Nix-Darwin
            let data = swh.data.clone();
            let data_fn = Box::new(move || Ok(data.clone()));
            let mut f = Finder::new_with_data_fn(swh.source.clone(), Some(data_fn), None, None);
            assert_eq!(
            f.find_blocking("asdfasdfasdf", Some(5))
                .expect("find blocking should not fail")
                .len(),
            0,
            "Either empty searches crash or a search term that was thought to yield no results now does."
        );
        }
    }
}
