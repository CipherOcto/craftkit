//! The build script: the dependency list is the plugin list.
//!
//! The same wiring the studio uses, and the same crate does it, so the two
//! applications cannot disagree about what a plugin is.

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let manifest = std::path::Path::new(&manifest).join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed=build.rs");

    let out = std::env::var("OUT_DIR").unwrap_or_default();
    // The features that are on decide which optional plugins are wired in.
    let features = craftkit_link::enabled_features();
    if let Err(error) = craftkit_link::generate(&manifest, std::path::Path::new(&out), &features) {
        panic!("{error}");
    }
}
