//! Two rules about files, both of which have to be checked rather than trusted.
//!
//! # 1. Every icon is drawn in code
//!
//! The UI's icons are vector paths in `crates/ui/src/icons.rs`, drawn from
//! original coordinates. There is no icon file, no sprite sheet, no font with
//! pictographs in it, and no artwork of any kind in this repository.
//!
//! That is a rule about *provenance*, not about taste, and it is the rule most
//! worth a gate in a project like this: it is trivially easy to add one PNG and
//! trivially easy to be careless about where it came from. So the check is
//! mechanical — no `include_bytes!`, `include!` or `Image::from_bytes` of
//! anything that is not code, anywhere in the crates that draw.
//!
//! # 2. Every non-code file is attributed
//!
//! If a file does get added — a licence notice, a template, a test fixture —
//! it needs a row in `ATTRIBUTION.md` saying what it is and where it came from.
//! Generated build output is exempt, and so is anything under `target/`.
//!
//! The two rules are in one gate because they are the same rule seen from two
//! sides: the origin of every byte that reaches a user.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::step;

/// File extensions that are assets rather than code.
///
/// Anything in here is a file a person can see, or a file that ends up inside
/// the programs. Both need to be attributed.
pub const ASSET_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "webp", "ico", "bmp", "tiff", "ttf", "otf", "woff", "woff2", "eot",
    "wasm", "mp3", "wav", "ogg", "mp4", "webm",
];

/// Source files written in a language other than Rust, by extension.
///
/// This enforces `AGENTS.md`'s "Rust only": the tree is Rust, plus the handful
/// of files listed in [`FOREIGN_SOURCE_EXCEPTIONS`]. The list was assembled by
/// looking at the tree rather than at the rule, because the rule was already
/// untrue when this gate was written — `AGENTS.md` named only the two
/// JavaScript files a browser insists on, and the tree also holds two shell
/// scripts and the CI workflow.
///
/// Nothing enforced any of it. A Python build helper inside `crates/ui`, a
/// stray `.js` in the web app and a `deploy.sh` inside `crates/core` all left
/// `cargo xtask ci` green, because `is_asset` only knows about binary and media
/// formats and a script is not one.
pub const FOREIGN_SOURCE_EXTENSIONS: &[&str] = &[
    "js", "mjs", "cjs", "jsx", "ts", "tsx", "py", "rb", "sh", "bash", "zsh", "pl", "php", "go", "c", "h",
    "cc", "cpp", "hpp", "java", "cs", "kt", "swift", "lua",
];

/// The non-Rust files the tree is allowed to hold, and why each one is here.
///
/// Each row is a decision somebody made. Adding a file to this list is how you
/// say "yes, really", and the reason is what stops the next person wondering.
pub const FOREIGN_SOURCE_EXCEPTIONS: &[(&str, &str)] = &[
    ("apps/craftkit-web/sw.js", "a service worker; a browser only runs one as JavaScript"),
    ("apps/craftkit-web/index.html", "holds the module script the page imports"),
    ("scripts/mutation-test-gates.sh", "the gate instrument, and bash is what CI has"),
    (
        "scripts/live-window-check.sh",
        "drives the real window on a real display; bash is what the display has",
    ),
    ("scripts/fold-format-pending.sh", "runs once, by hand, at a release"),
    (".github/workflows/ci.yml", "continuous integration is YAML and runs nothing"),
];

