//! Test-local state.

use std::cell::Cell;
use std::sync::OnceLock;

pub use nix_your_shell::ShellKind;

static NIX_YOUR_SHELL_BIN: OnceLock<&'static str> = OnceLock::new();

thread_local! {
    static CURRENT_SHELL: Cell<Option<ShellKind>> = const { Cell::new(None) };
}

pub fn ensure_nix_your_shell_bin(path: &'static str) {
    // We don't care if it was already set.
    let _ = NIX_YOUR_SHELL_BIN.set(path);
}

pub fn ensure_shell_kind(shell: ShellKind) {
    CURRENT_SHELL.set(Some(shell));
}

pub(crate) fn get_nix_your_shell_bin() -> &'static str {
    NIX_YOUR_SHELL_BIN.get().expect("TestEnvironment::command can only be called from tests annotated with #[test_harness::test]")
}

pub(crate) fn get_current_shell() -> ShellKind {
    CURRENT_SHELL
        .get()
        .expect("Test can only be created in tests annotated with #[test_harness::test]")
}
