//! The dependency layering rules, for the v2 tree.
//!
//! The rule, in one sentence: **a crate may depend only on crates in strictly
//! lower layers, a plugin may not depend on another plugin or on an
//! application, and nothing below the application layer may name a UI
//! toolkit.**
//!
//! # Where the table comes from
//!
//! The table is **the v2 tree's own**. `crates/craftkit-gates/src/lib.rs` holds
//! `pub const LAYERS`, and that file is the **authority**: it is the table the
//! tree's own gate enforces, and `docs/architecture.md` says so in as many
//! words. There is one table, and this gate checks the tree against it.
//!
//! The xtask cannot depend on `craftkit-gates` — its manifest says
//! "**Nothing of the tree's own**", because a checker that cannot be built
//! when the tree does not compile is a checker that is silent exactly when it
//! is needed — so the table is read **out of the authority's source text**
//! rather than linked against it. Reading it as text has one failure mode,
//! which is a change to the shape of the table's declaration, and that
//! failure is loud: [`authority_table`] fails with the line it could not read.
//! It never guesses.
//!
//! [`TABLE`] below is a **mirror** of the authority: the same rows, in the
//! same order, as a constant the rest of the xtask can read without a
//! filesystem. A mirror is a second copy, and a second copy is a thing that
//! drifts — so the gate **compares the two** on every run ([`drift`]) and
//! fails when they differ, and a unit test does the same. The authority is
//! the table; the mirror is a copy of it that is not allowed to disagree.
//!
//! # The rules, and what each one is for
//!
//! - **Strictly lower layers.** The value crates are where the interesting
//!   rules live, and they are also where a UI dependency creeps in — somebody
//!   wants a `Vec2` for a layout, reaches for `egui`, and the crate stops
//!   being testable without a window. Nothing about that is visible in
//!   review, and nothing about it fails until much later, on somebody else's
//!   machine.
//! - **The applications are exempt.** They depend on everything, by design,
//!   and every real layering mistake has been made in a library.
//! - **A plugin is a plugin.** The seam between a battery and the rest of the
//!   tree is the SDK, the macro and the libraries. A plugin that reaches for
//!   another plugin, or for an application, has reached around that seam.
//! - **Nothing below the applications names a UI toolkit.** The window is an
//!   application and owns its window; the batteries and the libraries are
//!   drawn *into* it and must not know how.
//!
//! The model here is deliberately independent of `cargo metadata`, so the rules
//! can be unit-tested without running cargo. [`from_metadata`] is the only part
//! that knows where the data comes from.

use std::path::Path;

use serde_json::Value;

use crate::{cargo, command, step, wasm};

/// The file that holds the v2 tree's own layering table.
///
/// **The authority.** Every other copy of the table is a copy of this one.
pub const AUTHORITY: &str = "crates/craftkit-gates/src/lib.rs";

/// The layer the applications are at.
///
/// The applications are exempt from the layering rule, and they are the only
/// crates at this layer, so the layer number is also how the rules say "this
/// is an application".
pub const APP_LAYER: u8 = 5;

/// Where a workspace crate sits.
///
/// `Exempt` exists for the build tool. The applications *are* at a layer
/// ([`APP_LAYER`]) and are exempt by the rules, not by the table: a table
/// that hid them would lose the fact that they exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A library crate at this layer, or an application at [`APP_LAYER`].
    Layer(u8),
    /// The build tool: it is not part of the tree the table describes.
    Exempt,
}

impl Class {
    /// The layer number, or `None` when the rules do not apply.
    pub fn layer(self) -> Option<u8> {
        match self {
            Class::Layer(l) => Some(l),
            Class::Exempt => None,
        }
    }

    /// How a violation report should name this crate.
    pub fn describe(self) -> String {
        match self {
            Class::Layer(l) => format!("L{l}"),
            Class::Exempt => "exempt".to_string(),
        }
    }
}