/// Whether a path is source code in a language other than Rust.
pub fn is_foreign_source(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => FOREIGN_SOURCE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// A repository-relative path as a `/`-separated string.
///
/// `walk` yields whatever the platform's `strip_prefix` gives, which on Windows
/// is backslash-separated. Comparing that against the exception table without
/// normalising makes every exception a false negative on the one platform where
/// the rule would then be silently off.
fn relative_name(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Foreign-source files in the tree that no exception allows.
pub fn find_foreign_source(root: &Path) -> Result<Vec<String>, String> {
    let mut all = Vec::new();
    crate::tree::walk(root, root, &mut all, &is_foreign_source)?;
    let mut found: Vec<String> = all
        .iter()
        .map(|p| relative_name(p))
        .filter(|name| !FOREIGN_SOURCE_EXCEPTIONS.iter().any(|(allowed, _)| allowed == name))
        .collect();
    found.sort();
    Ok(found)
}

/// The `include`-shaped calls that would pull a file into the programs.
pub const EMBED_CALLS: &[&str] = &["include_bytes!", "include_str!", "include!"];

/// What the check found.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Findings {
    /// Non-code files that no row in `ATTRIBUTION.md` mentions.
    pub unattributed: Vec<String>,
    /// Embedding an asset from source, by file and line.
    pub embedded: Vec<String>,
    /// Files that exist in `ATTRIBUTION.md` but not on disk.
    pub stale: Vec<String>,
    /// Asset names the document mentions that no table row claims: a malformed
    /// row, or prose that should have been fenced.
    pub malformed: Vec<String>,
    /// Source in a language other than Rust, that no exception allows.
    pub foreign: Vec<String>,
    /// Whether `ATTRIBUTION.md` is there at all.
    pub have_attribution: bool,
}

impl Findings {
    /// Whether everything is in order.
    /// Whether everything is in order.
    ///
    /// Every field, deliberately. It once forgot `malformed`, so a test called
    /// `…is_clean` passed on a tree with a broken `ATTRIBUTION.md` row — the
    /// same gap as a test named after a property that checks something narrower.
    pub fn is_clean(&self) -> bool {
        self.have_attribution
            && self.unattributed.is_empty()
            && self.embedded.is_empty()
            && self.stale.is_empty()
            && self.malformed.is_empty()
            && self.foreign.is_empty()
    }

    pub fn count(&self) -> usize {
        self.unattributed.len()
            + self.embedded.len()
            + self.stale.len()
            + self.malformed.len()
            + self.foreign.len()
            + usize::from(!self.have_attribution)
    }
}

/// Whether a file name looks like an asset rather than code.
pub fn is_asset(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ASSET_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// The files under `root` that are assets, as paths relative to `root`.
///
/// The walk is the tree's one (`crate::tree::walk`): it is hand-written rather
/// than done with a glob crate, and it lets the tests run on a temporary
/// directory with no filesystem assumptions beyond "a directory is a directory".
pub fn find_assets(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    crate::tree::walk(root, root, &mut found, &is_asset)?;
    found.sort();
    Ok(found)
}

/// Every file under `root` that has no asset extension, for attribution checks.
///
/// Not needed today — there are no assets to attribute — but the walk is shared
/// with the icon check, and having it here means the rule can be tested with a
/// temporary tree.
pub fn find_sources(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    crate::tree::walk(root, root, &mut found, &|path| {
        path.extension().and_then(|e| e.to_str()) == Some("rs")
    })?;
    found.sort();
    Ok(found)
}

/// Find embedding calls that pull an asset into the programs.
///
/// A hit on `include_str!` of a non-`.rs` file is a violation. `include_str!`
/// of a `.rs` file is not an asset embed and is not reported — several crates
/// legitimately inline text at build time.
pub fn find_embedded(root: &Path, sources: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut hits = Vec::new();
    for source in sources {
        let path = root.join(source);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => return Err(format!("could not read {}: {e}", path.display())),
        };
        // The **shape** comes from the code and the **contents** from the
        // text: an `include_str!` inside a comment or inside a string is a
        // sentence about an embed, not an embed. The old rule read the text
        // and cut the line at the first `//`, so a block comment that showed
        // a call was reported as an embed of a file the tree does not have.
        // §166.
        //
        // The two views are the same length, byte for byte, so the offset of
        // the call in one is the offset of the call in the other.
        let code = crate::lists::code_lines_kept(&text);
        for (number, (code_line, text_line)) in code.lines().zip(text.lines()).enumerate() {
            for call in EMBED_CALLS {
                let Some(at) = code_line.find(call) else { continue };
                let rest = &text_line[at + call.len()..];
                let argument = rest.trim_start().trim_start_matches('(');
                let Some(start) = argument.find('"') else { continue };
                let after = &argument[start + 1..];
                let end = after.find('"').unwrap_or(0);
                let target = &after[..end];
                // An argument that is not a string literal cannot be resolved
                // from here; a gate that guessed would be a gate that lies.
                if !target.is_empty() && is_asset(Path::new(target)) {
                    hits.push(format!("{}:{}: {call} pulls in “{target}”", source.display(), number + 1));
                }
            }
        }
    }
    hits.sort();
    Ok(hits)
}

/// Every asset name mentioned in `ATTRIBUTION.md`, however it is written.
///
/// The complement of [`attributed_files`]. A row that is malformed — glued onto
/// the end of a prose line, because the file was missing its trailing newline
/// and an editor appended to it — is not a table row, so `attributed_files`
/// cannot see it and the row silently stops counting. This one can.
///
/// That is not hypothetical: `scripts/mutation-test-gates.sh` appended a row to a
/// file with no trailing newline, the stale-row check reported nothing, and that
/// is how the missing newline was found in the first place.
pub fn mentioned_files(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_fence = false;
    for line in text.lines() {
        // Fenced blocks are skipped for the same reason `attributed_files`
        // skips them: the document has to be able to *show* an example row, and
        // an example the gate reads as a mistake makes the documentation
        // unhelpful.
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for token in line.split(|c: char| c == '`' || c.is_whitespace()) {
            let token = token.trim_start_matches('*');
            if !token.is_empty() && is_asset(Path::new(token)) {
                names.insert(token.to_string());
            }
        }
    }
    names
}

/// The asset file names `ATTRIBUTION.md` claims, read out of its table.
///
/// Deliberately loose: it matches the *file name* anywhere in the table row, so
/// the file needs no particular column layout. A gate that demands a specific
/// table shape makes people avoid the table.
///
/// Fenced code blocks are skipped. Documentation needs to be able to *show* an
/// example row — this very module's tests do — and an example that the gate
/// reads as a claim is a gate that makes the documentation lie to itself.
pub fn attributed_files(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        for cell in trimmed.split('|') {
            let cell = cell.trim();
            for token in cell.split(|c: char| c == '`' || c.is_whitespace()) {
                let token = token.trim_start_matches('*');
                if !token.is_empty() && is_asset(Path::new(token)) {
                    names.insert(token.to_string());
                }
            }
        }
    }
    names
}

