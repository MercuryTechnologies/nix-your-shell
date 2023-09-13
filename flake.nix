{
  description = "A `nix` and `nix-shell` wrapper for shells other than `bash`";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    ...
  }:
    let
      inherit (nixpkgs) lib;
      systems = ["aarch64-linux" "aarch64-darwin" "x86_64-darwin" "x86_64-linux"];
      eachSystem = fn: lib.genAttrs systems (system:
        let
          pkgs = import nixpkgs {
            localSystem = system;
            overlays = [self.overlays.default];
          };
        in
          fn pkgs system
      );
    in {
      packages = eachSystem (pkgs: system: {
        nix-your-shell = pkgs.nix-your-shell;
        default = self.packages.${system}.nix-your-shell;
      });

      checks = eachSystem (_: system: self.packages.${system});

        # for debugging
        # inherit pkgs;

        devShells = eachSystem (pkgs: system: {
          default = pkgs.nix-your-shell.overrideAttrs (
            old: {
              # Make rust-analyzer work
              RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

              # Any dev tools you use in excess of the rust ones
              nativeBuildInputs =
                old.nativeBuildInputs;
            }
          );
        });

      overlays.default = (
        final: prev: {
          nix-your-shell = final.rustPlatform.buildRustPackage {
            pname = "nix-your-shell";
            version = "1.3.0"; # LOAD-BEARING COMMENT. See: `.github/workflows/version.yaml`

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            src = ./.;

            # Tools on the builder machine needed to build; e.g. pkg-config
            nativeBuildInputs = [
              final.rustfmt
              final.clippy
            ];

            # Native libs
            buildInputs = [];

            postCheck = ''
              cargo fmt --check && echo "\`cargo fmt\` is OK"
              cargo clippy -- --deny warnings && echo "\`cargo clippy\` is OK"
            '';

            passthru.generate-config = shell: final.runCommand "nix-your-shell-config" { } ''
              ${final.nix-your-shell}/bin/nix-your-shell ${shell} >> $out
            '';

            meta = {
              homepage = "https://github.com/MercuryTechnologies/nix-your-shell";
              license = lib.licenses.mit;
              platforms = systems;
              mainProgram = "nix-your-shell";
            };
          };
        }
      );
    };
}
