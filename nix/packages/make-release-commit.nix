{
  writeShellApplication,
  cargo,
  cargo-release,
  git,
}:
writeShellApplication {
  name = "make-release-commit";

  runtimeInputs = [
    cargo
    cargo-release
    git
  ];

  text = ''
    if [[ -n "''${CI:-}" ]]; then
      git config --local user.email "github-actions[bot]@users.noreply.github.com"
      git config --local user.name "github-actions[bot]"
    fi

    cargo release --version

    cargo release \
      --execute \
      --no-confirm \
      "$@"
  '';
}
