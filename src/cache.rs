use crate::config::AppConfig;
use color_eyre::eyre::{eyre, Result};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) fn initialize_cache_dir(config: &AppConfig) -> Result<()> {
    if let Some(dir) = &config.cache_dir {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

pub(crate) struct CacheConfig {
    pub(crate) file: Option<PathBuf>,
    pub(crate) duration: Option<Duration>,
}

/// The possible statuses of a cache file
pub(crate) enum CacheStatus {
    Fresh,
    Outdated,
    Missing,
    /// Cache status was requested but the given config does not define a cache directory
    Undefined,
}

/// The validity states of cached data, independently of age considerations
pub(crate) enum CacheValidity {
    /// Use cache directly
    Good,
    /// Try to get fresh data, but if that fails, the cache may be used as a fallback
    Fallback,
    /// Even if a cache exists and can be read, its data is invalid and should not be used even as fallback
    Unusable,
}

pub(crate) trait Cacheable {
    type WithData: bitcode::Encode + for<'a> bitcode::Decode<'a>;
    const ZSTD_COMPRESSION_LEVEL: i32 = 0;

    fn get_expensive(&self) -> Result<Self::WithData>;
    fn cache_valid(&self, data: &Self::WithData) -> CacheValidity;

    fn store_cache(data: &Self::WithData, cache_file: &Path) -> Result<()> {
        let bitdata = bitcode::encode(data);
        let zstddata = zstd::stream::encode_all(bitdata.as_slice(), Self::ZSTD_COMPRESSION_LEVEL)?;
        std::fs::write(cache_file, zstddata)?;
        Ok(())
    }

    fn load_cache(path: &Path) -> Result<Self::WithData> {
        let zstddata = std::fs::read(path)?;
        let bitdata = zstd::stream::decode_all(zstddata.as_slice())?;
        let data = bitcode::decode(&bitdata)?;
        Ok(data)
    }

    /// Returns Ok(status) unless an underlying system error occurs.
    fn cache_status(&self, config: &CacheConfig) -> Result<CacheStatus> {
        let Some(ref cache_file) = config.file else {
            return Ok(CacheStatus::Undefined);
        };
        if !std::fs::exists(cache_file)? {
            return Ok(CacheStatus::Missing);
        }
        let f = std::fs::File::open(cache_file)?;
        let Some(max_age) = config.duration else {
            return Ok(CacheStatus::Fresh);
        };

        let last_modified = f.metadata()?.modified()?;
        let age = last_modified.elapsed()?;
        Ok(if age < max_age {
            CacheStatus::Fresh
        } else {
            CacheStatus::Outdated
        })
    }

    fn maybe_load_cache(&self, config: &CacheConfig) -> MaybeCache<Self::WithData> {
        let (Some(cache_path), Ok(status)) = (&config.file, self.cache_status(config)) else {
            return MaybeCache::None;
        };

        match status {
            CacheStatus::Fresh => {
                if let Ok(data) = Self::load_cache(cache_path) {
                    match self.cache_valid(&data) {
                        CacheValidity::Good => return MaybeCache::Good(data),
                        CacheValidity::Fallback => return MaybeCache::Fallback(data),
                        CacheValidity::Unusable => (),
                    }
                }
            }
            CacheStatus::Outdated => return MaybeCache::Outdated,
            _ => (),
        }
        MaybeCache::None
    }

    fn get_data(&self, config: &CacheConfig) -> Result<Self::WithData> {
        let maybe_cache = self.maybe_load_cache(config);
        if let MaybeCache::Good(data) = maybe_cache {
            return Ok(data);
        }

        if let Ok(data) = self.get_expensive() {
            // Cache is outdated, missing, or doesn't fully match with Self, but we can get fresh data
            if let Some(cache_path) = &config.file {
                // Update the cache, ignoring any errors
                drop(Self::store_cache(&data, cache_path));
            }
            return Ok(data);
        }

        match maybe_cache {
            MaybeCache::Outdated => {
                if let Some(cache_path) = &config.file {
                    if let Ok(data) = Self::load_cache(cache_path) {
                        match self.cache_valid(&data) {
                            CacheValidity::Good | CacheValidity::Fallback => return Ok(data),
                            CacheValidity::Unusable => (),
                        }
                    }
                }
            }
            MaybeCache::Good(_) => unreachable!(),
            MaybeCache::Fallback(data) => return Ok(data),
            MaybeCache::None => (),
        }
        Err(eyre!("Failed to get fresh data and no valid cache found"))
    }
}
pub(crate) enum MaybeCache<T> {
    Outdated,
    Good(T),
    Fallback(T),
    None,
}
