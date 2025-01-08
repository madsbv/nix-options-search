# nix-options-search

[![Crates.io](https://img.shields.io/crates/v/nix-options-search.svg)](https://crates.io/crates/nix-options-search)
[![CI](https://github.com/madsbv/nix-options-search/workflows/CI/badge.svg)](https://github.com/madsbv/nix-options-search/actions)

A simple command-line tool to look up options for configuring nix-darwin, nixOS, and home-manager quickly, with fuzzy finding.
<img width="1752" alt="TUI" src="https://github.com/madsbv/nix-options-search/assets/2766060/615ea8ed-8f70-41d3-abb9-9d8132c5757d">

## Installation

### Cargo

* Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
* run `cargo install nix-options-search`

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

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).
