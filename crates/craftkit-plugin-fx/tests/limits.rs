//! The **limits of the fx battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A comp with a layer in it, and the layer's id.
fn a_layer(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    outcome.data["id"].as_u64().expect("the layer has an id")
}

/// An **expression** over `MAX_CHARS` long is refused.
///
/// The expression is parsed before anything is written, and a length is
/// checked before the expression is looked at: an expression of a
/// thousand characters and more is not an expression anybody meant.
#[test]
fn an_expression_over_the_limit_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let expression = "1".repeat(1025);
    let error = session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "position", "expression": expression }),
        )
        .expect_err("that expression is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}
