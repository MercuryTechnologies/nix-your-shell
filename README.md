# nix-your-shell

Alternate shell support for `nix develop` and `nix-shell`.

`nix develop` and `nix-shell` use `bash` as the default shell, so
`nix-your-shell` prints shell snippets you can source to use the shell
you prefer inside of Nix shells.

## Usage

Add this to your shell configuration, replacing `fish` with your shell:

```fish
nix-your-shell fish | source
```

`nix-your-shell` will print out shell environment code you can source to
activate `nix-your-shell`.

The shell code will transform `nix` and `nix-shell` invocations that call
`nix-your-shell YOUR_SHELL nix ...` and `nix-your-shell YOUR_SHELL nix-shell ...` instead.
`nix-your-shell` will add a `--command YOUR_SHELL` argument to these commands
(unless you've already added one) so that it drops you into _your_ shell,
rather than `bash`.

You may want to only use `nix-your-shell` if it's installed, like this (for `fish`):

```fish
if command -q nix-your-shell
    nix-your-shell fish | source
end
```

## Installation

To install the latest version with `nix-env`, use:

```sh
nix-env --install --file https://github.com/MercuryTechnologies/nix-your-shell/archive/refs/heads/main.tar.gz
```

Run dynamically with `nix run`:

```sh
nix run github:MercuryTechnologies/nix-your-shell
```

Add to a NixOS flake configuration:

`flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    nix-your-shell = {
      url = "github:MercuryTechnologies/nix-your-shell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ attrs: {
    nixosConfigurations = {
      YOUR_HOSTNAME = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        specialArgs = attrs;
        modules = [./YOUR_HOSTNAME.nix];
      };
    };
  };
}
```

`./YOUR_HOSTNAME.nix`:

```nix
{
  config,
  pkgs,
  nix-your-shell,
  ...
}: {
  nixpkgs.overlays = [
    nix-your-shell.overlay
  ];

  environment.systemPackages = [
    pkgs.nix-your-shell
  ];

  # Example configuration for `fish`:
  programs.fish = {
    enable = true;
    promptInit = ''
      nix-your-shell fish | source
    '';
  };

  # ... extra configuration
}
```
