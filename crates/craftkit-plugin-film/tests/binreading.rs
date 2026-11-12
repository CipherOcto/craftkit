//! **Reading** a bin.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A **bin**, and its id.
fn a_bin(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    outcome.data["id"].as_u64().expect("the bin has an id")
}

/// The **answer** of a read.
fn read(session: &mut Session, bin: u64) -> Json {
    let outcome = session.execute("film.bin.get", &json!({ "bin": bin })).expect("the bin is read");
    outcome.data.clone()
}

/// A bin may be **read whole**.
#[test]
fn a_bin_may_be_read_whole() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let answer = read(&mut session, bin);
    assert_eq!(answer["id"], json!(bin), "the bin is named by its id");
    assert_eq!(answer["name"], Json::String("the bin".to_string()), "and by its name");
    assert_eq!(answer["items"], json!([item]), "and it holds the item");
    assert_eq!(answer["bins"], json!([]), "and no bin");
    assert_eq!(answer["parent"], Json::Null, "and it is in no bin");
}

/// A bin may hold a **bin**, and the bin in it is read as one.
#[test]
fn a_bin_may_hold_a_bin() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let outcome = session
        .execute("film.bin.add", &json!({ "name": "the inner bin", "parent": bin }))
        .expect("the bin is added");
    let inner = outcome.data["id"].as_u64().expect("the bin has an id");
    let answer = read(&mut session, bin);
    assert_eq!(answer["bins"], json!([inner]), "the bin in the bin is said");
    let inner_answer = read(&mut session, inner);
    assert_eq!(inner_answer["parent"], json!(bin), "and the bin in the bin knows where it is");
}

/// A bin that is **not there** is refused, and so is a node that is
/// not a bin.
#[test]
fn a_bin_that_is_not_there_is_refused() {
    let mut session = session();
    let bin = a_bin(&mut session);
    let error = session.execute("film.bin.get", &json!({ "bin": 999 })).expect_err("the bin is refused");
    assert!(error.to_string().contains("there is no bin"), "the refusal says why: {error}");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let error = session.execute("film.bin.get", &json!({ "bin": item })).expect_err("the item is not a bin");
    assert!(error.to_string().contains("not a bin"), "the refusal says why: {error}");
}
