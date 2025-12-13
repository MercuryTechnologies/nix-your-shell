#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process;

use calm_io::stdout as println;
use camino::Utf8Path;
use camino::Utf8PathBuf;
use clap::Parser;
use miette::miette;
use miette::Context;
use miette::IntoDiagnostic;

mod shell;
use shell::Shell;
use shell::ShellKind;

mod nix;

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

/// Environment variable that tracks packages across nested nix shells.
///
/// This is set when a Nix shell is launched, and is used to track the packages
/// that have been installed in the shell.
const NIX_YOUR_SHELL_PKGS_VAR: &str = "NIX_YOUR_SHELL_PKGS";

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

    /// Add information about the current shell to the right of the shell prompt.
    #[arg(long, default_value_t = false)]
    info_right: bool,

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
    /// Print information about the current shell.
    ShellInfo,
}

impl Default for Command {
    fn default() -> Self {
        Self::Env
    }
}

/// Build the accumulated packages environment variable value.
///
/// This combines any existing packages from the environment with new packages
/// to support nested nix shells.
fn build_packages_env(new_packages: &[String]) -> String {
    let existing = std::env::var(NIX_YOUR_SHELL_PKGS_VAR).unwrap_or_default();

    if new_packages.is_empty() {
        return existing;
    }

    let new_pkgs_str = new_packages.join(" ");

    if existing.is_empty() {
        new_pkgs_str
    } else {
        format!("{} {}", existing, new_pkgs_str)
    }
}

fn main() -> miette::Result<()> {
    let opts = Opts::parse();
    install_tracing(&opts.log)?;

    let shell = Shell::from_path(&opts.shell)?;
    tracing::debug!(%shell, input=opts.shell, "Detected shell");

    match opts.command.unwrap_or_default() {
        Command::Env => {
            let template = match shell.kind {
                ShellKind::Zsh | ShellKind::Bash => {
                    include_str!("../data/env.sh.j2")
                }

                ShellKind::Fish => {
                    include_str!("../data/env.fish.j2")
                }

                ShellKind::Nushell => {
                    include_str!("../data/env.nu.j2")
                }

                ShellKind::Xonsh => {
                    include_str!("../data/env.xsh.j2")
                }

                ShellKind::Other(shell) => {
                    return Err(miette!(
                        "I don't know how to generate a shell environment for `{shell}`\n\
                        Note: Supported shells are: `zsh`, `fish`, `nushell`, `xonsh`, and `bash`"
                    ))
                }
            };

            let current_exe =
                current_exe().wrap_err("Unable to determine absolute path of `nix-your-shell`")?;

            let formatted = minijinja::render!(
                template,
                executable => if opts.absolute || !executable_is_on_path(&current_exe)? {
                    current_exe.as_str()
                } else {
                    "nix-your-shell"
                },
                info_right => opts.info_right,
                extra_args => if opts.nom { vec!["--nom"] } else { vec![] },
                shell => shell.path.as_str(),
            );

            let _ = println!("{formatted}");
            Ok(())
        }

        Command::NixShell { args } => {
            let result = nix::transform_nix_shell(args, shell.path.as_str());
            let prog = if opts.nom { "nom-shell" } else { "nix-shell" };
            let command = shell_words::join(
                std::iter::once(prog).chain(result.args.iter().map(|s| s.as_str())),
            );
            tracing::debug!(
                %command,
                packages = ?result.packages,
                "Launching nix-shell"
            );

            // Build the accumulated packages string
            let pkgs_env = build_packages_env(&result.packages);

            Err(process::Command::new(prog)
                .args(result.args)
                .env(NIX_SOURCED_VAR, "1")
                .env(NIX_YOUR_SHELL_PKGS_VAR, pkgs_env)
                .exec())
            .into_diagnostic()
            .wrap_err_with(|| format!("Unable to launch {command}"))
        }

        Command::Nix { args } => {
            let result = nix::transform_nix(args, shell.path.as_str());
            let prog = if opts.nom
                && result
                    .subcommand
                    .as_ref()
                    .map(|subcommand| ["shell", "build", "develop"].contains(&subcommand.as_str()))
                    .unwrap_or(false)
            {
                "nom"
            } else {
                "nix"
            };
            let command = shell_words::join(
                std::iter::once(prog).chain(result.args.iter().map(|s| s.as_str())),
            );
            tracing::debug!(
                %command,
                packages = ?result.packages,
                "Launching nix"
            );

            // Build the accumulated packages string
            let pkgs_env = build_packages_env(&result.packages);

            Err(process::Command::new(prog)
                .args(result.args)
                .env(NIX_SOURCED_VAR, "1")
                .env(NIX_YOUR_SHELL_PKGS_VAR, pkgs_env)
                .exec())
            .into_diagnostic()
            .wrap_err_with(|| format!("Unable to launch {command}"))
        }

        Command::ShellInfo => {
            let named_shell = std::env::var("name").ok();
            let pkgs = std::env::var(NIX_YOUR_SHELL_PKGS_VAR).unwrap_or_default();

            let in_nix_shell = std::env::var("IN_NIX_SHELL").is_ok_and(|value| !value.is_empty())
                || std::env::var("IN_NIX_RUN").is_ok_and(|value| !value.is_empty());
            if !in_nix_shell {
                return Ok(());
            }

            let mut output = pkgs;
            if let Some(named_shell) = named_shell {
                if named_shell != "shell" {
                    output = format!("{output} {named_shell}");
                }
            }
            let output = output.trim();
            if !output.is_empty() {
                // Include a single empty space after the output since
                // Some terminals will not display the output if it is not followed by a space.
                let _ = println!("\x1b[1;32m{output} \x1b[0m");
            }

            Ok(())
        }
    }
}

fn install_tracing(filter_directives: &str) -> miette::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Layer;

    let env_filter = tracing_subscriber::EnvFilter::try_new(filter_directives).into_diagnostic()?;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .without_time()
        .with_filter(env_filter);

    let registry = tracing_subscriber::registry();

    registry.with(fmt_layer).init();

    Ok(())
}

/// Get the path to the current executable.
fn current_exe() -> miette::Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(
        std::env::current_exe()
            .into_diagnostic()
            .wrap_err("Unable to determine current executable")?,
    )
    .map_err(|path_buf| miette!("Path is not UTF-8: {path_buf:?}"))
}

fn executable_is_on_path(executable: &Utf8Path) -> miette::Result<bool> {
    let directory = executable
        .parent()
        .ok_or_else(|| miette!("Executable has no parent directory: {executable:?}"))?;
    let path = std::env::var("PATH")
        .into_diagnostic()
        .wrap_err("Failed to get $PATH environment variable")?;
    Ok(path
        .split(':')
        .map(Utf8Path::new)
        .any(|component| component == directory))
}
