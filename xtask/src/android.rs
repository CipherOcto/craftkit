//! `cargo xtask android` — the tree still builds for a phone.
//!
//! # Why this is a command and not a line in the workflow
//!
//! §230 wrote the phone's recipe into the tree's documentation, and §85 asked
//! for the next half of it: **a recipe in a document is not a job.** The
//! recipe was right — it was measured on a phone — but nothing checked it, so
//! a change that broke the phone build would have been found by somebody
//! holding a phone, or not at all.
//!
//! The gate is a command for the same reason the policy gate is (§173): the
//! rule and the check that applies it live in the same place, and the workflow
//! runs this command **by name**. `cargo xtask lists` checks that: a command
//! `ci` does not run must be run by name in the workflow.
//!
//! # What this gate is, exactly
//!
//! It builds the Android package for **both** ABIs the phone has — 64-bit
//! and 32-bit — and then checks the one thing the platform does that nothing
//! else checks: **that the built library exports the symbol the loader looks
//! up by name.** `android_main` is that symbol. A library that builds and
//! does not export it is a library the platform will refuse to start, and no
//! Rust compiler says so.
//!
//! # Why it is not a phase of `cargo xtask ci`
//!
//! It needs a tool the tree's other gates do not: **the Android NDK**. A gate
//! that fails for want of a tool gets switched off, so this one has the same
//! three rules the policy gate has (§217):
//!
//! - **The build does not work is a failure.** Not a warning.
//! - **The NDK is not installed is not a pass.** It is a skip, printed as a
//!   skip, naming what would provide it.
//! - **Where the NDK is required, its absence is a failure.** Under CI the
//!   job installs it before calling this, so a machine that silently failed
//!   to install it must not turn the rule into a skip.
//!
//! # What is not checked here
//!
//! **The package does not run.** A build proves that the tree compiles for a
//! phone; it does not prove that the application draws on one. That was
//! measured, on a phone, and it is §236's job.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{Outcome, command, skipped, step};

/// The tool the gate needs.
pub const TOOL: &str = "the Android NDK";

/// How long one build may take.
///
/// Measured on this machine: **25.3 s** for the 32-bit library and the
/// 64-bit one together, from a warm cache, and **29.1 s** for the whole
/// APK. A cold build is minutes. The budget is generous on purpose: a gate
/// that fails because a runner was slow is a gate that gets switched off.
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// The package the gate builds.
pub const PACKAGE: &str = "craftkit-android";

/// The symbol the platform's loader looks up **by name**.
///
/// The whole reason this crate is its own crate and the whole reason the
/// tree's unsafe rule has one named exception (§236).
pub const ENTRY: &str = "android_main";

/// The API level the package declares, and the one the toolchain is asked
/// for.
pub const API_LEVEL: u32 = 21;

/// The ABIs, and the toolchain's compiler for each.
///
/// The compiler's name carries the API level, which is why it is written out
/// rather than looked up: the NDK's directory is
/// `toolchains/llvm/prebuilt/<host>/bin/<triple><api>-clang`, and the
/// `<api>` is the one number in it.
pub const ABIS: &[(&str, &str)] = &[
    ("aarch64-linux-android", "aarch64-linux-android21-clang"),
    ("armv7-linux-androideabi", "armv7a-linux-androideabi21-clang"),
];

/// What the gate concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conclusion {
    /// The tree builds for a phone, and the symbol is there.
    Pass,
    /// The build was not checked, and nothing here required it to be.
    ///
    /// Never a pass. A gate that could not run and a gate that found nothing
    /// wrong must not look the same.
    NotChecked,
    /// The build was not checked, and it had to be: this is an error.
    Required,
    /// The build ran and did not work.
    Failed(String),
}

