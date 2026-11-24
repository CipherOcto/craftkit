//! The **tool** of the geometry battery: the gesture that runs a command.
//!
//! A tool is a gesture that runs a command: it has no parameters of its
//! own, it has an icon, and it says which command it runs. A tool that is
//! named by no test is a tool that nothing proves.

use craftkit_sdk::Ecosystem;

/// The battery's **anchor**: the one name that keeps its commands linked
/// into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_shapes::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The tool is there, and it is called **Rectangle**.
#[test]
fn the_tool_is_there() {
    let ecosystem = ecosystem();
    let tool = ecosystem.tools.get("tool.rect").expect("the tool is there");
    assert_eq!(tool.label, "Rectangle", "the tool is called Rectangle");
    assert_eq!(tool.layer, "geometry", "and the geometry layer owns it");
    assert_eq!(tool.command, "geometry.rectangle", "and it runs that command");
}

/// The tool runs a command that **is there**.
#[test]
fn the_tool_runs_a_command_that_is_linked_in() {
    let ecosystem = ecosystem();
    let tool = ecosystem.tools.get("tool.rect").expect("the tool is there");
    assert!(
        ecosystem.commands.get(tool.command).is_some(),
        "the tool runs `{}`, which is not linked in",
        tool.command
    );
}

/// The tool runs a command of **its own layer**, and nothing else.
///
/// A tool that ran another battery's command would be a battery reaching
/// into another battery, and that is a rule and not a hope.
#[test]
fn the_tool_runs_a_command_of_its_own_layer() {
    let ecosystem = ecosystem();
    let tool = ecosystem.tools.get("tool.rect").expect("the tool is there");
    let spec = ecosystem.commands.get(tool.command).expect("the command is there");
    assert_eq!(
        spec.layer, tool.layer,
        "the tool runs `{}`, which is of the `{}` layer",
        tool.command, spec.layer
    );
}
