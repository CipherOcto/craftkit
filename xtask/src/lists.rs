//! `cargo xtask lists` — the shared lines and the tables many changes touch.
//!
//! # What this gate is for
//!
//! The v2 tree has **its own** gate: `crates/craftkit-gates`. That crate owns
//! the rules about the tree's *crates* — the house deny attribute, the
//! layering, what a plugin may depend on, the bounds the prose names, and the
//! numbers the prose and the code must agree on. This gate does not repeat
//! any of them: one rule, one place.
//!
//! What lives here is the **tree-level** half — the facts that are about the
//! repository as a whole rather than about any one crate:
//!
//! - the layer table in `docs/architecture.md` and the table in the tree's own
//!   gate agree;
//! - every gate the tree names is actually run — by `ci`, or by name in the
//!   workflow;
//! - every document link in the prose points at a file that is in the tree;
//! - the `mod` groups of every crate root are sorted;
//! - the workspace's member list covers every crate that is in the tree;
//! - the xtask's own crate root carries the house deny list — the one crate
//!   the inner gate cannot see, because the inner gate looks in `crates/` and
//!   `apps/` and the xtask is beside them;
//! - every rule this gate retired is **named**, with the reason.
//!
//! # The rules that were retired
//!
//! The v1 tree's gate had rules about things the v2 tree does not have: a
//! fuzzing corpus and its table of targets, an Android package and the
//! platform's keyboard, a mutation-testing instrument and its budget, a
//! temporary-file convention, a `.craftkit` format with pending changes, a
//! shell and an engine with a session. Retiring a rule silently is how a tree
//! loses a guarantee without anybody noticing, so every one of them is named
//! in [`RETIRED`], with the reason it no longer applies — and
//! [`the_retired_rules_are_named`] checks that the table is not empty and
//! that every row says why.
//!
//! # Why the gate reads the tree as text
//!
//! The xtask deliberately depends on nothing of the tree's own: a checker
//! that cannot be built when the tree does not compile is a checker that is
//! silent exactly when it is needed. So every rule here reads files as text,
//! and says which view it wants when it reads a Rust file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::step;
use crate::tree::{Subject, read_rust, read_text, relative_name, walk};

/// The prose the tree's gates and documents live in.
///
/// The tree's own documents, and the two skills that tell an agent how to
/// change it and how to check it.
pub const PROSE: &[&str] = &["docs", "skills", "AGENTS.md", "README.md", "CONTRIBUTING.md"];

/// The tree's own gate: the crate that holds the layer table.
///
/// The table in this crate is the **authority** — the inner gate is what the
/// tree enforces, and the prose is a rendering of it. This gate checks that
/// the rendering has not drifted.
pub const GATES_CRATE: &str = "crates/craftkit-gates/src/lib.rs";

/// The document that renders the layer table.
pub const ARCHITECTURE: &str = "docs/architecture.md";

/// The workflow that decides where the gates run.
pub const WORKFLOW: &str = ".github/workflows/ci.yml";

/// The v1 rules that this gate does not have, and why.
///
/// **Nothing is retired silently.** A rule that is dropped without a reason
/// is a guarantee the tree lost without anybody noticing, and that is the
/// defect this table exists to prevent: the table is the record, and
/// [`the_retired_rules_are_named`] checks that it is a record and not an
/// excuse.
///
/// The name is the v1 rule's name, so that a reader who remembers it can
/// find what happened to it.
pub const RETIRED: &[(&str, &str)] = &[
    (
        "the_fuzz_targets_are_the_table",
        "the v2 tree has no fuzzer: no corpus, no targets, and no document that lists them",
    ),
    (
        "the_mutation_job_fits_the_suites_budget",
        "the v2 tree has no mutation instrument, so there is no run whose budget a job's timeout has to fit",
    ),
    (
        "the_phone_package_asks_for_what_its_channel_needs",
        "the v2 tree has no Android package: the phone is not a target the tree builds for",
    ),
    (
        "the_platforms_keyboard_is_behind_the_gate",
        "the v2 tree has no IME and no platform's keyboard behind a gate",
    ),
    ("the_instrument_is_not_cancelled_by_a_push", "there is no instrument for a push to cancel"),
    (
        "the_pending_changes_are_declared_in_code",
        "the v2 tree's format is a versioned contribution, not a folder of pending edits",
    ),
    (
        "the_step_bound_is_one_number_and_both_doors_name_it",
        "the v2 tree has no step bound and no script that has to be refused",
    ),
    ("a_reply_asks_the_tick", "the v2 tree's replies carry no tick: a command's answer is the answer"),
    (
        "the_raw_parameter_reader_has_two_homes",
        "the v2 tree reads its parameters through the SDK's typed `ParamKind`, which cannot be indexed",
    ),
    (
        "the_file_cap_is_derived_from_the_designs_budget",
        "the v2 tree has no design file with a byte budget of its own",
    ),
    (
        "the_memory_budgets_are_derived_from_the_models",
        "the v2 tree has no v1 model layer whose limits the budgets are derived from",
    ),
    (
        "the_trace_budget_is_derived_from_its_memory_budget",
        "the v2 tree has no trace buffer and no memory budget to derive it from",
    ),
    (
        "the_never_crash_table_names_every_crate_root",
        "the v2 tree's never-crash guarantee is the deny attribute every crate root carries, and the inner gate checks it crate by crate",
    ),
    (
        "the_never_crash_table_names_every_limit",
        "the v2 tree has no never-crash table of limits; the bounds are named in the README and checked by the inner gate",
    ),
    (
        "limits_are_enforced",
        "the v2 tree's limits are the batteries' own constants, and the inner gate checks the bounds the README names",
    ),
    (
        "panic_boundaries_are_declared",
        "the v2 tree has no catch-unwind boundary around a kernel: the SDK's session refuses, and a panic is a bug rather than a state to restore",
    ),
    (
        "every_parameter_a_command_reads_is_declared",
        "the v2 tree makes the mistake impossible to write: a command is handed a struct, so it cannot read a parameter it did not declare",
    ),
    (
        "the_shell_does_not_evaluate",
        "the v2 tree has no shell: the front ends read the session's plan and act through the registry",
    ),
    (
        "the_shell_caches_no_evaluated_value",
        "the v2 tree has no shell whose state could hold an evaluated value",
    ),
    (
        "the_shell_does_not_write_the_document",
        "the v2 tree has no shell: the front ends act through commands, and the inner gate checks that a front end stays thin",
    ),
    (
        "the_session_offers_no_second_path",
        "the v2 tree's registry is a value and every door reaches it the same way",
    ),
    ("fingerprint_sees_every_field", "the v2 tree has no fingerprint over a model's fields"),
    ("the_tree_writes_its_json_with_one_writer", "the v2 tree's format is written by the SDK's own writer"),
    ("the_tree_has_one_temporary_folder", "the v2 tree has no temporary-folder convention of its own"),
    (
        "the_tree_writes_its_files_through_one_home",
        "the v2 tree's file writing is the SDK's business and not a tree-level rule",
    ),
    (
        "the_tree_says_how_its_files_are_checked_out",
        "the v2 tree's checkout rules are `.gitattributes`' business and there is no promise in its prose to check",
    ),
    ("the_control_channel_secret_is_not_the_clock", "the v2 tree's control channel has a token and no clock"),
    (
        "a_promise_of_a_file_mode_says_where_it_holds",
        "the v2 tree's prose makes no promise about a file mode",
    ),
    (
        "the_applications_data_folder_is_built_in_one_place",
        "the v2 tree's applications have no shared data folder",
    ),
    (
        "the_applications_name_is_written_in_one_place",
        "the v2 tree has no application name that several files must agree on",
    ),
    (
        "the_servers_name_is_written_in_one_place",
        "the v2 tree has no server whose name is written in several places",
    ),
    (
        "the_scaffolding_section_names_every_crate_the_domain_touches",
        "the v2 tree's README is a design document, not a scaffolding section",
    ),
    (
        "the_tree_names_the_enforcement_it_has",
        "the v2 tree's `KNOWN-GAPS.md` names what it does not do; the enforcement it has is the inner gate, which the tree names in `AGENTS.md`",
    ),
    (
        "the_walk_has_one_home",
        "the xtask's walk is the xtask's own, and the v2 tree's crates do not walk the tree",
    ),
];

