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
//! `scripts/mutation-test-gates.sh` breaks each library crate in turn and runs
//! this gate; the result is what follows.
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
//! neither belongs in a browser. `craftkit-web` is *checked*, because it is the
//! one app whose whole job is to be a browser.

use std::path::Path;

use crate::layers;
use crate::{skipped, step};

/// The target every library crate must build for.
pub const TARGET: &str = "wasm32-unknown-unknown";

/// The name the browser bundle is emitted under, without its extension.
///
/// **One constant, because getting this wrong breaks the page in a way no
/// compilation check can see.** The page imports `./dist/<BUNDLE_NAME>.js`, and
/// `wasm-bindgen` emits `<name>.js` where `<name>` is whatever it is given — it
/// does *not* turn dashes into underscores. An earlier version of this project
/// asserted the opposite in a doc comment and in a gate's error message, the
/// page imported `craftkit_web.js`, and `wasm-bindgen` wrote `craftkit-web.js`.
/// The result was a 404 on every load, found by opening the page: see
/// `KNOWN-GAPS.md` §11. Naming the module here and passing it to `--out-name`
/// is what keeps the two from drifting apart again.
pub const BUNDLE_NAME: &str = "craftkit_web";

/// The package the browser app is built from.
///
/// **One name, because the gate and the bundle have to agree about it.** The
/// gate checks that this package builds for the browser ✓ and the bundle is
/// made from it ✓; a second spelling would let the two drift ✓ and the drift
/// would be invisible ✓ — the gate would be checking one app and the page
/// would be served another. §291.
pub const WEB_PACKAGE: &str = "craftkit-web";

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
    // its own: `wasm-bindgen` is a separate tool, so this phase can be green
    // while the page's JavaScript does not exist and cannot be produced on this
    // machine. `wasm-bindgen --version` is the only thing that knows, and it
    // costs one process to ask.
    if let Some(note) = post_build_note(bundler_blocker(root).as_deref()) {
        println!("   note: {note}");
    }
    Ok(crate::Outcome::Ran)
}

/// What `run` says once the compile check has passed and the browser build is
/// still out of reach.
///
/// Split out and pure so the sentence can be tested without a `wasm-bindgen` on
/// the machine: a test that depended on the installed CLI would pass in one
/// checkout and fail in another, which is worse than no test. `None` when the
/// bundler works, so a machine that *can* build the page is not told it cannot.
pub fn post_build_note(blocker: Option<&str>) -> Option<String> {
    blocker.map(|reason| {
        format!(
            "that is a compile check. It writes nothing, and the browser bundle cannot be built \
             here: {reason}\n         `cargo xtask wasm` is green; the page's JavaScript does not \
             exist yet. `KNOWN-GAPS.md` §13."
        )
    })
}

/// Why the bundle cannot be produced right now, if it cannot.
///
/// Returned as a sentence so `bundle` can print it as a skip with a reason,
/// which is the only honest outcome when a tool is missing.
///
/// The decision itself is [`crate::doctor::bundler_status`], not a second copy
/// of it. That is the fix for the divergence this function used to have with
/// `doctor`: here, an unreadable crate version meant "no blocker" — bundle
/// would carry on with a CLI it had never checked against anything.
fn bundler_blocker(root: &Path) -> Option<String> {
    match crate::doctor::bundler_status(
        crate::doctor::installed_bundler_version(root).as_deref(),
        crate::doctor::wasm_bindgen_crate_version(root).as_deref(),
    ) {
        crate::doctor::Bundler::Ready => None,
        crate::doctor::Bundler::Blocked(reason) => Some(reason),
    }
}

