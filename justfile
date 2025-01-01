alias ld := log-debug
log-debug:
    NOX_DATA="./data" RUST_LOG="nox=DEBUG" cargo run

clean:
    rm -rf data

alias cd := clean-debug
clean-debug: clean log-debug

default:
	just --list

run *args: check-git
	nix run {{args}}

build *args: check-git
	nix --extra-experimental-features 'nix-command flakes' build {{args}}

alias l := lint
lint:
	just run nixpkgs#deadnix
	just run nixpkgs#statix -- check

alias f := fix
fix:
	just run nixpkgs#deadnix -- -e
	just run nixpkgs#statix -- fix
	fd .nix$ | parallel 'just run nixpkgs#nixfmt-rfc-style -- {}'

# https://github.com/DeterminateSystems/flake-checker
# Health check for flake.lock
nfc:
	just run github:DeterminateSystems/flake-checker

# Lists all files that are neither tracked nor ignored. These will not be seen by nix, which might cause silent and confusing errors.
check-git:
	@if [[ -n $(git ls-files . --exclude-standard --others) ]]; then echo "The following files are not tracked and not ignored:"; git ls-files . --exclude-standard --others; exit 1; fi

alias c := check
check: check-git lint
	nix flake check

alias ca := check-all
check-all *args: check-git lint
	nix flake check --all-systems {{args}}
