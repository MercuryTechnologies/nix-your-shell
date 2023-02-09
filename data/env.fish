# If you see this output, you probably forgot to pipe it into `source`:
# nix-your-shell | source

function nix-shell --description "Start an interactive shell based on a Nix expression"
    nix-your-shell nix-shell $argv
end

function nix --description "Reproducible and declarative configuration management"
    nix-your-shell nix $argv
end
