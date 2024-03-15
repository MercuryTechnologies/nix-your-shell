#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process;

use calm_io::stdout as println;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use clap::Parser;
use color_eyre::eyre;
use color_eyre::eyre::eyre;
use color_eyre::eyre::Context;
use color_eyre::Help;

mod shell;
use shell::Shell;
use shell::ShellKind;

/// Environment variable that indicates that the Nix profile has already been sourced.
///
/// This is set when a Nix profile script is sourced:
/// - `$HOME/.nix-profile/etc/profile.d/nix.sh`
/// - `/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh`
///
/// We export this variable to prevent the profile script from being sourced twice, clobbering the
/// `$PATH`.
///
/// See: <https://github.com/MercuryTechnologies/nix-your-shell/issues/25>
const NIX_SOURCED_VAR: &str = "__ETC_PROFILE_NIX_SOURCED";

/// A `nix` and `nix-shell` wrapper for shells other than `bash`.
///
/// Use by adding `nix-your-shell | source` to your shell configuration.
#[derive(Debug, Clone, Parser)]
#[command(version, author, about)]
#[command(max_term_width = 100, disable_help_subcommand = true)]
pub struct Opts {
    /// Log filter directives, of the form `target[span{field=value}]=level`, where all components
    /// except the level are optional.
    ///
    /// Try `debug` or `trace`.
    #[arg(long, default_value = "info", env = "NIX_YOUR_SHELL_LOG")]
    log: String,

    /// Print absolute paths to `nix-your-shell` in shell environment code.
    ///
    /// Note that this will not transform the shell argument to an absolute path.
    ///
    /// Absolute paths are automatically printed if `nix-your-shell` isn't on the `$PATH`.
    #[arg(long)]
    absolute: bool,

    /// Use `nom` (`nix-output-monitor`) instead of `nix` for running commands.
    #[arg(long)]
    nom: bool,

    /// The shell to use for wrapped commands and the shell environment.
    /// This can be an executable name like `fish` or the path to an executable like
    /// `/opt/homebrew/bin/fish`.
    shell: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum Command {
    /// Print the shell environment code to use `nix-your-shell`.
    ///
    /// This generally prints functions for `nix` and `nix-shell` which will instead call
    /// `nix-your-shell nix ...` and `nix-your-shell nix-shell ...`.
    Env,
    /// Execute a `nix-shell` command, running the shell if no command is explicitly given.
    NixShell { args: Vec<String> },
    /// Execute a `nix` command, running the shell if no command is explicitly given.
    Nix { args: Vec<String> },
}

impl Default for Command {
    fn default() -> Self {
        Self::Env
    }
}

fn main() -> eyre::Result<()> {
    let opts = Opts::parse();
    color_eyre::install()?;
    install_tracing(&opts.log)?;

    let shell = Shell::from_path(&opts.shell)?;
    tracing::debug!(%shell, input=opts.shell, "Detected shell");

    match opts.command.unwrap_or_default() {
        Command::Env => {
            let mut shell_code = match shell.kind {
                ShellKind::Zsh | ShellKind::Bash => {
                    include_str!("../data/env.sh")
                }

                ShellKind::Fish => {
                    include_str!("../data/env.fish")
                }

                ShellKind::Nushell => {
                    include_str!("../data/env.nu")
                }

                ShellKind::Xonsh => {
                    include_str!("../data/env.xsh")
                }

                ShellKind::Other(shell) => {
                    return Err(eyre!(
                        "I don't know how to generate a shell environment for `{shell}`"
                    ))
                    .note("Supported shells are: `zsh`, `fish`, `nushell`, `xonsh`, and `bash`")
                }
            }
            .to_owned();

            // Do some string replacements to reflect the arguments passed to `nix-your-shell` in
            // the generated code.
            //
            // We could make this a bit "cleaner" with an actual templating language, but it's nice
            // that the snippets in `../data/` are valid code and not templates.

            let mut shell_args = shell_words::quote(shell.path.as_str());

            if opts.nom {
                shell_args += " --nom";
            }

            shell_code =
                shell_code.replace("nix-your-shell", &format!("nix-your-shell {}", shell_args));

            let current_exe =
                current_exe().wrap_err("Unable to determine absolute path of `nix-your-shell`")?;
            if opts.absolute || !executable_is_on_path(&current_exe)? {
                shell_code = shell_code.replace("nix-your-shell", current_exe.as_str());
            }

            let _ = println!("{shell_code}");
            Ok(())
        }

        Command::NixShell { args } => {
            let new_args = transform_nix_shell(args, shell.path.as_str());
            let prog = if opts.nom { "nom-shell" } else { "nix-shell" };
            tracing::debug!(
                command = shell_words::join(
                    std::iter::once(prog).chain(new_args.iter().map(|s| s.as_str()))
                ),
                "Launching nix-shell"
            );
            Err(process::Command::new(prog)
                .args(new_args)
                .env(NIX_SOURCED_VAR, "1")
                .exec()
                .into())
        }

        Command::Nix { args } => {
            let new_args = transform_nix(args, shell.path.as_str());
            let prog = if opts.nom { "nom" } else { "nix" };
            tracing::debug!(
                command = shell_words::join(
                    std::iter::once(prog).chain(new_args.iter().map(|s| s.as_str()))
                ),
                "Launching nix"
            );
            Err(process::Command::new(prog)
                .args(new_args)
                .env(NIX_SOURCED_VAR, "1")
                .exec()
                .into())
        }
    }
}

fn install_tracing(filter_directives: &str) -> eyre::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter_directives)?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .without_time()
        .with_filter(env_filter);

    let registry = tracing_subscriber::registry();

    registry.with(fmt_layer).init();

    Ok(())
}

