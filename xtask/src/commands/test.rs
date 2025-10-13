use crate::Result;
use crate::utils;

/// Runs all tests for the main sntpc crate.
///
/// This function executes `cargo test` on the main sntpc crate to run all
/// unit tests, integration tests, and doctests.
///
/// # Errors
///
/// Returns an error if:
/// - Cargo test command execution fails
/// - Any tests fail
/// - The test process returns a non-zero exit code
pub fn run_tests() -> Result<()> {
    utils::print_header("Running tests for main sntpc crate...");
    utils::run_cargo_test("sntpc/Cargo.toml")?;
    utils::print_success("✓ All tests passed!");

    Ok(())
}