/// The house deny list, in house order.
///
/// One list, so that there is nothing to choose between. The tree's own gate
/// holds the same list; this copy is the one the **xtask** is held to, and
/// the xtask is the one crate the inner gate cannot see.
pub const LINTS: &[&str] = &[
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::unimplemented",
    "clippy::todo",
    "clippy::unreachable",
];

/// The lints, normalised: no whitespace, lower case, no `clippy::`.
///
/// The formatting of the attribute is the formatter's business and not the
/// gate's: one line and six lines are the same six lints.
pub fn normalise(body: &str) -> Option<String> {
    let start = body.find("#![deny(")?;
    let rest = &body[start + "#![deny(".len()..];
    let end = rest.find(")]")?;
    Some(strip(&rest[..end]))
}

/// The house list in the same form [`normalise`] gives.
pub fn canonical() -> String {
    strip(LINTS.join(",").as_str())
}

/// Strip what the comparison does not care about.
fn strip(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase().replace("clippy::", "")
}

/// The crate roots of the tree: every directory under `crates/` and `apps/`,
/// plus the xtask.
///
/// Discovered rather than listed, so a new crate is checked the moment it is
/// created instead of the moment somebody remembers.
pub fn crate_roots(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for group in ["crates", "apps"] {
        let dir = root.join(group);
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            for name in ["lib.rs", "main.rs"] {
                let candidate = path.join("src").join(name);
                if candidate.is_file() {
                    roots.push(candidate);
                    break;
                }
            }
        }
    }
    // The xtask is beside `crates/` and `apps/`, not inside them.
    for name in ["lib.rs", "main.rs"] {
        let candidate = root.join("xtask").join("src").join(name);
        if candidate.is_file() {
            roots.push(candidate);
            break;
        }
    }
    roots.sort();
    Ok(roots)
}

/// The `mod` groups of a crate root: the runs of `mod` lines a blank line
/// separates.
///
/// A group is a list a change collides on, so it is sorted: two people adding
/// modules to the same crate should not have to merge a list.
pub fn mod_groups(source: &str) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("mod ") {
            let name = rest.split(';').next().unwrap_or("").trim().to_string();
            if !name.is_empty() {
                current.push(name);
                continue;
            }
        }
        if trimmed.is_empty() {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
            continue;
        }
        // A `pub mod` is still a module.
        if let Some(rest) = trimmed.strip_prefix("pub mod ") {
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

/// Whether a list is sorted.
///
/// Sorted means sorted: `a` before `b`, and `a` before `ab` because a
/// shorter name that is a prefix of a longer one comes first.
pub fn is_sorted(lines: &[String]) -> bool {
    let mut sorted = lines.to_vec();
    sorted.sort();
    sorted == lines
}

/// The workspace's member list, from the root manifest.
pub fn workspace_members(manifest: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut in_members = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_members = line == "[workspace]";
            continue;
        }
        if !in_members {
            continue;
        }
        if let Some(rest) = line.strip_prefix("members")
            && let Some(rest) = rest.trim_start().strip_prefix('=')
        {
            let list = rest.trim().trim_start_matches('[').trim_end_matches(']');
            for item in list.split(',') {
                let item = item.trim().trim_matches('"');
                if !item.is_empty() {
                    members.push(item.to_string());
                }
            }
        }
    }
    members
}

/// The layer table of the tree's own gate: every crate and its layer.
///
/// Read out of the source, because the inner gate is the **authority**: it
/// is what the tree enforces, and this gate's job is to check that the prose
/// has not drifted away from it.
pub fn the_layers_table(root: &Path) -> Result<BTreeMap<String, u8>, String> {
    let path = root.join(GATES_CRATE);
    let text =
        read_rust(&path, Subject::Everything).map_err(|e| format!("could not read {GATES_CRATE}: {e}"))?;
    let mut layers = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("(\"") else { continue };
        let Some((name, rest)) = rest.split_once("\", ") else { continue };
        // The layer is a number: `("name", 3),`.
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let Ok(layer) = digits.parse::<u8>() else { continue };
        layers.insert(name.to_string(), layer);
    }
    Ok(layers)
}

/// The layer rows of the architecture document: the crates of each layer.
///
/// The document renders the table as one row per layer, with the crates in
/// one cell.
pub fn the_document_layers(root: &Path) -> Result<BTreeMap<u8, BTreeSet<String>>, String> {
    let path = root.join(ARCHITECTURE);
    let text = read_text(&path).map_err(|e| format!("could not read {ARCHITECTURE}: {e}"))?;
    let mut rows: BTreeMap<u8, BTreeSet<String>> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let Some(first) = cells.get(1) else { continue };
        let layer = first.trim_matches('*').trim_start_matches("L").trim();
        let Ok(layer) = layer.parse::<u8>() else { continue };
        let Some(second) = cells.get(2) else { continue };
        let names: BTreeSet<String> = second
            .split(',')
            .map(|name| name.trim().trim_matches('`').to_string())
            .filter(|name| !name.is_empty())
            .collect();
        if names.is_empty() {
            continue;
        }
        let entry = rows.entry(layer).or_default();
        for name in names {
            entry.insert(name);
        }
    }
    Ok(rows)
}

/// Every `` `name.md` `` in `text`, as (line number, name).
///
/// Only a name that ends in `.md` and holds no space counts: a URL, a bare
/// word and a fenced example are all left alone.
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

/// Every markdown document in the tree's prose.
pub fn prose_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in PROSE {
        let path = root.join(entry);
        if path.is_file() {
            found.push(path);
        } else if path.is_dir() {
            let mut here = Vec::new();
            walk(&path, &path, &mut here, &|p| p.extension().and_then(|x| x.to_str()) == Some("md"));
            found.extend(here);
        }
    }
    found.sort();
    found
}

