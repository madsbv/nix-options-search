use super::user_config::SourceConfig;
use std::sync::LazyLock;

pub(crate) static NIX_DARWIN: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "Nix-Darwin".to_string(),
    url: "https://nix-darwin.github.io/nix-darwin/manual/index.html".to_string(),
    version_url: None,
});
pub(crate) static NIX_OS: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "NixOS".to_string(),
    url: "https://nixos.org/manual/nixos/stable/options".to_string(),
    version_url: Some("https://nixos.org/manual/nixos/stable/".to_string()),
});
pub(crate) static NIXOS_UNSTABLE: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "NixOS Unstable".to_string(),
    url: "https://nixos.org/manual/nixos/unstable/options".to_string(),
    version_url: Some("https://nixos.org/manual/nixos/unstable/".to_string()),
});
pub(crate) static HOMEMANAGER: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "Home Manager".to_string(),
    url: "https://nix-community.github.io/home-manager/options.xhtml".to_string(),
    version_url: Some("https://nix-community.github.io/home-manager/".to_string()),
});
pub(crate) static HOMEMANAGER_NIXOS: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "Home Manager NixOS".to_string(),
    url: "https://nix-community.github.io/home-manager/nixos-options.xhtml".to_string(),
    version_url: Some("https://nix-community.github.io/home-manager/".to_string()),
});
pub(crate) static HOMEMANAGER_NIX_DARWIN: LazyLock<SourceConfig> = LazyLock::new(|| SourceConfig {
    name: "Home Manager Nix-Darwin".to_string(),
    url: "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml".to_string(),
    version_url: Some("https://nix-community.github.io/home-manager/".to_string()),
});

pub(crate) static BUILTIN_SOURCES: LazyLock<[&'static SourceConfig; 6]> = LazyLock::new(|| {
    [
        &NIX_DARWIN,
        &NIX_OS,
        &NIXOS_UNSTABLE,
        &HOMEMANAGER,
        &HOMEMANAGER_NIXOS,
        &HOMEMANAGER_NIX_DARWIN,
    ]
});
