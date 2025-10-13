use crate::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Runs `cargo build` with the specified manifest path and additional arguments.
///
/// # Arguments
///
/// * `manifest_path` - Path to the Cargo.toml file
/// * `args` - Additional arguments to pass to cargo build
///
/// # Errors
///
/// Returns an error if:
/// - Failed to execute the cargo build command
/// - The build process returns a non-zero exit code
pub fn run_cargo_build(manifest_path: &str, args: &[&str]) -> Result<()> {
    let mut command = Command::new("cargo");
    command.args(["build", "--manifest-path", manifest_path]);
    command.args(args);

    let status = command
        .status()
        .with_context(|| format!("Failed to execute cargo build for {manifest_path}"))?;

    if !status.success() {
        anyhow::bail!("Build failed for {manifest_path}");
    }

    Ok(())
}

/// Runs `cargo test` with the specified manifest path.
///
/// # Arguments
///
/// * `manifest_path` - Path to the Cargo.toml file
///
/// # Errors
///
/// Returns an error if:
/// - Failed to execute the cargo test command
/// - Any tests fail (non-zero exit code)
pub fn run_cargo_test(manifest_path: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args(["test", "--manifest-path", manifest_path])
        .status()
        .context("Failed to execute cargo test")?;

    if !status.success() {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

/// Runs `cargo check` in the specified directory path.
///
/// # Arguments
///
/// * `path` - Directory path containing the Rust project
///
/// # Errors
///
/// Returns an error if:
/// - The specified path does not exist
/// - Failed to execute the cargo check command
/// - The check process finds compilation errors (non-zero exit code)
pub fn run_cargo_check(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("Path does not exist: {path}");
    }

    let status = Command::new("cargo")
        .args(["check"])
        .current_dir(path)
        .status()
        .with_context(|| format!("Failed to execute cargo check for {path}"))?;

    if !status.success() {
        anyhow::bail!("Check failed for {path}");
    }

    Ok(())
}

/// Runs `cargo clippy` in the specified directory path with the given arguments.
///
/// # Arguments
///
/// * `path` - Directory path containing the Rust project
/// * `args` - Additional arguments to pass to cargo clippy
///
/// # Errors
///
/// Returns an error if:
/// - The specified path does not exist
/// - Failed to execute the cargo clippy command
/// - Clippy finds issues (non-zero exit code)
pub fn run_cargo_clippy(path: &str, args: &[&str]) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("Path does not exist: {path}");
    }

    let mut cmd = Command::new("cargo");
    cmd.args(["clippy"]).current_dir(path);
    cmd.args(args);
    cmd.args(["--", "-D", "clippy::all", "-D", "clippy::pedantic"]);

    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute cargo clippy for {path}"))?;

    if !status.success() {
        anyhow::bail!("Clippy found issues in {path}");
    }

    Ok(())
}

/// Runs `cargo fmt` with the `--check` flag in the specified directory path.
///
/// # Arguments
///
/// * `path` - Directory path containing the Rust project
///
/// # Errors
///
/// Returns an error if:
/// - The specified path does not exist
/// - Failed to execute the cargo fmt check command
/// - The format check finds issues (non-zero exit code)
pub fn run_cargo_fmt_check(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("Path does not exist: {path}");
    }

    let status = Command::new("cargo")
        .args(["fmt", "--check"])
        .current_dir(path)
        .status()
        .with_context(|| format!("Failed to execute cargo fmt check for {path}"))?;

    if !status.success() {
        anyhow::bail!("Format check failed for {path}");
    }

    Ok(())
}

/// Runs `cargo fmt` in the specified directory path to automatically fix formatting issues.
///
/// # Arguments
///
/// * `path` - Directory path containing the Rust project
///
/// # Errors
///
/// Returns an error if:
/// - The specified path does not exist
/// - Failed to execute the cargo fmt command
/// - The format fix process returns a non-zero exit code
pub fn run_cargo_fmt_fix(path: &str) -> Result<()> {
    if !Path::new(path).exists() {
        anyhow::bail!("Path does not exist: {path}");
    }

    let status = Command::new("cargo")
        .args(["fmt", "--all"])
        .current_dir(path)
        .status()
        .with_context(|| format!("Failed to execute cargo fmt for {path}"))?;

    if !status.success() {
        anyhow::bail!("Format fix failed for {path}");
    }

    Ok(())
}

/// Runs `cargo clean` with the specified manifest path.
///
/// # Arguments
///
/// * `manifest_path` - Path to the Cargo.toml file
///
/// # Errors
///
/// Returns an error if:
/// - Failed to execute the cargo clean command
pub fn run_cargo_clean(manifest_path: &str) -> Result<()> {
    Command::new("cargo")
        .args(["clean", "--manifest-path", manifest_path])
        .output()
        .with_context(|| format!("Failed to clean {manifest_path}"))?;

    Ok(())
}