/// The tree's layers, and the document's rendering of them, agree.
pub fn the_layer_table_matches_the_code(root: &Path) -> Result<Vec<String>, String> {
    let code = the_layers_table(root)?;
    if code.is_empty() {
        return Err(format!("{GATES_CRATE} has no layer table; the scan is wrong, not the repository"));
    }
    let document = the_document_layers(root)?;
    if document.is_empty() {
        return Err(format!("{ARCHITECTURE} has no layer table"));
    }

    let mut problems = Vec::new();

    // Every crate of the tree is in the table.
    let mut by_layer: BTreeMap<u8, BTreeSet<String>> = BTreeMap::new();
    for (name, layer) in &code {
        by_layer.entry(*layer).or_default().insert(name.clone());
    }

    for (layer, names) in &by_layer {
        let Some(shown) = document.get(layer) else {
            problems.push(format!(
                "{ARCHITECTURE} has no row for L{layer}, and the tree has {} crate(s) in it",
                names.len()
            ));
            continue;
        };
        for name in names.difference(shown) {
            problems.push(format!(
                "{ARCHITECTURE} does not name `{name}`, which the tree's gate puts in L{layer}"
            ));
        }
    }

    for (layer, names) in &document {
        let Some(shown) = by_layer.get(layer) else {
            problems.push(format!(
                "{ARCHITECTURE} has a row for L{layer}, and the tree's gate has no crate in it"
            ));
            continue;
        };
        for name in names.difference(shown) {
            problems.push(format!("{ARCHITECTURE} names `{name}` in L{layer}, and the tree's gate does not"));
        }
    }

    Ok(problems)
}

/// Every gate the tree names is actually run.
///
/// A gate that is not run is a rule that is true only by accident, and the
/// tree's rule is that a command which is not a phase of `ci` must be run **by
/// name** in the workflow — so this gate reads both and checks that every
/// command is covered by one or the other.
pub fn every_gate_is_run(root: &Path) -> Result<Vec<String>, String> {
    let workflow = read_text(root.join(WORKFLOW)).map_err(|e| format!("could not read {WORKFLOW}: {e}"))?;
    let mut problems = Vec::new();
    for command in crate::COMMANDS {
        if !crate::is_gate(command.name) {
            continue;
        }
        let in_ci = crate::ci::PHASES.iter().any(|phase| phase.name == command.name);
        let in_workflow = workflow.contains(&format!("cargo xtask {}", command.name));
        if in_ci || in_workflow {
            continue;
        }
        problems.push(format!(
            "`cargo xtask {}` is a gate and nothing runs it: it is not a phase of `ci`, and \
             {WORKFLOW} does not name it.\n     A gate that nothing runs is a rule that is true \
             by accident.",
            command.name
        ));
    }
    Ok(problems)
}

/// Every document link in the prose points at a file that is in the tree.
pub fn broken_document_links(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for doc in prose_files(root) {
        let Ok(text) = read_text(&doc) else { continue };
        let relative = relative_name(root, &doc);
        for (number, target) in document_links(&text) {
            let beside = doc.parent().map(|d| d.join(&target));
            let from_root = root.join(&target);
            if beside.as_deref().is_some_and(|p| p.is_file()) || from_root.is_file() {
                continue;
            }
            problems
                .push(format!("{relative}: line {number} points at `{target}`, which is not in the tree"));
        }
    }
    problems.sort();
    Ok(problems)
}

/// Every cell of the **parity matrix** names the test that proves it.
///
/// The matrix is a claim about the doors, and a claim is worth only what
/// proves it. A cell that says something and names no test is a claim with
/// no proof, and a claim with no proof is true until the next change.
///
/// So the rule has two halves, and both of them bite:
///
/// - **every cell that says something names a test** — a dash says nothing,
///   and a dash is the only cell that needs no proof;
/// - **every test it names is a test that is there** — the file is in the
///   tree, it is under a `tests` directory, and it has a function of that
///   name in it.
///
/// The citation is `path::test_name`. The path is **relative to the crate
/// the row names** — the door's own crate, and the transport's crate for
/// the agent door — unless it starts at the root, and then it is the tree's
/// own path.
/// Every command the tree has is **run by a test**.
///
/// A command is a rule the kit publishes: it is a thing a door can run,
/// and a door runs it. A command that no test names is a command that
/// nothing proves works, and the tree's rule is that every rule has a
/// test.
///
/// The check is about the **name**: a command's id must be named in a
/// test file of the tree, as a name of its own and not as part of a
/// longer one — `task.list` is not named by a test that says
/// `task.list.all`.
///
/// The **commands** and the **kinds** of the batteries are read by the
/// index, which knows the macros a battery writes its rows with; the
/// kit's own are read from the crates that are not batteries, where the
/// attribute is at the start of its line.
///
/// A **kind** is what the document may hold, and it is a rule the kit
/// publishes as much as a command is: a kind that no test names is a
/// kind that nothing proves draws, or measures, or means what it says.
pub fn every_command_is_run_by_a_test(root: &Path) -> Result<Vec<String>, String> {
    let mut commands: Vec<String> = Vec::new();
    let mut kinds: Vec<String> = Vec::new();
    let index = crate::index::build(root)?;
    commands.extend(index.commands.iter().map(|command| command.id.clone()));
    kinds.extend(index.kinds.iter().map(|kind| kind.id.clone()));
    for directory in crate_directories(root) {
        let name = directory.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
        // The batteries are the index's own, and the index knows the
        // macros they write their rows with.
        if name.starts_with("craftkit-plugin-") {
            continue;
        }
        for file in crate::index::rust_files(&directory.join("src")) {
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            commands.extend(commands_at_the_start_of_a_line(&text));
            kinds.extend(crate::index::kinds_in(&text, &file, &name).into_iter().map(|kind| kind.id));
        }
    }
    // Every test file in the tree, and what it says.
    let mut tests: Vec<String> = Vec::new();
    for directory in crate_directories(root) {
        for file in crate::index::rust_files(&directory.join("tests")) {
            tests.push(std::fs::read_to_string(&file).unwrap_or_default());
        }
    }
    commands.sort();
    commands.dedup();
    kinds.sort();
    kinds.dedup();
    let mut problems = Vec::new();
    for command in commands {
        if tests.iter().any(|body| names_it(body, &command)) {
            continue;
        }
        problems.push(format!(
            "the command `{command}` is run by no test: every command the tree has is run by a test"
        ));
    }
    for kind in kinds {
        if tests.iter().any(|body| names_it(body, &kind)) {
            continue;
        }
        problems.push(format!(
            "the kind `{kind}` is named by no test: every kind the tree has is named by a test"
        ));
    }
    Ok(problems)
}

/// The crate **directories** of the tree: every directory under `crates/`
/// and `apps/`, plus the xtask.
///
/// The crate roots are the files; this is the folders, and a folder is
/// what holds a crate's tests.
fn crate_directories(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for group in ["crates", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.push(path);
            }
        }
    }
    let xtask = root.join("xtask");
    if xtask.is_dir() {
        out.push(xtask);
    }
    out.sort();
    out
}

/// The ids of the commands a file declares, where the attribute is at the
/// start of its line.
///
/// The attribute is at the start of its line when it is a **declaration**;
/// an attribute in a comment or in a string is an example of one, and the
/// example is not a command.
fn commands_at_the_start_of_a_line(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[command(") {
        let start = offset + at;
        let Some((block, end)) = crate::index::block_of(text, start + "#[command(".len()) else { break };
        // The attribute is the first thing on its line: what is before it
        // on that line is nothing.
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = crate::index::field(&block, "id")
        {
            found.push(id);
        }
        offset = end;
    }
    found
}

/// Whether a piece of text **names** a command.
///
/// The name is a name of its own: the characters around it are not part
/// of an id, so `task.list` is not named by a test that says
/// `task.list.all`.
fn names_it(body: &str, id: &str) -> bool {
    let mut start = 0usize;
    while let Some(at) = body[start..].find(id) {
        let at = start + at;
        let before = body[..at].chars().next_back();
        let after = body[at + id.len()..].chars().next();
        if is_not_part_of_an_id(before) && is_not_part_of_an_id(after) {
            return true;
        }
        start = at + 1;
    }
    false
}

/// Whether a character is not part of a command's id.
fn is_not_part_of_an_id(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        !character.is_ascii_lowercase() && !character.is_ascii_digit() && character != '.'
    })
}

