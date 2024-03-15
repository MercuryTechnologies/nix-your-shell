use std::process::Command;

mod command;
pub use command::CommandExt;

/// Get a `nix-your-shell` command.
pub fn command() -> Command {
    Command::new(test_bin::get_test_bin("nix-your-shell").get_program())
}
