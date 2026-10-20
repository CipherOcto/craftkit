//! The browser build's two hand-written files.
//!
//! Everything else in the web build is Rust. `index.html` and `sw.js` are
//! JavaScript because browsers only accept JavaScript in those two places, and
//! they are the entire reason the web build needs a gate of its own: a service
//! worker that caches the wrong thing is invisible until a user is offline.
//!
//! # What is checked
//!
//! - **The canvas id matches.** `index.html` and `CANVAS_ID` in Rust are two
//!   copies of one string, and the failure mode of a mismatch is a blank page.
//! - **The bundle path matches the crate name.** `craftkit-web` produces
//!   `craftkit_web.js`. A rename that misses this file is a broken deploy.
//! - **The worker precaches the bundle.** The page and the worker each hold a
//!   copy of the bundle's name, and for a long time only the page's copy was
//!   checked — so the worker could precache a file that does not exist while
//!   this gate said the two agreed. §67.
//! - **The service worker is safe.** It must not answer a non-GET, must not
//!   cache another origin, and must not cache a non-200. Those three rules are
//!   what keep a convenience feature from becoming a data leak.
//! - **The scripts parse.** With `node --check` when node is installed. Without
//!   node the step is reported as skipped, never as passed.

use std::path::Path;

use crate::{command, skipped, step};

/// The hand-written files, relative to the web app.
pub const INDEX: &str = "apps/craftkit-web/index.html";
pub const SERVICE_WORKER: &str = "apps/craftkit-web/sw.js";

/// The element id `eframe` draws into. Kept in step with
/// `apps/craftkit-web/src/stash.rs`.
pub const CANVAS_ID: &str = "craftkit-canvas";

/// The generated bundle the page imports.
/// The path the page imports, derived from the name the bundler is given.
///
/// Not a literal. An earlier version spelled it out here *and* explained it
/// with a claim that was false: "craftkit-web produces `craftkit_web.js` (the
/// dashes become underscores)". `wasm-bindgen` does not do that — it writes
/// whatever name it is handed, and left to itself it uses the crate's, dashes
/// and all. So the page imported `craftkit_web.js`, the bundler wrote
/// `craftkit-web.js`, and every compile check in this project was green over a
/// page that 404s on every load. Found by opening it in a browser; see
/// `KNOWN-GAPS.md` §11.
///
/// **Derived, not written twice.** An intermediate version of this fix kept a
/// second literal and had the check read *that* one, so renaming
/// `wasm::BUNDLE_NAME` changed nothing the gate could see — which the mutation
/// "the bundler is told to emit a name the page does not import" reported as
/// MISSED on its first run. One constant, or the claim that there is one is
/// another thing written down and not checked.
pub fn bundle_path() -> String {
    format!("./dist/{}.js", crate::wasm::BUNDLE_NAME)
}

/// The path the bundler writes the wasm module to.
///
/// The bundle is two files: the JavaScript module the page imports and the
/// wasm behind it. `wasm-bindgen --out-name` writes both, and the second one
/// is named after the first. §67.
pub fn wasm_path() -> String {
    format!("./dist/{}_bg.wasm", crate::wasm::BUNDLE_NAME)
}

/// The paths the service worker precaches.
///
/// Parsed out of the `SHELL` array rather than searched for in the file: a
/// substring search over the whole worker stays true when the bundle's name
/// survives in a comment, which is the same trap
/// [`canvas_element_is_correctly_identified`] exists to avoid — and a check
/// that a comment can satisfy is not a check.
///
/// A line that is only a comment is not an entry, so a commented-out path does
/// not count either.
pub fn worker_shell(worker: &str) -> Vec<String> {
    const OPEN: &str = "const SHELL = [";
    let Some(start) = worker.find(OPEN) else {
        return Vec::new();
    };
    let rest = &worker[start + OPEN.len()..];
    let Some(end) = rest.find("];") else {
        return Vec::new();
    };
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter_map(|line| {
            let quote = line.find('"')?;
            let after = &line[quote + 1..];
            let close = after.find('"')?;
            Some(after[..close].to_string())
        })
        .collect()
}

