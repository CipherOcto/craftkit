//! The dependency layering rules.
//!
//! The rule, in one sentence: **a crate may only depend on crates in strictly
//! lower layers, and nothing below [`UI_MIN_LAYER`] may touch a UI toolkit.**
//!
//! Why this is worth a gate at all. The value crates (core, model, eval,
//! kernel) are where the interesting rules live, and they are also where a UI
//! dependency creeps in — somebody wants a `Vec2` for a layout, reaches for
//! `egui`, and the crate stops being testable without a window. Nothing about
//! that is visible in review, and nothing about it fails until much later, on
//! somebody else's machine. A table in a file, checked by a program, catches it
//! on the machine that made the mistake.
//!
//! The model here is deliberately independent of `cargo metadata`, so the rules
//! can be unit-tested without running cargo. [`from_metadata`] is the only part
//! that knows where the data comes from.

use std::path::Path;

use serde_json::Value;

use crate::{cargo, command, step, wasm};

/// Where a workspace crate sits.
///
/// `Exempt` exists for the binaries and the build tool. The apps *do* depend on
/// everything, and layering rules that exempt them are not wrong, they are just
/// noise: every real layering mistake has been made in a library crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A layered library crate at this layer.
    Layer(u8),
    /// Binaries and build tooling.
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

/// The layering table: every workspace library crate and its layer.
///
/// **This table is the layering rules.** `docs/architecture.md` renders the same
/// table, and `cargo xtask lists` fails if the two disagree — a gate that
/// disagrees with its own documentation is worse than no gate, because it
/// teaches the wrong thing.
///
/// Names are the full package names, sorted by layer and then by name, so the
/// table itself is reviewable as a diff.
pub const TABLE: &[(&str, Class)] = &[
    // L0 — the generic half: readers, values, file plumbing, limits. No
    // workspace dependency at all, and nothing in it that knows what a
    // document is. §276.
    ("craftkit-foundation", Class::Layer(0)),
    // L1 — craftkit's own core: the exact time base and the identifiers.
    ("craftkit-core", Class::Layer(1)),
    // L2 — the numeric kernel, behind the panic guard.
    ("craftkit-kernel", Class::Layer(2)),
    // L2 — the document: nodes, properties, keyframes.
    ("craftkit-model", Class::Layer(2)),
    // L3 — two ways to get a value out of a document: evaluate it, or load it.
    ("craftkit-eval", Class::Layer(3)),
    ("craftkit-io", Class::Layer(3)),
    // L4 — the command registry and the session that runs it.
    ("craftkit-engine", Class::Layer(4)),
    // L5 — the two ways an agent reaches the engine.
    ("craftkit-control", Class::Layer(5)),
    ("craftkit-mcp", Class::Layer(5)),
    // L6 — the shell. The only library crate that may name egui.
    ("craftkit-ui", Class::Layer(6)),
    // The binaries and the build tool. Exempt.
    ("craftkit", Class::Exempt),
    // The Android package: the same application, packaged for the phone.
    // §236.
    ("craftkit-android", Class::Exempt),
    ("craftkit-cli", Class::Exempt),
    ("craftkit-web", Class::Exempt),
    ("xtask", Class::Exempt),
];

/// External crates that make a crate a UI crate.
///
/// Entries ending in `*` are prefixes. `image` is deliberately **not** here: it
/// is a PNG encoder, not a toolkit, and listing it would push the rule out of
/// the range where it is meaningful.
pub const UI_CRATES: &[&str] = &["egui*", "eframe", "egui-wgpu", "wgpu", "winit", "rfd", "accesskit*"];

/// The first layer allowed to name a UI crate.
pub const UI_MIN_LAYER: u8 = 6;

/// The UI crates the shell itself may name.
///
/// `egui` is the toolkit the shell draws with. Everything else in
/// [`UI_CRATES`] is a window or a renderer, and the whole point of the shell
/// being a crate of its own is that the same panels can be drawn in a window,
/// headlessly, or in a browser — which they cannot be if the shell owns the
/// window. §131.
pub const UI_CRATE_ALLOWED: &[&str] = &["egui"];

/// Whether `name` is a workspace crate in the table.
pub fn classify(name: &str) -> Option<Class> {
    TABLE.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
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
    /// A UI toolkit below [`UI_MIN_LAYER`].
    UiBelow { krate: String, dep: String, layer: u8 },
    /// A window or a renderer in the shell crate itself. §131.
    WindowingInTheUiCrate { krate: String, dep: String },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Unregistered { krate } => write!(
                f,
                "{krate}: a workspace crate that is not in the layering table.\n\
                 \x20     Register it in xtask/src/layers.rs TABLE and in the table in \
                 docs/architecture.md — `cargo xtask lists` checks they agree."
            ),
            Violation::Upward { krate, dep, from, to, kind } => write!(
                f,
                "{krate} (L{from}) depends on {dep} (L{to}) [{kind}]: a crate may only depend \
                 on strictly lower layers."
            ),
            Violation::UiBelow { krate, dep, layer } => write!(
                f,
                "{krate} (L{layer}) depends on `{dep}`: UI toolkits are only allowed in \
                 L{UI_MIN_LAYER} and above.\n\
                 \x20     A value crate that needs a Vec2 needs its own tiny vector type, not \
                 egui's."
            ),
            Violation::WindowingInTheUiCrate { krate, dep } => write!(
                f,
                "{krate} depends on `{dep}`: the shell owns its panels, not its window.\n\
                 \x20     The shell may depend on {} and nothing else — no windowing, no \
                 renderer — which is what lets the same panels run in a window, headlessly, \
                 and in a browser.",
                UI_CRATE_ALLOWED.join(", ")
            ),
        }
    }
}

