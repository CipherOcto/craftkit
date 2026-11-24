//! The **routing**: the buses, in the order they are fed in.
//!
//! The routing is derived: the document holds the edges, and the buses
//! are what they mean. Nothing stores the order, and nothing has to keep
//! it true.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// The **routing** the plan reports.
fn routing_of(session: &Session) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "sound.routing").expect("the plan has the routing");
    item.value["buses"].clone()
}

/// A session with one **bus** in it.
fn with_a_bus(session: &mut Session) -> u64 {
    let outcome = session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
    outcome.data["id"].as_u64().expect("the bus has an id")
}

/// The routing of a session with **no** track is empty: there is nothing
/// to route.
#[test]
fn a_session_with_no_track_has_no_routing() {
    let session = session();
    let routing = routing_of(&session);
    assert_eq!(routing.as_array().map(|buses| buses.len()), Some(0), "there are no buses: {routing}");
}

/// And a session with a bus has that bus in its routing: the buses are
/// what the routing is, and the order they are fed in is what the
/// routing says.
#[test]
fn a_bus_is_in_the_routing() {
    let mut session = session();
    let bus = with_a_bus(&mut session);
    let routing = routing_of(&session);
    let buses = routing.as_array().expect("there are buses");
    assert_eq!(buses.len(), 1, "the bus is in the routing: {routing}");
    assert_eq!(buses[0]["id"], json!(bus), "and it is that bus");
    assert_eq!(buses[0]["name"], json!("reverb"));
}
