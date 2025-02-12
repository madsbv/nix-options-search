{
  pkgs ? import <nixpkgs> { },
  lib ? pkgs.lib,
  # A set providing `buildRustPackage :: attrsets -> derivation`
  rustPlatform ? pkgs.rustPlatform,
  fetchFromGitHub ? pkgs.fetchFromGitHub,
  gitignoreSrc ? null,
}:

let
  gitignoreSource =
    if gitignoreSrc != null then
      gitignoreSrc.gitignoreSource
    else
      (import (fetchFromGitHub {
        owner = "hercules-ci";
        repo = "gitignore";
        rev = "c4662e662462e7bf3c2a968483478a665d00e717";
        sha256 = "0jx2x49p438ap6psy8513mc1nnpinmhm8ps0a4ngfms9jmvwrlbi";
      }) { inherit lib; }).gitignoreSource;
in
rustPlatform.buildRustPackage {
  pname = "nix-options-search";
  version = "0.3.3";

  src = gitignoreSource ./.;

  buildInputs = [ ];
  nativeBuildInputs = [ ];
  cargoLock.lockFile = ./Cargo.lock;

  meta = with lib; {
    homepage = "https://github.com/madsbv/nix-options-search";
    description = "A simple command-line tool to look up options for configuring nix-darwin, nixOS, and home-manager quickly, with fuzzy finding.";
    license = [
      licenses.mit
      licenses.asl20
    ];
    maintainers = [ maintainers.madsbv ];
    mainProgram = "nox";
  };
}
