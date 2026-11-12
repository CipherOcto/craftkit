//! The **host** and the **instance**: the other half of the contract.
//!
//! A plugin is not only a file: it is an **instance**, made by the
//! factory, given a **host**, and then lived through — initialised,
//! activated, deactivated, destroyed. These tests drive that life
//! against the reference plugin, which is a real shared library.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use std::path::PathBuf;

use craftkit_clap::ClapPlugin;
#[cfg(unix)]
use craftkit_clap::{ClapBlock, ClapTransport};
#[cfg(unix)]
use craftkit_clap_abi::ClapProcessStatus;

/// The name of the shared library cargo writes for a fixture.
///
/// Cargo names a shared library differently on each platform, and the tree's
/// tests open a **real** one, so the name has to be right on all three.
fn fixture_name(crate_name: &str) -> String {
    let stem = crate_name.replace('-', "_");
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

/// The shared library the tests open.
///
/// It is a **dev dependency** of the host, so cargo builds it before
/// the tests run, and it is a `cdylib`, so what is built is a shared
/// library and not a Rust library.
fn library_path() -> PathBuf {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    directory.join(fixture_name("craftkit-clap-fixture"))
}

/// Open the plugin, initialise it, and make the instance.
///
/// The instance borrows the factory, which borrows the plugin, so the
/// three of them live in one block: a macro is the way to say that
/// once.
#[cfg(unix)]
macro_rules! with_instance {
    (|$plugin:ident, $instance:ident| $body:block) => {{
        let path = library_path();
        let path = path.to_string_lossy().to_string();
        let $plugin = ClapPlugin::open(&path).expect("the plugin opens");
        $plugin.init().expect("the plugin initialises");
        let factory = $plugin.factory().expect("the factory is there");
        let $instance = factory.create("org.craftkit.fixture.clap").expect("the plugin is made");
        $body
    }};
}

/// A plugin is **made** by the factory, and it is the plugin the
/// factory described.
#[cfg(unix)]
#[test]
fn a_plugin_is_made_by_its_id() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        let descriptor = instance.descriptor().expect("the descriptor is there");
        assert_eq!(descriptor.id, "org.craftkit.fixture.clap", "the instance is that plugin");
        assert_eq!(descriptor.name, "Craftkit Reference", "and it is that one");
    });
}

/// The plugin is given a **host**, and it reads the host's name.
#[cfg(unix)]
#[test]
fn the_plugin_reads_the_host_it_was_given() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        // The instance is initialised, and the handshake happens there:
        // the plugin reads the host's name and asks the host to restart
        // it.
        instance.init().expect("the instance initialises");
        let requests = instance.requests();
        assert!(requests.restart, "the plugin asked the host to restart it: {requests:?}");
    });
}

/// The instance has a **life**: it is initialised, activated at a
/// rate, deactivated and destroyed.
#[cfg(unix)]
#[test]
fn the_instance_has_a_life() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        assert!(instance.requests().process, "the plugin asked to be called back");
        instance.deactivate().expect("the instance deactivates");
        instance.start_processing().expect("the instance starts");
        instance.stop_processing().expect("the instance stops");
        instance.reset().expect("the instance resets");
        instance.destroy();
    });
}

/// The audio **crosses the boundary**.
///
/// The reference plugin is a gain: it adds one to every sample. So the
/// samples that come out say two things at once — that the input
/// arrived where the plugin expected it, and that the output left
/// where the host expected it.
#[cfg(unix)]
#[test]
fn the_audio_crosses_the_boundary() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let mut block = ClapBlock::new(2, 4);
        block.inputs[0] = vec![0.5; 4];
        block.inputs[1] = vec![0.25; 4];
        let status = instance.process(4, &mut block).expect("the plugin processes");
        assert_eq!(status, ClapProcessStatus::Continue, "the plugin processed the block");
        assert_eq!(block.outputs[0], vec![1.5; 4], "the first channel is the first channel plus one");
        assert_eq!(block.outputs[1], vec![1.25; 4], "and so is the second");
    });
}

/// An instance that was never initialised does **no** audio.
///
/// The plugin says so, and the host believes it: a plugin that is not
/// ready must not be asked to work.
#[cfg(unix)]
#[test]
fn an_instance_that_is_not_ready_does_no_audio() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        let mut block = ClapBlock::new(1, 2);
        let status = instance.process(2, &mut block).expect("the plugin is asked");
        assert_eq!(status, ClapProcessStatus::ContinueIfNotProcessed, "the plugin did not process the block");
        assert_eq!(block.outputs[0], vec![0.0; 2], "and wrote nothing");
    });
}

/// The plugin says what **parameters** it holds.
///
/// The host asks the plugin for its parameters extension, and the
/// plugin answers with its own table: how many, what each of them is,
/// and what each of them is set to.
#[cfg(unix)]
#[test]
fn the_plugin_says_what_parameters_it_holds() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters.len(), 3, "the plugin holds three parameters");
        assert_eq!(parameters[0].id, 1, "the first parameter is the gain");
        assert_eq!(parameters[0].name, "gain", "and it is called that");
        assert_eq!(parameters[0].min_value, 0.0, "its range starts at zero");
        assert_eq!(parameters[0].max_value, 2.0, "and ends at two");
        assert_eq!(parameters[0].default_value, 1.0, "and it starts at one");
        assert_eq!(parameters[0].value, 1.0, "and it is set to one");
        assert_eq!(parameters[1].id, 2, "the second parameter is the tone");
        assert_eq!(parameters[1].name, "tone", "and it is called that");
        assert_eq!(parameters[1].value, 0.5, "and it is set to half");
        assert_eq!(parameters[2].id, 3, "the third parameter is the count of notes");
        assert_eq!(parameters[2].name, "notes", "and it is called that");
        assert_eq!(parameters[2].value, 0.0, "and no note has been played yet");
    });
}

