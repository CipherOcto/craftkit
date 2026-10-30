//! The art battery: the document model of an **artist's IDE**.
//!
//! Three programs in one: a video editor whose elements play **media
//! tokens**, a 3D stage the application does not model at all, and a
//! generation queue whose work comes back **hours later** under the
//! provider's own id.

use craftkit_sdk::Ecosystem;
// The tests that use it are the ones for the battery, and the battery is
// optional: the import is with them.
#[cfg(feature = "art")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A project with a main scene, a video track and two elements.
#[cfg(feature = "art")]
fn a_project() -> (craftkit_sdk::Session, u64) {
    let mut session = ecosystem().session();
    session
        .execute("art.project.add", &json!({ "name": "the film", "fps": 24 }))
        .expect("the project is added");
    let project = 1;
    session
        .execute("art.scene.add", &json!({ "project": project, "name": "act one", "main": true }))
        .expect("the scene is added");
    let scene = 2;
    session
        .execute("art.track.video.add", &json!({ "scene": scene, "name": "V1" }))
        .expect("the track is added");
    let track = 3;
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "the shot", "start": 48, "duration": 100, "media": "m_1" }),
        )
        .expect("the element is added");
    (session, track)
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "art")]
#[test]
fn the_art_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("art.project.add").is_some(), "the battery is in");
    assert!(ecosystem.commands.get("art.element.video.add").is_some());
    assert!(ecosystem.kinds.get("art.element.video").is_some());
    assert!(ecosystem.kinds.get("art.track.audio").is_some());
    assert!(ecosystem.evaluators.get("art.timeline").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "art"))]
#[test]
fn the_art_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("art.project.add").is_none(), "the battery is out");
}

/// A closed union of element types becomes **kinds**, and the kinds are
/// open: every one of them is its own kind.
#[cfg(feature = "art")]
#[test]
fn a_closed_union_becomes_kinds() {
    let ecosystem = ecosystem();
    for kind in [
        "art.element.video",
        "art.element.image",
        "art.element.audio",
        "art.element.text",
        "art.element.sticker",
        "art.element.graphic",
        "art.element.effect",
    ] {
        assert!(ecosystem.kinds.get(kind).is_some(), "{kind} is a kind");
    }
    for kind in
        ["art.track.video", "art.track.text", "art.track.audio", "art.track.graphic", "art.track.effect"]
    {
        assert!(ecosystem.kinds.get(kind).is_some(), "{kind} is a kind");
    }
}

/// An element plays **media**: a token, not a file.
#[cfg(feature = "art")]
#[test]
fn an_element_plays_a_token_not_a_file() {
    let (session, _) = a_project();
    let element = session.document().node(4).expect("the element is there");
    assert_eq!(element.kind, "art.element.video");
    assert_eq!(element.data["media"], json!("m_1"), "the media is a token");
    assert_eq!(element.data["start"], json!(48), "where it starts on the timeline");
    assert_eq!(element.data["duration"], json!(100), "how long it is");
}

/// The media is **not** an asset: the asset list is for files the document
/// owns, and a token is a pointer into somebody else's library.
#[cfg(feature = "art")]
#[test]
fn the_media_is_not_an_asset() {
    let (session, _) = a_project();
    assert!(
        session.document().assets.is_empty(),
        "the document owns no files: {:?}",
        session.document().assets
    );
}

/// The order of a timeline is **not** the order of the tree.
#[cfg(feature = "art")]
#[test]
fn the_order_of_a_timeline_is_not_the_order_of_the_tree() {
    let (mut session, track) = a_project();
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "the earlier shot", "start": 0, "duration": 24 }),
        )
        .expect("the element is added");
    let outcome = session.execute("art.timeline", &json!({})).expect("the timeline is read");
    let elements = outcome.data["scenes"][0]["tracks"][0]["elements"].as_array().expect("there are elements");
    assert_eq!(elements.len(), 2);
    // The tree has them in the order they were made: the element at 48
    // came first. The timeline has them in the order they start.
    assert_eq!(elements[0]["start"], json!(0), "the timeline is in order");
    assert_eq!(elements[1]["start"], json!(48));
}

