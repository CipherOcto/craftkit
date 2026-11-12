//! The **counts** this README says are the counts of the tree.
//!
//! The README says how many commands each battery carries, and a number that
//! is wrong is a claim that cannot be believed. This test checks the count of
//! every battery against **the registry** — the commands the linker put in —
//! and it checks the source of the tree is text.
//!
//! # Why the registry and not the source
//!
//! This test used to count the string `#[command(` in the source of each
//! battery. That is a *spelling*, and a spelling can be wrong in ways a
//! count cannot see: a command declared across two lines, a command in a
//! file the scan does not read, a command whose area is not its crate's name.
//!
//! The registry is the truth: every command is a `CommandSpec` in a
//! link-time inventory, and `Registry::from_inventory` reads it back. The
//! text scan is **kept** here as a **cross-check** — the two must agree —
//! because the tree's own tooling reads the source, and a tool that reads
//! the source must be right about it.
//!
//! # The batteries that are linked in
//!
//! A battery is an optional feature of this application, so a build without
//! the features has none of them linked in, and the registry has none of
//! their commands. The test therefore checks the batteries **this build
//! links**, by feature, and says so when it skips one.

use std::fs;
use std::path::{Path, PathBuf};

use craftkit_sdk::ecosystem::Ecosystem;

/// The **batteries** this README counts, and whether this build links each
/// one in.
///
/// **The feature is the answer.** The batteries are optional features of this
/// application, so the feature that turns a dependency on is exactly the
/// question "is this battery in this build?" — and a build without the
/// features has none of them linked in, and the registry has none of their
/// commands.
const BATTERIES: [(&str, bool); 9] = [
    ("grid", cfg!(feature = "grid")),
    ("prose", cfg!(feature = "prose")),
    ("pdf", cfg!(feature = "pdf")),
    ("film", cfg!(feature = "film")),
    ("layout", cfg!(feature = "layout")),
    ("cad", cfg!(feature = "cad")),
    ("fx", cfg!(feature = "fx")),
    ("sound", cfg!(feature = "sound")),
    ("vector", cfg!(feature = "vector")),
];

/// The **root** of the tree: the folder that holds the crates and this
/// README.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// The **README** of the tree.
fn readme(root: &Path) -> String {
    fs::read_to_string(root.join("README.md")).expect("the README is there")
}

/// The **source** of a crate: every file of it that is Rust.
fn source_of(root: &Path, crate_name: &str) -> Vec<PathBuf> {
    let directory = root.join("crates").join(format!("craftkit-plugin-{crate_name}")).join("src");
    let mut files = Vec::new();
    let entries = fs::read_dir(&directory).expect("the crate is there");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// The **commands** a crate has: the attributes that carry an id.
fn commands_of(root: &Path, crate_name: &str) -> usize {
    let mut total = 0;
    for path in source_of(root, crate_name) {
        let bytes = fs::read(&path).expect("the file is there");
        total += count(&bytes, b"#[command(");
        // A battery that is a **table** declares its commands in rows, and
        // a row is not an attribute: the scan has to read both spellings,
        // or it reads a table as a battery with no commands in it.
        total += rows_in(&bytes);
    }
    total
}

/// How many **rows** a file's tables declare.
///
/// The rows are the table's **top level**: a row's own braces hold the
/// parameters and the body, and what is inside them is not a row.
///
/// The scan is this file's own — the xtask reads the tree with another
/// one, and the two have to agree.
fn rows_in(bytes: &[u8]) -> usize {
    let text = String::from_utf8_lossy(bytes);
    let mut total = 0;
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("commands!") {
        let start = offset + at;
        let Some(open) = text[start..].find('{') else { break };
        let body_start = start + open + 1;
        let Some(end) = matching_brace(&text, body_start) else { break };
        total += rows_of(&text[body_start..end]);
        offset = end;
    }
    total
}

/// The rows of one table.
fn rows_of(block: &str) -> usize {
    let bytes = block.as_bytes();
    let mut total = 0;
    let mut at = 0usize;
    let mut depth = 0usize;
    let mut head_start = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            at += 1;
            continue;
        }
        match c {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    let head = block[head_start..at].trim();
                    let last = head.split_whitespace().last().unwrap_or("");
                    if is_an_id(last) {
                        total += 1;
                    }
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    head_start = at + 1;
                }
            }
            _ => {}
        }
        at += 1;
    }
    total
}

/// Whether a word is a command's **id**: dotted, lower case.
///
/// The same rule the tree's index uses, and this file's own copy of it:
/// two readers, and they have to agree.
fn is_an_id(word: &str) -> bool {
    let mut parts = word.split('.');
    let Some(first) = parts.next() else { return false };
    let ok = |part: &str| {
        !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    if !ok(first) {
        return false;
    }
    let mut more = false;
    for part in parts {
        if !ok(part) {
            return false;
        }
        more = true;
    }
    more
}

/// Where the brace that closes the one at `body_start` is.
fn matching_brace(text: &str, body_start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut at = body_start;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(at);
                    }
                }
                _ => {}
            }
        }
        at += 1;
    }
    None
}

/// How many times a **needle** is in a **haystack**.
fn count(haystack: &[u8], needle: &[u8]) -> usize {
    let mut total = 0;
    let mut at = 0;
    while at + needle.len() <= haystack.len() {
        if &haystack[at..at + needle.len()] == needle {
            total += 1;
            at += needle.len();
        } else {
            at += 1;
        }
    }
    total
}

