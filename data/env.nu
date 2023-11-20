# If you see this output, you probably forgot to pipe it into `source`:
# nix-your-shell nu | save nix-your-shell.nu

def _nix_your_shell (command: string, args: list<string>) {
  if not (which nix-your-shell | is-empty) {
    let args = ["--"] ++ $args
    run-external nix-your-shell $command $args
  } else {
    run-external $command $args
  }
}

def --wrapped nix-shell (...args) {
  _nix_your_shell nix-shell $args
}

def --wrapped nix (...args) {
  _nix_your_shell nix $args
}
