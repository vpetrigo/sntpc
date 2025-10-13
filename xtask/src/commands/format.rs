use crate::Result;
use crate::utils;
use std::path::Path;

pub fn check_formatting() -> Result<()> {
    utils::print_header("Checking code formatting for main crate and all examples...");

    // Check main crate
    check_format_crate("sntpc", "Main crate")?;

    // Check all examples
    let all_examples = utils::get_all_examples()?;

    for example in all_examples {
        let example_path = format!("examples/{example}");
        check_format_crate(&example_path, &format!("Example: {example}"))?;
    }

    utils::print_success("✓ All formatting checks passed!");
    Ok(())
}

pub fn fix_formatting() -> Result<()> {
    utils::print_header("Fixing code formatting for the main crate and all examples...");

    // Fix main crate
    fix_format_crate("sntpc", "Main crate")?;

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
