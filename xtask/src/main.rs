use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for sntpc crate and examples")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build no-std examples (simple-no-std)
    BuildNostd,
    /// Build Unix-specific examples (all except simple-no-std)
    BuildUnix,
    /// Build cross-platform examples (simple-request, tokio, timesync)
    BuildCrossPlatform,
    /// Build all examples
    BuildAll,
    /// Build the main sntpc crate
    BuildCrate {
        /// Build with all features enabled
        #[arg(long, conflicts_with = "no_default_features")]
        all_features: bool,
        /// Build with no default features
        #[arg(long, conflicts_with = "all_features")]
        no_default_features: bool,
    },
    /// Run tests for the main crate
    Test,
    /// Check all code (main crate and examples)
    Check,
    /// Run clippy on all code with strict linting
    Clippy,
    /// Check code formatting for the main crate and all examples
    Format {
        /// Check formatting without making changes
        #[arg(long, conflicts_with = "fix")]
        check: bool,
        /// Fix formatting issues
        #[arg(long, conflicts_with = "check")]
        fix: bool,
    },
    /// Clean all build artifacts
    Clean,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildNostd => build_nostd_examples(),
        Commands::BuildUnix => build_unix_examples(),
        Commands::BuildCrossPlatform => build_cross_platform_examples(),
        Commands::BuildAll => build_all_examples(),
        Commands::BuildCrate {
            all_features,
            no_default_features,
        } => build_main_crate(all_features, no_default_features),
        Commands::Test => run_tests(),
        Commands::Check => check_all(),
        Commands::Clean => clean_all(),
        Commands::Clippy => run_clippy(),
        Commands::Format { check, fix } => {
            if check {
                check_formatting()?;
            } else if fix {
                fix_formatting()?;
            } else {
                // Default to checking if no flag is provided
                check_formatting()?;
            }

            Ok(())
        }
    }
}

fn get_all_examples() -> Result<Vec<String>> {
    let examples_dir = Path::new("examples");
    if !examples_dir.exists() {
        return Ok(vec![]);
    }

    let mut examples = Vec::new();
    for entry in fs::read_dir(examples_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            // Check if it has a Cargo.toml file
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                examples.push(name.to_string());
            }
        }
    }
    examples.sort();
    Ok(examples)
}

fn get_nostd_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for no-std examples (currently just simple-no-std, but this could be extended)
    Ok(all_examples
        .into_iter()
        .filter(|name| name == "simple-no-std")
        .collect())
}

fn get_unix_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter out no-std examples for Unix examples
    Ok(all_examples
        .into_iter()
        .filter(|name| name != "simple-no-std")
        .collect())
}

fn get_cross_platform_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for cross-platform examples (examples that should work on any platform)
    let cross_platform = ["simple-request", "tokio", "timesync"];
    Ok(all_examples
        .into_iter()
        .filter(|name| cross_platform.contains(&name.as_str()))
        .collect())
}

fn build_nostd_examples() -> Result<()> {
    println!("{}", "Building no-std examples...".bright_blue().bold());

    let examples = get_nostd_examples()?;

    if examples.is_empty() {
        println!("{}", "⚠ No no-std examples found".bright_yellow());
        return Ok(());
    }

    for example in examples {
        build_example(&example)?;
    }

    println!("{}", "✓ All no-std examples built successfully!".bright_green().bold());
    Ok(())
}

fn build_unix_examples() -> Result<()> {
    println!("{}", "Building Unix-specific examples...".bright_blue().bold());

    let examples = get_unix_examples()?;

    if examples.is_empty() {
        println!("{}", "⚠ No Unix-specific examples found".bright_yellow());
        return Ok(());
    }

    // Check if we're on a Unix-like system
    if !cfg!(unix) {
        println!(
            "{}",
            "Warning: Not on Unix system, some examples may fail".bright_yellow()
        );
    }

    for example in examples {
        build_example(&example)?;
    }

    println!(
        "{}",
        "✓ All Unix-specific examples built successfully!".bright_green().bold()
    );
    Ok(())
}

fn build_cross_platform_examples() -> Result<()> {
    println!("{}", "Building cross-platform examples...".bright_blue().bold());

    let examples = get_cross_platform_examples()?;

    if examples.is_empty() {
        println!("{}", "⚠ No cross-platform examples found".bright_yellow());
        return Ok(());
    }

    for example in examples {
        build_example(&example)?;
    }

    println!(
        "{}",
        "✓ All cross-platform examples built successfully!"
            .bright_green()
            .bold()
    );
    Ok(())
}