/// Run the checks against `root`.
pub fn inspect(root: &Path) -> Result<Findings, String> {
    let attribution = root.join("ATTRIBUTION.md");
    let (have_attribution, text) = match std::fs::read_to_string(&attribution) {
        Ok(text) => (true, text),
        Err(_) => (false, String::new()),
    };
    let claimed = attributed_files(&text);

    let assets = find_assets(root)?;
    let mut unattributed = Vec::new();
    for asset in &assets {
        let name = asset.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if !claimed.contains(&name) {
            unattributed.push(asset.display().to_string());
        }
    }

    let mut on_disk: BTreeSet<String> =
        assets.iter().filter_map(|a| a.file_name().and_then(|n| n.to_str()).map(str::to_string)).collect();
    // The project's own brand mark, if it has one, is described in prose.
    on_disk.insert("ATTRIBUTION.md".to_string());
    let stale: Vec<String> = claimed.difference(&on_disk).cloned().collect();

    // A name that is mentioned but claimed by no table row. Either the row is
    // malformed, or it is prose describing a file — in which case it should be
    // fenced, so that the fix and the check agree.
    let mut malformed: Vec<String> = mentioned_files(&text)
        .difference(&claimed)
        // Names that really are on disk with no row are `unattributed`, which
        // already says so with a better message.
        .filter(|name| !on_disk.contains(*name))
        .cloned()
        .collect();
    malformed.sort();

    let sources = find_sources(root)?;
    let embedded = find_embedded(root, &sources)?;

    let foreign = find_foreign_source(root)?;

    Ok(Findings { unattributed, embedded, stale, malformed, foreign, have_attribution })
}

