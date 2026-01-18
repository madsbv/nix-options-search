#!/usr/bin/env python3
import argparse
import subprocess
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

BUILTIN_SOURCES = [
    {
        "name": "Nix-Darwin",
        "url": "https://nix-darwin.github.io/nix-darwin/manual/index.html",
        "version_url": None,
    },
    {
        "name": "NixOS",
        "url": "https://nixos.org/manual/nixos/stable/options",
        "version_url": "https://nixos.org/manual/nixos/stable/",
    },
    {
        "name": "NixOS Unstable",
        "url": "https://nixos.org/manual/nixos/unstable/options",
        "version_url": "https://nixos.org/manual/nixos/unstable/",
    },
    {
        "name": "Home Manager",
        "url": "https://nix-community.github.io/home-manager/options.xhtml",
        "version_url": "https://nix-community.github.io/home-manager/",
    },
    {
        "name": "Home Manager NixOS",
        "url": "https://nix-community.github.io/home-manager/nixos-options.xhtml",
        "version_url": "https://nix-community.github.io/home-manager/",
    },
    {
        "name": "Home Manager Nix-Darwin",
        "url": "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml",
        "version_url": "https://nix-community.github.io/home-manager/",
    },
    {
        "name": "Nix Built-ins",
        "url": "https://nix.dev/manual/nix/2.28/language/builtins.html",
        "version_url": None,
    },
]


def fetch_url(url):
    """Internal helper to fetch content using standard urllib."""
    headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"}
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=15) as response:
        # Decode bytes to string; assumes utf-8
        return response.read().decode("utf-8")


def process_source(s, base_dir):
    """Worker function to process a single source into a specific base directory."""
    # Create the source-specific subdirectory inside the base directory
    dir_path = base_dir / s["name"]
    dir_path.mkdir(parents=True, exist_ok=True)

    print(f"Processing: {s['name']}")

    # 1. Save URL and Version URL metadata files
    (dir_path / "url").write_text(str(s["url"]), encoding="utf-8")
    (dir_path / "version_url").write_text(str(s["version_url"]), encoding="utf-8")

    # 2. Download data_html
    try:
        content = fetch_url(s["url"])
        (dir_path / "data_html").write_text(content, encoding="utf-8")
    except Exception as e:
        print(f"  [Error] {s['name']} (data): {e}")

    # 3. Download version_html (if applicable)
    if s["version_url"]:
        try:
            content = fetch_url(s["version_url"])
            (dir_path / "version_html").write_text(content, encoding="utf-8")
        except Exception as e:
            print(f"  [Error] {s['name']} (version): {e}")

    print(f"Completed: {s['name']}")


def get_git_root():
    """Returns the root directory of the current git repository."""
    try:
        root = (
            subprocess.check_output(
                ["git", "rev-parse", "--show-toplevel"], stderr=subprocess.STDOUT
            )
            .decode("utf-8")
            .strip()
        )
        return Path(root)
    except subprocess.CalledProcessError:
        raise RuntimeError("The script must be run inside a git repository.")


def main():
    # Automatically determine the target directory: <git-root>/testing/data
    git_root = get_git_root()
    default_base_dir = git_root / "testing" / "data"
    # Setup CLI argument parsing
    parser = argparse.ArgumentParser(description="Download Nix documentation sources.")
    parser.add_argument(
        "output",
        nargs="?",
        default=default_base_dir,
        help="Optional base directory for downloads (defaults to $(git rev-parse --show-toplevel)/testing/data)",
    )
    args = parser.parse_args()

    # Convert output to a Path object and ensure it exists
    base_dir = Path(args.output)
    base_dir.mkdir(parents=True, exist_ok=True)

    # Setting max_workers to allow all downloads to trigger simultaneously.
    with ThreadPoolExecutor(max_workers=len(BUILTIN_SOURCES)) as executor:
        executor.map(lambda s: process_source(s, base_dir), BUILTIN_SOURCES)

    print("\nAll tasks completed.")


if __name__ == "__main__":
    main()