fn build_all_examples() -> Result<()> {
    println!("{}", "Building all examples...".bright_blue().bold());

    build_nostd_examples()?;
    build_unix_examples()?;

    println!("{}", "✓ All examples built successfully!".bright_green().bold());
    Ok(())
}

fn build_main_crate(all_features: bool, no_default_features: bool) -> Result<()> {
    let mut message = "Building main sntpc crate".to_string();

    if all_features {
        message.push_str(" (with all features)");
    } else if no_default_features {
        message.push_str(" (with no default features)");
    }

    message.push_str("...");
    println!("{}", message.bright_blue().bold());

    let mut command = Command::new("cargo");
    command.args(["build", "--manifest-path", "sntpc/Cargo.toml"]);

    if all_features && no_default_features {
        eprintln!(
            "{}",
            "✗ Cannot specify both --all-features and --no-default-features"
                .bright_red()
                .bold()
        );
        anyhow::bail!("Conflicting feature flags");
    }

    if all_features {
        command.arg("--all-features");
    } else if no_default_features {
        command.arg("--no-default-features");
    }

    let status = command
        .status()
        .context("Failed to execute cargo build for main crate")?;

    if !status.success() {
        eprintln!("{}", "✗ Failed to build main crate".bright_red().bold());
        anyhow::bail!("Build failed");
    }

    println!("{}", "✓ Main sntpc crate built successfully!".bright_green().bold());
    Ok(())
}

fn run_tests() -> Result<()> {
    println!("{}", "Running tests for main sntpc crate...".bright_blue().bold());

    let status = Command::new("cargo")
        .args(["test", "--manifest-path", "sntpc/Cargo.toml"])
        .status()
        .context("Failed to execute cargo test")?;

    if !status.success() {
        eprintln!("{}", "✗ Tests failed".bright_red().bold());
        anyhow::bail!("Tests failed");
    }

    println!("{}", "✓ All tests passed!".bright_green().bold());
    Ok(())
}

fn check_all() -> Result<()> {
    println!("{}", "Checking main crate and all examples...".bright_blue().bold());

    // Check main crate
    check_crate("sntpc", "Main crate")?;

    // Check all examples
    let examples = get_all_examples()?;

    for example in examples {
        let example_path = format!("examples/{example}");
        check_crate(&example_path, &format!("Example: {example}"))?;
    }

    println!("{}", "✓ All checks passed!".bright_green().bold());
    Ok(())
}

fn clean_all() -> Result<()> {
    println!("{}", "Cleaning all build artifacts...".bright_blue().bold());

    // Clean main crate
    Command::new("cargo")
        .args(["clean", "--manifest-path", "sntpc/Cargo.toml"])
        .output()?;

    // Clean all examples
    let examples = get_all_examples()?;

    for example in examples {
        let manifest_path = format!("examples/{example}/Cargo.toml");
        if Path::new(&manifest_path).exists() {
            let _ = Command::new("cargo")
                .args(["clean", "--manifest-path", &manifest_path])
                .output();
        }
    }

    println!("{}", "✓ All build artifacts cleaned!".bright_green().bold());
    Ok(())
}

fn run_clippy() -> Result<()> {
    println!(
        "{}",
        "Running Clippy with strict linting on all code...".bright_blue().bold()
    );

    // Run clippy on the main sntpc crate with all features
    println!("  {} Main sntpc crate (all features)", "Clippy".bright_blue());
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
        eprintln!(
            "{}",
            "✗ Clippy found issues in main crate (all features)".bright_red().bold()
        );
        anyhow::bail!("Clippy found issues in main crate");
    }

    println!("  {} Main sntpc crate (all features)", "✓".bright_green());

    // Run clippy on the main sntpc crate with no default features
    println!("  {} Main sntpc crate (no default features)", "Clippy".bright_blue());
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
        eprintln!(
            "{}",
            "✗ Clippy found issues in main crate (no default features)"
                .bright_red()
                .bold()
        );
        anyhow::bail!("Clippy found issues in main crate");
    }

    println!("  {} Main sntpc crate (no default features)", "✓".bright_green());

    // Run clippy on all examples
    let all_examples = get_all_examples()?;
    let nostd_examples = get_nostd_examples()?;

    for example in all_examples {
        let is_nostd = nostd_examples.contains(&example);
        clippy_run(&example, is_nostd)?;
    }

    println!("{}", "✓ All Clippy checks passed!".bright_green().bold());
    Ok(())
}

fn check_formatting() -> Result<()> {
    println!(
        "{}",
        "Checking code formatting for main crate and all examples..."
            .bright_blue()
            .bold()
    );

    // Check main crate
    check_format_crate("sntpc", "Main crate")?;

    // Check all examples
    let examples = get_all_examples()?;

    for example in examples {
        let example_path = format!("examples/{example}");
        check_format_crate(&example_path, &format!("Example: {example}"))?;
    }

    println!("{}", "✓ All formatting checks passed!".bright_green().bold());
    Ok(())
}

