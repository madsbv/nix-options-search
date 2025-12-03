# Nox

[![Crates.io](https://img.shields.io/crates/v/nix-options-search.svg)](https://crates.io/crates/nix-options-search)
[![CI](https://github.com/madsbv/nix-options-search/workflows/CI/badge.svg)](https://github.com/madsbv/nix-options-search/actions)

A fast and convenient command-line tool to look up options for configuring nix-darwin, nixOS, and home-manager quickly, with fuzzy finding.

![Made with VHS](https://vhs.charm.sh/vhs-5zsL56XNOM7Map2ixPdu4w.gif)

## Motivation

Setting up any part of a system using nixos, nix-darwin or home-manager involves a significant amount of time spent looking through the docs to figure out the names of all the relevant options and what kind of input they expect. However, the only convenient first-party methods for accessing the docs are through man pages or corresponding static webpages (like the [nixos manual](https://nixos.org/manual/nixos/stable/options); ~20MB of html!). This means you have to either scroll through a huge alphabetized list, or be able to search for an exact substring of the option name you want.

Nox is built with fuzzy searching so you don't have to know exactly what you're looking for to find it. It is also fast: After the first run, it uses an internal cache to provide basically instantaneous startup, and the search results likewise update instantly on every keystroke.


## Quick start

### Install using the Rust toolchain/Cargo

``` sh
cargo install nix-options-search
```

If you don't already have the Rust toolchain installed, get it from your favorite package manager, or following the [official rust-lang guide](https://www.rust-lang.org/tools/install).

### Run 

``` sh
nox
```

## Other installation methods

### Nix flakes

Run nox using nix with flakes enabled with `nix run github:madsbv/nix-options-search`.

To add nox to a nixOS, nix-darwin or home-manager configuration using flakes, add this repository as a flake input and add `inputs.nox.packages.${system}.default` to your package list. For example, for a nixOS system with hostname `${hostname}` and system type `${system}` (one of `x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin`):
``` nix
inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nox = {
        url = "github:madsbv/nix-options-search";
        inputs.nixpkgs.follows = "nixpkgs";
    };
};
outputs = inputs: {
    nixosConfigurations.${hostname} = nixpkgs.lib.nixosSystem {
        system = ${system}
        modules = [{ 
                environment.systemPackages = [ inputs.nox.packages.${system}.default ];
        }];
    };
};
```

### Prebuilt binaries

See [Releases](https://github.com/madsbv/nix-options-search/releases).

## Usage

Nox works mainly through fuzzy searching on option names. Navigate to the tab you want (nix-darwin, nixos, home-manager etc.) with arrows or `<ctrl+h>` or `<ctrl+l>`, then start typing!

You can scroll through the results list with `<up>`/`<down>`/`<ctrl+k>`/`<ctrl+j>`. With an item highlighted, `<ctrl+o>` opens the file that defines that option in the source repository, while `<enter>` opens the online documentation page at the corresponding entry.

The first startup might take a while; the nixos documentation alone is ~20MB of data that has to be retrieved. After that however, the data is by default cached locally and only refreshed occasionally.

## Configuration

Nox supports some configuration through CLI flags, environment variables and a configuration file. To see the CLI flags, run `nox help`.

A configuration file in TOML format can be specified with the `--config` flag. Otherwise, Nox looks for a `nox.toml` file in the following locations in order:
1. The path specified by the environment variable `NOX_CONFIG` if set,
2. The OS standard config directory (usually `$HOME/.config/nox` on Linux, `$HOME/Library/Application Support/dev.mvil.nox` on Mac and `$HOME\AppData\Roaming\mvil\nox` on Windows),
3. The current directory.

Run `nox print-config default` to print the default configuration, with documentation of all options, to `stdout`, or `nox print-config --write default` to write this default configuration to the default location of `nox.toml` as defined above.

The configuration supports enabling/disabling caching and logging, as well as customizing the policies for those; as well as customizing which tabs are shown in nox in what order, and adding custom tabs

## Contributing

### Clone the repo

```sh
git clone https://github.com/madsbv/nix-options-search
cd nix-options-search
```

### Build the project

```sh
cargo build
```

### Run the project

```sh
cargo run
```

### Run the tests

```sh
cargo test
```

Some tests are gated behind the `online-test` feature flag since they require a functioning network connection to pass, which is not necessarily available in the nix sandbox.

Run all tests with

``` sh
cargo test --features online-tests
```

### Submit a pull request

If you'd like to contribute, please fork the repository and open a pull request to the `main` branch.


## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
