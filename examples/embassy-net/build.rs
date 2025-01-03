use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if cfg!(target_os = "linux") {
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        // The default linker script can be overriden by setting `LDSCRIPTDIR` and `LDSCRIPT`.
        // If not specified, the binutils default linker script for the target architecture is used.
        let ldscript_dir = env::var("LDSCRIPTDIR").map_or_else(
            |_| Path::new(&manifest_dir).join("ldscripts"),
            PathBuf::from,
        );
        let ldscript = env::var("LDSCRIPT")
            .unwrap_or_else(|_| format!("elf_{target_arch}.x"));

        // Add the linker script to the search path.
        println!("cargo:rustc-link-search={}", ldscript_dir.to_string_lossy());
        println!("cargo:rustc-link-arg=-T{ldscript}");

        // Add the defmt strings section to the elf file.
        println!("cargo:rustc-link-arg=-Tdefmt.x");

        // Enable info level logging for the current crate (if not already set).
        if env::var("DEFMT_LOG").is_err() {
            println!("cargo:rustc-env=DEFMT_LOG=info");
        }
    }
}
