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
    utils::run_cargo_test("sntpc/Cargo.toml", "--all-features")?;
    utils::run_cargo_test("sntpc/Cargo.toml", "--no-default-features")?;
    utils::print_success("✓ All tests passed!");

    Ok(())
}

/// Executes the test suite for the main `sntpc` crate with code coverage enabled.
///
/// This function performs the following steps:
/// 1. Prints a header message indicating that tests are being run with code coverage.
/// 2. Invokes the utility function to run the tests with code coverage for the `sntpc` crate.
/// 3. Prints a success message if all tests pass successfully.
///
/// # Errors
///
/// This function will return an error in the following cases:
/// - If the `utils::run_tests_with_coverage` function fails to execute the tests properly.
/// - Any other issues encountered during the execution of this function.
pub fn run_tests_with_coverage() -> Result<()> {
    utils::print_header("Running tests for main sntpc crate with code coverage enabled...");
    utils::run_tests_with_coverage("sntpc/Cargo.toml")?;
    utils::print_success("✓ All tests passed!");

    Ok(())
}
