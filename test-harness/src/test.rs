use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use miette::Context;
use miette::IntoDiagnostic;
use rexpect::session::PtySession;

use crate::state::get_current_shell;
use crate::state::get_nix_your_shell_bin;
use crate::state::ShellKind;
use crate::test_data::copy_test_data;

pub(crate) fn default_expect_timeout() -> Duration {
    match std::env::var("CI") {
        Ok(value) if value != "0" => Duration::from_secs(5),
        _ => Duration::from_secs(1),
    }
}

/// A test environment with an isolated local Nix store.
///
/// This provides an isolated Nix environment for running tests that require
/// `nix develop` or other Nix commands. Uses direct local store access via
/// `local?root=...` instead of a daemon.
pub struct Test {
    /// The shell being tested.
    shell: ShellKind,
    /// The temporary directory containing the test flake.
    /// Kept alive to ensure cleanup on drop.
    #[allow(dead_code)]
    flake_dir: tempfile::TempDir,
    /// Canonicalized path to the flake directory (resolves symlinks like /var -> /private/var).
    flake_path: PathBuf,
    /// The temporary directory containing the Nix store.
    /// Kept alive to ensure cleanup on drop.
    #[allow(dead_code)]
    nix_root: tempfile::TempDir,
    /// Canonicalized path to the nix root directory.
    nix_root_path: PathBuf,
    /// Default timeout for [`rexpect`] sessions.
    timeout: Option<Duration>,
}

impl Test {
    /// Create a new test environment with a local Nix store.
    ///
    /// This copies the test data to a temp directory and sets up an isolated
    /// Nix store for running Nix commands without needing a daemon.
    pub fn new(subpath: impl AsRef<Path>) -> miette::Result<Self> {
        let shell = get_current_shell();
        let flake_dir = copy_test_data(subpath)?;
        // Canonicalize to resolve symlinks (e.g., /var -> /private/var on macOS).
        // This prevents Nix from seeing mismatched paths and reporting "escapes from" errors.
        let flake_path = flake_dir
            .path()
            .canonicalize()
            .into_diagnostic()
            .wrap_err("Failed to canonicalize flake dir path")?;

        // Create a separate temp dir for the Nix store
        let nix_root = tempfile::tempdir()
            .into_diagnostic()
            .wrap_err("Failed to create nix root tempdir")?;
        let nix_root_path = nix_root
            .path()
            .canonicalize()
            .into_diagnostic()
            .wrap_err("Failed to canonicalize nix root path")?;

        let env = Self {
            shell,
            flake_dir,
            flake_path,
            nix_root,
            nix_root_path,
            timeout: Some(default_expect_timeout()),
        };

        // Create required directories
        fs::create_dir_all(env.nix_conf_dir())
            .into_diagnostic()
            .wrap_err("Failed to create conf dir")?;
        fs::create_dir_all(env.nix_store_dir())
            .into_diagnostic()
            .wrap_err("Failed to create store dir")?;
        fs::create_dir_all(env.nix_state_dir())
            .into_diagnostic()
            .wrap_err("Failed to create state dir")?;
        fs::create_dir_all(env.nix_log_dir())
            .into_diagnostic()
            .wrap_err("Failed to create log dir")?;
        fs::create_dir_all(env.home())
            .into_diagnostic()
            .wrap_err("Failed to create home dir")?;

        fs::write(env.home().join(".zshrc"), "")
            .into_diagnostic()
            .wrap_err("Failed to write .zshrc")?;

        fs::write(
            env.home().join(".xonshrc"),
            // Do our best to prevent `xonsh` from printing huge amounts of nonsense ANSI escapes.
            "$PROMPT = '$ '\n\
            $TITLE = ''\n\
            $BOTTOM_TOOLBAR = ''\n\
            $COLOR_INPUT = False\n\
            # This is fucking LOAD BEARING lmao the tests cannot see seemingly anything that xonsh\n\
            # outputs with the `prompt_toolkit` shell type.\n\
            $SHELL_TYPE = 'readline'\n\
            ",
        )
        .into_diagnostic()
        .wrap_err("Failed to write .xonshrc")?;

        // Write nix.conf with required experimental features and isolated settings
        fs::write(
            env.nix_conf_dir().join("nix.conf"),
            "experimental-features = nix-command flakes\n\
            substituters =\n\
            use-registries = false\n\
            sandbox = false\n\
            ",
        )
        .into_diagnostic()
        .wrap_err("Failed to write nix.conf")?;

        Ok(env)
    }

    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get a raw `nix-your-shell` command with the correct environment and working directory.
    ///
    /// This does not add the shell name or shell-specific arguments.
    /// Use this for testing nix-your-shell commands that don't use a shell subcommand,
    /// like `--help` or `--version`.
    pub fn command_raw(&self) -> Command {
        let mut command = Command::new(get_nix_your_shell_bin());
        command.current_dir(self.path()).envs(self.env_vars());
        command
    }

