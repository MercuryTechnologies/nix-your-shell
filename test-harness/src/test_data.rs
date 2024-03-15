use std::path::Path;
use std::path::PathBuf;

use command_error::CommandExt;
use miette::Context;
use miette::IntoDiagnostic;

use crate::git::git_command;
use crate::template::process_template;

/// Get the directory containing test data.
pub(crate) fn test_data_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR points to test-harness, so we need to go up one level
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test-harness should have a parent directory")
        .join("tests")
}

/// Set up a test environment with a Git repo in a temporary directory.
///
/// Returns the `TempDir` handle. The directory will be automatically cleaned up
/// when the returned `TempDir` is dropped.
pub(crate) fn copy_test_data(subpath: impl AsRef<Path>) -> miette::Result<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()
        .into_diagnostic()
        .wrap_err("Failed to create tempdir")?;
    let temp_path = temp_dir.path();

    fs_extra::dir::copy(test_data_dir().join(subpath), temp_path, &{
        let mut options = fs_extra::dir::CopyOptions::new();
        options.content_only = true; // Copy only the contents, not the directory itself
        options
    })
    .into_diagnostic()
    .wrap_err("Failed to copy test data")?;

    process_template(&temp_path.join("config.nix.in"))?;

    git_command()
        .arg("init")
        .current_dir(temp_path)
        .output_checked_utf8()?;

    // From `man git-config`:
    //
    // > When writing, the new value is written to the repository local configuration file by
    // > default, ...
    git_command()
        .args(["config", "user.email", "user@example.com"])
        .current_dir(temp_path)
        .output_checked_utf8()?;

    git_command()
        .args(["config", "user.name", "Example User"])
        .current_dir(temp_path)
        .output_checked_utf8()?;

    git_command()
        .args(["add", "."])
        .current_dir(temp_path)
        .output_checked_utf8()?;

    git_command()
        .args(["commit", "-m", "Initial commit"])
        .current_dir(temp_path)
        .output_checked_utf8()?;

    Ok(temp_dir)
}
