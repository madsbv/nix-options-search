{
  description = "Flake based installation of nix-options-search";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      crane,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        inherit (pkgs) lib;

        # Latest nightly that has every requested component
        nightly = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
          }
        );

        craneStable = crane.mkLib nixpkgs.legacyPackages.${system};
        craneNightly = (crane.mkLib pkgs).overrideToolchain (_: nightly);

        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            (craneNightly.fileset.commonCargoSources ./.)
            ./testing/data # Read by the tests through CARGO_MANIFEST_DIR
          ];
        };

        nox = craneStable.buildPackage {
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

        # Dev profile matches cargo's default for test/clippy, and avoids release LTO in tests
        checkArgs = {
          inherit src;
          strictDeps = true;
          CARGO_PROFILE = "dev";
        };
        nightlyArtifacts = craneNightly.buildDepsOnly checkArgs;
        stableArtifacts = craneStable.buildDepsOnly checkArgs;

        nixSrc = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.fileFilter (file: file.hasExt "nix") ./.;
        };

        checks = {
          nox-fmt = craneNightly.cargoFmt { inherit src; };
          nox-clippy = craneNightly.cargoClippy (
            checkArgs
            // {
              cargoArtifacts = nightlyArtifacts;
              cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
            }
          );
          nox-test = craneNightly.cargoTest (
            checkArgs
            // {
              cargoArtifacts = nightlyArtifacts;
              cargoTestExtraArgs = "--all-targets";
            }
          );
          nox-doc = craneNightly.cargoDoc (
            checkArgs
            // {
              cargoArtifacts = nightlyArtifacts;
              # No --examples: as a target filter it selects only examples, and there are none
              cargoDocExtraArgs = "--no-deps --document-private-items --all-features --workspace";
              env.RUSTDOCFLAGS = "-D warnings";
            }
          );
          # Same tests on the nixpkgs toolchain, so at least one run uses stable
          nox-test-stable = craneStable.cargoTest (
            checkArgs
            // {
              cargoArtifacts = stableArtifacts;
              cargoTestExtraArgs = "--all-targets";
            }
          );
          nox-lint =
            pkgs.runCommand "nox-lint"
              {
                nativeBuildInputs = [
                  pkgs.deadnix
                  pkgs.statix
                ];
              }
              ''
                cd ${nixSrc}
                deadnix --fail .
                statix check .
                touch $out
              '';
        };

        app = flake-utils.lib.mkApp { drv = nox; };
      in
      {
        inherit checks;

        packages = {
          inherit nox;
          default = nox;
        };

        apps = {
          nox = app;
          default = app;
        };

        devShells.default = craneNightly.devShell {
          # Don't pass `checks` here: it would pull the nixpkgs toolchain into the shell
          packages = [
            nightly
            pkgs.deadnix
            pkgs.statix
            pkgs.nixfmt-tree
            pkgs.just
          ];
        };
      }
    );
}
