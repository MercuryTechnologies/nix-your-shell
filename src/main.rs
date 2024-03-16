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
                    return Err(eyre!(
                        "I don't know how to generate a shell environment for `{shell}`"
                    ))
                    .note("Supported shells are: `zsh`, `fish`, `nushell`, `xonsh`, and `bash`")
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
                extra_args => if opts.nom { vec!["--nom"] } else { vec![] },
                shell => shell.path.as_str(),
            );

            let _ = println!("{formatted}");
            Ok(())
        }

        Command::NixShell { args } => {
            let new_args = nix::transform_nix_shell(args, shell.path.as_str());
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
            let new_args = nix::transform_nix(args, shell.path.as_str());
            let prog = if opts.nom
                && new_args
                    .subcommand
                    .map(|subcommand| ["shell", "build", "develop"].contains(&subcommand.as_str()))
                    .unwrap_or(false)
            {
                "nom"
            } else {
                "nix"
            };
            tracing::debug!(
                command = shell_words::join(
                    std::iter::once(prog).chain(new_args.args.iter().map(|s| s.as_str()))
                ),
                "Launching nix"
            );
            Err(process::Command::new(prog)
                .args(new_args.args)
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
