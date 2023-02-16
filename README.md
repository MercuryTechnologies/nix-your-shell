# nix-your-shell

[![nixpkgs](https://repology.org/badge/version-for-repo/nix_unstable/nix-your-shell.svg?header=nixpkgs)](https://repology.org/project/nix-your-shell/versions)
[![Crates.io](https://img.shields.io/crates/v/nix-your-shell)](https://crates.io/crates/nix-your-shell)

A `nix` and `nix-shell` wrapper for shells other than `bash`.

`nix develop` and `nix-shell` use `bash` as the default shell, so
`nix-your-shell` prints shell snippets you can source to use the shell
you prefer inside of Nix shells.

## Usage

`nix-your-shell` will print out shell environment code you can source to
activate `nix-your-shell`.

Then, `nix-shell`, `nix develop`, and `nix shell` will use your shell instead
of bash, unless overridden with a `--command` argument.

### Fish

Add to your `~/.config/fish/config.fish`:

```fish
if command -q nix-your-shell
  nix-your-shell fish | source
end
```

### Zsh

Add to your `~/.zshrc`:

```zsh
if command -v nix-your-shell > /dev/null; then
  nix-your-shell zsh | source /dev/stdin
fi
```

## Installation

You can either install `nix-your-shell` from this repository or from `nixpkgs`.
The version packaged in `nixpkgs` will probably lag behind this repository by
about a week or so.

### nix profile

To install the latest version with `nix profile`, use one of:

```sh
nix profile install github:MercuryTechnologies/nix-your-shell
nix profile install "nixpkgs#nix-your-shell"
```

### nix-env

To install the latest version with `nix-env`, use one of:

```sh
nix-env --install --file https://github.com/MercuryTechnologies/nix-your-shell/archive/refs/heads/main.tar.gz
nix-env --install nix-your-shell
```

You can later remove the installed program with `nix-env --uninstall nix-your-shell`.

### nix run

Run dynamically with `nix run`:

```sh
nix run github:MercuryTechnologies/nix-your-shell -- zsh
nix run "nixpkgs#nix-your-shell" -- zsh
```

Note that because the generated shell code will refer to the dynamically-built
`nix-your-shell` executable, it may get [garbage
collected][nix-collect-garbage] and cause problems later.

[nix-collect-garbage]: https://nixos.org/manual/nix/stable/package-management/garbage-collection.html

### Flakes

`nix-your-shell` is packaged in `nixpkgs`, so you can add `pkgs.nix-your-shell`
to `environment.systemPackages` if you don't need the bleeding edge releases
from this repo.

Add to a NixOS flake configuration using the overlay:

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
