use bitcode::{Decode, Encode};
use color_eyre::eyre::Result;
use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{
    consts::BUILTIN_SOURCES,
    project_paths::{self, project_env_name},
};

#[derive(PartialEq, Debug, Deserialize, Serialize)]
pub(crate) struct UserConfig {
    /// Order matters
    pub(super) sources: Vec<SourceConfig>,
    pub(super) use_cache: bool,
    pub(super) auto_refresh_cache: bool,
    #[serde(with = "humantime_serde")]
    pub(super) cache_duration: std::time::Duration,
    pub(super) cache_dir: PathBuf,
    pub(super) enable_logging: bool,
    /// The directives syntax: <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax>
    pub(super) log_level: String,
    pub(super) log_file: PathBuf,
}

// Source specification loaded from user config.
// Combine with global cache config to get an actual source.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Deserialize, Serialize)]
pub(crate) struct SourceConfig {
    /// The name/title of the source
    pub(crate) name: String,
    /// The url with data to parse
    pub(crate) url: String,
    /// An optional url from which to try to parse the version number for the source, if it's not found on the main data page
    pub(crate) version_url: Option<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            sources: BUILTIN_SOURCES.into_iter().cloned().collect(),
            use_cache: true,
            auto_refresh_cache: true,
            cache_duration: Duration::from_secs(7 * 24 * 60 * 60),
            cache_dir: project_paths::default_cache_dir().clone(),
            enable_logging: true,
            log_level: String::from("error"),
            log_file: project_paths::default_log_file().clone(),
        }
    }
}

impl UserConfig {
    fn figment(config_file: &Path) -> Figment {
        Figment::from(Serialized::defaults(UserConfig::default()))
            .merge(Toml::file(config_file))
            .merge(Env::prefixed(format!("{}_", project_env_name()).as_str()))
    }

    pub(super) fn build(config_file: &Path) -> Result<Self> {
        Ok(Self::figment(config_file).extract()?)
    }

    pub(crate) fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

pub(crate) fn default_config_toml() -> String {
    // File paths have to be computed at runtime because of differences between operating systems, if nothing else.
    let def = UserConfig::default();
    format!(
        r#"
# Whether to cache parsed options to disk
use_cache = true

# Whether to automatically refresh cache
auto_refresh_cache = true

# The duration to keep cached results around for before automatically refreshing, if auto_refresh_cache = true.
# Examples of valid duration specifications: "1week", "10days 2hours", "1d 2h 3m"
# For all options see https://docs.rs/humantime/latest/humantime/fn.parse_duration.html
cache_duration = "1week"

# Directory in which to store cached results
cache_dir = "{}"

# Whether to enable logging to file (mostly useful for debugging during development)
enable_logging = true

# Which events to emit to the log.
# Syntax: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#example-syntax
log_level = "error"

# Location of the log file, if used.
log_file = "{}"

### Config sources ###
# Each [[sources]] entry defines a separate config source and corresponding tab in nox.
# The order of entries here determines the order the tabs are displayed in nox.

[[sources]]
# The name of the entry. This will be displayed as the title of the corresponding tab,
# but otherwise has no function.
name = "Nix-Darwin"
# The URL from which to get the data for this entry.
url = "https://nix-darwin.github.io/nix-darwin/manual/index.html"

[[sources]]
name = "NixOS"
url = "https://nixos.org/manual/nixos/stable/options"
# Sometimes, the version number is not contained in the html of the page
# describing the configuration options you might be interested in, but might
# be found in a different page. That can be specified here.
version_url = "https://nixos.org/manual/nixos/stable/"

[[sources]]
# The "NixOS Unstable" name currently triggers special behaviour to fix links to the source
# of each configuration option.
# Namely, leading up to each new NixOS stable release, the Unstable documentation switches to
# linking to the new stable branch of the github.com/nixos/nixpkgs repo before that stable branch
# has been created, resulting in HTTP 404 errors.
# As a special case, for a source with the name "NixOS Unstable", if a substring of the form "release-\d{{2}}\.\d{{2}}"
# (e.g. "release-25.11") is detected in links to the source code defining a given option,
# it is replaced with "nixos-unstable".
name = "NixOS Unstable"
url = "https://nixos.org/manual/nixos/unstable/options"
version_url = "https://nixos.org/manual/nixos/unstable/"

[[sources]]
name = "Home Manager"
url = "https://nix-community.github.io/home-manager/options.xhtml"
version_url = "https://nix-community.github.io/home-manager/"

[[sources]]
name = "Home Manager NixOS"
url = "https://nix-community.github.io/home-manager/nixos-options.xhtml"
version_url = "https://nix-community.github.io/home-manager/"

[[sources]]
name = "Home Manager Nix-Darwin"
url = "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml"
version_url = "https://nix-community.github.io/home-manager/"

[[sources]]
name = "Nix Built-ins"
url = "https://nix.dev/manual/nix/2.28/language/builtins.html"
"#,
        def.cache_dir.display(),
        def.log_file.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assure that the documented default config that might be printed for the user matches the internally used `UserConfig::default()`.
    #[test]
    fn test_documented_default_config_is_correct() -> Result<()> {
        let internal_defaults = UserConfig::default();
        let documented_defaults = toml::from_str::<UserConfig>(&default_config_toml())?;
        // Assert that internal_defaults and documented_defaults are equal, but in such a way that it's easier to read the differences
        if internal_defaults != documented_defaults {
            assert_eq!(internal_defaults.use_cache, documented_defaults.use_cache);
            assert_eq!(
                internal_defaults.auto_refresh_cache,
                documented_defaults.auto_refresh_cache
            );
            assert_eq!(internal_defaults.cache_dir, documented_defaults.cache_dir);
            assert_eq!(
                internal_defaults.cache_duration,
                documented_defaults.cache_duration
            );
            assert_eq!(
                internal_defaults.enable_logging,
                documented_defaults.enable_logging
            );
            assert_eq!(internal_defaults.log_level, documented_defaults.log_level);
            assert_eq!(internal_defaults.log_file, documented_defaults.log_file);
            if internal_defaults.sources != documented_defaults.sources {
                eprintln!("internal_defaults.sources:");
                eprintln!("{:#?}", internal_defaults.sources);
                eprintln!("documented_defaults.sources:");
                eprintln!("{:#?}", documented_defaults.sources);
                panic!("internal_defaults.sources and documented_defaults.sources are different");
            }
            assert_eq!(internal_defaults, documented_defaults);
        }
        Ok(())
    }
}
