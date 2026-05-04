/// Transform arguments to a `devenv` invocation to run the specified `command` with the specified
/// `command_args` when entering an interactive `devenv shell`.
///
/// Only modifies `devenv shell` invocations that do not already specify a command to run.
pub fn transform_devenv(args: Vec<String>, command: &str, command_args: &[String]) -> Vec<String> {
    let Some(shell_index) = shell_subcommand_index(&args) else {
        return args;
    };

    if devenv_shell_has_command(&args[(shell_index + 1)..]) {
        return args;
    }

    let mut ret = Vec::with_capacity(args.len() + 1 + command_args.len());
    let ends_with_separator = args.last().map(|arg| arg == "--").unwrap_or(false);
    ret.extend(args);
    if !ends_with_separator {
        ret.push("--".into());
    }
    ret.push(command.into());
    ret.extend(command_args.iter().cloned());
    ret
}

fn shell_subcommand_index(args: &[String]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" | "--version" | "-V" | "version" | "help" => return None,
            "shell" => return Some(i),
            _ => match advance_devenv_option(args, i) {
                Some(next) => i = next,
                None => return None,
            },
        }
    }

    None
}

fn devenv_shell_has_command(args: &[String]) -> bool {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--" => return i + 1 < args.len(),
            "--help" | "-h" | "--version" | "-V" => return true,
            _ => match advance_devenv_option(args, i) {
                Some(next) => i = next,
                None if args[i].starts_with('-') => i += 1,
                None => return true,
            },
        }
    }

    false
}

fn advance_devenv_option(args: &[String], i: usize) -> Option<usize> {
    match args[i].as_str() {
        // Options with two arguments.
        "--override-input" | "-o" | "--option" | "-O" | "--nix-option" => Some(i + 3),

        // Options with one argument.
        "--from"
        | "--max-jobs"
        | "-j"
        | "--cores"
        | "-u"
        | "--system"
        | "-s"
        | "--profile"
        | "-P"
        | "--secretspec-provider"
        | "--secretspec-profile"
        | "--trace-output"
        | "--trace-format"
        | "--tui" => Some(i + 2),

        // Options with zero arguments.
        "--impure"
        | "-i"
        | "--no-impure"
        | "--offline"
        | "--nix-debugger"
        | "--reload"
        | "--no-reload"
        | "--eval-cache"
        | "--no-eval-cache"
        | "--refresh-eval-cache"
        | "--refresh-task-cache"
        | "--verbose"
        | "-v"
        | "--quiet"
        | "-q"
        | "--no-tui" => Some(i + 1),

        // `--clean` accepts zero or more values. Treat following non-option arguments as values.
        "--clean" | "-c" => {
            let mut next = i + 1;
            while next < args.len() && !args[next].starts_with('-') {
                next += 1;
            }
            Some(next)
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::transform_devenv;

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn adds_command_to_devenv_shell() {
        assert_eq!(
            transform_devenv(args(&["shell"]), "fish", &[]),
            args(&["shell", "--", "fish"])
        );
    }

    #[test]
    fn adds_command_after_options() {
        assert_eq!(
            transform_devenv(
                args(&["--from", "path:.", "shell", "--profile", "backend"]),
                "zsh",
                &["-l".into()]
            ),
            args(&[
                "--from",
                "path:.",
                "shell",
                "--profile",
                "backend",
                "--",
                "zsh",
                "-l"
            ])
        );
    }

    #[test]
    fn adds_command_after_top_level_shell_options() {
        assert_eq!(
            transform_devenv(
                args(&["--profile", "backend", "--reload", "shell"]),
                "fish",
                &[]
            ),
            args(&["--profile", "backend", "--reload", "shell", "--", "fish"])
        );
    }

    #[test]
    fn handles_option_arities() {
        assert_eq!(
            transform_devenv(
                args(&[
                    "--override-input",
                    "nixpkgs",
                    "github:NixOS/nixpkgs",
                    "--from",
                    "path:.",
                    "--reload",
                    "shell",
                ]),
                "fish",
                &[]
            ),
            args(&[
                "--override-input",
                "nixpkgs",
                "github:NixOS/nixpkgs",
                "--from",
                "path:.",
                "--reload",
                "shell",
                "--",
                "fish",
            ])
        );
    }

    #[test]
    fn does_not_replace_existing_command() {
        assert_eq!(
            transform_devenv(args(&["shell", "echo", "hi"]), "fish", &[]),
            args(&["shell", "echo", "hi"])
        );
        assert_eq!(
            transform_devenv(args(&["shell", "--", "echo", "hi"]), "fish", &[]),
            args(&["shell", "--", "echo", "hi"])
        );
    }

    #[test]
    fn leaves_other_devenv_commands_unchanged() {
        assert_eq!(transform_devenv(args(&["up"]), "fish", &[]), args(&["up"]));
    }

    #[test]
    fn leaves_help_unchanged() {
        assert_eq!(
            transform_devenv(args(&["shell", "--help"]), "fish", &[]),
            args(&["shell", "--help"])
        );
    }

    #[test]
    fn reuses_existing_separator() {
        assert_eq!(
            transform_devenv(args(&["shell", "--"]), "fish", &[]),
            args(&["shell", "--", "fish"])
        );
    }
}
