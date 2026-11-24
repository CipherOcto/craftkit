//! The **wire protocol** of a subprocess plugin, and what it carries.
//!
//! A plugin in another process has no Rust types: it sends a **catalogue**
//! of names and kinds, and the host reads it. So what the catalogue says
//! about a parameter is all the host knows — and if the catalogue cannot
//! say what a parameter takes, the host has to guess, and a guess is a
//! place where the plugin and the host can disagree.
//!
//! These tests say what the catalogue carries: the kind, the kind's
//! **leniency** (a length is a number, and `25mm` is a length too), and
//! the **other spelling's** kind and leniency for a parameter that has
//! two spellings.

#![forbid(unsafe_code)]

use craftkit_sdk::Doc;
use craftkit_sdk::plugin::PluginParam;
use craftkit_sdk::serde_json::json;

/// A parameter, as it travels.
fn param() -> PluginParam {
    PluginParam {
        name: "length".to_string(),
        kind: Doc::Number,
        required: true,
        help: "How long.".to_string(),
        alternative: Some("length_text".to_string()),
        type_name: Some("length".to_string()),
        unit: Some("mm".to_string()),
        accepts_text: true,
        alternative_kind: Some(Doc::Number),
        alternative_accepts_text: true,
    }
}

/// Everything a parameter says survives the wire.
#[test]
fn a_parameter_says_what_it_takes_across_the_wire() {
    let sent = param();
    let came = PluginParam::from_json(&sent.to_json()).expect("the parameter is read");
    assert_eq!(came, sent, "the parameter says something else on the other side");
    assert!(came.accepts_text, "the kind's leniency travels");
    assert_eq!(came.alternative_kind, Some(Doc::Number), "and so does the other spelling's kind");
    assert!(came.alternative_accepts_text, "and the other spelling's leniency too");
}

/// The wire carries the kinds by **name**, so that a peer in another
/// language can read them.
#[test]
fn the_wire_carries_the_kinds_by_name() {
    let wire = param().to_json();
    assert_eq!(wire["kind"], json!("number"));
    assert_eq!(wire["alternative_kind"], json!("number"));
    assert_eq!(wire["accepts_text"], json!(true));
    assert_eq!(wire["alternative_accepts_text"], json!(true));
}

/// An **older peer** — one whose catalogue says nothing about the
/// leniency — is read without it: the flag is false and the other
/// spelling has no kind of its own.
///
/// The host then checks what the peer said and nothing more, which is
/// the honest limit of a protocol that grew a field.
#[test]
fn an_older_peer_is_read_without_the_fields_it_does_not_have() {
    let wire = json!({
        "name": "length",
        "kind": "number",
        "required": true,
        "help": "How long.",
    });
    let came = PluginParam::from_json(&wire).expect("the parameter is read");
    assert!(!came.accepts_text, "the leniency is not guessed");
    assert_eq!(came.alternative_kind, None, "and the other spelling has no kind of its own");
    assert!(!came.alternative_accepts_text, "and no leniency either");
}
