//! The **plugin** in the document: an identity and a state.
//!
//! A CLAP plugin is a program, and the document cannot hold a program. So
//! the document holds what the plugin *said*: its name, its parameters and
//! its state — the plugin's own bytes, which the host never reads and the
//! plugin always can.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use craftkit_sdk::serde_json::json;

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
    directory.join(fixture_name("craftkit-clap-fixture")).to_string_lossy().to_string()
}

/// The id of the reference plugin.
#[cfg(unix)]
const PLUGIN: &str = "org.craftkit.fixture.clap";

/// A plugin is **attached** to the document.
///
/// The node says which plugin it is, and the payload holds the plugin's
/// identity: the library it came from, its name and its id.
#[cfg(all(feature = "clap", unix))]
#[test]
fn a_plugin_is_attached_to_the_document() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");

    let node = session.document().nodes.first().expect("there is a node");
    assert_eq!(node.kind, "clap.plugin", "the node is a plugin");
    assert_eq!(node.data.get("id").and_then(|id| id.as_str()), Some(PLUGIN), "and it is that plugin");
    assert_eq!(node.data.get("name").and_then(|name| name.as_str()), Some("Craftkit Reference"));
    // The shape of the plugin's audio is the plugin's own, and the
    // document says what the plugin said.
    assert_eq!(node.data.get("channels_in").and_then(|count| count.as_u64()), Some(2));
    assert_eq!(node.data.get("channels_out").and_then(|count| count.as_u64()), Some(2));
}

/// The **state** is the plugin's own bytes, and the host does not read
/// them.
///
/// The payload carries the state as base64 — the same answer the PDF
/// battery gives for a stream — and the bytes are the plugin's: the kit
/// can say how many there are and nothing else.
#[cfg(all(feature = "clap", unix))]
#[test]
fn the_state_is_the_plugins_own_bytes() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node");

    let state = node.data.get("state").and_then(|state| state.as_str()).expect("there is a state");
    let bytes = craftkit_sdk::bytes::decode(state).expect("the state is base64");
    assert!(!bytes.is_empty(), "the plugin saved something");

    // The plugin is asked what it makes of the state the document holds,
    // and it says how many bytes there are: the same number.
    let outcome = session.execute("clap.state", &json!({ "node": node.id })).expect("the state is read");
    assert_eq!(
        outcome.data.get("state_bytes").and_then(|bytes| bytes.as_u64()),
        Some(bytes.len() as u64),
        "the plugin reads the same bytes the document holds"
    );
}

/// A **parameter** moves, and the state follows.
///
/// The value is offered to the plugin as an event, the plugin reads it
/// while it works, and the state it saves afterwards is what the
/// document keeps. So the document's copy of the state is never a copy
/// of something the plugin did not do.
#[cfg(all(feature = "clap", unix))]
#[test]
fn a_parameter_moves_and_the_state_follows() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node").id;

    let before = session
        .document()
        .node(node)
        .and_then(|node| node.data.get("state").and_then(|state| state.as_str()))
        .map(|state| state.to_string())
        .expect("there is a state");
    session
        .execute("clap.parameter.set", &json!({ "node": node, "parameter": 1, "value": 0.25 }))
        .expect("the parameter is set");

    let after = session
        .document()
        .node(node)
        .and_then(|node| node.data.get("state").and_then(|state| state.as_str()))
        .map(|state| state.to_string())
        .expect("there is a state");
    assert_ne!(before, after, "the state changed with the parameter");

    // And the plugin says so: it is given the state's bytes and reports
    // the value it holds.
    let outcome = session.execute("clap.state", &json!({ "node": node })).expect("the state is read");
    let params = outcome.data.get("params").and_then(|params| params.as_array()).expect("there are params");
    let gain = params.iter().find(|param| param.get("id").and_then(|id| id.as_u64()) == Some(1));
    assert_eq!(gain.and_then(|gain| gain.get("value")).and_then(|value| value.as_f64()), Some(0.25));
}

/// The **plugins** in a library are read without touching the document.
///
/// It is a measurement, so it is not an undo step: the document is
/// exactly as it was.
#[cfg(all(feature = "clap", unix))]
#[test]
fn the_plugins_in_a_library_are_listed() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let outcome =
        session.execute("clap.list", &json!({ "path": library_path() })).expect("the library is read");
    let plugins =
        outcome.data.get("plugins").and_then(|plugins| plugins.as_array()).expect("there are plugins");
    assert_eq!(plugins.len(), 1, "the reference library holds one plugin");
    assert_eq!(plugins[0].get("id").and_then(|id| id.as_str()), Some(PLUGIN));
    assert!(session.document().nodes.is_empty(), "the document was not touched");
}

/// A library that is **not there** is refused, and nothing is added.
#[cfg(feature = "clap")]
#[test]
fn a_library_that_is_not_there_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let error = session
        .execute("clap.attach", &json!({ "path": "/nonexistent/libcraftkit_nothing.so" }))
        .expect_err("that is not a library");
    assert!(error.to_string().contains("clap"), "{error}");
    assert!(session.document().nodes.is_empty(), "the node was added anyway");
}

