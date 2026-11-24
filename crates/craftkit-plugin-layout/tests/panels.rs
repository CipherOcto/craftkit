//! The **panel** of the layout battery: what it shows.
//!
//! A panel is a battery's own window of commands: it names the commands
//! it shows, in order, and a front end shows them together. A panel that
//! is named by no test is a panel that nothing proves.

use craftkit_sdk::Ecosystem;

/// The battery's **anchor**: the one name that keeps its commands linked
/// into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The panel is there, and it is called **Layout**.
#[test]
fn the_panel_is_there() {
    let ecosystem = ecosystem();
    let panel = ecosystem.panels.get("panel.layout").expect("the panel is there");
    assert_eq!(panel.label, "Layout", "the panel is called Layout");
    assert_eq!(panel.layer, "layout", "and the layout layer owns it");
}

/// The panel shows the commands it says, and **they are all there**.
#[test]
fn the_panel_shows_the_commands_it_says() {
    let ecosystem = ecosystem();
    let panel = ecosystem.panels.get("panel.layout").expect("the panel is there");
    assert!(!panel.commands.is_empty(), "the panel shows something");
    for command in panel.commands {
        assert!(
            ecosystem.commands.get(command).is_some(),
            "the panel shows `{command}`, which is not linked in"
        );
    }
}

/// The panel shows the commands of **its own layer**, and nothing else.
///
/// A panel that showed another battery's command would be a battery
/// reaching into another battery, and that is a rule and not a hope.
#[test]
fn the_panel_shows_only_its_own_layers_commands() {
    let ecosystem = ecosystem();
    let panel = ecosystem.panels.get("panel.layout").expect("the panel is there");
    for command in panel.commands {
        let spec = ecosystem.commands.get(command).expect("the command is there");
        assert_eq!(
            spec.layer, panel.layer,
            "the panel shows `{command}`, which is of the `{}` layer",
            spec.layer
        );
    }
}
