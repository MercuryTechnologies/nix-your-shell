# If you see this output, you probably forgot to pipe it into `source`:
# nix-your-shell | source

function nix-shell () {
    nix-your-shell nix-shell "$@"
}

function nix () {
    nix-your-shell nix "$@"
}
