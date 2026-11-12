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

    // Every port of the manifest is still in the tree.
    let index = crate::index::build(root).map_err(|e| format!("the tree could not be indexed: {e}"))?;
    problems.extend(crate::ports::problems(root, &index));

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
}
