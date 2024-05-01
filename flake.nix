{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      flake = false;
    };
  };

  outputs =
    inputs@{
      nixpkgs,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          gitignoreSrc = pkgs.callPackage inputs.gitignore { };
        in
        rec {
          packages.nox = pkgs.callPackage ./default.nix { inherit gitignoreSrc; };

          legacyPackages = packages;

          defaultPackage = packages.nox;

          devShell = pkgs.mkShell {
            CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

            buildInputs = with pkgs; [
              cargo
              rustc
              git
            ];
          };
        }
      );
}
