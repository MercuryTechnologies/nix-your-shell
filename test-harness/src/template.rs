use std::fs;
use std::path::Path;

use miette::Context;
use miette::IntoDiagnostic;

/// Process a template file by replacing placeholders with Nix store paths.
///
/// The template file (with `.in` extension) contains placeholders like `@bash@`
/// that get replaced with actual paths from environment variables. The processed
/// content is written without the `.in` extension, and the original is removed.
///
/// Returns an error if any required environment variables are not set.
pub(crate) fn process_template(template_path: &Path) -> miette::Result<()> {
    let bash = std::env::var("NIX_YOUR_SHELL_TEST_BASH")
        .into_diagnostic()
        .wrap_err("NIX_YOUR_SHELL_TEST_BASH not set")?;
    let coreutils = std::env::var("NIX_YOUR_SHELL_TEST_COREUTILS")
        .into_diagnostic()
        .wrap_err("NIX_YOUR_SHELL_TEST_COREUTILS not set")?;
    let system = std::env::var("NIX_YOUR_SHELL_TEST_SYSTEM")
        .into_diagnostic()
        .wrap_err("NIX_YOUR_SHELL_TEST_SYSTEM not set")?;

    let content = fs::read_to_string(template_path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to read template: {}", template_path.display()))?;

    let processed = content
        .replace("@bash@", &bash)
        .replace("@coreutils@", &coreutils)
        .replace("@system@", &system);

    let output_path = template_path.with_extension("");
    fs::write(&output_path, processed)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to write: {}", output_path.display()))?;

    fs::remove_file(template_path)
        .into_diagnostic()
        .wrap_err_with(|| format!("Failed to remove: {}", template_path.display()))?;

    Ok(())
}
