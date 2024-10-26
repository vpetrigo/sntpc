fn main() {
    if cfg!(target_os = "linux") {
        println!("cargo::rustc-link-arg=-nostartfiles");
        println!("cargo::rustc-link-arg=-lc");
    }
    println!("cargo::rerun-if-changed=build.rs");
}
