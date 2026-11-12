//! `cargo xtask wasm` — does everything below the apps still build for a browser?
//!
//! The rule: **every library crate must compile for
//! `wasm32-unknown-unknown`.** The web build is a first-class target, not an
//! afterthought, and the way that stays true is by checking it in CI rather than
//! by hoping.
//!
//! # What this catches, precisely
//!
//! The boundary here is narrower than it looks, and getting that wrong is worse
//! than having no check, so it was measured rather than assumed.
//!
//! **Caught:**
//!
//! - a dependency that does not build for `wasm32-unknown-unknown` at all. This
//!   is the gate's real value and it is a big one: a transitive dependency that
//!   turns out to be native-only is caught here rather than by whoever opens the
//!   page first.
//! - a `#[cfg(target_arch = "wasm32")]` branch that does not compile, because
//!   nothing builds it until somebody opens a browser.
//! - a `crate-type`, feature or linker setting that only resolves natively.
//!
//! **Not caught — and this surprises people:**
//!
//! - `std::fs`, `std::env`, `std::net`, `std::thread`, `std::process` and
//!   `std::time::SystemTime` all **compile** for `wasm32-unknown-unknown`,
//!   including `std::os::unix::fs::symlink_metadata`. `std` provides stubs that
//!   return an error at run time instead of being compiled out. A library crate
//!   can therefore reach for the filesystem, pass this gate, and fail in a
//!   browser.
//!
//! So the rule this gate enforces is "everything below the apps still *builds*
//! for a browser", not "everything below the apps still *works* in one". Nothing
//! here substitutes for actually opening the page. See `KNOWN-GAPS.md`.
//!
//! The apps are excluded: the desktop app is a window and the CLI rasterises;
//! neither belongs in a browser. `craftkit-wasm` is *checked*, because it is the
//! one app whose whole job is to be a browser.
//!
//! # There is no bundler here
//!
//! The v1 browser door was `wasm-bindgen`: a separate CLI that turned a
//! `.wasm` into a `.js` and a `_bg.wasm`, and had to be installed, and had to
//! have a version that matched the crate. **The v2 has none of that.** The
//! browser door is a raw `cdylib` that exports four functions by name, the
//! page fetches the `.wasm` itself and instantiates it, and the "bundle" is
//! the tree's own program (`apps/craftkit-wasm/src/bin/bundle.rs`) copying two
//! files into `dist/`.
//!
//! So the one name that matters is the **module's** name, and it has to be the
//! same in three places: the artefact cargo builds, the file the bundler
//! writes, and the path the page fetches. That is what [`BUNDLE_NAME`] is.

use std::path::{Path, PathBuf};

use crate::layers;
use crate::{skipped, step};

/// The target every library crate must build for.
pub const TARGET: &str = "wasm32-unknown-unknown";

/// The **module's** name, without its extension.
///
/// **One constant, because the v2's page fetches the module itself.** There is
/// no `wasm-bindgen` glue in the v2: no `.js` file, no `_bg.wasm`, no CLI to
/// install. The page does `fetch("./craftkit_wasm.wasm")` and hands the bytes
/// to `WebAssembly.instantiate`. So the name that matters is the module's, and
/// it must be the same name in three places:
///
/// - the artefact cargo builds — `target/wasm32-unknown-unknown/release/
///   craftkit_wasm.wasm` (the package's dashes become underscores, because
///   the crate's *lib* target is `craftkit_wasm`);
/// - the file the bundler writes into `dist/` — `craftkit_wasm.wasm`;
/// - the path the page fetches — `./craftkit_wasm.wasm`.
///
/// The v1 file made the same point about a name it got wrong: `wasm-bindgen`
/// does not turn dashes into underscores, it writes whatever name it is
/// handed, and the page imported `craftkit_web.js` while the bundler wrote
/// `craftkit-web.js` — a 404 on every load, behind four green checks. The
/// v2's mistake would be the same shape, so the gate checks all three names
/// against this one constant; see `xtask/src/web.rs`.
pub const BUNDLE_NAME: &str = "craftkit_wasm";

