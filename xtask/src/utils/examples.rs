use crate::Result;
use std::fs;
use std::path::Path;

/// Discovers and returns all available example projects.
///
/// This function scans the `examples` directory and returns a list of all
/// subdirectories that contain a `Cargo.toml` file, indicating they are
/// valid Rust projects.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to read the examples directory
/// - Failed to access directory entries or their metadata
/// - I/O errors occur while scanning the filesystem
pub fn get_all_examples() -> Result<Vec<String>> {
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

/// Returns all no-std examples that can run on embedded targets.
///
/// This function filters the complete list of examples to return only those
/// designed for embedded environments without the standard library.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover all examples (propagated from `get_all_examples`)
/// - I/O errors occur while scanning for examples
pub fn get_nostd_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for no-std examples (currently just simple-no-std, but this could be extended)
    Ok(all_examples
        .into_iter()
        .filter(|name| name == "simple-no-std")
        .collect())
}

/// Returns all Unix-specific examples that require Unix system features.
///
/// This function filters the complete list of examples to return only those
/// that use Unix-specific networking or system features (excludes no-std examples).
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover all examples (propagated from `get_all_examples`)
/// - I/O errors occur while scanning for examples
pub fn get_unix_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter out no-std examples for Unix examples
    Ok(all_examples
        .into_iter()
        .filter(|name| name != "simple-no-std")
        .collect())
}

/// Returns all cross-platform examples that work on multiple operating systems.
///
/// This function filters the complete list of examples to return only those
/// designed to work across different platforms without platform-specific dependencies.
///
/// # Errors
///
/// Returns an error if:
/// - Failed to discover all examples (propagated from `get_all_examples`)
/// - I/O errors occur while scanning for examples
pub fn get_cross_platform_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for cross-platform examples (examples that should work on any platform)
    let cross_platform = ["simple-request", "tokio", "timesync"];
    Ok(all_examples
        .into_iter()
        .filter(|name| cross_platform.contains(&name.as_str()))
        .collect())
}