/// Decide what the gate concludes.
///
/// **Pure, and a function of its own, so that a test can pin it** — the
/// shape the policy gate uses (§173), for the same reason.
pub fn conclude(ndk: bool, required: bool, built: Option<(String, bool)>) -> Conclusion {
    if !ndk {
        return if required { Conclusion::Required } else { Conclusion::NotChecked };
    }
    match built {
        None => Conclusion::Failed("the build did not run".to_string()),
        Some((_abi, true)) => Conclusion::Pass,
        Some((abi, false)) => Conclusion::Failed(format!("{abi}: the library does not export `{ENTRY}`")),
    }
}

/// Does this machine require the NDK?
///
/// Under CI the build is not optional: a job that installs the NDK and then
/// cannot find it has failed to install it, and saying "skipped" would turn
/// a broken job into a green one.
pub fn required() -> bool {
    match std::env::var("CI") {
        Ok(value) => {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        }
        Err(_) => false,
    }
}

/// The NDK on this machine, if there is one.
///
/// **Three places, in this order**: the variable a developer sets, the
/// variable the CI image sets, and the directory the SDK keeps them in. The
/// third one is a directory read, and the newest version in it is the one
/// that is used — an SDK with two NDKs in it has a current one.
pub fn ndk_home() -> Option<PathBuf> {
    for name in ["ANDROID_NDK_HOME", "ANDROID_NDK_LATEST_HOME"] {
        if let Ok(value) = std::env::var(name) {
            let path = PathBuf::from(value);
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    let sdk = std::env::var("ANDROID_HOME").or_else(|_| std::env::var("ANDROID_SDK_ROOT")).ok()?;
    let dir = PathBuf::from(sdk).join("ndk");
    let mut found: Vec<PathBuf> =
        std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    // The newest first: the versions are numbered, and the highest number is
    // the current one.
    found.sort();
    found.pop()
}

/// The environment a build for one ABI needs.
///
/// **A function of its own, so that a test can pin it.** The NDK's compiler
/// is the linker; the C compiler and the archiver are named for the target,
/// because `cc` and `ar` on the path are the host's.
///
/// The compiler's name is **not** the target's name: the 32-bit target is
/// `armv7-linux-androideabi` and the toolchain's compiler for it is
/// `armv7a-linux-androideabi21-clang`. That is why the name is a second
/// column of [`ABIS`] and not something worked out here.
pub fn build_env(ndk: &Path, triple: &str, clang_name: &str) -> Vec<(String, String)> {
    let host = if cfg!(target_os = "macos") { "darwin-x86_64" } else { "linux-x86_64" };
    let bin = ndk.join("toolchains/llvm/prebuilt").join(host).join("bin");
    let clang = bin.join(clang_name);
    let ar = bin.join("llvm-ar");
    let linker_var = format!("CARGO_TARGET_{}_LINKER", triple.to_uppercase().replace('-', "_"));
    let cc_var = format!("CC_{}", triple.replace('-', "_"));
    let ar_var = format!("AR_{}", triple.replace('-', "_"));
    vec![
        (linker_var, clang.to_string_lossy().into_owned()),
        (cc_var, clang.to_string_lossy().into_owned()),
        (ar_var, ar.to_string_lossy().into_owned()),
        ("ANDROID_NDK_HOME".to_string(), ndk.to_string_lossy().into_owned()),
    ]
}

/// The library the build produces.
pub fn library(root: &Path, triple: &str) -> PathBuf {
    root.join("target").join(triple).join("debug").join("libcraftkit_android.so")
}

/// Does a symbol listing say the entry point is **exported**?
///
/// **Pure, and a function of its own, so that a test can pin it.** The
/// listing is what `nm -D --defined-only` prints, and a line of it is
/// `000000000033e5dc T android_main`: the address, the **kind**, and the
/// name.
///
/// The kind is what makes this a real check. `T` is a defined text symbol —
/// **exported**; `U` is undefined, which means the library *wants* the symbol
/// and does not have it; `W` is a weak symbol, which the platform's loader
/// will not take as the entry point. Only `T` counts.
///
/// **The last two fields, and not the second and third.** A defined symbol
/// prints as `000000000033eaec T android_main` — an address, a kind and a
/// name — but an **undefined** one prints as `                 U abort`: **no
/// address at all**, and two fields. Reading the kind out of the second field
/// would read the *name* of an undefined symbol as its kind, and the check
/// would pass for the wrong reason. Measured on the host's `nm`:
/// `nm -D /usr/bin/ls | grep " U "` prints two fields, and
/// `nm -D --defined-only` on the phone's library prints three.
pub fn exports_entry_in(listing: &str) -> bool {
    listing.lines().any(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 {
            return false;
        }
        let name = fields[fields.len() - 1];
        let kind = fields[fields.len() - 2];
        kind == "T" && name == ENTRY
    })
}

/// Does the built library export the symbol the platform looks up?
///
/// **The NDK's own `llvm-nm`**, because it is there with the toolchain, and
/// the host's `nm` because it usually is too. The reading is [`exports_entry_in`]
/// — the parsing is separated so that it can be tested without a phone and
/// without a toolchain.
pub fn exports_entry(ndk: Option<&Path>, library: &Path) -> bool {
    let mut tools: Vec<PathBuf> = Vec::new();
    if let Some(ndk) = ndk {
        let host = if cfg!(target_os = "macos") { "darwin-x86_64" } else { "linux-x86_64" };
        tools.push(ndk.join("toolchains/llvm/prebuilt").join(host).join("bin").join("llvm-nm"));
    }
    tools.push(PathBuf::from("nm"));
    for tool in tools {
        let Ok(out) = command(
            &tool.to_string_lossy(),
            &["-D", "--defined-only", &library.to_string_lossy()],
            Path::new("."),
        ) else {
            continue;
        };
        if !out.ok() {
            continue;
        }
        return exports_entry_in(&out.stdout);
    }
    false
}

/// The gate.
pub fn run(root: &Path, _args: &[String]) -> Result<Outcome, String> {
    step("android");
    let Some(ndk) = ndk_home() else {
        return match conclude(false, required(), None) {
            Conclusion::Required => Err(format!(
                "{TOOL} is required here and is not installed: set ANDROID_NDK_HOME, or install it with `sdkmanager \"ndk;{API_LEVEL}.2.9519653\"`"
            )),
            _ => {
                skipped(
                    "the phone build",
                    &format!(
                        "{TOOL} is not installed (set ANDROID_NDK_HOME, or `sdkmanager \"ndk;{API_LEVEL}.2.9519653\"`)"
                    ),
                );
                Ok(Outcome::Skipped)
            }
        };
    };

    for (triple, clang_name) in ABIS {
        let env = build_env(&ndk, triple, clang_name);
        let args = vec![
            "build".to_string(),
            "-p".to_string(),
            PACKAGE.to_string(),
            "--target".to_string(),
            triple.to_string(),
        ];
        let outcome = crate::command_timed(Path::new("cargo"), &args, root, BUILD_TIMEOUT, &env)?;
        if outcome.timed_out {
            return Err(format!(
                "the tree did not build for {triple} in {} seconds",
                BUILD_TIMEOUT.as_secs()
            ));
        }
        if outcome.code != Some(0) {
            return Err(format!("the tree did not build for {triple}:\n\n{}", outcome.all()));
        }
        let library = library(root, triple);
        if !library.is_file() {
            return Err(format!("{triple}: the build produced no {}", library.display()));
        }
        if !exports_entry(Some(&ndk), &library) {
            return Err(format!(
                "{triple}: {} does not export `{ENTRY}`, and the platform's loader looks that symbol up by name",
                library.display()
            ));
        }
        println!("   ok: {triple} builds, and exports `{ENTRY}`");
    }
    Ok(Outcome::Ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_that_works_and_exports_the_symbol_is_a_pass() {
        assert_eq!(conclude(true, false, Some(("armv7".to_string(), true))), Conclusion::Pass);
    }

    #[test]
    fn a_library_without_the_symbol_is_a_failure_and_not_a_skip() {
        // The failure this catches: the package builds, and the platform
        // cannot start it, and the gate says "skipped" — which reads in a CI
        // log like a machine that had no NDK.
        let conclusion = conclude(true, false, Some(("armv7".to_string(), false)));
        assert!(matches!(conclusion, Conclusion::Failed(_)), "{conclusion:?}");
        assert_ne!(conclusion, Conclusion::NotChecked);
        assert_ne!(conclusion, Conclusion::Pass);
    }

    #[test]
    fn a_machine_without_the_ndk_is_not_a_pass() {
        assert_eq!(conclude(false, false, None), Conclusion::NotChecked);
        assert_ne!(conclude(false, false, None), Conclusion::Pass);
    }

    #[test]
    fn a_machine_that_requires_the_ndk_and_has_none_is_an_error() {
        assert_eq!(conclude(false, true, None), Conclusion::Required);
        assert_ne!(conclude(false, true, None), Conclusion::NotChecked);
        assert_ne!(conclude(false, true, None), Conclusion::Pass);
    }

    #[test]
    fn the_environment_names_the_linker_the_target_uses() {
        let env = build_env(Path::new("/ndk"), "aarch64-linux-android", "aarch64-linux-android21-clang");
        let linker = env
            .iter()
            .find(|(k, _)| k == "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER")
            .map(|(_, v)| v.clone());
        assert_eq!(
            linker,
            Some("/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang".to_string())
        );
    }

    #[test]
    fn the_package_declares_the_api_level_the_toolchain_is_asked_for() {
        // One number, in one place. The gate asks the toolchain for API 21
        // and the package says 21; if the package says 24, the gate would
        // build something the package does not claim to support.
        // The workspace root, not the crate directory: a test's working
        // directory is the crate it is in, and the manifest is one level up
        // and two over.
        let root = crate::root().expect("the workspace root");
        let manifest = crate::tree::read_text(root.join("apps/craftkit-android/Cargo.toml"))
            .expect("the package's manifest");
        assert!(
            manifest.contains(&format!("min_sdk_version = {API_LEVEL}")),
            "the package's min_sdk_version is not {API_LEVEL}:\\n{manifest}"
        );
    }

    #[test]
    fn a_defined_text_symbol_is_an_export() {
        // The real shape: `nm -D --defined-only` on the phone's library.
        let listing = "000000000033e5dc T android_main\n000000000033e5e0 T other\n";
        assert!(exports_entry_in(listing));
    }

    #[test]
    fn an_undefined_symbol_is_not_an_export() {
        // The failure this catches: the library *wants* `android_main` and does
        // not have it, and the gate says it is exported. `U` is undefined, and
        // the platform's loader would not find the entry point.
        //
        // **The real shape has no address**: `nm -D /usr/bin/ls | grep " U "`
        // prints `                 U abort` — two fields. A parser that read
        // the kind out of the second field would read the *name* as the kind.
        let listing = "                 U android_main\n";
        assert!(!exports_entry_in(listing));
    }

    #[test]
    fn a_weak_symbol_is_not_an_export() {
        // `W` is weak: the loader will not take it as the entry point.
        let listing = "000000000033e5dc W android_main\n";
        assert!(!exports_entry_in(listing));
        let listing = "                 W android_main\n";
        assert!(!exports_entry_in(listing));
    }

    #[test]
    fn a_listing_without_the_entry_point_is_not_an_export() {
        let listing = "000000000033e5dc T something_else\n";
        assert!(!exports_entry_in(listing));
        assert!(!exports_entry_in(""));
    }

    #[test]
    fn the_abis_are_the_two_the_phone_has() {
        let triples: Vec<&str> = ABIS.iter().map(|(t, _)| *t).collect();
        assert_eq!(triples, vec!["aarch64-linux-android", "armv7-linux-androideabi"]);
    }
}
