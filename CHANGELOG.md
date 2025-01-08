# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2025-01-07
- Add a tab for NixOS Unstable channel options
- Display the version of each source (e.g. NixOS 24.11 for the current stable branch)

## [0.3.0] - 2025-01-02
- Add option to scroll through list and highlight individual items to show more information.
- Add option to open link to the source of highlighted item, and the docs webpage.
- Add Vim style bindings.
- Add dynamically updated, smaller, and much faster caching system. Nox now no longer has a compiled-in cache, but instead manages and auto-updates its cache in its data directory. The location of the cache is determined by `$NOX_DATA` if it is defined; else to a project-specific folder in the standard data directory for your system (`$XDG_DATA_HOME` on Linux, `~/Library/Application Support` on Darwin).
- Add a nix flake for this package.

## [0.2.1] - 2024-04-18
- Update dependencies and cached html files

## [0.2.0] - 2024-03-29
- Changed binary name to `nox`.
- Updated documentation.
- Fixed vanishing text bug.
- Fixed bug causing some text to appear entirely in lowercase.

## [0.1.0] - 2024-03-08
Initial release.

### Added
- Backend code for retrieving, parsing and searching options for Nix-Darwin, NixOS and Home Manager.
- Frontend TUI
- README.md
