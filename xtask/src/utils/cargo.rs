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
pub fn run_cargo_test(manifest_path: &str, features: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args([
            "nextest",
            "--manifest-path",
            manifest_path,
            "run",
            features,
            "--retries",
            "5",
            "--profile",
            "ci",
        ])
        .status()
        .context("Failed to execute cargo test")?;

    if !status.success() {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

/// Executes the tests for a Rust project using `cargo nextest` with specific options and generates a code coverage report in `lcov` format.
///
/// # Arguments
/// - `manifest_path`: A string slice that represents the path to the `Cargo.toml` manifest file of the Rust project.
///
/// # Returns
/// - `Ok(())`: If the tests execute successfully with coverage.
/// - `Err(anyhow::Error)`: If an error occurs while executing the command or if the tests fail.
///
/// # Behavior
/// - Invokes the `cargo nextest` command to run tests with the following options:
///   - Disables default features.
///   - Enables specified features: `std`, `std-socket`, and `sync`.
///   - Retries failed tests up to 3 times.
///   - Uses the `ci` test profile.
///   - Generates a code coverage report in `lcov` format.
///   - Outputs the coverage report to a file named `lcov.info`.
///
/// # Errors
/// - Returns an error if the command fails to execute (e.g., `cargo` is not installed or there is an issue with the project).
/// - Returns an error if the test run does not complete successfully (e.g., test failures).
///
/// # Dependencies
/// - Requires the `nextest` cargo subcommand to be installed.
/// - Assumes the project is configured to use `nextest` for testing.
pub fn run_tests_with_coverage(manifest_path: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args([
            "llvm-cov",
            "nextest",
            "--manifest-path",
            manifest_path,
            "--no-default-features",
            "--features",
            "std,sync",
            "--retries",
            "5",
            "--lcov",
            "--output-path",
            "lcov.info",
            "--profile",
            "ci",
        ])
        .status()
        .context("Failed to execute cargo with coverage")?;

    if !status.success() {
        anyhow::bail!("Tests with coverage failed");
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

    let mut args = vec!["check"];

    if path.contains("no-std") {
        args.extend_from_slice(&["--profile", "no-std"]);
    }

    let status = Command::new("cargo")
        .args(args)
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
