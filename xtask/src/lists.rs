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
        // assert that it was caught, and that is not a boundary.
        let production = match text.find("\nmod tests {") {
            Some(at) => &text[..at],
            None => &text[..],
        };
        if !code_only(production).contains("catch_unwind(") {
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
pub fn defines(source: &str, name: &str) -> bool {
    for keyword in ["fn ", "struct ", "enum ", "const ", "static ", "mod ", "trait ", "type "] {
        if source.contains(&format!("{keyword}{name}")) {
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
fn tree_modules(root: &Path) -> std::collections::BTreeSet<String> {
    let mut modules = std::collections::BTreeSet::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        for line in text.lines() {
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
            let quoted = functions_in(&block);
            let structs = structs_in(&block);
            if quoted.is_empty() && structs.is_empty() {
                problems.push(format!(
                    "{relative}: the block at line {number} is marked exact but quotes no code"
                ));
                continue;
            }
            for (name, lines) in quoted {
                match function_in(&code, &name) {
                    None => problems.push(format!(
                        "{relative}: the block at line {number} quotes `{name}`, and {source} has no function of that name"
                    )),
                    Some(original) if original == lines => {}
                    Some(original) => problems.push(format!(
                        "{relative}: the block at line {number} quotes `{name}`, and it is not the one in {source}\n     \
                         the document says:\n{}\n     the file says:\n{}",
                        indent(&lines),
                        indent(&original)
                    )),
                }
            }
            for (name, fields) in structs {
                match struct_in(&code, &name) {
                    None => problems.push(format!(
                        "{relative}: the block at line {number} quotes the struct `{name}`, and {source} has no struct of that name"
                    )),
                    Some(original) if original == fields => {}
                    Some(original) => problems.push(format!(
                        "{relative}: the block at line {number} quotes the struct `{name}`, and it is not the one in {source}\n     \
                         the document says:\n{}\n     the file says:\n{}",
                        indent(&fields.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>()),
                        indent(&original.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>())
                    )),
                }
            }
        }
    }
    problems.sort();
    Ok(problems)
}

/// The structs a block quotes, as (name, fields).
///
/// Only a **braced** struct: a tuple struct has no fields to compare, and an
/// enum's variants are a different shape (and are usually quoted with
/// elisions in them, which is exactly what the `exact` marker forbids).
fn structs_in(block: &[String]) -> Vec<(String, Vec<(String, String)>)> {
    let mut found = Vec::new();
    for (index, line) in block.iter().enumerate() {
        let Some(rest) = after_visibility(line.trim()) else { continue };
        let Some(rest) = rest.strip_prefix("struct ") else { continue };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        let tail = rest[name.len()..].trim_start();
        if tail.starts_with('{') {
            found.push((name, struct_fields(&block[index..])));
        } else if let Some(types) = tuple_fields(tail) {
            found.push((name, types));
        }
    }
    found
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
    let lines: Vec<&str> = code.lines().collect();
    let index = lines.iter().position(|line| {
        let Some(rest) = after_visibility(line.trim()) else { return false };
        let Some(rest) = rest.strip_prefix("struct ") else { return false };
        let found: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        found == name
    })?;
    let line = lines[index].trim();
    let rest = line.strip_prefix("pub ").unwrap_or(line);
    let rest = rest.strip_prefix("struct ").unwrap_or(rest);
    let found: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let tail = rest[found.len()..].trim_start();
    if tail.starts_with('{') {
        let owned: Vec<String> = lines[index..].iter().map(|l| (*l).to_string()).collect();
        return Some(struct_fields(&owned));
    }
    tuple_fields(tail)
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

/// The functions a block quotes, as (name, lines).
fn functions_in(block: &[String]) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for (index, line) in block.iter().enumerate() {
        let Some(rest) = after_visibility(line.trim()) else { continue };
        let Some(rest) = rest.strip_prefix("fn ") else { continue };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if name.is_empty() {
            continue;
        }
        found.push((name, function_lines(block, index)));
    }
    found
}

/// A function in `code`, by name.
fn function_in(code: &str, name: &str) -> Option<Vec<String>> {
    let lines: Vec<String> = code.lines().map(|l| l.to_string()).collect();
    let index = lines.iter().position(|line| {
        let Some(rest) = after_visibility(line.trim()) else { return false };
        rest.starts_with(&format!("fn {name}("))
    })?;
    Some(function_lines(&lines, index))
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
                        Some(next) if next.1.contains("<<") || next.1.contains('*') => next.1.trim(),
                        _ => continue,
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
    for line in source.lines() {
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
        let shown = if value.chars().any(|c| c.is_ascii_digit()) { value.as_str() } else { "a re-export" };
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
pub fn limits_are_enforced(root: &Path) -> Result<Vec<String>, String> {
    let table = root.join("crates/model/src/lib.rs");
    let source =
        std::fs::read_to_string(&table).map_err(|e| format!("could not read the limits table: {e}"))?;
    let code = without_comments(&source);
    let Some((declared, rest)) = limits_table(&code) else {
        return Ok(vec!["the model's limits table could not be found".to_string()]);
    };
    if declared.is_empty() {
        return Ok(vec!["the model's limits table declares no limits at all".to_string()]);
    }
    let mut used: Vec<String> = Vec::new();
    for path in rust_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let body = if path == table { rest.clone() } else { without_comments(&text) };
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

/// The code in a file, with the comments removed.
///
/// A limit that only a comment mentions is not enforced, so the check has to
/// see code and not prose. A string is left alone: a `//` inside one is not
/// a comment, and neither is a `//` in a URL.
fn without_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        if in_string {
            out.push(c);
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
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// The code in a file, with the comments **and the string literals** removed.
///
/// A mention in a string is not a call: this file's own needle is one, and a
/// check that flagged itself would be a check nobody trusts. §78.
fn code_only(source: &str) -> String {
    let without = without_comments(source);
    let chars: Vec<char> = without.chars().collect();
    let mut out = String::with_capacity(without.len());
    let mut at = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    while at < chars.len() {
        let c = chars[at];
        at += 1;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            out.push(' ');
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
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
             methods are the methods the prose lists, and every module path in prose \
             names a module this tree has",
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
        let code = code_only(&body.join("\n"));
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
}
