//! The **kinds** of the art battery: what a project and a scene mean.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_art::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the art battery.
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

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **project** is a kind, and it means its scenes and its frame rate.
#[test]
fn a_project_means_its_scenes_and_its_frame_rate() {
    let mut session = session();
    let outcome = session
        .execute("art.project.add", &json!({ "name": "the film", "fps": 24 }))
        .expect("the project is added");
    let project = outcome.data["id"].as_u64().expect("the project has an id");
    let node = session.document().node(project).expect("the project is there");
    assert_eq!(node.kind, "art.project", "the node is a project");
    let said = said_of(&session, project);
    assert_eq!(said["fps"], json!(24), "the project says how fast it runs");
    assert_eq!(said["scenes"], json!(0), "and how many scenes it holds");
}

/// A **scene** is a kind, and it means its tracks and whether it is the
/// **main** one — the one the project's length comes from.
#[test]
fn a_scene_means_its_tracks_and_whether_it_is_the_main_one() {
    let mut session = session();
    let outcome =
        session.execute("art.project.add", &json!({ "name": "the film" })).expect("the project is added");
    let project = outcome.data["id"].as_u64().expect("the project has an id");
    let outcome = session
        .execute("art.scene.add", &json!({ "project": project, "name": "act one", "main": true }))
        .expect("the scene is added");
    let scene = outcome.data["id"].as_u64().expect("the scene has an id");
    let node = session.document().node(scene).expect("the scene is there");
    assert_eq!(node.kind, "art.scene", "the node is a scene");
    let said = said_of(&session, scene);
    assert_eq!(said["tracks"], json!(0), "the scene says how many tracks it holds");
    assert_eq!(said["main"], json!(true), "and whether it is the main one");
}