/// The layering table: every crate of the tree and its layer.
///
/// **A mirror of [`AUTHORITY`].** The rows are the authority's rows, in the
/// authority's order, and the gate fails when the two differ — see
/// [`drift`].
///
/// The names are the full package names, in the authority's order, so the
/// table itself is reviewable as a diff.
pub const TABLE: &[(&str, Class)] = &[
    // L0 — the half that knows nothing about a document: the allocator gate,
    // the macro, the three ABIs, the build-time linker, the readers and the
    // libraries that are arithmetic over plain numbers.
    ("craftkit-alloc", Class::Layer(0)),
    ("craftkit-derive", Class::Layer(0)),
    ("craftkit-ffi-abi", Class::Layer(0)),
    ("craftkit-clap-abi", Class::Layer(0)),
    ("craftkit-vst3-abi", Class::Layer(0)),
    ("craftkit-au-abi", Class::Layer(0)),
    ("craftkit-gates", Class::Layer(0)),
    ("craftkit-link", Class::Layer(0)),
    ("craftkit-image", Class::Layer(0)),
    ("craftkit-jpeg", Class::Layer(0)),
    ("craftkit-deflate", Class::Layer(0)),
    ("craftkit-xml", Class::Layer(0)),
    ("craftkit-payload", Class::Layer(0)),
    ("craftkit-sound-time", Class::Layer(0)),
    ("craftkit-text-layout", Class::Layer(0)),
    ("craftkit-hyphen", Class::Layer(0)),
    ("craftkit-font", Class::Layer(0)),
    ("craftkit-wav", Class::Layer(0)),
    ("craftkit-audio", Class::Layer(0)),
    ("craftkit-midi", Class::Layer(0)),
    ("craftkit-plugins", Class::Layer(0)),
    // The scratch a test works in: a path, and the removal of it.
    ("craftkit-scratch", Class::Layer(0)),
    // L1 — the doors: opening a shared library and calling the table in it,
    // and the reference plugins they open.
    ("craftkit-ffi", Class::Layer(1)),
    ("craftkit-clap", Class::Layer(1)),
    ("craftkit-clap-fixture", Class::Layer(1)),
    ("craftkit-vst3", Class::Layer(1)),
    ("craftkit-vst3-fixture", Class::Layer(1)),
    ("craftkit-au", Class::Layer(1)),
    ("craftkit-au-fixture", Class::Layer(1)),
    ("craftkit-png", Class::Layer(1)),
    ("craftkit-zip", Class::Layer(1)),
    // L2 — the whole surface a plugin needs: the registry, the session, the
    // document and the format.
    ("craftkit-sdk", Class::Layer(2)),
    // L3 — the things above the SDK that are not plugins: the control channel,
    // the MCP door, and the libraries that speak the SDK's own types.
    ("craftkit-control", Class::Layer(3)),
    ("craftkit-mcp", Class::Layer(3)),
    ("craftkit-numeric", Class::Layer(3)),
    ("craftkit-raster", Class::Layer(3)),
    ("craftkit-vector-geom", Class::Layer(3)),
    // L4 — the batteries: the plugins, each one a document model of its own.
    ("craftkit-plugin-art", Class::Layer(4)),
    ("craftkit-plugin-cad", Class::Layer(4)),
    ("craftkit-plugin-clap", Class::Layer(4)),
    ("craftkit-plugin-film", Class::Layer(4)),
    ("craftkit-plugin-fx", Class::Layer(4)),
    ("craftkit-plugin-grid", Class::Layer(4)),
    ("craftkit-plugin-hello", Class::Layer(4)),
    ("craftkit-plugin-image", Class::Layer(4)),
    ("craftkit-plugin-layout", Class::Layer(4)),
    ("craftkit-plugin-numeric", Class::Layer(4)),
    ("craftkit-plugin-pdf", Class::Layer(4)),
    ("craftkit-plugin-prose", Class::Layer(4)),
    ("craftkit-plugin-shapes", Class::Layer(4)),
    ("craftkit-plugin-text", Class::Layer(4)),
    ("craftkit-plugin-shared", Class::Layer(4)),
    ("craftkit-plugin-sound", Class::Layer(4)),
    ("craftkit-plugin-vector", Class::Layer(4)),
    ("craftkit-plugin-vst3", Class::Layer(4)),
    // L5 — the applications. They depend on everything, and the layering rule
    // does not apply to them.
    ("craftkit-bridge", Class::Layer(5)),
    ("craftkit-hello-server", Class::Layer(5)),
    ("craftkit-mcp-server", Class::Layer(5)),
    ("craftkit-repl", Class::Layer(5)),
    ("craftkit-studio", Class::Layer(5)),
    ("craftkit-window", Class::Layer(5)),
    ("craftkit-wasm", Class::Layer(5)),
    // The build tool. It is not part of the tree the authority describes, and
    // it depends on nothing of the tree's own.
    ("xtask", Class::Exempt),
];