/// Whether the worker precaches the bundle the page imports.
///
/// Both files are needed: the JavaScript module the page imports, and the wasm
/// module behind it. Missing either means the page is cached and the
/// application is not — the page loads from the network, the worker has
/// nothing to fall back on, and the user who is offline gets a page that
/// cannot start.
pub fn worker_caches_the_bundle(worker: &str) -> bool {
    let shell = worker_shell(worker);
    shell.iter().any(|entry| entry == &bundle_path()) && shell.iter().any(|entry| entry == &wasm_path())
}

/// A rule the service worker must not break.
pub struct Rule {
    pub what: &'static str,
    pub why: &'static str,
    /// The text that must appear, with the rule stated positively.
    pub needle: &'static str,
    /// Text that would mean the rule has been broken.
    pub forbidden: &'static [&'static str],
}

/// The service worker's rules, in the order they are reported.
pub const RULES: &[Rule] = &[
    Rule {
        what: "only GET is answered",
        why: "answering a non-GET from a cache turns a bug into a data bug",
        needle: "request.method !== \"GET\"",
        forbidden: &[],
    },
    Rule {
        what: "only same-origin is answered",
        why: "a service worker must never serve another origin's response",
        needle: "url.origin !== self.location.origin",
        forbidden: &[],
    },
    Rule {
        what: "only a 200 is stored",
        why: "an opaque or redirected response is not ours to keep",
        needle: "response.status === 200",
        forbidden: &[],
    },
    Rule {
        what: "the network is tried first",
        why: "a stale cached bundle is worse than a slow one",
        needle: "await fetch(request)",
        forbidden: &["cache.match(request)\n        return"],
    },
];

/// A problem with the web build's hand-written files.
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

/// Whether the page carries an element whose **id attribute** is [`CANVAS_ID`].
///
/// Not a substring search. `craftkit-canvas` also appears in the stylesheet and in
/// a comment, so `contains(CANVAS_ID)` stays true after someone renames the
/// element — which `scripts/mutation-test-gates.sh` demonstrated, and which
/// this function exists because of. A gate that cannot see the one edit it was
/// written for is worse than no gate, because it teaches the reader to trust it.
pub fn canvas_element_is_correctly_identified(html: &str) -> bool {
    html.lines()
        .map(str::trim)
        .filter_map(|line| {
            if !line.starts_with("<canvas") {
                return None;
            }
            // Cut at the first `>` so `<canvas id="x"></canvas>` yields the tag
            // and not the whole line — the closing tag is a neighbour, not an
            // attribute value.
            let end = line.find('>')?;
            Some(&line[..=end])
        })
        .flat_map(|tag| tag.split_whitespace())
        .filter_map(|attribute| attribute.strip_prefix("id="))
        // Trim punctuation rather than just quotes: the token may still carry
        // the `>` from the cut above, and may be quoted with either character.
        .any(|value| value.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') == CANVAS_ID)
}

/// Whether the stylesheet has a rule for `#CANVAS_ID`.
///
/// A separate rule from the element, and checked separately: renaming one and not
/// the other leaves the page loading and rendering at the wrong size, which no
/// check on the id alone would notice.
pub fn styles_the_canvas_as_full_bleed(html: &str) -> bool {
    html.lines().map(str::trim).any(|line| line == format!("#{CANVAS_ID} {{"))
}

/// Check the entry page and the service worker.
///
/// Takes their contents rather than reading them, so the rules are testable
/// without a repository on disk.
pub fn check(index: &str, worker: &str) -> Vec<Problem> {
    let mut problems = Vec::new();

    if !canvas_element_is_correctly_identified(index) {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: format!("no element with id `{CANVAS_ID}`; eframe will not find a canvas to draw into"),
        });
    }
    if !styles_the_canvas_as_full_bleed(index) {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: format!(
                "the stylesheet does not rule on #{CANVAS_ID}. A canvas that is not sized to the \
                 viewport renders at its default 300x150 and the application looks broken — and \
                 `scripts/mutation-test-gates.sh` showed that a rename in one place while the \
                 other keeps the old name slips past a substring search"
            ),
        });
    }
    if !index.contains(&bundle_path()) {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: format!(
                "no import of `{}`.\n     \
                 `cargo xtask bundle` emits `{}` and nothing else, so this path and \
                 the `--out-name` it is given are the same constant for a reason.",
                bundle_path(),
                crate::wasm::BUNDLE_NAME
            ),
        });
    }
    if !index.contains("sw.js") {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: "the service worker is never registered, so the build will not work offline".to_string(),
        });
    }
    // The page and the worker each hold a copy of the bundle's name, and the
    // page's copy was checked while the worker's was not — so the worker could
    // precache a bundle that does not exist, and the gate said the two agreed.
    // §67.
    if !worker_caches_the_bundle(worker) {
        problems.push(Problem {
            file: SERVICE_WORKER.to_string(),
            message: format!(
                "the shell it precaches does not include the bundle.\n     \
                 `cargo xtask bundle` writes {} and {}, and the page imports {}.\n     \
                 The page is cached and the application is not, so a user who goes \
                 offline gets a page that cannot start — and the only trace is the \
                 console warning the install step prints.",
                wasm_path(),
                bundle_path(),
                bundle_path()
            ),
        });
    }
    // A page that fails to start must say so. Without this the only symptom is
    // a blank window, and the console is not somewhere a user looks.
    if !index.contains("fallback") {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: "no failure notice: a wasm build that does not start would be a blank page with no explanation".to_string(),
        });
    }
    // The failure box is the only instruction a reader sees when the page is
    // broken, and it used to name the one command that cannot fix it:
    // `cargo xtask wasm` only *checks* that the crates compile. `KNOWN-GAPS.md`
    // §11 recorded that page as a 404 behind four green checks; the authoring
    // instructions were corrected, and this second copy was not — which is the
    // lesson of §11 arriving one file later.
    //
    // Checked as a sentence rather than a substring: `wasm` appears in this page
    // legitimately in the comment explaining why `bundle` is the right command.
    if index.contains("Run:  cargo xtask wasm") {
        problems.push(Problem {
            file: INDEX.to_string(),
            message: "the failure box tells the reader to run `cargo xtask wasm`, which only checks \
                      that the crates compile and writes nothing.\n     \
                      `cargo xtask bundle` is the command that produces the file this page imports."
                .to_string(),
        });
    }

    for rule in RULES {
        if !worker.contains(rule.needle) {
            problems.push(Problem {
                file: SERVICE_WORKER.to_string(),
                message: format!("the rule “{}” is missing ({})", rule.what, rule.why),
            });
        }
        for forbidden in rule.forbidden {
            if worker.contains(forbidden) {
                problems.push(Problem {
                    file: SERVICE_WORKER.to_string(),
                    message: format!("the rule “{}” is broken ({})", rule.what, rule.why),
                });
            }
        }
    }

    problems
}

