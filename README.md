# nix-your-shell

![Crates.io](https://img.shields.io/crates/v/nix-your-shell)
[![nixpkgs](https://repology.org/badge/version-for-repo/nix_unstable/nix-your-shell.svg?header=nixpkgs)](https://repology.org/project/nix-your-shell/versions)

A `nix` and `nix-shell` wrapper for shells other than `bash`.

`nix develop` and `nix-shell` use `bash` as the default shell, so
`nix-your-shell` prints shell snippets you can source to use the shell
you prefer inside of Nix shells.

## Usage

`nix-your-shell` will print out shell environment code you can source to
activate `nix-your-shell`.

The shell code will transform `nix` and `nix-shell` invocations that call
`nix-your-shell YOUR_SHELL nix ...` and `nix-your-shell YOUR_SHELL nix-shell ...` instead.
`nix-your-shell` will add a `--command YOUR_SHELL` argument to these commands
(unless you've already added one) so that it drops you into _your_ shell,
rather than `bash`.


### Fish

Add to your `~/.config/fish/config.fish`:

```fish
if command -q nix-your-shell
  nix-your-shell fish | source
end
```

### Zsh

Add to your `~/.zshrc`:

```
if command -v nix-your-shell > /dev/null; then
  nix-your-shell zsh | source /dev/stdin
fi
```

## Installation

### nix-env

To install the latest version with `nix-env`, use:

```sh
nix-env --install --file https://github.com/MercuryTechnologies/nix-your-shell/archive/refs/heads/main.tar.gz
```

You can later remove the installed program with `nix-env --uninstall nix-your-shell`.

### nix run

Run dynamically with `nix run`:

```sh
nix run github:MercuryTechnologies/nix-your-shell
```

### Flakes

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

## Comparison with `any-nix-shell`

[`any-nix-shell`](https://github.com/haslersn/any-nix-shell) does roughly the
same thing, and serves as the inspiration for `nix-your-shell`.

There are a few reasons I wrote `nix-your-shell` as a competitor:

- `any-nix-shell` doesn't support Nix flakes through `nix develop`. `nix-your-shell` does.
- `any-nix-shell` is a hodgepodge of shell scripts with multiple layers of
  `eval` and templating, making hacking or modifying it challenging. In
  contrast, `nix-your-shell` is written in Rust with a relatively
  straightforward structure (the shell environment code generation command and
  the `nix` wrappers are the same, so there's no need for dotfile executables
  on your `$PATH`).

However, `any-nix-shell` can optionally display the packages in the current
shell on a righthand prompt. `nix-your-shell` does not support this.
