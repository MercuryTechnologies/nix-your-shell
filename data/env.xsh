# If you see this output, you probably forgot to pipe it into `source`:
# nix-your-shell | source

aliases['nix-shell'] = 'nix-your-shell nix-shell -- @($args)'
aliases['nix'] = 'nix-your-shell nix -- @($args)'
