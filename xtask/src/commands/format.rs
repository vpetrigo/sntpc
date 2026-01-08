use crate::Result;
use crate::utils;
use std::path::Path;

/// Checks code formatting for the main crate and all examples.
///
/// This function runs `cargo fmt --check` on the main sntpc crate and all
/// discovered examples to verify that the code is properly formatted without
/// making any changes.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover examples
/// - Cargo fmt check command execution fails
/// - Any formatting violations are found
/// - The format check process returns a non-zero exit code
pub fn check_formatting() -> Result<()> {
    utils::print_header("Checking code formatting for main crate and all examples...");

    // Check main crates
    let main_crates = utils::get_workspace_crates()?;

    for main_crate in main_crates {
        check_format_crate(main_crate.as_str(), format!("Main crate: {main_crate}").as_str())?;
    }

    // Check all examples
    let all_examples = utils::get_all_examples()?;

    for example in all_examples {
        let example_path = format!("examples/{example}");
        check_format_crate(&example_path, &format!("Example: {example}"))?;
    }

    utils::print_success("✓ All formatting checks passed!");
    Ok(())
}

/// Fixes code formatting for the main crate and all examples.
///
/// This function runs `cargo fmt` on the main sntpc crate and all discovered
/// examples to automatically fix formatting issues according to rustfmt rules.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover examples
/// - Cargo fmt command execution fails
/// - The formatting process returns a non-zero exit code
pub fn fix_formatting() -> Result<()> {
    utils::print_header("Fixing code formatting for the main crate and all examples...");

    // Fix main crate
    let main_crates = utils::get_workspace_crates()?;

    for main_crate in main_crates {
        fix_format_crate(main_crate.as_str(), format!("Main crate: {main_crate}").as_str())?;
    }

    // Fix all examples
    let all_examples = utils::get_all_examples()?;

    for example in all_examples {
        let example_path = format!("examples/{example}");
        fix_format_crate(&example_path, &format!("Example: {example}"))?;
    }

    utils::print_success("✓ All formatting issues fixed!");
    Ok(())
}

fn check_format_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        utils::print_step_warning(&format!("⚠ Skipping {name}: directory not found"));
        return Ok(());
    }

    utils::print_step("Checking format", name);
    utils::run_cargo_fmt_check(path)?;
    utils::print_step_success(name);

    Ok(())
}

fn fix_format_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        utils::print_step_warning(&format!("⚠ Skipping {name}: directory not found"));
        return Ok(());
    }

    utils::print_step("Fixing format", name);
    utils::run_cargo_fmt_fix(path)?;
    utils::print_step_success(name);

    Ok(())
}
