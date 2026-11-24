//! The **a2ui battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! The a2ui battery is a battery like any other: it adds commands and
//! kinds to an application, and the application links it in **without
//! naming it**. A change to its ids, its labels, its layers, its icons,
//! its menu paths, its parameters or its flags is a change to the tree,
//! and this test is what says so.
//!
//! # Why it is here and not in the battery
//!
//! A battery's own test binary sees **no commands**: the plugin's items
//! are behind its anchor, and the anchor keeps them for a program that
//! links the plugin. The studio links the batteries, so the studio's
//! ecosystem is where the registry of a battery can be read.

#![cfg(feature = "a2ui")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the a2ui battery, as the registry holds them.
///
/// **The snapshot**: taken from the registry.
const EXPECTED: &[&str] = &[
    r##"a2ui.component.invoke | Invoke an a2ui component | a2ui | command | core |  | Run a component's action. | - | a2ui | surface_id(text)! help="The surface the component is on.",component_id(text)! help="The component's name.",action(json)! help="The action: an event, or a function call." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"a2ui.data.set | Set an a2ui data value | a2ui | command | core |  | Write a value into the data model, and mirror it into craftkit. | - | a2ui | surface_id(text)! help="The surface the value belongs to.",path(text)! help="Where the value goes. The empty path is the whole model.",value(json)! help="The value." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"a2ui.surface.apply | Apply an a2ui component tree | a2ui | command | core |  | Draw a component tree onto a surface. | - | a2ui | surface_id(text)! help="The surface the tree is drawn onto.",components(json)! help="The component tree." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"a2ui.surface.create | Create an a2ui surface | a2ui | command | core |  | Make a surface, and the node it draws into. | - | a2ui | surface_id(text)! help="The surface's name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"a2ui.surface.delete | Delete an a2ui surface | a2ui | command | core |  | Remove a surface, and everything it holds. | - | a2ui | surface_id(text)! help="The surface to remove." | mutates=true | undoable=true | journal=true | example=false"##,
];

/// The commands of the a2ui battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "a2ui").cloned().collect();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// One command, written down.
///
/// **Everything the registry carries that is data**: the id, the label, the
/// area, the capability, the layer, the icon, the documentation, the
/// chord, the menu path, the parameters — with each parameter's **help**,
/// its other spelling, its kind's own name and its unit — and the flags.
///
/// The `enabled` predicate is a **function**, and a function is not
/// something a snapshot can write down; the snapshot says everything else.
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            let mut text = format!(
                "{}({}){}",
                param.name,
                param.kind.json_name(),
                if param.required { "!" } else { "?" }
            );
            if !param.help.is_empty() {
                text.push_str(&format!(" help=\"{}\"", quoted(param.help)));
            }
            if let Some(alternative) = param.alternative {
                text.push_str(&format!(" or=\"{}\"", quoted(alternative)));
            }
            if let Some(type_name) = param.type_name {
                text.push_str(&format!(" type=\"{}\"", quoted(type_name)));
            }
            if let Some(unit) = param.unit {
                text.push_str(&format!(" unit=\"{}\"", quoted(unit)));
            }
            text
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={} | journal={} | example={}",
        command.id,
        command.label,
        command.area,
        command.capability,
        command.layer,
        command.icon,
        command.doc,
        command.shortcut.unwrap_or("-"),
        command.menu.join("/"),
        params.join(","),
        command.mutates,
        command.undoable,
        command.journal,
        command.example
    )
}

/// A text, with its backslashes and its quotes escaped.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r##\"{line}\"##,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
