{
  description = "Simple test shell (pure, uses injected Nix store paths)";

  outputs =
    { self }:
    let
      # Import the generated config.nix (produced from config.nix.in during test setup).
      config = import ./config.nix;
      inherit (config) bash system mkDerivation;

      hello = mkDerivation {
        name = "hello";
        buildCommand = ''
          mkdir -p $out/bin
          cat > $out/bin/hello << 'EOF'
          #!${bash}
          echo "Hello, world!"
          EOF
          chmod +x $out/bin/hello
        '';
      };

    in
    {
      packages.${system} = {
        inherit hello;
      };

      devShells.${system} = {
        default = mkDerivation {
          name = "simple-shell";
          nativeBuildInputs = [ hello ];
          buildCommand = ''
            touch $out
          '';
        };
      };
    };
}
