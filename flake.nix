{
  description = "Flake based installation of nix-options-search";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            (craneLib.fileset.commonCargoSources ./.)
            ./testing/data # Read by the tests through CARGO_MANIFEST_DIR
          ];
        };

        nox = craneLib.buildPackage {
          inherit src;
          strictDeps = true;
          doCheck = false; # Tests run as a separate check

          meta = {
            homepage = "https://github.com/madsbv/nix-options-search";
            description = "A simple command-line tool to look up options for configuring nix-darwin, nixOS, and home-manager quickly, with fuzzy finding.";
            license = [
              lib.licenses.mit
              lib.licenses.asl20
            ];
            maintainers = [ lib.maintainers.madsbv ];
            mainProgram = "nox";
          };
        };

        app = flake-utils.lib.mkApp { drv = nox; };
      in
      {
        packages = {
          inherit nox;
          default = nox;
        };

        apps = {
          nox = app;
          default = app;
        };

        devShells.default = pkgs.mkShell {
          CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

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
