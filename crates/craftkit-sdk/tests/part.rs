//! The **part**: the piece of a node that is picked out.
//!
//! A selection may hold a part of a node, and a part is in the node's
//! **own units**: samples for a clip in a sound document, characters for
//! a paragraph. The core does not know what those units are, so it asks
//! the kind — and the answer may be more than one piece.

use craftkit_sdk::command::{CommandRun, CommandSpec};
use craftkit_sdk::contribution::Registry;
use craftkit_sdk::document::{NodeKindSpec, RecordingCanvas, Rect};
use craftkit_sdk::{Node, Outcome, Selected, Session};

/// A kind that says where its picked-out part is: **two** pieces,
/// because a part may be in more than one.
fn a_kind() -> NodeKindSpec {
    NodeKindSpec {
        kind: "test.part",
        label: "Part",
        layer: "core",
        paint: |_node, _document, _canvas| {},
        hit: |_node, _document, _point| false,
        evaluate: None,
        place: None,
        part: Some(|_node, _document, from, to| {
            Some(vec![Rect::new(from, 0.0, to - from, 10.0), Rect::new(from, 20.0, to - from, 10.0)])
        }),
    }
}

/// A kind that says **nothing** about its part.
fn a_silent_kind() -> NodeKindSpec {
    NodeKindSpec {
        kind: "test.silent",
        label: "Silent",
        layer: "core",
        paint: |_node, _document, _canvas| {},
        hit: |_node, _document, _point| false,
        evaluate: None,
        place: None,
        part: None,
    }
}

/// A command that adds a node of that kind.
///
/// The document is handed out only to a command, so a test that wants a
/// node has to **ask** for one — the same way any other caller does.
fn an_add_command(id: &'static str, kind: &'static str) -> CommandSpec {
    CommandSpec {
        id,
        label: "Add",
        area: "test",
        capability: "command",
        layer: "core",
        icon: "plus",
        doc: "Add a node.",
        shortcut: None,
        menu: &["test"],
        params: &[],
        mutates: true,
        example: false,
        undoable: true,
        journal: true,
        enabled: craftkit_sdk::always,
        run: CommandRun::Dynamic(std::sync::Arc::new(move |session, _params| {
            let id = session.document_mut()?.add_node("node", kind, Some(Rect::new(0.0, 0.0, 100.0, 100.0)));
            Ok(Outcome::changed(format!("added {id}"))
                .with_data(craftkit_sdk::serde_json::json!({ "id": id })))
        })),
    }
}

/// A session with those two kinds in it, and the commands that put a
/// node of either kind in the document.
fn session() -> Session {
    let mut kinds: Registry<NodeKindSpec> = Registry::new();
    kinds.register(a_kind()).expect("the kind is registered");
    kinds.register(a_silent_kind()).expect("the kind is registered");
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands.register(an_add_command("test.add.part", "test.part")).expect("the command is registered");
    commands.register(an_add_command("test.add.silent", "test.silent")).expect("the command is registered");
    Session::with_formats(commands, kinds, Registry::new(), Registry::new(), Registry::new())
}

/// A node of that kind, at that place.
fn a_node(session: &mut Session, kind: &str) -> u64 {
    let command = if kind == "test.silent" { "test.add.silent" } else { "test.add.part" };
    let outcome = session.execute(command, &craftkit_sdk::serde_json::json!({})).expect("the node is added");
    outcome.data.get("id").and_then(|id| id.as_u64()).expect("the node has an id")
}

/// A picked-out **part** is washed over, in as many pieces as the kind
/// says there are — and it has no handle, because a part cannot be
/// resized.
#[test]
fn a_picked_out_part_is_washed_over() {
    let mut session = session();
    let id = a_node(&mut session, "test.part");
    session.select(&[Selected::span(id, 10.0, 30.0)]);
    let mut canvas = RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn == "rect(10, 0, 20, 10)"),
        "the first piece is washed over: {:?}",
        canvas.drawn
    );
    assert!(
        canvas.drawn.iter().any(|drawn| drawn == "rect(10, 20, 20, 10)"),
        "and so is the second: {:?}",
        canvas.drawn
    );
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn.starts_with("path(")),
        "a part is not framed: {:?}",
        canvas.drawn
    );
}

/// A whole node is **framed**, and it has a handle.
#[test]
fn a_whole_node_is_framed_and_has_a_handle() {
    let mut session = session();
    let id = a_node(&mut session, "test.part");
    session.select(&[Selected::node(id)]);
    let mut canvas = RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("path(")),
        "the frame is drawn: {:?}",
        canvas.drawn
    );
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("rect(")),
        "and so is the handle: {:?}",
        canvas.drawn
    );
}

/// A kind that cannot place its part leaves the **frame** to say that
/// something is picked out.
#[test]
fn a_kind_that_cannot_place_its_part_leaves_the_frame() {
    let mut session = session();
    let id = a_node(&mut session, "test.silent");
    session.select(&[Selected::span(id, 10.0, 30.0)]);
    let mut canvas = RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("path(")),
        "the frame is drawn: {:?}",
        canvas.drawn
    );
}

/// The **picture of the document** has no selection in it: the selection
/// is chrome, and chrome belongs to the screen.
#[test]
fn a_picture_of_the_document_has_no_selection_in_it() {
    let mut session = session();
    let id = a_node(&mut session, "test.part");
    session.select(&[Selected::span(id, 10.0, 30.0)]);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn.starts_with("rect(")),
        "the wash is not in the picture: {:?}",
        canvas.drawn
    );
}

/// The core asks the **kind**, and the kind's answer is the parts.
#[test]
fn the_core_asks_the_kind_where_its_part_is() {
    let mut session = session();
    let id = a_node(&mut session, "test.part");
    let node: &Node = session.document().node(id).expect("the node is there");
    let parts = session.parts_of(node, 5.0, 9.0);
    assert_eq!(parts.len(), 2, "the kind said two pieces");
    assert_eq!(parts[0].x, 5.0);
    assert_eq!(parts[0].w, 4.0);
    assert_eq!(parts[1].y, 20.0);
}

/// A kind that says nothing has **no** part.
#[test]
fn a_kind_that_says_nothing_has_no_part() {
    let mut session = session();
    let id = a_node(&mut session, "test.silent");
    let node: &Node = session.document().node(id).expect("the node is there");
    assert!(session.parts_of(node, 5.0, 9.0).is_empty());
}

/// The core never matches on a kind to work out where a part is: it
/// asks, the way it asks where the node is.
#[test]
fn the_core_does_not_match_on_the_kind() {
    let session = session();
    let kinds = session.kinds();
    assert!(kinds.get("test.part").expect("the kind is there").part.is_some());
    assert!(kinds.get("test.silent").expect("the kind is there").part.is_none());
}
