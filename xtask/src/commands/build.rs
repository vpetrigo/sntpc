use crate::utils;
use crate::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn build_nostd_examples() -> Result<()> {
    utils::print_header("Building no-std examples...");

    let examples = utils::get_nostd_examples()?;

    if examples.is_empty() {
        utils::print_warning("⚠ No no-std examples found");
        return Ok(());
    }

    for example in examples {
        build_example(&example, "no-std")?;
    }

    utils::print_success("✓ All no-std examples built successfully!");
    Ok(())
}

pub fn build_unix_examples() -> Result<()> {
    utils::print_header("Building Unix-specific examples...");

    let examples = utils::get_unix_examples()?;

    if examples.is_empty() {
        utils::print_warning("⚠ No Unix-specific examples found");
        return Ok(());
    }

    // Check if we're on a Unix-like system
    if !cfg!(unix) {
        utils::print_warning("Warning: Not on Unix system, some examples may fail");
    }

    for example in examples {
        build_example(&example, "unix")?;
    }

    utils::print_success("✓ All Unix-specific examples built successfully!");
    Ok(())
}

pub fn build_cross_platform_examples() -> Result<()> {
    utils::print_header("Building cross-platform examples...");

    let examples = utils::get_cross_platform_examples()?;

    if examples.is_empty() {
        utils::print_warning("⚠ No cross-platform examples found");
        return Ok(());
    }

    for example in examples {
        build_example(&example, "cross-platform")?;
    }

    utils::print_success("✓ All cross-platform examples built successfully!");
    Ok(())
}

pub fn build_all_examples() -> Result<()> {
    utils::print_header("Building all examples...");

    build_nostd_examples()?;
    build_unix_examples()?;

    utils::print_success("✓ All examples built successfully!");
    Ok(())
}

pub fn build_main_crate(all_features: bool, no_default_features: bool) -> Result<()> {
    let mut message = "Building main sntpc crate".to_string();

    if all_features {
        message.push_str(" (with all features)");
    } else if no_default_features {
        message.push_str(" (with no default features)");
    }

    message.push_str("...");
    utils::print_header(&message);

    let mut command = Command::new("cargo");
    command.args(["build", "--manifest-path", "sntpc/Cargo.toml"]);

    if all_features && no_default_features {
        utils::print_error("✗ Cannot specify both --all-features and --no-default-features");
        anyhow::bail!("Conflicting feature flags");
    }

    if all_features {
        command.arg("--all-features");
    } else if no_default_features {
        command.arg("--no-default-features");
    }

    let status = command
        .status()
        .context("Failed to execute cargo build for the main crate")?;

    if !status.success() {
        utils::print_error("✗ Failed to build the main crate");
        anyhow::bail!("Build failed");
    }

    utils::print_success("✓ Main sntpc crate built successfully!");
    Ok(())
}

fn build_example(example_name: &str, category: &str) -> Result<()> {
    let example_dir = format!("examples/{example_name}");

    if !Path::new(&example_dir).exists() {
        utils::print_step_warning(&format!("⚠ Skipping {example_name}: directory not found"));
        return Ok(());
    }

    utils::print_step("Building", example_name);

    let mut cmd = Command::new("cargo");
    cmd.args(["build"]).current_dir(&example_dir);

    // Add special flags for no-std examples
    if category == "no-std" {
        cmd.args(["--profile", "no-std"]);
    }

    let status = cmd
        .status()
        .context(format!("Failed to execute cargo build for {example_name}"))?;

    if !status.success() {
        utils::print_step_error(&format!("✗ Failed to build {example_name}"));
        anyhow::bail!("Build failed for {example_name}");
    }

    utils::print_step_success(example_name);
    Ok(())
}