/// The **highest** count the README gives for a battery.
///
/// The commands of a battery are only ever added, so the highest count
/// the README says is the count of the tree.
fn highest_count(readme: &str, battery: &str) -> Option<usize> {
    // The README writes the name of a battery as it likes — `PDF` and
    // `pdf` are the same battery — and the words are read without
    // regard to case.
    let flat: String = readme.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let needle = format!("{} battery ", battery.to_lowercase());
    let mut found = None;
    let mut rest = flat.as_str();
    while let Some(at) = rest.find(&needle) {
        let tail = &rest[at + needle.len()..];
        // The count follows the words, with **now** between them.
        let tail = tail.strip_prefix("now ").unwrap_or(tail);
        if let Some(open) = tail.strip_prefix("carries **") {
            let number = &open[..open.find("**")?];
            if let Ok(count) = number.parse::<usize>() {
                found = Some(found.map_or(count, |held: usize| held.max(count)));
            }
        }
        rest = &rest[at + needle.len()..];
    }
    found
}

/// The ecosystem of this program: everything it was linked with.
///
/// **The application's own ecosystem**, not a bare inventory. Asking the
/// application is what makes the batteries *linked in*: the build script
/// writes the references that keep them, and this is the call that reaches
/// them.
fn the_ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built from what was linked in")
}

/// The count of a battery, by the registry, and by the source.
///
/// **Both, always.** The registry is the truth and the source is what the
/// tree's own tools read, and a tool that reads the source must be right
/// about it — so the two are compared, and a disagreement is a failure of
/// one of them.
#[test]
fn the_counts_of_the_readme_are_the_counts_of_the_tree() {
    let root = root();
    let readme = readme(&root);
    let ecosystem = the_ecosystem();
    for (battery, linked) in BATTERIES {
        if !linked {
            // The battery is not in this build. The registry cannot be asked
            // about it, and the README's number is not this build's number.
            continue;
        }
        let by_registry = ecosystem.commands.of_area(battery).len();
        let claimed = highest_count(&readme, battery)
            .unwrap_or_else(|| panic!("the README counts the {battery} battery"));
        assert_eq!(
            claimed, by_registry,
            "the README says the {battery} battery carries {claimed} commands, and it carries \
             {by_registry}"
        );
        let by_source = commands_of(&root, battery);
        assert_eq!(
            by_registry, by_source,
            "the registry says the {battery} battery carries {by_registry} commands, and its source \
             carries {by_source}: the tools that read the source must agree with the tree"
        );
    }
}

/// Whether this build links a battery in.
///
/// The table says, and the table is the feature.
fn linked_in(battery: &str) -> bool {
    BATTERIES.iter().find(|(name, _)| *name == battery).map(|(_, linked)| *linked).unwrap_or(false)
}

/// The **counted block** of the README, and the counts it holds.
///
/// The block is where the tree's nine numbers are written down together:
/// ```text
/// <!-- the counts of the tree: begin -->
/// | battery | commands |
/// | grid | 188 |
/// <!-- the counts of the tree: end -->
/// ```
/// It is written by `cargo xtask counts --write`, and it is checked here
/// against the registry — so the block and the tree cannot disagree.
fn the_counted_block(readme: &str) -> Vec<(String, usize)> {
    let begin = "<!-- the counts of the tree: begin -->";
    let end = "<!-- the counts of the tree: end -->";
    let start = readme.find(begin).expect("the README has the counted block");
    let stop = readme.find(end).expect("the README has the counted block");
    assert!(stop > start, "the counted block's markers are the wrong way round");
    let mut counts = Vec::new();
    for line in readme[start..stop].lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('|') else { continue };
        let Some(rest) = rest.strip_suffix('|') else { continue };
        let mut parts = rest.split('|').map(str::trim);
        let (Some(name), Some(number)) = (parts.next(), parts.next()) else { continue };
        if parts.next().is_some() {
            continue;
        }
        let Ok(count) = number.parse::<usize>() else { continue };
        counts.push((name.to_string(), count));
    }
    counts
}

/// The counted block of the README is the registry's count.
///
/// **One place where the tree's numbers are written down**, and it is checked
/// against the truth: the registry of the program.
#[test]
fn the_counted_block_is_the_registry() {
    let readme = readme(&root());
    let ecosystem = the_ecosystem();
    let block = the_counted_block(&readme);
    assert!(!block.is_empty(), "the counted block holds no counts");
    for (battery, claimed) in &block {
        if !linked_in(battery) {
            continue;
        }
        let count = ecosystem.commands.of_area(battery).len();
        assert_eq!(
            count, *claimed,
            "the README's counted block says the {battery} battery carries {claimed} commands, and \
             it carries {count}"
        );
    }
}

/// The **source** of the tree is text: no file of it holds a NUL byte,
/// which would make it a file of bytes and not a file of words.
#[test]
fn the_source_of_the_tree_is_text() {
    let root = root();
    for folder in ["crates", "apps"] {
        let mut stack = vec![root.join(folder)];
        while let Some(directory) = stack.pop() {
            let entries = fs::read_dir(&directory).expect("the folder is there");
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                    continue;
                }
                let bytes = fs::read(&path).expect("the file is there");
                assert!(!bytes.contains(&0u8), "the file {} holds a NUL byte", path.display());
            }
        }
    }
}
