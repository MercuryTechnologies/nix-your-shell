{
  lib,
  nix-your-shell,
  writeShellApplication,
}:
writeShellApplication {
  name = "get-crate-version";

  text = ''
    VERSION=${lib.escapeShellArg nix-your-shell.version}

    echo "Version in \`Cargo.toml\` is $VERSION" 1>&2

    echo "$VERSION"
  '';
}
