//! The VST3 host, against a **real** plugin.
//!
//! The plugin under test is `craftkit-vst3-fixture`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds the `GetPluginFactory`
//! symbol in it with `dlsym`, **calls** it, and calls through the
//! plugin's own function pointers.

use std::path::PathBuf;

use craftkit_vst3::{BUS_DIRECTION_INPUT, BUS_DIRECTION_OUTPUT, Vst3Interface, Vst3Plugin};

/// The shared library the tests open.
///
/// It is a **dev dependency** of the host, so cargo builds it before
/// the tests run, and it is a `cdylib`, so what is built is a shared
/// library and not a Rust library.
///
/// The path is derived from the test binary's own path: cargo puts the
/// binary in `target/<profile>/deps` and the shared library next to it
/// in `target/<profile>`.
fn library_path() -> PathBuf {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    directory.join("libcraftkit_vst3_fixture.so")
}

/// Open the plugin.
fn open() -> Vst3Plugin {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    Vst3Plugin::open(&path).unwrap_or_else(|error| panic!("the plugin opens: {error}"))
}

/// The host opens a VST3 plugin and calls its entry.
#[test]
fn the_host_opens_a_plugin_and_calls_the_entry() {
    let plugin = open();
    assert_eq!(plugin.factory().count(), 1, "the plugin holds one class");
}

/// The class says what it is, and the strings are the plugin's own.
#[test]
fn the_class_says_what_it_is() {
    let plugin = open();
    let classes = plugin.factory().classes().expect("the classes are read");
    assert_eq!(classes.len(), 1, "one class");
    let class = &classes[0];
    assert_eq!(class.cid, "Craftkit::Fixture", "the class is named");
    assert_eq!(class.name, "Craftkit Reference", "and it has a name");
    assert_eq!(class.vendor, "Craftkit", "and a vendor");
    assert_eq!(class.category, "Audio Module Class", "and a kind");
    assert_eq!(class.version, 1, "and a version");
}

/// An instance is made, and it is initialised.
#[test]
fn an_instance_is_made_and_initialised() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    instance.initialize().expect("the plugin initialises");
    assert_eq!(instance.interface(), Vst3Interface::Component, "and it is the component");
}

/// The plugin's buses are its own: the host asks and does not guess.
#[test]
fn the_buses_are_the_plugins_own() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    assert_eq!(instance.bus_count(BUS_DIRECTION_INPUT), 1, "one bus in");
    assert_eq!(instance.bus_count(BUS_DIRECTION_OUTPUT), 1, "and one bus out");
    let bus = instance.bus_info(BUS_DIRECTION_INPUT, 0).expect("the bus is read");
    assert_eq!(bus.channels, 1, "with one channel");
    assert_eq!(bus.name, "Main", "and it is called Main");
}

/// The parameter is there, and its range is the plugin's.
#[test]
fn the_parameter_is_there() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    assert_eq!(instance.param_count(), 1, "one parameter");
    let parameter = instance.param_info(0).expect("the parameter is read");
    assert_eq!(parameter.id, "gain", "it is the gain");
    assert_eq!(parameter.title, "Gain", "and it says so");
    assert_eq!(parameter.min, 0.0, "from nothing");
    assert_eq!(parameter.max, 2.0, "to twice as much");
}

/// A value that is set is a value that is read back.
#[test]
fn a_value_that_is_set_is_read_back() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    let before = instance.param_value(0).expect("the value is read");
    assert!((before - 0.5).abs() < 0.001, "the gain starts at one, which is half way");
    instance.set_param_value(0, 0.25).expect("the value is set");
    let after = instance.param_value(0).expect("the value is read");
    assert!((after - 0.25).abs() < 0.001, "and the value took");
}

/// The **state** is the plugin's own bytes, and they travel.
#[test]
fn the_state_travels() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    instance.set_param_value(0, 0.75).expect("the value is set");
    let state = instance.state().expect("the state is read");
    assert_eq!(state.len(), 4, "the plugin's own state is four bytes");
    instance.set_param_value(0, 0.0).expect("the value is set");
    instance.set_state(&state).expect("the state is given back");
    let value = instance.param_value(0).expect("the value is read");
    assert!((value - 0.75).abs() < 0.001, "and the state carried it");
}

/// The audio goes through: what comes out is what went in, louder.
#[test]
fn the_audio_goes_through() {
    let plugin = open();
    let instance =
        plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the instance is made");
    instance.set_param_value(0, 0.75).expect("the value is set");
    let input = [0.25f32, 0.5, -0.25, 1.0];
    let mut output = [0.0f32; 4];
    instance.process(&[&input], &[&mut output], 4).expect("the block is processed");
    // The gain is one and a half: three quarters of the way from zero
    // to two.
    assert!((output[0] - 0.375).abs() < 0.0001, "the first sample is louder");
    assert!((output[1] - 0.75).abs() < 0.0001, "and the second");
    assert!((output[3] - 1.5).abs() < 0.0001, "and the last");
}

/// The **audio processor** is the same instance, seen through another
/// table.
#[test]
fn the_audio_processor_is_the_same_instance() {
    let plugin = open();
    let component = plugin
        .factory()
        .create("Craftkit::Fixture", Vst3Interface::Component)
        .expect("the component is made");
    component.set_param_value(0, 0.25).expect("the value is set");
    let processor = plugin
        .factory()
        .create("Craftkit::Fixture", Vst3Interface::AudioProcessor)
        .expect("the processor is made");
    assert_eq!(processor.interface(), Vst3Interface::AudioProcessor, "it is the processor");
    // The processor does the sound and the component holds the
    // parameter: the state behind them is one.
    let input = [1.0f32, 1.0];
    let mut output = [0.0f32; 2];
    processor.process(&[&input], &[&mut output], 2).expect("the block is processed");
    assert!((output[0] - 0.5).abs() < 0.0001, "the gain the component set is the gain it used");
}

/// The processor is not the component, and the host says so.
#[test]
fn the_processor_is_not_the_component() {
    let plugin = open();
    plugin.factory().create("Craftkit::Fixture", Vst3Interface::Component).expect("the component is made");
    let processor = plugin
        .factory()
        .create("Craftkit::Fixture", Vst3Interface::AudioProcessor)
        .expect("the processor is made");
    assert!(processor.param_count() == 0, "the processor holds no parameters of its own");
    assert!(processor.state().is_err(), "and it will not say what the component's state is");
}

/// A class that is not there is **refused**, and not guessed at.
#[test]
fn a_class_that_is_not_there_is_refused() {
    let plugin = open();
    let made = plugin.factory().create("Nothing::Here", Vst3Interface::Component);
    assert!(made.is_err(), "the plugin would not make a class it does not hold");
}

/// A library that is not a plugin is **refused**.
#[test]
fn a_library_that_is_not_a_plugin_is_refused() {
    let path = std::env::temp_dir().join("craftkit-v2-vst3-not-a-plugin.so");
    std::fs::write(&path, b"this is not a shared library").expect("the file is written");
    let path = path.to_string_lossy().to_string();
    let opened = Vst3Plugin::open(&path);
    assert!(opened.is_err(), "the library did not open");
}
