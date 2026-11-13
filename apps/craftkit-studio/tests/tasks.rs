//! **Tasks**: work that may outlive the process.
//!
//! A generation is enqueued with a provider, gets the provider's own id,
//! is polled, is written down so that it survives the application being
//! closed, and is finished by an event that arrives from outside. None of
//! that is a job in a pool: the pool owns its work, and the work is over
//! when the worker returns.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with a task registered in it.
fn with_a_task() -> (Session, u64) {
    let mut session = ecosystem().session();
    let outcome =
        session.execute("task.add", &json!({ "kind": "artcraft.generate" })).expect("the task is registered");
    let id = outcome.data["task"].as_u64().expect("the task is numbered");
    (session, id)
}

/// A task is registered, and it is pending.
#[test]
fn a_task_is_registered_and_it_is_pending() {
    let (mut session, id) = with_a_task();
    let outcome = session.execute("task.list", &json!({})).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"][0]["id"], json!(id));
    assert_eq!(outcome.data["tasks"][0]["state"], json!("pending"));
    assert_eq!(outcome.data["tasks"][0]["kind"], json!("artcraft.generate"));
}

/// A task moves: pending, running, done.
#[test]
fn a_task_moves_from_pending_to_running_to_done() {
    let (mut session, id) = with_a_task();
    session.execute("task.start", &json!({ "task": id })).expect("the task starts");
    session.execute("task.progress", &json!({ "task": id, "percent": 40 })).expect("the progress is set");
    session
        .execute("task.complete", &json!({ "task": id, "result": { "url": "https://example/x" } }))
        .expect("the task is done");
    let outcome = session.execute("task.list", &json!({})).expect("the tasks are read");
    let task = &outcome.data["tasks"][0];
    assert_eq!(task["state"], json!("done"));
    assert_eq!(task["progress"], json!(100));
    assert_eq!(task["result"]["url"], json!("https://example/x"));
}

/// A task may be finished **without this process having started it**.
///
/// That is the whole point: the event that says it is done may come from
/// a server, from another process, or from a timer.
#[test]
fn a_task_may_be_finished_from_outside() {
    let (mut session, id) = with_a_task();
    session
        .execute("task.complete", &json!({ "task": id, "result": "from outside" }))
        .expect("the task is done");
    let outcome = session.execute("task.list", &json!({})).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"][0]["state"], json!("done"));
}

/// The other end's own id is kept: it is the thing to ask about.
#[test]
fn the_other_ends_own_id_is_kept() {
    let (mut session, id) = with_a_task();
    session
        .execute("task.attach", &json!({ "task": id, "handle": "provider-42" }))
        .expect("the handle is attached");
    let outcome = session.execute("task.list", &json!({})).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"][0]["handle"], json!("provider-42"));
}

/// A finished task does not move again.
#[test]
fn a_finished_task_does_not_move_again() {
    let (mut session, id) = with_a_task();
    session.execute("task.complete", &json!({ "task": id, "result": null })).expect("the task is done");
    let error = session.execute("task.start", &json!({ "task": id })).expect_err("the task is finished");
    assert!(error.to_string().contains("not pending"), "{error}");
}

/// The list of tasks **survives the process**.
///
/// This is what makes a task that outlives the program possible: the
/// list is written down, and a program that is closed and opened again
/// finds the tasks it left behind.
#[test]
fn the_list_of_tasks_survives_the_process() {
    let path = craftkit_scratch::file("craftkit-v2-tasks.json").expect("the scratch is made");
    let path = path.to_string_lossy().to_string();
    let (mut session, id) = with_a_task();
    session.execute("task.start", &json!({ "task": id })).expect("the task starts");
    session
        .execute("task.attach", &json!({ "task": id, "handle": "provider-42" }))
        .expect("the handle is attached");
    session.execute("task.save", &json!({ "path": path })).expect("the tasks are written");

    // A new program: a new session, with nothing in it.
    let mut other = ecosystem().session();
    other.execute("task.load", &json!({ "path": path })).expect("the tasks are read");
    let outcome = other.execute("task.list", &json!({})).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"][0]["handle"], json!("provider-42"), "the handle came back");
    assert_eq!(outcome.data["tasks"][0]["state"], json!("running"), "the state came back");
}

/// A task that was left running can be finished **later**, by another
/// process, from what it left behind.
#[test]
fn a_task_left_running_can_be_finished_later() {
    let path = craftkit_scratch::file("craftkit-v2-tasks-later.json").expect("the scratch is made");
    let path = path.to_string_lossy().to_string();
    let (mut session, id) = with_a_task();
    session.execute("task.start", &json!({ "task": id })).expect("the task starts");
    session.execute("task.save", &json!({ "path": path })).expect("the tasks are written");

    // Another program, later: it reads the list and finishes the task.
    let mut later = ecosystem().session();
    later.execute("task.load", &json!({ "path": path })).expect("the tasks are read");
    later
        .execute("task.complete", &json!({ "task": id, "result": "finished elsewhere" }))
        .expect("the task is finished");
    let outcome = later.execute("task.list", &json!({})).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"][0]["state"], json!("done"));
}

/// The tasks can be asked for by state.
#[test]
fn the_tasks_can_be_asked_for_by_state() {
    let (mut session, id) = with_a_task();
    session.execute("task.start", &json!({ "task": id })).expect("the task starts");
    let outcome = session.execute("task.list", &json!({ "state": "running" })).expect("the tasks are read");
    assert_eq!(outcome.data["tasks"].as_array().map(|tasks| tasks.len()), Some(1));
    let none = session.execute("task.list", &json!({ "state": "done" })).expect("the tasks are read");
    assert_eq!(none.data["tasks"].as_array().map(|tasks| tasks.len()), Some(0));
}
