use crate::cache::Cacheable;
use crate::config::SourceConfig;
use crate::parsing::{parse_options, parse_version, OptText};
use bitcode::{Decode, Encode};
use color_eyre::eyre::Result;
use lazy_regex::regex_replace_all;
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{error, instrument};

#[derive(Debug, Clone, Encode, Decode, PartialEq, Deserialize, Serialize)]
pub(crate) struct Source {
    pub(crate) inner: SourceConfig,
}

impl Cacheable for Source {
    type WithData = SourceData;

    fn get_expensive(&self) -> Result<Self::WithData> {
        self.get_online_data()
    }

    fn cache_valid(&self, data: &Self::WithData) -> crate::cache::CacheValidity {
        if data.source == *self {
            crate::cache::CacheValidity::Good
        } else if data.source.url() == self.url() {
            crate::cache::CacheValidity::Fallback
        } else {
            crate::cache::CacheValidity::Unusable
        }
    }
}

impl Source {
    // From docs: Compression level 0 means "use zstd default compression level", currently 3
    pub(crate) fn from(source: &SourceConfig) -> Self {
        Self {
            inner: source.clone(),
        }
    }

    pub(crate) fn url(&self) -> &str {
        &self.inner.url
    }

    pub(crate) fn version_url(&self) -> &str {
        self.inner.version_url.as_ref().unwrap_or(&self.inner.url)
    }

    pub(crate) fn doc_url_to(&self, opt: &OptText) -> String {
        format!("{}#{}", self.url(), opt.id)
    }

    pub(crate) fn get_data_html(&self) -> Result<String> {
        Ok(ureq::get(self.url())
            .call()?
            .body_mut()
            .with_config()
            // 30 MB reading limit.
            // The default is 10MB, but the nixos docs are 20-21MB, at least uncompressed.
            .limit(30 * 1024 * 1024)
            .read_to_string()?)
    }

    pub(crate) fn get_version_html(&self) -> Result<String> {
        Ok(ureq::get(self.version_url())
            .call()?
            .body_mut()
            .read_to_string()?)
    }

    pub(crate) fn parse_data(&self, data_html: &str, version_html: &str) -> Result<SourceData> {
        let opts = parse_options(data_html)?;

        let version = match parse_version(version_html) {
            Ok(Some(version)) => version,
            Ok(None) => "No version number found".to_string(),
            Err(err) => {
                // Log error on failed version parsing, but keep running
                error!(
                    "Parsing version number failed for {}: {err}",
                    self.inner.name
                );
                "Error parsing version".to_string()
            }
        };

        let mut data = SourceData {
            source: self.clone(),
            opts,
            version,
        };
        data.nixos_unstable_declared_by_hack();
        Ok(data)
    }

    #[instrument(err, level = "debug")]
    pub(crate) fn get_online_data(&self) -> Result<SourceData> {
        let data_html = self.get_data_html()?;
        let version_html = if self.url() == self.version_url() {
            &data_html
        } else {
            &self.get_version_html()?
        };
        self.parse_data(&data_html, version_html)
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner.name)
    }
}

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
pub(crate) struct SourceData {
    pub(crate) source: Source,
    pub(crate) opts: Vec<OptText>,
    pub(crate) version: String,
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

// #[cfg(test)]
// mod tests {
//     #[cfg(feature = "online-tests")]
//     use super::*;
//     #[cfg(feature = "online-tests")]
//     use crate::config::consts;

//     #[test]
//     #[cfg(feature = "online-tests")]
//     fn test_cache_roundtrip() {
//         use std::path::PathBuf;
//         use tempfile::tempdir;

//         let s = Source::from(&consts::NIX_DARWIN);
//         let Ok(opts) = s.get_online_data() else {
//             panic!(
//                 "Can get and parse options for {s} from the web (tests require network connection)"
//             )
//         };

//         let tmpdir = tempdir().expect("Can create temporary directory");
//         let path = tmpdir.path().join(PathBuf::from(format!("{s}.zst")));

//         Source::store_cache_to(&opts, &path)
//             .expect("Can encode, compress and store cache to local testing directory");
//         let roundtrip_opts = Source::load_cache_from(&path).expect(
//             "Can read, decompress and decode stored cache data from local testing directory",
//         );

//         assert_eq!(opts, roundtrip_opts);
//     }

//     #[test]
//     #[cfg(feature = "online-tests")]
//     fn test_doc_urls_trimmed() {
//         // Previously, Source::url_to returned urls with a trailing newline. Still not sure where the newline originates.
//         let s = Source::from(&consts::NIX_DARWIN);
//         let urls = s
//             .get_data(None, None)
//             .expect("Can get data")
//             .opts
//             .into_iter()
//             .map(|opt| s.doc_url_to(&opt));
//         for url in urls {
//             assert_eq!(url, url.trim());
//             assert_ne!(url.chars().last(), Some('\n'));
//         }
//     }

//     #[test]
//     #[cfg(feature = "online-tests")]
//     fn test_get_version() {
//         use crate::config::consts;

//         for s in consts::BUILTIN_SOURCES.iter() {
//             let s = Source::from(s);
//             let version = s.get_version(None).expect("Can get version");
//             assert!(version.contains("Version"), "Version string: {version}");
//         }
//     }
// }