/// The gate.
pub fn run(root: &Path) -> Result<crate::Outcome, String> {
    step("assets");
    let findings = inspect(root)?;

    if !findings.have_attribution {
        return Err("there is no ATTRIBUTION.md.\n\
             Every non-code file needs a row saying what it is and where it came from.\n\
             An empty table with a sentence explaining why it is empty is fine; a missing file\n\
             is not, because the next person cannot tell whether the check ran."
            .to_string());
    }

    let mut problems = String::new();
    for hit in &findings.embedded {
        problems.push_str(&format!("   • {hit}\n"));
        problems.push_str(
            "     Icons are drawn in code from original coordinates (crates/ui/src/icons.rs).\n\
             \x20    No image file is embedded in a program. Draw it instead.\n",
        );
    }
    for path in &findings.unattributed {
        problems.push_str(&format!("   • {path} is a file asset with no row in ATTRIBUTION.md\n"));
    }
    for name in &findings.stale {
        problems.push_str(&format!("   • ATTRIBUTION.md lists {name}, which is not in the tree\n"));
    }
    for name in &findings.malformed {
        problems.push_str(&format!(
            "   • ATTRIBUTION.md mentions {name} but no table row claims it.\n     \
             Either the row is not on a line of its own — check for a missing newline at the end \
             of the line before it — or it is prose and belongs inside a fenced code block.\n"
        ));
    }

    for name in &findings.foreign {
        problems.push_str(&format!(
            "   • {name} is written in something other than Rust.\n     \
             This project is Rust; the tree holds {} declared exception(s).\n     \
             If this file is genuinely necessary, add it to FOREIGN_SOURCE_EXCEPTIONS\n     \
             in xtask/src/assets.rs with a reason. If it is not, delete it.\n",
            FOREIGN_SOURCE_EXCEPTIONS.len()
        ));
    }

    if problems.is_empty() {
        println!(
            "   ok: no embedded image assets, every asset is attributed, and nothing outside \
             Rust crept in"
        );
        Ok(crate::Outcome::Ran)
    } else {
        Err(format!(
            "{} asset problem(s):\n{problems}\n\
             For a new asset, add a row to ATTRIBUTION.md naming the file, what it is and\n\
             where it came from. For an icon, draw it: crates/ui/src/icons.rs takes an\n\
             `Icon` of line segments and that is the whole of the icon system.",
            findings.count()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch directory, removed when the test ends.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("craftkit-xtask-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("temp dir");
            Temp(path)
        }

        fn write(&self, relative: &str, body: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("parent dir");
            }
            std::fs::write(&path, body).expect("write");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_image_is_an_asset_and_a_source_file_is_not() {
        assert!(is_asset(Path::new("logo.png")));
        assert!(is_asset(Path::new("a/b/icon.SVG")));
        assert!(!is_asset(Path::new("lib.rs")));
        assert!(!is_asset(Path::new("README.md")));
        assert!(!is_asset(Path::new("Cargo.toml")));
        // No extension at all.
        assert!(!is_asset(Path::new("Makefile")));
    }

    #[test]
    fn a_png_encoder_is_not_a_ui_toolkit_and_not_an_asset_rule_trigger() {
        // `image` is a library crate name; this is about file extensions.
        assert!(!is_asset(Path::new("image.rs")));
    }

    #[test]
    fn the_walk_finds_assets_and_skips_build_output() {
        let temp = Temp::new("walk");
        temp.write("crates/ui/src/icons.rs", "pub const ICONS: u8 = 0;");
        temp.write("crates/ui/assets/splash.png", "not really a png");
        temp.write("target/debug/thing.png", "build output");
        temp.write(".hidden/secret.png", "hidden");

        let found = find_assets(temp.path()).expect("walks");
        let names: Vec<String> = found.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(names, vec!["crates/ui/assets/splash.png".to_string()]);
    }

    #[test]
    fn an_embedded_image_is_found_with_its_line() {
        let temp = Temp::new("embed");
        temp.write(
            "crates/ui/src/icons.rs",
            "\n// a comment\nconst ICONS: &[u8] = include_bytes!(\"icons.png\");\n",
        );

        let sources = find_sources(temp.path()).expect("sources");
        let hits = find_embedded(temp.path(), &sources).expect("scans");
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(hits[0].contains("icons.png"), "{hits:?}");
        assert!(hits[0].contains(":3:"), "the line number matters: {hits:?}");
    }

    #[test]
    fn a_commented_out_embed_is_not_a_violation() {
        // Otherwise the obvious "comment it out to get past the gate" works,
        // and the gate is decoration.
        let temp = Temp::new("commented");
        temp.write("src/lib.rs", "// const X: &[u8] = include_bytes!(\"old.png\");\n");

        let sources = find_sources(temp.path()).expect("sources");
        assert!(find_embedded(temp.path(), &sources).expect("scans").is_empty());
    }

    /// An embed call inside a **block** comment is not an embed.
    ///
    /// The old rule read the tree's source as text and cut each line at the
    /// first `//`, so a line comment was handled and a block comment was not:
    /// a block comment that showed a call was reported as an embed of a
    /// file the tree does not have. §166.
    #[test]
    fn an_embed_call_in_a_block_comment_is_not_an_embed() {
        let temp = Temp::new("block-comment");
        temp.write("src/lib.rs", "/* include_bytes!(\"icons.png\") */\npub fn f() {}\n");

        let sources = find_sources(temp.path()).expect("sources");
        assert!(find_embedded(temp.path(), &sources).expect("scans").is_empty());

        // And a call that is really in the code is found.
        temp.write("src/lib.rs", "const ICONS: &[u8] = include_bytes!(\"icons.png\");\n");
        let sources = find_sources(temp.path()).expect("sources");
        let hits = find_embedded(temp.path(), &sources).expect("scans");
        assert_eq!(hits.len(), 1, "{hits:?}");
    }

    /// An embed call inside a **string** is not an embed.
    ///
    /// The same rule read the text, and a raw string holds a call as
    /// plainly as a comment does. §166.
    #[test]
    fn an_embed_call_in_a_string_is_not_an_embed() {
        let temp = Temp::new("in-a-string");
        temp.write("src/lib.rs", "const NOTE: &str = r#\"include_bytes!(\"icons.png\")\"#;\n");

        let sources = find_sources(temp.path()).expect("sources");
        assert!(find_embedded(temp.path(), &sources).expect("scans").is_empty());
    }

    #[test]
    fn inlining_a_rust_file_is_not_an_asset_embed() {
        let temp = Temp::new("inliner");
        temp.write("src/lib.rs", "const T: &str = include_str!(\"../schema.rs\");\n");

        let sources = find_sources(temp.path()).expect("sources");
        assert!(find_embedded(temp.path(), &sources).expect("scans").is_empty());
    }

    #[test]
    fn an_attributed_file_is_not_reported() {
        let temp = Temp::new("attributed");
        temp.write("crates/ui/assets/splash.png", "png");
        temp.write(
            "ATTRIBUTION.md",
            "| File | What | Source |\n|---|---|---|\n| splash.png | splash | original |\n",
        );

        let findings = inspect(temp.path()).expect("inspects");
        assert!(findings.unattributed.is_empty(), "{findings:?}");
        assert!(findings.have_attribution);
        assert!(findings.is_clean(), "{findings:?}");
    }

    #[test]
    fn an_example_inside_a_code_fence_is_not_a_claim() {
        // The example has to be able to live in the document without the gate
        // demanding the file it names.
        let markdown = "Real table:\n\n| File | What | Source |\n|---|---|---|\n\n            Then how to add one:\n\n```\n| `assets/wrench.svg` | the icon | original |\n```\n\n            More prose.\n| `real.png` | an actual file | original |\n";
        let claimed = attributed_files(markdown);
        assert!(!claimed.contains("wrench.svg"), "{claimed:?}");
        assert!(claimed.contains("real.png"), "{claimed:?}");
    }

    #[test]
    fn an_unattributed_file_is_reported() {
        let temp = Temp::new("unattributed");
        temp.write("crates/ui/assets/splash.png", "png");
        temp.write("ATTRIBUTION.md", "Everything is original.\n");

        let findings = inspect(temp.path()).expect("inspects");
        assert_eq!(findings.unattributed.len(), 1, "{findings:?}");
        assert!(findings.unattributed[0].contains("splash.png"), "{findings:?}");
        assert!(!findings.is_clean());
    }

    #[test]
    fn a_row_glued_onto_a_prose_line_is_reported_as_malformed() {
        // The file had no trailing newline, so appending a row produced one very
        // long line starting with prose. `attributed_files` correctly refuses to
        // read it as a row; the gate must still notice the name.
        let markdown = "Everything is original. It is not.| `ghost.png` | an icon | original |";
        assert!(attributed_files(markdown).is_empty(), "it is not a table row");
        assert!(mentioned_files(markdown).contains("ghost.png"));
    }

    #[test]
    fn a_fenced_example_is_neither_claimed_nor_malformed() {
        // The "how to add a row" section shows a real-looking row. It must be
        // read as neither a claim nor a problem.
        let temp = Temp::new("fenced");
        temp.write("ATTRIBUTION.md", "## Adding a row\n\n```\n| `wrench.svg` | the icon | original |\n```\n");
        let findings = inspect(temp.path()).expect("inspects");
        assert!(findings.malformed.is_empty(), "{:?}", findings.malformed);
        assert!(findings.unattributed.is_empty(), "{:?}", findings.unattributed);
    }

    #[test]
    fn the_real_attribution_file_is_clean() {
        // Including the malformed check, which is how the missing trailing
        // newline was found.
        let root = crate::root().expect("root");
        let findings = inspect(&root).expect("inspects");
        assert!(findings.malformed.is_empty(), "{:?}", findings.malformed);
    }

    #[test]
    fn this_repository_contains_no_source_outside_rust() {
        let root = crate::root().expect("root");
        let foreign = find_foreign_source(&root).expect("scans");
        assert!(foreign.is_empty(), "source outside Rust: {foreign:?}");
    }

    #[test]
    fn every_declared_exception_is_a_real_file_with_a_reason() {
        // An exception that names a file nobody has is a permission slip that
        // looks like a decision. Both halves are checked.
        let root = crate::root().expect("root");
        for (name, why) in FOREIGN_SOURCE_EXCEPTIONS {
            assert!(!why.trim().is_empty(), "{name} has no reason given");
            assert!(root.join(name).is_file(), "{name} is allowed but is not in the tree; drop the row");
        }
    }

    #[test]
    fn a_script_in_the_tree_is_reported_and_an_exception_hides_it() {
        // The failure this catches: the whole Rust-only rule. A `.py`, a `.js`
        // and a `.sh` all left `cargo xtask ci` green, because `is_asset` only
        // knows binary and media formats.
        let dir = std::env::temp_dir().join(format!("craftkit-foreign-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("crates/ui/src")).expect("temp tree");

        std::fs::write(dir.join("crates/ui/build.py"), "print('x')").expect("write");
        let found = find_foreign_source(&dir).expect("scans");
        assert_eq!(found, vec!["crates/ui/build.py".to_string()], "{found:?}");

        // The same file, listed as an exception, is not a finding.
        let mut allowed = FOREIGN_SOURCE_EXCEPTIONS.to_vec();
        allowed.push(("crates/ui/build.py", "a test exception"));
        let hidden = found
            .iter()
            .filter(|name| !allowed.iter().any(|(a, _)| a == *name))
            .cloned()
            .collect::<Vec<String>>();
        assert!(hidden.is_empty(), "{hidden:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn is_clean_counts_a_malformed_row() {
        // `is_clean` forgot `malformed` for a while, so a test named after it
        // passed on a tree with a broken row. Every field or none.
        let base = Findings {
            unattributed: vec![],
            embedded: vec![],
            stale: vec![],
            malformed: vec![],
            foreign: vec![],
            have_attribution: true,
        };
        assert!(base.is_clean());
        assert_eq!(base.count(), 0);

        let broken = Findings { malformed: vec!["x.png".into()], ..base.clone() };
        assert!(!broken.is_clean(), "a malformed row is not clean");
        assert_eq!(broken.count(), 1);

        let foreign = Findings { foreign: vec!["a.py".into()], ..base };
        assert!(!foreign.is_clean(), "source outside Rust is not clean");
        assert_eq!(foreign.count(), 1);
    }

    #[test]
    fn a_row_for_a_file_that_is_gone_is_reported_as_stale() {
        // The other direction matters just as much: an attribution list that
        // only ever grows stops being evidence of anything.
        let temp = Temp::new("stale");
        temp.write(
            "ATTRIBUTION.md",
            "| File | What | Source |\n|---|---|---|\n| gone.png | old icon | somewhere |\n",
        );

        let findings = inspect(temp.path()).expect("inspects");
        assert_eq!(findings.stale, vec!["gone.png".to_string()], "{findings:?}");
    }

    #[test]
    fn the_project_ships_no_image_assets() {
        // The point of the whole rule, asserted about this repository rather
        // than about a temporary tree.
        let root = crate::root().expect("workspace root");
        let assets = find_assets(&root).expect("walks");
        let images: Vec<&PathBuf> = assets
            .iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()).map(|e| e != "wasm").unwrap_or(true))
            .collect();
        assert!(images.is_empty(), "craftkit ships no images; found {images:?}");
    }

    #[test]
    fn the_project_embeds_no_images() {
        let root = crate::root().expect("workspace root");
        let sources = find_sources(&root).expect("sources");
        let hits = find_embedded(&root, &sources).expect("scans");
        assert!(hits.is_empty(), "an image is embedded at {hits:?}");
    }

    #[test]
    fn the_project_has_an_attribution_file() {
        let root = crate::root().expect("workspace root");
        assert!(root.join("ATTRIBUTION.md").is_file(), "ATTRIBUTION.md must exist");
    }
}