/// The package the browser app is built from.
///
/// **One name, because the gate and the bundle have to agree about it.** The
/// gate checks that this package builds for the browser ✓ and the bundle is
/// made from it ✓; a second spelling would let the two drift ✓ and the drift
/// would be invisible ✓ — the gate would be checking one app and the page
/// would be served another.
pub const WEB_PACKAGE: &str = "craftkit-wasm";

/// The module's file name: `craftkit_wasm.wasm`.
///
/// **Derived, never written twice.** The bundler's output name and the page's
/// `fetch` are both this name, and the gate compares both against it.
pub fn module_file_name() -> String {
    format!("{BUNDLE_NAME}.wasm")
}

/// The path the page fetches, relative to the page: `./craftkit_wasm.wasm`.
///
/// The page and its module have to be served from one directory, so the path
/// is relative to the page and never absolute.
pub fn module_path() -> String {
    format!("./{BUNDLE_NAME}.wasm")
}

/// Where the bundler writes the bundle: `apps/craftkit-wasm/dist`.
///
/// The bundler writes beside the page, into the page's own app directory —
/// see `apps/craftkit-wasm/src/bin/bundle.rs`.
pub fn bundle_dir(root: &Path) -> PathBuf {
    root.join("apps").join(WEB_PACKAGE).join("dist")
}

/// The module cargo builds, where cargo leaves it.
///
/// `target/wasm32-unknown-unknown/release/craftkit_wasm.wasm`.
pub fn module_artefact(root: &Path) -> PathBuf {
    root.join("target").join(TARGET).join("release").join(module_file_name())
}

/// The two files the bundle holds: the module and the page.
///
/// The bundle is the page **and** the module, side by side: a page loads its
/// module with `fetch`, and `fetch` does not read a file off the disk, so the
/// two have to be served from one directory.
pub fn bundle_files() -> Vec<String> {
    vec![module_file_name(), "index.html".to_string()]
}

/// Is the target installed on this machine?
pub fn target_installed() -> bool {
    match crate::command("rustup", &["target", "list", "--installed"], Path::new(".")) {
        Ok(out) if out.ok() => out.stdout.lines().any(|line| line.trim() == TARGET),
        // No rustup is a normal way to have Rust; ask cargo instead.
        _ => crate::command("rustc", &["--print", "target-list"], Path::new("."))
            .map(|out| out.ok() && out.stdout.lines().any(|line| line.trim() == TARGET))
            .unwrap_or(false),
    }
}

/// The gate.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("wasm");
    if !target_installed() {
        skipped(
            "wasm32 check",
            &format!("the {TARGET} target is not installed (rustup target add {TARGET})"),
        );
        return Ok(crate::Outcome::Skipped);
    }
    build(root)?;

    // A compile check is not a browser build, and nothing above can say so on
    // its own: it writes nothing, so the page's module is not there until
    // somebody runs `cargo xtask bundle`. When that is the case, the gate says
    // so — a green gate over a page that 404s is the failure this tree has
    // already paid for once.
    if let Some(note) = post_build_note(bundle_blocker(root).as_deref()) {
        println!("   note: {note}");
    }
    Ok(crate::Outcome::Ran)
}

/// What `run` says once the compile check has passed and the browser bundle is
/// still out of reach.
///
/// Split out and pure so the sentence can be tested without a toolchain on the
/// machine: a test that depended on the installed target would pass in one
/// checkout and fail in another, which is worse than no test. `None` when
/// there is nothing to say, so a tree whose bundle is already there is not
/// told it is not.
///
/// **The v2 has no `wasm-bindgen-cli` to install**, and this note must never
/// tell a reader to install one: there is none in the tree, and the bundle is
/// built by the tree's own program.
pub fn post_build_note(blocker: Option<&str>) -> Option<String> {
    blocker.map(|reason| {
        format!(
            "that is a compile check. It writes nothing. {reason}\n         \
             `cargo xtask wasm` is green; the page's module does not exist yet. \
             `cargo xtask bundle` is the command that writes it."
        )
    })
}