pub fn every_matrix_cell_names_a_test(root: &Path) -> Result<Vec<String>, String> {
    let matrix = root.join("docs/door-parity.md");
    let text = read_text(&matrix).map_err(|error| format!("could not read docs/door-parity.md: {error}"))?;
    let mut problems = matrix_cell_problems(&text, |path| {
        let file = root.join(path);
        if file.is_file() { Some(file) } else { None }
    });
    problems.sort();
    Ok(problems)
}

/// The problems of a matrix, and a way to find the file a citation names.
///
/// The finder is a **parameter**, so that a test can ask what happens when
/// a cell names a test that is not there — and a gate that cannot fail is
/// a gate that is not there.
fn matrix_cell_problems(text: &str, find: impl Fn(&str) -> Option<std::path::PathBuf>) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(rows) = matrix_rows(text) else {
        problems.push(
            "docs/door-parity.md: there is no table under `## The matrix`, and a matrix with no \
             table is not a matrix"
                .to_string(),
        );
        return problems;
    };
    if rows.is_empty() {
        problems.push(
            "docs/door-parity.md: the matrix has no rows, and a table with no rows says nothing".to_string(),
        );
        return problems;
    }
    for row in rows {
        let Some(base) = row.base else {
            problems.push(format!(
                "docs/door-parity.md: line {}: the row of `{}` does not name the crate its proofs \
                 live in",
                row.number, row.door
            ));
            continue;
        };
        for cell in &row.cells {
            if cell.text == "—" {
                continue;
            }
            let citations = matrix_test_citations(&cell.text);
            if citations.is_empty() {
                problems.push(format!(
                    "docs/door-parity.md: line {}: the cell `{}` of the `{}` row names no test, and \
                     a cell that says something needs one",
                    cell.number, cell.text, row.door
                ));
                continue;
            }
            for (_, path, name) in citations {
                let full = if path.starts_with("crates/") || path.starts_with("apps/") {
                    path.to_string()
                } else {
                    format!("{base}/{path}")
                };
                let Some(file) = find(&full) else {
                    problems.push(format!(
                        "docs/door-parity.md: line {}: names `{path}`, which is not in the tree",
                        cell.number
                    ));
                    continue;
                };
                if !file.to_string_lossy().replace('\\', "/").contains("/tests/") {
                    problems.push(format!(
                        "docs/door-parity.md: line {}: `{path}` is not a test file, and a proof is \
                         a test",
                        cell.number
                    ));
                    continue;
                }
                let Ok(source) = read_rust(&file, Subject::Everything) else {
                    problems.push(format!(
                        "docs/door-parity.md: line {}: names `{path}`, which could not be read",
                        cell.number
                    ));
                    continue;
                };
                if !has_test_function(&source, &name) {
                    problems.push(format!(
                        "docs/door-parity.md: line {}: names `{path}::{name}`, and that file has no \
                         such test",
                        cell.number
                    ));
                }
            }
        }
    }
    problems
}

/// One row of the matrix, and the cells that are in it.
struct MatrixRow {
    /// The line the row is on.
    number: usize,
    /// What the row is about: the door.
    door: String,
    /// The crate the row's proofs live in.
    base: Option<String>,
    /// The cells that say something.
    cells: Vec<MatrixCell>,
}

/// One cell of the matrix.
struct MatrixCell {
    /// The line the cell is on.
    number: usize,
    /// What the cell says.
    text: String,
}

/// Whether a file has a **test** of that name in it.
///
/// A proof is a test, and not any function that happens to carry the
/// name: the attribute is what makes it one, and the tree's tests put it
/// on the line above the function.
///
/// The window is bounded: the attribute is a few lines above the
/// function, and the tree puts a doc comment between them at most.
fn has_test_function(source: &str, name: &str) -> bool {
    let needle = format!("fn {name}(");
    let mut from = 0;
    while let Some(at) = source[from..].find(&needle) {
        let start = from + at;
        // The window is the last few hundred bytes before the function,
        // moved forward to the start of a character.
        let mut window = start.saturating_sub(400);
        while window < start && !source.is_char_boundary(window) {
            window += 1;
        }
        if source[window..start].contains("#[test]") {
            return true;
        }
        from = start + needle.len();
    }
    false
}

/// The rows of the matrix table, under `## The matrix`.
///
/// The table is found by its **heading**, and the heading is what says
/// which table is the matrix: a document may hold several tables, and only
/// one of them is the claim about the doors.
fn matrix_rows(text: &str) -> Option<Vec<MatrixRow>> {
    let mut after = false;
    let mut header = false;
    let mut table = false;
    let mut rows = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if line.starts_with("## ") {
            after = line.trim() == "## The matrix";
            header = false;
            continue;
        }
        if !after {
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if rows.is_empty() {
                continue;
            }
            break;
        }
        if !header {
            header = true;
            table = true;
            continue;
        }
        if trimmed.starts_with("|---") {
            continue;
        }
        let cells = trimmed.strip_prefix('|')?.strip_suffix('|')?;
        let parts: Vec<String> = cells.split('|').map(|cell| cell.trim().to_string()).collect();
        let Some(first) = parts.first() else { continue };
        let door = first.split('—').next().unwrap_or(first).trim().to_string();
        let base = first.split('`').nth(1).filter(|path| path.contains('/')).map(|path| path.to_string());
        let mut cells = Vec::new();
        for part in parts.iter().skip(1) {
            cells.push(MatrixCell { number: number + 1, text: part.clone() });
        }
        rows.push(MatrixRow { number: number + 1, door, base, cells });
    }
    if table { Some(rows) } else { None }
}

/// The `path::test_name` citations in a piece of prose.
///
/// A citation is a path with a `/` in it, a `::`, and a name that looks
/// like a Rust function: letters, digits and underscores.
pub fn matrix_test_citations(text: &str) -> Vec<(usize, String, String)> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(open) = rest.find('`') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('`') else { break };
            let name = &after[..close];
            if let Some((path, test)) = name.split_once("::") {
                let looks_like_a_test = path.contains('/') && path.ends_with(".rs") && !path.contains(' ');
                let looks_like_a_function =
                    !test.is_empty() && test.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                if looks_like_a_test && looks_like_a_function {
                    found.push((number + 1, path.to_string(), test.to_string()));
                }
            }
            rest = &after[close + 1..];
        }
    }
    found
}

/// The `mod` groups of every crate root are sorted.
pub fn the_mod_groups_are_sorted(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for path in crate_roots(root)? {
        let Ok(source) = read_rust(&path, Subject::Everything) else { continue };
        for group in mod_groups(&source) {
            if is_sorted(&group) {
                continue;
            }
            let mut sorted = group.clone();
            sorted.sort();
            problems.push(format!(
                "{}: these `mod` lines are not sorted: {}\n     Sorted: {}",
                relative_name(root, &path),
                group.join(", "),
                sorted.join(", ")
            ));
        }
    }
    Ok(problems)
}