/// The length of a scene is **derived**: it is the end of its last
/// element.
#[cfg(feature = "art")]
#[test]
fn the_length_of_a_scene_is_derived() {
    let (session, _) = a_project();
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline.len(), 2, "the scene and the project: {plan:?}");
    assert_eq!(timeline[0].value["end"], json!(148), "48 + 100");
    assert_eq!(timeline[1].value["end"], json!(148), "the project is as long as its main scene");
}

/// And it changes when an element moves.
#[cfg(feature = "art")]
#[test]
fn the_length_changes_when_an_element_moves() {
    let (mut session, track) = a_project();
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "the late shot", "start": 200, "duration": 24 }),
        )
        .expect("the element is added");
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "art.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(224), "the longest element wins");
}

/// The 3D stage is **one opaque file**, and the document says which file.
#[cfg(feature = "art")]
#[test]
fn the_stage_is_one_opaque_file() {
    let mut session = ecosystem().session();
    session
        .execute("art.stage.add", &json!({ "name": "the room", "path": "scenes/room.json" }))
        .expect("the stage is added");
    let stage = session.document().node(1).expect("the stage is there");
    assert_eq!(stage.kind, "art.stage");
    assert_eq!(stage.stream.as_deref(), Some("scenes/room.json"), "the scene is a file beside the document");
}

/// A stage that says no file is refused.
#[cfg(feature = "art")]
#[test]
fn a_stage_that_says_no_file_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("art.stage.add", &json!({ "name": "the room", "path": "  " }))
        .expect_err("the stage says nothing");
    assert!(error.to_string().contains("which file"), "{error}");
}

/// A generation is a **task**, and the provider's own id is its handle.
#[cfg(feature = "art")]
#[test]
fn a_generation_is_a_task_with_a_handle() {
    let mut session = ecosystem().session();
    let outcome = session
        .execute("art.generate", &json!({ "model": "artcraft.image", "handle": "jinf_7" }))
        .expect("the generation is started");
    assert_eq!(outcome.data["handle"], json!("jinf_7"), "the provider's own id");
    let task = outcome.data["task"].as_u64().expect("the task is numbered");
    let stored = session.tasks().task(task).expect("the task is there");
    assert_eq!(stored.kind, "artcraft.generate");
    assert_eq!(stored.handle.as_deref(), Some("jinf_7"));
}

/// And it is finished **from outside**, by the provider's own id.
#[cfg(feature = "art")]
#[test]
fn a_generation_is_finished_from_outside() {
    let mut session = ecosystem().session();
    session
        .execute("art.generate", &json!({ "model": "artcraft.image", "handle": "jinf_7" }))
        .expect("the generation is started");
    session.execute("art.job.progress", &json!({ "task": 1, "percent": 40 })).expect("the progress is set");
    session
        .execute("art.job.finish", &json!({ "handle": "jinf_7", "media": "m_9" }))
        .expect("the job is finished");
    let task = session.tasks().task(1).expect("the task is there");
    assert!(task.state.is_finished(), "the task is finished");
    assert_eq!(task.result.as_ref().expect("there is a result")["media"], json!("m_9"));
}

/// A job that fails is failed by the provider's own id too.
#[cfg(feature = "art")]
#[test]
fn a_generation_that_fails_is_failed_from_outside() {
    let mut session = ecosystem().session();
    session
        .execute("art.generate", &json!({ "model": "artcraft.image", "handle": "jinf_8" }))
        .expect("the generation is started");
    session
        .execute("art.job.fail", &json!({ "handle": "jinf_8", "message": "the model is gone" }))
        .expect("the job fails");
    let task = session.tasks().task(1).expect("the task is there");
    assert!(task.state.is_finished(), "the task is finished");
    assert_eq!(task.message.as_deref(), Some("the model is gone"));
}

