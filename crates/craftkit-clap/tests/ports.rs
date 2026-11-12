//! The **ports**: the layout a plugin asks for.
//!
//! A plugin says how many ports it has and what each one is, and a host
//! that does not ask is a host that guesses. These tests are about the
//! asking and about what happens when the host guesses anyway.

use std::path::PathBuf;

use craftkit_clap::{ClapBlock, ClapPlugin};
use craftkit_clap_abi::ClapProcessStatus;

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

/// A plugin **says** how many channels it has.
///
/// The ports are an extension: the host asks for it by name, and the
/// plugin answers with its own table. The reference plugin is stereo —
/// one port in and one port out, and two channels in each.
#[test]
fn a_plugin_says_how_many_channels_it_has() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        let outputs = instance.audio_ports(false).expect("the output ports are there");
        assert_eq!(outputs.len(), 1, "the plugin has one output port");
        assert_eq!(outputs[0].name, "main", "and it is called that");
        assert_eq!(outputs[0].channel_count, 2, "and it has two channels");
        assert_eq!(outputs[0].port_type, "stereo", "and it is a stereo port");
        assert_ne!(outputs[0].flags & craftkit_clap_abi::AUDIO_PORT_IS_MAIN, 0, "and it is the main one");

        let inputs = instance.audio_ports(true).expect("the input ports are there");
        assert_eq!(inputs.len(), 1, "the plugin has one input port");
        assert_eq!(inputs[0].channel_count, 2, "and it has two channels");
        assert_eq!(instance.channels(false).expect("the channels are counted"), 2);
        assert_eq!(instance.channels(true).expect("the channels are counted"), 2);
    });
}

/// The host **asks**, and it believes the answer.
///
/// The layout is asked for once, when the plugin is activated, and the
/// host builds its blocks from it. A host that guessed would give this
/// plugin one channel, and this plugin has two.
#[test]
fn the_host_asks_and_believes_the_answer() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let layout = instance.layout().expect("the plugin said what shape its audio is");
        assert_eq!(layout.inputs, 2, "two channels in");
        assert_eq!(layout.outputs, 2, "and two out");

        let block = instance.block(4).expect("the block is the plugin's shape");
        assert_eq!(block.inputs.len(), 2, "the block has the plugin's inputs");
        assert_eq!(block.outputs.len(), 2, "and the plugin's outputs");
        assert_eq!(block.inputs[0].len(), 4, "and the frames the host asked for");
    });
}

/// A block that is **not** the layout the plugin asked for is refused.
///
/// The plugin does the refusing: it says it did not process the block,
/// and it writes nothing. That is the rule the format states — the
/// plugin is activated with the layout it asked for — and a plugin that
/// checks is a plugin that breaks a host that guesses.
#[test]
fn a_block_that_is_not_the_layout_is_refused() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        // One channel, where the plugin asked for two.
        let mut block = ClapBlock::new(1, 4);
        let status = instance.process(4, &mut block).expect("the plugin is asked");
        assert_eq!(
            status,
            ClapProcessStatus::ContinueIfNotProcessed,
            "the plugin did not process a block that is not its shape"
        );
        assert_eq!(block.outputs[0], vec![0.0; 4], "and it wrote nothing");
    });
}

/// The **audio** crosses with the layout the plugin asked for.
///
/// The same plugin, the same block, and the layout it said: two
/// channels in, two out, and the samples arrive where the plugin
/// expects them and leave where the host expects them.
#[test]
fn the_audio_crosses_with_the_layout_the_plugin_asked_for() {
    with_instance!(|plugin, instance| {
        let _ = plugin;
        instance.init().expect("the instance initialises");
        instance.activate(48_000.0, 1, 512).expect("the instance activates");
        let mut block = instance.block(2).expect("the block is the plugin's shape");
        block.inputs[0] = vec![0.5; 2];
        block.inputs[1] = vec![0.25; 2];
        let status = instance.process(2, &mut block).expect("the plugin processes");
        assert_eq!(status, ClapProcessStatus::Continue, "the plugin processed the block");
        assert_eq!(block.outputs[0], vec![1.5; 2], "the first channel is the first channel plus one");
        assert_eq!(block.outputs[1], vec![1.25; 2], "and so is the second");
    });
}