/// The workspace's member list covers every crate that is in the tree.
///
/// The list may be a glob — `crates/*` — and a glob covers what it matches,
/// so the rule is about the crates a glob would miss: a crate in a directory
/// the member list does not name.
pub fn every_crate_is_a_member(root: &Path) -> Result<Vec<String>, String> {
    let manifest =
        read_text(root.join("Cargo.toml")).map_err(|e| format!("could not read Cargo.toml: {e}"))?;
    let members = workspace_members(&manifest);
    if members.is_empty() {
        return Err("Cargo.toml has no members list to check".to_string());
    }
    let mut problems = Vec::new();
    for group in ["crates", "apps"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            let covered = members.iter().any(|member| {
                member == name
                    || member == &format!("{group}/*")
                    || member == &format!("{group}/{name}")
                    || (member.ends_with("/*")
                        && member.trim_end_matches("/*") == group
                        && member == &format!("{group}/*"))
            });
            if covered {
                continue;
            }
            problems.push(format!(
                "{group}/{name} is in the tree and the workspace's members list does not cover it"
            ));
        }
    }
    Ok(problems)
}

/// The xtask's own crate root carries the house deny list.
///
/// The tree's inner gate looks in `crates/` and `apps/`. The xtask is beside
/// them, so nothing else would hold it to the same list — and a gate that
/// does not hold itself to the tree's rules is a gate whose rules are a
/// preference.
pub fn the_xtask_carries_the_house_deny(root: &Path) -> Result<Vec<String>, String> {
    let path = root.join("xtask").join("src").join("lib.rs");
    let source = read_rust(&path, Subject::Everything)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    match normalise(&source) {
        Some(found) if found == canonical() => Ok(Vec::new()),
        Some(found) => {
            Ok(vec![format!("{} does not carry the house lints: {found}", relative_name(root, &path))])
        }
        None => Ok(vec![format!("{} has no deny attribute", relative_name(root, &path))]),
    }
}

/// Every retired rule is named, and every name says why.
pub fn the_retired_rules_are_named() -> Vec<String> {
    let mut problems = Vec::new();
    if RETIRED.is_empty() {
        problems.push(
            "the table of retired rules is empty: a rule that is dropped without a reason is a \
             guarantee the tree lost without anybody noticing"
                .to_string(),
        );
        return problems;
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for (name, reason) in RETIRED {
        if !seen.insert(name) {
            problems.push(format!("the retired rule `{name}` is named twice"));
        }
        if reason.trim().is_empty() {
            problems.push(format!("the retired rule `{name}` does not say why it no longer applies"));
        }
    }
    problems
}

/// The delegation rule is stated in the prose, and the prose that carries it
/// is named.
///
/// **A rule of conduct is a rule like any other**, and it decays the same
/// way: the rule that was written once is forgotten, and the tree goes back
/// to spawning a survey for every question. So the rule is in the tree's
/// prose, and the gate checks that the prose still says it.
///
/// Two files carry it, because two kinds of reader read them: the playbook
/// for whoever drives the tree, and `AGENTS.md` for whoever works in it.
pub fn the_delegation_rule_is_stated(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for file in ["docs/agent-playbook.md", "AGENTS.md"] {
        let text = read_text(root.join(file)).map_err(|e| format!("could not read {file}: {e}"))?;
        if !text.contains("bounded, mechanical brief") {
            problems.push(format!(
                "{file} does not state the delegation rule: a sub-agent gets a bounded, mechanical \
                 brief, or it does not get spawned"
            ));
        }
    }
    Ok(problems)
}

/// The **measurement** has a procedure, and the procedure is in the prose.
///
/// The plan's exit numbers are session-side: the calls, the round trips and
/// the machine time are properties of the session that did the work, so
/// they are not in the tree — but the **procedure** that reads them is, and
/// a procedure that is not written down is a procedure nobody follows.
///
/// So the playbook states the rule and names the baseline, and the gate
/// says so when it does not.
pub fn the_measurement_procedure_is_stated(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let playbook = root.join("docs/agent-playbook.md");
    let text = read_text(&playbook).map_err(|e| format!("could not read the playbook: {e}"))?;
    // The rule: a shortcut that does not move a number did not happen.
    if !text.contains("does not move a number") {
        problems.push(
            "the playbook does not state the rule of the measurement: a shortcut that does not move \
             a number did not happen"
                .to_string(),
        );
    }
    // The baseline: the numbers the survey measured, and the tree's own
    // half is named.
    for needle in ["35.4", "959", "cargo xtask census"] {
        if !text.contains(needle) {
            problems
                .push(format!("the playbook does not name `{needle}`: the baseline is read, not remembered"));
        }
    }
    // The survey holds the baseline the playbook reads against.
    let survey = root.join("docs/survey-cost-and-meta.md");
    let survey_text = read_text(&survey).map_err(|e| format!("could not read the survey: {e}"))?;
    for needle in ["35.4", "26.8", "959"] {
        if !survey_text.contains(needle) {
            problems.push(format!(
                "the survey does not hold `{needle}`: the baseline the waves move against is not \
                 there"
            ));
        }
    }
    Ok(problems)
}

/// The **second survey** is held to the same rule as the first.
///
/// A survey that is repeated has to keep its numbers, or the repetition is a
/// story rather than a measurement. So the second survey is in the tree, and
/// the gate says so when it loses the numbers it measured — the calls an arc
/// makes, the machine time an arc takes, the arcs a day.
pub fn the_second_survey_holds_its_numbers(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let survey = root.join("docs/survey-cost-and-meta-2.md");
    let text = read_text(&survey).map_err(|e| format!("could not read the second survey: {e}"))?;
    for needle in ["48.3", "12.5 min", "26.4", "cargo xtask census"] {
        if !text.contains(needle) {
            problems.push(format!(
                "the second survey does not hold `{needle}`: the measurement it repeated is not \
                 there"
            ));
        }
    }
    // The second survey says what it found, including what it did not meet.
    if !text.contains("not met") {
        problems.push(
            "the second survey does not say which wave exits were not met: a measurement that \
             reports only the wins is not a measurement"
                .to_string(),
        );
    }
    Ok(problems)
}

/// The **language server** was measured, and the measurement is in the tree.
///
/// The house says *ask it before you grep*. The survey says what that costs
/// and how often it was done, and the gate holds the survey to its numbers —
/// because a house rule that has been measured is a rule that may be
/// narrowed, and a narrowing that is not written down is a rule that only
/// lives in somebody's head.
pub fn the_lsp_survey_holds_its_numbers(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let survey = root.join("docs/survey-lsp.md");
    let text = read_text(&survey).map_err(|e| format!("could not read the survey of the lsp: {e}"))?;
    for needle in ["0.005", "rust-analyzer", "batch mode", "never again"] {
        if !text.contains(needle) {
            problems.push(format!(
                "the survey of the lsp does not hold `{needle}`: the measurement it made is not there"
            ));
        }
    }
    Ok(problems)
}

/// The **third survey** holds the numbers it measured.
///
/// The second survey promised that the next measurement would be **one
/// number** — the cost of an arc, end to end — and the third survey is
/// that measurement. A survey that has lost its numbers is a survey that
/// says nothing, and the gate says so.
pub fn the_third_survey_holds_its_numbers(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let survey = root.join("docs/survey-cost-and-meta-3.md");
    let text = read_text(&survey).map_err(|e| format!("could not read the third survey: {e}"))?;
    for needle in ["58.5", "5.1 min", "50.6", "13.3 min", "cargo xtask survey"] {
        if !text.contains(needle) {
            problems.push(format!(
                "the third survey does not hold `{needle}`: the measurement it made is not there"
            ));
        }
    }
    // The third survey says what it found, including what it did not meet.
    if !text.contains("not met") {
        problems.push(
            "the third survey does not say which wave exits were not met: a measurement that \
             reports only the wins is not a measurement"
                .to_string(),
        );
    }
    Ok(problems)
}

/// The **gap survey** holds what it found.
///
/// A survey of what is left is a claim about the tree, and a claim that
/// is not held to its numbers is a claim that drifts: the manifest grows
/// and the survey still says what it said.
pub fn the_gap_survey_holds_its_numbers(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let survey = root.join("docs/survey-gap.md");
    let text = read_text(&survey).map_err(|e| format!("could not read the gap survey: {e}"))?;
    for needle in ["240", "out of scope", "not read"] {
        if !text.contains(needle) {
            problems.push(format!("the gap survey does not hold `{needle}`: what it found is not there"));
        }
    }
    Ok(problems)
}

/// The **survey of the batteries** holds what it found.
///
/// The survey of the batteries is a claim about the whole tree: it names
/// every battery, every battery's family tree, and what is left of each.
/// A survey that loses a battery is a survey that says less than it
/// measured, and the gate says so.
pub fn the_battery_survey_holds_its_numbers(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let survey = root.join("docs/survey-batteries.md");
    let text = read_text(&survey).map_err(|e| format!("could not read the survey of the batteries: {e}"))?;
    // **Every battery has a row.** The survey is of the batteries, and a
    // battery it does not mention is a battery it did not read.
    for battery in ["pdf", "prose", "film", "fx", "cad", "vector", "sound", "art", "layout"] {
        if !text.contains(&format!("| **{battery}** |")) {
            problems
                .push(format!("the survey of the batteries does not have a row for the `{battery}` battery"));
        }
    }
    // The four lists are the survey's own shape: what is coarse, what is
    // refused, what is out of scope, and what is to judge.
    for needle in
        ["deliberately coarse", "refused, with a reason", "out of scope", "to judge — the owner's list"]
    {
        if !text.contains(needle) {
            problems.push(format!(
                "the survey of the batteries does not hold `{needle}`: one of its four lists is not \
                 there"
            ));
        }
    }
    // And it says what it does not settle.
    if !text.contains("What this survey does not settle") {
        problems.push(
            "the survey of the batteries does not say what it does not settle: a measurement that \
             reports only the wins is not a measurement"
                .to_string(),
        );
    }
    Ok(problems)
}

/// The **counts a battery states in its own words** are the tree's counts.
///
/// A battery that says how many commands it carries is making a claim
/// about the tree, and a claim that is not held to the tree drifts. The
/// sound battery said a hundred and thirty-six; the tree has a hundred
/// and forty-four.
///
/// **The number may be spelled out.** "a hundred and forty-four" is a
/// number, and the tree's prose writes its numbers in words.
pub fn the_batteries_counts_are_true(root: &Path) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    let index = crate::index::build(root).map_err(|e| format!("the tree could not be indexed: {e}"))?;
    for battery in crate::index::battery_names(root) {
        let file = root.join("crates").join(format!("craftkit-plugin-{battery}")).join("src/lib.rs");
        let Ok(text) = crate::tree::read_rust(&file, crate::tree::Subject::Everything) else {
            continue;
        };
        let count = crate::index::commands_of(&index, &battery).len();
        for stated in stated_counts(&text) {
            if stated != count {
                problems.push(format!(
                    "the `{battery}` battery says it carries **{stated} commands**, and the tree's \
                     index finds {count}"
                ));
            }
        }
    }
    Ok(problems)
}

