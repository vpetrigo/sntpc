fn main() {
    if cfg!(target_os = "linux") {
        let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

        // Add back the default linker script.
        println!("cargo:rustc-link-search=/usr/lib/{target_arch}-linux-gnu/ldscripts");
        println!("cargo:rustc-link-arg=-Telf_{target_arch}.x");

        // Add the defmt strings section to the elf file.
        println!("cargo:rustc-link-arg=-Tdefmt.x");

        // Enable info level logging for the current crate (if not already set).
        if std::env::var("DEFMT_LOG").is_err() {
            println!("cargo:rustc-env=DEFMT_LOG=info");
        }
    }
}