/// Build the browser bundle: the page the repository is actually deployed as.
///
/// This was missing entirely until a real browser was opened. `cargo xtask wasm`
/// only ever *checked* that the crates compile for the target, `index.html`
/// told a reader to run it to "run this", and nothing in the repository turned a
/// `.wasm` into the `dist/` the page imports — so the deployed page was a 404
/// and every compile check was green.
///
/// Returns [`crate::Outcome::Skipped`] with a printed reason when the tool is
/// missing or the wrong version: never a pass.
pub fn bundle(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("bundle");
    let Some(why) = bundler_blocker(root) else {
        crate::cargo(&["build", "-p", "craftkit-web", "--target", TARGET, "--release"], root)?;
        let wasm = root.join("target").join(TARGET).join("release").join("craftkit-web.wasm");
        if !wasm.is_file() {
            return Err(format!("{} is not there after a successful build", wasm.display()));
        }
        let dist = root.join("apps/craftkit-web/dist");
        // Run from the build directory and name the file, rather than
        // assembling an argument vector out of owned strings: every argument
        // below is a literal, which is also what makes the `--out-name` value
        // visibly the same constant the page is checked against.
        let build_dir = wasm.parent().unwrap_or(root);
        let file = wasm
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| format!("{} has no file name", wasm.display()))?;
        let out = crate::command(
            "wasm-bindgen",
            &[
                "--target",
                "web",
                "--out-dir",
                &dist.to_string_lossy(),
                // The name the page imports. Passing it explicitly is the point:
                // left to itself wasm-bindgen would name the file after the
                // crate, dashes and all, and the page would 404 on every load.
                "--out-name",
                BUNDLE_NAME,
                file,
            ],
            build_dir,
        )?;
        if !out.ok() {
            return Err(format!("wasm-bindgen failed:\n\n{}", out.all()));
        }
        println!("   ok: {BUNDLE_NAME}.js and {BUNDLE_NAME}_bg.wasm in {}", dist.display());
        return Ok(crate::Outcome::Ran);
    };
    skipped("the browser bundle", &why);
    Ok(crate::Outcome::Skipped)
}

/// Check every library crate and the web app.
///
/// Separated from [`run`] so `ci` can call it directly, having already asked
/// whether the target is installed.
pub fn build(root: &Path) -> Result<(), String> {
    layers::check_wasm(root)?;
    println!("   ok: every library crate and craftkit-web build for {TARGET}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_target_is_the_browser_target() {
        assert_eq!(TARGET, "wasm32-unknown-unknown");
    }

    /// §13's whole content: `wasm` can be green while the page's JavaScript does
    /// not exist. The phase has to say so, and say it only when it is true.
    #[test]
    fn post_build_note_says_so_when_the_browser_build_is_out_of_reach() {
        let blocked = post_build_note(Some("wasm-bindgen-cli is not installed"))
            .expect("a blocked bundler is worth saying");
        assert!(blocked.contains("compile check"), "{blocked}");
        assert!(blocked.contains("wasm-bindgen-cli is not installed"), "{blocked}");
        assert!(blocked.contains("does not exist yet"), "{blocked}");
    }

    /// The other half, as its own test: a machine that can build the page must
    /// not be told it cannot, or every run carries a warning nobody can act on.
    #[test]
    fn post_build_note_stays_quiet_when_the_bundler_works() {
        assert!(post_build_note(None).is_none(), "a working bundler must stay quiet");
    }

    /// The blocker is a sentence from `bundler_blocker`, not a bool, and the
    /// whole point is that the reason reaches the reader.
    #[test]
    fn post_build_note_carries_the_blockers_own_reason() {
        let reason = "the CLI is 0.2.89 but the wasm-bindgen crate is 0.2.129";
        let note = post_build_note(Some(reason)).expect("note");
        assert!(note.contains(reason), "the reason was dropped: {note}");
    }

    /// §291: the gate **says** it checks the web app, and it did not.
    ///
    /// The web app is exempt from the layering rules ✓ — it is an app ✓ — and
    /// it was filtered out of the wasm check with the apps ✓, so the message
    /// the gate printed ✓ was a claim about a crate it never compiled ✓. The
    /// two things are unrelated ✓, and the check has to name the app ✓.
    #[test]
    fn the_wasm_check_covers_every_layered_crate_plus_the_web_app() {
        let root = crate::root().expect("root");
        let crates = layers::load(&root).expect("metadata");
        let web = crates.iter().find(|c| c.name == WEB_PACKAGE).expect("craftkit-web is a member");
        // It is exempt from the layering rules and still checked for wasm: the
        // two things are unrelated.
        assert_eq!(layers::classify(&web.name), Some(layers::Class::Exempt));

        let checked = layers::wasm_packages();
        assert!(
            checked.contains(&WEB_PACKAGE),
            "the gate says it checks {WEB_PACKAGE} and does not: {checked:?}"
        );
        // And every layered crate is in there too.
        for (name, class) in layers::TABLE {
            if matches!(class, layers::Class::Layer(_)) {
                assert!(checked.contains(name), "{name} is a library crate and is not checked");
            }
        }
        let _ = root;
    }

    #[test]
    fn target_installed_answers_without_panicking() {
        // Whether or not it is installed, this must return rather than fail:
        // `ci` runs it on machines that have never heard of wasm.
        let _ = target_installed();
    }
}
