# If you see this output, you probably forgot to pipe it into `source`:
# nix-your-shell nu | save nix-your-shell.nu

def call-wrapper (command: string, args: list<string>) {
  if not (which nix-your-shell | is-empty) {
    let args = ["--"] ++ $args

    run-external nix-your-shell $command $args
  } else {
    run-external $command $args
  }
}

extern-wrapped nix-shell (...args) {
  call-wrapper nix-shell $args
}

extern-wrapped nix (...args) {
  call-wrapper nix $args
}
