//! The files many changes touch, and the lines every crate has to share.
//!
//! # The problem
//!
//! Four things in this repository are edited by almost every change:
//!
//! 1. the deny line in every crate root,
//! 2. the layering table,
//! 3. the layering table *as rendered in the documentation*,
//! 4. the sorted lists — workspace members, `mod` lines.
//!
//! These are the files where two people working at once collide for no good
//! reason, and where a merge conflict gets resolved by taking the wrong side.
//! The fix is not to stop editing them; it is to make them *mechanical*, so a
//! change that gets one of them wrong fails a gate instead of quietly
//! disagreeing with the others.
//!
//! # What is checked
//!
//! - Every crate root carries [`DENY`] and [`FORBID`] byte-for-byte. Not "has
//!   a deny attribute": *the same one*, because a deny list that varies by crate
//!   is a deny list nobody can reason about.
//! - The layering table in [`layers::TABLE`] and the table in
//!   `docs/architecture.md` name the same crates in the same order.
//! - That documentation table's "depends on" column agrees with
//!   `cargo metadata`. Documentation that drifts from the code is worse than no
//!   documentation, because it is believed.
//! - The workspace member list and every `mod` group are sorted.
//!
//! No `--fix` here, deliberately. Every one of these problems has a correct
//! answer that is not mechanical: a deny attribute with the wrong lints needs a
//! decision about which lints, and a documentation dependency column has to be
//! recomputed from cargo and checked by a person who knows whether the new
//! dependency is right. So the report prints what to paste and names what it
//! found, and the edit stays a human one.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::layers::{self, TABLE};
use crate::step;

/// The deny list, in house order.
///
/// `unimplemented, todo, unreachable` last, in that order, as the crate family
/// this was distilled from writes them. One list, so there is nothing to choose
/// between.
pub const LINTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::unimplemented",
    "clippy::todo",
    "clippy::unreachable",
];

/// The deny attribute, exactly as `cargo fmt` leaves it.
///
/// Multi-line, and that is not a style choice: the single line is 112
/// characters and `rustfmt.toml` allows 110, so rustfmt reflows it. A gate that
/// insisted on the one-line form would be fighting the formatter, and the
/// formatter wins that argument every time. This is what a pasted-in block
/// becomes after a format, so it is what the report shows.
pub const DENY: &str = concat!(
    "#![deny(\n",
    "    clippy::unwrap_used,\n",
    "    clippy::expect_used,\n",
    "    clippy::panic,\n",
    "    clippy::unimplemented,\n",
    "    clippy::todo,\n",
    "    clippy::unreachable\n",
    ")]"
);

/// The unsafe line, exactly as every crate root must carry it.
pub const FORBID: &str = "#![forbid(unsafe_code)]";

/// The deny attribute of a file, with every space removed and the `clippy::`
/// prefix dropped, for comparison against [`canonical`].
///
/// Whitespace is ignored and nothing else is: a crate root that denies a
/// different set of lints, or the same set in a different order, is a different
/// rule, and this is where that is caught. The attribute is extracted whole
/// first, so an unrelated `deny(` further down the file cannot stand in for it.
pub fn normalise(body: &str) -> Option<String> {
    let start = body.find("#![deny(")?;
    let rest = &body[start + "#![deny(".len()..];
    let end = rest.find(")]")?;
    Some(strip(&rest[..end]))
}

/// [`DENY`] in the same normalised form as [`normalise`] produces.
pub fn canonical() -> String {
    let inner = DENY.strip_prefix("#![deny(").and_then(|rest| rest.strip_suffix(")]")).unwrap_or(DENY);
    strip(inner)
}

fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase().replace("clippy::", "")
}

/// Where the layering table is documented.
pub const ARCHITECTURE: &str = "docs/architecture.md";

/// The areas that own commands, so a dotted name with one of these prefixes is
/// a command id and must exist.
///
/// Deliberately a prefix list rather than "anything dotted": the prose is full
/// of filenames (`format.rs`, `deny.toml`), control-channel methods
/// (`engine.execute`), and property paths (`p.bool`), and a check that flagged
/// those would be noise dressed as a gate.
const COMMAND_AREAS: &[&str] =
    &["doc", "node", "prop", "key", "playback", "history", "diagnostics", "plan", "session"];

/// Dotted names in prose that look like commands but are not, each with why.
///
/// An allowlist rather than a pattern, because every row is a decision somebody
/// made and the reason is what stops the next person wondering.
const NOT_COMMANDS: &[(&str, &str)] = &[
    ("session.state", "a control-channel method, not a command"),
    ("session.execute", "`Session::execute`, the Rust method every channel calls"),
    ("diagnostics.crash_reports", "a control-channel method, not a command"),
];

/// A command id that one specific document may name without it existing.
///
/// This gate cannot tell prose that *describes* a command from prose that
/// *proposes* one, and `docs/agent-playbook.md` is a worked example of adding
/// this very command. Keying the exemption to the **file** rather than to the
/// id is the whole point: an id exempt everywhere would also be exempt in
/// `docs/recipes/add-a-command.md`, which is exactly where a fabricated command
/// does real damage. Narrow enough that any other invented id in the playbook
/// is still caught, and inert once the command really exists.
const PROPOSED_IN: &[(&str, &str)] = &[
    ("node.enable", "docs/agent-playbook.md"),
    // The same worked example, from the other end: this recipe is how you add
    // a command, so it names the one being added.
    ("node.enable", "docs/recipes/add-a-command.md"),
];

/// Every `` `<area>.<name>` `` in prose that claims to be a command.
///
/// Matches the backticked dotted form only. An unquoted name in a sentence is
/// not a claim that can be checked, and pretending otherwise would make this
/// gate fail on ordinary English.
pub fn command_ids_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        let token = &after[..end];
        let Some((area, name)) = token.split_once('.') else {
            rest = &after[end + 1..];
            continue;
        };
        let shaped = !name.is_empty()
            && name.chars().all(|c| c.is_ascii_lowercase() || c == '.' || c.is_ascii_digit());
        if shaped && COMMAND_AREAS.contains(&area) {
            found.push(token.to_string());
        }
        rest = &after[end + 1..];
    }
    found.sort();
    found.dedup();
    found
}

