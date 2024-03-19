{
  lib,
  stdenv,
  libiconv,
  darwin,
  crane-lib,
  inputs,
  rustPlatform,
  rust-analyzer,
}: let
  src = lib.cleanSourceWith {
    src = crane-lib.path ../../.;
    # Keep template/test data.
    filter = path: type:
      lib.hasInfix "/data" path
      || (crane-lib.filterCargoSources path type);
  };
  cargoToml = lib.importTOML ../../Cargo.toml;

  commonArgs' = {
    inherit src;
    inherit (cargoToml.package) version;
    pname = cargoToml.package.name;

    nativeBuildInputs = lib.optionals stdenv.isDarwin [
      libiconv
      darwin.apple_sdk.frameworks.CoreServices
    ];
  };

  # Build *just* the cargo dependencies, so we can reuse
  # all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = crane-lib.buildDepsOnly commonArgs';

  commonArgs =
    commonArgs'
    // {
      inherit cargoArtifacts;
    };

  checks = {
    tests = crane-lib.cargoNextest (commonArgs
      // {
        NEXTEST_PROFILE = "ci";
        NEXTEST_HIDE_PROGRESS_BAR = "true";
      });
    clippy = crane-lib.cargoClippy (commonArgs
      // {
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });
    doc = crane-lib.cargoDoc (commonArgs
      // {
        cargoDocExtraArgs = "--document-private-items";
        RUSTDOCFLAGS = "-D warnings";
      });
    fmt = crane-lib.cargoFmt commonArgs;
    audit = crane-lib.cargoAudit (commonArgs
      // {
        inherit (inputs) advisory-db;
      });
  };

  devShell = crane-lib.devShell {
    inherit checks;

    # Make rust-analyzer work
    RUST_SRC_PATH = rustPlatform.rustLibSrc;

    # Extra development tools (cargo and rustc are included by default).
    packages = [
      rust-analyzer
    ];
  };
in
  # Build the actual crate itself, reusing the dependency
  # artifacts from above.
  crane-lib.buildPackage (commonArgs
    // {
      # Don't run tests; we'll do that in a separate derivation.
      # This will allow people to install and depend on `ghciwatch`
      # without downloading a half dozen different versions of GHC.
      doCheck = false;

      # Only build `ghciwatch`, not the test macros.
      cargoBuildCommand = "cargoWithProfile build";

      passthru = {
        inherit checks devShell;
      };

      meta = {
        inherit (cargoToml.package) description homepage;
        license = lib.licenses.mit;
        maintainers = [lib.maintainers._9999years];
        platforms = import inputs.systems;
        mainProgram = cargoToml.package.name;
      };
    })