fn fix_formatting() -> Result<()> {
    println!(
        "{}",
        "Fixing code formatting for main crate and all examples..."
            .bright_blue()
            .bold()
    );

    // Fix main crate
    fix_format_crate("sntpc", "Main crate")?;

    // Fix all examples
    let examples = get_all_examples()?;

    for example in examples {
        let example_path = format!("examples/{example}");
        fix_format_crate(&example_path, &format!("Example: {example}"))?;
    }

    println!("{}", "✓ All formatting issues fixed!".bright_green().bold());
    Ok(())
}

fn clippy_run(example_name: &str, no_std: bool) -> Result<()> {
    let example_dir = format!("examples/{example_name}");

    if !Path::new(&example_dir).exists() {
        println!(
            "{}",
            format!("⚠ Skipping {example_name}: directory not found").bright_yellow()
        );
        return Ok(());
    }

    let feature_msg = if no_std { " (no-std)" } else { "" };
    println!("  {} {example_name}{feature_msg}", "Clippy".bright_blue());

    let mut cmd = Command::new("cargo");
    cmd.args(["clippy"]).current_dir(&example_dir);

    if no_std {
        cmd.args(["--no-default-features", "--profile", "no-std"]);
    }

    cmd.args(["--", "-D", "clippy::all", "-D", "clippy::pedantic"]);

    let status = cmd
        .status()
        .context(format!("Failed to execute cargo clippy for {example_name}"))?;

    if !status.success() {
        eprintln!("{}", format!("✗ Clippy found issues in {example_name}").bright_red());
        anyhow::bail!("Clippy found issues in {example_name}");
    }

    println!("  {} {}{}", "✓".bright_green(), example_name, feature_msg);
    Ok(())
}

fn build_example(example_name: &str) -> Result<()> {
    let example_dir = format!("examples/{example_name}");

    if !Path::new(&example_dir).exists() {
        println!(
            "{}",
            format!("⚠ Skipping {example_name}: directory not found").bright_yellow()
        );
        return Ok(());
    }

    println!("  {} {}", "Building".bright_blue(), example_name);

    let mut cmd = Command::new("cargo");
    cmd.args(["build"]).current_dir(&example_dir);

    let status = cmd
        .status()
        .context(format!("Failed to execute cargo build for {example_name}"))?;

    if !status.success() {
        eprintln!("{}", format!("✗ Failed to build {example_name}").bright_red());
        anyhow::bail!("Build failed for {example_name}");
    }

    println!("  {} {}", "✓".bright_green(), example_name);
    Ok(())
}

fn check_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        println!("{}", format!("⚠ Skipping {name}: directory not found").bright_yellow());
        return Ok(());
    }

    println!("  {} {}", "Checking".bright_blue(), name);

    let status = Command::new("cargo")
        .args(["check"])
        .current_dir(path)
        .status()
        .context(format!("Failed to execute cargo check for {name}"))?;

    if !status.success() {
        eprintln!("{}", format!("✗ Check failed for {name}").bright_red());
        anyhow::bail!("Check failed for {name}");
    }

    println!("  {} {name}", "✓".bright_green());
    Ok(())
}

fn check_format_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        println!("{}", format!("⚠ Skipping {name}: directory not found").bright_yellow());
        return Ok(());
    }

    println!("  {} {}", "Checking format".bright_blue(), name);

    let status = Command::new("cargo")
        .args(["fmt", "--check"])
        .current_dir(path)
        .status()
        .context(format!("Failed to execute cargo fmt for {name}"))?;

    if !status.success() {
        eprintln!("{}", format!("✗ Format check failed for {name}").bright_red());
        anyhow::bail!("Format check failed for {name}");
    }

    println!("  {} {name}", "✓".bright_green());
    Ok(())
}

fn fix_format_crate(path: &str, name: &str) -> Result<()> {
    if !Path::new(path).exists() {
        println!("{}", format!("⚠ Skipping {name}: directory not found").bright_yellow());
        return Ok(());
    }

    println!("  {} {}", "Fixing format".bright_blue(), name);

    let status = Command::new("cargo")
        .args(["fmt", "--all"])
        .current_dir(path)
        .status()
        .context(format!("Failed to execute cargo fmt for {name}"))?;

    if !status.success() {
        eprintln!("{}", format!("✗ Format fix failed for {name}").bright_red());
        anyhow::bail!("Format fix failed for {name}");
    }

    println!("  {} {name}", "✓".bright_green());
    Ok(())
}