/// External crates that make a crate a UI crate.
///
/// Entries ending in `*` are prefixes.
pub const UI_CRATES: &[&str] = &["egui*", "eframe", "egui-wgpu", "wgpu", "winit", "rfd", "accesskit*"];

/// The first layer allowed to name a UI crate.
///
/// The applications are at [`APP_LAYER`] and the window is one of them: it
/// owns its window and its renderer, and that is its whole job. Nothing
/// below it may name a toolkit — a value crate that needs a `Vec2` needs its
/// own tiny vector type, and not `egui`'s.
pub const UI_MIN_LAYER: u8 = APP_LAYER;

/// The prefix a plugin's name carries.
///
/// A plugin is a battery: a document model of its own, reached through the
/// SDK. The prefix is how the rules know which crates it applies to.
pub const PLUGIN_PREFIX: &str = "craftkit-plugin-";

/// Whether `name` is a workspace crate in the table.
pub fn classify(name: &str) -> Option<Class> {
    TABLE.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
}

/// Whether `name` is the name of a plugin.
pub fn is_plugin(name: &str) -> bool {
    name.starts_with(PLUGIN_PREFIX)
}

fn is_ui_crate(name: &str) -> bool {
    UI_CRATES.iter().any(|pattern| match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *pattern,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

impl std::fmt::Display for DepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DepKind::Normal => write!(f, "normal"),
            DepKind::Dev => write!(f, "dev"),
            DepKind::Build => write!(f, "build"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
    /// Whether the dependency is another crate in this workspace.
    pub workspace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Krate {
    pub name: String,
    pub deps: Vec<Dep>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    /// A workspace crate nobody registered in [`TABLE`].
    Unregistered { krate: String },
    /// A dependency on the same or a higher layer.
    Upward { krate: String, dep: String, from: u8, to: u8, kind: DepKind },
    /// A plugin reaching for another plugin.
    PluginOnPlugin { krate: String, dep: String },
    /// A plugin reaching for an application.
    PluginOnApp { krate: String, dep: String },
    /// A UI toolkit below [`UI_MIN_LAYER`].
    UiBelow { krate: String, dep: String, layer: u8 },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Unregistered { krate } => write!(
                f,
                "{krate}: a workspace crate that is not in the layering table.\n\
                 \x20     Register it in {AUTHORITY} — the tree's own gate reads the same table."
            ),
            Violation::Upward { krate, dep, from, to, kind } => write!(
                f,
                "{krate} (L{from}) depends on {dep} (L{to}) [{kind}]: a crate may only depend \
                 on strictly lower layers."
            ),
            Violation::PluginOnPlugin { krate, dep } => write!(
                f,
                "{krate} depends on {dep}, which is another plugin.\n\
                 \x20     A plugin reaches the rest of the tree through the SDK; a plugin that \
                 reaches for a plugin has reached around the seam."
            ),
            Violation::PluginOnApp { krate, dep } => write!(
                f,
                "{krate} depends on {dep}, which is an application.\n\
                 \x20     A plugin is a library: it may not depend on a program."
            ),
            Violation::UiBelow { krate, dep, layer } => write!(
                f,
                "{krate} (L{layer}) depends on `{dep}`: UI toolkits are only allowed in \
                 L{UI_MIN_LAYER} and above.\n\
                 \x20     A value crate that needs a Vec2 needs its own tiny vector type, not \
                 egui's."
            ),
        }
    }
}

/// Check every rule against the model. Output is sorted, so the same mistake
/// always produces the same report.
///
/// The table is [`TABLE`], the mirror of the authority; the gate checks the
/// mirror against the authority before it calls this, so the two cannot
/// disagree while this runs.
pub fn check(crates: &[Krate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for krate in crates {
        let Some(class) = classify(&krate.name) else {
            out.push(Violation::Unregistered { krate: krate.name.clone() });
            continue;
        };
        let Some(layer) = class.layer() else { continue };
        // An application depends on everything, by design.
        if layer == APP_LAYER {
            continue;
        }

        for dep in &krate.deps {
            // A crate depending on itself is a feature trick, not a cycle.
            if dep.name == krate.name {
                continue;
            }
            // A **dev** dependency is a testing arrangement and does not ship:
            // the door's loader tests need a shared library to open, and that
            // library is a plugin. The tree's own gate does not apply the
            // layering rules to a dev dependency either.
            if dep.kind == DepKind::Dev {
                continue;
            }
            if dep.workspace {
                match classify(&dep.name) {
                    // An unregistered dependency is reported from its own entry,
                    // not twice from here.
                    None => {}
                    Some(Class::Exempt) => out.push(Violation::Upward {
                        krate: krate.name.clone(),
                        dep: dep.name.clone(),
                        from: layer,
                        to: u8::MAX,
                        kind: dep.kind,
                    }),
                    Some(other) => {
                        let to = other.layer().unwrap_or(layer);
                        // A plugin is a plugin: a plugin reaching for another
                        // plugin, or for an application, is reported as that,
                        // and not again as an upward dependency. The specific
                        // rule says more than the general one.
                        if is_plugin(&krate.name) {
                            if is_plugin(&dep.name) {
                                out.push(Violation::PluginOnPlugin {
                                    krate: krate.name.clone(),
                                    dep: dep.name.clone(),
                                });
                                continue;
                            }
                            if to == APP_LAYER {
                                out.push(Violation::PluginOnApp {
                                    krate: krate.name.clone(),
                                    dep: dep.name.clone(),
                                });
                                continue;
                            }
                        }
                        // Strictly lower. Not "lower or equal": an equal-layer
                        // dependency is how two crates quietly become one.
                        if to >= layer {
                            out.push(Violation::Upward {
                                krate: krate.name.clone(),
                                dep: dep.name.clone(),
                                from: layer,
                                to,
                                kind: dep.kind,
                            });
                        }
                    }
                }
            } else if is_ui_crate(&dep.name) && layer < UI_MIN_LAYER {
                out.push(Violation::UiBelow { krate: krate.name.clone(), dep: dep.name.clone(), layer });
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Read the authority's table out of its source.
///
/// The xtask does not depend on `craftkit-gates` — see the module's note — so
/// the table is read as text. The shape it looks for is the shape the
/// authority writes:
///
/// ```text
/// pub const LAYERS: &[(&str, u8)] = &[
///     ("craftkit-alloc", 0),
///     …
/// ];
/// ```
///
/// A table that is not written that way is a **failure**, not an empty table:
/// the gate says which line it could not read.
pub fn parse_layers(text: &str) -> Result<Vec<(String, u8)>, String> {
    let header = "pub const LAYERS";
    let start = text.find(header).ok_or_else(|| {
        format!("{AUTHORITY} has no `{header}`: the tree's own layering table could not be read")
    })?;
    let after = &text[start..];
    let open = after
        .find("= &[")
        .ok_or_else(|| format!("{AUTHORITY} does not declare `{header}` as a table of `(&str, u8)` rows"))?;
    let block = &after[open + "= &[".len()..];

    let mut rows = Vec::new();
    for (number, line) in block.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("];") {
            break;
        }
        // A comment is not a row.
        let line = line.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let Some(open) = line.find('(') else { continue };
        let Some(close) = line.find(')') else {
            return Err(format!("{AUTHORITY}: the row on line {} has no closing bracket", number + 1));
        };
        let inner = &line[open + 1..close];
        let mut cells = inner.split(',');
        let name = cells.next().unwrap_or("").trim().trim_matches('"').to_string();
        let level = cells.next().unwrap_or("").trim();
        if name.is_empty() || level.is_empty() {
            return Err(format!("{AUTHORITY}: the row on line {} is not `(\"name\", layer)`", number + 1));
        }
        let level = level.parse::<u8>().map_err(|_| {
            format!(
                "{AUTHORITY}: the row on line {} has the layer `{level}`, which is not a number",
                number + 1
            )
        })?;
        rows.push((name, level));
    }
    if rows.is_empty() {
        return Err(format!("{AUTHORITY} declares an empty layering table"));
    }
    Ok(rows)
}

/// The authority's table, read from the tree at `root`.
pub fn authority_table(root: &Path) -> Result<Vec<(String, u8)>, String> {
    let path = root.join(AUTHORITY);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("could not read the layering authority at {}: {e}", path.display()))?;
    parse_layers(&text)
}

/// Where the mirror and the authority disagree.
///
/// An empty list means they agree: the same crates, at the same layers, in
/// the same order. Every disagreement is a sentence, because a gate that says
/// "the tables differ" and not where is a gate that sends the reader to look
/// for it.
pub fn drift(root: &Path) -> Result<Vec<String>, String> {
    let authority = authority_table(root)?;
    let mirror: Vec<(String, u8)> = TABLE
        .iter()
        .filter_map(|(name, class)| class.layer().map(|layer| ((*name).to_string(), layer)))
        .collect();
    let mut problems = Vec::new();

    for (name, layer) in &authority {
        if !mirror.iter().any(|(other, _)| other == name) {
            problems.push(format!(
                "{name} is L{layer} in {AUTHORITY} and is not in the outer table \
                 (xtask/src/layers.rs)"
            ));
        }
    }
    for (name, layer) in &mirror {
        if !authority.iter().any(|(other, _)| other == name) {
            problems.push(format!(
                "{name} is L{layer} in the outer table and is not in {AUTHORITY}: the tree's own \
                 gate does not know it"
            ));
        }
    }
    for (name, layer) in &authority {
        if let Some((_, mirror_layer)) = mirror.iter().find(|(other, _)| other == name)
            && mirror_layer != layer
        {
            problems
                .push(format!("{name} is L{layer} in {AUTHORITY} and L{mirror_layer} in the outer table"));
        }
    }

    // The **order** matters too: `docs/architecture.md` renders the table in
    // this order and the `lists` gate compares the two, so a mirror in a
    // different order is a mirror that makes that gate fail for the wrong
    // reason.
    let authority_names: Vec<&str> = authority.iter().map(|(name, _)| name.as_str()).collect();
    let mirror_names: Vec<&str> = mirror.iter().map(|(name, _)| name.as_str()).collect();
    if authority_names != mirror_names {
        problems.push(format!(
            "the order of the table differs: {AUTHORITY} has {}, the outer table has {}",
            authority_names.join(", "),
            mirror_names.join(", ")
        ));
    }
    Ok(problems)
}

/// Build the model from `cargo metadata`.
///
/// `--no-deps` keeps it to workspace members, which is the whole point: the
/// rules are about *our* crates, not about how many crates `serde` has.
pub fn from_metadata(meta: &Value) -> Result<Vec<Krate>, String> {
    let packages = meta["packages"].as_array().ok_or("cargo metadata: there is no `packages` array")?;
    let members: Vec<&str> = packages.iter().filter_map(|p| p["name"].as_str()).collect();

    let mut crates = Vec::with_capacity(packages.len());
    for package in packages {
        let name = package["name"].as_str().ok_or("cargo metadata: a package with no name")?.to_string();
        let mut deps = Vec::new();
        for dep in package["dependencies"].as_array().into_iter().flatten() {
            let dep_name = dep["name"].as_str().unwrap_or_default().to_string();
            if dep_name.is_empty() {
                continue;
            }
            let kind = match dep["kind"].as_str() {
                Some("dev") => DepKind::Dev,
                Some("build") => DepKind::Build,
                _ => DepKind::Normal,
            };
            let workspace = dep["path"].is_string() || members.contains(&dep_name.as_str());
            deps.push(Dep { name: dep_name, kind, workspace });
        }
        crates.push(Krate { name, deps });
    }
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(crates)
}

/// Read `cargo metadata` from `root`.
pub fn load(root: &Path) -> Result<Vec<Krate>, String> {
    let out = command("cargo", &["metadata", "--no-deps", "--format-version", "1", "--offline"], root)
        .or_else(|_| command("cargo", &["metadata", "--no-deps", "--format-version", "1"], root))?;
    if !out.ok() {
        return Err(format!("cargo metadata failed\n\n{}", out.all()));
    }
    from_metadata(&serde_json::from_str(&out.stdout).map_err(|e| format!("cargo metadata is not JSON: {e}"))?)
}

/// The gate.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    step("layers");

    // The mirror and the authority, before anything else: a gate that checks
    // the tree against a table that has drifted from the tree's own table is
    // a gate that reports the wrong thing.
    let drift = drift(root)?;
    if !drift.is_empty() {
        let mut message = format!(
            "the layering table in xtask/src/layers.rs and the tree's own table in {AUTHORITY} \
             disagree:\n"
        );
        for problem in &drift {
            message.push_str("   • ");
            message.push_str(problem);
            message.push('\n');
        }
        message.push_str(
            "\nThe tree's own table is the authority. Fix the outer table to match it, or fix the \
             tree's table and then the outer one.",
        );
        return Err(message);
    }

    let crates = load(root)?;
    report(&check(&crates))?;

    // The table is printed either way: an agent that has to ask "what layer is
    // this crate in?" is an agent that gets it wrong.
    println!("   the layering table ({})", TABLE.len());
    for (name, class) in TABLE {
        println!("      {:<18} {}", name, class.describe());
    }
    Ok(crate::Outcome::Ran)
}

/// Print violations and turn any into an error.
pub fn report(violations: &[Violation]) -> Result<(), String> {
    if violations.is_empty() {
        println!("   ok: every crate depends only on lower layers");
        return Ok(());
    }
    let mut message = format!("{} layering violation(s):\n", violations.len());
    for violation in violations {
        message.push_str("   • ");
        message.push_str(&violation.to_string());
        message.push('\n');
    }
    message.push_str(
        "\nThe layering is what keeps the tree testable without a window. If a crate genuinely\n\
         needs to move up a layer, move it in the tree's own table — crates/craftkit-gates/src/lib.rs\n\
         — and in docs/architecture.md, and the outer table follows it.",
    );
    Err(message)
}

/// Every package the wasm gate compiles.
///
/// The library crates ✓, **and the browser app** ✓. The applications are not
/// in the list: the window is a window and the repl is a terminal, and
/// neither belongs in a browser. `craftkit-wasm` is *checked*, because it is
/// the one application whose whole job is to be a browser.
pub fn wasm_packages() -> Vec<&'static str> {
    let mut packages: Vec<&'static str> = TABLE
        .iter()
        .filter(|(_, class)| matches!(class, Class::Layer(layer) if *layer < APP_LAYER))
        .map(|(name, _)| *name)
        .collect();
    packages.push(wasm::WEB_PACKAGE);
    packages.sort_unstable();
    packages
}

/// `cargo check` every library crate — and the browser app — for
/// `wasm32-unknown-unknown`.
pub fn check_wasm(root: &Path) -> Result<(), String> {
    let packages = wasm_packages();
    let mut args: Vec<&str> = vec!["check", "--target", wasm::TARGET, "--profile", "ci"];
    args.extend(packages.iter().flat_map(|name| ["-p", name]));
    cargo(&args, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(name: &str, deps: &[(&str, DepKind, bool)]) -> Krate {
        Krate {
            name: name.to_string(),
            deps: deps
                .iter()
                .map(|(n, kind, workspace)| Dep {
                    name: (*n).to_string(),
                    kind: *kind,
                    workspace: *workspace,
                })
                .collect(),
        }
    }

    fn names(violations: &[Violation]) -> Vec<String> {
        violations.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn the_table_has_no_duplicates() {
        let mut seen = std::collections::BTreeSet::new();
        for (name, _) in TABLE {
            assert!(seen.insert(*name), "{name} appears twice in the layering table");
        }
    }

    /// The table is in **layer** order — and within a layer it is in the
    /// authority's order, which is not alphabetical: the authority groups the
    /// crates of a layer by what they are, and `docs/architecture.md` renders
    /// them in that order.
    #[test]
    fn the_table_is_in_layer_order() {
        let rows: Vec<u8> = TABLE.iter().map(|(_, class)| class.layer().unwrap_or(u8::MAX)).collect();
        let mut sorted = rows.clone();
        sorted.sort();
        assert_eq!(rows, sorted, "the table is not in layer order; `lists` would fail");
    }

    /// The mirror and the authority are the same table.
    ///
    /// This is the test that keeps them the same table: the mirror is a copy,
    /// and a copy that nobody compares with the original is a second source
    /// of truth.
    #[test]
    fn the_outer_table_is_the_trees_own_table() {
        let root = crate::root().expect("root");
        let problems = drift(&root).expect("reads the authority");
        assert!(problems.is_empty(), "the outer table and {AUTHORITY} disagree: {problems:#?}");
    }

    /// Every crate of the tree is in the table, or the table does not know it.
    #[test]
    fn every_crate_of_the_tree_is_in_the_table() {
        let root = crate::root().expect("root");
        let mut found = Vec::new();
        for folder in ["crates", "apps"] {
            let Ok(entries) = std::fs::read_dir(root.join(folder)) else { continue };
            for entry in entries.flatten() {
                let manifest = entry.path().join("Cargo.toml");
                let Ok(text) = std::fs::read_to_string(&manifest) else { continue };
                let Some(name) = text.lines().find_map(|line| {
                    let line = line.trim();
                    line.strip_prefix("name")
                        .and_then(|rest| rest.trim_start().strip_prefix('='))
                        .map(|value| value.trim().trim_matches('"').to_string())
                }) else {
                    continue;
                };
                found.push(name);
            }
        }
        assert!(!found.is_empty(), "no crates were found under crates/ and apps/");
        for name in &found {
            assert!(classify(name).is_some(), "{name} is not in the layering table");
        }
    }

    #[test]
    fn the_authority_is_read_out_of_its_source() {
        let root = crate::root().expect("root");
        let table = authority_table(&root).expect("reads the authority");
        assert!(table.len() > 40, "the tree has more crates than that: {}", table.len());
        let alloc = table.iter().find(|(name, _)| name == "craftkit-alloc").expect("craftkit-alloc");
        assert_eq!(alloc.1, 0, "{alloc:?}");
        let window = table.iter().find(|(name, _)| name == "craftkit-window").expect("craftkit-window");
        assert_eq!(window.1, APP_LAYER, "{window:?}");
    }

    #[test]
    fn a_table_that_is_not_written_as_a_table_is_an_error_and_not_an_empty_one() {
        assert!(parse_layers("pub fn nothing() {}").is_err());
        assert!(parse_layers("pub const LAYERS: &[(&str, u8)] = &[];").is_err());
        // A row that is not `("name", layer)` is a failure with a line.
        let broken = "pub const LAYERS: &[(&str, u8)] = &[\n    (\"craftkit-alloc\",),\n];";
        let error = parse_layers(broken).expect_err("a broken row");
        assert!(error.contains("line 2"), "{error}");
    }

    #[test]
    fn a_downward_dependency_is_fine() {
        let crates = vec![krate("craftkit-plugin-hello", &[("craftkit-sdk", DepKind::Normal, true)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn an_upward_dependency_is_a_violation() {
        // The mistake this gate exists for: a library reaching up into a
        // battery, which would make the library depend on the thing that
        // draws it.
        let crates = vec![krate("craftkit-raster", &[("craftkit-plugin-hello", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::Upward { .. }), "{violations:?}");
        assert!(violations[0].to_string().contains("strictly lower layers"), "{violations:?}");
    }

    #[test]
    fn a_same_layer_dependency_is_a_violation_too() {
        // Two libraries at the same layer using each other means they have
        // become one crate wearing two names.
        let crates = vec![krate("craftkit-mcp", &[("craftkit-control", DepKind::Normal, true)])];
        assert_eq!(check(&crates).len(), 1, "an equal layer must not be allowed");
    }

    /// A **dev** dependency is a testing arrangement and does not ship.
    #[test]
    fn a_dev_dependency_does_not_count() {
        // The tree's own fixtures: the door's loader tests open a plugin, and
        // the plugin is at a higher layer than the door.
        let crates = vec![krate("craftkit-ffi", &[("craftkit-plugin-shared", DepKind::Dev, true)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn a_plugin_may_not_depend_on_another_plugin() {
        let crates = vec![krate("craftkit-plugin-art", &[("craftkit-plugin-hello", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::PluginOnPlugin { .. }), "{violations:?}");
    }

    #[test]
    fn a_plugin_may_not_depend_on_an_application() {
        let crates = vec![krate("craftkit-plugin-art", &[("craftkit-window", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::PluginOnApp { .. }), "{violations:?}");
    }

    #[test]
    fn an_application_depends_on_everything() {
        // The window is an application: it names the studio, the rasteriser and
        // the toolkit, and that is its whole job.
        let crates = vec![krate(
            "craftkit-window",
            &[
                ("craftkit-plugin-art", DepKind::Normal, true),
                ("craftkit-raster", DepKind::Normal, true),
                ("egui", DepKind::Normal, false),
                ("eframe", DepKind::Normal, false),
            ],
        )];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn a_library_may_not_name_a_ui_toolkit() {
        let crates = vec![krate("craftkit-raster", &[("egui", DepKind::Normal, false)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].to_string().contains("UI toolkits"), "{violations:?}");
    }

    #[test]
    fn a_library_may_not_name_a_window() {
        let crates = vec![krate("craftkit-raster", &[("eframe", DepKind::Normal, false)])];
        assert_eq!(check(&crates).len(), 1, "{:?}", names(&check(&crates)));
    }

    #[test]
    fn egui_is_a_ui_crate_too() {
        assert!(is_ui_crate("eframe"));
        assert!(is_ui_crate("egui"));
        assert!(is_ui_crate("egui_glow"));
        assert!(is_ui_crate("winit"));
        // And the things that are not: a PNG encoder is not a toolkit.
        assert!(!is_ui_crate("serde"));
        assert!(!is_ui_crate("image"));
        assert!(!is_ui_crate("tokio"));
    }

    #[test]
    fn a_crate_the_table_does_not_know_is_a_finding() {
        let crates = vec![krate("craftkit-thing", &[])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::Unregistered { .. }), "{violations:?}");
    }

    #[test]
    fn the_wasm_check_covers_the_libraries_and_the_browser_app() {
        let checked = wasm_packages();
        assert!(checked.contains(&wasm::WEB_PACKAGE), "the browser app is not checked: {checked:?}");
        // The applications are not libraries: the window is a window.
        assert!(!checked.contains(&"craftkit-window"), "the window is checked for the browser");
    }
}
