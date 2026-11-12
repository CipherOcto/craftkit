//! The **time** of a composition.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the fx battery.
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

/// A **composition**, and its id.
fn a_comp(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    outcome.data["id"].as_u64().expect("the comp has an id")
}

/// The **frame** the time is at.
fn frame(session: &mut Session, comp: u64) -> u64 {
    let outcome = session.execute("fx.time.get", &json!({ "comp": comp })).expect("the time is read");
    outcome.data["frame"].as_u64().expect("the time has a frame")
}

/// The time may be **set** and **read**.
#[test]
fn the_time_may_be_set_and_read() {
    let mut session = session();
    let comp = a_comp(&mut session);
    assert_eq!(frame(&mut session, comp), 0, "the time begins at the start");
    session.execute("fx.time.set", &json!({ "comp": comp, "frame": 12 })).expect("the time is set");
    assert_eq!(frame(&mut session, comp), 12, "and it is where it was put");
    session.execute("fx.time.set", &json!({ "comp": comp, "frame": -3 })).expect("the time is set again");
    assert_eq!(frame(&mut session, comp), 0, "and a time before the start is the start");
    assert!(
        session.execute("fx.time.set", &json!({ "comp": comp, "frame": 1.5 })).is_err(),
        "and a frame that is not a whole number is refused"
    );
}

/// The time may go to the **start** and to the **end**.
#[test]
fn the_time_may_go_to_the_start_and_to_the_end() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session.execute("fx.time.set", &json!({ "comp": comp, "frame": 20 })).expect("the time is set");
    session.execute("fx.time.end", &json!({ "comp": comp })).expect("the time goes to the end");
    assert_eq!(frame(&mut session, comp), 100, "the end is the end of the composition");
    session.execute("fx.time.start", &json!({ "comp": comp })).expect("the time goes to the start");
    assert_eq!(frame(&mut session, comp), 0, "and the start is the first frame");
}

/// The time may be **stepped**.
#[test]
fn the_time_may_be_stepped() {
    let mut session = session();
    let comp = a_comp(&mut session);
    session.execute("fx.time.set", &json!({ "comp": comp, "frame": 10 })).expect("the time is set");
    session.execute("fx.time.step", &json!({ "comp": comp, "frames": 5 })).expect("the time is stepped");
    assert_eq!(frame(&mut session, comp), 15, "five frames on");
    session
        .execute("fx.time.step", &json!({ "comp": comp, "frames": -20 }))
        .expect("the time is stepped back");
    assert_eq!(frame(&mut session, comp), 0, "and a step past the start stops at the start");
}
