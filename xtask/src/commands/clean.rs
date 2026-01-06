use crate::Result;
use crate::utils;
use std::path::Path;

/// Cleans all build artifacts from the main crate and examples.
///
/// This function runs `cargo clean` on the main sntpc crate and all discovered
/// examples to remove build artifacts and free up disk space.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover examples
/// - Cargo clean command execution fails for the main crate
/// - Critical cleanup operations fail (example cleanup failures are ignored)
pub fn clean_all() -> Result<()> {
    utils::print_header("Cleaning all build artifacts...");

    // Clean main crates
    utils::run_cargo_clean("sntpc/Cargo.toml")?;
    utils::run_cargo_clean("sntpc-net-embassy/Cargo.toml")?;
    utils::run_cargo_clean("sntpc-net-std/Cargo.toml")?;
    utils::run_cargo_clean("sntpc-net-tokio/Cargo.toml")?;

    // Clean all examples
    let all_examples = utils::get_all_examples()?;

    for example in all_examples {
        let manifest_path = format!("examples/{example}/Cargo.toml");
        if Path::new(&manifest_path).exists() {
            let _ = utils::run_cargo_clean(&manifest_path);
        }
    }

    utils::print_success("✓ All build artifacts cleaned!");
    Ok(())
}
