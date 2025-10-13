use crate::Result;
use crate::utils;

pub fn run_tests() -> Result<()> {
    utils::print_header("Running tests for main sntpc crate...");

    utils::run_cargo_test("sntpc/Cargo.toml")?;

    utils::print_success("✓ All tests passed!");
    Ok(())
}
