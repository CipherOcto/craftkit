//! The door: a host that opens a shared library and calls the plugin
//! table inside it.
//!
//! The library under test is `craftkit-plugin-shared`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds a symbol in it with
//! `dlsym`, and calls through the table.

use std::path::PathBuf;

use craftkit_ffi::{Library, Plugins};

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
    directory.join("libcraftkit_plugin_shared.so")
}

/// Open the shared library.
fn open() -> Library {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    Library::open(&path).unwrap_or_else(|error| panic!("the library opens: {error}"))
}

/// The host opens a shared library and finds the table in it.
#[test]
fn the_host_opens_a_library_and_finds_the_table() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    assert_eq!(plugins.count(), 2, "the library holds two plugins");
}

/// The plugin's own name comes back through the door.
#[test]
fn the_plugins_name_comes_back() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    assert_eq!(plugins.name(0).expect("the name is there"), "the demo");
    assert_eq!(plugins.name(1).expect("the name is there"), "the second");
}

/// And its parameters, which are JSON.
#[test]
fn the_plugins_parameters_come_back() {
    let library = open();
    let plugins = library.plugins().expect("the table is there");
    let params = plugins.params(0).expect("the parameters are there");
    assert_eq!(params, r#"{"kind":"demo","channels":2}"#, "the parameters are JSON, byte for byte");
}

/// A library that is not there is refused, by name.
#[test]
fn a_library_that_is_not_there_is_refused() {
    let error = Library::open("/nonexistent/libcraftkit_nothing.so").expect_err("it is not there");
    assert!(error.to_string().contains("did not open"), "{error}");
}

/// A library that is not a plugin is refused too: it has no table.
#[test]
fn a_library_without_a_table_is_refused() {
    // The C library of the machine itself: it is a shared object, and it
    // is not a craftkit plugin.
    let library = Library::open("libm.so.6").expect("the C library opens");
    let error = library.plugins().expect_err("it has no table");
    assert!(error.to_string().contains("craftkit_plugin_vtable"), "{error}");
}

/// The plugins cannot outlive the library they are in.
#[test]
fn the_plugins_cannot_outlive_the_library() {
    fn plugins_of(library: &Library) -> Plugins<'_> {
        library.plugins().expect("the table is there")
    }
    let library = open();
    let count = plugins_of(&library).count();
    assert_eq!(count, 2);
}
