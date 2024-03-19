{
  writeShellApplication,
  cargo,
  cargo-release,
  gitAndTools,
}:
writeShellApplication {
  name = "make-release-commit";

  runtimeInputs = [
    cargo
    cargo-release
    gitAndTools.git
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
