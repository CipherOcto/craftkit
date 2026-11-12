//! The **transport**, offered with every block.
//!
//! The transport says where the music is. The music does not stop
//! between two blocks, so a host that offered the transport once would
//! tell the plugin where the music *was* and then never again.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use std::path::PathBuf;

use craftkit_clap::ClapPlugin;
#[cfg(unix)]
use craftkit_clap::{ClapBlock, ClapTransport, parameter_values};

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

/// The transport reaches the plugin in **every** block.
///
/// The reference plugin counts the blocks that carried a transport and
/// reports the count back, so the host can be seen to have offered it
/// every time — and not once.
#[cfg(unix)]
#[test]
fn the_transport_is_offered_with_every_block() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        instance.set_transport(ClapTransport {
            song_pos_beats: 4.0,
            song_pos_seconds: 2.0,
            tempo: 120.0,
            tsig_num: 3,
            tsig_denom: 4,
            playing: true,
            recording: false,
        });

        for _ in 0..3 {
            instance.process(4, &mut ClapBlock::new(2, 4)).expect("the plugin processes");
        }

        // The plugin reports how many blocks carried a transport.
        let pushed = instance.pushed_events();
        let values = parameter_values(&pushed);
        assert!(values.contains(&(4, 3.0)), "the plugin saw the transport in three blocks: {values:?}");
        // And it took the tempo in the last one.
        let parameters = instance.parameters().expect("the parameters are there");
        assert_eq!(parameters[1].value, 120.0, "and the tempo is still the tempo");
    });
}

/// A host that never said where the music is says nothing.
///
/// Without a transport the plugin is offered no transport at all, and
/// its count stays at nothing.
#[cfg(unix)]
#[test]
fn no_transport_means_no_transport() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        instance.process(4, &mut ClapBlock::new(2, 4)).expect("the plugin processes");
        let pushed = instance.pushed_events();
        let values = parameter_values(&pushed);
        assert!(!values.contains(&(4, 1.0)), "the plugin was offered no transport: {values:?}");
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