/// Why the browser bundle is not there right now, if it is not.
///
/// Returned as a sentence so `run` can print it as a note, which is the only
/// honest outcome when the thing that is missing is the thing itself.
///
/// The v2's bundle needs no tool that has to be installed: the bundler is
/// `cargo run -p craftkit-wasm --bin bundle`, so the only thing that can stop
/// it is the target not being installed, or the bundle not having been
/// written yet.
fn bundle_blocker(root: &Path) -> Option<String> {
    if !target_installed() {
        return Some(format!(
            "the {TARGET} target is not installed, so the module cannot be built \
             (rustup target add {TARGET})."
        ));
    }
    if !bundle_is_written(root) {
        return Some(format!(
            "the bundle has not been written yet, and `cargo xtask bundle` is the command that \
             writes {} and the page beside it.",
            module_file_name()
        ));
    }
    None
}

/// Whether the bundle is written: the module and the page, side by side.
///
/// Both files, because a bundle with the page and no module is a page that
/// cannot start, and one with the module and no page is a module nobody can
/// reach.
pub fn bundle_is_written(root: &Path) -> bool {
    let dist = bundle_dir(root);
    bundle_files().iter().all(|name| dist.join(name).is_file())
}

/// Build the browser bundle: the page the repository is actually deployed as.
///
/// The v2's bundle is two files — the module cargo built and the page the
/// tree holds — written into `dist/` by the tree's own program. There is no
/// external tool to install and no glue to generate.
///
/// Returns [`crate::Outcome::Skipped`] with a printed reason when the target
/// is not installed: never a pass.
pub fn bundle(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("bundle");
    if !target_installed() {
        skipped(
            "the browser bundle",
            &format!("the {TARGET} target is not installed (rustup target add {TARGET})"),
        );
        return Ok(crate::Outcome::Skipped);
    }
    build_module(root)?;
    let wasm = module_artefact(root);
    if !wasm.is_file() {
        return Err(format!("{} is not there after a successful build", wasm.display()));
    }
    // The bundler takes the module's path and writes the module and the page
    // into `dist/`. The path is relative to the workspace root, because that
    // is where cargo is run from.
    let relative = Path::new("target").join(TARGET).join("release").join(module_file_name());
    let relative = relative.to_string_lossy().to_string();
    crate::cargo(&["run", "-p", WEB_PACKAGE, "--bin", "bundle", "--", &relative], root)?;

    let dist = bundle_dir(root);
    for name in bundle_files() {
        let path = dist.join(&name);
        if !path.is_file() {
            return Err(format!(
                "the bundler did not write {}; the bundle in {} is incomplete",
                path.display(),
                dist.display()
            ));
        }
    }
    println!("   ok: {} and index.html in {}", module_file_name(), dist.display());
    Ok(crate::Outcome::Ran)
}

/// Check every library crate and the browser app, and build the module.
///
/// Separated from [`run`] so `ci` can call it directly, having already asked
/// whether the target is installed.
pub fn build(root: &Path) -> Result<(), String> {
    build_module(root)?;
    layers::check_wasm(root)?;
    println!("   ok: every library crate and {WEB_PACKAGE} build for {TARGET}");
    Ok(())
}

