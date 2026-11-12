//! The door, reached the way everything is reached: through a command.
//!
//! A plugin that is not part of the program is a shared library, and a
//! session may have one open. The commands are `plugin.library.open`,
//! `plugin.library.list` and `plugin.library.close`, and the library
//! under test is a real `cdylib` that cargo builds.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;
use std::path::PathBuf;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

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
/// It is a **dev dependency** of the studio, so cargo builds it before
/// the tests run, and it is a `cdylib`, so what is built is a shared
/// library and not a Rust library.
fn library_path() -> String {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    let path: PathBuf = directory.join(fixture_name("craftkit-plugin-shared"));
    path.to_string_lossy().to_string()
}

/// The commands are in the registry.
#[test]
fn the_door_is_in_the_registry() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("plugin.library.open").is_some(), "the door is in");
    assert!(ecosystem.commands.get("plugin.library.list").is_some());
    assert!(ecosystem.commands.get("plugin.library.close").is_some());
}

/// A session opens a library and asks it what it has.
#[test]
fn a_session_opens_a_library_and_asks_it_what_it_has() {
    let mut session = ecosystem().session();
    let opened = session
        .execute("plugin.library.open", &json!({ "path": library_path() }))
        .expect("the library opens");
    assert_eq!(opened.data["library"], json!(1), "the library is numbered");

    let listed = session.execute("plugin.library.list", &json!({})).expect("the list is read");
    let libraries = listed.data["libraries"].as_array().expect("there are libraries");
    assert_eq!(libraries.len(), 1);
    let plugins = libraries[0]["plugins"].as_array().expect("there are plugins");
    assert_eq!(plugins.len(), 2, "the library holds two plugins");
    assert_eq!(plugins[0]["name"], json!("the demo"));
    assert_eq!(plugins[1]["params"], json!(r#"{"kind":"demo","channels":1}"#));
}

/// The library is **session state**: the undo does not touch it, and it
/// is not in the document.
#[test]
fn the_library_is_session_state_and_not_in_the_document() {
    let mut session = ecosystem().session();
    session.execute("plugin.library.open", &json!({ "path": library_path() })).expect("the library opens");
    assert_eq!(session.plugins().len(), 1, "the session holds the library");
    assert_eq!(session.document().nodes.len(), 0, "the document holds nothing");
    // The undo has nothing to undo: opening a library does not change
    // the document, so there is no state to go back to — and the library
    // is still open.
    let error = session.undo().expect_err("there is nothing to undo");
    assert!(error.to_string().contains("nothing to undo"), "{error}");
    assert_eq!(session.plugins().len(), 1, "the undo does not close a library");
}

/// A library that is not there is refused, by name.
#[test]
fn a_library_that_is_not_there_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("plugin.library.open", &json!({ "path": "/nonexistent/libnothing.so" }))
        .expect_err("it is not there");
    assert!(error.to_string().contains("did not open"), "{error}");
    assert_eq!(session.plugins().len(), 0, "nothing was kept");
}

/// A library that is closed twice is refused the second time.
#[test]
fn a_library_that_is_closed_twice_is_refused() {
    let mut session = ecosystem().session();
    session.execute("plugin.library.open", &json!({ "path": library_path() })).expect("the library opens");
    session.execute("plugin.library.close", &json!({ "library": 1 })).expect("it closes");
    let error = session.execute("plugin.library.close", &json!({ "library": 1 })).expect_err("it is closed");
    assert!(error.to_string().contains("no library"), "{error}");
}
