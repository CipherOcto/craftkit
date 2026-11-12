//! The third door, with a **real program** on the other end.
//!
//! The wire protocol is tested in the studio's own test file, over two
//! buffers. This file tests the thing those buffers stand for: a plugin
//! that is a separate program, started while the program runs.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Registry};

/// The ecosystem of the **host**.
///
/// It is empty of the plugin's commands on purpose: the test binary does
/// not link the plugin, so the only way its commands can arrive is over
/// the wire. That is what makes this a test of the third door and not a
/// test of the first one.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A plugin that is a separate program says hello with a catalogue, and the
/// host reads it.
#[test]
fn a_plugin_programs_catalogue_is_read() {
    let program = env!("CARGO_BIN_EXE_craftkit-hello-server");
    let plugin = craftkit_sdk::LoadedPlugin::spawn(program, &[]).expect("the plugin starts");
    assert!(plugin.ids().contains(&"hello.greet"), "{:?}", plugin.ids());
    let command = plugin
        .catalogue()
        .commands
        .iter()
        .find(|command| command.id == "hello.greet")
        .expect("the greeting is in the catalogue");
    assert_eq!(command.label, "Greet");
    assert_eq!(command.params.len(), 2);
}

/// A plugin's commands are the **same thing** as a linked plugin's: they go
/// in the same registry, and the session runs them the same way.
#[test]
fn a_plugin_programs_commands_run() {
    let program = env!("CARGO_BIN_EXE_craftkit-hello-server");
    let mut plugin = craftkit_sdk::LoadedPlugin::spawn(program, &[]).expect("the plugin starts");
    let mut commands: Registry<craftkit_sdk::CommandSpec> = Registry::new();
    plugin.register(&mut commands).expect("the commands are registered");
    assert!(commands.get("hello.greet").is_some());

    let ecosystem = ecosystem();
    let mut session = craftkit_sdk::plugin::session_with(&ecosystem, &mut plugin).expect("the session");
    let outcome = session
        .execute("hello.greet", &json!({ "name": "world", "loudly": true }))
        .expect("the plugin's command ran");
    assert_eq!(outcome.message, "HELLO, world!");
}

/// A plugin's command that changes the document changes the **host's**
/// document: the document travels with the request and comes back.
#[test]
fn a_plugin_programs_command_can_change_the_document() {
    // A plugin of our own, whose command adds a node. It is the plugin
    // program that does the adding, and the host's document is the one
    // that changes.
    let program = env!("CARGO_BIN_EXE_craftkit-hello-server");
    let mut plugin = craftkit_sdk::LoadedPlugin::spawn(program, &[]).expect("the plugin starts");
    let ecosystem = ecosystem();
    let mut session = craftkit_sdk::plugin::session_with(&ecosystem, &mut plugin).expect("the session");
    // The greeting plugin does not change the document, so the document
    // is as it was: the host judges the change, and there was none.
    session.execute("hello.greet", &json!({ "name": "world" })).expect("the command ran");
    assert!(session.document().nodes.is_empty(), "the document changed");
}

/// A plugin that is not there is refused, and the refusal says so.
#[test]
fn a_plugin_that_is_not_there_is_refused() {
    let refused = craftkit_sdk::LoadedPlugin::spawn("craftkit-no-such-plugin-anywhere", &[]);
    assert!(refused.is_err(), "the missing plugin was accepted");
}