/// A parameter **moves**, and the plugin knows.
///
/// The format sets a parameter by **offering an event**: the host
/// offers, the plugin reads while it works, and the value is there
/// afterwards. So the test sets the gain, processes a block, and reads
/// the parameter back.
#[cfg(unix)]
#[test]
fn a_parameter_moves_and_the_plugin_knows() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let mut block = ClapBlock::new(2, 4);
        // The gain is set to one and a half.
        instance.set_parameter(1, 1.5);
        instance.process(4, &mut block).expect("the plugin processes");
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters[0].value, 1.5, "the plugin took the value the host offered");
    });
}

/// The **transport** reaches the plugin, and the plugin reports it
/// back.
///
/// The transport is an event like any other: the host offers where the
/// music is, and the plugin reads it while it works. The plugin then
/// says what it made of it, by **offering an event of its own** — so
/// the test sees the tempo twice: in the plugin's own parameter, and in
/// the events it pushed.
#[cfg(unix)]
#[test]
fn the_transport_reaches_the_plugin_and_comes_back() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let mut block = ClapBlock::new(2, 4);
        // The music is four beats in, at a hundred and twenty beats a
        // minute, in three-four.
        instance.set_transport(ClapTransport {
            song_pos_beats: 4.0,
            song_pos_seconds: 2.0,
            tempo: 120.0,
            tsig_num: 3,
            tsig_denom: 4,
            playing: true,
            recording: false,
        });
        instance.process(4, &mut block).expect("the plugin processes");
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters[1].value, 120.0, "the plugin took the tempo the host told it");
        let pushed = instance.pushed_events();
        let values = craftkit_clap::parameter_values(&pushed);
        assert!(values.contains(&(2, 120.0)), "and the plugin reported it back: {values:?}");
    });
}

/// A **note** reaches the plugin, and the plugin counts it.
///
/// The MIDI message is an event like any other: the host offers it
/// and the plugin reads it while it works. The reference plugin counts
/// the notes it was played, and says so through a parameter — so the
/// test sees that the message arrived whole.
#[cfg(unix)]
#[test]
fn a_note_reaches_the_plugin() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let mut block = ClapBlock::new(2, 4);
        // A note on: middle C, at the velocity of a piano.
        instance.send_midi(0, [0x90, 60, 100]);
        instance.process(4, &mut block).expect("the plugin processes");
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters.len(), 3, "the plugin holds three parameters");
        assert_eq!(parameters[2].name, "notes", "the third is the count of notes");
        assert_eq!(parameters[2].value, 1.0, "and the plugin was played one note");
    });
}

/// The plugin's **state** is saved and given back.
///
/// The state is **opaque**: the host does not know what is in it, and
/// it does not have to — it keeps the bytes and gives them back. So
/// the test moves a parameter, saves the state, moves it again, gives
/// the state back, and finds the parameter where it was.
#[cfg(unix)]
#[test]
fn the_state_is_saved_and_given_back() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        // The gain is moved to one and a half.
        instance.set_parameter(1, 1.5);
        instance.process(4, &mut ClapBlock::new(2, 4)).expect("the plugin processes");
        let saved = instance.save_state().expect("the state is saved");
        assert!(!saved.is_empty(), "the state is not empty");
        // The gain is moved again, and the state is given back.
        instance.set_parameter(1, 0.5);
        instance.process(4, &mut ClapBlock::new(2, 4)).expect("the plugin processes");
        instance.load_state(&saved).expect("the state is given back");
        instance.process(4, &mut ClapBlock::new(2, 4)).expect("the plugin processes");
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters[0].value, 1.5, "the gain is where it was left");
    });
}

/// An id the factory does not know is refused, by name.
#[cfg(unix)]
#[test]
fn an_id_the_factory_does_not_know_is_refused() {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    let plugin = ClapPlugin::open(&path).expect("the plugin opens");
    plugin.init().expect("the plugin initialises");
    let factory = plugin.factory().expect("the factory is there");
    let error = factory.create("org.craftkit.fixture.nothing").expect_err("there is no such plugin");
    assert!(error.to_string().contains("made no plugin"), "{error}");
}

/// The host has **no extensions** to give, and says so.
#[cfg(unix)]
#[test]
fn the_host_has_no_extensions() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        // The plugin asked for nothing, and the host answered nothing:
        // the only thing the test can see is that the plugin is still
        // alive.
        assert!(instance.requests().restart, "the plugin asked for a restart and got nothing back");
    });
}

/// Where there is no shared library, the door **refuses**, and says why.
///
/// This test runs on every target that is not unix, so the door is not
/// untested there: it is tested for what it does, which is refuse.
#[cfg(not(unix))]
#[test]
fn a_shared_library_cannot_be_opened_on_this_target() {
    let path = library_path().to_string_lossy().to_string();
    let error = ClapPlugin::open(&path).expect_err("there is no door on this target");
    assert!(error.to_string().contains("a shared library cannot be opened on this target"), "{error}");
}