/// Build the browser module: the artefact the page loads.
///
/// The command is the tree's own, the one `apps/craftkit-wasm/src/bin/bundle.rs`
/// documents, and the artefact it leaves is the one the page fetches.
fn build_module(root: &Path) -> Result<(), String> {
    crate::cargo(&["build", "-p", WEB_PACKAGE, "--target", TARGET, "--release"], root)?;
    let wasm = module_artefact(root);
    if !wasm.is_file() {
        return Err(format!("{} is not there after a successful build", wasm.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_the_browser_target() {
        assert_eq!(TARGET, "wasm32-unknown-unknown");
    }

    /// The v2 has no glue: the name that matters is the module's, and it is
    /// the module's because the page fetches the `.wasm` itself.
    #[test]
    fn the_bundle_name_is_the_modules_name() {
        assert_eq!(BUNDLE_NAME, "craftkit_wasm");
        assert_eq!(module_file_name(), "craftkit_wasm.wasm");
        assert_eq!(module_path(), "./craftkit_wasm.wasm");
    }

    /// The package is the app's, and the app is the one that is a browser.
    #[test]
    fn the_web_package_is_the_browser_door() {
        assert_eq!(WEB_PACKAGE, "craftkit-wasm");
    }

    /// The artefact is where the page's module is: `target/<target>/release/`.
    #[test]
    fn the_artefact_is_where_cargo_leaves_the_module() {
        let root = Path::new("/tree");
        assert_eq!(
            module_artefact(root),
            Path::new("/tree/target/wasm32-unknown-unknown/release/craftkit_wasm.wasm")
        );
    }

    /// The bundle is the module and the page, side by side.
    #[test]
    fn the_bundle_is_the_module_and_the_page() {
        assert_eq!(bundle_files(), vec!["craftkit_wasm.wasm".to_string(), "index.html".to_string()]);
    }

    /// The bundler writes into the page's own app directory.
    #[test]
    fn the_bundle_is_written_beside_the_page() {
        assert_eq!(bundle_dir(Path::new("/tree")), Path::new("/tree/apps/craftkit-wasm/dist"));
    }

    /// §13's whole content: `wasm` can be green while the page's module does
    /// not exist. The phase has to say so, and say it only when it is true.
    #[test]
    fn post_build_note_says_so_when_the_browser_build_is_out_of_reach() {
        let blocked = post_build_note(Some("the wasm32 target is not installed"))
            .expect("a missing bundle is worth saying");
        assert!(blocked.contains("compile check"), "{blocked}");
        assert!(blocked.contains("the wasm32 target is not installed"), "{blocked}");
        assert!(blocked.contains("does not exist yet"), "{blocked}");
    }

    /// The other half, as its own test: a tree whose bundle is there must not
    /// be told it is not, or every run carries a warning nobody can act on.
    #[test]
    fn post_build_note_stays_quiet_when_there_is_nothing_to_say() {
        assert!(post_build_note(None).is_none(), "nothing to say must stay quiet");
    }

    /// The blocker is a sentence, not a bool, and the whole point is that the
    /// reason reaches the reader.
    #[test]
    fn post_build_note_carries_the_blockers_own_reason() {
        let reason = "the bundle has not been written yet";
        let note = post_build_note(Some(reason)).expect("note");
        assert!(note.contains(reason), "the reason was dropped: {note}");
    }

    /// The v1 note told the reader to install `wasm-bindgen-cli`. There is no
    /// such tool in the v2, and telling a reader to install it would send
    /// them to install a bundler for a glue format this tree does not have.
    #[test]
    fn the_note_never_mentions_wasm_bindgen() {
        let note = post_build_note(Some("the target is not installed")).expect("note");
        assert!(!note.contains("wasm-bindgen"), "the v1 note came back: {note}");
        assert!(note.contains("cargo xtask bundle"), "the note does not say what to do: {note}");
    }

    /// The gate **says** it checks the browser app, and it has to check the
    /// one the tree holds: the app is exempt from the layering rules — it is
    /// an app — and that has nothing to do with whether it builds for a
    /// browser.
    #[test]
    fn the_wasm_check_covers_every_layered_crate_plus_the_browser_app() {
        let checked = layers::wasm_packages();
        assert!(
            checked.contains(&WEB_PACKAGE),
            "the gate says it checks {WEB_PACKAGE} and does not: {checked:?}"
        );
    }

    #[test]
    fn target_installed_answers_without_panicking() {
        // Whether or not it is installed, this must return rather than fail:
        // `ci` runs it on machines that have never heard of wasm.
        let _ = target_installed();
    }
}
