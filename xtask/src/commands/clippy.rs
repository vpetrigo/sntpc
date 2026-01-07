use crate::utils;
use crate::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Runs Clippy linting with strict rules on all code in the project.
///
/// This function runs clippy on the main sntpc crate (with all features and
/// without default features) and all examples with strict linting rules including
/// `clippy::all` and `clippy::pedantic`.
///
/// # Errors
///
/// Returns an error if:
/// - Cargo clippy command execution fails
/// - Clippy finds any linting violations
/// - Failed to discover examples
/// - Any clippy process returns a non-zero exit code
pub fn run_clippy() -> Result<()> {
    utils::print_header("Running Clippy with strict linting on all code...");

    // Run clippy on all workspace crates
    let workspace_crates = utils::get_workspace_crates()?;

    for crate_path in workspace_crates {
        utils::print_step("Clippy", &format!("Workspace crate: {crate_path}"));

        let manifest_path = format!("{crate_path}/Cargo.toml");
        let status = Command::new("cargo")
            .args([
                "clippy",
                "--manifest-path",
                &manifest_path,
                "--all-features",
                "--",
                "-D",
                "clippy::all",
                "-D",
                "clippy::pedantic",
            ])
            .status()
            .context(format!("Failed to execute cargo clippy on {crate_path}"))?;

        if !status.success() {
            utils::print_error(&format!("✗ Clippy found issues in {crate_path}"));
            anyhow::bail!("Clippy found issues in {crate_path}");
        }

        utils::print_step_success(&format!("Workspace crate: {crate_path}"));
    }

    // Run clippy on the main sntpc crate with all features
    utils::print_step("Clippy", "Main sntpc crate (all features)");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--manifest-path",
            "sntpc/Cargo.toml",
            "--all-features",
            "--",
            "-D",
            "clippy::all",
            "-D",
            "clippy::pedantic",
        ])
        .status()
        .context("Failed to execute cargo clippy on main crate")?;

    if !status.success() {
        utils::print_error("✗ Clippy found issues in main crate (all features)");
        anyhow::bail!("Clippy found issues in main crate");
    }

    utils::print_step_success("Main sntpc crate (all features)");

    // Run clippy on the main sntpc crate with no default features
    utils::print_step("Clippy", "Main sntpc crate (no default features)");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--manifest-path",
            "sntpc/Cargo.toml",
            "--no-default-features",
            "--",
            "-D",
            "clippy::all",
            "-D",
            "clippy::pedantic",
        ])
        .status()
        .context("Failed to execute cargo clippy on main crate (no default features)")?;

    if !status.success() {
        utils::print_error("✗ Clippy found issues in main crate (no default features)");
        anyhow::bail!("Clippy found issues in main crate");
    }

    utils::print_step_success("Main sntpc crate (no default features)");

    // Run clippy on all examples
    let all_examples = utils::get_all_examples()?;
    let nostd_examples = utils::get_nostd_examples()?;

    for example in all_examples {
        let is_nostd = nostd_examples.contains(&example);
        clippy_run(&example, is_nostd)?;
    }

    utils::print_success("✓ All Clippy checks passed!");
    Ok(())
}

fn clippy_run(example_name: &str, no_std: bool) -> Result<()> {
    let example_dir = format!("examples/{example_name}");

    if !Path::new(&example_dir).exists() {
        utils::print_step_warning(&format!("⚠ Skipping {example_name}: directory not found"));
        return Ok(());
    }

    let feature_msg = if no_std { " (no-std)" } else { "" };
    utils::print_step("Clippy", &format!("{example_name}{feature_msg}"));

    let mut args = Vec::new();
    if no_std {
        args.extend_from_slice(&["--no-default-features", "--profile", "no-std"]);
    }

    utils::run_cargo_clippy(&example_dir, &args)?;
    utils::print_step_success(&format!("{example_name}{feature_msg}"));

    Ok(())
}
