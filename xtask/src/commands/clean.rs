use crate::Result;
use crate::utils;
use std::path::Path;

pub fn clean_all() -> Result<()> {
    utils::print_header("Cleaning all build artifacts...");

    // Clean main crate
    utils::run_cargo_clean("sntpc/Cargo.toml")?;

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
