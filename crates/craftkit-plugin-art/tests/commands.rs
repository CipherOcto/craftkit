//! The **commands of a macro** are the commands of the battery.
//!
//! The art battery declares twelve of its commands through a macro: the
//! macro holds the declaration and each call holds the arguments. A
//! macro that takes a label and drops it gives every one of them the
//! same button — and a button that says "Add" is not a button that says
//! what it adds.

use craftkit_sdk::Ecosystem;

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_art::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the art battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The twelve commands the macros declare, and the label each one has.
#[test]
fn the_commands_of_a_macro_carry_their_own_label() {
    let ecosystem = ecosystem();
    let commands = ecosystem.commands.all();
    for (id, label) in [
        ("art.track.video.add", "Video"),
        ("art.track.text.add", "Text"),
        ("art.track.audio.add", "Audio"),
        ("art.track.graphic.add", "Graphic"),
        ("art.track.effect.add", "Effect"),
        ("art.element.video.add", "Video"),
        ("art.element.image.add", "Image"),
        ("art.element.audio.add", "Audio"),
        ("art.element.text.add", "Text"),
        ("art.element.sticker.add", "Sticker"),
        ("art.element.graphic.add", "Graphic"),
        ("art.element.effect.add", "Effect"),
    ] {
        let command = commands
            .iter()
            .find(|command| command.id == id)
            .unwrap_or_else(|| panic!("the battery has no command called `{id}`"));
        assert_eq!(command.label, label, "the button of `{id}` says what it is");
    }
}

/// The battery carries **the commands of its macros** and the ones
/// written out beside them.
///
/// The exact count moves with every command the battery gains, so the
/// count is not asserted here: the claim of §1102 is that the twelve
/// commands of the macros are there, and that is what is checked.
#[test]
fn the_battery_carries_the_commands_of_its_macros() {
    let ecosystem = ecosystem();
    let commands: Vec<&craftkit_sdk::command::CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.id.starts_with("art.")).collect();
    assert!(
        commands.len() >= 23,
        "the battery carries the twelve commands of its macros and the eleven written out"
    );
}
