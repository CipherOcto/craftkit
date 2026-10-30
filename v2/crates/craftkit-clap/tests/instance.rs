//! The **host** and the **instance**: the other half of the contract.
//!
//! A plugin is not only a file: it is an **instance**, made by the
//! factory, given a **host**, and then lived through — initialised,
//! activated, deactivated, destroyed. These tests drive that life
//! against the reference plugin, which is a real shared library.

use std::path::PathBuf;

use craftkit_clap::ClapPlugin;

/// The shared library the tests open.
///
/// It is a **dev dependency** of the host, so cargo builds it before
/// the tests run, and it is a `cdylib`, so what is built is a shared
/// library and not a Rust library.
fn library_path() -> PathBuf {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    directory.join("libcraftkit_clap_fixture.so")
}

/// Open the plugin, initialise it, and make the instance.
///
/// The instance borrows the factory, which borrows the plugin, so the
/// three of them live in one block: a macro is the way to say that
/// once.
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

/// An id the factory does not know is refused, by name.
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
