use crate::Result;
use glob;
use std::fs;
use std::path::Path;
use toml;

/// Retrieves a list of all crates defined in the workspace's `Cargo.toml`.
///
/// This function reads the `Cargo.toml` file located in the root directory of the project
/// and parses the `workspace.members` section to determine the crates included in the workspace.
/// It also supports glob patterns in the `members` definition and expands them to include
/// all matching directory paths.
///
/// # Returns
/// - `Ok(Vec<String>)` containing the sorted list of crate names in the workspace.
/// - `Err(anyhow::Error)` if there is an issue reading the `Cargo.toml`, parsing its contents, or
///   resolving glob patterns.
///
/// # Errors
/// This function may return an error in the following scenarios:
/// - The `Cargo.toml` file is missing or cannot be read.
/// - The `workspace.members` field is missing or improperly formatted.
/// - Errors occur while expanding glob patterns in the `workspace.members` field.
pub fn get_workspace_crates() -> Result<Vec<String>> {
    let root_dir = Path::new(".");
    let cargo_toml_path = root_dir.join("Cargo.toml");

    let content = fs::read_to_string(cargo_toml_path)?;
    let workspace: toml::Value = toml::from_str(&content)?;

    let members = workspace
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse workspace members from Cargo.toml"))?;

    let mut crates = Vec::new();
    for member in members {
        if let Some(member_str) = member.as_str() {
            if member_str.contains('*') {
                for entry in glob::glob(member_str)?.flatten() {
                    if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
                        crates.push(name.to_string());
                    }
                }
            } else {
                crates.push(member_str.to_string());
            }
        }
    }

    crates.sort();
    Ok(crates)
}