/// A file that is **not a plugin** is refused too.
///
/// The host checks the version of the format before it calls anything in
/// the library, so a library without the entry is a library that says so.
#[cfg(feature = "clap")]
#[test]
fn a_file_that_is_not_a_plugin_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let path =
        craftkit_scratch::file(&fixture_name("craftkit-v2-clap-not-a-plugin")).expect("the scratch is made");
    std::fs::write(&path, b"this is not a shared library").expect("the file is written");
    let error = session
        .execute("clap.attach", &json!({ "path": path.to_string_lossy() }))
        .expect_err("that is not a plugin");
    assert!(error.to_string().contains("clap"), "{error}");
    assert!(session.document().nodes.is_empty(), "the node was added anyway");
}

/// The plugin says its state has **changed**, and the document is
/// refreshed.
///
/// The word comes from the plugin, through the host's own state
/// extension: a value that moves is a state that changed, and a host
/// that does not hear it would keep a state that is out of date.
#[cfg(all(feature = "clap", unix))]
#[test]
fn the_document_is_refreshed_when_the_plugin_says_it_changed() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node").id;

    let outcome = session
        .execute("clap.parameter.set", &json!({ "node": node, "parameter": 1, "value": 0.5 }))
        .expect("the parameter is set");
    assert_eq!(
        outcome.data.get("dirty").and_then(|dirty| dirty.as_bool()),
        Some(true),
        "the plugin said its state had changed"
    );

    // And the document holds what the plugin holds: the copy is current.
    let outcome = session.execute("clap.state", &json!({ "node": node })).expect("the state is read");
    assert_eq!(
        outcome.data.get("dirty").and_then(|dirty| dirty.as_bool()),
        Some(false),
        "and the document's copy is the plugin's own"
    );
}

/// The transport is **the kit's own time**.
///
/// The position is a tick, and the beats, the seconds, the tempo and
/// the meter the plugin is told about are worked out from it. Nothing
/// is stored twice: the node holds the tick and the map, and the rest
/// is derived.
#[cfg(all(feature = "clap", unix))]
#[test]
fn the_transport_is_the_kits_own_time() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node").id;

    // Two beats, at a hundred and twenty beats a minute: one second.
    let outcome = session
        .execute("clap.transport.set", &json!({ "node": node, "tick": 1920 }))
        .expect("the music is moved");
    let transport = outcome.data.get("transport").expect("there is a transport");
    assert_eq!(transport.get("beats").and_then(|beats| beats.as_f64()), Some(2.0));
    assert_eq!(transport.get("seconds").and_then(|seconds| seconds.as_f64()), Some(1.0));
    assert_eq!(transport.get("tempo").and_then(|tempo| tempo.as_f64()), Some(120.0));
    assert_eq!(transport.get("playing").and_then(|playing| playing.as_bool()), Some(true));
}

/// The **tempo** changes, and the transport follows.
///
/// The tempo is a map and not a number: the tempo of a piece changes,
/// and the change happens at a tick.
#[cfg(all(feature = "clap", unix))]
#[test]
fn the_tempo_changes_and_the_transport_follows() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("clap.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node").id;

    // The tempo halves from the second beat onwards.
    session
        .execute("clap.transport.tempo", &json!({ "node": node, "tick": 1920, "bpm": 60.0 }))
        .expect("the tempo is set");
    // The music is still at the beginning, so the tempo there is the
    // first one.
    let outcome = session
        .execute("clap.transport.set", &json!({ "node": node, "tick": 0 }))
        .expect("the music is moved");
    let transport = outcome.data.get("transport").expect("there is a transport");
    assert_eq!(transport.get("tempo").and_then(|tempo| tempo.as_f64()), Some(120.0));

    // And when the music reaches the change, the new tempo is the one
    // in force.
    let outcome = session
        .execute("clap.transport.set", &json!({ "node": node, "tick": 1920 }))
        .expect("the music is moved");
    let transport = outcome.data.get("transport").expect("there is a transport");
    assert_eq!(transport.get("tempo").and_then(|tempo| tempo.as_f64()), Some(60.0));

    // And the document holds the map, not the number it worked out.
    let node = session.document().node(node).expect("the node is there");
    let tempos = node
        .data
        .get("transport")
        .and_then(|transport| transport.get("tempos"))
        .and_then(|tempos| tempos.as_array())
        .expect("there are tempo events");
    assert_eq!(tempos.len(), 2, "the map has two events: the first and the change");
    assert_eq!(tempos[1].get("bpm").and_then(|bpm| bpm.as_f64()), Some(60.0));
}

/// Where there is no shared library, the door **refuses**, and says why.
///
/// This test runs on every target that is not unix, so the door is not
/// untested there: it is tested for what it does, which is refuse.
#[cfg(all(feature = "clap", not(unix)))]
#[test]
fn a_shared_library_cannot_be_opened_on_this_target() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let error = session
        .execute("clap.attach", &json!({ "path": library_path() }))
        .expect_err("there is no door on this target");
    assert!(error.to_string().contains("a shared library cannot be opened on this target"), "{error}");
    assert!(session.document().nodes.is_empty(), "the node was added anyway");
}
