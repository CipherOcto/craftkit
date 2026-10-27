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
    let bin = ndk.join("toolchains/llvm/prebuilt").join(host_dir()).join("bin");
    let clang = bin.join(compiler_name(clang_name));
    let ar = bin.join(compiler_name("llvm-ar"));
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

/// The NDK's directory for the host it runs on.
///
/// The NDK is built per host, and the directory says which: `linux-x86_64`,
/// `darwin-x86_64`, `windows-x86_64`. **This was wrong at first**, and the
/// xtask's own test on the macOS and Windows runners said so — the first
/// version of this function knew about macOS and treated everything else as
/// Linux, so on Windows it looked for the toolchain in a directory that does
/// not exist there. The gate runs on the Linux runner, so nothing would have
/// caught it if the tree's tests did not run on the other two. §241.
pub fn host_dir() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "darwin-x86_64"
    } else {
        "linux-x86_64"
    }
}

/// The tool's name on this host: on Windows the wrappers are `.cmd` files.
///
/// **Not measured here**: this machine is Linux, and the NDK's Windows layout
/// is not on it. What is measured is the *test* — the xtask's own tests run
/// on the Windows runner, and the one that checks this says what it expects.
/// If the expectation is wrong, that runner says so, which is the point of
/// running the tree's tests on all three platforms. §241.
pub fn compiler_name(name: &str) -> String {
    if cfg!(target_os = "windows") { format!("{name}.cmd") } else { name.to_string() }
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
    for tool in nm_tools(ndk) {
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

/// The tools that can list the symbols of a library, in the order they are
/// tried.
///
/// **The NDK's own `llvm-nm` first**, because it is there with the
/// toolchain, and the host's `nm` second, because it usually is too.
///
/// The NDK's directory is the **host's own**, from [`host_dir`] — and it was
/// a copy of the rule in this function once, which is how the Windows runner
/// came to look for a Linux toolchain in a Windows NDK. A rule written twice
/// is two chances to forget it, and this one was written twice and forgot
/// once. §245.
fn nm_tools(ndk: Option<&Path>) -> Vec<PathBuf> {
    let mut tools: Vec<PathBuf> = Vec::new();
    if let Some(ndk) = ndk {
        tools.push(ndk.join("toolchains/llvm/prebuilt").join(host_dir()).join("bin").join("llvm-nm"));
    }
    tools.push(PathBuf::from("nm"));
    tools
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
            .map(|(_, v)| v.clone())
            .expect("the linker is named");
        // The host's own directory: the NDK is built per host, and the tool
        // is in the one for the machine it runs on. §241.
        assert!(linker.contains(host_dir()), "{linker} does not name the host");
        // And on Windows the wrapper is a `.cmd`, because the file without
        // the extension is not what Windows runs.
        if cfg!(target_os = "windows") {
            assert!(linker.ends_with(".cmd"), "{linker} is not a command file");
        } else {
            assert!(linker.ends_with("aarch64-linux-android21-clang"), "{linker}");
        }
        // The path is compared as a **path**, not as a spelling: a path on
        // Windows spells its separators with backslashes, and the same path
        // with forward slashes is the same path. §241, §243.
        let linker = normalized(&linker);
        assert!(linker.starts_with("/ndk/toolchains/llvm/prebuilt/"), "{linker}");
    }

    /// A path, with its separators spelled the same way on every machine.
    ///
    /// Windows spells a separator with a backslash and Linux with a
    /// forward slash, and **both spellings are the same path**. A check
    /// that compares the spelling fails on one machine and passes on the
    /// other, and the machine it fails on is the one that was never asked.
    ///
    /// The rule is a function of a string, so a test can ask it about a
    /// path from the other platform on this one — which is how the Windows
    /// runner's failure became a check that runs everywhere. §243.
    fn normalized(path: &str) -> String {
        path.replace('\\', "/")
    }

    /// The tool that lists the symbols is the NDK's own, in the host's own
    /// directory.
    ///
    /// §245: the host directory was written **twice** — once in the rule and
    /// once in the code that needed it — and the second copy was the one
    /// that forgot Windows. The test asks the two for their answer, so a
    /// third copy cannot appear without this failing.
    #[test]
    fn the_symbol_lister_is_the_ndks_own_in_the_hosts_own_directory() {
        let tools = nm_tools(Some(Path::new("/ndk")));
        let first = tools.first().expect("the NDK's own tool comes first");
        let text = first.to_string_lossy();
        assert!(text.contains("llvm-nm"), "{text}");
        assert!(text.contains(host_dir()), "{text} does not name the host");
        // And the host's own `nm` is the fallback, because it usually is
        // there.
        assert!(tools.iter().any(|tool| tool == &PathBuf::from("nm")), "{tools:?}");
    }

    /// A path from another platform is compared as the same path.
    ///
    /// The check that failed on the Windows runner was a comparison of a
    /// path against a spelling, and this is the same comparison with the
    /// spelling taken out — so it runs on the machine that writes it.
    #[test]
    fn a_path_is_compared_as_a_path_and_not_as_a_spelling() {
        // The shape the tree's own test uses: a root-relative path, with no
        // drive letter. Windows spells its separators with backslashes, and
        // the same path with forward slashes is the same path.
        let windows = r"\ndk\toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android21-clang.cmd";
        assert!(normalized(windows).starts_with("/ndk/toolchains/llvm/prebuilt/"), "{windows}");
        // And the same path, spelled the other way, is the same path.
        assert_eq!(
            normalized(windows),
            normalized("/ndk/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android21-clang.cmd")
        );
        // A drive letter is a prefix of its own, and the tree's paths are
        // root-relative — so the comparison is about the path, not about
        // where the machine keeps its drives.
        let with_drive = format!("C:{windows}");
        assert!(normalized(&with_drive).contains("/ndk/toolchains/llvm/prebuilt/"), "{with_drive}");
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
