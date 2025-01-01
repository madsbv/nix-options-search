{
  description = "Flake based installation of nix-options-search";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      flake = false;
    };
  };

  outputs =
    inputs@{ nixpkgs, flake-utils, ... }:
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
          packages.nox = pkgs.callPackage ./default.nix { inherit gitignoreSrc pkgs; };

          # legacyPackages = packages;

          packages.default = import ./default.nix { inherit gitignoreSrc pkgs; };

          devShells.${system}.default = pkgs.mkShell {
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
