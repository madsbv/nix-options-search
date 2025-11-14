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
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        gitignoreSrc = pkgs.callPackage inputs.gitignore { };
      in
      {
        packages = rec {
          nox = pkgs.callPackage ./default.nix { inherit gitignoreSrc pkgs; };
          default = nox;
        };

        apps = rec {
          nox = flake-utils.lib.mkApp { drv = import ./default.nix { inherit gitignoreSrc pkgs; }; };
          default = nox;
        };

        devShells.default = pkgs.mkShell {
          CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

          buildInputs = with pkgs; [
            cargo
            rustc
            git
          ];

          packages = with pkgs; [
            deadnix
            statix
            nixfmt-tree
            just
          ];
        };
      }
    );
}
