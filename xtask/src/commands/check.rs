use crate::Result;
use crate::utils;
use std::path::Path;

/// Checks all code in the main crate and examples for compilation errors.
/// 
/// This function runs `cargo check` on the main sntpc crate and all discovered
/// examples to verify that the code compiles without errors.
/// 
/// # Errors
/// 
/// Returns an error if:
/// - Failed to discover examples
/// - Any cargo check command execution fails
/// - The check process returns a non-zero exit code for any crate or example
pub fn check_all() -> Result<()> {
    utils::print_header("Checking main crate and all examples...");

    // Check main crate
    check_crate("sntpc", "Main crate")?;

    // Check all examples
    let all_examples = utils::get_all_examples()?;

    for example in all_examples {
        let example_path = format!("examples/{example}");
        check_crate(&example_path, &format!("Example: {example}"))?;
    }

    utils::print_success("✓ All checks passed!");

    Ok(())
}

fn check_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        utils::print_step_warning(&format!("⚠ Skipping {name}: directory not found"));
        return Ok(());
    }

    utils::print_step("Checking", name);
    utils::run_cargo_check(path)?;
    utils::print_step_success(name);

    Ok(())
}
