//! The application's build script.
//!
//! # What it does
//!
//! **The dependency list is the plugin list.**
//!
//! A plugin crate has to be a dependency of the application — that is what
//! "linked in" means — but the application should not have to *name* it.
//! Rust will drop a dependency that nothing refers to, and a dropped
//! dependency takes its registrations with it, so something has to refer to
//! it.
//!
//! This script reads the application's own manifest, finds the dependencies
//! that are plugins, and writes the references:
//!
//! ```text
//! use <plugin> as _;
//!
//! #[used]
//! pub static CRAFTKIT_PLUGIN_ANCHOR_<PLUGIN>: &[fn() -> usize] =
//!     <plugin>::CRAFTKIT_PLUGIN_ANCHOR;
//! ```
//!
//! The second part is the **anchor**: a relocation to the plugin's items,
//! which a linker cannot drop. It is what keeps the plugin in the program
//! on `wasm32-unknown-unknown`, where a crate that nothing names is
//! dropped whole — measured, and the registry was empty there.
//!
//! So the wiring is **generated from the manifest**, and the manifest is the
//! only place a plugin is written down. Adding a plugin is one line in
//! `Cargo.toml` and nothing anywhere else.
//!
//! # Why a plugin is a dependency and not a path found at run time
//!
//! A plugin could also be a `.so` loaded with `dlopen` while the program
//! runs. That is a different door with different properties — it needs a
//! shared build of the SDK, or an explicit hand-off of the registry — and it
//! is worth having. What this script does is the other door: the plugin is
//! part of the program, so the compiler checks it, the linker keeps it, and
//! its commands are there from the first instruction.
//!
//! # Why the work is in a crate
//!
//! Because more than one application has plugins, and a copy of this in two
//! places is two places for the two copies to disagree. `craftkit-link` is a
//! **build dependency**: it is compiled for the build script, and it is not
//! in the application's own dependency list.

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let manifest = std::path::Path::new(&manifest).join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed=build.rs");

    let out = std::env::var("OUT_DIR").unwrap_or_default();
    if let Err(error) = craftkit_link::generate(&manifest, std::path::Path::new(&out)) {
        panic!("{error}");
    }
}