    /// Get a `nix-your-shell` command with the correct environment and working directory.
    ///
    /// The shell name is pre-filled as the first argument. Shell-specific arguments
    /// and environment variables are added automatically.
    pub fn command(&self) -> Command {
        let mut command = self.command_raw();

        // Add shell name as first argument
        command.arg(self.shell.to_string());

        // Add shell-specific arguments
        #[expect(clippy::single_match)]
        match &self.shell {
            ShellKind::Fish => {
                command.arg("--shell-arg=--no-config");
            }
            _ => {}
        }

        command
    }

    pub fn spawn(&self, command: &mut Command) -> Result<PtySession, rexpect::error::Error> {
        // lol
        let command = std::mem::replace(command, Command::new(""));
        rexpect::session::spawn_command(
            command,
            self.timeout.map(|duration| duration.as_millis() as u64),
        )
    }

    /// Get the path to the flake directory.
    pub fn path(&self) -> &Path {
        &self.flake_path
    }

    /// Get the path to the Nix store directory.
    pub fn nix_store_dir(&self) -> PathBuf {
        self.nix_root_path.join("nix/store")
    }

    /// Get the path to the Nix configuration directory.
    pub fn nix_conf_dir(&self) -> PathBuf {
        self.nix_root_path.join("etc")
    }

    /// Get the path to the Nix state directory.
    pub fn nix_state_dir(&self) -> PathBuf {
        self.nix_root_path.join("var/nix")
    }

    /// Get the path to the Nix log directory.
    pub fn nix_log_dir(&self) -> PathBuf {
        self.nix_root_path.join("var/log/nix")
    }

    /// Get the path to the home directory.
    pub fn home(&self) -> PathBuf {
        self.nix_root_path.join("home/me")
    }

    /// Get environment variables needed to use this Nix environment.
    pub fn env_vars(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        // Use the system Nix store but with isolated configuration and state.
        // We can't use a diverted store (local?root=...) on macOS because
        // building is not supported with diverted stores on that platform.
        // Set NIX_REMOTE to local to use local store directly (no daemon).
        env.insert("NIX_REMOTE".to_string(), "local".to_string());
        env.insert(
            "NIX_STORE_DIR".to_string(),
            self.nix_store_dir().to_string_lossy().to_string(),
        );
        env.insert(
            "NIX_CONF_DIR".to_string(),
            self.nix_conf_dir().to_string_lossy().to_string(),
        );
        // Redirect state directories to our temp dir (like Lix tests do).
        env.insert(
            "NIX_STATE_DIR".to_string(),
            self.nix_state_dir().to_string_lossy().to_string(),
        );
        env.insert(
            "NIX_LOG_DIR".to_string(),
            self.nix_log_dir().to_string_lossy().to_string(),
        );
        // Set HOME so Nix can create its cache directory
        env.insert(
            "HOME".to_string(),
            self.home().to_string_lossy().to_string(),
        );
        // Disable sandboxing for builds (needed when running inside a Nix build)
        env.insert("_NIX_TEST_NO_SANDBOX".to_string(), "1".to_string());
        // Allow symlinked store paths (needed on macOS where /var is a symlink to /private/var)
        env.insert("NIX_IGNORE_SYMLINK_STORE".to_string(), "1".to_string());

        // Tell Lix not to output ANSI colors.
        env.insert("NOCOLOR".to_string(), "1".to_string());

        // See: https://github.com/fish-shell/fish-shell/blob/72870d83311a5259bdb5ab11277a415691ca91a9/tests/pexpect_helper.py#L183
        env.insert(
            "FISH_TEST_NO_RECURRENT_QUERIES".to_string(),
            "1".to_string(),
        );

        // Shell-specific environment variables
        #[expect(clippy::single_match)]
        match &self.shell {
            ShellKind::Fish => {
                // See: https://github.com/fish-shell/fish-shell/blob/72870d83311a5259bdb5ab11277a415691ca91a9/tests/pexpect_helper.py#L194
                env.insert("TERM".to_string(), "dumb".to_string());
            }
            _ => {}
        }

        env
    }
}

impl Deref for Test {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}