/// The gate.
pub fn run(root: &Path) -> Result<crate::Outcome, String> {
    step("web");
    let index = read(root, INDEX)?;
    let worker = read(root, SERVICE_WORKER)?;

    let problems = check(&index, &worker);
    if !problems.is_empty() {
        let mut message = format!("{} problem(s) in the browser build:\n", problems.len());
        for problem in &problems {
            message.push_str(&format!("   • {problem}\n"));
        }
        return Err(message);
    }
    println!("   ok: the canvas, the bundle path, the worker's shell and its rules all agree");

    // index.html is not a standalone script — the module inside it is, and node
    // cannot check HTML — so only the worker is handed to the parser. When there
    // is no parser, this is skipped, not passed.
    match command("node", &["--check", SERVICE_WORKER], root) {
        Ok(out) if out.ok() => {
            println!("   ok: {SERVICE_WORKER} parses");
            Ok(crate::Outcome::Ran)
        }
        Ok(out) => Err(format!("{SERVICE_WORKER} does not parse:\n\n{}", out.all())),
        Err(_) => {
            skipped(SERVICE_WORKER, "node is not installed");
            Ok(crate::Outcome::Skipped)
        }
    }
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_INDEX: &str = r#"<style>
#craftkit-canvas {
  width: 100vw;
}
</style>
<canvas id="craftkit-canvas"></canvas>
<script type="module">const { default: init } = await import("./dist/craftkit_web.js"); init();</script>
<div id="fallback"></div>
navigator.serviceWorker.register("./sw.js");"#;

    /// The worker's safety rules, without its shell.
    const GOOD_WORKER_RULES: &str = r#"
