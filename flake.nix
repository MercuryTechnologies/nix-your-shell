{
  description = "A `nix` and `nix-shell` wrapper for shells other than `bash`";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    systems.url = "github:nix-systems/default";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  nixConfig = {
    extra-substituters = ["https://cache.garnix.io"];
    extra-trusted-substituters = ["https://cache.garnix.io"];
    extra-trusted-public-keys = ["cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="];
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    systems,
    ...
  }: let
    inherit (nixpkgs) lib;
    makePkgs = system:
      import nixpkgs {
        localSystem = system;
      };
    eachSystem = fn: lib.genAttrs (import systems) fn;
  in {
    pkgs = eachSystem (system: makePkgs system);

    localPkgs = eachSystem (system: self.pkgs.${system}.callPackage ./nix/makePackages.nix {inherit inputs;});

    packages = eachSystem (system: let
      localPkgs = self.localPkgs.${system};
    in
      (lib.filterAttrs (_name: lib.isDerivation) localPkgs)
      // {
        default = localPkgs.nix-your-shell;

        nix-your-shell-from-overlay = let
          overlayed = self.pkgs.${system}.appendOverlays [
            self.overlays.default
          ];
        in
          overlayed.nix-your-shell;
      });

    checks = eachSystem (
      system:
        builtins.removeAttrs
        self.localPkgs.${system}.checks
        # Ugh.
        ["override" "overrideDerivation"]
    );

    devShells = eachSystem (system: {
      default = self.localPkgs.${system}.nix-your-shell.devShell;
    });

    overlays.default = final: prev: let
      localPkgs = prev.callPackage ./nix/makePackages.nix {inherit inputs;};
    in {
      inherit (localPkgs) nix-your-shell;
    };
  };
}