/// The boundaries that catch a panic, and the document that declares them.
///
/// The pattern document says: **two boundaries, with distinct jobs, and no
/// more** — and that a `catch_unwind` anywhere else is a bug in the design.
/// §78: the tree had three, and the document listed two. The third — the
/// control channel's `ui.*` handler — was argued for in the code and missing
/// from the document; the fourth — the render path's — arrived with §76, and
/// the document was not re-read.
///
/// The check is that every file with a `catch_unwind` in it is a *declared*
/// boundary, and that every declared boundary is named in the document. A
/// fifth boundary has to be written down before it is written.
pub fn panic_boundaries_are_declared(root: &Path) -> Result<Vec<String>, String> {
    /// The files that catch a panic, and the document that says so.
    const DECLARED: &[&str] = &[
        "crates/kernel/src/lib.rs",
        "crates/engine/src/session.rs",
        "crates/control/src/server.rs",
        "apps/craftkit/src/main.rs",
    ];
    const DOCUMENT: &str = "docs/patterns/one-trust-boundary.md";

    let mut problems = Vec::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // The tests are not the production code: a test may catch a panic to
        // assert that it was caught, and that is not a boundary. The cut has
        // to reach a **top-level** test item as well as a test module, or a
        // test written outside a module is counted as a boundary. §153.
        let production = production_of(&text);
        if !code_lines_kept(&production).contains("catch_unwind(") {
            continue;
        }
        let relative = relative_name(root, &path);
        if !DECLARED.contains(&relative.as_str()) {
            problems.push(format!(
                "{relative}: catches a panic, and the pattern document does not declare it a boundary"
            ));
        }
    }
    let document = std::fs::read_to_string(root.join(DOCUMENT))
        .map_err(|e| format!("could not read {DOCUMENT}: {e}"))?;
    for file in DECLARED {
        if !document.contains(file) {
            problems.push(format!("{DOCUMENT}: does not mention {file}, which catches a panic"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Every command id the shipped registry defines.
///
/// Read from the source rather than by linking `craftkit-engine`, for the same
/// reason this crate depends on nothing but `serde`: the build tool should not
/// have to be the thing it builds. `citation_drift` already reads Rust files
/// for the same reason.
///
/// Test-only commands live in the same files and are not in the shipped
/// catalogue, so they are not names prose may promise — `test.panic` is
/// `#[cfg(test)]`, and asking a user to run it would be asking for a refusal.
pub fn registry_command_ids(root: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let mut ids = std::collections::BTreeSet::new();
    for rel in ["crates/engine/src/commands.rs", "crates/engine/src/command.rs"] {
        let path = root.join(rel);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("id: \"") else { continue };
            let Some(id) = rest.split('"').next() else { continue };
            if id.starts_with("test.") {
                continue;
            }
            ids.insert(id.to_string());
        }
    }
    Ok(ids)
}

/// A number of commands in prose is the registry's number.
///
/// §71's lesson: **a number written by hand about a thing that grows is
/// stale by construction.** The registry grows — every command added makes
/// "21 commands" wrong — and the two documents that say it were both written
/// by hand, with nothing to notice when the count changed. §77.
///
/// The areas are the same: the `Area` enum's variants *are* the areas, so
/// "7 areas" is a number about a thing that grows too.
///
/// The number has to be written in digits. A number in words — "seven
/// areas" — is invisible to this check, which is a hole and is written down
/// as one; the prose uses digits so the check can see it.
pub fn counts_in_prose_match_the_code(root: &Path) -> Result<Vec<String>, String> {
    let commands = registry_command_ids(root)?.len();
    let areas = area_variants(root)?;
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (at, claimed, noun) in counts_in(&text) {
            let actual = match noun {
                "commands" => commands,
                "areas" => areas,
                _ => continue,
            };
            if claimed != actual {
                let line = text[..at].bytes().filter(|b| *b == b'\n').count() + 1;
                problems.push(format!(
                    "{relative}: line {line} says there are {claimed} {noun}, and there are {actual}"
                ));
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// The areas the registry has: the variants of `Area`.
///
/// Read from the source for the same reason the command ids are: the build
/// tool should not have to be the thing it builds.
fn area_variants(root: &Path) -> Result<usize, String> {
    let path = root.join("crates/engine/src/command.rs");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let Some(start) = text.find("pub enum Area {") else {
        return Err("the engine has no `Area` enum".to_string());
    };
    let body = &text[start..];
    let end = body.find("\n}").unwrap_or(body.len());
    let variants = body[..end]
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#[") {
                return None;
            }
            let name = trimmed.trim_end_matches(',');
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return None;
            }
            Some(name.to_string())
        })
        .count();
    Ok(variants)
}

/// The numbers in a text that count something: (where the number starts, the
/// number, what it counts).
///
/// Hand-rolled, like the rest of this file: the tree takes no dependencies
/// here, and a count in prose is a pattern simple enough to read.
fn counts_in(text: &str) -> Vec<(usize, usize, &'static str)> {
    let mut found = Vec::new();
    for (noun, kind) in [(" commands", "commands"), (" areas", "areas")] {
        let mut at = 0usize;
        while let Some(found_at) = text[at..].find(noun) {
            let number_at = at + found_at;
            at = number_at + noun.len();
            let bytes = text.as_bytes();
            let mut end = number_at;
            while end > 0 && bytes[end - 1].is_ascii_digit() {
                end -= 1;
            }
            if end == number_at {
                continue;
            }
            // A number that is part of a longer token — a version, a file
            // name — is not a count of things.
            if end > 0 && !bytes[end - 1].is_ascii_whitespace() {
                continue;
            }
            let Ok(number) = text[end..number_at].parse::<usize>() else { continue };
            found.push((end, number, kind));
        }
    }
    found
}

/// Dotted names in prose that name a command the registry does not have.
///
/// `docs/recipes/add-a-command.md` listed `node.enable` among the live
/// `Area::Node` entries. It has never existed — the nearest real command is
/// `node.mute` — and a recipe is exactly the document somebody trusts when they
/// are about to type a command id into a file.
pub fn unknown_command_ids(root: &Path) -> Result<Vec<String>, String> {
    let known = registry_command_ids(root)?;
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for id in command_ids_in(&text) {
            let proposed_here = PROPOSED_IN.iter().any(|(n, file)| *n == id && *file == relative);
            if known.contains(&id) || proposed_here {
                continue;
            }
            if NOT_COMMANDS.iter().any(|(n, _)| *n == id) {
                continue;
            }
            problems
                .push(format!("{relative}: `{id}` is named as a command, but the registry has no such id"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Control-channel requests written in prose that could not be sent.
///
/// `docs/agent-playbook.md` and `skills/craftkit-checking/SKILL.md` both showed
/// `{"method":"ui.render","params":{"action":"click"}}` for years. That spelling
/// worked only by accident — the host read `params.action` and ignored the
/// method the caller had typed — and once `ui.click` was honoured (§32) the
/// accident stopped being load-bearing. Prose is not data, so an example can rot
/// silently; this makes the rot a gate failure.
///
/// What it can check is that each request-shaped line is JSON, names a method
/// [`craftkit_control::Method`] accepts, and does not contradict itself. Whether
/// the example is a *sensible* request is a judgement about prose and is left
/// alone, the same way [`citation_drift`] leaves a citation that lands on real
/// code alone.
pub fn prose_examples_in_docs_resolve_to_real_methods(root: &Path) -> Result<Vec<String>, String> {
    let (methods, actions) = wire_names(root)?;
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for problem in request_example_problems(&text, &methods, &actions) {
            problems.push(format!("{relative}: {problem}"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The methods and ui actions the control channel accepts, read from its source.
///
/// Not a constant here. `Method::parse` and `UiAction::parse` are the only
/// definitions of what exists, and a second list is how §22 happened: two
/// implementations of one rule that could disagree. So this reads
/// `crates/control/src/protocol.rs` and takes the names from the match arms.
///
/// # Errors
///
/// If the scan finds no arms at all, it says so rather than checking against an
/// empty set and finding nothing wrong — the failure mode of a parser that
/// matches nothing is a gate that passes because it looked at nothing, and that
/// is the defect this repository keeps writing down.
pub fn wire_names(root: &Path) -> Result<(Vec<String>, Vec<String>), String> {
    let path = root.join("crates/control/src/protocol.rs");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;

    let methods = arms_named(&text, "Method::");
    let actions = arms_named(&text, "UiAction::");
    if methods.is_empty() {
        return Err(format!(
            "{}: found no `Method::` match arms; the scan is wrong, not the protocol",
            path.display()
        ));
    }
    if actions.is_empty() {
        return Err(format!(
            "{}: found no `UiAction::` match arms; the scan is wrong, not the protocol",
            path.display()
        ));
    }
    Ok((methods, actions))
}

/// Every `"name" => Something::…` arm in `text`.
///
/// Hand-rolled, for the reason [`citations`] gives: the pattern is small, and a
/// dependency here would be the only one the build tool has that is not already
/// in the tree. It reads the arms of the two `parse` functions, which is where
/// the names are defined.
fn arms_named(text: &str, marker: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        // Only arms, not the function signature or a `=> return None` arm.
        if !trimmed.starts_with('"') {
            continue;
        }
        let Some(arrow) = line.find("=>") else { continue };
        if !line[arrow..].contains(marker) {
            continue;
        }
        // `ui" | "ui.click" | "ui.render" => Method::Ui` — one line, several names.
        for part in line.split("=>").next().unwrap_or_default().split('|') {
            let name = part.trim().trim_matches('"').trim();
            if !name.is_empty() && !name.contains(char::is_whitespace) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// The problems in one document's request examples.
///
/// Pure: it takes text and returns sentences, so the rules can be tested against
/// examples written here rather than by editing a document to see whether the
/// gate notices. `methods` and `actions` come from [`wire_names`].
pub fn request_example_problems(text: &str, methods: &[String], actions: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let Some(raw) = request_shaped(line) else { continue };
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(e) => {
                problems.push(format!(
                    "line {}: looks like a control-channel request but is not JSON ({e}): {raw}",
                    number + 1
                ));
                continue;
            }
        };
        let method = value.get("method").and_then(serde_json::Value::as_str).unwrap_or_default();
        if !methods.iter().any(|m| m == method) {
            problems.push(format!(
                "line {}: `{method}` is not a control-channel method; try {}",
                number + 1,
                methods.join(", ")
            ));
            continue;
        }
        // Only `ui.*` has a second place to name an action, and the two must agree.
        let Some(from_method) = ui_action_of(method) else { continue };
        let Some(asked) = value.get("params").and_then(|p| p.get("action")) else { continue };
        let Some(asked) = asked.as_str() else {
            problems.push(format!(
                "line {}: `action` must be a string naming one of {}, not {}",
                number + 1,
                actions.join(", "),
                kind_of(asked)
            ));
            continue;
        };
        if !actions.iter().any(|a| a == asked) {
            problems.push(format!(
                "line {}: `action: {asked:?}` is not a ui action; try {}",
                number + 1,
                actions.join(", ")
            ));
            continue;
        }
        match from_method {
            Some(expected) if expected != asked => problems.push(format!(
                "line {}: `{method}` asks to {expected} and `action` asks to {asked}; \
                 the channel refuses that, so this example cannot be sent",
                number + 1
            )),
            // The bare `ui` decides by its action, which is what it is for.
            None | Some(_) => {}
        }
    }
    problems
}

/// What a `ui.*` method means, or `None` for the bare `ui`.
///
/// The four that mean something of their own. `ui` is in `wire_names` but names
/// no action, which is why it is matched here and returns `None` rather than
/// being absent.
fn ui_action_of(method: &str) -> Option<Option<&'static str>> {
    Some(match method {
        "ui.render" => Some("render"),
        "ui.screenshot" => Some("screenshot"),
        "ui.click" => Some("click"),
        "ui" => None,
        _ => return None,
    })
}

/// A JSON value described by its kind.
fn kind_of(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// The JSON object on `line`, if there is one, with any `"token"` blanked.
///
/// A document shows the token as `…`, which is not a usable one, and the token is
/// not what this check is about. Blanking it keeps the line parseable and keeps
/// the check from caring about a secret.
fn request_shaped(line: &str) -> Option<String> {
    let start = line.find('{')?;
    if !line[start..].contains("\"method\"") {
        return None;
    }
    let end = line.rfind('}')?;
    if end < start {
        return None;
    }
    Some(blend_token(&line[start..=end]))
}

/// `{"token":"…", …}` with the value replaced, by hand: no regex in this tree.
fn blend_token(json: &str) -> String {
    let Some(at) = json.find("\"token\"") else { return json.to_string() };
    let after = &json[at + "\"token\"".len()..];
    let Some(colon) = after.find(':') else { return json.to_string() };
    let value_at = at + "\"token\"".len() + colon + 1;
    let rest = &json[value_at..];
    let trimmed = rest.trim_start();
    let lead = rest.len() - trimmed.len();
    let Some(quote) = trimmed.strip_prefix('"') else { return json.to_string() };
    let Some(end_quote) = quote.find('"') else { return json.to_string() };
    let value_end = value_at + lead + 1 + end_quote + 1;
    format!("{}\"…\"{}", &json[..value_at], &json[value_end..])
}

/// Where prose is allowed to point at code.
const PROSE: &[&str] = &["docs", "skills", "AGENTS.md", "README.md", "CONTRIBUTING.md"];

/// References from one document to another that do not resolve.
///
/// `AGENTS.md` told every agent reading it that "`docs/control-protocol.md` has
/// the wire format". That file has never existed. `citation_drift` below checks
/// `file.rs:12` and nothing else, so a document can point at a missing *document*
/// with no gate noticing — which is the same defect in the same place, wearing a
/// different extension.
///
/// A reference is `` `something.md` `` in prose: backticked, rooted or relative.
/// It resolves if it exists next to the referring document *or* from the
/// repository root, because both are how a reader tries one.
pub fn broken_document_links(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, target) in document_links(&text) {
            let beside = doc.parent().map(|d| d.join(&target));
            let from_root = root.join(&target);
            if in_the_tree(beside.as_deref()) || in_the_tree(Some(&from_root)) {
                continue;
            }
            problems
                .push(format!("{relative}: line {number} points at `{target}`, which is not in the tree"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Whether a path is a file in the tree.
///
/// One question, asked one way. An earlier version of this also asked the
/// *directory* what names it held, on the belief that `stat` was answering
/// "no such file" for a document that demonstrably existed. It was not: the
/// document did not exist, under the name that was being asked for. The gate
/// was right and the belief was wrong, so the fallback went — a name the
/// directory lists is a name that exists, and a directory that happens to be
/// called `notes.md` is not a document.
fn in_the_tree(path: Option<&std::path::Path>) -> bool {
    path.is_some_and(|p| p.is_file())
}

/// Every `` `name.md` `` in `text`, as (line number, name).
///
/// Hand-rolled for the same reason [`citations`] is: no dependency in the build
/// tool that the tree does not already have. Only backticked names ending in
/// `.md` count, so a URL, a bare word and a fenced example are all left alone.
pub fn document_links(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let name = &after[..close];
            let looks_like_a_path =
                name.ends_with(".md") && !name.contains(' ') && !name.starts_with("http") && name.len() > 3;
            if looks_like_a_path {
                found.push((number + 1, name.to_string()));
            }
            rest = &after[close + 1..];
        }
    }
    found
}

/// A symbol that prose says is in a file, and is not.
///
/// §29 swept the backticked names in `skills/` against the source and found
/// three places naming `pending_list_is_sorted` when the test is called
/// `the_pending_list_is_sorted`. It deliberately did not gate that, because the
/// rule that separates a test name from `catch_unwind` is a heuristic that
/// eventually flags something real.
///
/// This is the part that *can* be gated, because it is anchored: when prose says
/// **"the `X` test in `path.rs`"**, the file is named, and the file can be
/// checked. Eleven such claims exist in the tree and **two were false** — a
/// recipe naming `every_command_has_an_icon_it_can_draw` when the test is
/// `every_command_has_an_icon_this_set_can_draw`, in the same sentence as the
/// correct name.
///
/// The rule: a backticked symbol immediately followed by a backticked `*.rs`
/// path is a claim that the symbol is in that file. "Immediately" means the
/// next backticked thing, so `` `a` and `b` in `f.rs` `` pairs `b` with `f.rs`
/// and leaves `a` alone.
pub fn symbol_file_claims(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, symbol, file) in symbol_claims(&text) {
            let path = root.join(&file);
            if !path.is_file() {
                // A file that is not there is the document-links gate's business.
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else { continue };
            if defines(&source, &symbol) {
                continue;
            }
            problems.push(format!("{relative}: line {number} says `{symbol}` is in `{file}`, and it is not"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The claims one document makes, as (line number, symbol, file).
///
/// The backticked spans of the text, in order. A symbol that looks like a test
/// name and is *immediately* followed by a `*.rs` path is a claim about that
/// file.
pub fn symbol_claims(text: &str) -> Vec<(usize, String, String)> {
    let spans = backticked(text);
    let mut claims = Vec::new();
    for pair in spans.windows(2) {
        let (_, symbol_end, symbol) = &pair[0];
        let (file_start, _, file) = &pair[1];
        // The file may carry a line number — `crates/engine/src/session.rs:359`
        // is how most of these are written — and the number is not part of the
        // name of the file. The first version of this rule only matched a bare
        // path and so missed every citation of that shape, which is the shape
        // the documents actually use.
        let file = file.split(':').next().unwrap_or(file);
        if !looks_like_a_test_name(symbol) || !file.ends_with(".rs") {
            continue;
        }
        // The claim is *about* that file, so the two have to be in the same
        // breath: no blank line between them and nothing much in between. The
        // first version of this rule crossed a paragraph boundary and paired a
        // test with a file named three sections later.
        //
        // The longest gap in a real claim is ten characters — ", in " and
        // " test in " — so forty is generous and still says "the same
        // sentence".
        let between = &text[*symbol_end..*file_start];
        if between.contains("\n\n") || between.len() > 40 {
            continue;
        }
        let line = text[..pair[0].0].matches('\n').count() + 1;
        claims.push((line, symbol.clone(), file.to_string()));
    }
    claims
}

/// Every backticked span of `text`, as (start, end, content).
fn backticked(text: &str) -> Vec<(usize, usize, String)> {
    let mut spans = Vec::new();
    let mut at = 0usize;
    while let Some(open) = text[at..].find('`') {
        let start = at + open;
        let after = start + 1;
        let Some(close) = text[after..].find('`') else { break };
        spans.push((start, after + close, text[after..after + close].to_string()));
        at = after + close + 1;
    }
    spans
}

/// Whether a name is shaped like one of this tree's tests.
///
/// The four prefixes the tests use. Deliberately narrow: `catch_unwind` and
/// `unwrap_used` are not test names, and §29's warning about heuristics is
/// exactly why they are not in here.
fn looks_like_a_test_name(name: &str) -> bool {
    name.contains('_')
        && (name.starts_with("every_")
            || name.starts_with("the_")
            || name.starts_with("a_")
            || name.starts_with("an_"))
}

/// Whether `source` defines `name`.
///
/// Any of the ways this tree defines things, so the check is about the name
/// and not about how it is spelled.
///
/// The **code** of the source, not its text: a test that stands only inside a
/// string literal or a comment is not a test the tree has, and a document
/// that claims it is there is claiming something false. §149.
pub fn defines(source: &str, name: &str) -> bool {
    let code = code_lines_kept(source);
    for keyword in ["fn ", "struct ", "enum ", "const ", "static ", "mod ", "trait ", "type "] {
        if code.contains(&format!("{keyword}{name}")) {
            return true;
        }
    }
    false
}

/// The roots of the standard library and of the tools, which prose names in
/// the same shape as a module of this tree.
///
/// `std::fs` and `clippy::unwrap_used` are not claims about this tree, and a
/// check that flagged them would be noise dressed as a gate.
const FOREIGN_ROOTS: &[&str] = &["std", "core", "alloc", "proc_macro", "clippy", "cargo", "rustc"];

/// The primitive types, which are lower case and so look like modules.
///
/// `i64::MIN` is a constant of a primitive, not of a module this tree owns.
const PRIMITIVE_TYPES: &[&str] = &[
    "bool", "char", "str", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
    "u64", "u128", "usize",
];

/// A module-shaped name that one specific document may name without it existing.
///
/// The same shape as the command ids' exemption: a recipe may propose names
/// that do not exist yet, and the exemption is keyed to the **file**, so an
/// invented name anywhere else is still caught.
const PROPOSED_MODULES: &[(&str, &str)] =
    &[("v1", "docs/recipes/change-the-file-format.md"), ("v2", "docs/recipes/change-the-file-format.md")];

/// The fields inside a dependency's braces, which are not crates.
///
/// `version = "1"` and `features = [ … ]` are part of a dependency's
/// *specification*, not names of crates, and a multi-line feature list puts
/// them on lines of their own.
const DEPENDENCY_FIELDS: &[&str] =
    &["version", "features", "default-features", "package", "path", "workspace", "optional"];

/// A module-shaped dotted name in prose that names nothing this tree has.
///
/// §130: `AGENTS.md` — the file every agent reads first — told them that any
/// new canvas calls `navigation::handle`. This tree has no canvas, no camera
/// and no `navigation` module: the rule names a module that was never built
/// here, and an agent that follows it looks for something that is not there.
///
/// The two checks that could have seen it do not. `symbol_file_claims` needs
/// the symbol to be followed by a `*.rs` path; the document-links check needs
/// a `.md`; this one is neither.
///
/// The rule: a backticked `a::b` whose first part is **module-shaped** — lower
/// case, so a module rather than a type — must name a crate this tree uses or
/// a module this tree declares. A type path (`Session::execute`) is not
/// module-shaped and is not this check's business.
pub fn unknown_module_paths(root: &Path) -> Result<Vec<String>, String> {
    let modules = tree_modules(root);
    let crates = known_crate_roots(root);
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (start, _, token) in backticked(&text) {
            let Some(head) = module_path_head(&token) else { continue };
            if is_known_module_root(head, &modules, &crates) {
                continue;
            }
            if PROPOSED_MODULES.iter().any(|(name, file)| *name == head && *file == relative) {
                continue;
            }
            let line = text[..start].matches('\n').count() + 1;
            problems.push(format!(
                "{relative}: line {line} says `{token}`, and `{head}` is not a module or a crate \
                 this tree has"
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The head of a dotted name that claims to be a module of this tree.
///
/// `Some("navigation")` for `navigation::handle`; `None` for a type path
/// (`Session::execute` — a type, not a module), for a primitive
/// (`i64::MIN` — a constant of a primitive), for a file path, and for
/// anything that is not a dotted name at all.
pub fn module_path_head(token: &str) -> Option<&str> {
    let (head, rest) = token.split_once("::")?;
    if rest.is_empty() || rest.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    if !head.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        return None;
    }
    if !head.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return None;
    }
    Some(head)
}

/// Whether `head` names something this tree has.
///
/// A module of the tree, a crate the tree uses, the standard library, a tool,
/// or a primitive type. The last two are not claims about this tree, and a
/// check that flagged them would be noise dressed as a gate.
fn is_known_module_root(
    head: &str,
    modules: &std::collections::BTreeSet<String>,
    crates: &std::collections::BTreeSet<String>,
) -> bool {
    modules.contains(head)
        || crates.contains(head)
        || FOREIGN_ROOTS.contains(&head)
        || PRIMITIVE_TYPES.contains(&head)
}

/// The modules this tree declares, by name.
///
/// Every `mod` in every Rust file, and every `pub use` of one: the tree's
/// modules are file modules, so the name is what a reader of the prose would
/// search for.
///
/// The **code** of each file, not its text: a `mod` line that stands only
/// inside a string literal is not a module the tree has, and a document that
/// names it is naming something that is not there. §149.
fn tree_modules(root: &Path) -> std::collections::BTreeSet<String> {
    let mut modules = std::collections::BTreeSet::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let code = code_lines_kept(&text);
        for line in code.lines() {
            let trimmed = line.trim();
            for keyword in ["mod ", "pub mod ", "pub(crate) mod "] {
                let Some(rest) = trimmed.strip_prefix(keyword) else { continue };
                let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() {
                    modules.insert(name);
                }
            }
        }
    }
    modules
}

/// The crates this tree uses: its own, and the ones it depends on.
///
/// Read from the manifests rather than from `cargo metadata`, for the same
/// reason `citation_drift` reads the Rust files: the build tool should not
/// have to be the thing it builds. A crate is named the way Rust names it,
/// with underscores.
fn known_crate_roots(root: &Path) -> std::collections::BTreeSet<String> {
    let mut names = std::collections::BTreeSet::new();
    let mut manifests = Vec::new();
    collect_manifests(root, &mut manifests);
    for manifest in manifests {
        let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
        let mut section = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                section = trimmed.trim_matches(|c| c == '[' || c == ']').to_string();
                continue;
            }
            let Some((key, value)) = trimmed.split_once('=') else { continue };
            let key = key.trim();
            if section == "package" && key == "name" {
                names.insert(value.trim().trim_matches('"').replace('-', "_"));
            } else if section.ends_with("dependencies") {
                // A dependency is `name = { … }` or `name = "1"`. The fields
                // *inside* the braces are not crates, and one of them — a
                // `features` list — can be on a line of its own.
                let spec = value.trim();
                let is_a_spec = spec.starts_with('{')
                    || spec.starts_with('"')
                    || spec.starts_with(|c: char| c.is_ascii_digit());
                if is_a_spec && !DEPENDENCY_FIELDS.contains(&key) {
                    names.insert(key.split('.').next().unwrap_or(key).replace('-', "_"));
                }
            }
        }
    }
    names
}

/// The methods a derived trait puts on a type.
///
/// `#[derive(Clone)]` gives a type a `clone`, and prose that says
/// `Document::clone` names something that exists. Without this table every
/// mention of a derived method would be a false positive, and a gate that
/// cries wolf is a gate nobody runs.
const DERIVED_MEMBERS: &[(&str, &[&str])] = &[
    ("Clone", &["clone"]),
    ("Default", &["default"]),
    ("Debug", &["debug"]),
    ("PartialEq", &["eq", "ne"]),
    ("PartialOrd", &["partial_eq", "partial_cmp"]),
    ("Ord", &["cmp", "partial_eq", "partial_cmp"]),
    ("Hash", &["hash"]),
    ("From", &["from"]),
    ("Into", &["into"]),
    ("AsRef", &["as_ref"]),
    ("Borrow", &["borrow"]),
    ("Serialize", &["serialize"]),
    ("Deserialize", &["deserialize"]),
    ("Display", &["fmt", "to_string"]),
    ("ToString", &["to_string"]),
];

/// A type path in prose that names a member the type does not have.
///
/// The twin of [`unknown_module_paths`]. §130 checked `a::b` — a path whose
/// head is a **module** — and said, in so many words, that a *type* path
/// (`Session::execute`) was "not this check's business". §143 is that
/// business.
///
/// The defect it finds: `docs/recipes/add-a-control-channel-method.md` told an
/// agent to add a `Method::Render` variant to the control channel. The tree
/// has no such variant, and `ui.render` — the spelling the recipe was adding
/// — already exists and deliberately reaches `Method::Ui`. An agent that
/// follows the recipe splits the `ui.*` family, and `ui.render` stops
/// reaching the host that draws the frame.
///
/// The rule: a `Type::member` in prose, where `Type` is a type **this tree
/// defines**, must name a member that type has — a variant, a field, an item
/// of an `impl` block, or a method a derived trait provides. A path whose
/// head is another crate's type (`serde_json::Value`) is not this check's
/// business, and neither is a module's path: that is §130's.
pub fn unknown_type_paths(root: &Path) -> Result<Vec<String>, String> {
    let types = tree_types(root);
    let used = type_paths_in_code(root);
    let own: BTreeSet<String> = TABLE.iter().map(|(name, _)| (*name).to_string()).collect();
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (line, ty, member, qualifier) in type_paths(&text) {
            // A path into somebody else's crate is not this tree's business:
            // `serde_json::Value::Number` names a variant of *serde's* value,
            // and the tree's own `Value` has nothing to do with it.
            if let Some(qualifier) = qualifier
                && !own.contains(&path_root(&qualifier))
            {
                continue;
            }
            let Some(members) = types.get(&ty) else { continue };
            if members.contains(&member) || used.contains(&format!("{ty}::{member}")) {
                continue;
            }
            problems.push(format!(
                "{relative}: line {line} names `{ty}::{member}`, and {ty} has no member of that name"
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The first segment of a dotted path: the crate it starts in.
fn path_root(path: &str) -> String {
    path.split("::").next().unwrap_or(path).to_string()
}

/// The types this tree defines, and the members each one has.
///
/// A member is an enum variant, a struct field, an item of an `impl` block, or
/// a method a derived trait provides. Visibility is **not** part of it: a
/// document about the tree's own internals may name a `pub(crate)` field, and
/// a gate that flagged those would be flagging the documents that are most
/// precise about how the tree works.
fn tree_types(root: &Path) -> std::collections::BTreeMap<String, BTreeSet<String>> {
    let mut types: std::collections::BTreeMap<String, BTreeSet<String>> = std::collections::BTreeMap::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        collect_types_in(&code_lines_kept(&text), &mut types);
    }
    types
}

/// Every `Type::member` the tree's own code writes, as `Type::member`.
///
/// A path the code writes is a path that exists — including the ones a
/// derived trait provides and the ones a trait implementation supplies.
fn type_paths_in_code(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // The strings and the comments do not count: a doc comment that says
        // `Method::Render` is describing something that is not there.
        let code = code_lines_kept(&text);
        for (_, ty, member, _) in type_paths(&code) {
            found.insert(format!("{ty}::{member}"));
        }
    }
    found
}

/// Every `Type::member` in `text`, as (line, type, member, qualifier).
///
/// The head must be an upper-case identifier — a tree type — and the member is
/// the identifier after the `::`. A path whose head is lower case is a
/// module's, and [`unknown_module_paths`] has it.
///
/// The **qualifier** is whatever came before the type: `Some("serde_json")`
/// for `serde_json::Value::Number`, and `None` for a bare `Document::new`.
/// It is what tells the tree's `Value` from serde's.
fn type_paths(text: &str) -> Vec<(usize, String, String, Option<String>)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(rel) = text[at..].find("::") {
        let separator = at + rel;
        let mut head = separator;
        while head > 0 && is_word_byte(bytes[head - 1]) {
            head -= 1;
        }
        let mut tail = separator + 2;
        while tail < bytes.len() && is_word_byte(bytes[tail]) {
            tail += 1;
        }
        let ty = &text[head..separator];
        let member = &text[separator + 2..tail];
        let head_is_a_type = ty.chars().next().is_some_and(|c| c.is_ascii_uppercase());
        let member_is_a_name = member.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if head_is_a_type && member_is_a_name {
            let line = text[..head].matches('\n').count() + 1;
            let qualifier = qualifier_before(text, head);
            found.push((line, ty.to_string(), member.to_string(), qualifier));
        }
        at = separator + 2;
    }
    found
}

/// The path in front of a type, when there is one.
///
/// `serde_json::Value` is qualified by `serde_json`; a bare `Document` is not
/// qualified at all.
fn qualifier_before(text: &str, head: usize) -> Option<String> {
    let before = text[..head].trim_end();
    let before = before.strip_suffix("::")?;
    let bytes = before.as_bytes();
    let mut start = before.len();
    while start > 0 && is_word_byte(bytes[start - 1]) {
        start -= 1;
    }
    if start == before.len() {
        return None;
    }
    Some(before[start..].to_string())
}

/// The types and members in one file of code.
///
/// The code has been stripped of its strings and comments already, so the
/// braces in it are the braces that structure it.
fn collect_types_in(code: &str, types: &mut std::collections::BTreeMap<String, BTreeSet<String>>) {
    let bytes = code.as_bytes();
    let mut at = 0usize;
    let mut derives: Vec<String> = Vec::new();
    while at < bytes.len() {
        if !is_word_byte(bytes[at]) || bytes[at].is_ascii_digit() {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && is_word_byte(bytes[at]) {
            at += 1;
        }
        let word = &code[start..at];
        match word {
            "derive" => derives = derive_traits(code, at),
            "struct" | "enum" | "trait" => {
                let Some(name) = next_identifier(code, at) else { continue };
                let members = types.entry(name.to_string()).or_default();
                for (trait_name, methods) in DERIVED_MEMBERS {
                    if derives.iter().any(|d| d == trait_name) {
                        for method in *methods {
                            members.insert((*method).to_string());
                        }
                    }
                }
                derives.clear();
                if let Some(body) = braced_body(code, at) {
                    collect_members(&body, word, members);
                }
            }
            "impl" => {
                let Some((name, body_at)) = impl_self_type(code, at) else { continue };
                let members = types.entry(name).or_default();
                if let Some(body) = braced_body(code, body_at) {
                    collect_items(&body, members);
                }
            }
            _ => {}
        }
    }
}

/// The traits in a `#[derive(…)]`, as names.
fn derive_traits(code: &str, at: usize) -> Vec<String> {
    let bytes = code.as_bytes();
    let mut i = at;
    while i < bytes.len() && bytes[i] != b'(' {
        if bytes[i] == b']' {
            return Vec::new();
        }
        i += 1;
    }
    if i >= bytes.len() {
        return Vec::new();
    }
    let end = code[i..].find(')').map(|n| i + n).unwrap_or(code.len());
    code[i + 1..end].split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
}

/// The identifier that starts at or after `at`.
fn next_identifier(code: &str, at: usize) -> Option<&str> {
    let bytes = code.as_bytes();
    let mut i = at;
    while i < bytes.len() && !is_word_byte(bytes[i]) {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && is_word_byte(bytes[i]) {
        i += 1;
    }
    if start == i {
        return None;
    }
    Some(&code[start..i])
}

/// The body of the first braced block that starts at or after `at`.
///
/// A tuple struct has no braces of its own, and the block after it belongs to
/// something else — so a `(` or a `;` first means there is no body.
fn braced_body(code: &str, at: usize) -> Option<String> {
    let bytes = code.as_bytes();
    let mut i = at;
    while i < bytes.len() && bytes[i] != b'{' {
        if bytes[i] == b'(' || bytes[i] == b';' || bytes[i] == b',' {
            return None;
        }
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let mut depth = 0i32;
    let mut j = i;
    while j < bytes.len() {
        match bytes[j] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(code[i + 1..j].to_string());
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// The type an `impl` block is about, and where its body starts.
///
/// `impl Foo` is about `Foo`; `impl Trait for Foo` is about `Foo` as well —
/// the trait is what the block implements, not what it is on.
fn impl_self_type(code: &str, at: usize) -> Option<(String, usize)> {
    let bytes = code.as_bytes();
    let mut end = at;
    while end < bytes.len() && bytes[end] != b'{' {
        end += 1;
    }
    if end >= bytes.len() {
        return None;
    }
    let header = &code[at..end];
    let header = match header.split(" for ").nth(1) {
        Some(after) => after,
        None => header,
    };
    let mut i = 0usize;
    let hb = header.as_bytes();
    // A generic parameter list is not the type.
    if i < hb.len() && hb[i] == b'<' {
        let mut depth = 0i32;
        while i < hb.len() {
            match hb[i] {
                b'<' => depth += 1,
                b'>' => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    while i < hb.len() && !is_word_byte(hb[i]) {
        i += 1;
    }
    let start = i;
    while i < hb.len() && is_word_byte(hb[i]) {
        i += 1;
    }
    if start == i {
        return None;
    }
    Some((header[start..i].to_string(), end))
}

/// The variants of an enum, or the fields of a struct.
fn collect_members(body: &str, kind: &str, members: &mut BTreeSet<String>) {
    for line in body.lines() {
        let mut trimmed = line.trim();
        for prefix in ["pub ", "pub(crate) "] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                trimmed = rest.trim_start();
            }
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if kind == "enum" {
            let name: String = trimmed.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                members.insert(name);
            }
        } else if let Some((name, _)) = trimmed.split_once(':') {
            let name = name.trim();
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                members.insert(name.to_string());
            }
        }
    }
}

/// The items of an `impl` block: its functions, its constants, its types.
fn collect_items(body: &str, members: &mut BTreeSet<String>) {
    for line in body.lines() {
        let mut trimmed = line.trim();
        loop {
            let mut stripped = false;
            for prefix in ["pub ", "pub(crate) ", "async ", "unsafe ", "default ", "const "] {
                if let Some(rest) = trimmed.strip_prefix(prefix) {
                    trimmed = rest.trim_start();
                    stripped = true;
                }
            }
            if !stripped {
                break;
            }
        }
        for prefix in ["fn ", "const ", "type "] {
            let Some(rest) = trimmed.strip_prefix(prefix) else { continue };
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() {
                members.insert(name);
            }
        }
    }
}

/// Every `Cargo.toml` in the tree, except the ones in a build directory.
fn collect_manifests(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            collect_manifests(&path, out);
        } else if name == "Cargo.toml" {
            out.push(path);
        }
    }
}

/// The two files that may read a parameter's raw JSON.
///
/// `AGENTS.md`: "Read them through `craftkit_core::Params`' typed helpers;
/// never index." The typed helpers are typed, and a typed value cannot be
/// indexed — but `Params::raw_json` hands back a `&Json`, and a `&Json` can:
/// `json["key"]` panics on anything that is not an object, and answers `null`
/// for a key that is not there. So the rule is true of the readers and a
/// convention for everybody else. §136.
///
/// Two homes, and both are readers: the module that owns the parameters, and
/// the engine's module where a parameter becomes a typed value. A third file
/// that calls it is a third place where the shape of a parameter is decided,
/// and the shape is decided by matching, never by indexing.
const RAW_PARAMETER_HOMES: &[&str] = &["crates/core/src/params.rs", "crates/engine/src/params.rs"];

/// The raw parameter reader has two homes, and no more.
///
/// §136: the tree had a rule — *a parameter is never indexed* — in four
/// places, and nothing held it down. `Params::raw` was public and returned a
/// `&Json`, which is indexable; six call sites used it, all of them safely,
/// and a seventh would not have had to be careful.
pub fn the_raw_parameter_reader_has_two_homes(root: &Path) -> Result<Vec<String>, String> {
    let mut sources = Vec::new();
    collect_rust(root, &mut sources)?;
    let mut problems = Vec::new();
    for path in sources {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // Strings and comments do not count: this file names the reader, and
        // a doc comment that says `raw_json` is saying the opposite.
        //
        // And neither do the **tests**: the rule is about how the product
        // reads a parameter, and a test that calls the reader to prove the
        // reader works is not a third place where the shape of a parameter is
        // decided. §151.
        let code = code_lines_kept(&production_of(&text));
        if !code.contains("raw_json(") {
            continue;
        }
        let relative = relative_name(root, &path);
        if RAW_PARAMETER_HOMES.contains(&relative.as_str()) {
            continue;
        }
        let line = code.lines().position(|l| l.contains("raw_json(")).map_or(1, |n| n + 1);
        problems.push(format!(
            "{relative}: line {line} reads a parameter's raw JSON: the two readers are \
             `crates/core/src/params.rs` and `crates/engine/src/params.rs`, and a parameter is \
             never indexed"
        ));
    }
    problems.sort();
    Ok(problems)
}

/// The clippy settings that re-enable a lint in test code, and the lint each
/// one re-enables.
///
/// A setting that names a lint no crate denies **enforces nothing**: it
/// re-enables a lint that was never switched on. `clippy.toml` carried such
/// a setting — `allow-indexing-slicing-in-tests`, for a lint that is denied
/// in no crate — and it made the file look as though it held down the rule
/// that a parameter is never indexed. §136.
const TEST_ONLY_LINTS: &[(&str, &str)] = &[
    ("allow-unwrap-in-tests", "clippy::unwrap_used"),
    ("allow-expect-in-tests", "clippy::expect_used"),
    ("allow-panic-in-tests", "clippy::panic"),
];

/// Every setting in `clippy.toml` re-enables a lint the tree denies.
///
/// The file's own header used to claim it was "the one place that decides
/// whether the 'never crash' rule is enforced", and one of its four keys
/// enforced nothing at all: `clippy::indexing_slicing` is denied in no crate,
/// so allowing it in tests allowed something that was allowed everywhere.
/// §136.
pub fn every_test_only_setting_names_a_denied_lint(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("clippy.toml");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut problems = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("allow-") || !trimmed.ends_with("= true") {
            continue;
        }
        let setting = trimmed.trim_end_matches("= true").trim();
        let Some((_, lint)) = TEST_ONLY_LINTS.iter().find(|(s, _)| *s == setting) else {
            problems.push(format!(
                "clippy.toml: line {} sets `{setting}`, and it names no lint this tree knows: a \
                 setting that re-enables nothing enforces nothing",
                number + 1
            ));
            continue;
        };
        if !LINTS.contains(lint) {
            problems.push(format!(
                "clippy.toml: line {} sets `{setting}`, which re-enables `{lint}` — and no crate \
                 root denies it",
                number + 1
            ));
        }
    }
    Ok(problems)
}

/// The design file's byte cap, and the budget it has to come from.
///
/// §141. The cap used to be `64 * 1024 * 1024`, written out as a literal, and
/// the two numbers it has to agree with — the model's budget for a document,
/// and the format's expansion from a document to its text — were nowhere in
/// the same place. A literal that happens to be the right number today is a
/// number that can be wrong tomorrow: raise the model's budget and the
/// reader's cap stays where it was.
///
/// The runtime test in the io crate cannot see this, because the literal and
/// the derivation have the same value today. That is what a gate is for.
pub fn the_file_cap_is_derived_from_the_designs_budget(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("crates/io/src/format.rs");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut problems = Vec::new();
    // The code, not the text: a cap that stands only inside a string literal
    // is not a cap. §148.
    let code = code_lines_kept(&text);
    let lines: Vec<&str> = code.lines().collect();
    let Some(index) = lines.iter().position(|l| l.trim_start().starts_with("pub const MAX_FILE_BYTES"))
    else {
        problems.push(
            "crates/io/src/format.rs: there is no `MAX_FILE_BYTES`: the design file's cap is the \
             number the reader and the writer share"
                .to_string(),
        );
        return Ok(problems);
    };
    // The statement may be wrapped over several lines, and the names it has to
    // name are on one of them.
    let mut statement = lines[index].to_string();
    let mut last = index;
    while !statement.contains(';') && last + 1 < lines.len() {
        last += 1;
        statement.push(' ');
        statement.push_str(lines[last].trim());
    }
    if !statement.contains("MAX_DOCUMENT_BYTES") {
        problems.push(format!(
            "crates/io/src/format.rs: line {} writes the design file's cap without the model's \
             budget for a design in it; the cap has to be that budget times the format's \
             expansion, so that a design the model accepts can always be saved. §141.",
            index + 1
        ));
    }
    if !statement.contains("FILE_EXPANSION") {
        problems.push(format!(
            "crates/io/src/format.rs: line {} writes the design file's cap without the format's \
             expansion in it; the cap has to be the model's budget times the number of bytes of \
             text one byte of design takes. §141.",
            index + 1
        ));
    }
    Ok(problems)
}

/// The memory budgets, and the budget they have to come from.
///
/// §144. The model's limit table said a document's budget was *"the number
/// that says what the process actually uses"*, and the process holds **four**
/// documents: the one being edited, the base the command trace starts from,
/// and the undo and redo stacks. Measured, with a design at the model's
/// limit: 118.12 MiB of documents, 102.67 MiB of resident memory — **3.18
/// times** the number the table named.
///
/// Two caps had to come from the model's budget and neither said so. The
/// history's was `64 * 1024 * 1024` and the session's would have been the
/// same — which is also the design file's cap, a cap §141 had already
/// derived. Three caps with the same value and no relation between them are
/// three caps that drift apart the moment one of them is changed.
///
/// The runtime test in the engine cannot see this, because the literal and
/// the derivation have the same value today. That is what a gate is for.
pub fn the_memory_budgets_are_derived_from_the_models(root: &Path) -> Result<Vec<String>, String> {
    // (file, constant, the names its statement has to name, what it bounds)
    let wanted = [
        (
            "crates/engine/src/history.rs",
            "MAX_HISTORY_BYTES",
            &["MAX_DOCUMENT_BYTES", "HISTORY_EXPANSION"][..],
            "the undo and redo stacks",
        ),
        (
            "crates/engine/src/session.rs",
            "MAX_SESSION_BYTES",
            &["MAX_DOCUMENT_BYTES", "SESSION_DOCUMENTS"][..],
            "the documents a session holds",
        ),
        (
            "crates/engine/src/session.rs",
            "SESSION_DOCUMENTS",
            &["SESSION_OWN_DOCUMENTS", "SESSION_SNAPSHOT_DOCUMENTS", "HISTORY_EXPANSION"][..],
            "the documents a session holds",
        ),
    ];
    let mut problems = Vec::new();
    for (relative, constant, names, what) in wanted {
        let path = root.join(relative);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => return Err(format!("could not read {}: {e}", path.display())),
        };
        // The code, not the text: a budget that stands only inside a string
        // literal is not a budget. §148.
        let code = code_lines_kept(&text);
        let lines: Vec<&str> = code.lines().collect();
        let Some(index) =
            lines.iter().position(|l| l.trim_start().starts_with(&format!("pub const {constant}")))
        else {
            problems.push(format!(
                "{relative}: there is no `{constant}`: {what} have a budget, and it has to be \
                 the model's"
            ));
            continue;
        };
        // The statement may be wrapped over several lines, and the names it
        // has to name are on one of them.
        let mut statement = lines[index].to_string();
        let mut last = index;
        while !statement.contains(';') && last + 1 < lines.len() {
            last += 1;
            statement.push(' ');
            statement.push_str(lines[last].trim());
        }
        for name in names {
            if statement.contains(name) {
                continue;
            }
            problems.push(format!(
                "{relative}: line {} writes `{constant}` without `{name}` in it; {what} are \
                 measured in the model's own currency, so the cap has to be that budget times a \
                 number of documents. §144.",
                index + 1
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Every bound the tree has, and the ones its prose names.
///
/// §145. `docs/patterns/clone-before-edit.md` keeps a table of what undoing
/// costs, and it listed the *count* of undo entries (128) and the number of
/// nodes in a document (10 000) — and not the bound that actually binds, the
/// undo and redo stacks' 64 MiB. The paragraph under the table said the
/// product was the memory bound and that *"the fix is to raise `MAX_ENTRIES`
/// and add a size cap"*. §52 had already added the size cap, thirteen
/// sections earlier, and raising the count changes nothing for a document
/// over 512 KiB: measured, a design at the model's limit keeps **two**
/// snapshots, not a hundred and twenty-eight.
///
/// The rule, in two halves:
///
/// - a bound whose name says **bytes** or **memory** is named in the tree's
///   prose, so a reader can look it up;
/// - the prose does not tell a reader to **add a size cap** the tree has.
pub fn the_byte_bounds_are_named_in_the_prose(root: &Path) -> Result<Vec<String>, String> {
    let mut prose = String::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        prose.push_str(&text);
        prose.push('\n');
    }
    let mut problems = Vec::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        for name in byte_bounds(&code_lines_kept(&text)) {
            if prose.contains(&name) {
                continue;
            }
            problems.push(format!(
                "{}: the tree has `{name}` and no document in the tree names it; a bound a \
                 reader cannot look up is a bound a reader cannot check.",
                relative_name(root, &path)
            ));
        }
    }
    // The phrase is searched for with the whitespace collapsed, because a
    // document that wraps it across two lines is the same sentence.
    let flat: String = prose.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.contains("add a size cap") {
        problems.push(
            "a document tells a reader to add a size cap, and the tree has one: \
             `craftkit_engine::history::MAX_HISTORY_BYTES`. §145."
                .to_string(),
        );
    }
    problems.sort();
    Ok(problems)
}

/// The bounds in a file of Rust whose names say bytes or memory.
fn byte_bounds(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("pub const ") else { continue };
        let Some((name, _)) = rest.split_once(':') else { continue };
        let name = name.trim();
        if !name.starts_with("MAX_") {
            continue;
        }
        if name.contains("BYTES") || name.contains("MEMORY") {
            found.push(name.to_string());
        }
    }
    found
}

/// The tree's JSON has one writer, and the places that write a design use it.
///
/// §142. The crash report's three files were written with
/// `serde_json::to_string_pretty` — the tree's layout *without* the bound on
/// the indentation — so a report of a design the model accepts was 73.5 MiB of
/// text, and the crash reader refuses anything past 64 MiB. The app wrote
/// crash reports it could not read back.
///
/// The tree has one writer for its JSON (`craftkit_core::writer`), and the
/// crash folder is written with it, so a design is the same bytes in the crash
/// folder as it is on disk.
pub fn the_tree_writes_its_json_with_one_writer(root: &Path) -> Result<Vec<String>, String> {
    let files =
        [("crates/io/src/format.rs", "the design file"), ("crates/engine/src/crash.rs", "the crash report")];
    let mut problems = Vec::new();
    for (relative, what) in files {
        let path = root.join(relative);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
        // The tests are not the writer: a test may compare two shapes of the
        // same report with serde_json's own printer, and that is not the tree
        // writing anything.
        let production = match text.find("#[cfg(test)]") {
            Some(at) => &text[..at],
            None => &text[..],
        };
        // Strings and comments do not count: this file names the writer, and
        // a doc comment that says `to_string_pretty` is saying the opposite.
        let code = code_lines_kept(production);
        for (needle, because) in [
            (
                "to_string_pretty(",
                "serde_json's pretty printer, which is the tree's layout without the bound on the \
                 indentation",
            ),
            (
                "to_writer_pretty(",
                "serde_json's pretty writer, which is the tree's layout without the bound on the \
                 indentation",
            ),
        ] {
            if let Some(line) = code.lines().position(|l| l.contains(needle)).map(|n| n + 1) {
                problems.push(format!(
                    "{relative}: line {line} writes {what} with {because}; the tree has one writer \
                     for its JSON, and it is `craftkit_core::writer`. §142."
                ));
            }
        }
        if !code.contains("craftkit_core::writer::write_json") {
            problems.push(format!(
                "{relative}: {what} is not written with the tree's writer \
                 (`craftkit_core::writer::write_json`); a design written in two places has to be \
                 written the same way in both. §142."
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The types the evaluator's answers arrive in.
///
/// `Plan` is the whole answer, `NodeState` one node's, `Resolved` one
/// property's, `Diagnostic` one problem's. A shell that holds one of them
/// holds a **copy** of an answer, and a copy is stale the moment the design
/// changes. §138.
const EVALUATOR_TYPES: &[&str] = &["Plan", "NodeState", "Resolved", "Diagnostic", "Diagnostics", "Severity"];

/// The types a design's own value arrives in.
///
/// `Value` is what a property holds and `Tick` is the instant a plan was
/// made at: the two the evaluator hands out inside its own answers.
const DESIGN_VALUES: &[&str] = &["Value", "Tick"];

/// The numbers a level or an amount is.
///
/// `NodeState`'s `level_in` and `level_out` are `f64`, and so is every
/// `Value::Num`. A number in the shell's state is a number the design said,
/// and the design can change.
const EVALUATOR_NUMBERS: &[&str] = &["f64", "f32"];

/// The crates whose types are not the design's.
///
/// `serde_json::Value` is a JSON blob — a way of carrying what the person
/// typed — and not a property. The rule is about the design's values.
const NOT_DESIGN: &[&str] = &["serde_json"];

/// The shell may not cache a value the evaluator computes.
///
/// `AGENTS.md`: "A panel never writes to the document, never calls the
/// evaluator, never caches a value the evaluator computes."
///
/// The first half of that sentence is a gate (§135) and the second is a gate
/// (§132). The third was an **inspection** of `ShellState` — and an
/// inspection is a snapshot of the code, taken by somebody, that nobody keeps
/// taking. §138.
///
/// # What is checked, and what is not
///
/// The scope is the shell's **state**: `ShellState` and everything it holds.
/// The state is what the person is doing — a selection, a status line, the
/// text of the fields they are typing into — and every number in it would be
/// a number the *design* said, which is stale the moment the design changes.
/// So the state may hold no number, and no type of the design's.
///
/// A type that *carries* the state — `Shell`, `Frame`, `Click` — may hold a
/// screen coordinate or a frame's size: those numbers describe the screen,
/// not the design. It may not hold the evaluator's own answer either, because
/// a plan copied into a frame is a plan that goes stale while the frame is
/// still on the screen.
///
/// The scope is found rather than listed: the state is `ShellState`, the
/// structs it holds are found by following the field types, and the structs
/// that hold it are found the same way in the other direction. A struct the
/// state starts holding is covered the day it is added.
pub fn the_shell_caches_no_evaluated_value(root: &Path) -> Result<Vec<String>, String> {
    let structs = ui_structs(root);
    let mut problems = Vec::new();

    // The anchor. If the state is not where it was, the scope below is empty
    // and the gate would pass for the wrong reason — which is worse than not
    // having it.
    if !structs.iter().any(|s| s.name == "ShellState") {
        problems.push(
            "crates/ui/src/shell.rs: there is no `ShellState` to check: the shell's state has \
             been renamed or moved, and this gate is now watching nothing"
                .to_string(),
        );
        return Ok(problems);
    }

    let state = reach(&structs, "ShellState", true);
    let carried = reach(&structs, "ShellState", false);

    for subject in &structs {
        let holds_state = state.contains(&subject.name);
        let carries_state = carried.contains(&subject.name);
        if !holds_state && !carries_state {
            continue;
        }
        for (line, field, ty) in &subject.fields {
            for value in EVALUATOR_TYPES {
                if holds_type(ty, value) {
                    problems.push(format!(
                        "{file}: line {line} gives `{name}.{field}` the type `{ty}`: the shell \
                         holds what the person is doing, not a copy of what the evaluator \
                         computed",
                        file = subject.file,
                        name = subject.name,
                    ));
                }
            }
            if !holds_state {
                continue;
            }
            for value in DESIGN_VALUES {
                if holds_type(ty, value) {
                    problems.push(format!(
                        "{file}: line {line} gives `{name}.{field}` the type `{ty}`: that is a \
                         value the design said, and the design can change",
                        file = subject.file,
                        name = subject.name,
                    ));
                }
            }
            for value in EVALUATOR_NUMBERS {
                if holds_type(ty, value) {
                    problems.push(format!(
                        "{file}: line {line} gives `{name}.{field}` the type `{ty}`: that is a \
                         number the design said, and the design can change",
                        file = subject.file,
                        name = subject.name,
                    ));
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// A struct in the ui crate, and the fields it declares.
struct UiStruct {
    /// The file it is in, relative to the root.
    file: String,
    /// The struct's name.
    name: String,
    /// Its fields: the file line, the name, and the type as written.
    fields: Vec<(usize, String, String)>,
}

/// Every struct the ui crate declares, with its fields.
///
/// The fields are read from the source with the comments and the string
/// contents blanked out, so a type mentioned in a comment is not a type the
/// struct holds. A **tuple** struct is skipped: its fields are positions,
/// and the ui crate's one tuple struct holds a name.
fn ui_structs(root: &Path) -> Vec<UiStruct> {
    let mut out = Vec::new();
    for path in rust_files(&root.join("crates/ui/src")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let code = code_lines_kept(&text);
        let relative = relative_name(root, &path);
        let lines: Vec<&str> = code.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some(rest) = after_visibility(line.trim()) else { continue };
            let Some(rest) = rest.strip_prefix("struct ") else { continue };
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if name.is_empty() || !is_braced_struct(&rest[name.len()..]) {
                continue;
            }
            let mut fields = Vec::new();
            for (offset, inner) in lines.iter().skip(index + 1).enumerate() {
                let trimmed = inner.trim();
                if trimmed.starts_with('}') {
                    break;
                }
                if trimmed.is_empty() || trimmed.starts_with("#[") {
                    continue;
                }
                let Some((field, ty)) = trimmed.split_once(':') else { continue };
                let field = after_visibility(field.trim()).unwrap_or(field.trim());
                if !is_field_name(field) {
                    continue;
                }
                let ty = ty.split('=').next().unwrap_or(ty).trim().trim_end_matches(',').trim();
                if ty.is_empty() {
                    continue;
                }
                fields.push((index + offset + 2, field.to_string(), ty.to_string()));
            }
            out.push(UiStruct { file: relative.clone(), name, fields });
        }
    }
    out
}

/// Whether a struct declaration opens a brace — as opposed to a tuple.
///
/// The declaration may carry generic parameters: `pub struct Shell<'a> {`.
/// Those are skipped, and a `->` inside a bound is not the end of them.
fn is_braced_struct(tail: &str) -> bool {
    let tail = tail.trim_start();
    if !tail.starts_with('<') {
        return tail.starts_with('{');
    }
    let mut depth = 0usize;
    for (at, c) in tail.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                if at > 0 && tail.as_bytes()[at - 1] == b'-' {
                    continue;
                }
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return tail[at + 1..].trim_start().starts_with('{');
                }
            }
            _ => {}
        }
    }
    false
}

/// Whether a word is a field's name.
///
/// A field's name is an identifier: `pub selected: Option<NodeId>` has one and
/// `#[allow(clippy::unwrap_used)]` does not.
fn is_field_name(word: &str) -> bool {
    let mut chars = word.chars();
    let Some(first) = chars.next() else { return false };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The structs reachable from `start` by one relation over the field types.
///
/// `downward` finds what the set **holds**: `ShellState` holds `Prompt`.
/// Otherwise it finds what **holds** the set: `Frame` holds `ShellState`.
/// Both directions follow the types as they are written, so a struct that
/// mentions the state by name is in the scope of the rule.
fn reach(structs: &[UiStruct], start: &str, downward: bool) -> Vec<String> {
    let mut set = vec![start.to_string()];
    loop {
        let mut added = false;
        for subject in structs {
            if set.contains(&subject.name) {
                continue;
            }
            let joins = if downward {
                // Something in the set mentions the subject: the set holds it.
                structs.iter().any(|holder| {
                    holder.fields.iter().any(|(_, _, ty)| holds_type(ty, &subject.name))
                        && set.contains(&holder.name)
                })
            } else {
                // The subject mentions something in the set: it holds the set.
                subject.fields.iter().any(|(_, _, ty)| set.iter().any(|n| holds_type(ty, n)))
            };
            if joins {
                set.push(subject.name.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    set
}

/// Whether a type mentions a name the rule forbids.
///
/// A whole word, because `Value` is not `PromptValue` and `Plan` is not
/// `Planned`. And a path into a crate whose types are **not** the design's
/// does not count: `serde_json::Value` is a JSON blob and not a property, so
/// it is replaced before the word is looked for.
fn holds_type(ty: &str, name: &str) -> bool {
    let mut text = ty.to_string();
    for other in NOT_DESIGN {
        text = text.replace(&format!("{other}::{name}"), "not-the-design");
    }
    has_word(&text, name)
}

// ------------------------------------------------------------------ §139 ---
//
// `docs/agent-playbook.md` rule 4: *"Every parameter you read is declared in
// `params`"* — and the rule says its own check is *"call your command with
// its own parameters in a test"*, and that the test the tree has
// (`every_parameter_is_documented_and_uniquely_named`) **does not catch it**.
//
// The consequence is not a style point. `Session::execute` runs
// `p.deny_unknown(&spec.param_names())` before the command's `run`, so a
// parameter a command reads but does not declare is **refused**: the command
// cannot be given the value it needs, and every caller that tries is told
// "unknown parameter".
//
// A command that *declares* a parameter it never reads is the other half,
// and it is the silent one: the caller passes the value, the command ignores
// it, and the caller believes it did something.
//
// The check is on the **source**, because that is where the truth is: the
// command's `run` body says which names it reads, and its `params` list says
// which names it declares. Both are read from the same file, so they cannot
// drift without the gate noticing.

/// The name of the type a reader takes.
///
/// A **reader** is a function that takes a `&Params` — the only way to read a
/// parameter — or a method of `Params` itself.
const PARAMS_TYPE: &str = "Params";

/// The name of the argument that carries the parameter's name.
///
/// A reader's name argument is the one typed `key: &str`. A reader without
/// one reads whatever its own body reads: `node_by_id_or_name` takes the
/// document and the parameters and reads `node` and `node_name` itself.
const KEY_ARG: &str = "key: &str";

/// A function in the tree that can read a command parameter.
struct Reader {
    /// The file line its `fn` is on.
    line: usize,
    /// Its name.
    name: String,
    /// The index of its name argument, if it has one.
    key: Option<usize>,
    /// Its body, used to work out the names a reader without a name argument
    /// of its own reads.
    body: String,
}

/// A command in the registry, read from the source.
struct CommandSource {
    /// The command's id.
    id: String,
    /// The file line its declaration is on.
    line: usize,
    /// The names it declares.
    declared: Vec<String>,
    /// The body of its `run`, and the file line that body starts on.
    body: String,
    body_line: usize,
}

/// The offset of the delimiter that closes the one at `open`.
///
/// Strings are stepped over, so a parenthesis inside one does not count, and
/// the depth never goes below one, so a stray closer cannot underflow.
fn matching(source: &str, open: usize, left: u8, right: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut at = open;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == left {
            depth += 1;
        } else if c == right {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(at);
            }
        }
        at += 1;
    }
    None
}

/// The `impl` blocks in a source, as (header, first byte, last byte).
///
/// The header is what an `impl` says before its brace — `impl Params<'_>` —
/// and the range is what a function's position is tested against to find the
/// block it is in.
fn impl_blocks(source: &str) -> Vec<(String, usize, usize)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(found) = source[at..].find("impl") {
        let start = at + found;
        at = start + 4;
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }
        let Some(brace) = source[at..].find('{') else { break };
        let brace = at + brace;
        let Some(end) = matching(source, brace, b'{', b'}') else { continue };
        out.push((source[start..brace].trim().to_string(), start, end));
    }
    out
}

/// Every function in a Rust file, with its signature, its body and the block
/// it is written in.
fn rust_functions(file: &str, source: &str) -> Vec<(String, usize, String, String, String)> {
    let bytes = source.as_bytes();
    let blocks = impl_blocks(source);
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(found) = source[at..].find("fn ") {
        let start = at + found;
        at = start + 3;
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }
        let name: String = source[at..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        at += name.len();
        let Some(open) = source[at..].find('(') else { continue };
        let open = at + open;
        let Some(close) = matching(source, open, b'(', b')') else { continue };
        let signature = source[open + 1..close].to_string();
        let after = close + 1;
        let brace = source[after..].find('{').map(|i| after + i);
        let (body, line) = match brace {
            Some(bs) => {
                let line = source[..bs].lines().count() + 1;
                match matching(source, bs, b'{', b'}') {
                    Some(be) => (source[bs..=be].to_string(), line),
                    None => (String::new(), line),
                }
            }
            None => (String::new(), source[..open].lines().count() + 1),
        };
        // The block the function is written in: the innermost `impl` that
        // holds it.
        let block = blocks
            .iter()
            .filter(|(_, first, last)| *first < start && start < *last)
            .max_by_key(|(_, first, _)| *first)
            .map(|(header, _, _)| header.clone())
            .unwrap_or_default();
        out.push((name, line, signature, body, block));
    }
    let _ = file;
    out
}

/// The arguments of a call, as they are written.
///
/// The text between the brackets, split at the commas that are not inside a
/// bracket or a string.
fn arguments_of(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            current.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                current.push(c);
            }
            '(' | '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            ',' if depth == 0 => {
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// The string a piece of text is, if it is one.
fn as_string_literal(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.len() < 2 || !trimmed.starts_with('"') || !trimmed.ends_with('"') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.contains('"') {
        return None;
    }
    Some(inner.to_string())
}

/// The index of a reader's name argument, if it has one.
///
/// The name argument is the one typed `key: &str`. A method's receiver is not
/// an argument — it is written in the signature and not in the call — so it
/// does not count: `text(&self, key: &str, max: usize)` has its name at
/// index 0 and `node_ref(p: &Params<'_>, key: &str)` has its name at index 1.
fn key_index(signature: &str) -> Option<usize> {
    arguments_of(signature)
        .iter()
        .filter(|arg| !matches!(arg.as_str(), "&self" | "&mut self" | "self"))
        .position(|arg| arg.starts_with(KEY_ARG) || arg.starts_with("key:&str"))
}

/// Every reader in the tree.
///
/// A reader is a function whose signature takes a `&Params`, or a method of
/// `Params` itself. The list is **derived**, not written down: a reader that
/// is added is covered the day it is added, and the gate cannot go blind
/// because somebody forgot to update a list.
fn parameter_readers(root: &Path) -> Vec<Reader> {
    let mut out = Vec::new();
    for path in rust_files(&root.join("crates")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let relative = relative_name(root, &path);
        for (name, line, signature, body, block) in rust_functions(&relative, &text) {
            let is_method = has_word(&block, PARAMS_TYPE);
            let takes_params = signature.contains(&format!("&{PARAMS_TYPE}"));
            if !is_method && !takes_params {
                continue;
            }
            out.push(Reader { line, name, key: key_index(&signature), body });
        }
    }
    out
}

/// The names a body reads, and where it reads them.
///
/// A reader with a name argument contributes the string at that argument. A
/// reader without one contributes the names its own body reads.
fn names_read(body: &str, body_line: usize, readers: &[Reader], depth: usize) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    if depth > 3 {
        return out;
    }
    let bytes = body.as_bytes();
    for reader in readers {
        let mut at = 0usize;
        while let Some(found) = body[at..].find(&reader.name) {
            let start = at + found;
            at = start + reader.name.len();
            // The name must stand alone: `text` is not `required_text`.
            if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
                continue;
            }
            // And it must be a call.
            let Some(open) = body[at..].find('(') else { continue };
            let open = at + open;
            if !body[at..open].trim().is_empty() {
                continue;
            }
            let Some(close) = matching(body, open, b'(', b')') else { continue };
            let call_line = body_line + body[..open].lines().count();
            match reader.key {
                Some(index) => {
                    let args = arguments_of(&body[open + 1..close]);
                    let Some(arg) = args.get(index) else { continue };
                    if let Some(name) = as_string_literal(arg) {
                        out.push((name, call_line));
                    }
                }
                None => {
                    // A reader with no name argument of its own reads what its
                    // own body reads — but the read happens **here**, at the
                    // call site, so that is the line the report names.
                    for (name, _) in names_read(&reader.body, reader.line, readers, depth + 1) {
                        out.push((name, call_line));
                    }
                }
            }
        }
    }
    out
}

/// The commands in the registry, read from the source.
fn registry_commands(root: &Path) -> Vec<CommandSource> {
    let path = root.join("crates/engine/src/commands.rs");
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new() };
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0usize;
    while let Some(found) = text[at..].find("pub static ") {
        let start = at + found;
        at = start + 10;
        if start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            continue;
        }
        let Some(brace) = text[at..].find('{') else { break };
        let brace = at + brace;
        let Some(end) = matching(&text, brace, b'{', b'}') else { continue };
        let block = &text[brace..=end];
        let line = text[..brace].lines().count() + 1;
        let Some(id) = block.find("id: \"") else { continue };
        let id_start = id + 5;
        let Some(id_end) = block[id_start..].find('"') else { continue };
        let id = block[id_start..id_start + id_end].to_string();

        // The names the command declares.
        let mut declared = Vec::new();
        let mut scan = 0usize;
        while let Some(found) = block[scan..].find("ParamDoc::") {
            let mark = scan + found;
            scan = mark + 10;
            let Some(open) = block[scan..].find('(') else { break };
            let open = scan + open;
            let Some(close) = matching(block, open, b'(', b')') else { continue };
            let args = arguments_of(&block[open + 1..close]);
            if let Some(name) = args.first().and_then(|a| as_string_literal(a)) {
                declared.push(name);
            }
        }

        // The body of its `run`.
        let mut body = String::new();
        let mut body_line = line;
        if let Some(run) = block.find("run:") {
            let after = run + 4;
            if let Some(brace) = block[after..].find('{') {
                let open = after + brace;
                if let Some(close) = matching(block, open, b'{', b'}') {
                    body = block[open..=close].to_string();
                    body_line = line + block[..open].lines().count();
                }
            }
        }
        out.push(CommandSource { id, line, declared, body, body_line });
    }
    out
}

/// Every parameter a command reads is declared, and every parameter it
/// declares is read.
///
/// `docs/agent-playbook.md` rule 4: *"Every parameter you read is declared in
/// `params`"* — and the rule says its own check is *"call your command with
/// its own parameters in a test"*, and that the test the tree has does not
/// catch it. §139.
///
/// A parameter a command reads but does not declare is **refused** by
/// `Session::execute`, which runs `p.deny_unknown(&spec.param_names())`
/// before the command's `run`: the command cannot be given the value it
/// needs, and every caller that tries is told "unknown parameter".
///
/// A parameter a command declares but never reads is the silent half: the
/// caller passes the value, the command ignores it, and the caller believes
/// it did something.
pub fn every_parameter_a_command_reads_is_declared(root: &Path) -> Result<Vec<String>, String> {
    let readers = parameter_readers(root);
    let commands = registry_commands(root);
    let mut problems = Vec::new();

    if commands.is_empty() {
        problems.push(
            "crates/engine/src/commands.rs: no command was found: the gate is watching nothing".to_string(),
        );
        return Ok(problems);
    }

    for command in &commands {
        let read = names_read(&command.body, command.body_line, &readers, 0);
        let declared: Vec<&str> = command.declared.iter().map(|s| s.as_str()).collect();

        for (name, line) in &read {
            if declared.contains(&name.as_str()) {
                continue;
            }
            problems.push(format!(
                "crates/engine/src/commands.rs: line {line} reads the parameter `{name}`, which \
                 `{}` does not declare: the engine refuses an undeclared parameter, so the \
                 command cannot be given the value it needs",
                command.id
            ));
        }
        for name in &command.declared {
            if read.iter().any(|(n, _)| n == name) {
                continue;
            }
            problems.push(format!(
                "crates/engine/src/commands.rs: line {line} declares the parameter `{name}`, \
                 which `{}` never reads: a caller that passes it is ignored",
                command.id,
                line = command.line
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The names that evaluate a document from outside `craftkit-eval`.
///
/// `evaluate_once` is the function; `Evaluator` is the struct whose `evaluate`
/// method does the same thing. Those two are the whole of it: everything else
/// `craftkit-eval` offers the outside world is a type or a number.
const EVALUATOR_NAMES: &[&str] = &["evaluate_once", "Evaluator"];

/// The shell may read the plan; it may not evaluate the document.
///
/// `AGENTS.md`: "A panel never writes to the document, never calls the
/// evaluator, never caches a value the evaluator computes."
///
/// The first half is enforced by the engine's API: the only mutable accessor,
/// `Session::doc_mut`, is `pub(crate)` to the engine, so the shell cannot
/// reach it. The third half holds by what `ShellState` is. The middle half
/// had nothing behind it: `crates/ui` depends on `craftkit-eval` for two of
/// its *types* — `Plan` and `Severity` — and could have called the evaluator
/// itself, which would put a second road to the answer on the screen and
/// cost every frame an evaluation of the whole document. §132.
///
/// The check is on the **names**, and the names are exact. Comments and
/// strings do not count: a doc comment that says the word is saying the
/// opposite.
pub fn the_shell_does_not_evaluate(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for path in rust_files(&root.join("crates/ui/src")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // The tests are not the panels: a test may name the evaluator to
        // prove that the shell does not call it.
        let production = match text.find("\nmod tests {") {
            Some(at) => &text[..at],
            None => &text[..],
        };
        let code = code_lines_kept(production);
        let relative = relative_name(root, &path);
        for name in EVALUATOR_NAMES {
            if !has_word(&code, name) {
                continue;
            }
            let line = code.lines().position(|l| has_word(l, name)).map_or(1, |n| n + 1);
            problems.push(format!(
                "{relative}: line {line} names `{name}`: the shell may read the plan, but it may \
                 not evaluate the document"
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// The source with its comments, its string contents and its char literals
/// blanked out, and every line kept.
///
/// This is the tree's **one** way to read code out of a file, and there is
/// one because §149 measured what happens when there are three: the rule that
/// a mention in a string is not code was written three times — as
/// `code_only`, as `without_comments` and as `strip_code` — and the gates
/// that read a *symbol* or a *module* used none of them, so a test that
/// stood only inside a string satisfied a document's claim about it. A rule
/// written three times is three chances to forget it.
///
/// Every line is kept, because a check that names a line has to name the
/// **file's** line: dropping a block comment's newlines moved the line of
/// every constant after it, and the gate that reported the line was wrong by
/// the size of the comment.
///
/// A **char literal** is blanked too, because a brace inside one is not a
/// brace: the scanner that reads a type's members counts braces, and a
/// `'{'` in the source would open a body that is not there. A **lifetime**
/// is not a char literal: `fn f<'a>(x: &'a str)` has one, and treating it as
/// a literal would swallow the rest of the file.
fn code_lines_kept(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_block = false;
    let mut in_line = false;
    while let Some(c) = chars.next() {
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
                out.push(' ');
            } else if c == '\n' {
                out.push('\n');
            } else {
                out.push(' ');
            }
            continue;
        }
        if in_string {
            if escaped {
                // A **continuation**: a backslash at the end of a line inside
                // a string. The line it ends is still a line of the file, and
                // a check that names a line has to name the file's line —
                // the tree's own `crates/io/src/format.rs` has three of them.
                out.push(if c == '\n' { '\n' } else { ' ' });
                escaped = false;
                continue;
            }
            match c {
                '\\' => {
                    escaped = true;
                    out.push(' ');
                }
                '"' => {
                    in_string = false;
                    out.push(' ');
                }
                '\n' => out.push('\n'),
                _ => out.push(' '),
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(' ');
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line = true;
                out.push(' ');
                out.push(' ');
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block = true;
                out.push(' ');
                out.push(' ');
            }
            '\'' => {
                // A char literal, or a lifetime.
                if !opens_a_char_literal(&chars) {
                    out.push('\'');
                    continue;
                }
                out.push(' ');
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        out.push(' ');
                        if let Some(next) = chars.next() {
                            out.push(if next == '\n' { '\n' } else { ' ' });
                        }
                        continue;
                    }
                    if c == '\'' {
                        out.push(' ');
                        break;
                    }
                    out.push(if c == '\n' { '\n' } else { ' ' });
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Whether the quote just read opens a char literal rather than a lifetime.
///
/// `'a'` is a literal and `'a` is a lifetime. The difference is the quote that
/// closes it: a literal is one character — or an escape — between two quotes,
/// and a lifetime is a name with no closing quote. The escape counts because
/// `'\''` is three characters and `'a'` is one, and the two have to be told
/// apart the same way.
fn opens_a_char_literal(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut ahead = chars.clone();
    match ahead.next() {
        Some('\\') => true,
        Some(_) => ahead.next() == Some('\''),
        None => false,
    }
}

/// Whether `code` contains `name` as a whole word.
///
/// A whole word, because `Evaluator` is not `Evaluators` and `evaluate_once`
/// is not `evaluate_once_at`.
fn has_word(code: &str, name: &str) -> bool {
    let bytes = code.as_bytes();
    let mut at = 0usize;
    while at < code.len() {
        let Some(found) = code[at..].find(name) else { break };
        let start = at + found;
        let end = start + name.len();
        let before = start == 0 || !is_word_byte(bytes[start - 1]);
        let after = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before && after {
            return true;
        }
        at = end;
    }
    false
}

/// Whether a byte is part of an identifier.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The public methods a session may have.
///
/// `AGENTS.md`: "User-visible behaviour = a `CommandSpec` in
/// `crates/engine/src/commands.rs`. That one registry is what the toolbar, the
/// dialogs, scripts, the CLI, the control channel and the MCP server all
/// reach. **There is no second path.**"
///
/// The front ends honoured that — measured, every one of them reaches the
/// registry and nothing else that changes a design — but the **engine's own
/// public API** was a second path, and a front end was free to take it.
/// `Session::recompute` re-evaluated the document with no command, no trace
/// step and no log entry, and it had **no caller at all**: it was public by
/// accident. `Session::seek_to` moved the playhead without the clamp the
/// registry applies, and it had a caller — the crash replay. §135.
///
/// So the rule is put where it can be enforced. The list is short on purpose:
///
/// - `new` and `sample` — the constructors.
/// - the readers — nothing here changes anything;
/// - `execute` — the registry, the one road into the design;
/// - `replace_document` and `load_document` — the host's business. File I/O
///   is not behaviour (§18), and the desktop app, the web app and the CLI
///   all open designs this way.
///
/// Anything else that is `pub` on `Session` is a second path, and the gate
/// says so.
const SESSION_PUBLIC_METHODS: &[&str] = &[
    "new",
    "sample",
    "document",
    "node",
    "at",
    "plan",
    "is_dirty",
    "can_undo",
    "can_redo",
    "undo_depth",
    "history_bytes",
    // A reader, and the other half of `history_bytes`: the document being
    // edited and the base the trace starts from are the rest of what a
    // session holds, and §144 is the section that says so.
    "document_bytes",
    "log",
    "trace",
    "trace_base",
    "trace_total",
    "trace_dropped",
    "snapshot_json",
    "execute",
    "replace_document",
    "load_document",
];

/// The session's public methods, and what they are allowed to be.
///
/// §135: "There is no second path" had nothing behind it. The check is on the
/// **names of the public methods**, and the list is an allow list: a new
/// public method on `Session` is refused until somebody says what it is and
/// why it is not a second path. That is the only direction that fails safe.
pub fn the_session_offers_no_second_path(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("crates/engine/src/session.rs");
    let source =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let Some((block, first_line)) = impl_block(&source, "impl Session {") else {
        return Err(format!("{} has no `impl Session` block to read", path.display()));
    };
    // Comments and strings do not count: a doc comment that says `pub fn` is
    // saying the opposite.
    let code = code_lines_kept(block);
    let relative = relative_name(root, &path);
    let mut problems = Vec::new();
    // A method marked `#[cfg(test)]` is not in the shipped build, so it is not
    // a public way into the design: the gate's subject is the **public** API,
    // and a test's method is not part of it. §152.
    let mut test_only = false;
    for (offset, line) in code.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("#[cfg(test)]") {
            test_only = true;
            continue;
        }
        if trimmed.starts_with("#[") {
            // Another attribute: the test-only mark is spent unless it says
            // `cfg(test)` itself.
            test_only = false;
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("pub fn ") else {
            // Any other item ends the mark's reach.
            test_only = false;
            continue;
        };
        let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if name.is_empty() || SESSION_PUBLIC_METHODS.contains(&name.as_str()) {
            test_only = false;
            continue;
        }
        if test_only {
            test_only = false;
            continue;
        }
        problems.push(format!(
            "{relative}: line {} makes `Session::{name}` public: the session's public methods are the \
             registry and the things that read it — a public way to change the design is a second path",
            first_line + offset
        ));
    }
    Ok(problems)
}

/// An `impl` block, and the file line its first line is on.
///
/// The block ends at the first `}` in the first column: everything inside an
/// `impl` is indented, so a closing brace in the first column is the block's
/// own. The line number is carried because a check that names a line has to
/// name the **file's** line.
fn impl_block<'a>(source: &'a str, header: &str) -> Option<(&'a str, usize)> {
    let start = source.find(header)?;
    let body = &source[start + header.len()..];
    let end = body.find("\n}\n")?;
    let block = &body[..end];
    // The header's own line, plus the lines before it in the file.
    let first_line = source[..start].lines().count() + 1;
    Some((block, first_line))
}

/// The names that put a document into a session.
const DOCUMENT_WRITERS: &[&str] = &["load_document", "replace_document"];

/// The shell may not write the document.
///
/// `AGENTS.md`: "A panel never writes to the document, never calls the
/// evaluator, never caches a value the evaluator computes."
///
/// §132 said the first half of that sentence was enforced by the engine's
/// API, because `Session::doc_mut` is `pub(crate)` to the engine. It was
/// enforced for the **mutator** and not for the **writers**:
/// `Session::load_document` and `Session::replace_document` are public — they
/// have to be, because the desktop app, the web app and the CLI open designs
/// with them (§18) — so a panel could have put a document into the session
/// and replaced the whole design under the user's hands. It never did; the
/// shell's production code calls neither. But "did not" is not "may not", and
/// the difference is exactly what a gate is for. §135.
pub fn the_shell_does_not_write_the_document(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for path in rust_files(&root.join("crates/ui/src")) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        // The tests are not the panels: a test may open a design to have
        // something to draw.
        let production = match text.find("\nmod tests {") {
            Some(at) => &text[..at],
            None => &text[..],
        };
        // Only the tail is cut, so the line numbers are the file's.
        let code = code_lines_kept(production);
        let relative = relative_name(root, &path);
        for name in DOCUMENT_WRITERS {
            if !has_word(&code, name) {
                continue;
            }
            let line = code.lines().position(|l| has_word(l, name)).map_or(1, |n| n + 1);
            problems.push(format!(
                "{relative}: line {line} names `{name}`: the shell may read the design, but it may \
                 not put a document into the session"
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Citations in prose that provably no longer point at anything.
///
/// The mechanically detectable half, and deliberately only that half. A
/// `file.rs:12` that lands on a blank line or on a lone brace is certainly
/// wrong — the lines above it moved. A citation that lands on real code which
/// says something else is not detectable by any tool, because "what it should
/// have said" is a judgement about prose, and this does not pretend otherwise.
///
/// It exists because the tree carried ninety such citations and nothing looked at
/// any of them. Five pointed at blank lines or braces, and two more — checked by
/// hand — pointed at the import block of a file whose skip logic they claimed to
/// quote. Some of that drift was caused by this audit editing the very files it
/// cited, which is exactly how it stays wrong.
///
/// Prefer a symbol over a line number when you write one. `wasm::run` still means
/// something next year; `wasm.rs:41` does not.
pub fn citation_drift(root: &Path) -> Result<Vec<String>, String> {
    // Every Rust file in the tree, so a bare `wasm.rs` can be resolved. The
    // second component is a score for preferring the citation's own suffix
    // match over a first-found name.
    let mut sources: Vec<PathBuf> = Vec::new();
    collect_rust(root, &mut sources)?;

    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let text = match std::fs::read_to_string(&doc) {
            Ok(t) => t,
            Err(e) => return Err(format!("could not read {}: {e}", doc.display())),
        };
        let relative = relative_name(root, &doc);
        for (name, from, to) in citations(&text) {
            let Some(path) = resolve(&sources, &name) else {
                problems.push(format!("{relative}: cites `{name}`, which is not in the tree"));
                continue;
            };
            let lines = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let total = lines.lines().count();
            if from == 0 || to < from || to > total {
                problems.push(format!("{relative}: cites `{name}:{from}`, outside a {total}-line file"));
                continue;
            }
            let body: Vec<&str> = lines.lines().skip(from - 1).take(to - from + 1).map(str::trim).collect();
            let blank = body.iter().all(|l| l.is_empty());
            let braces = body.iter().all(|l| matches!(*l, "}" | "{" | ");" | "]"));
            if blank || braces {
                problems.push(format!(
                    "{relative}: cites `{name}:{from}`, which points at {} — the line has \
                     moved. Cite the symbol instead.",
                    if blank { "nothing but blank lines" } else { "a lone brace" }
                ));
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// Bare line citations — `` `:123` `` — which name a line without naming a file.
///
/// A relative citation is a real convention: the file was named a sentence
/// earlier, and the number is a short way back to it. It is also a citation
/// with no owner, and **twenty-four of them were in this tree and thirteen were
/// wrong** — pointing at a line that had moved, or at a *different file* from
/// the one the sentence had led to. A reader of `docs/recipes/add-a-control-
/// channel-method.md` following "(`the_token_is_only_needed_on_the_first_request`
/// (`:446`))" lands in `protocol.rs`, and the test is in `server.rs`.
///
/// No gate checked them: `citation_drift` only reads `name.rs:12`, and a bare
/// `:12` has no name in it. So the rule is not "check the bare form" — it is
/// **"do not use the bare form"**. A citation that names a symbol rots into a
/// wrong *reading* at worst; a citation that names a line rots into a wrong
/// *place*, and a wrong place is what sends somebody to the wrong file.
///
/// # Errors
///
/// Every occurrence is reported, with the line it is on.
pub fn bare_line_citations(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, cited) in bare_citations(&text) {
            problems.push(format!(
                "{relative}: line {number} cites a line number with no file (`:{cited}`). \
                 Cite the symbol, or name the file: `name.rs:{cited}`"
            ));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Every `` `:123` `` in `text`, as (line number, the number).
///
/// Hand-written, for the reason [`citations`] gives. A backticked token that is
/// only a colon and digits is unambiguous: a port is written `4711`, a URL
/// carries a scheme, and a range is written `:12-14`.
pub fn bare_citations(text: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let token = &after[..close];
            if let Some(digits) = token.strip_prefix(':')
                && !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit())
                && let Ok(value) = digits.parse::<usize>()
            {
                found.push((number + 1, value));
            }
            rest = &after[close + 1..];
        }
    }
    found
}

/// Every `name.rs:12` or `name.rs:12-14` in `text`, as (name, first, last).
///
/// Hand-written rather than pulled from a crate, for the reason `args.rs` gives
/// for the command line: the pattern is small, and a dependency here would be
/// the only one the build tool has that is not already in the tree.
pub fn citations(text: &str) -> Vec<(String, usize, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut search = 0usize;

    while let Some(offset) = text[search..].find(".rs:") {
        let dot = search + offset;

        // The name runs backwards to the first byte that cannot be part of one.
        let mut start = dot;
        while start > 0 {
            let c = bytes[start - 1];
            if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'/') {
                start -= 1;
            } else {
                break;
            }
        }
        // `dot` is the `.` of `.rs:`, so the name runs to three bytes past it.
        let name = &text[start..dot + 3];

        let mut at = dot + 4;
        match read_number(bytes, &mut at) {
            None => {
                search = dot + 4;
                continue;
            }
            Some(first) => {
                let last = read_range_end(bytes, at).unwrap_or(first);
                if !name.is_empty() {
                    found.push((name.to_string(), first, last));
                }
                search = dot + 4;
            }
        }
    }
    found
}

/// A number after an optional range separator: spaces, a dash, spaces.
///
/// Handles an en dash as well as a hyphen, because prose uses both and a
/// citation written with one is still a citation.
fn read_range_end(bytes: &[u8], at: usize) -> Option<usize> {
    let mut probe = at;
    while probe < bytes.len() && bytes[probe] == b' ' {
        probe += 1;
    }
    if probe < bytes.len() && matches!(bytes[probe], b'-' | 0xe2) {
        // Step over the dash itself, then the rest of it if it is multi-byte,
        // then any spaces. Detecting the dash without stepping past it leaves
        // `read_number` looking at a `-`, which is not a digit.
        probe += 1;
        while probe < bytes.len() && (bytes[probe] == b' ' || bytes[probe] >= 0x80) {
            probe += 1;
        }
        return read_number(bytes, &mut probe);
    }
    None
}

/// Read a decimal number at `at`, advancing past it. `None` if there is not one.
fn read_number(bytes: &[u8], at: &mut usize) -> Option<usize> {
    let start = *at;
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == start {
        return None;
    }
    *at = end;
    std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok()
}

/// The Markdown files the tree points at code from.
fn prose_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in PROSE {
        let path = root.join(entry);
        if path.is_file() {
            found.push(path);
        } else if path.is_dir() {
            collect_markdown(&path, &mut found);
        }
    }
    found.sort();
    found
}

/// Every `.md` under `dir`, at any depth.
///
/// Recursive because it was not, and that was not a small gap: `docs/` has
/// three markdown files beside it and **fifteen** under `recipes/`, `design/`
/// and `patterns/`, holding 49 `file.rs:12` citations. The gate whose closing
/// line says every citation in prose points at something had never opened one
/// of them. A walk that only looks at the top level is a walk that looks
/// almost nowhere, and it did not say so anywhere.
fn collect_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "dist") {
                continue;
            }
            collect_markdown(&path, out);
        } else if path.extension().and_then(|x| x.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name.starts_with('.') || matches!(name.as_str(), "target" | "node_modules" | "dist") {
                continue;
            }
            collect_rust(&path, out)?;
        } else if path.extension().and_then(|x| x.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The file a citation names, preferring one whose tail matches what was written.
fn resolve(sources: &[PathBuf], name: &str) -> Option<PathBuf> {
    let wanted = name.rsplit('/').next().unwrap_or(name);
    sources
        .iter()
        .find(|p| p.file_name().and_then(|f| f.to_str()) == Some(name))
        .or_else(|| {
            sources.iter().find(|p| {
                p.to_string_lossy().replace('\\', "/").ends_with(name)
                    || p.file_name().and_then(|f| f.to_str()) == Some(wanted)
            })
        })
        .cloned()
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// The crates whose roots carry the shared lines.
///
/// Every directory under `crates/` and `apps/`, plus `xtask`. Discovered rather
/// than listed, so a new crate is checked the moment it is created instead of
/// the moment somebody remembers.
pub fn crate_roots(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut roots = Vec::new();
    for group in ["crates", "apps"] {
        let dir = root.join(group);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            for file in ["src/lib.rs", "src/main.rs"] {
                let candidate = entry.path().join(file);
                if candidate.is_file() {
                    roots.push(candidate);
                }
            }
        }
    }
    let xtask = root.join("xtask/src/lib.rs");
    if xtask.is_file() {
        roots.push(xtask);
    }
    roots.sort();
    Ok(roots)
}

/// Check the shared lines in every crate root.
///
/// Returns a description of what is missing *and* the line to paste, because a
/// report that says "missing deny attribute" and stops there has made the
/// reader go and find the exact string.
pub fn check_roots(roots: &[std::path::PathBuf]) -> Vec<String> {
    let mut problems = Vec::new();
    for path in roots {
        let body = match std::fs::read_to_string(path) {
            Ok(body) => body,
            // A root that cannot be read is reported rather than skipped: a gate
            // that quietly ignores files it cannot see is not a gate.
            Err(e) => {
                problems.push(format!("{}: could not be read ({e})", path.display()));
                continue;
            }
        };
        match normalise(&body) {
            None => problems.push(format!(
                "{}: no deny attribute.\n     Add this at the top of the file:\n{}\n     It must deny exactly these lints: {}",
                path.display(),
                DENY.lines().map(|l| format!("     {l}")).collect::<Vec<_>>().join("\n"),
                LINTS.join(", ")
            )),
            Some(found) if found == canonical() => {}
            Some(found) => problems.push(format!(
                "{}: the deny attribute is not the house one.\n     found: {found}\n     want:  {}\n     Add this at the top of the file:\n{}",
                path.display(),
                canonical(),
                DENY.lines().map(|l| format!("     {l}")).collect::<Vec<_>>().join("\n"),
            )),
        }
        if !body.contains(FORBID) {
            problems.push(format!("{}: no `#![forbid(unsafe_code)]`.\n     Add: {FORBID}", path.display()));
        }
    }
    problems
}

/// One row of the documentation's layering table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocRow {
    pub layer: u8,
    pub crates: Vec<String>,
    /// The short names the row claims each crate depends on.
    pub depends: Vec<String>,
}

/// Parse the layering table out of `docs/architecture.md`.
///
/// The shape it looks for is the one the documentation actually uses:
///
/// ```text
/// | L3 | `craftkit-eval` | core, model, kernel |
/// ```
///
/// A table that has been rewritten into prose, or whose header has changed,
/// parses to nothing — and a table that parses to nothing is a failure, not a
/// pass. A gate that cannot find what it is checking must say so.
pub fn parse_table(markdown: &str) -> Vec<DocRow> {
    let mut rows = Vec::new();
    for line in markdown.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line.split('|').skip(1).map(|c| c.trim().to_string()).collect();
        if cells.len() < 3 {
            continue;
        }
        let Some(layer) = cells[0].trim_start_matches('L').parse::<u8>().ok() else { continue };
        let crates: Vec<String> = cells[1]
            .split(',')
            .map(|c| c.trim().trim_matches('`').to_string())
            .filter(|c| !c.is_empty())
            .collect();
        if crates.is_empty() {
            continue;
        }
        let depends: Vec<String> = cells[2]
            .split(',')
            .map(|c| c.trim().trim_matches('`').to_string())
            .filter(|c| !c.is_empty() && *c != "nothing (no workspace deps)")
            .collect();
        rows.push(DocRow { layer, crates, depends });
    }
    rows
}

/// The short name of a package: `craftkit-engine` is `engine`.
pub fn short(package: &str) -> String {
    package.strip_prefix("craftkit-").unwrap_or(package).to_string()
}

/// Compare the documentation's table with the code.
///
/// Returns a list of human-readable disagreements. Two different kinds are
/// checked, because they fail for different reasons and an agent fixing one
/// should not assume the other:
///
/// - the **order** of the rows, which is a merge conflict waiting to happen;
/// - the **dependency column**, which is documentation drifting from code.
pub fn compare_table(docs: &[DocRow], crates: &[layers::Krate]) -> Vec<String> {
    let mut problems = Vec::new();

    // The order, taken from the table in layers.rs: every layered crate in
    // table order, then the apps.
    let expected: Vec<(u8, String)> = TABLE
        .iter()
        .filter(|(_, class)| matches!(class, layers::Class::Layer(_)))
        .map(|(name, class)| (class.layer().unwrap_or(0), (*name).to_string()))
        .collect();
    // The apps row names directories, not packages, and every app is exempt
    // from the rules, so it is not part of the order being compared.
    let documented: Vec<(u8, String)> = docs
        .iter()
        .flat_map(|row| {
            row.crates
                .iter()
                .filter(|c| !c.contains('/'))
                .cloned()
                .map(move |c| (row.layer, c))
                .collect::<Vec<_>>()
        })
        .collect();
    if expected != documented {
        problems.push(format!(
            "the layering table in {ARCHITECTURE} does not match xtask/src/layers.rs.\n\
             \x20    code:  {}\n\
             \x20    docs:  {}\n\
             \x20    the code table is the one that is enforced; fix the documentation to match it.",
            expected.iter().map(|(l, n)| format!("L{l} {n}")).collect::<Vec<_>>().join(", "),
            documented.iter().map(|(l, n)| format!("L{l} {n}")).collect::<Vec<_>>().join(", "),
        ));
    }

    // The dependency column, against what cargo actually says.
    for row in docs {
        for name in &row.crates {
            let Some(krate) = crates.iter().find(|c| &c.name == name) else {
                if name.contains('/') {
                    continue; // the apps row lists paths, not packages
                }
                problems.push(format!("{ARCHITECTURE} documents `{name}`, which is not a workspace crate"));
                continue;
            };
            let actual: BTreeSet<String> =
                krate.deps.iter().filter(|d| d.workspace).map(|d| short(&d.name)).collect();
            let claimed: BTreeSet<String> = row.depends.iter().map(|d| short(d)).collect();
            if actual != claimed && !(claimed.is_empty() && actual.is_empty()) {
                problems.push(format!(
                    "{ARCHITECTURE} says `{name}` depends on [{}] but cargo says [{}]",
                    claimed.into_iter().collect::<Vec<_>>().join(", "),
                    actual.into_iter().collect::<Vec<_>>().join(", "),
                ));
            }
        }
    }
    problems
}

/// Is a list of strings sorted, ignoring duplicates?
pub fn is_sorted(lines: &[String]) -> bool {
    let mut previous: Option<&String> = None;
    for line in lines {
        if let Some(previous) = previous
            && line.as_str() < previous.as_str()
        {
            return false;
        }
        previous = Some(line);
    }
    true
}

/// The workspace member globs, read out of the root manifest.
///
/// Hand-parsed rather than pulled from `cargo metadata`: `metadata` expands the
/// globs and loses the order, and the order is the entire point.
pub fn workspace_members(manifest: &str) -> Vec<String> {
    manifest
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("members"))
        .skip(1)
        .take_while(|l| l.trim_start().starts_with('"'))
        .map(|l| l.trim().trim_matches('"').to_string())
        .collect()
}

/// The layer table in `README.md`, compared with the one in the code.
///
/// `docs/architecture.md` has a table, and `compare_table` checks it against
/// `layers::TABLE`. The README has a **second copy of the same table** in a
/// different shape — `crates/core     L0   ticks, values, ids, hostile
/// parameters` — and nothing kept the two in step. They agree today; the
/// agreement is a coincidence of two people having been careful.
///
/// This is §22's shape, one layer down: two implementations of one rule, one of
/// which is enforced and one of which is decoration.
///
/// The README names *directories* and the code names *packages*, so the
/// directory's `Cargo.toml` is what joins them.
pub fn readme_layers(root: &Path) -> Result<Vec<String>, String> {
    let readme = root.join("README.md");
    let text =
        std::fs::read_to_string(&readme).map_err(|e| format!("could not read {}: {e}", readme.display()))?;

    let documented = readme_layer_rows(&text);
    if documented.is_empty() {
        return Err(format!(
            "{}: found no `crates/… L<n>` rows; the scan is wrong, not the README",
            readme.display()
        ));
    }

    // The code side: every layered crate, in table order.
    let code: Vec<(String, u8)> = layers::TABLE
        .iter()
        .filter_map(|(name, class)| match class {
            layers::Class::Layer(level) => Some(((*name).to_string(), *level)),
            _ => None,
        })
        .collect();

    let mut problems = Vec::new();
    for (directory, level) in &documented {
        let manifest = root.join(directory).join("Cargo.toml");
        let manifest_text = std::fs::read_to_string(&manifest)
            .map_err(|e| format!("could not read {}: {e}", manifest.display()))?;
        let package = package_name(&manifest_text);
        match code.iter().find(|(name, _)| *name == package) {
            Some((_, code_level)) if code_level == level => {}
            Some((_, code_level)) => {
                problems.push(format!("README.md says {directory} is L{level}; the code says L{code_level}"))
            }
            None => problems.push(format!(
                "README.md lists {directory} as L{level}, but {package} is not a layered crate"
            )),
        }
    }

    // And the other direction: a crate the code has and the README does not.
    for (package, level) in &code {
        let documented_here = documented.iter().any(|(directory, _)| {
            root.join(directory).join("Cargo.toml").is_file()
                && std::fs::read_to_string(root.join(directory).join("Cargo.toml"))
                    .map(|t| package_name(&t) == *package)
                    .unwrap_or(false)
        });
        if !documented_here {
            problems.push(format!("README.md's layer table does not mention {package} (L{level})"));
        }
    }

    problems.sort();
    Ok(problems)
}

/// The `crates/<dir>   L<n>` rows of a README, as (directory, layer).
///
/// The README's table is a fenced text block, and the row shape is: a
/// directory, a layer, and a description. Only the first two are facts; the
/// description is prose.
pub fn readme_layer_rows(text: &str) -> Vec<(String, u8)> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("crates/") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(directory) = parts.next() else { continue };
        let Some(layer) = parts.next() else { continue };
        let Some(level) = layer.strip_prefix('L').and_then(|n| n.parse::<u8>().ok()) else { continue };
        rows.push((directory.to_string(), level));
    }
    rows
}

/// The `name = "…"` of a manifest, read by hand.
///
/// The manifest is a list of tables and the name is in the first one; scanning
/// for the first `name =` line is what a person does, and the build tool has no
/// dependency to do it properly.
pub fn package_name(manifest: &str) -> String {
    for line in manifest.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                return rest.trim().trim_matches('"').to_string();
            }
        }
    }
    String::new()
}

/// The `mod x;` lines of one file, split into groups by blank lines or comments.
pub fn mod_groups(source: &str) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            continue;
        }
        let rest = line.strip_prefix("pub mod ").or_else(|| line.strip_prefix("mod ")).unwrap_or("");
        {
            let name = rest.split(';').next().unwrap_or("").trim().to_string();
            if !name.is_empty() {
                current.push(name);
            }
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

/// A function a document quotes, and the function it is quoting.
///
/// `docs/patterns/clone-before-edit.md` used to say "the four restore
/// statements" — and, in three other places, "one", "two" and "three". The
/// count was what rotted: the code changed and the prose did not, and nothing
/// noticed, because a number in a sentence is not a citation and
/// `citation_drift` only reads `file.rs:12`.
///
/// The fix was to quote the code instead of counting it. A fenced block whose
/// info string says `exact` is a claim that it is the source, word for word,
/// and this check holds that claim: every function such a block quotes must
/// be the function in the file the block names, line for line, in both
/// directions. A line added to the function and not to the document is caught,
/// and so is a line in the document that the function does not have.
///
/// The marker is deliberate. Most blocks in the tree are *illustrations* with
/// elisions in them, and pretending otherwise would make the check cry wolf.
///
/// **And a struct is held to it too.** §75: `docs/recipes/fix-a-crash.md`
/// quoted the whole of `CrashReport` — every field, with the types — and §72
/// changed two of those types without re-reading the recipe. The quote was
/// the one document a person reads when their application crashed, and it
/// described a report that no longer existed. A struct quoted exactly is
/// compared field by field: the same names, in the same order, with the same
/// types.
pub fn quoted_code_matches_the_source(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in quoting_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, block) in exact_blocks(&text) {
            let Some(source) = source_named_by(&block) else {
                problems.push(format!(
                    "{relative}: the block at line {number} is marked exact but does not name the file it quotes"
                ));
                continue;
            };
            let path = root.join(&source);
            let Ok(code) = std::fs::read_to_string(&path) else {
                problems.push(format!(
                    "{relative}: the block at line {number} is marked exact but {source} is not in the tree"
                ));
                continue;
            };
            let quotes = quotes_in(&block);
            if quotes.compared.is_empty() && quotes.uses.is_empty() && quotes.incomparable.is_empty() {
                problems.push(format!(
                    "{relative}: the block at line {number} is marked exact but quotes no code"
                ));
                continue;
            }
            for item in quotes.compared {
                problems.extend(compare_item(&relative, number, &source, &code, item));
            }
            for (index, statement) in quotes.uses {
                if !code.lines().any(|line| line.trim() == statement) {
                    problems.push(format!(
                        "{relative}: the block at line {} uses `{statement}`, and {source} does not have it",
                        number + 1 + index
                    ));
                }
            }
            for (index, kind) in quotes.incomparable {
                problems.push(format!(
                    "{relative}: the block at line {} quotes a `{kind}`, which this check does not compare",
                    number + 1 + index
                ));
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// An item an `exact` block quotes.
///
/// Every kind the gate can compare is one of these, and nothing else is: an
/// item it cannot compare is reported by name rather than skipped, because a
/// block marked `exact` promises that it is the code in the file. §146.
enum Quoted {
    /// An item compared line for line: a function, a static, a const, an
    /// enum, a trait or a type alias.
    Lines { kind: &'static str, name: String, lines: Vec<String> },
    /// A struct, compared as its fields.
    Fields { name: String, fields: Vec<(String, String)> },
}

/// What an `exact` block quotes, and what the gate does with each item.
struct Quotes {
    /// The items the gate compares with the source.
    compared: Vec<Quoted>,
    /// The `use` declarations the block holds, as (index in the block, text).
    uses: Vec<(usize, String)>,
    /// The items the gate cannot compare, as (index in the block, kind).
    incomparable: Vec<(usize, &'static str)>,
}

/// The kinds of Rust item a block can start.
///
/// The list is what the gate *recognises*, so it is kept to the kinds the
/// tree's documents quote or would quote next. A kind that is not here is not
/// seen at all — which is why `impl`, `mod` and `macro_rules!` are here even
/// though they are not compared: they are reported instead.
const ITEM_KINDS: &[&str] =
    &["fn", "struct", "enum", "static", "const", "trait", "type", "use", "impl", "mod", "macro_rules!"];

/// What the gate does with a kind of item.
enum Comparison {
    /// Compared line for line.
    Lines,
    /// Compared as its fields.
    Fields,
    /// A declaration the source has to contain.
    Context,
    /// Not compared: reported by name.
    Incomparable,
}

/// What the gate does with a kind of item.
fn comparison_of(kind: &str) -> Comparison {
    match kind {
        "fn" | "static" | "const" | "enum" | "trait" | "type" => Comparison::Lines,
        "struct" => Comparison::Fields,
        "use" => Comparison::Context,
        _ => Comparison::Incomparable,
    }
}

/// The item a line starts, as (kind, the rest of the line).
///
/// One classifier for the comparators and for the "not compared" report, so
/// the two cannot disagree: a kind no comparator handles is a kind the
/// report names. §146.
fn item_head(line: &str) -> Option<(&'static str, &str)> {
    let rest = after_visibility(line.trim())?;
    let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    for kind in ITEM_KINDS {
        let Some(tail) = rest.strip_prefix(*kind) else { continue };
        if !tail.starts_with(' ') && !tail.starts_with('\t') {
            continue;
        }
        return Some((kind, tail.trim_start()));
    }
    None
}

/// The name of an item and what follows it, as (name, rest).
fn item_name<'a>(kind: &str, tail: &'a str) -> Option<(String, &'a str)> {
    if kind == "use" {
        return None;
    }
    if kind == "static" && tail.starts_with("mut ") {
        return None;
    }
    let end = tail
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
        .map(|(at, _)| at)
        .unwrap_or(tail.len());
    if end == 0 {
        return None;
    }
    Some((tail[..end].to_string(), tail[end..].trim_start()))
}

/// What an `exact` block quotes.
///
/// Only items at the left margin: a nested item is part of the item around
/// it, and the item around it is compared whole.
fn quotes_in(block: &[String]) -> Quotes {
    let mut quotes = Quotes { compared: Vec::new(), uses: Vec::new(), incomparable: Vec::new() };
    for (index, line) in block.iter().enumerate() {
        if !at_left_margin(line) {
            continue;
        }
        let Some((kind, tail)) = item_head(line) else { continue };
        match comparison_of(kind) {
            Comparison::Lines => {
                let Some((name, _)) = item_name(kind, tail) else { continue };
                quotes.compared.push(Quoted::Lines { kind, name, lines: item_lines(block, index, kind) });
            }
            Comparison::Fields => {
                let Some((name, rest)) = item_name(kind, tail) else { continue };
                if rest.starts_with('{') {
                    quotes.compared.push(Quoted::Fields { name, fields: struct_fields(&block[index..]) });
                } else if let Some(types) = tuple_fields(rest) {
                    quotes.compared.push(Quoted::Fields { name, fields: types });
                }
            }
            Comparison::Context => quotes.uses.push((index, line.trim().to_string())),
            Comparison::Incomparable => quotes.incomparable.push((index, kind)),
        }
    }
    quotes
}

/// Compare one quoted item with the one in the source.
fn compare_item(relative: &str, number: usize, source: &str, code: &str, item: Quoted) -> Vec<String> {
    match item {
        Quoted::Lines { kind, name, lines } => match item_in(code, kind, &name) {
            None => vec![format!(
                "{relative}: the block at line {number} quotes `{name}`, and {source} has no {kind} of that name"
            )],
            Some(original) if original == lines => Vec::new(),
            Some(original) => vec![format!(
                "{relative}: the block at line {number} quotes `{name}`, and it is not the one in {source}\n     \
                 the document says:\n{}\n     the file says:\n{}",
                indent(&lines),
                indent(&original)
            )],
        },
        Quoted::Fields { name, fields } => match struct_in(code, &name) {
            None => vec![format!(
                "{relative}: the block at line {number} quotes the struct `{name}`, and {source} has no struct of that name"
            )],
            Some(original) if original == fields => Vec::new(),
            Some(original) => vec![format!(
                "{relative}: the block at line {number} quotes the struct `{name}`, and it is not the one in {source}\n     \
                 the document says:\n{}\n     the file says:\n{}",
                indent(&fields.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>()),
                indent(&original.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>())
            )],
        },
    }
}

/// An item in `code`, by kind and name.
///
/// The source is searched at any depth: a method is quoted from inside the
/// `impl` block that holds it, and the quote is written flat.
fn item_in(code: &str, kind: &str, name: &str) -> Option<Vec<String>> {
    let lines: Vec<String> = code.lines().map(|l| l.to_string()).collect();
    let index = lines.iter().position(|line| {
        let Some((found, tail)) = item_head(line) else { return false };
        if found != kind {
            return false;
        }
        item_name(kind, tail).is_some_and(|(found, _)| found == name)
    })?;
    Some(item_lines(&lines, index, kind))
}

/// The lines of an item, from its first line to the one that ends it.
fn item_lines(lines: &[String], start: usize, kind: &str) -> Vec<String> {
    match kind {
        "fn" => function_lines(lines, start),
        "static" => static_lines(lines, start),
        "const" | "type" => const_lines(lines, start),
        "enum" | "trait" => braced_lines(lines, start),
        _ => vec![lines[start].trim().to_string()],
    }
}

/// The lines of a braced item, from its first line to the `}` that closes it.
fn braced_lines(lines: &[String], start: usize) -> Vec<String> {
    let indent = indent_of(&lines[start]);
    let mut out = vec![lines[start].trim().to_string()];
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(trimmed.to_string());
        if trimmed == "}" && indent_of(line) == indent {
            break;
        }
    }
    out
}

/// The lines of a statement, from its first line to the `;` that ends it.
///
/// A const or a type alias may hold its value over several lines, so the
/// statement ends at the semicolon and not at the end of the first line.
fn const_lines(lines: &[String], start: usize) -> Vec<String> {
    let indent = indent_of(&lines[start]);
    let mut out = vec![lines[start].trim().to_string()];
    if lines[start].trim_end().ends_with(';') {
        return out;
    }
    let mut depth = bracket_depth(&lines[start]);
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        depth += bracket_depth(trimmed);
        out.push(trimmed.to_string());
        if depth <= 0 && trimmed.ends_with(';') && indent_of(line) == indent {
            break;
        }
    }
    out
}

/// How many brackets a line opens beyond the ones it closes.
fn bracket_depth(line: &str) -> i32 {
    let mut depth = 0;
    for ch in line.chars() {
        match ch {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            _ => {}
        }
    }
    depth
}

/// How far a line is indented.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether a line starts at the left margin, where an item starts.
fn at_left_margin(line: &str) -> bool {
    !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t')
}

/// The lines of a static, from its `static` line to the `};` that closes it.
///
/// A static is quoted **whole**: the recipe for adding a command shows the
/// command, and a command that is one field out of date is a command that
/// does not compile. §140.
fn static_lines(lines: &[String], start: usize) -> Vec<String> {
    let indent = lines[start].len() - lines[start].trim_start().len();
    let mut out = vec![lines[start].trim().to_string()];
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let own = line.len() - line.trim_start().len();
        if trimmed == "};" && own == indent {
            out.push(trimmed.to_string());
            break;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// The types of a tuple struct, in order: (position, type).
///
/// A tuple struct has no field names, and the types are what the quote is
/// about — `Tick(i128)` is the whole point of that quote.
fn tuple_fields(tail: &str) -> Option<Vec<(String, String)>> {
    let inner = tail.strip_prefix('(')?.trim_end_matches(';').strip_suffix(')')?;
    let mut out = Vec::new();
    for (at, part) in inner.split(',').enumerate() {
        let part = part.split("//").next().unwrap_or(part).trim();
        if part.is_empty() {
            continue;
        }
        out.push((at.to_string(), part.to_string()));
    }
    Some(out)
}

/// The fields of a struct, as (name, type).
///
/// The attributes, the doc comments and the trailing comments are not fields:
/// a document that quotes a struct may leave the comments out and keep the
/// shape, and the shape is what this compares.
fn struct_fields(lines: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in lines.iter().skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            break;
        }
        if trimmed.starts_with("#[") || trimmed.starts_with("///") || trimmed.starts_with("//") {
            continue;
        }
        let head = trimmed.split("//").next().unwrap_or(trimmed).trim();
        let head = head.trim_end_matches(',').trim();
        let Some((name, ty)) = head.split_once(':') else { continue };
        out.push((name.trim().to_string(), ty.trim().to_string()));
    }
    out
}

/// A struct in `code`, by name, as its fields.
fn struct_in(code: &str, name: &str) -> Option<Vec<(String, String)>> {
    let lines: Vec<String> = code.lines().map(|l| l.to_string()).collect();
    let index = lines.iter().position(|line| {
        let Some((kind, tail)) = item_head(line) else { return false };
        if kind != "struct" {
            return false;
        }
        item_name(kind, tail).is_some_and(|(found, _)| found == name)
    })?;
    let (_, rest) = item_head(&lines[index]).and_then(|(kind, tail)| item_name(kind, tail))?;
    if rest.starts_with('{') {
        return Some(struct_fields(&lines[index..]));
    }
    tuple_fields(rest)
}

/// The documents that may carry an `exact` block.
///
/// Every prose file: the marker is the contract, and the file that broke it
/// was a recipe, not a pattern document. §75.
fn quoting_files(root: &Path) -> Vec<PathBuf> {
    prose_files(root)
}

/// Every fenced block in `text` that is marked exact, as (line, lines).
fn exact_blocks(text: &str) -> Vec<(usize, Vec<String>)> {
    let mut found = Vec::new();
    let mut lines = text.lines().enumerate();
    while let Some((number, line)) = lines.next() {
        let Some(info) = line.trim().strip_prefix("```") else { continue };
        if !info.split_whitespace().any(|word| word == "exact") {
            continue;
        }
        let mut body = Vec::new();
        for (_, inner) in lines.by_ref() {
            if inner.trim() == "```" {
                break;
            }
            body.push(inner.to_string());
        }
        found.push((number + 1, body));
    }
    found
}

/// The source file a block names, if it names one.
fn source_named_by(block: &[String]) -> Option<String> {
    block.iter().find_map(|line| {
        let rest = line.trim().strip_prefix("// ")?;
        // `// crates/engine/src/session.rs — "One catch_unwind"` names the file
        // and then goes on; the file is the part before the dash.
        let name = rest.split('\u{2014}').next().unwrap_or(rest).trim();
        if name.ends_with(".rs") && name.contains('/') { Some(name.to_string()) } else { None }
    })
}

/// The line with its visibility keyword removed, if it has one.
fn after_visibility(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("pub(crate) ").or_else(|| line.strip_prefix("pub "));
    Some(rest.unwrap_or(line))
}

/// The lines of a function, from its `fn` line to the `}` that closes it.
fn function_lines(lines: &[String], start: usize) -> Vec<String> {
    let indent = lines[start].len() - lines[start].trim_start().len();
    let mut out = vec![lines[start].trim().to_string()];
    for line in &lines[start + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let own = line.len() - line.trim_start().len();
        if trimmed == "}" && own == indent {
            out.push(trimmed.to_string());
            break;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// A block of lines, indented for a message.
fn indent(lines: &[String]) -> String {
    lines.iter().map(|l| format!("       {l}")).collect::<Vec<_>>().join("\n")
}

/// A line count in prose, and the file it is about.
///
/// `docs/agent-playbook.md` said `crates/io/src/format/pending.rs` was "110
/// lines". `docs/recipes/change-the-file-format.md` said the same file was
/// "98 lines". **Both were wrong** — it is 174 — and they disagreed with
/// each other, which is exactly what a number in a sentence looks like when
/// nothing checks it. §36 removed the bare line *citations* from this tree
/// and left the line *counts* behind.
///
/// The rule is anchored the way §39's is: the number means something only
/// because the file it is about is named. The nearest backticked `*.rs` path
/// before the number, in the same paragraph, is that file.
///
/// Two forms count, and only two:
///
/// - "N lines" — the file is N lines long. `~900` does not count: an
///   approximation is not a claim that can be wrong.
/// - "the tests start at line N" — the file's test module begins there.
pub fn prose_line_counts_match_the_files(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, paragraph) in paragraphs(&text) {
            for (at, count) in line_counts(&paragraph) {
                let number = number + paragraph[..at].bytes().filter(|b| *b == b'\n').count();
                let Some(file) = nearest_rust_path_before(&paragraph, at) else { continue };
                let path = root.join(&file);
                let Ok(body) = std::fs::read_to_string(&path) else { continue };
                let lines: Vec<&str> = body.lines().collect();
                if count > lines.len() {
                    problems.push(format!(
                        "{relative}: line {number} says `{file}` is {count} lines, and it has {}",
                        lines.len()
                    ));
                    continue;
                }
                if lines[count - 1].trim() != "mod tests {" {
                    problems.push(format!(
                        "{relative}: line {number} says the tests in `{file}` start at line {count}, \
                         and that line is `{}`",
                        lines[count - 1].trim()
                    ));
                }
            }
            for (at, count) in file_lengths(&paragraph) {
                let number = number + paragraph[..at].bytes().filter(|b| *b == b'\n').count();
                let Some(file) = nearest_rust_path_before(&paragraph, at) else { continue };
                let path = root.join(&file);
                let Ok(body) = std::fs::read_to_string(&path) else { continue };
                let actual = body.lines().count();
                if actual != count {
                    problems.push(format!(
                        "{relative}: line {number} says `{file}` is {count} lines, and it is {actual}"
                    ));
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// The paragraphs of a text, as (line number, paragraph).
fn paragraphs(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut start = 0usize;
    let mut current = String::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                found.push((start + 1, std::mem::take(&mut current)));
            }
            start = index + 1;
            continue;
        }
        if current.is_empty() {
            start = index;
        }
        current.push_str(line);
        current.push('\n');
    }
    if !current.trim().is_empty() {
        found.push((start + 1, current));
    }
    found
}

/// "N lines" claims in a paragraph, as (offset, count).
///
/// An approximation is not a claim that can be wrong, so `~900 lines` and
/// "about 900 lines" are not claims about a file's length.
fn file_lengths(paragraph: &str) -> Vec<(usize, usize)> {
    let bytes = paragraph.as_bytes();
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(found_at) = paragraph[at..].find(" lines") {
        let number_at = at + found_at;
        at = number_at + " lines".len();
        // Walk back over the digits.
        let mut start = number_at;
        while start > 0 && bytes[start - 1].is_ascii_digit() {
            start -= 1;
        }
        if start == number_at {
            continue;
        }
        let count: usize = match paragraph[start..number_at].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        // An approximation marker in front of the number means "about".
        let before = paragraph[..start].trim_end();
        let approximate = before.ends_with('~')
            || before.ends_with("about")
            || before.ends_with("around")
            || before.ends_with("roughly")
            || before.ends_with("nearly");
        if approximate {
            continue;
        }
        found.push((start, count));
    }
    found
}

/// "the tests start at line N" claims in a paragraph, as (offset, line).
fn line_counts(paragraph: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let mut at = 0usize;
    while let Some(found_at) = paragraph[at..].find("start at line ") {
        let number_at = at + found_at + "start at line ".len();
        at = number_at;
        let mut end = number_at;
        let bytes = paragraph.as_bytes();
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == number_at {
            continue;
        }
        if let Ok(line) = paragraph[number_at..end].parse::<usize>() {
            found.push((number_at, line));
        }
    }
    found
}

/// The backticked `*.rs` path nearest before `at` in a paragraph.
fn nearest_rust_path_before(paragraph: &str, at: usize) -> Option<String> {
    let mut found = None;
    let mut cursor = 0usize;
    while let Some(open) = paragraph[cursor..].find('`') {
        let open = cursor + open;
        let after = &paragraph[open + 1..];
        let Some(close) = after.find('`') else { break };
        let name = &after[..close];
        if open < at && name.ends_with(".rs") && name.contains('/') {
            found = Some(name.to_string());
        }
        cursor = open + close + 2;
    }
    found
}

/// Code that prose attributes to a file, and the file it is attributed to.
///
/// §42: the rollback is described in five places in this tree, and three of
/// them were wrong — two because they repeated a false reason, and one
/// because §40's own refactor deleted the lines it quoted. Nothing checks
/// prose that quotes code.
///
/// A claim is a *pair*: a backticked piece of code, and the backticked
/// `*.rs` path it is about, next to each other in the same paragraph. Two
/// forms count:
///
/// - `Type::method` — the method must be in that file.
/// - a statement, ending in `;` — the statement must be a line of that file.
///
/// §39's rule is the same shape for test names, and deliberately narrower: a
/// test name is a *name*, a `Type::method` is a *place*.
pub fn attributed_code_is_in_the_file(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, paragraph) in paragraphs(&text) {
            let spans = backticked_spans(&paragraph);
            for pair in spans.windows(2) {
                let (left, right) = (&pair[0], &pair[1]);
                let gap = right.0.saturating_sub(left.2);
                if gap > 40 {
                    continue;
                }
                for (code, file) in [(left.1, right.1), (right.1, left.1)] {
                    if !file.ends_with(".rs") || !file.contains('/') {
                        continue;
                    }
                    let Some(claim) = code_claim(code) else { continue };
                    let path = root.join(file);
                    let Ok(body) = std::fs::read_to_string(&path) else { continue };
                    let present = match &claim {
                        Claim::Method(name) => body.contains(name.as_str()),
                        Claim::Statement(line) => body.lines().any(|l| l.trim() == line),
                    };
                    if present {
                        continue;
                    }
                    let at = number + paragraph[..left.0].bytes().filter(|b| *b == b'\n').count();
                    problems
                        .push(format!("{relative}: line {at} says `{code}` is in `{file}`, and it is not"));
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// The backticked spans of a paragraph, as (start, text, end) of the text
/// inside the backticks.
fn backticked_spans(paragraph: &str) -> Vec<(usize, &str, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while let Some(open) = paragraph[cursor..].find('`') {
        let open = cursor + open;
        let after = &paragraph[open + 1..];
        let Some(close) = after.find('`') else { break };
        spans.push((open + 1, &after[..close], open + close + 2));
        cursor = open + close + 2;
    }
    spans
}

/// What a piece of quoted code claims to be.
enum Claim {
    /// `Type::method` — the method must be in the file.
    Method(String),
    /// A statement — the statement must be a line of the file.
    Statement(String),
}

/// The claim a piece of quoted code makes, if it makes one at all.
fn code_claim(code: &str) -> Option<Claim> {
    let code = code.trim();
    if let Some((_, method)) = code.rsplit_once("::") {
        let owner = code[..code.len() - method.len() - 2].trim();
        let valid_owner = !owner.is_empty()
            && owner.chars().next().is_some_and(|c| c.is_uppercase())
            && owner.chars().all(|c| c.is_alphanumeric() || c == '_');
        let valid_method = method.chars().next().is_some_and(|c| c.is_lowercase() || c == '_')
            && method.chars().all(|c| c.is_alphanumeric() || c == '_');
        if valid_owner && valid_method {
            return Some(Claim::Method(method.to_string()));
        }
    }
    let looks_like_a_statement =
        code.ends_with(';') && code.len() <= 70 && code.contains(['=', '(', '.']) && !code.contains('`');
    if looks_like_a_statement {
        return Some(Claim::Statement(code.to_string()));
    }
    None
}

/// A constant in prose, and the value the code gives it.
///
/// The safety limits are the most consequential numbers in this tree: they are
/// what bounds a hostile file, and they are written down in three documents
/// so that an agent coding against them uses the right ones. Sixteen claims
/// exist and **all sixteen were true** — but they were true by luck, because
/// nothing compared them with the code. Change `MAX_NODES` and the
/// architecture table keeps saying 10 000.
///
/// Two forms count:
///
/// - `` `MAX_NODES` 10 000 `` — the number is the constant's value.
/// - `` `MAX_LINE` `1 << 20` `` — the expression is the constant's value.
pub fn prose_constants_match_the_code(root: &Path) -> Result<Vec<String>, String> {
    let mut constants: Vec<(String, String, String)> = Vec::new();
    for path in rust_files(root) {
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        let where_ = relative_name(root, &path);
        for (name, value) in constants_in(&source) {
            constants.push((name, value, where_.clone()));
        }
    }
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, line) in text.lines().enumerate() {
            // A table row: one cell holds the constant and another holds the
            // value. The two are in different cells, so the paragraph pass
            // below cannot see them.
            let trimmed = line.trim();
            if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
                continue;
            }
            let cells: Vec<String> = trimmed
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_string())
                .collect();
            for (index, cell) in cells.iter().enumerate() {
                let name = cell.rsplit("::").next().unwrap_or(cell);
                if !is_constant_name(name) {
                    continue;
                }
                let values: Vec<(String, String)> = constants
                    .iter()
                    .filter(|(known, _, _)| known == name)
                    .map(|(_, value, file)| (value.clone(), file.clone()))
                    .collect();
                if values.is_empty() {
                    continue;
                }
                for (other_index, other) in cells.iter().enumerate() {
                    if other_index == index || !other.starts_with(|c: char| c.is_ascii_digit()) {
                        continue;
                    }
                    if !values.iter().any(|(value, _)| numbers_agree(other, value)) {
                        problems.push(format!(
                            "{relative}: line {} says `{name}` is `{other}`, and the code says {}",
                            number + 1,
                            say_the_values(&values)
                        ));
                    }
                }
            }
        }
        for (number, paragraph) in paragraphs(&text) {
            let spans = backticked_spans(&paragraph);
            for span in &spans {
                let name = span.1.rsplit("::").next().unwrap_or(span.1);
                if !is_constant_name(name) {
                    continue;
                }
                let values: Vec<(String, String)> = constants
                    .iter()
                    .filter(|(known, _, _)| known == name)
                    .map(|(_, value, file)| (value.clone(), file.clone()))
                    .collect();
                if values.is_empty() {
                    continue;
                }
                let at = number + paragraph[..span.0].bytes().filter(|b| *b == b'\n').count();
                // The value is either the text right after the name — "NODES`
                // 10 000" — or the next backticked span, when it is an
                // expression: "`MAX_LINE` is `1 << 20` bytes".
                let after = paragraph[span.2..].trim_start();
                let after = after.trim_start_matches(['(', ':', '=']);
                let claimed = if after.starts_with(|c: char| c.is_ascii_digit()) {
                    after
                        .split(|c: char| !(c.is_ascii_digit() || c == ' ' || c == '_'))
                        .next()
                        .unwrap_or("")
                        .trim()
                } else {
                    let next = spans.iter().find(|other| other.0 >= span.2);
                    match next {
                        Some(next) => {
                            let candidate = next.1.trim();
                            // The span is the constant's value only if it is
                            // an expression the code could have written.
                            // A path is not: `crates/core/src/lib.rs` sits
                            // beside constant names all over the tree's
                            // prose, and reading it as a value would turn a
                            // mention into a claim. §147.
                            let recognised = candidate.contains("<<") || candidate.contains('*');
                            if !recognised && !is_a_value_claim(candidate, &values) {
                                continue;
                            }
                            candidate
                        }
                        None => continue,
                    }
                };
                if claimed.is_empty() {
                    continue;
                }
                let matches = values
                    .iter()
                    .any(|(value, _)| numbers_agree(claimed, value) || expressions_agree(claimed, value));
                if !matches {
                    problems.push(format!(
                        "{relative}: line {at} says `{name}` is `{claimed}`, and the code says {}",
                        say_the_values(&values)
                    ));
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// Every `pub const NAME: … = VALUE;` in a source file.
fn constants_in(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    // The code, not the text: a constant that stands only inside a string
    // literal is not a constant of the tree, and a fabricated one can mask a
    // real drift. The tree has the rule already — §78's `code_only` — and it
    // is applied here. §148.
    let code = code_lines_kept(source);
    for line in code.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub const ") else { continue };
        let Some((name, value)) = rest.split_once('=') else { continue };
        let name = name.split(':').next().unwrap_or("").trim();
        if !is_constant_name(name) {
            continue;
        }
        let value = value.trim().trim_end_matches(';').trim();
        if value.is_empty() {
            continue;
        }
        found.push((name.to_string(), value.to_string()));
    }
    found
}

/// What the code says about a constant, with the file it is in.
///
/// Two constants can share a name — the engine's history keeps 128 entries
/// and its log keeps 256 — so the message names the file, and a claim that
/// matches neither is reported against both.
fn say_the_values(values: &[(String, String)]) -> String {
    let mut parts = Vec::new();
    for (value, file) in values {
        // A value with an operator is an expression, and it is shown even when
        // it has no digit in it: `MAX_JSON_MEMORY_BYTES / JSON_EXPANSION` is
        // not a re-export. §147.
        let expression = value.split_whitespace().any(is_an_operator);
        let shown = if expression || value.chars().any(|c| c.is_ascii_digit()) {
            value.as_str()
        } else {
            "a re-export"
        };
        parts.push(format!("`{shown}` in {file}"));
    }
    parts.join(" or ")
}

/// Whether a name is shaped like a constant.
fn is_constant_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() >= 3
        && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Whether a number in prose is the constant's value.
///
/// `10 000` and `10_000` are the same number written for different readers.
fn numbers_agree(claimed: &str, value: &str) -> bool {
    let claimed: String = claimed.chars().filter(|c| c.is_ascii_digit()).collect();
    let value: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    !claimed.is_empty() && claimed == value
}

/// Whether an expression in prose is the constant's value.
fn expressions_agree(claimed: &str, value: &str) -> bool {
    let squeeze = |text: &str| text.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    squeeze(claimed) == squeeze(value)
}

/// Whether the span is the constant's value, and not a mention beside it.
///
/// A span that follows a constant's name is a **claim** about that constant
/// only if it could be the code's value for it. Two things decide that, and
/// both are needed:
///
/// - the span has to be an expression at all — every operand an identifier
///   or a number, so a path is not one;
/// - and it has to be *this* constant's expression — the same operators, or
///   an operand in common with it.
///
/// The second test is what keeps a path out. `docs/patterns/exact-time-not-
/// float-seconds.md` says the range used to be `i128::MIN / 4` to
/// `i128::MAX / 4`, and `craftkit_core::ids::MAX` is a real constant whose
/// value is `(1 << 52) - 1`: the same shape would not save it, but no operand
/// is in common, so the sentence is a mention and not a claim. §147.
fn is_a_value_claim(claimed: &str, values: &[(String, String)]) -> bool {
    let Some((claimed_operators, claimed_operands)) = expression_parts(claimed) else { return false };
    for (value, _) in values {
        let Some((value_operators, value_operands)) = expression_parts(value) else { continue };
        let same_shape = value_operators == claimed_operators;
        let shares = value_operands.iter().any(|operand| claimed_operands.contains(operand));
        if same_shape || shares {
            return true;
        }
    }
    false
}

/// The operators and the operands of an expression.
///
/// `None` when the text is not an expression: every operand has to be an
/// identifier or a number, so `crates/core/src/lib.rs` — whose last segment
/// is not an identifier — is not one. The operands are keyed by their last
/// `::` segment, so `craftkit_core::JSON_EXPANSION` and `JSON_EXPANSION` are
/// the same operand.
fn expression_parts(text: &str) -> Option<(Vec<String>, Vec<String>)> {
    let mut operators = Vec::new();
    let mut operands = Vec::new();
    for token in text.split_whitespace() {
        if is_an_operator(token) {
            operators.push(token.to_string());
            continue;
        }
        let trimmed = token.trim_matches(|c| c == '(' || c == ')');
        let key = operand_key(trimmed)?;
        operands.push(key);
    }
    if operators.is_empty() {
        return None;
    }
    Some((operators, operands))
}

/// Whether a token is an arithmetic operator.
fn is_an_operator(token: &str) -> bool {
    matches!(token, "*" | "/" | "+" | "-" | "%" | "<<" | ">>" | "&" | "|" | "^")
}

/// The key of an operand: its last `::` segment.
///
/// `None` when the token is not an identifier or a number.
fn operand_key(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let last = token.rsplit("::").next().unwrap_or(token);
    if last.is_empty() {
        return None;
    }
    if last.chars().all(|c| c.is_ascii_digit() || c == '_' || c == '.')
        && last.chars().any(|c| c.is_ascii_digit())
    {
        return Some(last.to_string());
    }
    let mut chars = last.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(last.to_string())
}

/// Every Rust file in the tree.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_rust(root, &mut found).unwrap_or_default();
    found
}

/// Every limit in the model's table is enforced somewhere in the tree.
///
/// The table in `crates/model/src/lib.rs` is the tree's list of safety limits,
/// and two documents quote it as one: the architecture table and the recipe
/// for adding a saved field. A limit nobody enforces is a wish, and a wish in
/// a table of limits is worse than no table at all — an agent told
/// "`MAX_EDGES` is 20 000" caps something with it and believes it is
/// protected. §74 found one: `MAX_EDGES` bounded "the reverse index", and
/// there is no reverse index — `Document::index_of` is a scan, bounded by
/// `MAX_NODES`.
///
/// The test is that every constant the table declares is **used in code**
/// outside the table itself. A mention in a comment is not enforcement, and
/// neither is a mention in prose, so both are left out.
///
/// A mention in a **test** is not enforcement either: a test that reads the
/// limit to compare it with another limit is a sentence about the limit, in
/// code. The tree's own `crates/model/src/lib.rs` has a test that reads
/// `MAX_PROPERTIES` to check that the limits are ordered, and that read is
/// perfectly good code — and it enforces nothing. §150.
pub fn limits_are_enforced(root: &Path) -> Result<Vec<String>, String> {
    let table = root.join("crates/model/src/lib.rs");
    let source =
        std::fs::read_to_string(&table).map_err(|e| format!("could not read the limits table: {e}"))?;
    let code = code_lines_kept(&production_of(&source));
    let Some((declared, rest)) = limits_table(&code) else {
        return Ok(vec!["the model's limits table could not be found".to_string()]);
    };
    if declared.is_empty() {
        return Ok(vec!["the model's limits table declares no limits at all".to_string()]);
    }
    let mut used: Vec<String> = Vec::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let body = if path == table { rest.clone() } else { code_lines_kept(&production_of(&text)) };
        for name in &declared {
            if body.contains(name.as_str()) && !used.contains(name) {
                used.push(name.clone());
            }
        }
    }
    let mut problems = Vec::new();
    for name in declared {
        if !used.contains(&name) {
            problems.push(format!(
                "`{name}` is in the model's limits table and nothing in the tree uses it: \
                 a limit nobody enforces is a wish"
            ));
        }
    }
    Ok(problems)
}

/// The production code of a file: everything except its test items.
///
/// The tests are not the production code, and a gate whose subject is a
/// *production* property has to say so: a test may name the thing to prove
/// that the production code does not do it. §150.
///
/// A **test item** is a `mod tests { … }` block, or a top-level item marked
/// `#[cfg(test)]`. Both are removed **whole**, line by line, so that
/// production code *after* a test item is still production code: a cut that
/// truncated the file at the first test item would hide a limit used further
/// down, and would hide a panic boundary caught further down. §153.
///
/// An **indented** `#[cfg(test)]` is not a test item: it marks a method
/// inside an impl block, and the methods around it are production code.
fn production_of(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for line in text.lines() {
        if skipping {
            // A top-level item ends at the first `}` in the first column:
            // everything inside it is indented.
            if line == "}" || line == ";" {
                skipping = false;
            }
            out.push('\n');
            continue;
        }
        if line.starts_with("mod tests {") || line.starts_with("#[cfg(test)]") {
            skipping = true;
            out.push('\n');
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The limits the model's table declares, and the rest of the file.
///
/// The table is a `pub mod limits { … }` block, so the block is found by its
/// braces rather than by a marker: the file's own text says where the
/// constants are, and a marker would be a second place to forget.
fn limits_table(code: &str) -> Option<(Vec<String>, String)> {
    let start = code.find("pub mod limits")?;
    let open = code[start..].find('{')? + start;
    let mut depth = 0usize;
    let mut end = None;
    for (at, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + at);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let block = &code[open..end];
    let declared = constants_in(block).into_iter().map(|(name, _)| name).collect();
    let rest = format!("{}{}", &code[..open], &code[end..]);
    Some((declared, rest))
}

/// A citation that names a test, and the line it points at.
///
/// §45: four citations of the form "`the_test` (`file.rs:123`)" pointed at
/// lines 9, 52, 100 and 147 away from the test they name. A line number
/// rots; §36 says to cite the symbol instead, and these four had not been
/// caught because `citation_drift` only asks whether the line is *in* the
/// file, and a doc comment is.
///
/// The rule is narrow on purpose: the citation must be **paired** with a
/// test-shaped name, next to it in the same paragraph, in either order. A
/// citation of a file's *region* — a struct field, a block of code — is not
/// a claim about one symbol, and the tree has plenty of those.
pub fn citations_of_tests_point_at_the_test(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, paragraph) in paragraphs(&text) {
            let spans = backticked_spans(&paragraph);
            for pair in spans.windows(2) {
                if pair[1].0.saturating_sub(pair[0].2) > 40 {
                    continue;
                }
                for (left, right) in [(&pair[0], &pair[1]), (&pair[1], &pair[0])] {
                    let Some((file, line)) = cited_line(right.1) else { continue };
                    let name = left.1;
                    if !is_test_name(name) {
                        continue;
                    }
                    let path = root.join(&file);
                    let Ok(body) = std::fs::read_to_string(&path) else { continue };
                    let lines: Vec<&str> = body.lines().collect();
                    if line == 0 || line > lines.len() {
                        continue;
                    }
                    let Some(definition) = line_of_test(&body, name) else { continue };
                    if definition.abs_diff(line) > 2 {
                        let at = number + paragraph[..left.0].bytes().filter(|b| *b == b'\n').count();
                        problems.push(format!(
                            "{relative}: line {at} cites `{name}` at `{file}:{line}`, and it is at line {definition}"
                        ));
                    }
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// A `file.rs:123` citation, if that is what the span is.
fn cited_line(span: &str) -> Option<(String, usize)> {
    let (file, line) = span.rsplit_once(':')?;
    if !file.ends_with(".rs") || !file.contains('/') {
        return None;
    }
    let Ok(line) = line.parse::<usize>() else { return None };
    Some((file.to_string(), line))
}

/// Whether a name is shaped like a test.
fn is_test_name(name: &str) -> bool {
    let mut parts = name.split('_');
    let first = parts.next().unwrap_or("");
    if !matches!(first, "the" | "a" | "an" | "every" | "each") {
        return false;
    }
    name.len() > 4 && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
}

/// The line a test is defined on.
fn line_of_test(body: &str, name: &str) -> Option<usize> {
    body.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&format!("fn {name}(")))
        .map(|(index, _)| index + 1)
}

/// The mutation script's closing ratio, and whether it is computed.
///
/// §71: the script's own entry in `KNOWN-GAPS.md` says the number in its
/// closing line "is computed by counting the cases the script runs, not
/// written by hand" — and a hard-coded ratio would be an instrument that
/// reports a number about itself without looking, which is the defect §1 is
/// about, in the instrument that found it.
///
/// **This gate cannot be mutation-tested**, and the reason is the medium
/// rather than the will: bash reads a script as it goes, so a file that
/// changes under a running script is a corrupted run. The check is proved by
/// a fabricated copy instead — the same shape as the fabricated-id case in
/// §26.
pub fn mutation_ratio_is_computed(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(index) = lines.iter().position(|l| l.contains("%d/%d caught")) else {
        return vec!["the mutation script prints no closing ratio at all".to_string()];
    };
    let arguments = lines.get(index + 1).copied().unwrap_or_default();
    let mut problems = Vec::new();
    if !arguments.contains("${#CAUGHT[@]}") || !arguments.contains("$CASES") {
        problems.push(format!(
            "the mutation script's closing ratio is written by hand: {arguments}\n     \
             It must count the cases the run had, or it is a number about the script rather than \
             a result of it."
        ));
    }
    problems
}

/// The gate.
pub fn run(root: &Path) -> Result<crate::Outcome, String> {
    step("lists");
    let mut problems = Vec::new();

    let roots = crate_roots(root)?;
    if roots.is_empty() {
        return Err("found no crate roots at all; the scan is wrong, not the repository".to_string());
    }
    problems.extend(check_roots(&roots));

    // The layering table, in the code and in the documentation.
    let architecture = root.join(ARCHITECTURE);
    let markdown = match std::fs::read_to_string(&architecture) {
        Ok(text) => text,
        Err(e) => return Err(format!("could not read {ARCHITECTURE}: {e}")),
    };
    let rows = parse_table(&markdown);
    if rows.is_empty() {
        problems.push(format!(
            "{ARCHITECTURE} has no parseable layering table.\n\
             \x20    Expected rows like: | L3 | `craftkit-eval` | core, model, kernel |"
        ));
    } else {
        let crates = layers::load(root)?;
        problems.extend(compare_table(&rows, &crates));
    }

    // The workspace member list. This used to be checked only by a unit test,
    // which meant `cargo xtask lists` said "lists are sorted" while the list a
    // change actually collides on was not among them. Found by
    // `scripts/mutation-test-gates.sh`.
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| format!("could not read Cargo.toml: {e}"))?;
    let members = workspace_members(&manifest);
    if members.is_empty() {
        problems.push("Cargo.toml has no members list to check".to_string());
    } else if !is_sorted(&members) {
        let mut sorted = members.clone();
        sorted.sort();
        problems.push(format!(
            "the workspace members list is not sorted.\n     now:  {}\n     want:  {}\n     \
             almost every change that adds a crate edits this list, and an unsorted one turns that \
             into a merge conflict about nothing.",
            members.join(", "),
            sorted.join(", ")
        ));
    }

    // The `mod` groups of this crate.
    for path in [root.join("xtask/src/lib.rs"), root.join("apps/craftkit-web/src/main.rs")] {
        let Ok(source) = std::fs::read_to_string(&path) else { continue };
        for group in mod_groups(&source) {
            if !is_sorted(&group) {
                problems.push(format!(
                    "{}: these `mod` lines are not sorted: {}\n     Sorted: {}",
                    path.display(),
                    group.join(", "),
                    {
                        let mut sorted = group.clone();
                        sorted.sort();
                        sorted.join(", ")
                    }
                ));
            }
        }
    }

    // Citations in prose that point at nothing. See `citation_drift`.
    problems.extend(
        citation_drift(root)?.into_iter().map(|p| format!("{p}\n     Cite the symbol, not the line number.")),
    );

    // Dotted names in prose that name a command the registry does not have.
    problems.extend(
        unknown_command_ids(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Run `craftkit-cli commands` for the ids that exist.")),
    );

    // The README's second copy of the layer table. `docs/architecture.md` is
    // checked; the README is decoration until now.
    problems.extend(
        readme_layers(root)?
            .into_iter()
            .map(|p| format!("{p}\n     The code table in xtask/src/layers.rs is the enforced one.")),
    );

    // A symbol that prose says is in a file, and is not.
    problems.extend(
        symbol_file_claims(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Name the test that is actually there.")),
    );

    // Bare line citations: a line number with no file. §36 measured thirteen
    // wrong ones out of twenty-four, and no gate could see them.
    problems.extend(
        bare_line_citations(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Cite the symbol, or name the file.")),
    );

    // Documents pointing at documents. `AGENTS.md` sent every agent to
    // `docs/control-protocol.md`, which does not exist and never has.
    problems.extend(
        broken_document_links(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Point it at a file that is in the tree.")),
    );

    // The control-channel examples in prose. They are the requests an agent
    // copies, so one that could not be sent is a defect in the only example
    // there is. What it checks is mechanical: JSON, a real method, and no
    // self-contradiction. Whether the request is a sensible one is not this
    // gate's business — §32.
    problems.extend(
        prose_examples_in_docs_resolve_to_real_methods(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Send it, or drop the example.")),
    );

    // A function a document quotes, and the function it is quoting. §40.
    problems.extend(
        quoted_code_matches_the_source(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Quote the code, or stop calling the block exact.")),
    );

    // A line count in prose, and the file it is about. §41.
    problems.extend(
        prose_line_counts_match_the_files(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Count the file, or drop the number.")),
    );

    // The instrument's own closing ratio: computed, or written by hand. §71.
    if let Ok(script) = std::fs::read_to_string(root.join("scripts/mutation-test-gates.sh")) {
        problems.extend(
            mutation_ratio_is_computed(&script)
                .into_iter()
                .map(|p| format!("{p}\n     The ratio is the number the run produced.")),
        );
    }

    // Code that prose attributes to a file, and the file it attributes it to. §42.
    problems.extend(
        attributed_code_is_in_the_file(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Name the symbol that is really there.")),
    );

    // A constant in prose, and the value the code gives it. §41.
    problems.extend(
        prose_constants_match_the_code(root)?
            .into_iter()
            .map(|p| format!("{p}\n     The limit in the code is the one that counts.")),
    );

    // A citation that names a test, and the line it points at. §45.
    problems.extend(
        citations_of_tests_point_at_the_test(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Cite the symbol, not the line.")),
    );

    // A boundary that catches a panic, and the document that declares it. §78.
    problems.extend(
        panic_boundaries_are_declared(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A boundary has to be written down before it is written.")),
    );

    // A number of commands in prose, and the registry's number. §77.
    problems.extend(counts_in_prose_match_the_code(root)?.into_iter().map(|p| {
        format!("{p}\n     A number written by hand about a thing that grows is stale by construction.")
    }));

    // A limit in the model's table, and the code that enforces it. §74.
    problems.extend(
        limits_are_enforced(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A limit nobody enforces protects nothing.")),
    );

    // A field in the model, and the hash that has to see it. §80.
    problems.extend(
        fingerprint_sees_every_field(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A field the hash forgets is a stale answer with no warning.")),
    );

    // The control channel's methods, as the code has them and as the prose
    // has them. §82.
    problems.extend(
        control_methods_are_listed(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A list of methods with a hole in it is the wrong list.")),
    );

    // A module path in prose, and the module it names. §130.
    problems.extend(
        unknown_module_paths(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Name a module this tree has, or drop the name.")),
    );

    // A type path in prose, and the member it names. §143.
    problems.extend(
        unknown_type_paths(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Name a member this type has, or drop the name.")),
    );

    // The shell, and the evaluator it may read but not call. §132.
    problems.extend(
        the_shell_does_not_evaluate(root)?
            .into_iter()
            .map(|p| format!("{p}\n     The engine owns the evaluation; the shell reads the plan.")),
    );

    // The session, and the roads into the design it does not have. §135.
    problems.extend(
        the_session_offers_no_second_path(root)?
            .into_iter()
            .map(|p| format!("{p}\n     There is no second path: the registry is the way in.")),
    );

    // The shell, and the documents it may not put into the session. §135.
    problems.extend(
        the_shell_does_not_write_the_document(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A panel reads the design; the host opens it.")),
    );

    // The raw parameter reader, and the two homes it has. §136.
    problems.extend(
        the_raw_parameter_reader_has_two_homes(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A parameter is read by a reader, and never indexed.")),
    );

    // The clippy settings, and the lints they re-enable. §136.
    problems.extend(
        every_test_only_setting_names_a_denied_lint(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A setting that re-enables nothing enforces nothing.")),
    );

    // The design file's cap, and the budget it has to come from. §141.
    problems.extend(
        the_file_cap_is_derived_from_the_designs_budget(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A cap that is a number of its own drifts from the budget.")),
    );

    // The tree's JSON, and the writer it is written with. §142.
    problems.extend(
        the_tree_writes_its_json_with_one_writer(root)?
            .into_iter()
            .map(|p| format!("{p}\n     One design, one writer.")),
    );

    // The memory budgets, and the budget they have to come from. §144.
    problems.extend(
        the_memory_budgets_are_derived_from_the_models(root)?
            .into_iter()
            .map(|p| format!("{p}\n     A budget that is a number of its own drifts from the budget.")),
    );

    // The bounds the tree has, and the ones its prose names. §145.
    problems.extend(
        the_byte_bounds_are_named_in_the_prose(root)?
            .into_iter()
            .map(|p| format!("{p}\n     Name the bound, so a reader can look it up.")),
    );

    // The shell's state, and the values it may not hold. §138.
    problems.extend(
        the_shell_caches_no_evaluated_value(root)?
            .into_iter()
            .map(|p| format!("{p}\n     The plan is read every frame; the state holds no copy.")),
    );

    // Every parameter a command reads is declared, and every parameter it
    // declares is read. §139.
    problems.extend(every_parameter_a_command_reads_is_declared(root)?.into_iter().map(|p| {
        format!(
            "{p}\n     A parameter the engine refuses is a command that cannot be used; \
                     a parameter nobody reads is a value the caller believes in."
        )
    }));

    if problems.is_empty() {
        println!(
            "   ok: {} crate roots share one deny line, the layer table matches the docs, \
             lists are sorted, every `file.rs:12` and every `.md` reference in prose points at \
             something, the README's layer table matches the code, no citation names a line \
             without a file, every symbol claimed to be in a file is in it, every exact block \
             quotes the code it claims to, every limit in prose is the constant's value, every \
             citation of a test points at that test, every control-channel example in prose \
             could be sent, every limit in the model's table is enforced in code, every \
             number of commands in prose is the registry's number, every boundary that \
             catches a panic is declared in the pattern document, every field the \
             fingerprint hashes is a field the model has, the control channel's \
             methods are the methods the prose lists, every module path in prose \
             names a module this tree has, every type path in prose names a \
             member the type has, every memory budget is derived from the \
             model's, every byte bound is named in the prose, the shell never \
             evaluates the \
             document, the shell never writes the document, the raw parameter \
             reader has two homes, every clippy setting names a denied lint, the \
             shell's state holds no evaluated value, every parameter a command \
             reads is declared, and the session offers no way into the design \
             but the registry",
            roots.len()
        );
        return Ok(crate::Outcome::Ran);
    }
    Err(format!("{} shared-list problem(s):\n   • {}", problems.len(), problems.join("\n   • ")))
}

// --------------------------------------------------------------- §82 -------

/// The control channel's methods, as the code defines them.
///
/// The names come from `Method::as_str`, which is the one place a method's
/// wire name is written down.
fn control_methods(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("crates/control/src/protocol.rs");
    let source =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();
    let Some(start) = lines.iter().position(|l| l.trim().starts_with("pub fn as_str")) else {
        return Err(format!("{} has no `Method::as_str` to read the methods from", path.display()));
    };
    let body = function_lines(&lines, start);
    let mut names = Vec::new();
    for line in &body {
        let trimmed = line.trim();
        let Some((_, literal)) = trimmed.split_once("=>") else { continue };
        let literal = literal.trim().trim_end_matches(',').trim();
        let Some(name) = literal.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
            continue;
        };
        names.push(name.to_string());
    }
    if names.is_empty() {
        return Err(format!("no method names found in {}'s `as_str`", path.display()));
    }
    Ok(names)
}

/// The methods the recipe's table lists.
///
/// The table is the canonical list — it is the one place the methods are
/// written as a list, and the one an agent is sent to.
fn methods_in_the_recipe_table(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("docs/recipes/add-a-control-channel-method.md");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let mut names = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("| `") {
            continue;
        }
        let cell = trimmed.split('|').nth(1).unwrap_or("").trim();
        let Some(name) = cell.split('`').nth(1) else { continue };
        names.push(name.to_string());
    }
    Ok(names)
}

/// The control channel's methods, as the code has them and as the prose has
/// them.
///
/// §82: the methods are written out by hand in three places — the recipe's
/// table, the checking skill's list, and `AGENTS.md`'s sentence — and nothing
/// checked any of them. `AGENTS.md` said *"Its methods are `engine.execute`,
/// `engine.describe`, `session.state`, `diagnostics.list` and `ui`"*, which is
/// five of the eight: `diagnostics.crash_reports`, `ping` and `shutdown` were
/// missing, and the sentence reads as the whole list. A reader of `AGENTS.md`
/// would not know three of the eight methods existed.
///
/// Two rules, because two things can go wrong:
///
/// * The recipe's table is **exactly** the code's methods — in both
///   directions. A table is a list, so it can be compared; a paragraph cannot.
/// * A paragraph that names **more than half** of the methods is talking about
///   the method set, and a set with a hole in it is the wrong set. The
///   threshold is what keeps this from firing on a paragraph that happens to
///   mention a method in passing.
pub fn control_methods_are_listed(root: &Path) -> Result<Vec<String>, String> {
    let methods = control_methods(root)?;
    let listed = methods_in_the_recipe_table(root)?;
    let mut problems = Vec::new();
    for name in &methods {
        if !listed.contains(name) {
            problems.push(format!(
                "the control method `{name}` is not in the table in \
                 docs/recipes/add-a-control-channel-method.md"
            ));
        }
    }
    for name in &listed {
        if !methods.contains(name) {
            problems.push(format!(
                "the table in docs/recipes/add-a-control-channel-method.md lists `{name}`, \
                 which is not a method the code has"
            ));
        }
    }

    let half = methods.len() / 2;
    for doc in prose_files(root) {
        let Ok(text) = std::fs::read_to_string(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, paragraph) in paragraphs(&text) {
            let named: Vec<&str> = methods
                .iter()
                .map(|m| m.as_str())
                .filter(|m| paragraph.contains(&format!("`{m}`")))
                .collect();
            if named.len() <= half {
                continue;
            }
            for method in &methods {
                if !named.contains(&method.as_str()) {
                    problems.push(format!(
                        "{relative}: line {number} names {} of the {} methods and not `{method}`",
                        named.len(),
                        methods.len()
                    ));
                }
            }
        }
    }
    Ok(problems)
}

// --------------------------------------------------------------- §80 -------

/// Every field of every type the fingerprint hashes is hashed.
///
/// The fingerprint is a hand-written hash. It was moved out of the evaluator
/// and into the model in §53, because hashing the serialised form cost 52 ms
/// where the evaluation it was meant to save cost 7 µs. The price of a
/// hand-written hash is that a field the model grows and the hash forgets is a
/// **silent wrong answer**: the evaluator's cache is keyed on the fingerprint,
/// so it serves a state computed from the old value, and nothing says so.
///
/// The guard against that was a test in the same file, and the test was a
/// hand-written list of the same fields the hash covers — a list that drifts
/// with the thing it checks. Measured: removing the keyframe's easing from
/// the hash left **every test in the workspace green**, because the test
/// changed a key's time, value and easing together and only needed one of
/// them to move the fingerprint. §80.
///
/// So this check reads the tree: the fields come from the struct definitions
/// and the hashed ones from the hash's own source. A field that is in the
/// model and nowhere in the hash is a gate failure, not a silent wrong
/// answer.
///
/// # What it does not cover, and why
///
/// * An **enum** is matched exhaustively by the hash, so a new variant is a
///   compile error rather than a silent wrong answer.
/// * A **tuple struct** is reached through an accessor — `Keyframes` is
///   `Keyframes(Vec<Key>)` — and this check reads fields, not accessors. What
///   it holds is checked through `Key`.
pub fn fingerprint_sees_every_field(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("crates/model/src/fingerprint.rs");
    let source =
        std::fs::read_to_string(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();
    let targets = fingerprint_targets(&lines);
    if targets.is_empty() {
        return Ok(vec!["the fingerprint has no hash block for this check to read".to_string()]);
    }
    let sources = rust_files(root);
    let mut problems = Vec::new();
    for (ty, receiver, body) in targets {
        let Some((_, shape)) = shape_of(&sources, &ty) else {
            problems.push(format!("the fingerprint hashes `{ty}`, and the tree does not define it"));
            continue;
        };
        let Shape::Named(fields) = shape else { continue };
        // Comments and strings are left out: a sentence about a field is not a
        // read of it.
        let code = code_lines_kept(&body.join("\n"));
        for field in fields {
            if !reads_field(&code, &receiver, &field) {
                problems.push(format!(
                    "the fingerprint does not hash `{ty}`'s field `{field}` \
                     (crates/model/src/fingerprint.rs)"
                ));
            }
        }
    }
    Ok(problems)
}

/// The shape of a type, as the tree declares it.
#[derive(Debug)]
enum Shape {
    /// A struct with named fields.
    Named(Vec<String>),
    /// A struct with positional fields.
    Tuple,
    /// An enum, whose variants the hash matches exhaustively.
    Enum,
}

/// The shape of `name`, and the file that declares it.
fn shape_of(sources: &[PathBuf], name: &str) -> Option<(PathBuf, Shape)> {
    for path in sources {
        let Ok(text) = std::fs::read_to_string(path) else { continue };
        let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        for (index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let Some(rest) = after_visibility(trimmed) else { continue };
            if let Some(rest) = rest.strip_prefix("enum ") {
                if identifier(rest) == name {
                    return Some((path.clone(), Shape::Enum));
                }
                continue;
            }
            let Some(rest) = rest.strip_prefix("struct ") else { continue };
            if identifier(rest) != name {
                continue;
            }
            let tail = rest[rest.find(char::is_whitespace).unwrap_or(rest.len())..].trim_start();
            if !tail.starts_with('{') {
                return Some((path.clone(), Shape::Tuple));
            }
            let owned: Vec<String> = lines[index..].iter().map(|l| l.to_string()).collect();
            return Some((path.clone(), Shape::Named(named_fields(&owned))));
        }
    }
    None
}

/// The named fields of a struct, without the visibility in front of them.
///
/// `pub id: NodeId` is the field `id`; the `pub` is how it is published, not
/// what it is called.
fn named_fields(lines: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines.iter().skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with('}') {
            break;
        }
        if trimmed.starts_with("#[") || trimmed.starts_with("///") || trimmed.starts_with("//") {
            continue;
        }
        let head = trimmed.split("//").next().unwrap_or(trimmed).trim();
        let head = head.trim_end_matches(',').trim();
        let Some((name, _)) = head.split_once(':') else { continue };
        let name = name.trim();
        let name = name.rsplit(' ').next().unwrap_or(name).trim();
        if name.is_empty() {
            continue;
        }
        out.push(name.to_string());
    }
    out
}

/// The blocks of the fingerprint that hash a type: (type, receiver, body).
///
/// The blocks are found by their shape — a method called `fingerprint` or
/// `hash_into`, and a free function that hashes one type — so a type the hash
/// walks is a type the check looks at, with no list to keep in step with the
/// file.
fn fingerprint_targets(lines: &[String]) -> Vec<(String, String, Vec<String>)> {
    let impls = enclosing_impls(lines);
    let mut out = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = impls[index].clone() {
            let Some(rest) = after_visibility(trimmed) else { continue };
            if rest.starts_with("fn fingerprint(") || rest.starts_with("fn hash_into(") {
                out.push((name, "self".to_string(), function_lines(lines, index)));
            }
            continue;
        }
        if trimmed.starts_with("fn hash_")
            && let Some((ty, receiver)) = hashed_parameter(trimmed)
        {
            out.push((ty, receiver, function_lines(lines, index)));
        }
    }
    out
}

/// The impl each line sits in, if any.
fn enclosing_impls(lines: &[String]) -> Vec<Option<String>> {
    let mut out = Vec::with_capacity(lines.len());
    let mut current: Option<String> = None;
    for line in lines {
        let trimmed = line.trim();
        let indent = line.len() - line.trim_start().len();
        if let Some(rest) = trimmed.strip_prefix("impl ") {
            current = Some(identifier(rest));
        } else if trimmed == "}" && indent == 0 {
            current = None;
        }
        out.push(current.clone());
    }
    out
}

/// The type and the variable a free hash function hashes, as (type, variable).
///
/// `fn hash_key(h: &mut Fnv, key: &Key)` hashes a `Key`, and reads it through
/// `key`.
fn hashed_parameter(signature: &str) -> Option<(String, String)> {
    let open = signature.find('(')?;
    let close = signature[open..].find(')')? + open;
    let inner = &signature[open + 1..close];
    let last = inner.rsplit(',').next()?.trim();
    let (name, ty) = last.split_once(':')?;
    let ty = ty.trim().trim_start_matches('&').trim();
    if ty.is_empty() {
        return None;
    }
    Some((ty.to_string(), name.trim().to_string()))
}

/// The identifier at the start of `text`.
fn identifier(text: &str) -> String {
    text.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
}

/// Whether `body` reads `receiver.field`.
///
/// The whole name has to be there: `self.id` is a read of `id`, and `self.idx`
/// is not.
fn reads_field(body: &str, receiver: &str, field: &str) -> bool {
    let needle = format!("{receiver}.{field}");
    let mut at = 0usize;
    while let Some(found) = body[at..].find(needle.as_str()) {
        let start = at + found;
        let end = start + needle.len();
        let before = body[..start].chars().last();
        let after = body[end..].chars().next();
        let before_ok = before.map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true);
        let after_ok = after.map(|c| !(c.is_alphanumeric() || c == '_')).unwrap_or(true);
        if before_ok && after_ok {
            return true;
        }
        at = end;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree's own limits table: every limit in it is enforced.
    ///
    /// The real tree, not a copy of it. §74 found a limit in this table that
    /// nothing enforced, and this is the check that notices the next one.
    #[test]
    fn every_limit_in_the_table_is_enforced() {
        let root = crate::root().expect("the workspace root");
        let problems = limits_are_enforced(&root).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A limit nobody enforces is caught.
    ///
    /// The check reads the tree, so the tree it reads has to be *made*: a
    /// limits table with a constant in it that nothing uses, and no code
    /// that uses it. The fabricated copy is the proof that the check bites —
    /// the same shape as §26's fabricated command id.
    #[test]
    fn a_limit_nobody_enforces_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-limits-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        let table = model.join("lib.rs");
        std::fs::write(
            &table,
            "pub mod limits {\n    /// A limit nothing uses.\n    pub const MAX_UNUSED: usize = 7;\n}\n",
        )
        .expect("the fabricated table");
        // A file that uses a *different* limit, and not this one.
        std::fs::write(model.join("other.rs"), "fn f() -> usize { limits::MAX_OTHER }\n")
            .expect("the fabricated code");

        let problems = limits_are_enforced(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_UNUSED"), "{problems:#?}");

        // And the same tree with the limit used is clean.
        std::fs::write(model.join("other.rs"), "fn f() -> usize { limits::MAX_UNUSED }\n")
            .expect("the fabricated code");
        let problems = limits_are_enforced(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A mention in a comment is not enforcement.
    ///
    /// The check has to see code: a doc comment that names the limit is a
    /// sentence about it, and a sentence is not a bound.
    #[test]
    fn a_limit_only_a_comment_mentions_is_still_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-limits-comment-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::write(
            model.join("lib.rs"),
            "pub mod limits {\n    /// A limit nothing uses.\n    pub const MAX_UNUSED: usize = 7;\n}\n",
        )
        .expect("the fabricated table");
        std::fs::write(model.join("other.rs"), "// the limit is limits::MAX_UNUSED\nfn f() -> usize { 1 }\n")
            .expect("the fabricated code");

        let problems = limits_are_enforced(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_UNUSED"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A claimed expression the gate does not recognise is still checked.
    ///
    /// The gate used to follow only an expression holding `<<` or `*`, so the
    /// tree's own claim that `MAX_JSON_TEXT_BYTES` is
    /// `MAX_JSON_MEMORY_BYTES / JSON_EXPANSION` was never compared with the
    /// code: the divisor could change and the prose would keep claiming the
    /// old one. §147.
    #[test]
    fn a_claimed_expression_the_gate_does_not_know_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-claim-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        let source = |divisor: &str| {
            // One physical line, with the newline as an escape: a string
            // broken across two lines puts a "pub const" at the left margin
            // of the second one, and the gate reads the tree's constants out
            // of the source text — so a test that fabricates a constant
            // would fabricate one for the tree. §147.
            format!(
                "pub const MAX_JSON_MEMORY_BYTES: usize = 32 * 1024 * 1024;\npub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / {divisor};"
            )
        };
        std::fs::write(core.join("lib.rs"), source("JSON_EXPANSION")).expect("the fabricated source");
        std::fs::write(
            docs.join("claim.md"),
            "The number is derived from a memory budget rather than chosen on its own: \
             `MAX_JSON_TEXT_BYTES` is `MAX_JSON_MEMORY_BYTES / JSON_EXPANSION`.\n",
        )
        .expect("the fabricated claim");

        let problems = prose_constants_match_the_code(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        std::fs::write(core.join("lib.rs"), source("16")).expect("the fabricated source");
        let problems = prose_constants_match_the_code(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_JSON_TEXT_BYTES"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A path beside a constant's name is a mention, not a claim.
    ///
    /// The tree's prose names a file beside a constant in dozens of places,
    /// and a file path is not an expression: `crates/core/src/lib.rs` has no
    /// operand in it that a constant could have written.
    #[test]
    fn a_path_beside_a_constant_is_not_a_claim() {
        let dir = std::env::temp_dir().join(format!("craftkit-claim-path-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(
            core.join("lib.rs"),
            "pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / JSON_EXPANSION;\n",
        )
        .expect("the fabricated source");
        std::fs::write(
            docs.join("claim.md"),
            "`MAX_JSON_TEXT_BYTES` is set in `crates/core/src/lib.rs`, and it is the limit.\n",
        )
        .expect("the fabricated mention");

        let problems = prose_constants_match_the_code(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An expression with no operand in common is not a claim either.
    ///
    /// `docs/patterns/exact-time-not-float-seconds.md` says the range used to
    /// be `i128::MIN / 4` to `i128::MAX / 4`, and `craftkit_core::ids::MAX` is
    /// a real constant whose value is `(1 << 52) - 1`. The sentence is about
    /// `i128::MAX`, not about the tree's constant, and the gate has to be
    /// able to tell the two apart.
    #[test]
    fn an_expression_with_no_operand_in_common_is_not_a_claim() {
        let dir = std::env::temp_dir().join(format!("craftkit-claim-mention-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(core.join("ids.rs"), "pub const MAX: u64 = (1 << 52) - 1;\n")
            .expect("the fabricated source");
        std::fs::write(
            docs.join("mention.md"),
            "The range is the one a reply can hold: `i64::MIN` to `u64::MAX`, the integers a JSON \
             number holds. It was wider - `i128::MIN / 4` to `i128::MAX / 4` - and that broke it.\n",
        )
        .expect("the fabricated mention");

        let problems = prose_constants_match_the_code(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A constant that stands only inside a string is not a constant of the tree.
    ///
    /// The gates read the tree's limits out of its source, and the tree's
    /// tests fabricate source files — in strings. A string written across two
    /// lines puts a `pub const` at the left margin of the second one, and a
    /// gate that reads the *text* cannot tell that from code. Measured: a
    /// design file's cap that stood only inside a string satisfied the gate
    /// that exists to check that cap; so did the undo stacks' budget; and a
    /// fabricated constant **masked** a real drift in the prose, because the
    /// gate accepts a claim that matches *any* value of that name. §148.
    #[test]
    fn a_constant_that_stands_only_in_a_string_is_not_a_constant_of_the_tree() {
        let dir = std::env::temp_dir().join(format!("craftkit-string-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let io = dir.join("crates/io/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&io).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        // The code says one thing...
        std::fs::write(
            core.join("lib.rs"),
            "pub const MAX_JSON_MEMORY_BYTES: usize = 32 * 1024 * 1024;\n\
             pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / 16;\n",
        )
        .expect("the fabricated source");
        // ...and the tree's prose claims another.
        std::fs::write(
            docs.join("claim.md"),
            "`MAX_JSON_TEXT_BYTES` is `MAX_JSON_MEMORY_BYTES / JSON_EXPANSION`.\n",
        )
        .expect("the fabricated claim");
        // A test that fabricates a source file holds a constant in a string,
        // and that constant is not the tree's.
        std::fs::write(
            io.join("format.rs"),
            "const FABRICATED: &str = \"\\\n\
             pub const MAX_JSON_TEXT_BYTES: usize = MAX_JSON_MEMORY_BYTES / JSON_EXPANSION;\n\
             \";\n",
        )
        .expect("the fabricated source");

        let problems = prose_constants_match_the_code(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_JSON_TEXT_BYTES"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A limit that stands only inside a string is not a limit.
    ///
    /// Two gates read a limit out of the tree's source and check what its
    /// statement says: the design file's cap and the undo stacks' budget.
    /// Both read the *text*, so a cap that had left the code and stood in a
    /// string satisfied them. §148.
    #[test]
    fn a_limit_that_stands_only_in_a_string_is_not_a_limit() {
        let dir = std::env::temp_dir().join(format!("craftkit-string-cap-{}", std::process::id()));
        let io = dir.join("crates/io/src");
        let engine = dir.join("crates/engine/src");
        std::fs::create_dir_all(&io).expect("the fabricated tree");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::write(
            io.join("format.rs"),
            "pub const FILE_EXPANSION: usize = 2;\n\
             const FABRICATED: &str = \"\\\n\
             pub const MAX_FILE_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * FILE_EXPANSION;\n\
             \";\n",
        )
        .expect("the fabricated source");
        std::fs::write(
            engine.join("history.rs"),
            "pub const HISTORY_EXPANSION: usize = 2;\n\
             const FABRICATED: &str = \"\\\n\
             pub const MAX_HISTORY_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION;\n\
             \";\n",
        )
        .expect("the fabricated source");
        // The session's own budget is in code, and is right: the gate has to
        // report the history's and nothing else.
        std::fs::write(
            engine.join("session.rs"),
            "pub const SESSION_OWN_DOCUMENTS: usize = 2;\n\
             pub const SESSION_SNAPSHOT_DOCUMENTS: usize = 1;\n\
             pub const HISTORY_EXPANSION: usize = 2;\n\
             pub const SESSION_DOCUMENTS: usize = 4;\n\
             pub const MAX_SESSION_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS;\n",
        )
        .expect("the fabricated source");

        let problems = the_file_cap_is_derived_from_the_designs_budget(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_FILE_BYTES"), "{problems:#?}");

        let problems = the_memory_budgets_are_derived_from_the_models(&dir).expect("the check runs");
        // The gate looks for three budgets; this tree has none of them in
        // code, so what matters is that the fabricated one is among them.
        assert!(problems.iter().any(|p| p.contains("MAX_HISTORY_BYTES")), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A bound that stands only inside a string is not a bound of the tree.
    ///
    /// The gate that says every bound the tree has is named in the tree's
    /// prose read the tree's source as text, so a bound that stood only in a
    /// string was a bound it then said no document named. §148.
    #[test]
    fn a_bound_that_stands_only_in_a_string_is_not_a_bound_of_the_tree() {
        let dir = std::env::temp_dir().join(format!("craftkit-string-bound-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        let docs = dir.join("docs/architecture.md");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::create_dir_all(docs.parent().expect("the documents")).expect("the fabricated tree");
        std::fs::write(
            model.join("lib.rs"),
            "pub const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;\n\
             const FABRICATED: &str = \"\\\n\
             pub const MAX_FABRICATED_BYTES: usize = 1;\n\
             \";\n",
        )
        .expect("the fabricated source");
        std::fs::write(&docs, "The design's budget is `MAX_DOCUMENT_BYTES`.\n").expect("the prose");

        let problems = the_byte_bounds_are_named_in_the_prose(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A symbol that stands only in a string is not a symbol of the tree.
    ///
    /// The gate that checks the claims prose makes about the tree's own files
    /// read the file as **text**, so a test that stood only inside a string
    /// literal satisfied a document's claim about it. Measured: the claim was
    /// true when the test was in the code, true when it stood in a string,
    /// true when it stood in a comment, and false — as it should be — only
    /// when it was nowhere. §149.
    #[test]
    fn a_symbol_that_stands_only_in_a_string_is_not_a_symbol_of_the_tree() {
        let dir = std::env::temp_dir().join(format!("craftkit-symbol-string-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        // The test the document claims is there, standing only in a string.
        std::fs::write(core.join("lib.rs"), "const FABRICATED: &str = \"fn the_ghost_test() {}\";\n")
            .expect("the fabricated source");
        std::fs::write(
            docs.join("claim.md"),
            "The `the_ghost_test` test in `crates/core/src/lib.rs` checks it.\n",
        )
        .expect("the fabricated claim");

        let problems = symbol_file_claims(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("the_ghost_test"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A symbol that stands only in a comment is not a symbol of the tree.
    ///
    /// The same gate, the other half of the rule: a comment is a sentence
    /// about the code, and a sentence is not a test. §149.
    #[test]
    fn a_symbol_that_stands_only_in_a_comment_is_not_a_symbol_of_the_tree() {
        let dir = std::env::temp_dir().join(format!("craftkit-symbol-comment-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(core.join("lib.rs"), "// fn the_ghost_test() {}\n").expect("the fabricated source");
        std::fs::write(
            docs.join("claim.md"),
            "The `the_ghost_test` test in `crates/core/src/lib.rs` checks it.\n",
        )
        .expect("the fabricated claim");

        let problems = symbol_file_claims(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("the_ghost_test"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A module that stands only in a string is not a module of the tree.
    ///
    /// The gate that says every module path in prose names a module this tree
    /// has read the tree's files as **text**, so a `mod` line that stood only
    /// inside a string literal satisfied a document's claim about it. §149.
    #[test]
    fn a_module_that_stands_only_in_a_string_is_not_a_module_of_the_tree() {
        let dir = std::env::temp_dir().join(format!("craftkit-module-string-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        // A `mod` line of its own, standing only inside a string: the shape
        // the reader of the tree's *text* would have taken for a module.
        std::fs::write(core.join("lib.rs"), "const FABRICATED: &str = \"\\\nmod ghostmod;\n\";\n")
            .expect("the fabricated source");
        std::fs::write(docs.join("claim.md"), "The `ghostmod::handle` entry point does the work.\n")
            .expect("the fabricated claim");

        let problems = unknown_module_paths(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("ghostmod"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A limit that stands only in a string is not enforced.
    ///
    /// The gate that says every limit in the model's table is enforced
    /// somewhere read the tree's files with the comments taken out and the
    /// strings **left in**, so a limit whose only mention was a string
    /// counted as enforced. §149.
    #[test]
    fn a_limit_that_stands_only_in_a_string_is_not_enforced() {
        let dir = std::env::temp_dir().join(format!("craftkit-limit-string-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::write(
            model.join("lib.rs"),
            "pub mod limits {\n    /// A limit nothing uses.\n    pub const MAX_UNUSED: usize = 7;\n}\n",
        )
        .expect("the fabricated table");
        // The limit's name, in a string and nowhere else.
        std::fs::write(model.join("other.rs"), "const NOTE: &str = \"MAX_UNUSED\";\n")
            .expect("the fabricated code");

        let problems = limits_are_enforced(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_UNUSED"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A gate that names a line names the **file's** line.
    ///
    /// Three ways a reader can lose a line: a block comment, whose newlines
    /// used to be dropped whole; a string continuation, whose newline used to
    /// be blanked; and a line comment, which was always safe. The tree's own
    /// `crates/io/src/format.rs` has three continuations, so the second of
    /// those was not a shape but a fact. §149.
    #[test]
    fn a_gate_that_names_a_line_names_the_file_s_own_line() {
        let dir = std::env::temp_dir().join(format!("craftkit-line-{}", std::process::id()));
        let io = dir.join("crates/io/src");
        std::fs::create_dir_all(&io).expect("the fabricated tree");
        std::fs::write(
            io.join("format.rs"),
            "pub const FILE_EXPANSION: usize = 2;\n\
             /*\n\
              * A block comment.\n\
              */\n\
             const NOTE: &str = \"a string \\\n\
             that continues\";\n\
             // a line comment\n\
             pub const MAX_FILE_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * 2;\n",
        )
        .expect("the fabricated source");

        let problems = the_file_cap_is_derived_from_the_designs_budget(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        // The cap is on the eighth line of the file, and that is the line the
        // gate has to name.
        assert!(problems[0].contains("line 8"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The tree has one way to read code out of a file.
    ///
    /// The rule that a mention in a string is not code was written three
    /// times — as `code_only`, as `without_comments` and as `strip_code` —
    /// and the gates that read a symbol or a module used none of them, so a
    /// test that stood only inside a string satisfied a document's claim
    /// about it. Three copies of a rule is three chances to forget it, and
    /// the tree forgot it in three places at once. §149.
    ///
    /// The marker is a reader, and a reader has two parts: it keeps track of
    /// whether it is inside a string (`in_string`), and it **blanks** what is
    /// inside one (`push(' ')`). The tree has two other scanners that skip
    /// strings — `matching` and `arguments_of` — and neither blanks
    /// anything, so the pair tells them apart. §149.
    #[test]
    fn the_tree_has_one_code_reader() {
        let root = crate::root().expect("the workspace root");
        let mut readers = Vec::new();
        for name in
            ["lib.rs", "assets.rs", "ci.rs", "doctor.rs", "layers.rs", "lists.rs", "wasm.rs", "web.rs"]
        {
            let path = root.join("xtask/src").join(name);
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            // The tests are not the production code, and this test's own
            // marker is the very word it looks for.
            let production = match text.find("\nmod tests {") {
                Some(at) => &text[..at],
                None => &text[..],
            };
            readers.extend(code_readers_in(production));
        }
        assert_eq!(readers, vec!["code_lines_kept".to_string()], "the tree reads code one way");
    }

    /// The functions in a source that read code out of a file, by name.
    ///
    /// A reader knows a string is not code — so it says `in_string` — and it
    /// blanks what is inside one — so it says `push(' ')`. A scanner that only
    /// *skips* a string says the first and not the second, and is not a
    /// reader: it wants to know where the string ends, not what the code
    /// around it says.
    fn code_readers_in(source: &str) -> Vec<String> {
        let mut readers = Vec::new();
        let mut name = String::new();
        let mut body = String::new();
        let mut pending = String::new();
        for line in source.lines() {
            if line.starts_with("///") {
                pending.push_str(line);
                pending.push('\n');
                continue;
            }
            let head = line.strip_prefix("pub ").unwrap_or(line);
            if let Some(rest) = head.strip_prefix("fn ") {
                if !name.is_empty() && is_a_code_reader(&body) {
                    readers.push(name.clone());
                }
                name = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                body = std::mem::take(&mut pending);
                continue;
            }
            if !name.is_empty() {
                body.push_str(line);
                body.push('\n');
            }
        }
        if !name.is_empty() && is_a_code_reader(&body) {
            readers.push(name);
        }
        readers
    }

    /// Whether a function's body reads code out of a source.
    ///
    /// The two halves of the rule: it tracks the strings, and it blanks
    /// them. Either half alone is not enough — `matching` has the first, and
    /// a function that pads a sentence with spaces has the second.
    fn is_a_code_reader(body: &str) -> bool {
        body.contains("in_string") && (body.contains("push(' ')") || body.contains("push(b' ')"))
    }

    /// A test that calls the raw reader is not a third home.
    ///
    /// The rule the gate holds is about how the **product** reads a parameter:
    /// the shape of a parameter is decided by matching, never by indexing. A
    /// test that calls the reader to prove the reader works is not a third
    /// place where that shape is decided — and the tree's own test does
    /// exactly that, inside one of the two homes. §151.
    #[test]
    fn a_test_that_calls_the_raw_reader_is_not_a_third_home() {
        let dir = std::env::temp_dir().join(format!("craftkit-raw-home-{}", std::process::id()));
        let crate_dir = dir.join("crates/thing/src");
        std::fs::create_dir_all(&crate_dir).expect("the fabricated tree");
        // A third file whose *test* calls the reader.
        std::fs::write(
            crate_dir.join("thing.rs"),
            "fn f() -> usize { 1 }\n\
             \n\
             #[cfg(test)]\n\
             mod tests {\n\
             #[test]\n\
             fn the_reader_works() {\n\
             let p = craftkit_core::Params::new(&craftkit_core::Value::Null);\n\
             let _ = p.raw_json(\"x\");\n\
             }\n\
             }\n",
        )
        .expect("the fabricated code");

        let problems = the_raw_parameter_reader_has_two_homes(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // And the same call in production code is a third home.
        std::fs::write(
            crate_dir.join("thing.rs"),
            "fn f(p: &craftkit_core::Params<'_>) -> usize {\n\
             let _ = p.raw_json(\"x\");\n\
             1\n\
             }\n",
        )
        .expect("the fabricated code");
        let problems = the_raw_parameter_reader_has_two_homes(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("raw JSON"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A test's method is not a public path into the design.
    ///
    /// The gate that says the session offers no second path counts every
    /// `pub fn` in the session's impl block. A method marked `#[cfg(test)]` is
    /// not in the shipped build, so it is not a public way to change the
    /// design — and a gate that says it is would cry wolf at the first test
    /// that adds a helper to the session. §152.
    #[test]
    fn a_test_only_method_is_not_a_second_path_into_the_design() {
        let dir = std::env::temp_dir().join(format!("craftkit-session-path-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::write(
            engine.join("session.rs"),
            "pub struct Session;\n\
             \n\
             impl Session {\n\
             /// A test's helper.\n\
             #[cfg(test)]\n\
             pub fn a_test_helper(&self) -> usize {\n\
             0\n\
             }\n\
             }\n",
        )
        .expect("the fabricated session");

        let problems = the_session_offers_no_second_path(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // And a public method that is not test-only is still a second path.
        std::fs::write(
            engine.join("session.rs"),
            "pub struct Session;\n\
             \n\
             impl Session {\n\
             pub fn a_public_way_in(&self) -> usize {\n\
             0\n\
             }\n\
             }\n",
        )
        .expect("the fabricated session");
        let problems = the_session_offers_no_second_path(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("a_public_way_in"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A test that catches a panic is not a boundary.
    ///
    /// The gate says every file with a `catch_unwind` in it is a *declared*
    /// boundary, and a test may catch a panic to assert that it was caught.
    /// The gate cut the file at its test **module**, so a test written outside
    /// a module — a top-level `#[cfg(test)] fn` — was counted as a boundary.
    /// §153.
    #[test]
    fn a_test_that_catches_a_panic_is_not_a_boundary() {
        let dir = std::env::temp_dir().join(format!("craftkit-boundary-{}", std::process::id()));
        let crate_dir = dir.join("crates/thing/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&crate_dir).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        // The document has to name every declared boundary, or the gate says
        // so and the test is about the wrong thing.
        std::fs::write(
            docs.join("one-trust-boundary.md"),
            "The boundaries are `crates/kernel/src/lib.rs`, `crates/engine/src/session.rs`, \
             `crates/control/src/server.rs` and `apps/craftkit/src/main.rs`.\n",
        )
        .expect("the fabricated document");
        // A test outside a module, catching a panic.
        std::fs::write(
            crate_dir.join("thing.rs"),
            "#[cfg(test)]\n\
             fn a_test_catches() {\n\
             let _ = std::panic::catch_unwind(|| 1);\n\
             }\n",
        )
        .expect("the fabricated code");

        let problems = panic_boundaries_are_declared(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // And a production catch is still a boundary.
        std::fs::write(
            crate_dir.join("thing.rs"),
            "fn f() {\n\
             let _ = std::panic::catch_unwind(|| 1);\n\
             }\n",
        )
        .expect("the fabricated code");
        let problems = panic_boundaries_are_declared(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("thing.rs"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Production code after a test item is still production code.
    ///
    /// §150's cut truncated the file at the first test item, so a limit used
    /// *after* a test item was invisible to the gate that says every limit is
    /// enforced. The cut now removes the test items and keeps the rest.
    #[test]
    fn production_code_after_a_test_item_is_still_production_code() {
        let text = "fn uses_a_limit() -> usize { limits::MAX_NODES }\n\
                    \n\
                    #[cfg(test)]\n\
                    fn a_test() {\n\
                    let _ = 1;\n\
                    }\n\
                    \n\
                    fn uses_another_limit() -> usize { limits::MAX_KEYS }\n";
        let production = production_of(text);
        assert!(production.contains("MAX_NODES"), "{production}");
        assert!(production.contains("MAX_KEYS"), "{production}");
        assert!(!production.contains("a_test"), "{production}");
        // And the lines are still the file's lines.
        assert_eq!(production.lines().count(), text.lines().count(), "{production}");
    }

    /// A limit that only a test mentions is not enforced.
    ///
    /// The gate that says every limit in the model's table is enforced
    /// somewhere reads the tree's files whole, and a test **is** code: a test
    /// that reads the limit to compare it with another limit is a sentence
    /// about the limit, and it enforces nothing. The tree's own
    /// `crates/model/src/lib.rs` has a test that reads `MAX_PROPERTIES` to
    /// check that the limits are ordered, and that read is perfectly good
    /// code — and it enforces nothing. §150.
    #[test]
    fn a_limit_only_a_test_mentions_is_still_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-limit-test-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::write(
            model.join("lib.rs"),
            "pub mod limits {\n\
             /// A limit nothing enforces.\n\
             pub const MAX_UNUSED: usize = 7;\n\
             }\n\
             \n\
             #[cfg(test)]\n\
             mod tests {\n\
             #[test]\n\
             fn it_is_ordered() {\n\
             let _ = crate::limits::MAX_UNUSED;\n\
             }\n\
             }\n",
        )
        .expect("the fabricated table");
        // And the same, in another file.
        std::fs::write(
            model.join("other.rs"),
            "fn f() -> usize { 1 }\n\
             \n\
             #[cfg(test)]\n\
             mod tests {\n\
             #[test]\n\
             fn it_is_ordered() {\n\
             let _ = crate::limits::MAX_UNUSED;\n\
             }\n\
             }\n",
        )
        .expect("the fabricated code");

        let problems = limits_are_enforced(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("MAX_UNUSED"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The tree's own fingerprint: every field it hashes is a field the model
    /// has.
    ///
    /// The real tree, not a copy of it. §80 found a field the hash did not
    /// read, and this is the check that notices the next one.
    #[test]
    fn every_field_the_fingerprint_hashes_is_a_field_the_model_has() {
        let root = crate::root().expect("the workspace root");
        let problems = fingerprint_sees_every_field(&root).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A field the hash does not read is caught.
    ///
    /// The check reads the tree, so the tree it reads has to be *made*: a
    /// struct with a field, and a hash that reads the other one. The
    /// fabricated copy is the proof that the check bites — the same shape as
    /// §26's fabricated command id.
    #[test]
    fn a_field_the_fingerprint_does_not_read_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-fingerprint-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::write(
            model.join("widget.rs"),
            "pub struct Widget {\n    pub id: u64,\n    pub label: String,\n}\n",
        )
        .expect("the fabricated struct");
        std::fs::write(
            model.join("fingerprint.rs"),
            "impl Widget {\n    pub(crate) fn hash_into(&self, h: &mut Fnv) {\n        h.write_u64(self.id);\n    }\n}\n",
        )
        .expect("the fabricated hash");

        let problems = fingerprint_sees_every_field(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("label"), "{problems:#?}");

        // And the same tree with the field read is clean.
        std::fs::write(
            model.join("fingerprint.rs"),
            "impl Widget {\n    pub(crate) fn hash_into(&self, h: &mut Fnv) {\n        h.write_u64(self.id);\n        h.write_str(&self.label);\n    }\n}\n",
        )
        .expect("the fabricated hash");
        let problems = fingerprint_sees_every_field(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A mention in a comment is not a read.
    ///
    /// The check has to see code: a doc comment that names the field is a
    /// sentence about it, and a sentence is not a hash.
    #[test]
    fn a_field_only_a_comment_mentions_is_still_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-fingerprint-comment-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::write(
            model.join("widget.rs"),
            "pub struct Widget {\n    pub id: u64,\n    pub label: String,\n}\n",
        )
        .expect("the fabricated struct");
        std::fs::write(
            model.join("fingerprint.rs"),
            "impl Widget {\n    pub(crate) fn hash_into(&self, h: &mut Fnv) {\n        h.write_u64(self.id);\n        // self.label\n    }\n}\n",
        )
        .expect("the fabricated hash");

        let problems = fingerprint_sees_every_field(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("label"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The control channel's methods, as the code has them and as the prose
    /// has them.
    ///
    /// §82: `AGENTS.md` said *"Its methods are …"* and listed five of the
    /// eight. This is the check that notices.
    #[test]
    fn the_control_methods_are_listed_in_the_prose() {
        let root = crate::root().expect("the workspace root");
        let problems = control_methods_are_listed(&root).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A list of methods with a hole in it is caught.
    ///
    /// The fabricated tree is the point: the check has to bite on a tree that
    /// is not this one, or it is only a fact about today.
    #[test]
    fn a_method_the_prose_forgets_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-methods-{}", std::process::id()));
        let control = dir.join("crates/control/src");
        let recipe = dir.join("docs/recipes");
        std::fs::create_dir_all(&control).expect("the fabricated tree");
        std::fs::create_dir_all(&recipe).expect("the fabricated tree");
        std::fs::write(
            control.join("protocol.rs"),
            "impl Method {\n    pub fn as_str(self) -> &'static str {\n        match self {\n            \
             Method::One => \"one.thing\",\n            Method::Two => \"two.thing\",\n            \
             Method::Three => \"three.thing\",\n        }\n    }\n}\n",
        )
        .expect("the fabricated methods");
        std::fs::write(
            recipe.join("add-a-control-channel-method.md"),
            "| Wire name | `Method` | What it does |\n| --- | --- | --- |\n| `one.thing` | `One` | a |\n| \
             `two.thing` | `Two` | b |\n| `three.thing` | `Three` | c |\n",
        )
        .expect("the fabricated table");

        // A document that names two of the three methods is talking about the
        // method set, and is missing one.
        std::fs::write(dir.join("AGENTS.md"), "Its methods are `one.thing` and `two.thing`.\n")
            .expect("the fabricated prose");
        let problems = control_methods_are_listed(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("three.thing"), "{problems:#?}");

        // And the table is the code's list, in both directions.
        std::fs::write(
            recipe.join("add-a-control-channel-method.md"),
            "| Wire name | `Method` | What it does |\n| --- | --- | --- |\n| `one.thing` | `One` | a |\n| \
             `two.thing` | `Two` | b |\n| `four.thing` | `Four` | d |\n",
        )
        .expect("the fabricated table");
        let problems = control_methods_are_listed(&dir).expect("the check runs");
        assert!(
            problems.iter().any(|p| p.contains("three.thing") && p.contains("not in the table")),
            "{problems:#?}"
        );
        assert!(
            problems.iter().any(|p| p.contains("four.thing") && p.contains("not a method the code has")),
            "{problems:#?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A quoted struct that is not the one in the source is caught.
    ///
    /// The check reads the tree, so the tree it reads has to be *made*: a
    /// document with an `exact` block quoting a struct, and a source file
    /// with the struct in it. The quote's type is the one that drifted in
    /// §75 — the same names, a different type — and the check has to report
    /// it. The same shape as §26's fabricated command id.
    #[test]
    fn a_quoted_struct_that_drifted_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(
            model.join("document.rs"),
            "pub struct Report {\n    pub id: String,\n    pub design: Document,\n}\n",
        )
        .expect("the fabricated source");
        let doc = docs.join("quote.md");
        let quote = |ty: &str| {
            format!(
                "```rust exact\n// crates/model/src/document.rs\npub struct Report {{\n    pub id: String,\n    pub design: {ty},\n}}\n```\n"
            )
        };

        // The drifted quote: the same field names, a different type.
        std::fs::write(&doc, quote("Value")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("Report"), "{problems:#?}");

        // And the quote that is right.
        std::fs::write(&doc, quote("Document")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A quoted tuple struct is held to the source too.
    ///
    /// `Tick(i128)` is the whole point of the time pattern's quote: a tuple
    /// struct has no field names, and the type is the claim.
    #[test]
    fn a_quoted_tuple_struct_that_drifted_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-tuple-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(model.join("tick.rs"), "pub struct Tick(i128);\n").expect("the fabricated source");
        let quote =
            |ty: &str| format!("```rust exact\n// crates/model/src/tick.rs\npub struct Tick({ty});\n```\n");

        std::fs::write(docs.join("quote.md"), quote("i128")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        std::fs::write(docs.join("quote.md"), quote("u64")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A field the source does not have is caught too.
    ///
    /// The other direction: a document that quotes a field the source has
    /// dropped is as wrong as one that quotes a field with the wrong type.
    #[test]
    fn a_quoted_field_the_source_does_not_have_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-field-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        let docs = dir.join("docs/recipes");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(model.join("document.rs"), "pub struct Report {\n    pub id: String,\n}\n")
            .expect("the fabricated source");
        std::fs::write(
            docs.join("quote.md"),
            "```rust exact\n// crates/model/src/document.rs\npub struct Report {\n    pub id: String,\n    pub gone: String,\n}\n```\n",
        )
        .expect("the fabricated quote");

        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("gone"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A quoted const that drifted is caught.
    ///
    /// The gate read functions, structs and statics and nothing else, so a
    /// const in an exact block was invisible to it: the tree quotes one, in
    /// the time pattern, and the quote could have said any number at all.
    /// §146.
    #[test]
    fn a_quoted_const_that_drifted_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-const-{}", std::process::id()));
        let core = dir.join("crates/core/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&core).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(core.join("tick.rs"), "pub const TICKS_PER_SECOND: i128 = 254_016_000_000;\n")
            .expect("the fabricated source");
        let quote = |value: &str| {
            format!(
                "```rust exact\n// crates/core/src/tick.rs\npub const TICKS_PER_SECOND: i128 = {value};\n```\n"
            )
        };

        std::fs::write(docs.join("quote.md"), quote("254_016_000_000")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        std::fs::write(docs.join("quote.md"), quote("999_999_999_999")).expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("TICKS_PER_SECOND"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A quoted enum that drifted is caught.
    ///
    /// The control channel's whole vocabulary is an enum, and §143's defect
    /// was a variant of one, so the shape a document is most likely to quote
    /// is the shape the gate could not see. §146.
    #[test]
    fn a_quoted_enum_that_drifted_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-enum-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(
            engine.join("command.rs"),
            "pub enum Area {\n    Document,\n    Node,\n    Debug,\n}\n",
        )
        .expect("the fabricated source");
        let quote = |variants: &str| {
            format!("```rust exact\n// crates/engine/src/command.rs\npub enum Area {{\n{variants}}}\n```\n")
        };

        std::fs::write(docs.join("quote.md"), quote("    Document,\n    Node,\n    Debug,\n"))
            .expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // A variant the source has and the document does not.
        std::fs::write(docs.join("quote.md"), quote("    Document,\n    Node,\n"))
            .expect("the fabricated quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("Area"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A block that quotes an enum is not told it quotes no code.
    ///
    /// Before §146 the gate reported an enum-only block as *"quotes no code"*,
    /// which is a false alarm: the block was right and the gate could not
    /// read it. A gate that calls a correct document broken teaches people to
    /// ignore it.
    #[test]
    fn a_block_that_quotes_an_enum_is_not_a_false_alarm() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-enum-only-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(engine.join("command.rs"), "pub enum Area {\n    Document,\n}\n")
            .expect("the fabricated source");
        std::fs::write(
            docs.join("quote.md"),
            "```rust exact\n// crates/engine/src/command.rs\npub enum Area {\n    Document,\n}\n```\n",
        )
        .expect("the fabricated quote");

        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An item the gate cannot compare is reported by name.
    ///
    /// The other half of §146: a block marked `exact` promises that it is the
    /// code in the file. An `impl` block cannot be compared line for line, so
    /// the gate says so instead of passing the block as checked.
    #[test]
    fn an_item_the_gate_cannot_compare_is_reported_by_name() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-impl-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(engine.join("command.rs"), "pub enum Area {\n    Document,\n}\n")
            .expect("the fabricated source");
        std::fs::write(
            docs.join("quote.md"),
            "```rust exact\n// crates/engine/src/command.rs\nimpl Area {\n    pub fn key(self) -> u8 {\n        0\n    }\n}\n```\n",
        )
        .expect("the fabricated quote");

        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("impl"), "{problems:#?}");
        assert!(problems[0].contains("does not compare"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `use` the source does not have is caught.
    ///
    /// A declaration is not an item to compare, but it is code the quote
    /// depends on: a document that says `use craftkit_model::Document;` when
    /// the file says something else does not compile. §146.
    #[test]
    fn a_quoted_use_the_source_does_not_have_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-quote-use-{}", std::process::id()));
        let model = dir.join("crates/model/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&model).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(model.join("document.rs"), "use craftkit_core::Ids;\n\npub struct Report;\n")
            .expect("the fabricated source");
        let quote = |statement: &str| {
            format!(
                "```rust exact\n// crates/model/src/document.rs\n{statement}\n\npub struct Report;\n```\n"
            )
        };

        std::fs::write(docs.join("quote.md"), quote("use craftkit_core::Ids;")).expect("the quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        std::fs::write(docs.join("quote.md"), quote("use craftkit_core::Tick;")).expect("the quote");
        let problems = quoted_code_matches_the_source(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("craftkit_core::Tick"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The tree's own prose: the numbers it states are the code's numbers.
    ///
    /// The real tree, not a copy of it. §77 found the command count written
    /// by hand in two documents, and this is the check that notices the next
    /// command.
    #[test]
    fn the_numbers_in_the_prose_are_the_codes_numbers() {
        let root = crate::root().expect("the workspace root");
        let problems = counts_in_prose_match_the_code(&root).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A count in prose that is wrong is caught.
    ///
    /// The check reads the tree, so the tree it reads has to be *made*: a
    /// registry with two commands, an `Area` enum with two variants, and a
    /// document that says there are five commands and three areas. The same
    /// shape as §26's fabricated command id.
    #[test]
    fn a_count_in_prose_that_is_wrong_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-counts-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        let docs = dir.join("docs");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(
            engine.join("commands.rs"),
            "pub const SPECS: &[CommandSpec] = &[\n    CommandSpec {\n        id: \"doc.one\",\n    },\n    CommandSpec {\n        id: \"doc.two\",\n    },\n];\n",
        )
        .expect("the fabricated registry");
        std::fs::write(engine.join("command.rs"), "pub enum Area {\n    One,\n    Two,\n}\n")
            .expect("the fabricated areas");
        std::fs::write(docs.join("notes.md"), "The registry holds 5 commands in 3 areas.\n")
            .expect("the fabricated prose");

        let problems = counts_in_prose_match_the_code(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");
        assert!(problems.iter().any(|p| p.contains("commands")), "{problems:#?}");
        assert!(problems.iter().any(|p| p.contains("areas")), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A number that is part of a longer token is not a count.
    ///
    /// "v1.21 commands" is not a claim about the registry, and a check that
    /// read it as one would cry wolf.
    #[test]
    fn a_number_inside_a_token_is_not_a_count() {
        let found = counts_in("see v1.21 commands and 3 areas");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].1, 3);
        assert_eq!(found[0].2, "areas");
    }

    /// The tree's own boundaries: every one that catches a panic is declared.
    ///
    /// The real tree, not a copy of it. §78 found the pattern document
    /// listing two boundaries and the tree having three — and a fourth
    /// arrived with §76.
    #[test]
    fn every_boundary_that_catches_a_panic_is_declared() {
        let root = crate::root().expect("the workspace root");
        let problems = panic_boundaries_are_declared(&root).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A boundary nobody declared is caught.
    ///
    /// The check reads the tree, so the tree it reads has to be *made*: a
    /// file that catches a panic and is not in the declared list, and a
    /// document that does not mention a declared boundary. The same shape as
    /// §26's fabricated command id.
    #[test]
    fn an_undeclared_boundary_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-boundaries-{}", std::process::id()));
        let src = dir.join("crates/engine/src");
        let docs = dir.join("docs/patterns");
        std::fs::create_dir_all(&src).expect("the fabricated tree");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(src.join("session.rs"), "fn f() { let _ = std::panic::catch_unwind(|| 1); }\n")
            .expect("the fabricated boundary");
        std::fs::write(docs.join("one-trust-boundary.md"), "no boundaries here\n")
            .expect("the fabricated document");

        let problems = panic_boundaries_are_declared(&dir).expect("the check runs");
        // The undeclared boundary, and every declared boundary the document
        // does not mention.
        assert!(problems.len() > 1, "{problems:#?}");
        assert!(problems.iter().any(|p| p.contains("session.rs")), "{problems:#?}");
        assert!(problems.iter().any(|p| p.contains("does not mention")), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_documentation_table_parses() {
        let rows = parse_table(
            "| Layer | Crate | Depends on |\n\
             |---|---|---|\n\
             | L0 | `craftkit-core` | nothing (no workspace deps) |\n\
             | L3 | `craftkit-eval` | core, model, kernel |\n\
             | L3 | `craftkit-io` | core, model |\n\
             | L7 | `apps/craftkit` | all of the above |\n",
        );
        assert_eq!(rows.len(), 4, "the header and separator rows must not parse as data");
        assert_eq!(rows[0].layer, 0);
        assert_eq!(rows[0].crates, vec!["craftkit-core".to_string()]);
        assert!(rows[0].depends.is_empty(), "“nothing” is not a dependency");
        assert_eq!(rows[1].depends, vec!["core".to_string(), "model".to_string(), "kernel".to_string()]);
        assert_eq!(rows[3].layer, 7);
    }

    #[test]
    fn a_table_with_no_rows_parses_to_nothing_rather_than_to_everything() {
        // "I could not find the table" must not look like "the table is fine".
        assert!(parse_table("# Architecture\n\nSome prose about layers.\n").is_empty());
    }

    #[test]
    fn the_real_architecture_table_matches_the_code_table() {
        let root = crate::root().expect("root");
        let markdown = std::fs::read_to_string(root.join(ARCHITECTURE)).expect("architecture.md");
        let rows = parse_table(&markdown);
        assert!(!rows.is_empty(), "{ARCHITECTURE} has no parseable layering table");

        let crates = layers::load(&root).expect("metadata");
        let problems = compare_table(&rows, &crates);
        assert!(problems.is_empty(), "{ARCHITECTURE} disagrees with the code:\n{}", problems.join("\n"));
    }

    #[test]
    fn a_dependency_column_that_drifted_is_caught() {
        let rows = parse_table("| L4 | `craftkit-engine` | core, model |\n");
        let crates = vec![layers::Krate {
            name: "craftkit-engine".to_string(),
            deps: vec![
                layers::Dep {
                    name: "craftkit-core".to_string(),
                    kind: layers::DepKind::Normal,
                    workspace: true,
                },
                layers::Dep {
                    name: "craftkit-eval".to_string(),
                    kind: layers::DepKind::Normal,
                    workspace: true,
                },
            ],
        }];
        let problems = compare_table(&rows, &crates);
        assert!(problems.iter().any(|p| p.contains("core, eval")), "the drift must be named: {problems:?}");
    }

    #[test]
    fn a_table_in_the_wrong_order_is_caught() {
        // Two rows, reversed. The dependency columns are right, so only the
        // order check can see it.
        let rows = parse_table(
            "| L3 | `craftkit-io` | core, model |\n| L3 | `craftkit-eval` | core, model, kernel |\n",
        );
        assert!(compare_table(&rows, &[]).iter().any(|p| p.contains("does not match")), "{rows:?}");
    }

    #[test]
    fn a_documented_crate_that_does_not_exist_is_caught() {
        let rows =
            parse_table("| L5 | `craftkit-control` | core, engine |\n| L5 | `craftkit-telepathy` | core |\n");
        let problems = compare_table(&rows, &[]);
        assert!(problems.iter().any(|p| p.contains("telepathy")), "{problems:?}");
    }

    #[test]
    fn the_deny_attribute_is_matched_whatever_the_formatting() {
        let one_line = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]";
        assert_eq!(normalise(one_line).as_deref(), Some(canonical().as_str()));
        assert_eq!(normalise(DENY).as_deref(), Some(canonical().as_str()));
        // Indented inside a module, which is legal and equivalent.
        let indented = "mod inner {\n    #![deny(\n        clippy::unwrap_used,\n        clippy::expect_used,\n        clippy::panic,\n        clippy::unimplemented,\n        clippy::todo,\n        clippy::unreachable\n    )]\n}\n";
        assert_eq!(normalise(indented).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_deny_of_a_different_set_of_lints_is_not_the_house_one() {
        // Dropping `todo` is exactly the kind of drift this catches: still a
        // deny attribute, still plausible, and a `todo!()` would now compile.
        let shorter = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]";
        assert_ne!(normalise(shorter).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_deny_in_a_different_order_is_not_the_house_one() {
        let swapped = "#![deny(clippy::todo, clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]";
        assert_ne!(normalise(swapped).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_file_with_no_deny_attribute_normalises_to_nothing() {
        assert_eq!(normalise("fn main() {}"), None);
    }

    #[test]
    fn the_shared_lines_are_present_in_every_crate_root() {
        let root = crate::root().expect("root");
        let roots = crate_roots(&root).expect("roots");
        assert!(roots.len() >= 9, "found {} crate roots", roots.len());
        assert!(check_roots(&roots).is_empty(), "{:?}", check_roots(&roots));
    }

    #[test]
    fn a_crate_root_without_the_deny_line_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-lists-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let crate_dir = dir.join("crates/nope");
        std::fs::create_dir_all(&crate_dir).expect("dir");
        let lib = crate_dir.join("src/lib.rs");
        std::fs::create_dir_all(lib.parent().expect("parent")).expect("src");
        std::fs::write(&lib, format!("// a crate\n{FORBID}\n")).expect("write");

        let problems = check_roots(&[lib]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("no deny attribute"), "{problems:?}");
        // The report shows the block indented into its own message, so check a
        // fragment of it rather than the whole constant.
        assert!(
            problems[0].contains("clippy::unwrap_used"),
            "the report must show what to paste: {problems:?}"
        );
        assert!(problems[0].contains("#![deny("), "{problems:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_crate_root_without_forbid_unsafe_is_caught() {
        let dir = std::env::temp_dir().join(format!("craftkit-lists-unsafe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let crate_dir = dir.join("crates/nope");
        std::fs::create_dir_all(crate_dir.join("src")).expect("dir");
        let lib = crate_dir.join("src/lib.rs");
        std::fs::write(&lib, format!("// a crate\n{DENY}\n")).expect("write");

        let problems = check_roots(&[lib]);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("forbid"), "{problems:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sorted_lists_are_recognised() {
        assert!(is_sorted(&["a".to_string(), "b".to_string(), "c".to_string()]));
        assert!(!is_sorted(&["b".to_string(), "a".to_string()]));
        assert!(is_sorted(&[]), "an empty list is sorted");
    }

    #[test]
    fn a_citation_is_found_in_prose() {
        let text = "See `xtask/src/wasm.rs:41` and `session.rs:9-12`, also `a.rs:7\u{2013}8`.";
        let got = citations(text);
        println!("GOT: {got:?}");
        assert_eq!(
            got,
            vec![
                ("xtask/src/wasm.rs".to_string(), 41, 41),
                ("session.rs".to_string(), 9, 12),
                ("a.rs".to_string(), 7, 8),
            ]
        );
    }

    #[test]
    fn a_citation_at_a_blank_line_or_past_the_end_is_drift() {
        // On a temporary tree, because the real one is clean and a test that
        // only ever passes is the defect this whole check exists to find.
        let dir = std::env::temp_dir().join(format!("craftkit-cites-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).expect("temp tree");
        // Line 3 is blank; the file is six lines long.
        std::fs::write(dir.join("lib.rs"), "one\ntwo\n\nfour\nfive\nsix\n").expect("write");
        std::fs::write(dir.join("docs/a.md"), "See `lib.rs:3`, `lib.rs:99`, and `gone.rs:1`.\n")
            .expect("write");

        let found = citation_drift(&dir).expect("scans");
        assert_eq!(found.len(), 3, "{found:#?}");
        // Sorted, so the order is by name and not by which rule fired first.
        assert!(found[0].contains("not in the tree"), "{found:#?}");
        assert!(found[1].contains("blank lines"), "{found:#?}");
        assert!(found[2].contains("outside a 6-line file"), "{found:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_clean_tree_has_no_drift() {
        let root = crate::root().expect("root");
        let found = citation_drift(&root).expect("scans");
        assert!(found.is_empty(), "documentation points at nothing:\n{}", found.join("\n"));
    }

    #[test]
    fn mod_groups_split_on_blank_lines() {
        let groups = mod_groups("mod a;\nmod b;\n\n// a comment\nmod c;\n");
        assert_eq!(groups.len(), 2, "{groups:?}");
        assert_eq!(groups[0], vec!["a".to_string(), "b".to_string()]);
        assert_eq!(groups[1], vec!["c".to_string()]);
    }

    #[test]
    fn the_xtask_mod_groups_are_sorted() {
        let root = crate::root().expect("root");
        let source = std::fs::read_to_string(root.join("xtask/src/lib.rs")).expect("lib.rs");
        let groups = mod_groups(&source);
        assert!(!groups.is_empty(), "no mod lines found in xtask/src/lib.rs");
        for group in groups {
            assert!(is_sorted(&group), "unsorted mod group: {group:?}");
        }
    }

    #[test]
    fn the_workspace_member_list_is_sorted() {
        let root = crate::root().expect("root");
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
        let members = workspace_members(&manifest);
        assert!(members.len() >= 3, "found {members:?}");
        assert!(is_sorted(&members), "workspace members are not sorted: {members:?}");
    }

    /// The names this gate checks against come out of the protocol's own source,
    /// and this is what stops that read from becoming a gate that checks nothing.
    #[test]
    fn the_methods_and_actions_come_from_the_protocol_itself() {
        let (methods, actions) = wire_names(&crate::root().expect("root")).expect("reads protocol.rs");
        for method in
            ["engine.execute", "session.state", "diagnostics.list", "ui", "ui.click", "ping", "shutdown"]
        {
            assert!(
                methods.iter().any(|m| m == method),
                "{method} was not found in protocol.rs: {methods:?}"
            );
        }
        for action in ["render", "screenshot", "widgets", "click"] {
            assert!(actions.iter().any(|a| a == action), "{action} was not found: {actions:?}");
        }
        // And nothing invented: a name the protocol does not have must not be
        // among them, or the gate would check the examples against a fiction.
        assert!(!methods.iter().any(|m| m == "ui.destroy"), "{methods:?}");
        assert!(!actions.iter().any(|a| a == "nuke"), "{actions:?}");
    }

    /// The rule, stated as examples rather than as a description.
    #[test]
    fn an_example_that_could_be_sent_is_not_a_problem() {
        let (methods, actions) = wire_names(&crate::root().expect("root")).expect("protocol");
        let text = r#"
{"id":1,"token":"…","method":"engine.execute","params":{"command":"node.add"}}
{"id":2,"method":"ui.render","params":{"width":1180,"height":720}}
{"id":3,"method":"ui.click","params":{"x":400,"y":12}}
{"id":4,"method":"ui","params":{"action":"widgets"}}
{"id":5,"method":"ui.screenshot","params":{"width":300,"height":200}}
{"id":6,"method":"ui.click","params":{"action":"click","x":400,"y":12}}
{"id":7,"method":"ping"}
"#;
        assert_eq!(request_example_problems(text, &methods, &actions), Vec::<String>::new());
    }

    /// The two shapes this check exists for, plus the hostile ones.
    #[test]
    fn an_example_that_could_not_be_sent_is_reported() {
        let (methods, actions) = wire_names(&crate::root().expect("root")).expect("protocol");
        let cases = [
            // The defect of §32, in the spelling both documents used for months.
            (
                r#"{"id":3,"method":"ui.render","params":{"action":"click","x":120,"y":240}}"#,
                "cannot be sent",
            ),
            // A method nobody has heard of.
            (r#"{"id":1,"method":"engine.destroy","params":{}}"#, "is not a control-channel method"),
            // An action nobody has heard of.
            (r#"{"id":1,"method":"ui","params":{"action":"nuke"}}"#, "is not a ui action"),
            // Hostile shapes: an example is not a command-line argument, but a
            // document is still input, so nothing here may panic.
            (r#"{"id":1,"method":"ui","params":{"action":7}}"#, "must be a string"),
            (r#"{"id":1,"method":"ui","params":{"action":["click"]}}"#, "must be a string"),
            (r#"{"id":1,"method":"ui","params":{"action":null}}"#, "must be a string"),
            // Not JSON at all, while still looking like one.
            (r#"{"id":1,"method":"ui.click","params":{"x":}}"#, "is not JSON"),
        ];
        for (line, expected) in cases {
            let problems = request_example_problems(line, &methods, &actions);
            assert!(
                problems.iter().any(|p| p.contains(expected)),
                "{line}\n  expected {expected:?}, got {problems:?}"
            );
        }
    }

    /// Lines that are not requests are not reported, however much JSON they hold.
    #[test]
    fn a_line_that_is_not_a_request_is_left_alone() {
        let (methods, actions) = wire_names(&crate::root().expect("root")).expect("protocol");
        let text = r#"
{"format":"craftkit/1","meta":{"written_by":"hand"},"document":{"nodes":[]}}
{"name":"ramp","commands":[{"command":"node.add","params":{"name":"In"}}]}
`{"method":"ping"}` in backticks is prose, not a line to send.
# {"method":"ping"} — a heading, still not a line to send.
"#;
        assert_eq!(request_example_problems(text, &methods, &actions), Vec::<String>::new());
    }

    /// The token is blanked rather than refused: every document shows one, and it
    /// is never the token.
    #[test]
    fn a_shown_token_does_not_make_an_example_unparseable() {
        let (methods, actions) = wire_names(&crate::root().expect("root")).expect("protocol");
        let with_token = r#"{"id":1,"token":"…","method":"ping"}"#;
        assert_eq!(request_example_problems(with_token, &methods, &actions), Vec::<String>::new());
        // And a real-looking token of any length is still just a string.
        let long = format!(r#"{{"id":1,"token":"{}","method":"ping"}}"#, "a".repeat(64));
        assert_eq!(request_example_problems(&long, &methods, &actions), Vec::<String>::new());
    }

    /// A reference to a document that is not there, and references that are.
    ///
    /// The first one is `AGENTS.md`'s "`docs/control-protocol.md` has the wire
    /// format", which is the sentence every agent reading the file is given.
    #[test]
    fn a_document_that_points_at_a_missing_document_is_reported() {
        let root = crate::root().expect("root");
        assert!(broken_document_links(&root).expect("runs").is_empty(), "the tree has a broken reference");

        // Both ways a reader tries one: beside the referring file, and from the
        // root. `README.md` is at the root, so `README.md` in `docs/foo.md`
        // resolves only the second way.
        assert_eq!(
            document_links("see `docs/agent-playbook.md` and `crates/io/src/store.rs`"),
            vec![(1, "docs/agent-playbook.md".to_string())],
            "only a .md name in backticks is a reference"
        );
        assert_eq!(document_links("`http://example.com/x.md`").len(), 0, "a URL is not a path here");
        assert_eq!(document_links("`AGENTS.md`").len(), 1);
    }

    /// The walk has to find the references that exist, or "no problems" is
    /// another way of saying "looked at nothing".
    #[test]
    fn the_repository_has_document_references_and_they_all_resolve() {
        let root = crate::root().expect("root");
        let mut seen = 0;
        for doc in prose_files(&root) {
            let Ok(text) = std::fs::read_to_string(&doc) else { continue };
            seen += document_links(&text).len();
        }
        assert!(seen >= 20, "the walk found only {seen} references; it is probably looking at nothing");
    }

    /// A line number with no file is a citation nobody can check.
    ///
    /// §36 found twenty-four of them in the tree and thirteen were wrong, and
    /// `citation_drift` could not see one of them: it reads `name.rs:12`, and a
    /// bare `:12` has no name in it. So the rule is not "check the bare form" —
    /// it is "do not use the bare form".
    #[test]
    fn a_bare_line_citation_is_reported() {
        let root = crate::root().expect("root");
        assert!(bare_line_citations(&root).expect("runs").is_empty(), "the tree has a bare citation");

        // What counts as one, and what does not.
        assert_eq!(bare_citations("(`crates/io/src/store.rs:12`)"), Vec::<(usize, usize)>::new());
        assert_eq!(bare_citations("(`:12`)"), vec![(1, 12)]);
        assert_eq!(bare_citations("`UI_CRATES`, `:93`; `UI_MIN_LAYER`, `:96`"), vec![(1, 93), (1, 96)]);
        assert_eq!(bare_citations("a port on 127.0.0.1:4711"), Vec::<(usize, usize)>::new());
        assert_eq!(bare_citations("`http://example.com:8080/x`"), Vec::<(usize, usize)>::new());
        assert_eq!(bare_citations("`:`"), Vec::<(usize, usize)>::new());
        assert_eq!(bare_citations("`12`"), Vec::<(usize, usize)>::new());
        assert_eq!(bare_citations("`:12-14`"), Vec::<(usize, usize)>::new());
    }

    /// The README's layer table is a second copy of the code's, and the two have
    /// to agree.
    #[test]
    fn the_readme_layer_table_matches_the_code() {
        let root = crate::root().expect("root");
        assert!(readme_layers(&root).expect("runs").is_empty(), "the README and the code disagree");

        // The rows are found, or the comparison above is comparing nothing.
        let text = std::fs::read_to_string(root.join("README.md")).expect("README.md");
        let rows = readme_layer_rows(&text);
        assert_eq!(rows.len(), 9, "the README's table should list nine crates: {rows:?}");
        assert!(rows.contains(&("crates/core".to_string(), 0)), "{rows:?}");
        assert!(rows.contains(&("crates/ui".to_string(), 6)), "{rows:?}");
    }

    /// The two directions the check has to catch.
    #[test]
    fn a_readme_layer_that_is_wrong_is_reported() {
        let rows = readme_layer_rows(
            "crates/core     L0   ticks\napps/*             the desktop window\nxtask               the gates\n",
        );
        assert_eq!(rows, vec![("crates/core".to_string(), 0)], "only the rows with a layer count: {rows:?}");
        // A row with no layer is not a layer claim.
        assert!(readme_layer_rows("crates/core  the ticks").is_empty());
        // Neither is prose that merely mentions a directory.
        assert!(readme_layer_rows("the crates/core crate is L0 in the table").is_empty());
    }

    #[test]
    fn a_package_name_is_read_from_its_manifest() {
        assert_eq!(package_name("[package]\nname = \"craftkit-core\"\nversion = \"0.1.0\""), "craftkit-core");
        // A dependency table's `name` is not the package's name, and the first
        // one wins — which is the one in `[package]`.
        assert_eq!(
            package_name("[package]\nname = \"craftkit-ui\"\n\n[dependencies]\nimage = \"0.25\""),
            "craftkit-ui"
        );
        assert_eq!(package_name("[dependencies]\nimage = \"0.25\""), "");
    }

    /// The shell may read the plan, and may not evaluate the document.
    ///
    /// §132: `AGENTS.md` says "A panel never writes to the document, never
    /// calls the evaluator, never caches a value the evaluator computes". The
    /// first half is enforced by the engine's API and the third by what
    /// `ShellState` is; the middle half had nothing behind it.
    #[test]
    fn the_shell_may_read_the_plan_and_may_not_evaluate() {
        let root = crate::root().expect("root");
        assert!(
            the_shell_does_not_evaluate(&root).expect("runs").is_empty(),
            "the shell evaluates the document"
        );

        // The names are whole words: `Evaluator` is not `Evaluators`.
        assert!(has_word("craftkit_eval::Evaluator::new()", "Evaluator"));
        assert!(!has_word("craftkit_eval::Evaluators::new()", "Evaluator"));
        assert!(has_word("let _ = evaluate_once(doc, at);", "evaluate_once"));
        assert!(!has_word("let _ = evaluate_once_at(doc, at);", "evaluate_once"));

        // A comment is not a call: the doc comment may say the opposite.
        let commented = "/// The shell never calls the Evaluator.\nfn panel() {}\n";
        assert!(!has_word(&code_lines_kept(commented), "Evaluator"));

        // A string is not a call either.
        let named = "fn panel() { let _ = \"the Evaluator\"; }\n";
        assert!(!has_word(&code_lines_kept(named), "Evaluator"));

        // And the lines are kept, so a report names the file's line.
        let code = code_lines_kept("fn one() {}\n/* two\n   three */\nfn four() {}\n");
        assert_eq!(code.lines().count(), 4, "the line count changed");
        assert!(code.lines().nth(3).is_some_and(|l| l.contains("four")));
    }

    /// The session offers no second path into the design.
    ///
    /// §135: `AGENTS.md` says "There is no second path", and the front ends
    /// honoured it — but `Session::recompute` (no caller at all) and
    /// `Session::seek_to` (the crash replay's road to the playhead, with no
    /// clamp) were public, so the engine's own API was a second path.
    #[test]
    fn the_session_has_no_road_into_the_design_but_the_registry() {
        let root = crate::root().expect("root");
        assert!(
            the_session_offers_no_second_path(&root).expect("runs").is_empty(),
            "the session offers a second path into the design"
        );

        // The list is an allow list, and it is checked against the code.
        assert!(SESSION_PUBLIC_METHODS.contains(&"execute"), "the registry is on the list");
        assert!(SESSION_PUBLIC_METHODS.contains(&"document"), "the readers are on the list");
        assert!(
            !SESSION_PUBLIC_METHODS.contains(&"recompute"),
            "a way to re-evaluate without a command is a second path"
        );
        assert!(
            !SESSION_PUBLIC_METHODS.contains(&"seek_to"),
            "a second road to the playhead is a second path"
        );

        // The block is found, and the line it names is the file's line.
        let source =
            "impl Session {\n    pub fn document(&self) {}\n\n    pub fn recompute(&mut self) {}\n}\n";
        let (block, first_line) = impl_block(source, "impl Session {").expect("the block is there");
        assert_eq!(first_line, 1, "the block's first line is the file's first line");
        assert!(block.contains("recompute"), "the block holds the methods");

        // And a method that is not on the list is reported.
        let mut problems = Vec::new();
        for (offset, line) in code_lines_kept(block).lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub fn ") else { continue };
            let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if name.is_empty() || SESSION_PUBLIC_METHODS.contains(&name.as_str()) {
                continue;
            }
            problems.push(format!("{}:{name}", first_line + offset));
        }
        assert_eq!(problems, vec!["4:recompute".to_string()], "the second path is reported");
    }

    /// The shell may not put a document into the session.
    ///
    /// §135: `AGENTS.md` says "A panel never writes to the document", and
    /// §132 called that half enforced by the engine's API. It was enforced
    /// for the mutator and not for the writers: `load_document` and
    /// `replace_document` are public, and a panel could have replaced the
    /// whole design.
    #[test]
    fn the_shell_may_not_write_the_document() {
        let root = crate::root().expect("root");
        assert!(
            the_shell_does_not_write_the_document(&root).expect("runs").is_empty(),
            "the shell puts a document into the session"
        );

        // The names are the writers and nothing else.
        assert!(DOCUMENT_WRITERS.contains(&"load_document"));
        assert!(DOCUMENT_WRITERS.contains(&"replace_document"));
        assert!(!DOCUMENT_WRITERS.contains(&"execute"), "the registry is not a writer");

        // A word: `load_document` is not `load_documents`.
        assert!(has_word("session.load_document(doc);", "load_document"));
        assert!(!has_word("session.load_documents(doc);", "load_document"));

        // And a comment is not a call: the doc comment may say the opposite.
        let commented = "/// The shell never calls load_document.\nfn panel() {}\n";
        assert!(!has_word(&code_lines_kept(commented), "load_document"));
    }

    /// Every clippy setting names a lint the tree denies.
    ///
    /// §136: `clippy.toml` carried `allow-indexing-slicing-in-tests`, and
    /// `clippy::indexing_slicing` is denied in no crate — so the setting
    /// enforced nothing and made the file look as though it held down the rule
    /// that a parameter is never indexed.
    #[test]
    fn every_clippy_setting_names_a_denied_lint() {
        let root = crate::root().expect("root");
        assert!(
            every_test_only_setting_names_a_denied_lint(&root).expect("runs").is_empty(),
            "a clippy setting re-enables a lint nobody denies"
        );

        // The three settings are the three panicking lints the crate roots
        // deny, and the fourth — the one that was there — is not.
        assert_eq!(TEST_ONLY_LINTS.len(), 3, "the mapping is the list of settings");
        for (setting, lint) in TEST_ONLY_LINTS {
            assert!(LINTS.contains(lint), "{setting} re-enables {lint}, which no crate denies");
        }
        assert!(
            !TEST_ONLY_LINTS.iter().any(|(s, _)| *s == "allow-indexing-slicing-in-tests"),
            "the setting that enforced nothing is gone"
        );
    }

    /// The raw parameter reader has two homes.
    ///
    /// §136: "a parameter is never index" was a rule in four places and a
    /// convention in the code: `Params::raw` was public and returned a
    /// `&Json`, and a `&Json` can be indexed.
    #[test]
    fn the_raw_parameter_reader_has_no_third_home() {
        let root = crate::root().expect("root");
        assert!(
            the_raw_parameter_reader_has_two_homes(&root).expect("runs").is_empty(),
            "a third file reads a parameter's raw JSON"
        );

        // The two homes are the two readers.
        assert!(RAW_PARAMETER_HOMES.contains(&"crates/core/src/params.rs"));
        assert!(RAW_PARAMETER_HOMES.contains(&"crates/engine/src/params.rs"));
        assert_eq!(RAW_PARAMETER_HOMES.len(), 2, "a third home is a third reader");

        // A string is not a call, and a comment is not a call: the gate names
        // the reader, and this file is not a home.
        let named = "fn f() { let _ = \"raw_json(\"; }\n";
        assert!(!code_lines_kept(named).contains("raw_json("), "a string is not a call");
        let commented = "/// It calls raw_json(key).\nfn f() {}\n";
        assert!(!code_lines_kept(commented).contains("raw_json("), "a comment is not a call");
        // But a call is a call.
        let called = "fn f(p: &Params<'_>) { let _ = p.raw_json(\"x\"); }\n";
        assert!(code_lines_kept(called).contains("raw_json("), "a call is a call");
    }

    /// A module path in prose that names nothing this tree has.
    ///
    /// §130: `AGENTS.md` told every agent reading it that any new canvas calls
    /// `navigation::handle`. This tree has no canvas, no camera and no
    /// `navigation` module, so the rule named a module that was never built
    /// here — and the two checks that could have seen it do not: one needs the
    /// symbol to be followed by a `*.rs` path, the other needs a `.md`.
    #[test]
    fn a_module_path_in_prose_that_names_nothing_is_reported() {
        let root = crate::root().expect("root");
        assert!(
            unknown_module_paths(&root).expect("runs").is_empty(),
            "a module path in prose names nothing this tree has"
        );

        // The head of a module path.
        assert_eq!(module_path_head("navigation::handle"), Some("navigation"));
        assert_eq!(module_path_head("craftkit_ui::shell::Shell::start"), Some("craftkit_ui"));

        // A type path is not a module path: `Session::execute` is a method of a
        // type, and this check is about modules.
        assert_eq!(module_path_head("Session::execute"), None);
        // A file path is not a module path.
        assert_eq!(module_path_head("crates/core/src/params.rs"), None);
        // And neither is a name that is not dotted at all.
        assert_eq!(module_path_head("catch_unwind"), None);

        // What the tree knows.
        let modules = tree_modules(&root);
        let crates = known_crate_roots(&root);
        assert!(is_known_module_root("shell", &modules, &crates), "the ui crate has a shell module");
        assert!(is_known_module_root("craftkit_core", &modules, &crates), "craftkit-core is a crate");
        assert!(is_known_module_root("serde_json", &modules, &crates), "serde_json is a dependency");
        assert!(is_known_module_root("std", &modules, &crates), "std is the standard library");
        assert!(is_known_module_root("i64", &modules, &crates), "i64 is a primitive type");
        assert!(!is_known_module_root("navigation", &modules, &crates), "navigation is not a module");
    }

    /// A symbol that prose says is in a file, and is not.
    ///
    /// §29 could not gate the general case and said so. This is the anchored
    /// part: when the sentence names the file, the file can be checked.
    #[test]
    fn a_symbol_claimed_to_be_in_a_file_is_in_it() {
        let root = crate::root().expect("root");
        assert!(symbol_file_claims(&root).expect("runs").is_empty(), "a claim is false");

        // The claim is made by the *next* backticked thing being a path, in the
        // same paragraph.
        let claims = symbol_claims("`a_test` in `crates/ui/src/icons.rs`.");
        assert_eq!(claims, vec![(1, "a_test".to_string(), "crates/ui/src/icons.rs".to_string())]);

        // And it is not made across a paragraph break, or across a long gap.
        assert!(
            symbol_claims(
                "`a_test` asserts.

See `crates/ui/src/icons.rs`."
            )
            .is_empty()
        );
        assert!(
            symbol_claims(
                "`a_test` and a great deal of other words in between go here `crates/ui/src/icons.rs`"
            )
            .is_empty()
        );

        // A name that is not shaped like a test is not a claim: `catch_unwind`
        // is in the standard library, not in this tree.
        assert!(symbol_claims("`catch_unwind` in `crates/engine/src/session.rs`").is_empty());
    }

    /// The claim has to be about the file it names.
    #[test]
    fn a_symbol_is_looked_for_in_the_file_it_is_claimed_to_be_in() {
        let source = "    fn a_real_test() {}\n";
        assert!(defines(source, "a_real_test"));
        assert!(!defines(source, "a_missing_test"));
        // Any of the ways this tree defines things.
        assert!(defines("pub const A_REAL: u8 = 0;", "A_REAL"));
        assert!(defines("pub struct AReal;", "AReal"));
    }

    /// The gate has to find the bare citations that exist, or "no problems" is
    /// another way of saying "looked at nothing".
    #[test]
    fn the_repository_had_bare_citations_and_now_has_none() {
        let root = crate::root().expect("root");
        let mut seen = 0;
        for doc in prose_files(&root) {
            let Ok(text) = std::fs::read_to_string(&doc) else { continue };
            seen += bare_citations(&text).len();
        }
        assert_eq!(seen, 0, "the tree has {seen} bare citation(s) left");
    }

    /// The whole gate, on the repository as it stands: it must find the examples
    /// it is meant to find, and they must be the ones in the documents.
    #[test]
    fn the_repository_has_examples_and_the_gate_is_happy_with_them() {
        let root = crate::root().expect("root");
        let problems = prose_examples_in_docs_resolve_to_real_methods(&root).expect("runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // "Happy with none" would also pass the line above, so count the
        // examples the gate actually looked at.
        let mut seen = 0;
        for doc in prose_files(&root) {
            let Ok(text) = std::fs::read_to_string(&doc) else { continue };
            seen += text.lines().filter(|l| request_shaped(l).is_some()).count();
        }
        assert!(seen >= 5, "the gate only found {seen} example(s); it is probably looking at nothing");
    }

    /// The instrument's closing ratio is computed from the cases it ran. §71.
    #[test]
    fn the_mutation_script_computes_its_own_closing_ratio() {
        let root = crate::root().expect("root");
        let script =
            std::fs::read_to_string(root.join("scripts/mutation-test-gates.sh")).expect("the script");
        let problems = mutation_ratio_is_computed(&script);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// And a ratio written by hand is caught.
    ///
    /// The fabricated case: the instrument cannot be mutated while it is
    /// running, so the check is proved on a copy of the thing it reads.
    #[test]
    fn a_closing_ratio_written_by_hand_is_caught() {
        let fabricated = "\
printf '%s%d/%d caught.%s\\n' \\
    \"$GREEN\" \"28\" \"29\" \"$RESET\"
";
        let problems = mutation_ratio_is_computed(fabricated);
        assert!(!problems.is_empty(), "a hand-written ratio was not noticed");
        assert!(problems[0].contains("written by hand"), "{problems:?}");
    }

    /// The shell's state holds nothing the evaluator computes. §138.
    ///
    /// `AGENTS.md` says "A panel never writes to the document, never calls
    /// the evaluator, never caches a value the evaluator computes", and then
    /// says "All three are checked". Two were. The third was somebody looking
    /// at `ShellState` once, and the looking was not repeated.
    #[test]
    fn the_shell_state_holds_nothing_the_evaluator_computes() {
        let root = crate::root().expect("root");
        assert!(
            the_shell_caches_no_evaluated_value(&root).expect("runs").is_empty(),
            "the shell's state holds a value the evaluator computes"
        );

        // The scope is found, not listed, and it is the scope it should be.
        let structs = ui_structs(&root);
        let state = reach(&structs, "ShellState", true);
        let carried = reach(&structs, "ShellState", false);
        assert!(state.contains(&"ShellState".to_string()), "the state is the state: {state:?}");
        assert!(state.contains(&"Prompt".to_string()), "the prompt is state: {state:?}");
        assert!(state.contains(&"PromptField".to_string()), "a field is state: {state:?}");
        assert!(
            !state.contains(&"NodeRow".to_string()),
            "a row is drawn every frame, so it is not state: {state:?}"
        );
        assert!(carried.contains(&"Shell".to_string()), "the shell carries it: {carried:?}");
        assert!(carried.contains(&"Frame".to_string()), "a frame carries it: {carried:?}");

        // A number in the state is a number the design said.
        assert!(holds_type("f64", "f64"));
        assert!(holds_type("Option<f64>", "f64"));
        assert!(holds_type("Vec<f64>", "f64"));
        // A word: `f64` is not `f64x`, and `Plan` is not `Planned`.
        assert!(!holds_type("f64x", "f64"));
        assert!(!holds_type("Planned", "Plan"));
        assert!(holds_type("Option<craftkit_eval::Plan>", "Plan"));
        // And a JSON blob is not the design's value.
        assert!(!holds_type("serde_json::Value", "Value"));
        assert!(holds_type("craftkit_core::Value", "Value"));
        assert!(holds_type("Option<Value>", "Value"));

        // And the fields are read with the file's own lines.
        let shell = structs.iter().find(|s| s.name == "ShellState").expect("the state");
        let selected = shell.fields.iter().find(|(_, name, _)| name == "selected").expect("a field");
        assert_eq!(selected.2, "Option<NodeId>", "the type as written: {selected:?}");
        assert!(selected.0 > 1, "the line is the file's, not the block's: {selected:?}");
    }

    /// Every parameter a command reads is declared, and every parameter it
    /// declares is read. §139.
    ///
    /// The playbook's rule 4 says its own check is *"call your command with
    /// its own parameters in a test"*, and says the test the tree has does
    /// not catch it. The rule was right about itself.
    #[test]
    fn a_command_declares_every_parameter_it_reads() {
        let root = crate::root().expect("root");
        assert!(
            every_parameter_a_command_reads_is_declared(&root).expect("runs").is_empty(),
            "a command reads a parameter it does not declare, or declares one it never reads"
        );

        // The commands are found, and there are more than a couple of them —
        // a gate that found one command would pass for the wrong reason.
        let commands = registry_commands(&root);
        assert!(commands.len() > 20, "the gate found {} commands", commands.len());

        // The readers are **derived**, not listed: a function that takes a
        // `&Params` is a reader, and one that is not in any list is still
        // found.
        let readers = parameter_readers(&root);
        for name in ["text", "num", "num_in", "int", "bool", "color", "num_list", "has", "raw_json"] {
            assert!(
                readers.iter().any(|r| r.name == name),
                "`{name}` reads a parameter and the gate does not know it"
            );
        }
        assert!(
            readers.iter().any(|r| r.name == "node_by_id_or_name"),
            "the reader that takes a node by id or by name is missing"
        );

        // A reader without a name argument of its own reads what its body
        // reads: `node_by_id_or_name` reads `node` and `node_name`.
        let by_id_or_name = readers.iter().find(|r| r.name == "node_by_id_or_name").expect("it");
        assert!(by_id_or_name.key.is_none(), "it takes no name of its own");
        let names = names_read(&by_id_or_name.body, by_id_or_name.line, &readers, 0);
        let mut read: Vec<&str> = names.iter().map(|(n, _)| n.as_str()).collect();
        read.sort();
        assert_eq!(read, vec!["node", "node_name"], "the names it reads: {read:?}");

        // And the two halves of the rule are told apart.
        let rename = commands.iter().find(|c| c.id == "doc.rename").expect("the command");
        assert_eq!(rename.declared, vec!["name".to_string()], "the names it declares");
        let read = names_read(&rename.body, rename.body_line, &readers, 0);
        assert_eq!(read.len(), 1, "it reads one parameter: {read:?}");
        assert_eq!(read[0].0, "name", "and it is the one it declares");
    }

    /// The design file's cap is the design's own budget, times the format's
    /// expansion. §141.
    ///
    /// The runtime test in the io crate cannot see this one: the literal and
    /// the derivation have the same value today, so only the source can tell
    /// them apart. That is what a gate is for.
    #[test]
    fn a_file_cap_that_is_a_number_of_its_own_is_caught() {
        let root = crate::root().expect("root");
        assert!(
            the_file_cap_is_derived_from_the_designs_budget(&root).expect("runs").is_empty(),
            "the design file's cap is not the design's own budget times the format's expansion"
        );

        let dir = std::env::temp_dir().join(format!("craftkit-file-cap-{}", std::process::id()));
        let io = dir.join("crates/io/src");
        std::fs::create_dir_all(&io).expect("the fabricated tree");

        // A cap somebody chose: the same number, and no way to see it.
        std::fs::write(io.join("format.rs"), "pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;\n")
            .expect("the fabricated source");
        let problems = the_file_cap_is_derived_from_the_designs_budget(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");

        // The cap the tree has, written over two lines: the gate reads the
        // statement, not the line.
        std::fs::write(
            io.join("format.rs"),
            "pub const MAX_FILE_BYTES: usize =\n    craftkit_model::limits::MAX_DOCUMENT_BYTES * \
             FILE_EXPANSION;\n",
        )
        .expect("the fabricated source");
        let problems = the_file_cap_is_derived_from_the_designs_budget(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // And a cap that names the budget but not the expansion is caught too:
        // the expansion is the number that says how many bytes of text one
        // byte of design takes.
        std::fs::write(
            io.join("format.rs"),
            "pub const MAX_FILE_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES;\n",
        )
        .expect("the fabricated source");
        let problems = the_file_cap_is_derived_from_the_designs_budget(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The tree's JSON has one writer, and the places that write a design use
    /// it. §142.
    ///
    /// The crash report's files were written with serde_json's pretty printer,
    /// which is the tree's layout without the bound on the indentation, so the
    /// app wrote crash reports it could not read back.
    #[test]
    fn a_design_written_with_a_writer_of_its_own_is_caught() {
        let root = crate::root().expect("root");
        assert!(
            the_tree_writes_its_json_with_one_writer(&root).expect("runs").is_empty(),
            "a design is written with a writer of its own"
        );

        let dir = std::env::temp_dir().join(format!("craftkit-one-writer-{}", std::process::id()));
        let io = dir.join("crates/io/src");
        let engine = dir.join("crates/engine/src");
        std::fs::create_dir_all(&io).expect("the fabricated tree");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        let good = "let _ = craftkit_core::writer::write_json(&mut out, &value);\n";
        std::fs::write(io.join("format.rs"), good).expect("the fabricated source");
        std::fs::write(engine.join("crash.rs"), good).expect("the fabricated source");
        let problems = the_tree_writes_its_json_with_one_writer(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // The crash folder, written with serde_json's pretty printer: the
        // layout without the bound, and a report the reader refuses.
        std::fs::write(engine.join("crash.rs"), "let _ = serde_json::to_string_pretty(&report);\n")
            .expect("the fabricated source");
        let problems = the_tree_writes_its_json_with_one_writer(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");

        // And a writer of its own that is bounded: the report would fit, and
        // the design in it would still not be the design on disk. One
        // problem — the writer is not the tree's — and the crash report's
        // test would not notice, which is why this is a gate.
        std::fs::write(engine.join("crash.rs"), "let _ = serde_json::to_string(&report);\n")
            .expect("the fabricated source");
        let problems = the_tree_writes_its_json_with_one_writer(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");

        // And a test may compare two shapes of a report with serde_json's own
        // printer: the tests are not the writer.
        std::fs::write(
            engine.join("crash.rs"),
            "let _ = craftkit_core::writer::write_json(&mut out, &value);\n\
             #[cfg(test)]\nmod tests {\n    fn a_shape() {\n        let _ = serde_json::to_string_pretty(&report);\n    }\n}\n",
        )
        .expect("the fabricated source");
        let problems = the_tree_writes_its_json_with_one_writer(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A type path in prose must name a member the type has. §143.
    ///
    /// The recipe for adding a control-channel method told an agent to add a
    /// `Method::Render` variant to a tree that has never had one, and to move
    /// `ui.render` out of the group of spellings that reach `Method::Ui`. §143
    /// is the twin of §130's module-path gate: §130 checked `a::b` and said in
    /// so many words that a *type* path was "not this check's business".
    #[test]
    fn a_type_path_that_names_a_member_the_type_does_not_have_is_caught() {
        let root = crate::root().expect("root");
        assert!(
            unknown_type_paths(&root).expect("runs").is_empty(),
            "the tree's prose names a member a type does not have"
        );

        let dir = std::env::temp_dir().join(format!("craftkit-type-paths-{}", std::process::id()));
        let docs = dir.join("docs");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::write(
            dir.join("protocol.rs"),
            "pub enum Method {\n    Ui,\n    Ping,\n}\n\
             impl Method {\n    pub fn parse(s: &str) -> Option<Self> { None }\n}\n",
        )
        .expect("the fabricated source");

        // The defect: a variant the tree has never had.
        std::fs::write(docs.join("recipe.md"), "Add a `Method::Render` variant.\n")
            .expect("the fabricated prose");
        let problems = unknown_type_paths(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("Method::Render"), "{problems:#?}");

        // A path into somebody else's crate is not this tree's business:
        // `serde_json::Value::Number` names a variant of *serde's* value, and
        // the tree has a `Value` of its own that has nothing to do with it.
        std::fs::write(docs.join("recipe.md"), "Match `serde_json::Value::Number(n)`.\n")
            .expect("the fabricated prose");
        let problems = unknown_type_paths(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // And a member the type does have is not a problem.
        std::fs::write(docs.join("recipe.md"), "Call `Method::parse`.\n").expect("the fabricated prose");
        let problems = unknown_type_paths(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The scanner that reads the tree's types has to survive the things Rust
    /// source is full of.
    ///
    /// A **lifetime** is not a char literal, and a brace inside a string is
    /// not a brace. Both mistakes eat the rest of the file, and the gate then
    /// believes a type has members it does not have.
    #[test]
    fn the_type_scanner_survives_lifetimes_strings_and_char_literals() {
        let source = "pub struct Holder<'a> {\n    name: &'a str,\n}\n\
                      impl<'a> Holder<'a> {\n    pub fn get(&self) -> &str { self.name }\n}\n\
                      const OPEN: char = '{';\n\
                      const CLOSE: char = '}';\n\
                      // a comment with a } in it\n\
                      pub fn quoted() -> &'static str { \"a } brace\" }\n";
        let stripped = code_lines_kept(source);
        // The braces inside the string and the char literals are gone, and the
        // lifetime is still a lifetime.
        assert!(!stripped.contains("'{'"), "{stripped}");
        assert!(!stripped.contains("'}'"), "{stripped}");
        assert!(stripped.contains("'a"), "{stripped}");
        assert!(!stripped.contains("a } brace"), "{stripped}");

        let mut types: std::collections::BTreeMap<String, BTreeSet<String>> =
            std::collections::BTreeMap::new();
        collect_types_in(&stripped, &mut types);
        let members = types.get("Holder").expect("the type");
        assert!(members.contains("name"), "{members:?}");
        assert!(members.contains("get"), "{members:?}");
        // The functions after the string are still in the same file.
        assert!(types.contains_key("Holder"), "{types:?}");
    }

    /// The memory budgets are the model's budget times a number of documents.
    /// §144.
    ///
    /// The model's limit table said a document's budget was *"the number that
    /// says what the process actually uses"*, and the process holds four
    /// documents. The caps that decide how many were literals that happened
    /// to be the design file's cap.
    #[test]
    fn a_memory_budget_that_is_a_number_of_its_own_is_caught() {
        let root = crate::root().expect("root");
        assert!(
            the_memory_budgets_are_derived_from_the_models(&root).expect("runs").is_empty(),
            "a memory budget is not derived from the model's budget"
        );

        let dir = std::env::temp_dir().join(format!("craftkit-memory-budgets-{}", std::process::id()));
        let engine = dir.join("crates/engine/src");
        std::fs::create_dir_all(&engine).expect("the fabricated tree");
        let good_history = "pub const MAX_HISTORY_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * HISTORY_EXPANSION;\n";
        let good_session = "pub const MAX_SESSION_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS;\n\
                            pub const SESSION_DOCUMENTS: usize = SESSION_OWN_DOCUMENTS + SESSION_SNAPSHOT_DOCUMENTS + crate::history::HISTORY_EXPANSION;\n";
        std::fs::write(engine.join("history.rs"), good_history).expect("the fabricated source");
        std::fs::write(engine.join("session.rs"), good_session).expect("the fabricated source");
        let problems = the_memory_budgets_are_derived_from_the_models(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        // A cap somebody chose: the same number, and no way to see it.
        std::fs::write(engine.join("history.rs"), "pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;\n")
            .expect("the fabricated source");
        let problems = the_memory_budgets_are_derived_from_the_models(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");

        // And a session's budget that forgot the stacks it holds: the
        // statement names none of the three parts, and each is reported.
        std::fs::write(engine.join("history.rs"), good_history).expect("the fabricated source");
        std::fs::write(
            engine.join("session.rs"),
            "pub const MAX_SESSION_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * SESSION_DOCUMENTS;\n\
             pub const SESSION_DOCUMENTS: usize = 2;\n",
        )
        .expect("the fabricated source");
        let problems = the_memory_budgets_are_derived_from_the_models(&dir).expect("the check runs");
        assert_eq!(problems.len(), 3, "{problems:#?}");

        // And a session's budget that forgot the model's own: the statement
        // names neither half of the derivation, and each half is reported.
        std::fs::write(
            engine.join("session.rs"),
            "pub const MAX_SESSION_BYTES: usize = 128 * 1024 * 1024;\n\
             pub const SESSION_DOCUMENTS: usize = SESSION_OWN_DOCUMENTS + SESSION_SNAPSHOT_DOCUMENTS + crate::history::HISTORY_EXPANSION;\n",
        )
        .expect("the fabricated source");
        let problems = the_memory_budgets_are_derived_from_the_models(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A bound the tree has is named in the prose, and the prose does not
    /// tell a reader to add a size cap the tree has. §145.
    #[test]
    fn a_byte_bound_the_prose_does_not_name_is_caught() {
        let root = crate::root().expect("root");
        assert!(
            the_byte_bounds_are_named_in_the_prose(&root).expect("runs").is_empty(),
            "a byte bound the tree has is not named in the prose"
        );

        let dir = std::env::temp_dir().join(format!("craftkit-byte-bounds-{}", std::process::id()));
        let docs = dir.join("docs");
        let src = dir.join("crates/engine/src");
        std::fs::create_dir_all(&docs).expect("the fabricated tree");
        std::fs::create_dir_all(&src).expect("the fabricated tree");
        std::fs::write(src.join("history.rs"), "pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;\n")
            .expect("the fabricated source");
        std::fs::write(docs.join("notes.md"), "Nothing here names a bound.\n").expect("the fabricated prose");
        let problems = the_byte_bounds_are_named_in_the_prose(&dir).expect("the check runs");
        assert_eq!(problems.len(), 1, "{problems:#?}");

        // And a document that tells a reader to add a size cap the tree has.
        // The phrase is wrapped across two lines, because a document that
        // wraps it is the same sentence.
        std::fs::write(docs.join("notes.md"), "The fix is to add a size\ncap.\n")
            .expect("the fabricated prose");
        let problems = the_byte_bounds_are_named_in_the_prose(&dir).expect("the check runs");
        assert_eq!(problems.len(), 2, "{problems:#?}");

        // And once the prose names the bound, there is nothing to report.
        std::fs::write(docs.join("notes.md"), "`MAX_HISTORY_BYTES` is the budget.\n")
            .expect("the fabricated prose");
        let problems = the_byte_bounds_are_named_in_the_prose(&dir).expect("the check runs");
        assert!(problems.is_empty(), "{problems:#?}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
