//! The browser door's hand-written files.
//!
//! The v2's browser door is a **page** and a **WebAssembly module**, and the
//! two are joined by four exported functions and two buffers. Everything in
//! that boundary is Rust; the two files that are not are the page itself and
//! the node script that drives the same module without a browser.
//!
//! # What is checked
//!
//! - **The page fetches the module the bundler writes.** The page does
//!   `fetch("./craftkit_wasm.wasm")` and the bundler writes
//!   `craftkit_wasm.wasm` into `dist/`. They are two copies of one name, and
//!   the failure of a mismatch is a 404 on every load — behind a green gate.
//!   The name is compared against **one** constant, `wasm::BUNDLE_NAME`.
//! - **The module exports what the page calls.** The page calls
//!   `craftkit_wasm_request_address`, `craftkit_wasm_run` and
//!   `craftkit_wasm_response_address`, and reads `exports.memory`. If the
//!   module stops exporting one of them, the page breaks at run time and
//!   nothing else in the tree notices.
//! - **The page and the node script call the same names.** They are two
//!   copies of one protocol: `browser-check.mjs` drives the module the way
//!   the page does, without a browser. If they drift, one of the two is
//!   broken, and the drift is invisible until somebody opens the page.
//! - **The page can reach the module's memory.** The page reads
//!   `exports.memory`, and only a `cdylib` gets its memory exported by the
//!   linker. Nothing in the Rust source declares it, so the manifest is what
//!   has to say it.
//! - **The script parses.** With `node --check` when node is installed.
//!   Without node the step is reported as skipped, never as passed.
//!
//! The v1 file checked a canvas element and a service worker. The v2's page
//! has neither: it is a form and an answer, not a canvas, and it has no
//! service worker — so those checks are gone rather than rewritten.

use std::path::Path;

use crate::{command, skipped, step};

/// The page, relative to the workspace root.
pub const INDEX: &str = "apps/craftkit-wasm/index.html";

/// The node script that drives the module without a browser.
pub const BROWSER_CHECK: &str = "apps/craftkit-wasm/browser-check.mjs";

/// The module's own source: where the exports are declared.
pub const MODULE_SOURCE: &str = "apps/craftkit-wasm/src/lib.rs";

/// The bundler: where the module's name is written down.
pub const BUNDLER_SOURCE: &str = "apps/craftkit-wasm/src/bin/bundle.rs";

/// The browser door's manifest: where the `crate-type` is declared.
pub const MANIFEST: &str = "apps/craftkit-wasm/Cargo.toml";

/// Every export the module makes by name.
///
/// These are the four functions of the boundary, and they are declared in
/// `apps/craftkit-wasm/src/lib.rs` as `#[unsafe(no_mangle)] pub extern "C"
/// fn`. The page uses three of them; the fourth says how long the answer is,
/// and the page takes that from `craftkit_wasm_run`'s return value instead.
/// The module exports all four, and the gate holds it to that.
pub const EXPORTS: &[&str] = &[
    "craftkit_wasm_request_address",
    "craftkit_wasm_run",
    "craftkit_wasm_response_address",
    "craftkit_wasm_response_length",
];

/// The exports the page and the node script call.
///
/// The protocol, in the order it is used: make room for the command, run it,
/// find the answer. The length of the answer comes back from `run`, so the
/// page does not need `craftkit_wasm_response_length` — but the module still
/// exports it, and the Rust test in `apps/craftkit-wasm/tests/door.rs` uses
/// it.
pub const CALLED: &[&str] =
    &["craftkit_wasm_request_address", "craftkit_wasm_run", "craftkit_wasm_response_address"];

/// The module's memory, as the page sees it: `exports.memory`.
///
/// Not declared in Rust: the linker exports the memory of a `cdylib`, and
/// nothing else. So the thing that has to say so is the manifest.
pub const MEMORY: &str = "memory";

/// A problem with the browser door's hand-written files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub file: String,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// The four files the checks read, taken as text.
///
/// Taken rather than read, so the rules are testable without a repository on
/// disk.
#[derive(Debug, Clone)]
pub struct Sources<'a> {
    pub page: &'a str,
    pub browser_check: &'a str,
    pub module: &'a str,
    pub bundler: &'a str,
    pub manifest: &'a str,
}

/// The names a script calls on the module's exports.
///
/// Every `exports.<name>` in the script, in the order it appears. The page
/// and the node script both reach the module only this way, so this is the
/// whole of what they ask of it.
pub fn called_exports(script: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = script;
    while let Some(at) = rest.find("exports.") {
        let after = &rest[at + "exports.".len()..];
        let end = after.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(after.len());
        if end > 0 {
            names.push(after[..end].to_string());
        }
        rest = &after[end..];
    }
    names
}