/// The counts a file states in its own words.
///
/// **A bullet, and only a bullet.** A line that begins with a bullet and
/// whose bold words are a number and the word *commands* is a count; a
/// sentence that happens to say "twelve commands" is not one.
pub fn stated_counts(text: &str) -> Vec<usize> {
    let mut found = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // **The comment's own mark is not the bullet.** The prose of a
        // crate is written as `//! - **…**`.
        let trimmed =
            trimmed.strip_prefix("//!").or_else(|| trimmed.strip_prefix("//")).unwrap_or(trimmed).trim();
        let Some(rest) = trimmed.strip_prefix("- **") else { continue };
        let Some(end) = rest.find("**") else { continue };
        let words = &rest[..end];
        let Some(number) = words.strip_suffix(" commands") else { continue };
        if let Some(count) = number_in_words(number) {
            found.push(count);
        }
    }
    found
}

/// A number in words: "one", "sixty-seven", "a hundred and forty-four".
pub fn number_in_words(words: &str) -> Option<usize> {
    let words = words.trim().to_ascii_lowercase();
    if words.is_empty() {
        return None;
    }
    // A number written in digits is a number too.
    if let Ok(number) = words.parse::<usize>() {
        return Some(number);
    }
    let mut total = 0usize;
    let mut part = 0usize;
    for word in words.split(['-', ' ']) {
        if word.is_empty() || word == "and" || word == "a" {
            continue;
        }
        match word {
            "one" => part += 1,
            "two" => part += 2,
            "three" => part += 3,
            "four" => part += 4,
            "five" => part += 5,
            "six" => part += 6,
            "seven" => part += 7,
            "eight" => part += 8,
            "nine" => part += 9,
            "ten" => part += 10,
            "eleven" => part += 11,
            "twelve" => part += 12,
            "thirteen" => part += 13,
            "fourteen" => part += 14,
            "fifteen" => part += 15,
            "sixteen" => part += 16,
            "seventeen" => part += 17,
            "eighteen" => part += 18,
            "nineteen" => part += 19,
            "twenty" => part += 20,
            "thirty" => part += 30,
            "forty" => part += 40,
            "fifty" => part += 50,
            "sixty" => part += 60,
            "seventy" => part += 70,
            "eighty" => part += 80,
            "ninety" => part += 90,
            "hundred" => part = (part.max(1)) * 100,
            "thousand" => total += part.max(1) * 1000,
            _ => return None,
        }
    }
    Some(total + part)
}

