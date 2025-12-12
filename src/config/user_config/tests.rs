use super::*;
use std::str::FromStr;

// Note that the behaviour of Path and PathBuf types is platform-dependent, and must therefore be tested differently on each platform.

// Assure that the documented default config that might be printed for the user matches the internally used `UserConfig::default()`.
#[test]
fn documented_default_config_is_correct() -> Result<()> {
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

#[test]
#[cfg(target_family = "unix")]
fn roundtrip_linux_paths() -> Result<()> {
    let conf = UserConfig {
        sources: vec![],
        use_cache: true,
        auto_refresh_cache: true,
        cache_duration: Duration::from_secs(7 * 24 * 60 * 60),
        cache_dir: PathBuf::from_str("/home/runneradmin/.cache/nox")?,
        enable_logging: true,
        log_level: String::from("error"),
        log_file: PathBuf::from_str("logs/nox.log")?,
    };

    let toml = conf.to_toml()?;
    // No escaping necessary so to_toml just puts "" around this
    assert!(toml.contains(r#"cache_dir = "/home/runneradmin/.cache/nox""#));
    assert!(toml.contains(r#"log_file = "logs/nox.log""#));
    let roundtrip_conf = toml::from_str::<UserConfig>(&toml)?;
    assert_eq!(conf, roundtrip_conf);
    Ok(())
}

#[test]
#[cfg(target_family = "unix")]
fn parse_toml_linux_paths() -> Result<()> {
    let toml = r#"
use_cache = true
auto_refresh_cache = false
cache_duration = "2months"
cache_dir = '/home/me/.ca\che/nox'
enable_logging = true
log_level = "warn,nix-options-search=info"
log_file = "äéáßfð·\\comp/.log"

[[sources]]
name = "Some random name"
url = "not even an url"
"#;

    let conf = toml::from_str::<UserConfig>(toml)?;
    assert!(conf.use_cache);
    assert!(!conf.auto_refresh_cache);
    assert!(conf.cache_duration > Duration::from_secs(60 * 60 * 24 * 7 * 7)); // I don't care to verify the exact semantics of a "2months" specification in humantime, but a rough bound is a good sanity check
    assert_eq!(
        conf.cache_dir,
        PathBuf::from_iter(["/", "home", "me", r".ca\che", "nox"]),
        "conf.cache_dir.components() = {:?}",
        conf.cache_dir.components()
    );
    assert!(conf.enable_logging);
    assert_eq!(conf.log_level, String::from("warn,nix-options-search=info"));
    assert_eq!(conf.log_file, PathBuf::from_iter([r"äéáßfð·\comp", ".log"]));
    Ok(())
}

#[test]
#[cfg(target_family = "windows")]
fn roundtrip_windows_paths() -> Result<()> {
    let conf = UserConfig {
        sources: vec![],
        use_cache: true,
        auto_refresh_cache: true,
        cache_duration: Duration::from_secs(7 * 24 * 60 * 60),
        cache_dir: PathBuf::from_str(r"C:\Users\runneradmin\AppData\Local\mvil\nox\cache")?,
        enable_logging: true,
        log_level: String::from("error"),
        log_file: PathBuf::from_str(r"logs\nox.log")?,
    };
    let toml = conf.to_toml()?;
    // Escaping necessary so '' is used
    assert!(toml.contains(r#"cache_dir = 'C:\Users\runneradmin\AppData\Local\mvil\nox\cache'"#));
    assert!(toml.contains(r#"log_file = 'logs\nox.log'"#));
    let roundtrip_conf = toml::from_str::<UserConfig>(&toml)?;
    assert_eq!(conf, roundtrip_conf);
    Ok(())
}

#[test]
fn roundtrip_mixed_paths_windows() -> Result<()> {
    let conf = UserConfig {
        sources: vec![],
        use_cache: true,
        auto_refresh_cache: true,
        cache_duration: Duration::from_secs(7 * 24 * 60 * 60),
        cache_dir: PathBuf::from_str(r"C:\home/runneradmin/.cache/nox")?,
        enable_logging: true,
        log_level: String::from("error"),
        log_file: PathBuf::from_str("logs/nox.log")?,
    };
    let toml = conf.to_toml()?;
    // `/` should also be a path separator on Windows
    assert!(toml.contains(r"cache_dir = 'C:\home/runneradmin/.cache/nox'"));
    assert!(toml.contains(r#"log_file = "logs/nox.log""#));
    let roundtrip_conf = toml::from_str::<UserConfig>(&toml)?;
    assert_eq!(conf, roundtrip_conf);
    Ok(())
}

#[test]
#[cfg(target_family = "windows")]
fn parse_toml_windows_paths() -> Result<()> {
    let toml = r#"
use_cache = true
auto_refresh_cache = false
cache_duration = "2months"
# Deliberately test escaped basic strings (and comments)
cache_dir = "C:\\Users\\me\\cache"
enable_logging = true
log_level = "warn,nix-options-search=info"
log_file = 'äéáßfð·\.log'

[[sources]]
name = "Some random name"
url = "not even an url"
"#;

    let conf = toml::from_str::<UserConfig>(toml)?;
    assert!(conf.use_cache);
    assert!(!conf.auto_refresh_cache);
    assert!(conf.cache_duration > Duration::from_secs(60 * 60 * 24 * 7 * 7)); // I don't care to verify the exact semantics of a "2months" specification in humantime, but a rough bound is a good sanity check
    assert_eq!(
        conf.cache_dir,
        PathBuf::from_iter([r"C:\", "Users", "me", "cache"]),
        "conf.cache_dir.components() = {:?}",
        conf.cache_dir.components()
    );
    assert!(conf.enable_logging);
    assert_eq!(conf.log_level, String::from("warn,nix-options-search=info"));
    assert_eq!(conf.log_file, PathBuf::from_iter(["äéáßfð·", ".log"]));
    Ok(())
}
