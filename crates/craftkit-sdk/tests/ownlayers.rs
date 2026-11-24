//! The **own layers**: a panel shows its own layer's commands.
//!
//! The house rule is that a plugin may not depend on another plugin.
//! The tree's gate says so about a **Cargo manifest**: a battery's
//! manifest may not name another battery. But a panel is a *list of
//! command names*, and a panel that named another battery's command
//! would be a battery reaching into another battery without a word in
//! any manifest.
//!
//! These tests say what the ecosystem does with such a panel, and what
//! it does with a panel that stays inside its own layer.

use craftkit_sdk::command::CommandRun;
use craftkit_sdk::{CommandSpec, Ecosystem, LayerSpec, Outcome, PanelSpec, Registry, ToolSpec};

/// A layer.
fn layer(id: &'static str, requires: &'static [&'static str]) -> LayerSpec {
    LayerSpec { id, name: "A layer", requires }
}

/// A command of a layer.
fn command(id: &'static str, layer: &'static str) -> CommandSpec {
    CommandSpec {
        id,
        label: "Add",
        area: "core",
        capability: "command",
        layer,
        icon: "plus",
        doc: "",
        shortcut: None,
        menu: &[],
        params: &[],
        mutates: true,
        undoable: true,
        journal: true,
        example: false,
        enabled: |_| true,
        run: CommandRun::Static(|_session, _params| Ok(Outcome::changed("nothing"))),
    }
}

/// An ecosystem with two layers, a command in each, and a panel that
/// shows one of them.
fn an_ecosystem(panel_commands: &'static [&'static str], panel_layer: &'static str) -> Ecosystem {
    let mut ecosystem = Ecosystem::new();
    ecosystem.layers = Registry::new();
    ecosystem.layers.register(layer("core", &[])).expect("the core layer is registered");
    ecosystem.layers.register(layer("art", &["core"])).expect("the art layer is registered");
    ecosystem.layers.register(layer("cad", &["core"])).expect("the cad layer is registered");
    ecosystem.commands = Registry::new();
    ecosystem.commands.register(command("art.project.add", "art")).expect("the art command is registered");
    ecosystem.commands.register(command("cad.drawing.add", "cad")).expect("the cad command is registered");
    ecosystem.panels = Registry::new();
    ecosystem
        .panels
        .register(PanelSpec { id: "panel.art", label: "Art", layer: panel_layer, commands: panel_commands })
        .expect("the panel is registered");
    ecosystem
}

/// A panel that shows its own layer's command is **accepted**.
#[test]
fn a_panel_that_shows_its_own_layer_is_accepted() {
    let ecosystem = an_ecosystem(&["art.project.add"], "art");
    ecosystem.validate().expect("the ecosystem is fine");
}

/// A panel that shows another layer's command is **refused**.
///
/// The rule that a plugin may not depend on another plugin is a rule
/// about data as well as about Cargo.
#[test]
fn a_panel_that_shows_another_layers_command_is_refused() {
    let ecosystem = an_ecosystem(&["cad.drawing.add"], "art");
    let error = ecosystem.validate().expect_err("the ecosystem is refused");
    let message = error.to_string();
    assert!(message.contains("panel.art"), "{message}");
    assert!(message.contains("cad.drawing.add"), "{message}");
}

/// A panel may show the commands of a layer it **requires**.
#[test]
fn a_panel_may_show_the_commands_of_a_layer_it_requires() {
    let mut ecosystem = an_ecosystem(&[], "art");
    ecosystem.commands = Registry::new();
    ecosystem.commands.register(command("core.layer.add", "core")).expect("the core command is registered");
    ecosystem.panels = Registry::new();
    ecosystem
        .panels
        .register(PanelSpec { id: "panel.art", label: "Art", layer: "art", commands: &["core.layer.add"] })
        .expect("the panel is registered");
    ecosystem.validate().expect("the ecosystem is fine");
}

/// A tool that runs another layer's command is **refused**.
#[test]
fn a_tool_that_runs_another_layers_command_is_refused() {
    let mut ecosystem = an_ecosystem(&[], "art");
    ecosystem.tools = Registry::new();
    ecosystem
        .tools
        .register(ToolSpec {
            id: "tool.art",
            label: "Add",
            layer: "art",
            icon: "plus",
            shortcut: None,
            command: "cad.drawing.add",
        })
        .expect("the tool is registered");
    let error = ecosystem.validate().expect_err("the ecosystem is refused");
    let message = error.to_string();
    assert!(message.contains("tool.art"), "{message}");
    assert!(message.contains("cad.drawing.add"), "{message}");
}

/// And a tool that runs its own layer's command is **accepted**.
#[test]
fn a_tool_that_runs_its_own_layers_command_is_accepted() {
    let mut ecosystem = an_ecosystem(&[], "art");
    ecosystem.tools = Registry::new();
    ecosystem
        .tools
        .register(ToolSpec {
            id: "tool.art",
            label: "Add",
            layer: "art",
            icon: "plus",
            shortcut: None,
            command: "art.project.add",
        })
        .expect("the tool is registered");
    ecosystem.validate().expect("the ecosystem is fine");
}