/// The names a script calls that the module does not export.
///
/// A call to a name the module does not have is a `TypeError` at run time,
/// in the browser, on the first keystroke — and nothing else in the tree
/// would say so.
pub fn unknown_exports(script: &str) -> Vec<String> {
    called_exports(script)
        .into_iter()
        .filter(|name| name != MEMORY && !EXPORTS.contains(&name.as_str()))
        .collect()
}

/// Whether the module's source declares the export.
///
/// Looked for as the declaration itself — `pub extern "C" fn <name>` — and
/// not as a substring of the name: a comment that names an export is not an
/// export. So a commented-out declaration does not count, the same trap the
/// v1's service-worker shell had.
pub fn module_exports(source: &str, name: &str) -> bool {
    let declaration = format!("pub extern \"C\" fn {name}(");
    source.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with("//") && trimmed.contains(&declaration)
    })
}

/// The names a script calls, sorted and without repeats.
///
/// Two scripts that call the same names in a different order are calling the
/// same names: the order is the script's business, and the protocol is not
/// about order.
pub fn called_exports_set(script: &str) -> Vec<String> {
    let mut names = called_exports(script);
    names.sort();
    names.dedup();
    names
}

/// Whether the manifest makes the module a `cdylib`.
///
/// Only a `cdylib` gets its memory exported by the linker, and the page
/// reads `exports.memory`. A manifest that lost the `cdylib` would leave the
/// page reading an export that is not there.
pub fn is_a_cdylib(manifest: &str) -> bool {
    manifest.contains("\"cdylib\"")
}

/// Check the browser door's hand-written files.
///
/// The rules, in the order they are reported.
pub fn check(sources: &Sources<'_>) -> Vec<Problem> {
    let mut problems = Vec::new();

    // The name the bundler writes, and the name the page fetches, are two
    // copies of one string. Both are compared against the one constant.
    let module = crate::wasm::module_file_name();
    let fetched = crate::wasm::module_path();
    if !sources.page.contains(&format!("\"{fetched}\"")) {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: format!(
                "the page does not fetch `{fetched}`.\n     \
                 `cargo xtask bundle` writes `{module}` and the page beside it, so this \
                 path and the module's name are the same constant for a reason."
            ),
        });
    }
    if !sources.bundler.contains(&format!("\"{module}\"")) {
        problems.push(Problem {
            file: BUNDLER_SOURCE.to_string(),
            message: format!(
                "the bundler does not write `{module}`.\n     \
                 The page fetches `{fetched}`; a bundle that writes a different file is a \
                 404 on every load."
            ),
        });
    }

    // The module has to export what the page calls.
    for name in EXPORTS {
        if !module_exports(sources.module, name) {
            problems.push(Problem {
                file: MODULE_SOURCE.to_string(),
                message: format!(
                    "the module does not export `{name}`.\n     \
                     The page calls it by name, and a name that is not exported is a \
                     TypeError in the browser on the first keystroke."
                ),
            });
        }
    }

    // The page and the node script are two copies of one protocol. Both must
    // call the exports, and neither may call something the module does not
    // have.
    for (file, script) in [(INDEX, sources.page), (BROWSER_CHECK, sources.browser_check)] {
        for name in CALLED {
            if !script.contains(&format!("exports.{name}")) {
                problems.push(Problem {
                    file: file.to_string(),
                    message: format!(
                        "the script never calls `exports.{name}`.\n     \
                         The protocol is: make room for the command, run it, find the answer."
                    ),
                });
            }
        }
        if !script.contains(&format!("exports.{MEMORY}")) {
            problems.push(Problem {
                file: file.to_string(),
                message: format!(
                    "the script never reads `exports.{MEMORY}`.\n     \
                     The answer comes out of the module's memory, and the module's memory \
                     is an export."
                ),
            });
        }
        for name in unknown_exports(script) {
            problems.push(Problem {
                file: file.to_string(),
                message: format!(
                    "the script calls `exports.{name}`, which the module does not export.\n     \
                     The module exports: {}.",
                    EXPORTS.join(", ")
                ),
            });
        }
    }

    // The two copies of the protocol must agree. If they drift, one of them is
    // broken and the other one is what everybody reads.
    let page_calls = called_exports_set(sources.page);
    let check_calls = called_exports_set(sources.browser_check);
    if page_calls != check_calls {
        problems.push(Problem {
            file: BROWSER_CHECK.to_string(),
            message: format!(
                "the page and the node script do not call the same exports.\n     \
                 The page calls {page_calls:?}; the script calls {check_calls:?}.\n     \
                 They are two copies of one protocol, so a drift means one of the two is \
                 broken."
            ),
        });
    }

    // `exports.memory` is the linker's doing, not the source's, so the
    // manifest is the only place that can say the module is the kind of
    // module that has an exported memory.
    if !is_a_cdylib(sources.manifest) {
        problems.push(Problem {
            file: MANIFEST.to_string(),
            message: format!(
                "the manifest does not declare `cdylib`.\n     \
                 Only a cdylib gets its memory exported by the linker, and the page reads \
                 `exports.{MEMORY}`."
            ),
        });
    }

    problems
}

