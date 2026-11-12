//! The CLAP host, against a **real** plugin.
//!
//! The plugin under test is `craftkit-clap-fixture`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds the `clap_entry` symbol in
//! it with `dlsym`, reads the pointer inside it, and calls through the
//! plugin's own function pointers.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use std::path::PathBuf;

use craftkit_clap::ClapPlugin;
#[cfg(unix)]
use craftkit_clap_abi::ClapVersion;

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
///
/// The path is derived from the test binary's own path: cargo puts the
/// binary in `target/<profile>/deps` and the shared library next to it
/// in `target/<profile>`.
fn library_path() -> PathBuf {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    directory.join(fixture_name("craftkit-clap-fixture"))
}

/// A shared library of the machine itself: it exists, and it is not a
/// plugin.
///
/// Linux names the C maths library `libm.so.6`; macOS keeps the C
/// library in `/usr/lib/libSystem.B.dylib`, which is a stub that still
/// exists and is still openable.
#[cfg(unix)]
fn a_library_that_is_not_a_plugin() -> &'static str {
    if cfg!(target_os = "macos") { "/usr/lib/libSystem.B.dylib" } else { "libm.so.6" }
}

/// Open the plugin.
#[cfg(unix)]
fn open() -> ClapPlugin {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    ClapPlugin::open(&path).unwrap_or_else(|error| panic!("the plugin opens: {error}"))
}

/// Open the plugin and initialise it.
#[cfg(unix)]
fn open_and_init() -> ClapPlugin {
    let plugin = open();
    plugin.init().unwrap_or_else(|error| panic!("the plugin initialises: {error}"));
    plugin
}

/// The host opens a CLAP plugin and finds the entry it exports.
#[cfg(unix)]
#[test]
fn the_host_opens_a_plugin_and_finds_the_entry() {
    let plugin = open();
    assert_eq!(plugin.version(), ClapVersion::CURRENT, "the plugin says which version it speaks");
}

/// The version is checked **before** anything in the plugin is called.
#[cfg(unix)]
#[test]
fn the_version_is_checked_before_anything_is_called() {
    let plugin = open();
    // A plugin of a different major version is refused: the shapes may
    // have changed, and a host that calls into it is guessing.
    assert!(
        plugin.version().is_compatible_with(ClapVersion::CURRENT),
        "the plugin the host opened is one it can use"
    );
    assert!(
        ClapVersion::CURRENT.is_compatible_with(ClapVersion::CURRENT),
        "the same major version is compatible"
    );
    assert!(
        !ClapVersion::CURRENT.is_compatible_with(ClapVersion { major: 2, minor: 0, revision: 0 }),
        "another major version is not"
    );
    assert!(
        ClapVersion::CURRENT.is_compatible_with(ClapVersion { major: 1, minor: 9, revision: 7 }),
        "the minor version does not decide"
    );
}

/// The factory is not asked for before the plugin is initialised.
#[cfg(unix)]
#[test]
fn the_factory_comes_after_the_initialisation() {
    let plugin = open();
    let error = plugin.factory().expect_err("the factory is not there yet");
    assert!(error.to_string().contains("not been initialised"), "{error}");
}

/// Initialising twice is refused: the format does not say what happens
/// then.
#[cfg(unix)]
#[test]
fn initialising_twice_is_refused() {
    let plugin = open_and_init();
    let error = plugin.init().expect_err("it is already initialised");
    assert!(error.to_string().contains("already initialised"), "{error}");
}

/// The factory holds the plugin, and says how many it holds.
#[cfg(unix)]
#[test]
fn the_factory_holds_the_plugin() {
    let plugin = open_and_init();
    let factory = plugin.factory().expect("the factory is there");
    assert_eq!(factory.count(), 1, "the fixture holds one plugin");
}

/// The descriptor comes back, field for field.
#[cfg(unix)]
#[test]
fn the_descriptor_comes_back() {
    let plugin = open_and_init();
    let factory = plugin.factory().expect("the factory is there");
    let descriptor = factory.descriptor(0).expect("the descriptor is there");
    assert_eq!(descriptor.id, "org.craftkit.fixture.clap", "the id is the plugin's own");
    assert_eq!(descriptor.name, "Craftkit Reference", "the name is the plugin's own");
    assert_eq!(descriptor.vendor, "Craftkit", "the vendor is the plugin's own");
    assert_eq!(descriptor.version, "1.0.0", "the version is the plugin's own");
    assert_eq!(descriptor.url, "https://example.invalid/craftkit", "the url is the plugin's own");
    assert_eq!(
        descriptor.description, "A reference plugin: it holds one plugin and does nothing.",
        "the description is the plugin's own"
    );
}

/// The features are a list of C strings, and they all come back.
#[cfg(unix)]
#[test]
fn the_features_come_back() {
    let plugin = open_and_init();
    let factory = plugin.factory().expect("the factory is there");
    let descriptor = factory.descriptor(0).expect("the descriptor is there");
    assert_eq!(
        descriptor.features,
        vec!["audio-effect", "stereo", "mono"],
        "the features are the plugin's own"
    );
}

/// An index the factory does not have is refused, by number.
#[cfg(unix)]
#[test]
fn an_index_that_is_not_there_is_refused() {
    let plugin = open_and_init();
    let factory = plugin.factory().expect("the factory is there");
    let error = factory.descriptor(7).expect_err("there is no plugin at seven");
    assert!(error.to_string().contains('7'), "{error}");
}

/// A library that is not there is refused, by name.
#[cfg(unix)]
#[test]
fn a_library_that_is_not_there_is_refused() {
    let error = ClapPlugin::open("/nonexistent/libcraftkit_nothing.so").expect_err("it is not there");
    assert!(error.to_string().contains("did not open"), "{error}");
}

/// A library that is not a CLAP plugin is refused too: it has no entry.
#[cfg(unix)]
#[test]
fn a_library_without_an_entry_is_refused() {
    // The C library of the machine itself: it is a shared object, and it
    // is not a CLAP plugin.
    let error = ClapPlugin::open(a_library_that_is_not_a_plugin()).expect_err("it has no entry");
    assert!(error.to_string().contains("clap_entry"), "{error}");
}

/// The plugin cannot be forgotten: the library is closed when the plugin
/// is dropped, and the plugin is deinitialised first.
#[cfg(unix)]
#[test]
fn the_plugin_is_deinitialised_when_it_is_dropped() {
    let plugin = open_and_init();
    let path = plugin.path().to_string();
    assert_eq!(path, library_path().to_string_lossy(), "the plugin knows its own path");
    drop(plugin);
    // The plugin is gone, and nothing of it is left open: the drop is
    // the only place that closes the library.
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
