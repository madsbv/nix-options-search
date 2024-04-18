update-cache:
    curl "https://daiderd.com/nix-darwin/manual/index.html" > "data/nix-darwin-index.html"
    curl "https://nixos.org/manual/nixos/stable/options" > "data/nixos-index.html"
    curl "https://nix-community.github.io/home-manager/options.xhtml" > "data/home-manager-index.html"
    curl "https://nix-community.github.io/home-manager/nixos-options.xhtml" > "data/home-manager-nixos-index.html"
    curl "https://nix-community.github.io/home-manager/nix-darwin-options.xhtml" > "data/home-manager-nix-darwin-index.html"
