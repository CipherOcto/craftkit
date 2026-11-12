//! The **plugin** in the document: an identity and a state.
//!
//! A VST3 plugin is a program, and the document cannot hold a program. So
//! the document holds what the plugin *said*: its name, its parameters and
//! its state — the plugin's own bytes, which the host never reads and the
//! plugin always can.

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
    directory.join(fixture_name("craftkit-vst3-fixture")).to_string_lossy().to_string()
}

/// The id of the reference plugin.
const PLUGIN: &str = "Craftkit::Fixture";

/// A plugin is **attached** to the document.
///
/// The node says which plugin it is, and the payload holds the plugin's
/// identity: the library it came from, its name and its class.
#[cfg(feature = "vst3")]
#[test]
fn a_plugin_is_attached_to_the_document() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("vst3.attach", &json!({ "path": library_path() })).expect("the plugin is attached");

    let node = session.document().nodes.first().expect("there is a node");
    assert_eq!(node.kind, "vst3.plugin", "the node is a plugin");
    assert_eq!(node.data.get("cid").and_then(|cid| cid.as_str()), Some(PLUGIN), "and it is that class");
    assert_eq!(node.data.get("name").and_then(|name| name.as_str()), Some("Craftkit Reference"));
    // The shape of the plugin's audio is the plugin's own, and the
    // document says what the plugin said.
    assert_eq!(node.data.get("channels_in").and_then(|count| count.as_u64()), Some(1));
    assert_eq!(node.data.get("channels_out").and_then(|count| count.as_u64()), Some(1));
}

/// The **classes** in a library are read by the host.
#[cfg(feature = "vst3")]
#[test]
fn the_classes_in_a_library_are_read() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let outcome =
        session.execute("vst3.list", &json!({ "path": library_path() })).expect("the library is read");
    let classes = outcome.data.get("classes").and_then(|classes| classes.as_array());
    let classes = classes.expect("there are classes");
    assert_eq!(classes.len(), 1, "one class");
    assert_eq!(classes[0].get("cid").and_then(|cid| cid.as_str()), Some(PLUGIN));
}

/// The **state** is the plugin's own bytes, and the host does not read
/// them.
///
/// The payload carries the state as base64 — the same answer the PDF
/// battery gives for a stream — and the bytes are the plugin's: the kit
/// can say how many there are and nothing else.
#[cfg(feature = "vst3")]
#[test]
fn the_state_is_the_plugins_own_bytes() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("vst3.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node");

    let state = node.data.get("state").and_then(|state| state.as_str()).expect("there is a state");
    let bytes = craftkit_sdk::bytes::decode(state).expect("the state is base64");
    assert!(!bytes.is_empty(), "the plugin saved something");

    // The plugin is asked what it makes of the state the document holds,
    // and it says how many bytes there are: the same number.
    let outcome = session.execute("vst3.state", &json!({ "node": node.id })).expect("the state is read");
    assert_eq!(
        outcome.data.get("state_bytes").and_then(|bytes| bytes.as_u64()),
        Some(bytes.len() as u64),
        "the plugin reads the same bytes the document holds"
    );
}

/// A **parameter** moves, and the state follows.
///
/// The parameter is named, because that is what the format names it by,
/// and the value is the one a person sees.
#[cfg(feature = "vst3")]
#[test]
fn a_parameter_moves_and_the_state_follows() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("vst3.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node");

    let outcome = session
        .execute("vst3.parameter.set", &json!({ "node": node.id, "parameter": "gain", "value": 1.0 }))
        .expect("the parameter is set");
    let params = outcome.data.get("params").and_then(|params| params.as_array());
    let params = params.expect("there are parameters");
    let gain = params.iter().find(|param| param.get("id").and_then(|id| id.as_str()) == Some("gain"));
    let gain = gain.expect("there is a gain");
    // The gain runs from nothing to twice as much, so half of that is
    // one, and the value the plugin holds is between zero and one.
    assert!((gain.get("plain").and_then(|plain| plain.as_f64()).unwrap_or(0.0) - 1.0).abs() < 0.001);
    assert!((gain.get("value").and_then(|value| value.as_f64()).unwrap_or(0.0) - 0.5).abs() < 0.001);
}

/// The **audio** goes through the plugin, and the document says what
/// came out.
#[cfg(feature = "vst3")]
#[test]
fn the_audio_goes_through_the_plugin() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("vst3.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node");

    let outcome = session
        .execute("vst3.process", &json!({ "node": node.id, "input": [0.25, 0.5, -0.25, 1.0] }))
        .expect("the block is processed");
    let output = outcome.data.get("output").and_then(|output| output.as_array());
    let output = output.expect("there is an output");
    assert_eq!(output.len(), 4, "four samples come out");
    // The gain starts at one, so what goes in is what comes out.
    let first = output[0].as_f64().unwrap_or(0.0);
    assert!((first - 0.25).abs() < 0.0001, "the first sample is what went in");
}

/// A library that is not a plugin is **refused**, and the refusal says
/// what went wrong.
#[cfg(feature = "vst3")]
#[test]
fn a_library_that_is_not_a_plugin_is_refused() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    let error = session
        .execute("vst3.attach", &json!({ "path": "/nonexistent/libcraftkit_nothing.so" }))
        .expect_err("the library is refused");
    assert!(error.to_string().contains("vst3"), "{error}");
}

/// A plugin is **detached**, and the node goes with it.
#[cfg(feature = "vst3")]
#[test]
fn a_plugin_is_detached() {
    let mut session = craftkit_studio::ecosystem().expect("the ecosystem is built").session();
    session.execute("vst3.attach", &json!({ "path": library_path() })).expect("the plugin is attached");
    let node = session.document().nodes.first().expect("there is a node");
    session.execute("vst3.detach", &json!({ "node": node.id })).expect("the plugin is detached");
    assert!(session.document().nodes.is_empty(), "and the node is gone");
}