/// The gate.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("web");
    let page = read(root, INDEX)?;
    let browser_check = read(root, BROWSER_CHECK)?;
    let module = read_rust(root, MODULE_SOURCE)?;
    let bundler = read_rust(root, BUNDLER_SOURCE)?;
    let manifest = read(root, MANIFEST)?;
    let sources = Sources {
        page: &page,
        browser_check: &browser_check,
        module: &module,
        bundler: &bundler,
        manifest: &manifest,
    };

    let problems = check(&sources);
    if !problems.is_empty() {
        let mut message = format!("{} problem(s) in the browser door:\n", problems.len());
        for problem in &problems {
            message.push_str(&format!("   • {problem}\n"));
        }
        return Err(message);
    }
    println!(
        "   ok: the page fetches {}, the module exports the four names, and the page and \
         the node script call the same ones",
        crate::wasm::module_file_name()
    );

    // The page's script is inline in the HTML, so `node --check` does not
    // apply to it — node cannot read HTML, and the module inside it is
    // checked by the browser. The node script *is* a script, so it is the
    // one that is parsed. When there is no parser, this is skipped, not
    // passed.
    match command("node", &["--check", BROWSER_CHECK], root) {
        Ok(out) if out.ok() => {
            println!("   ok: {BROWSER_CHECK} parses");
            Ok(crate::Outcome::Ran)
        }
        Ok(out) => Err(format!("{BROWSER_CHECK} does not parse:\n\n{}", out.all())),
        Err(_) => {
            skipped(BROWSER_CHECK, "node is not installed");
            Ok(crate::Outcome::Skipped)
        }
    }
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    crate::tree::read_text(&path).map_err(|e| format!("could not read {}: {e}", path.display()))
}