/// A job that comes back under an id nobody knows is refused.
#[cfg(feature = "art")]
#[test]
fn a_job_under_an_unknown_id_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute("art.job.finish", &json!({ "handle": "jinf_nobody", "media": "m_9" }))
        .expect_err("there is no such job");
    assert!(error.to_string().contains("no task with the handle"), "{error}");
}

/// An element that is zero ticks long is refused.
#[cfg(feature = "art")]
#[test]
fn an_element_that_is_zero_ticks_long_is_refused() {
    let (mut session, track) = a_project();
    let error = session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "nothing", "start": 0, "duration": 0 }),
        )
        .expect_err("the element has no length");
    assert!(error.to_string().contains("zero ticks"), "{error}");
}

/// A fractional number of ticks is refused: the time base is whole.
#[cfg(feature = "art")]
#[test]
fn a_fractional_number_of_ticks_is_refused() {
    let (mut session, track) = a_project();
    let error = session
        .execute(
            "art.element.video.add",
            &json!({ "track": track, "name": "half a tick", "start": 0.5, "duration": 10 }),
        )
        .expect_err("a tick is a whole number");
    assert!(error.to_string().contains("whole number"), "{error}");
}

/// An element on a track that is not there is refused.
#[cfg(feature = "art")]
#[test]
fn an_element_on_a_track_that_is_not_there_is_refused() {
    let mut session = ecosystem().session();
    let error = session
        .execute(
            "art.element.video.add",
            &json!({ "track": 999, "name": "nothing", "start": 0, "duration": 10 }),
        )
        .expect_err("the track is not there");
    assert!(error.to_string().contains("no track"), "{error}");
}

/// The undo takes the elements with it.
#[cfg(feature = "art")]
#[test]
fn the_undo_takes_the_elements_with_it() {
    let (mut session, _) = a_project();
    let before = session.document().walk().count();
    session
        .execute(
            "art.element.video.add",
            &json!({ "track": 3, "name": "another", "start": 0, "duration": 24 }),
        )
        .expect("the element is added");
    assert_eq!(session.document().walk().count(), before + 1);
    session.undo().expect("the undo works");
    assert_eq!(session.document().walk().count(), before, "the element is gone");
}

/// A directory of this test's own, so that the files do not mix.
#[cfg(feature = "art")]
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-art-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **plugin's own file** travels in the bundle.
///
/// This is the gap §21.2 listed: a plugin that keeps a file of its own
/// had no way to say which file it was. It does now — the file is the
/// node's **stream**, and the bundle carries the streams.
#[cfg(feature = "art")]
#[test]
fn a_plugins_own_file_travels_in_the_bundle() {
    let from = directory("stage");
    let into = directory("stage-into");
    let mut session = ecosystem().session();
    session
        .execute("art.stage.add", &json!({ "name": "the room", "path": "scenes/room.json" }))
        .expect("the stage is added");
    std::fs::create_dir_all(std::path::Path::new(&from).join("scenes")).expect("the folder is made");
    std::fs::write(std::path::Path::new(&from).join("scenes/room.json"), b"{\"version\":3}")
        .expect("the scene is written");

    let bundle = format!("{from}/the-film.bundle");
    session
        .execute("doc.save.bundle", &json!({ "path": bundle, "from": from }))
        .expect("the bundle is written");

    let mut opened = ecosystem().session();
    opened
        .execute("doc.open.bundle", &json!({ "path": bundle, "into": into }))
        .expect("the bundle is opened");
    let stage = opened.document().node(1).expect("the stage is there");
    assert_eq!(stage.kind, "art.stage");
    assert_eq!(stage.stream.as_deref(), Some("scenes/room.json"), "the scene is still a file");
    let bytes =
        std::fs::read(std::path::Path::new(&into).join("scenes/room.json")).expect("the file is there");
    assert_eq!(bytes, b"{\"version\":3}", "the bytes came back byte for byte");
}