/// Get the path to the current executable.
fn current_exe() -> eyre::Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(std::env::current_exe()?)
        .map_err(|path_buf| eyre!("Path is not UTF-8: {path_buf:?}"))
}

fn executable_is_on_path(executable: &Utf8Path) -> eyre::Result<bool> {
    let directory = executable
        .parent()
        .ok_or_else(|| eyre!("Executable has no parent directory: {executable:?}"))?;
    let path = std::env::var("PATH").wrap_err("Failed to get $PATH environment variable")?;
    Ok(path
        .split(':')
        .map(Utf8Path::new)
        .any(|component| component == directory))
}

/// Transform arguments to a `nix` invocation to run the specified `command`.
///
/// Only modifies `nix develop` and `nix shell` commands.
fn transform_nix(args: Vec<String>, command: &str) -> Vec<String> {
    let mut ret = Vec::with_capacity(args.len() + 2);

    let mut subcommand = None;

    let mut i = 0;
    while i < args.len() {
        ret.push(args[i].clone());

        match args[i].as_str() {
            "--help" | "--version"
                | "-c" | "--command"
                => {
                // We already have a command to run.
                return args;
            }

            // Two arguments
            "--option"
                | "--redirect"
                | "--override-flake"
                | "--arg"
                | "--argstr"
                | "--override-input"
                => {
                ret.push(args[i + 1].clone());
                ret.push(args[i + 2].clone());
                i += 2;
            }

            // One argument
            "--log-format"
            | "--access-tokens"
            | "--allowed-impure-host-deps"
            | "--allowed-uris"
            | "--allowed-users"
            | "--bash-prompt"
            | "--bash-prompt-prefix"
            | "--bash-prompt-suffix"
            | "--build-hook"
            | "--build-poll-interval"
            | "--build-users-group"
            | "--builders"
            | "--commit-lockfile-summary"
            | "--connect-timeout"
            | "--cores"
            | "--diff-hook"
            | "--download-attempts"
            | "--download-speed"
            | "--experimental-features"
            | "--extra-access-tokens"
            | "--extra-allowed-impure-host-deps"
            | "--extra-allowed-uris"
            | "--extra-allowed-users"
            | "--extra-experimental-features"
            | "--extra-extra-platforms"
            | "--extra-hashed-mirrors"
            | "--extra-nix-path"
            | "--extra-platforms"
            | "--extra-plugin-files"
            | "--extra-sandbox-paths"
            | "--extra-secret-key-files"
            | "--extra-substituters"
            | "--extra-system-features"
            | "--extra-trusted-public-keys"
            | "--extra-trusted-substituters"
            | "--extra-trusted-users"
            | "--flake-registry"
            | "--gc-reserved-space"
            | "--hashed-mirrors"
            | "--http-connections"
            | "--log-lines"
            | "--max-build-log-size"
            | "--max-free"
            | "--max-jobs"
            | "--max-silent-time"
            | "--min-free"
            | "--min-free-check-interval"
            | "--nar-buffer-size"
            | "--narinfo-cache-negative-ttl"
            | "--narinfo-cache-positive-ttl"
            | "--netrc-file"
            | "--nix-path"
            | "--plugin-files"
            | "--post-build-hook"
            | "--pre-build-hook"
            | "--repeat"
            | "--sandbox-paths"
            | "--secret-key-files"
            | "--stalled-download-timeout"
            | "--store"
            | "--substituters"
            | "--system"
            | "--system-features"
            | "--tarball-ttl"
            | "--timeout"
            | "--trusted-public-keys"
            | "--trusted-substituters"
            | "--trusted-users"
            | "--user-agent-suffix"
            // `nix develop` options
            | "-k" | "--keep"
            | "--phase"
            |"--profile"
            | "--unset"
            | "--eval-store"
            | "-I" | "--include"
            | "--inputs-from"
            | "--update-input"
            | "--expr"
            | "-f" | "--file"
            => {
                ret.push(args[i + 1].clone());
                i += 1;
            }

            // Zero arguments
            "--offline"
            | "--refresh"
            | "--debug"
            | "-L"
            | "--print-build-logs"
            | "--quiet"
            | "-v"
            | "--verbose"
            | "--accept-flake-config"
            | "--no-accept-flake-config"
            | "--allow-dirty"
            | "--no-allow-dirty"
            | "--allow-import-from-derivation"
            | "--no-allow-import-from-derivation"
            | "--allow-symlinked-store"
            | "--no-allow-symlinked-store"
            | "--allow-unsafe-native-code-during-evaluation"
            | "--no-allow-unsafe-native-code-during-evaluation"
            | "--auto-optimise-store"
            | "--no-auto-optimise-store"
            | "--builders-use-substitutes"
            | "--no-builders-use-substitutes"
            | "--compress-build-log"
            | "--no-compress-build-log"
            | "--darwin-log-sandbox-violations"
            | "--no-darwin-log-sandbox-violations"
            | "--enforce-determinism"
            | "--no-enforce-determinism"
            | "--eval-cache"
            | "--no-eval-cache"
            | "--fallback"
            | "--no-fallback"
            | "--fsync-metadata"
            | "--no-fsync-metadata"
            | "--http2"
            | "--no-http2"
            | "--ignore-try"
            | "--no-ignore-try"
            | "--impersonate-linux-26"
            | "--no-impersonate-linux-26"
            | "--keep-build-log"
            | "--no-keep-build-log"
            | "--keep-derivations"
            | "--no-keep-derivations"
            | "--keep-env-derivations"
            | "--no-keep-env-derivations"
            | "--keep-failed"
            | "--no-keep-failed"
            | "--keep-going"
            | "--no-keep-going"
            | "--keep-outputs"
            | "--no-keep-outputs"
            | "--preallocate-contents"
            | "--no-preallocate-contents"
            | "--print-missing"
            | "--no-print-missing"
            | "--pure-eval"
            | "--no-pure-eval"
            | "--require-sigs"
            | "--no-require-sigs"
            | "--restrict-eval"
            | "--no-restrict-eval"
            | "--run-diff-hook"
            | "--no-run-diff-hook"
            | "--sandbox"
            | "--no-sandbox"
            | "--sandbox-fallback"
            | "--no-sandbox-fallback"
            | "--show-trace"
            | "--no-show-trace"
            | "--substitute"
            | "--no-substitute"
            | "--sync-before-registering"
            | "--no-sync-before-registering"
            | "--trace-function-calls"
            | "--no-trace-function-calls"
            | "--trace-verbose"
            | "--no-trace-verbose"
            | "--use-case-hack"
            | "--no-use-case-hack"
            | "--use-registries"
            | "--no-use-registries"
            | "--use-sqlite-wal"
            | "--no-use-sqlite-wal"
            | "--warn-dirty"
            | "--no-warn-dirty"
            | "--relaxed-sandbox"
            // `nix develop` options
            | "--build"
            | "--check"
            | "--configure"
            | "--debugger"
            | "-i" | "--ignore-environment"
            | "--install"
            | "--installcheck"
            | "--unpack"
            | "--impure"
            | "--commit-lock-file"
            | "--no-registries"
            | "--no-update-lock-file"
            | "--no-write-lock-file"
            | "--recreate-lock-file"
            | "--derivation"
            => {}

            "build" | "develop" | "flake" | "help" | "profile" | "repl" | "run" | "search"
            | "shell" | "bundle" | "copy" | "edit" | "eval" | "fmt" | "log" | "path-info"
            | "registry" | "why-depends" | "daemon" | "describe-stores" | "hash" | "key"
            | "nar" | "print-dev-env" | "realisation" | "show-config" | "show-derivation"
            | "store" | "doctor" | "upgrade-nix" => {
                // Top-level subcommand.

                // Replace `subcommand` unless it already has a value.
                if subcommand.is_none() {
                    subcommand = Some(args[i].clone());
                }
            }

            _ => {
                // Unknown argument, ignore.
            }
        }

        i += 1;
    }

    // We want to add our `--command` flag right at the end, because `--command` makes *all the
    // rest of the positional arguments* get parsed as arguments to the command.
    //
    // Note that this behavior is unlike `nix-shell`, where the `--command` flag takes one argument
    // that may include spaces...
    match subcommand.as_deref() {
        Some("develop") | Some("shell") => {
            ret.push("--command".into());
            ret.push(command.into());
        }

        _ => {}
    }

    ret
}