if (request.method !== "GET") return;
if (url.origin !== self.location.origin) return;
const response = await fetch(request);
if (response && response.status === 200) { await cache.put(request, response.clone()); }
"#;

    /// A worker that precaches the bundle the page imports.
    ///
    /// Built from the *derived* paths rather than spelled out, so a rename of
    /// `wasm::BUNDLE_NAME` cannot quietly make this fixture wrong — which is
    /// exactly what a hand-written fixture did. §67.
    fn good_worker() -> String {
        let shell = format!(
            "const SHELL = [\n  \"./\",\n  \"./index.html\",\n  \"{}\",\n  \"{}\",\n];\n",
            bundle_path(),
            wasm_path()
        );
        format!("{shell}{GOOD_WORKER_RULES}")
    }

    #[test]
    fn the_failure_box_names_the_command_that_actually_builds_the_page() {
        // §11's falsehood, one file later: the authoring instructions said
        // `bundle` and the error box a broken page shows still said `wasm`.
        let root = crate::root().expect("root");
        let index = std::fs::read_to_string(root.join(INDEX)).expect("the page is readable");
        let worker = std::fs::read_to_string(root.join(SERVICE_WORKER)).expect("worker");
        let problems = check(&index, &worker);
        assert!(problems.is_empty(), "the page in the tree has a problem: {problems:#?}");
        assert!(index.contains("cargo xtask bundle"), "no run-this instruction at all");
    }

    #[test]
    fn a_good_pair_produces_no_problems() {
        assert!(check(GOOD_INDEX, &good_worker()).is_empty(), "{:?}", check(GOOD_INDEX, &good_worker()));
    }

    #[test]
    fn a_renamed_canvas_element_is_caught() {
        // Renaming the element's own `id`.
        let index = GOOD_INDEX.replace("<canvas id=\"craftkit-canvas\"", "<canvas id=\"canvas\"");
        let problems = check(&index, &good_worker());
        assert!(
            problems.iter().any(|p| p.message.contains("no element with id")),
            "the element's id changed and nothing noticed: {problems:?}"
        );
    }

    #[test]
    fn a_rename_that_leaves_the_name_elsewhere_is_still_caught() {
        // The exact mutation the substring search missed: the name survives in
        // the stylesheet and in a comment, so `contains(CANVAS_ID)` is still true.
        let index = format!(
            "<style>#{CANVAS_ID} {{ width: 100vw }}</style>\n\
             <!-- see {CANVAS_ID} -->\n\
             <canvas id=\"c-canvas\" tabindex=\"0\"></canvas>\n\
             <div id=\"fallback\"></div>\n\
             <script>import(\"./dist/craftkit_web.js\"); register(\"./sw.js\");</script>"
        );
        assert!(
            index.contains(CANVAS_ID),
            "the name is still in the file; a substring check would pass this"
        );
        let problems = check(&index, &good_worker());
        assert!(
            problems.iter().any(|p| p.message.contains("no element with id")),
            "the substring check is not enough and must not be used: {problems:?}"
        );
    }

    #[test]
    fn a_stylesheet_that_stops_styling_the_canvas_is_caught() {
        let index = GOOD_INDEX.replace(&format!("#{CANVAS_ID} {{"), "#something-else {");
        let problems = check(&index, &good_worker());
        assert!(problems.iter().any(|p| p.message.contains("stylesheet")), "{problems:?}");
    }

    #[test]
    fn a_bundle_path_the_bundler_would_not_emit_is_caught() {
        // The failure this catches is the one a real browser found: the page
        // imported `craftkit_web.js` while `wasm-bindgen` was writing
        // `craftkit-web.js`, so every load was a 404 with every compile check
        // green. The message says which command produces the bundle, because
        // "the dashes become underscores" was the false explanation this
        // replaced.
        let index = GOOD_INDEX.replace("craftkit_web.js", "craftkit.js");
        let problems = check(&index, &good_worker());
        assert!(problems.iter().any(|p| p.message.contains("cargo xtask bundle")), "{problems:?}");
        assert!(
            !problems.iter().any(|p| p.message.contains("dashes become underscores")),
            "the old message claimed something false and must not come back"
        );
    }

    #[test]
    fn the_bundle_path_is_derived_from_the_bundlers_name() {
        // One constant, so a rename on either side is a compile-time fact
        // rather than a 404.
        assert_eq!(bundle_path(), format!("./dist/{}.js", crate::wasm::BUNDLE_NAME));
    }

    #[test]
    fn the_wasm_path_is_derived_from_the_bundlers_name() {
        // The bundle is two files, and the worker has to cache both of them.
        assert_eq!(wasm_path(), format!("./dist/{}_bg.wasm", crate::wasm::BUNDLE_NAME));
    }

    #[test]
    fn a_worker_that_precaches_a_different_bundle_is_caught() {
        // §67: the page and the worker each hold a copy of the bundle's name,
        // and the worker's copy was not checked — so the worker could precache
        // a file that does not exist while the gate said the two agreed.
        let worker = good_worker().replace(&bundle_path(), "./dist/something_else.js");
        let problems = check(GOOD_INDEX, &worker);
        assert!(
            problems.iter().any(|p| p.message.contains("precaches")),
            "the worker precaches a bundle the page does not import and nothing noticed: {problems:?}"
        );
    }

    #[test]
    fn a_bundle_named_only_in_a_comment_is_not_in_the_shell() {
        // The trap the canvas id has its own function for: a substring search
        // over the whole file stays true when the name survives in a comment,
        // so the shell is parsed rather than searched.
        let worker = format!("// {}\nconst SHELL = [\n  \"./\",\n];\n", bundle_path());
        assert!(worker.contains(&bundle_path()), "the name is still in the file");
        assert!(!worker_caches_the_bundle(&worker), "a comment is not an entry");
    }

    #[test]
    fn the_worker_in_the_tree_precaches_the_bundle_the_page_imports() {
        let root = crate::root().expect("root");
        let worker = std::fs::read_to_string(root.join(SERVICE_WORKER)).expect("sw.js");
        assert!(worker_caches_the_bundle(&worker), "sw.js does not precache the bundle");
    }

    #[test]
    fn an_unregistered_service_worker_is_caught() {
        let index = GOOD_INDEX.replace("sw.js", "offline.js");
        let problems = check(&index, &good_worker());
        assert!(problems.iter().any(|p| p.message.contains("offline")), "{problems:?}");
    }

    #[test]
    fn a_page_with_no_failure_notice_is_caught() {
        let index = GOOD_INDEX.replace("fallback", "something-else");
        let problems = check(&index, &good_worker());
        assert!(problems.iter().any(|p| p.message.contains("blank page")), "{problems:?}");
    }

    #[test]
    fn a_service_worker_that_answers_any_method_is_caught() {
        let worker = good_worker().replace(r#"if (request.method !== "GET") return;"#, "");
        let problems = check(GOOD_INDEX, &worker);
        assert!(problems.iter().any(|p| p.message.contains("only GET")), "{problems:?}");
    }

    #[test]
    fn a_service_worker_that_caches_another_origin_is_caught() {
        let worker = good_worker().replace("if (url.origin !== self.location.origin) return;", "");
        let problems = check(GOOD_INDEX, &worker);
        assert!(problems.iter().any(|p| p.message.contains("same-origin")), "{problems:?}");
    }

    #[test]
    fn a_service_worker_that_stores_a_non_200_is_caught() {
        let worker = good_worker().replace("response.status === 200", "true");
        let problems = check(GOOD_INDEX, &worker);
        assert!(problems.iter().any(|p| p.message.contains("200")), "{problems:?}");
    }

    #[test]
    fn the_repository_web_files_pass() {
        let root = crate::root().expect("root");
        let index = std::fs::read_to_string(root.join(INDEX)).expect("index.html");
        let worker = std::fs::read_to_string(root.join(SERVICE_WORKER)).expect("sw.js");
        let problems = check(&index, &worker);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn the_canvas_id_matches_the_rust_constant() {
        // Two copies of one string; this is the test that keeps them together.
        let root = crate::root().expect("root");
        let stash = std::fs::read_to_string(root.join("apps/craftkit-web/src/stash.rs")).expect("stash.rs");
        assert!(
            stash.contains(&format!("\"{CANVAS_ID}\"")),
            "stash.rs does not use {CANVAS_ID}; update both"
        );
    }
}
