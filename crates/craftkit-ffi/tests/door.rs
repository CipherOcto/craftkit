//! The door: a host that opens a shared library and calls the plugin
//! table inside it.
//!
//! The library under test is `craftkit-plugin-shared`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds a symbol in it with
//! `dlsym`, and calls through the table.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use std::path::PathBuf;

use craftkit_ffi::Library;
#[cfg(unix)]
use craftkit_ffi::Plugins;

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
/// It is a **dev dependency** of the loader, so cargo builds it before
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
    directory.join(fixture_name("craftkit-plugin-shared"))
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

/// Open the shared library.
#[cfg(unix)]
fn open() -> Library {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    Library::open(&path).unwrap_or_else(|error| panic!("the library opens: {error}"))
}

/// The host opens a shared library and finds the table in it.
#[cfg(unix)]
#[test]
fn the_host_opens_a_library_and_finds_the_table() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    assert_eq!(plugins.count(), 2, "the library holds two plugins");
}

/// The plugin's own name comes back through the door.
#[cfg(unix)]
#[test]
fn the_plugins_name_comes_back() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    assert_eq!(plugins.name(0).expect("the name is there"), "the demo");
    assert_eq!(plugins.name(1).expect("the name is there"), "the second");
}

/// And its parameters, which are JSON.
#[cfg(unix)]
#[test]
fn the_plugins_parameters_come_back() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    let params = plugins.params(0).expect("the parameters are there");
    assert_eq!(params, r#"{"kind":"demo","channels":2}"#, "the parameters are JSON, byte for byte");
}

/// A library that is not there is refused, by name.
#[cfg(unix)]
#[test]
fn a_library_that_is_not_there_is_refused() {
    let error = Library::open("/nonexistent/libcraftkit_nothing.so").expect_err("it is not there");
    assert!(error.to_string().contains("did not open"), "{error}");
}

/// A library that is not a plugin is refused too: it has no table.
#[cfg(unix)]
#[test]
fn a_library_without_a_table_is_refused() {
    // The C library of the machine itself: it is a shared object, and it
    // is not a craftkit plugin.
    let library = Library::open(a_library_that_is_not_a_plugin()).expect("the C library opens");
    let error = library.plugins().expect_err("it has no table");
    assert!(error.to_string().contains("craftkit_plugin_vtable"), "{error}");
}

/// The plugins cannot outlive the library they are in.
#[cfg(unix)]
#[test]
fn the_plugins_cannot_outlive_the_library() {
    fn plugins_of(library: &Library) -> Plugins<'_> {
        library.plugins().expect("the table is there")
    }
    let library = open();
    let count = plugins_of(&library).count();
    assert_eq!(count, 2);
}

/// Where there is no shared library, the door **refuses**, and says why.
///
/// This test runs on every target that is not unix, so the door is not
/// untested there: it is tested for what it does, which is refuse.
#[cfg(not(unix))]
#[test]
fn a_shared_library_cannot_be_opened_on_this_target() {
    let path = library_path().to_string_lossy().to_string();
    let error = Library::open(&path).expect_err("there is no door on this target");
    assert!(error.to_string().contains("a shared library cannot be opened on this target"), "{error}");
}
