//! The **bundle**: the page and the module, side by side.
//!
//! A page loads its module with `fetch`, and `fetch` does not read a
//! file off the disk: the two have to be **served**. So the bundle
//! writes them into one directory — `dist/` — and a person serves that
//! directory with whatever they have.
//!
//! The module is built by cargo and the bundle is given its path:
//!
//! ```text
//! cargo build -p craftkit-wasm --target wasm32-unknown-unknown --release
//! cargo run -p craftkit-wasm --bin bundle -- \
//!     target/wasm32-unknown-unknown/release/craftkit_wasm.wasm
//! ```
//!
//! There is no bundler here, and that is the point: the module is the
//! file cargo built, and the page is the file the tree holds.

use std::path::{Path, PathBuf};

fn main() {
    let Some(module) = std::env::args().nth(1) else {
        eprintln!("the bundle needs the module's path: craftkit_wasm_bundle <path to craftkit_wasm.wasm>");
        std::process::exit(1);
    };
    match bundle(Path::new(&module)) {
        Ok(directory) => println!("the bundle is in {}", directory.display()),
        Err(message) => {
            eprintln!("the bundle could not be written: {message}");
            std::process::exit(1);
        }
    }
}

/// Write the page and the module into `dist/`.
fn bundle(module: &Path) -> Result<PathBuf, String> {
    let page = Path::new(env!("CARGO_MANIFEST_DIR")).join("index.html");
    let bytes = std::fs::read(module).map_err(|error| format!("the module could not be read: {error}"))?;
    // The bundle goes **beside the page**, wherever the program was
    // run from: a page and its module have to be served from one
    // directory, and the page's own directory is where it lives.
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("the directory could not be made: {error}"))?;
    let out = directory.join("craftkit_wasm.wasm");
    std::fs::write(&out, bytes).map_err(|error| format!("the module could not be written: {error}"))?;
    let text = std::fs::read(&page).map_err(|error| format!("the page could not be read: {error}"))?;
    std::fs::write(directory.join("index.html"), text)
        .map_err(|error| format!("the page could not be written: {error}"))?;
    Ok(directory.to_path_buf())
}
