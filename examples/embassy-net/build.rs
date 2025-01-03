fn main() {
    if cfg!(target_os = "linux") {
        let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        // The default linker script can be overriden by setting `LDSCRIPTDIR` and `LDSCRIPT`.
        // If not specified, the binutils default linker script for the target architecture is used.
        let ldscript_dir = std::env::var("LDSCRIPTDIR").unwrap_or_else(|_| {
            format!("/usr/lib/{target_arch}-linux-gnu/ldscripts")
        });
        let ldscript = std::env::var("LDSCRIPT")
            .unwrap_or_else(|_| format!("elf_{target_arch}.x"));

        println!("cargo:rustc-link-search={ldscript_dir}");
        println!("cargo:rustc-link-arg=-T{ldscript}");

        // Add the defmt strings section to the elf file.
        println!("cargo:rustc-link-arg=-Tdefmt.x");

        // Enable info level logging for the current crate (if not already set).
        if std::env::var("DEFMT_LOG").is_err() {
            println!("cargo:rustc-env=DEFMT_LOG=info");
        }
    }
}
