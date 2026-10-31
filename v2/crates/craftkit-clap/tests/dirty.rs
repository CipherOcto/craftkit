//! The **word** that the plugin's state has changed.
//!
//! A plugin says so through the **host's** state extension: the same
//! extension id on both sides of the format, the plugin's table saving
//! and loading, and the host's table marking the state dirty. These
//! tests are about the host hearing it.

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

/// The plugin **says** its state has changed, and the host hears it.
///
/// The reference plugin marks its state dirty when a value changes,
/// and the host keeps the word until it saves the state or gives it
/// back.
#[test]
fn the_plugin_says_its_state_has_changed() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        assert!(!instance.state_dirty(), "nothing has changed yet");

        // The gain is moved, and the plugin says so.
        instance.set_parameter(1, 1.5);
        instance.process(4, &mut craftkit_clap::ClapBlock::new(2, 4)).expect("the plugin processes");
        assert!(instance.state_dirty(), "the plugin said its state had changed");
    });
}

/// The word is good until the host **saves** the state.
///
/// After the save, what the host holds is what the plugin holds, and
/// the flag is cleared: a host that kept saying "dirty" forever would
/// be a host that saves on every block.
#[test]
fn the_word_is_good_until_the_state_is_saved() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        instance.set_parameter(1, 1.5);
        instance.process(4, &mut craftkit_clap::ClapBlock::new(2, 4)).expect("the plugin processes");
        assert!(instance.state_dirty(), "the plugin said its state had changed");

        let saved = instance.save_state().expect("the state is saved");
        assert!(!saved.is_empty(), "the state is not empty");
        assert!(!instance.state_dirty(), "and the host has it now");
    });
}

/// Giving the state **back** clears the word too.
///
/// The plugin has been given its state, so what the host holds is what
/// the plugin holds — the same rule, the other way round.
#[test]
fn giving_the_state_back_clears_the_word() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        instance.set_parameter(1, 1.5);
        instance.process(4, &mut craftkit_clap::ClapBlock::new(2, 4)).expect("the plugin processes");
        let saved = instance.save_state().expect("the state is saved");

        instance.set_parameter(1, 0.5);
        instance.process(4, &mut craftkit_clap::ClapBlock::new(2, 4)).expect("the plugin processes");
        assert!(instance.state_dirty(), "the plugin said its state had changed again");

        instance.load_state(&saved).expect("the state is given back");
        assert!(!instance.state_dirty(), "and the host and the plugin agree again");
    });
}