/// Read a Rust file for the production code it ships.
///
/// The exports are production code, and the tree's one reader asks which
/// view it wants: this gate wants the code the tree ships.
fn read_rust(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    crate::tree::read_rust(&path, crate::tree::Subject::Production)
        .map_err(|e| format!("could not read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page that fetches the module and calls the protocol.
    ///
    /// Built from the *derived* names rather than spelled out, so a rename of
    /// `wasm::BUNDLE_NAME` cannot quietly make this fixture wrong.
    fn good_page() -> String {
        format!(
            r#"<pre id="answer"></pre>
<form id="prompt"><input id="line" /></form>
<p class="status" id="status">loading…</p>
<script type="module">
const address = exports.craftkit_wasm_request_address(bytes.length);
const length = exports.craftkit_wasm_run();
const where_ = exports.craftkit_wasm_response_address();
const view = new Uint8Array(exports.memory.buffer);
const module = await fetch("./{module}");
</script>"#,
            module = crate::wasm::module_file_name()
        )
    }

    /// The node script, doing what the page does.
    fn good_browser_check() -> String {
        format!(
            r#"const address = exports.craftkit_wasm_request_address(encoded.length);
const length = exports.craftkit_wasm_run();
const where_ = exports.craftkit_wasm_response_address();
const view = new Uint8Array(exports.memory.buffer);
const module = await readFile("{module}");
"#,
            module = crate::wasm::module_file_name()
        )
    }

    /// The module's source, exporting the four names.
    fn good_module() -> String {
        EXPORTS
            .iter()
            .map(|name| format!("pub extern \"C\" fn {name}(length: u32) -> u32 {{ 0 }}"))
            .collect::<Vec<String>>()
            .join("\n")
    }

    /// The bundler, writing the module's name.
    fn good_bundler() -> String {
        format!("let out = directory.join(\"{}\");", crate::wasm::module_file_name())
    }

    /// The manifest, making the module a cdylib.
    fn good_manifest() -> String {
        "crate-type = [\"cdylib\", \"rlib\"]".to_string()
    }

    /// The five files, as owned strings, so a test can change one of them
    /// and ask again.
    struct Fixtures {
        page: String,
        browser_check: String,
        module: String,
        bundler: String,
        manifest: String,
    }

    impl Fixtures {
        fn good() -> Fixtures {
            Fixtures {
                page: good_page(),
                browser_check: good_browser_check(),
                module: good_module(),
                bundler: good_bundler(),
                manifest: good_manifest(),
            }
        }

        fn sources(&self) -> Sources<'_> {
            Sources {
                page: &self.page,
                browser_check: &self.browser_check,
                module: &self.module,
                bundler: &self.bundler,
                manifest: &self.manifest,
            }
        }
    }

    #[test]
    fn a_good_set_of_files_produces_no_problems() {
        let fixtures = Fixtures::good();
        let problems = check(&fixtures.sources());
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// The failure a real browser found in the v1: the page imported a file
    /// the bundler never wrote, so every load was a 404 with every compile
    /// check green.
    #[test]
    fn a_page_that_fetches_another_module_is_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.page = fixtures.page.replace(&crate::wasm::module_file_name(), "craftkit.wasm");
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("does not fetch")), "{problems:?}");
    }

    /// The other copy of the name: the bundler writes the file the page
    /// fetches, and if it stops doing so the page 404s.
    #[test]
    fn a_bundler_that_writes_another_module_is_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.bundler = fixtures.bundler.replace(&crate::wasm::module_file_name(), "craftkit.wasm");
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("does not write")), "{problems:?}");
    }

    #[test]
    fn a_module_that_stops_exporting_a_name_is_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.module = fixtures.module.replace(
            "pub extern \"C\" fn craftkit_wasm_response_address(",
            "pub extern \"C\" fn craftkit_wasm_response_where(",
        );
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("does not export")), "{problems:?}");
    }

    /// A name in a comment is not an export.
    #[test]
    fn an_export_named_only_in_a_comment_is_not_an_export() {
        let source = "// pub extern \"C\" fn craftkit_wasm_run(";
        assert!(!module_exports(source, "craftkit_wasm_run"));
    }

    #[test]
    fn a_page_that_calls_an_export_the_module_does_not_have_is_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.page =
            fixtures.page.replace("exports.craftkit_wasm_run()", "exports.craftkit_wasm_run_it()");
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("does not export")), "{problems:?}");
    }

    /// The two copies of the protocol: if they drift, one of them is broken.
    #[test]
    fn a_page_and_a_script_that_call_different_exports_are_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.browser_check = fixtures.browser_check.replace("exports.craftkit_wasm_run()", "");
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("do not call the same exports")), "{problems:?}");
    }

    /// `exports.memory` is the linker's doing, so the manifest is the only
    /// place that can say the module has one.
    #[test]
    fn a_manifest_without_a_cdylib_is_caught() {
        let mut fixtures = Fixtures::good();
        fixtures.manifest = fixtures.manifest.replace("\"cdylib\"", "\"rlib\"");
        let problems = check(&fixtures.sources());
        assert!(problems.iter().any(|p| p.message.contains("cdylib")), "{problems:?}");
    }

    #[test]
    fn the_called_names_are_read_out_of_the_script() {
        let names = called_exports("exports.craftkit_wasm_run(); exports.memory.buffer");
        assert_eq!(names, vec!["craftkit_wasm_run".to_string(), "memory".to_string()]);
    }

    #[test]
    fn a_manifest_without_a_cdylib_is_not_a_cdylib() {
        assert!(is_a_cdylib("crate-type = [\"cdylib\", \"rlib\"]"));
        assert!(!is_a_cdylib("crate-type = [\"rlib\"]"));
    }

    /// The files in the tree, as they are.
    #[test]
    fn the_repository_browser_door_passes() {
        let root = crate::root().expect("root");
        let page = read(&root, INDEX).expect("index.html");
        let browser_check = read(&root, BROWSER_CHECK).expect("browser-check.mjs");
        let module = read_rust(&root, MODULE_SOURCE).expect("lib.rs");
        let bundler = read_rust(&root, BUNDLER_SOURCE).expect("bundle.rs");
        let manifest = read(&root, MANIFEST).expect("Cargo.toml");
        let sources = Sources {
            page: &page,
            browser_check: &browser_check,
            module: &module,
            bundler: &bundler,
            manifest: &manifest,
        };
        let problems = check(&sources);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// The page and the node script in the tree call the same names, and the
    /// names are the module's.
    #[test]
    fn the_tree_s_page_and_script_agree_with_the_module() {
        let root = crate::root().expect("root");
        let page = read(&root, INDEX).expect("index.html");
        let script = read(&root, BROWSER_CHECK).expect("browser-check.mjs");
        let module = read_rust(&root, MODULE_SOURCE).expect("lib.rs");
        assert_eq!(called_exports_set(&page), called_exports_set(&script));
        for name in EXPORTS {
            assert!(module_exports(&module, name), "{name} is not exported");
        }
    }
}