/// The gate itself.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("lists");
    let mut problems = Vec::new();

    // The layer table, in the tree's own gate and in the document that renders
    // it. The inner gate is the authority; the document is the rendering.
    problems.extend(the_layer_table_matches_the_code(root)?);

    // Every gate the tree names is run.
    problems.extend(every_gate_is_run(root)?);

    // Every document link in the prose points at a file that is in the tree.
    problems.extend(broken_document_links(root)?);

    // Every cell of the parity matrix names a test, and the test is there.
    problems.extend(every_matrix_cell_names_a_test(root)?);

    // Every command the tree has is run by a test.
    problems.extend(every_command_is_run_by_a_test(root)?);

    // The `mod` groups are sorted.
    problems.extend(the_mod_groups_are_sorted(root)?);

    // Every crate in the tree is a member of the workspace.
    problems.extend(every_crate_is_a_member(root)?);

    // The xtask holds itself to the tree's deny list.
    problems.extend(the_xtask_carries_the_house_deny(root)?);

    // Every retired rule is named, and says why.
    problems.extend(the_retired_rules_are_named());

    // The rule about delegation is in the prose, and the prose is in the tree.
    problems.extend(the_delegation_rule_is_stated(root)?);

    // The measurement has a procedure, and the procedure is in the prose.
    problems.extend(the_measurement_procedure_is_stated(root)?);

    // The second survey is held to the same rule as the first.
    problems.extend(the_second_survey_holds_its_numbers(root)?);

    // The survey of the language server holds its own numbers.
    problems.extend(the_lsp_survey_holds_its_numbers(root)?);

    // The third survey holds its own numbers.
    problems.extend(the_third_survey_holds_its_numbers(root)?);

    // The gap survey holds what it found.
    problems.extend(the_gap_survey_holds_its_numbers(root)?);

    // The survey of the batteries holds what it found.
    problems.extend(the_battery_survey_holds_its_numbers(root)?);

    // The counts a battery states in its own words are the tree's counts.
    problems.extend(the_batteries_counts_are_true(root)?);

    // Every port of the manifest is still in the tree.
    let index = crate::index::build(root).map_err(|e| format!("the tree could not be indexed: {e}"))?;
    problems.extend(crate::ports::problems(root, &index));

    // The claim table is an index: what it cites exists, and what the tree
    // made is claimed.
    let readme = read_text(root.join("README.md")).map_err(|e| format!("could not read the README: {e}"))?;
    let port_sections: Vec<usize> = crate::ports::PORTS
        .iter()
        .filter_map(|port| port.section.trim_start_matches('§').parse::<usize>().ok())
        .collect();
    problems.extend(crate::claims::problems(&readme, &port_sections));

    // The claim table names the commands the tree has, and the registry
    // knows every one of them: a row that names a command the tree does not
    // have is a stale claim, and it is caught here.
    let commands: Vec<String> = index.commands.iter().map(|command| command.id.clone()).collect();
    problems.extend(crate::claims::problems_about_commands(&readme, &commands));

    if problems.is_empty() {
        println!("   ok");
        return Ok(crate::Outcome::Ran);
    }
    for problem in &problems {
        println!("   ✗ {problem}");
    }
    Err(format!("{} problem(s) in the tree's lists", problems.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deny_attribute_is_matched_whatever_the_formatting() {
        let one_line = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]";
        assert_eq!(normalise(one_line).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn a_deny_of_a_different_set_of_lints_is_not_the_house_one() {
        let shorter = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::unreachable)]";
        assert_ne!(normalise(shorter).as_deref(), Some(canonical().as_str()));
    }

    #[test]
    fn the_xtask_carries_the_house_deny_list() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = the_xtask_carries_the_house_deny(&root).expect("the xtask is there");
        assert!(problems.is_empty(), "the xtask does not carry the house lints: {problems:?}");
    }

    /// A command that no test names is **caught**.
    ///
    /// The tree the test builds is a small one: a crate with a command
    /// and a test that does not name it.
    #[test]
    fn a_command_that_no_test_names_is_caught() {
        let root = crate::temp::dir("lists-command").expect("a temporary folder");
        let crate_directory = root.join("crates").join("craftkit-thing");
        std::fs::create_dir_all(crate_directory.join("src")).expect("the directory is made");
        std::fs::create_dir_all(crate_directory.join("tests")).expect("the directory is made");
        std::fs::write(
            crate_directory.join("src").join("lib.rs"),
            "#[derive(Command)]\n#[command(id = \"thing.add\")]\npub struct Add;\n",
        )
        .expect("the source is written");
        std::fs::write(crate_directory.join("tests").join("thing.rs"), "#[test]\nfn a_test() {}\n")
            .expect("the test is written");
        let problems = every_command_is_run_by_a_test(&root).expect("the tree is read");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("thing.add"), "{problems:?}");
    }

    /// And a command that a test names is **not** caught.
    #[test]
    fn a_command_that_a_test_names_is_not_caught() {
        let root = crate::temp::dir("lists-command-named").expect("a temporary folder");
        let crate_directory = root.join("crates").join("craftkit-thing");
        std::fs::create_dir_all(crate_directory.join("src")).expect("the directory is made");
        std::fs::create_dir_all(crate_directory.join("tests")).expect("the directory is made");
        std::fs::write(
            crate_directory.join("src").join("lib.rs"),
            "#[derive(Command)]\n#[command(id = \"thing.add\")]\npub struct Add;\n",
        )
        .expect("the source is written");
        std::fs::write(
            crate_directory.join("tests").join("thing.rs"),
            "#[test]\nfn a_test() { session.execute(\"thing.add\", &json!({})); }\n",
        )
        .expect("the test is written");
        let problems = every_command_is_run_by_a_test(&root).expect("the tree is read");
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// A name that is **part** of a longer name is not a name.
    #[test]
    fn a_name_that_is_part_of_a_longer_one_is_not_a_name() {
        assert!(!names_it("the command task.list.all is run", "task.list"));
        assert!(names_it("the command task.list is run", "task.list"));
    }

    /// A kind that no test names is **caught**.
    ///
    /// The tree the test builds is a small one: a crate with a node kind
    /// and a test that does not name it.
    #[test]
    fn a_kind_that_no_test_names_is_caught() {
        let root = crate::temp::dir("lists-kind").expect("a temporary folder");
        let crate_directory = root.join("crates").join("craftkit-thing");
        std::fs::create_dir_all(crate_directory.join("src")).expect("the directory is made");
        std::fs::create_dir_all(crate_directory.join("tests")).expect("the directory is made");
        std::fs::write(
            crate_directory.join("src").join("lib.rs"),
            "#[derive(NodeKind)]\n#[node_kind(kind = \"thing.shape\")]\npub struct ShapeKind;\n",
        )
        .expect("the source is written");
        std::fs::write(crate_directory.join("tests").join("thing.rs"), "#[test]\nfn a_test() {}\n")
            .expect("the test is written");
        let problems = every_command_is_run_by_a_test(&root).expect("the tree is read");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("thing.shape"), "{problems:?}");
    }

    /// And a kind that a test names is **not** caught.
    #[test]
    fn a_kind_that_a_test_names_is_not_caught() {
        let root = crate::temp::dir("lists-kind-named").expect("a temporary folder");
        let crate_directory = root.join("crates").join("craftkit-thing");
        std::fs::create_dir_all(crate_directory.join("src")).expect("the directory is made");
        std::fs::create_dir_all(crate_directory.join("tests")).expect("the directory is made");
        std::fs::write(
            crate_directory.join("src").join("lib.rs"),
            "#[derive(NodeKind)]\n#[node_kind(kind = \"thing.shape\")]\npub struct ShapeKind;\n",
        )
        .expect("the source is written");
        std::fs::write(
            crate_directory.join("tests").join("thing.rs"),
            "#[test]\nfn a_test() { assert_eq!(\"thing.shape\", \"thing.shape\"); }\n",
        )
        .expect("the test is written");
        let problems = every_command_is_run_by_a_test(&root).expect("the tree is read");
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// The kind of a **macro** is read from the call, and the call says
    /// what it is.
    #[test]
    fn the_kind_of_a_macro_is_read_from_its_call() {
        let source = "macro_rules! kind_of {\n    ($id:literal) => {\n        #[node_kind(kind = $id)]\n        pub struct K;\n    };\n}\nkind_of!(\"thing.shape\");\n";
        let kinds = crate::index::kinds_in(source, std::path::Path::new("src/lib.rs"), "thing");
        assert!(kinds.is_empty(), "the declaration alone is not a kind: {kinds:?}");
        let mut arguments = std::collections::BTreeMap::new();
        arguments.insert("id".to_string(), 0usize);
        let macros = vec![crate::index::MacroDef {
            name: "kind_of".to_string(),
            arguments,
            id: None,
            label: None,
            node_kind: Some(0),
            evaluator: None,
            doc: None,
            example: false,
            file: std::path::PathBuf::from("src/lib.rs"),
            line: 1,
        }];
        let kinds =
            crate::index::macro_kinds_in(source, std::path::Path::new("src/lib.rs"), "thing", &macros);
        assert_eq!(kinds.len(), 1, "{kinds:?}");
        assert_eq!(kinds[0].id, "thing.shape");
    }

    /// And the attribute in a **comment** is an example, and not a
    /// command.
    #[test]
    fn an_attribute_in_a_comment_is_not_a_command() {
        let source = "//! #[command(id = \"thing.example\")]\n";
        assert!(commands_at_the_start_of_a_line(source).is_empty());
        let source = "#[command(id = \"thing.real\")]\n";
        assert_eq!(commands_at_the_start_of_a_line(source), vec!["thing.real".to_string()]);
    }

    #[test]
    fn a_mod_group_is_read() {
        let source = "mod alpha;\nmod beta;\n\nmod gamma;\n";
        let groups = mod_groups(source);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["alpha".to_string(), "beta".to_string()]);
        assert_eq!(groups[1], vec!["gamma".to_string()]);
    }

    #[test]
    fn a_public_mod_is_read() {
        let source = "pub mod alpha;\npub mod beta;\n";
        assert_eq!(mod_groups(source), vec![vec!["alpha".to_string(), "beta".to_string()]]);
    }

    #[test]
    fn the_mod_groups_of_the_tree_are_sorted() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = the_mod_groups_are_sorted(&root).expect("the tree is there");
        assert!(problems.is_empty(), "the mod groups are not sorted: {problems:?}");
    }

    #[test]
    fn the_members_list_is_read() {
        let manifest = "[workspace]\nmembers = [\"crates/*\", \"apps/*\", \"xtask\"]\n";
        assert_eq!(
            workspace_members(manifest),
            vec!["crates/*".to_string(), "apps/*".to_string(), "xtask".to_string()]
        );
    }

    #[test]
    fn a_sorted_list_is_sorted() {
        assert!(is_sorted(&["a".to_string(), "b".to_string()]));
        assert!(!is_sorted(&["b".to_string(), "a".to_string()]));
    }

    #[test]
    fn the_workspace_covers_every_crate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = every_crate_is_a_member(&root).expect("the tree is there");
        assert!(problems.is_empty(), "a crate is not a member: {problems:?}");
    }

    #[test]
    fn every_retired_rule_says_why() {
        let problems = the_retired_rules_are_named();
        assert!(problems.is_empty(), "the retired rules are not all named: {problems:?}");
    }

    #[test]
    fn the_layer_table_is_read_out_of_the_gates_crate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let layers = the_layers_table(&root).expect("the gate is there");
        assert_eq!(layers.get("craftkit-sdk").copied(), Some(2));
        assert_eq!(layers.get("craftkit-window").copied(), Some(5));
        assert!(layers.len() > 40, "the table has {} crates", layers.len());
    }

    #[test]
    fn the_document_agrees_with_the_gate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = the_layer_table_matches_the_code(&root).expect("the tree is there");
        assert!(problems.is_empty(), "the layer table has drifted: {problems:?}");
    }

    #[test]
    fn the_document_links_are_read() {
        let text = "see `docs/architecture.md` and `README.md`, not `not a path.md` or `http://x/y.md`";
        let links = document_links(text);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].1, "docs/architecture.md");
        assert_eq!(links[1].1, "README.md");
    }

    #[test]
    fn the_document_links_of_the_tree_are_not_broken() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = broken_document_links(&root).expect("the tree is there");
        assert!(problems.is_empty(), "a document link is broken: {problems:?}");
    }

    #[test]
    fn every_gate_is_run_list() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = every_gate_is_run(&root).expect("the tree is there");
        assert!(problems.is_empty(), "a gate is not run: {problems:?}");
    }

    #[test]
    fn the_test_citations_are_read() {
        let text = "see `apps/craftkit-bridge/tests/bridge.rs::a_refused_call_says_so` \
                    and `crates/craftkit-mcp/src/lib.rs` and `no::such` and `a/b.rs::not a fn`";
        let citations = matrix_test_citations(text);
        assert_eq!(citations.len(), 1, "{citations:?}");
        assert_eq!(citations[0].1, "apps/craftkit-bridge/tests/bridge.rs");
        assert_eq!(citations[0].2, "a_refused_call_says_so");
    }

    #[test]
    fn the_matrix_names_tests_that_are_there() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = every_matrix_cell_names_a_test(&root).expect("the tree is there");
        assert!(problems.is_empty(), "the matrix names a test that is not there: {problems:?}");
    }

    /// A matrix with a cell that names **no test** is a problem.
    ///
    /// This is the half of the rule that the tree's own matrix cannot
    /// show: a cell that says something and proves nothing is a claim
    /// that is true until the next change.
    #[test]
    fn a_cell_that_names_no_test_is_a_problem() {
        let text = "## The matrix\n\n| door | lists | runs |\n|---|---|---|\n| the sdk — `crates/craftkit-sdk` | ✓ `help` | ✓ `tests/runtime.rs::a_run_time_command_runs` |\n";
        let problems = matrix_cell_problems(text, |path| {
            let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
            if file.is_file() { Some(file) } else { None }
        });
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("names no test"), "{problems:?}");
    }

    /// A dash is the **only** cell that needs no proof.
    #[test]
    fn a_dash_needs_no_test() {
        let text = "## The matrix\n\n| door | lists | runs |\n|---|---|---|\n| the sdk — `crates/craftkit-sdk` | — | ✓ `tests/runtime.rs::a_run_time_command_runs` |\n";
        let problems = matrix_cell_problems(text, |path| {
            let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
            if file.is_file() { Some(file) } else { None }
        });
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// A matrix with **no table** is a problem: a rule that cannot fail
    /// is a rule that is not there.
    /// A proof is a **test**, and not any function with that name.
    #[test]
    fn a_proof_must_be_a_test() {
        let source = "fn helper() {}\n\n#[test]\nfn a_test() {}\n";
        assert!(has_test_function(source, "a_test"), "the test is there");
        assert!(!has_test_function(source, "helper"), "and the helper is not a test");
        assert!(!has_test_function(source, "nothing"), "and there is no such test");
    }

    #[test]
    fn a_matrix_with_no_table_is_a_problem() {
        let text = "# The doors\n\nThere is no table here.\n";
        let problems = matrix_cell_problems(text, |_| None);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("no table"), "{problems:?}");
    }
}
