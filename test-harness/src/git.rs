use std::process::Command;

/// Create a git command with isolated configuration.
///
/// This prevents git from reading the user's global/system config (which may have
/// commit signing enabled) and disables SSH agent access.
pub(crate) fn git_command() -> Command {
    let mut cmd = Command::new("git");
    // Prevent reading global/system git config (may have commit.gpgsign, etc.)
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null");
    cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
    // Disable SSH entirely
    cmd.env("SSH_AUTH_SOCK", "");
    cmd.env("GIT_SSH_COMMAND", "false");
    cmd
}