/// Transform arguments to a `nix-shell` invocation to run the specified `command`.
fn transform_nix_shell(args: Vec<String>, command: &str) -> Vec<String> {
    let mut ret = Vec::with_capacity(args.len() + 2);
    ret.push("--command".into());
    ret.push(command.into());

    let mut i = 0;
    while i < args.len() {
        ret.push(args[i].clone());
        match args[i].as_str() {
            // Two arguments
            "--arg" | "--argstr"
                // `nix-store`
                | "--option"
                // From `nix-build` source...
                | "--override-flake"
                => {
                ret.push(args[i + 1].clone());
                ret.push(args[i + 2].clone());
                i += 2;
            }

            // One argument
            "--attr" | "-A" | "--exclude" | "--keep"
                | "-i" // Interpreter, shebang only
                // `nix-store`
                | "--add-root"
                // From `nix-build` source...
                | "--cores"
                | "--max-silent-time"
                | "--timeout"
                | "--store-uri"
                | "-I" | "--include"
                | "--eval-store"
                | "-o" | "--out-link"
                => {
                ret.push(args[i + 1].clone());
                i += 1;
            }

            // Zero arguments
            "--pure" | "--impure"
                // `--packages` changes the meaning of positional arguments, so we effectively
                // ignore it.
                | "-p" | "--packages"
                // Also changes meaning of positional arguments.
                | "-E" | "--expr"
                // `nix-store`
                | "--dry-run" | "--ignore-unknown" | "--check"
                // From `nix-build` source...
                | "-Q" | "--no-build-output"
                | "-K" | "--keep-failed"
                | "-k" | "--keep-going"
                | "--fallback"
                | "--readonly-mode"
                | "--no-gc-warning"
                | "--add-drv-link" | "--indirect"
                | "--no-out-link" | "--no-link"
                | "--drv-link"
                | "--repair"
                | "--run-env"
                => {
                // Nothing to skip.
            }

            "--command" | "--run"
                | "--help"
                | "--version"
                => {
                // We already have a command to run; don't add our own `--command {command}`
                // arguments.
                return args;
            }

            _ => {
                // Unknown argument, ignore.
            }
        }

        i += 1;
    }

    ret
}