/// Check every rule against the model. Output is sorted, so the same mistake
/// always produces the same report.
pub fn check(crates: &[Krate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for krate in crates {
        let Some(class) = classify(&krate.name) else {
            out.push(Violation::Unregistered { krate: krate.name.clone() });
            continue;
        };
        let Some(layer) = class.layer() else { continue };

        for dep in &krate.deps {
            // A crate depending on itself is a feature trick, not a cycle.
            if dep.name == krate.name {
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
            } else if is_ui_crate(&dep.name) {
                if layer < UI_MIN_LAYER {
                    out.push(Violation::UiBelow { krate: krate.name.clone(), dep: dep.name.clone(), layer });
                } else if layer == UI_MIN_LAYER && !UI_CRATE_ALLOWED.contains(&dep.name.as_str()) {
                    out.push(Violation::WindowingInTheUiCrate {
                        krate: krate.name.clone(),
                        dep: dep.name.clone(),
                    });
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
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
        "\nThe layering is what keeps the engine testable without a window. If a crate\n\
         genuinely needs to move up a layer, move it in xtask/src/layers.rs and in\n\
         docs/architecture.md together — `cargo xtask lists` checks they agree.",
    );
    Err(message)
}

/// Every package the wasm gate compiles.
///
/// The library crates ✓, **and the browser app** ✓. §291: the gate's own
/// message says the web app builds for the browser ✓, and the app was
/// filtered out with the other apps ✓ — so the message was a claim about a
/// crate nothing compiled ✓. The browser app is the one app that belongs
/// here ✓: its whole job is to be a browser ✓.
///
/// The other apps are exempt on purpose ✓: the desktop app is a window ✓ and
/// the CLI rasterises ✓, and neither has any business in a browser ✓.
pub fn wasm_packages() -> Vec<&'static str> {
    let mut packages: Vec<&'static str> =
        TABLE.iter().filter(|(_, class)| matches!(class, Class::Layer(_))).map(|(name, _)| *name).collect();
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
    fn the_table_covers_every_layer_from_zero_to_six() {
        for layer in 0..=UI_MIN_LAYER {
            assert!(
                TABLE.iter().any(|(_, c)| *c == Class::Layer(layer)),
                "no crate is registered at L{layer}; a gap in the table is an unchecked layer"
            );
        }
    }

    #[test]
    fn the_table_has_no_duplicates() {
        let mut seen = std::collections::BTreeSet::new();
        for (name, _) in TABLE {
            assert!(seen.insert(*name), "{name} appears twice in the layering table");
        }
    }

    #[test]
    fn the_table_is_sorted_by_layer_then_name() {
        let rows: Vec<(u8, &str)> =
            TABLE.iter().map(|(name, class)| (class.layer().unwrap_or(u8::MAX), *name)).collect();
        let mut sorted = rows.clone();
        sorted.sort();
        assert_eq!(rows, sorted, "the table is not in layer order; `lists` would fail");
    }

    #[test]
    fn a_downward_dependency_is_fine() {
        let crates = vec![krate("craftkit-engine", &[("craftkit-model", DepKind::Normal, true)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    /// §131: the shell may depend on `egui` and nothing else.
    ///
    /// `AGENTS.md` says the shell "depends on `egui` and nothing else — no
    /// windowing, no renderer — which is what lets the same panels run in a
    /// window, headlessly, and in a browser". The rule had nothing behind it:
    /// the layer gate refused a UI toolkit *below* L6, and the shell *is* L6,
    /// so a window in the shell was not a violation of anything.
    #[test]
    fn the_shell_may_not_own_a_window() {
        // egui is the toolkit the shell draws with.
        let crates = vec![krate("craftkit-ui", &[("egui", DepKind::Normal, false)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));

        // A window is not.
        let crates = vec![krate("craftkit-ui", &[("eframe", DepKind::Normal, false)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::WindowingInTheUiCrate { .. }), "{violations:?}");

        // And neither is a renderer.
        let crates = vec![krate("craftkit-ui", &[("wgpu", DepKind::Normal, false)])];
        assert_eq!(check(&crates).len(), 1, "a renderer in the shell");

        // The apps are exempt: they are the ones that own the window.
        let crates = vec![krate("craftkit", &[("eframe", DepKind::Normal, false)])];
        assert!(check(&crates).is_empty(), "the app owns the window");
    }

    #[test]
    fn an_upward_dependency_is_a_violation() {
        // The mistake this gate exists for: eval reaching up into the engine,
        // which would make the evaluator depend on the thing that drives it.
        let crates = vec![krate("craftkit-eval", &[("craftkit-engine", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::Upward { .. }), "{violations:?}");
        assert!(violations[0].to_string().contains("strictly lower layers"), "{violations:?}");
    }

    #[test]
    fn a_same_layer_dependency_is_a_violation_too() {
        // eval and io are both L3. Either of them using the other means they
        // have become one crate wearing two names.
        let crates = vec![krate("craftkit-io", &[("craftkit-eval", DepKind::Normal, true)])];
        assert_eq!(check(&crates).len(), 1, "an equal layer must not be allowed");
    }

    #[test]
    fn a_value_crate_may_not_name_egui() {
        let crates = vec![krate("craftkit-core", &[("egui", DepKind::Normal, false)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(violations[0].to_string().contains("UI toolkits"), "{violations:?}");
    }

    #[test]
    fn eframe_is_a_ui_crate_too() {
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
    fn the_ui_crate_may_name_egui() {
        let crates = vec![krate("craftkit-ui", &[("egui", DepKind::Normal, false)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn a_library_crate_may_not_depend_on_an_app() {
        let crates = vec![krate("craftkit-core", &[("craftkit-cli", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
    }

    #[test]
    fn a_dev_dependency_still_has_to_point_downward() {
        // It is tempting to let a dev-dependency be exempt. It is not: a test
        // that can only run with a higher crate's help is testing the wrong side
        // of the boundary.
        let crates = vec![krate("craftkit-core", &[("craftkit-engine", DepKind::Dev, true)])];
        assert_eq!(check(&crates).len(), 1, "{:?}", names(&check(&crates)));
    }

    #[test]
    fn a_crate_may_depend_on_itself() {
        let crates = vec![krate("craftkit-engine", &[("craftkit-engine", DepKind::Dev, true)])];
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn an_unregistered_workspace_crate_is_reported_once() {
        let crates = vec![krate("craftkit-newthing", &[("craftkit-core", DepKind::Normal, true)])];
        let violations = check(&crates);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert!(matches!(violations[0], Violation::Unregistered { .. }), "{violations:?}");
    }

    #[test]
    fn the_apps_are_exempt() {
        for app in ["craftkit", "craftkit-cli", "craftkit-web", "xtask"] {
            assert_eq!(classify(app), Some(Class::Exempt), "{app} should be exempt");
        }
    }

    #[test]
    fn violations_are_reported_in_a_stable_order() {
        let crates = vec![
            krate(
                "craftkit-io",
                &[("craftkit-eval", DepKind::Normal, true), ("egui", DepKind::Normal, false)],
            ),
            krate("craftkit-core", &[("egui", DepKind::Normal, false)]),
        ];
        let first = names(&check(&crates));
        let mut reversed = crates.clone();
        reversed.reverse();
        assert_eq!(first, names(&check(&reversed)), "the same mistake must always print the same way");
    }

    #[test]
    fn metadata_becomes_a_model() {
        let meta: Value = serde_json::from_str(
            r#"{"packages":[
                {"name":"craftkit-ui","dependencies":[
                    {"name":"egui","path":null},
                    {"name":"craftkit-engine","path":"crates/engine"},
                    {"name":"tempfile","path":null,"kind":"dev"}
                ]},
                {"name":"xtask","dependencies":[]}
            ]}"#,
        )
        .expect("json");
        let crates = from_metadata(&meta).expect("model");
        assert_eq!(crates.len(), 2);
        let ui = crates.iter().find(|c| c.name == "craftkit-ui").expect("ui");
        let egui = ui.deps.iter().find(|d| d.name == "egui").expect("egui dep");
        assert!(!egui.workspace, "egui is not ours");
        let engine = ui.deps.iter().find(|d| d.name == "craftkit-engine").expect("engine dep");
        assert!(engine.workspace, "a path dependency is a workspace crate");
        let tempfile = ui.deps.iter().find(|d| d.name == "tempfile").expect("tempfile dep");
        assert_eq!(tempfile.kind, DepKind::Dev);
        assert!(check(&crates).is_empty(), "{:?}", names(&check(&crates)));
    }

    #[test]
    fn metadata_without_packages_is_an_error_not_a_panic() {
        let meta: Value = serde_json::from_str(r#"{"nope":true}"#).expect("json");
        assert!(from_metadata(&meta).is_err());
    }

    #[test]
    fn a_package_with_no_name_is_an_error_not_a_panic() {
        let meta: Value = serde_json::from_str(r#"{"packages":[{"dependencies":[]}]}"#).expect("json");
        assert!(from_metadata(&meta).is_err());
    }
}
