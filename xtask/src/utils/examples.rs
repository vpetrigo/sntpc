use crate::Result;
use std::fs;
use std::path::Path;

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

pub fn get_nostd_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for no-std examples (currently just simple-no-std, but this could be extended)
    Ok(all_examples
        .into_iter()
        .filter(|name| name == "simple-no-std")
        .collect())
}

pub fn get_unix_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter out no-std examples for Unix examples
    Ok(all_examples
        .into_iter()
        .filter(|name| name != "simple-no-std")
        .collect())
}

pub fn get_cross_platform_examples() -> Result<Vec<String>> {
    let all_examples = get_all_examples()?;
    // Filter for cross-platform examples (examples that should work on any platform)
    let cross_platform = ["simple-request", "tokio", "timesync"];
    Ok(all_examples
        .into_iter()
        .filter(|name| cross_platform.contains(&name.as_str()))
        .collect())
}
