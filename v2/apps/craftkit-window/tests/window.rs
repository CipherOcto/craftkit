//! The **window**: thin, and it says so.
//!
//! A front end reads the engine's state and acts through the registry.
//! These tests are about the window doing exactly that: the commands it
//! shows are the registry's, the document it shows is the document,
//! and the only way it changes anything is the session's one way in.

use craftkit_sdk::serde_json::json;
use craftkit_window::Window;

/// A window over the kit.
fn window() -> Window {
    Window::new().expect("the ecosystem is built")
}

/// The window shows **every command that was linked in**.
///
/// It has no list of its own: the commands it shows are the registry's,
/// and a command that is added to the registry appears without the
/// window being told.
#[test]
fn the_window_shows_every_command() {
    let window = window();
    let commands = window.commands();
    assert!(!commands.is_empty(), "there are commands");
    let ids: Vec<&str> = commands.iter().map(|command| command.id).collect();
    assert!(ids.contains(&"node.add"), "the core's own command is there: {ids:?}");
    assert!(ids.contains(&"hello.greet"), "and a plugin's: {ids:?}");
    // The commands are in order, so the window draws them the same way
    // every time.
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "the commands are in order");
}

/// The window runs a command **through the registry**.
///
/// The document changes, and the only way it changed is the session's
/// one way in.
#[test]
fn the_window_runs_a_command_through_the_registry() {
    let mut window = window();
    assert!(window.open("node.add"), "the command opens");
    assert!(window.set("name", json!("a shape")), "the name is set");
    assert!(window.set("kind", json!("geometry.rect")), "and the kind");
    let outcome = window.run().expect("the command runs");
    assert!(outcome.message.contains("a shape"), "{}", outcome.message);

    let tree = window.tree();
    assert_eq!(tree.len(), 1, "the document has a node: {tree:?}");
    assert_eq!(tree[0].name, "a shape");
    assert_eq!(tree[0].kind, "geometry.rect");
    assert_eq!(window.log().len(), 1, "and the window said so");
}

/// The window shows the **plan**: what the document means.
///
/// It is read from the session, and it is read again every time it is
/// asked for: a window that kept it would show a document that is not
/// there any more.
#[test]
fn the_window_shows_the_plan() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.run().expect("the command runs");

    let plan = window.plan();
    let about = plan.iter().find(|row| row.node == 1);
    assert!(about.is_some(), "the plan says what the node means: {plan:?}");
    assert_eq!(about.map(|row| row.kind.clone()), Some("geometry.rect".to_string()));

    // The plan is read again, and it is the same: the window keeps
    // nothing.
    assert_eq!(window.plan(), plan, "the plan is read again");
}

/// A command that is **not there** opens nothing.
#[test]
fn a_command_that_is_not_there_opens_nothing() {
    let mut window = window();
    assert!(!window.open("nothing.here"), "there is no such command");
    assert!(window.form().is_none(), "and no form was opened");
    assert!(window.run().is_err(), "and there is nothing to run");
}

/// A command that **refuses** says so, and the window says it too.
#[test]
fn a_command_that_refuses_says_so() {
    let mut window = window();
    // Grouping wants two nodes, and the document has none.
    assert!(window.open("node.group"), "the command opens");
    let error = window.run().expect_err("there is nothing to group");
    assert!(
        window.log().iter().any(|line| line.contains("refused")),
        "the window said so: {:?}",
        window.log()
    );
    assert!(error.to_string().contains("group"), "{error}");
}

/// The window **draws** the document, through the kinds.
///
/// Every node is offered to the kind that says what it is, and the
/// camera is the session's — so the picture is seen through the same
/// controls every other front end uses.
#[test]
fn the_window_draws_the_document_through_the_kinds() {
    let mut window = window();
    // A rectangle with a colour of its own: the shapes plugin's own
    // rectangle is white, and a white rectangle on a white picture is
    // a rectangle nobody sees.
    window.open("vector.rectangle");
    window.set("x", json!(-5.0));
    window.set("y", json!(-5.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.set("fill", json!("#3366cc"));
    window.run().expect("the command runs");

    let picture = window.picture(64, 64).expect("the document is drawn");
    assert_eq!(picture.width, 64, "the picture is the size it was asked for");
    assert_eq!(picture.height, 64);
    assert_eq!(picture.pixels.len(), 64 * 64 * 4, "four bytes a pixel");

    // The shape is **in** the picture: some pixel carries the colour it
    // was given.
    let painted = picture.pixels.chunks(4).any(|pixel| pixel[2] > 200 && pixel[0] < 100);
    assert!(painted, "the shape was drawn in its own colour");
}

/// The window **picks** the node under the pointer.
///
/// Every node is offered to the kind that says what it is, so the hit
/// is the kind's own — and the deepest hit wins.
#[test]
fn the_window_picks_the_node_under_the_pointer() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.set("rect", json!({ "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }));
    window.run().expect("the command runs");

    let inside = window.pick(craftkit_sdk::Point::new(5.0, 5.0));
    assert_eq!(inside, Some(1), "the shape is under the pointer");
    let outside = window.pick(craftkit_sdk::Point::new(500.0, 500.0));
    assert_eq!(outside, None, "and nothing is under that");
}

/// A click **selects** through the registry.
///
/// The selection is the session's state, and the command that sets it is
/// in the registry like every other.
#[test]
fn a_click_selects_through_the_registry() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.set("rect", json!({ "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }));
    window.run().expect("the command runs");

    window.select_node(1).expect("the selection runs");
    assert_eq!(window.session().selected_ids(), vec![1], "the node is selected");
}

/// The **camera** is the session's, and it has its own caps.
///
/// The wheel can send anything, and a camera that refuses to move feels
/// broken: so a zoom outside the caps is clamped, not refused.
#[test]
fn the_camera_is_the_sessions_and_it_clamps() {
    let mut window = window();
    window.zoom_by(2.0);
    assert_eq!(window.session().camera().zoom(), 2.0, "the zoom moved");

    window.pan(3.0, -4.0);
    let offset = window.session().camera().offset();
    assert_eq!(offset.x, 3.0, "the camera moved across");
    assert_eq!(offset.y, -4.0, "and down");

    window.zoom_by(1_000_000.0);
    assert_eq!(window.session().camera().zoom(), craftkit_sdk::Camera::MAX_ZOOM, "and it is capped");
}

/// A node **moves**, through the registry.
///
/// The node keeps its size: a move is a move, and a command that
/// changed the size as well would be two commands wearing one name.
#[test]
fn a_node_moves_through_the_registry() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.set("rect", json!({ "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }));
    window.run().expect("the command runs");

    window.move_node(1, 20.0, 30.0).expect("the move runs");
    let session = window.session();
    let node = session.document().node(1).expect("the node is there");
    let rect = node.rect.expect("the node has a place");
    assert_eq!(rect.x, 20.0, "the node moved across");
    assert_eq!(rect.y, 30.0, "and up");
    assert_eq!(rect.w, 10.0, "and it kept its size");
    assert_eq!(rect.h, 10.0);
}

/// The wheel zooms **about the pointer**.
///
/// The thing under the pointer stays under the pointer: a camera that
/// zooms about the middle of the screen slides the world out from under
/// the hand.
#[test]
fn the_wheel_zooms_about_the_pointer() {
    let mut window = window();
    window.pan(10.0, 10.0);
    let pointer = craftkit_sdk::Point::new(40.0, 20.0);
    let camera = *window.session().camera();
    let before = camera.to_world(pointer);

    window.zoom_at(2.0, pointer);

    let camera = *window.session().camera();
    assert_eq!(camera.zoom(), 2.0, "the zoom happened");
    let after = camera.to_world(pointer);
    assert!((after.x - before.x).abs() < 1e-9, "the world under the pointer did not move across");
    assert!((after.y - before.y).abs() < 1e-9, "and it did not move up");
}

/// The **handle** that resizes a node is the kit's own.
///
/// Every front end that resizes a node puts its handle in the same
/// corner, and the window finds it through the kit's rectangle and not
/// through a rule of its own.
#[test]
fn the_handle_is_the_kits_own() {
    let mut window = window();
    window.open("geometry.rectangle");
    window.set("x", json!(0.0));
    window.set("y", json!(0.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.run().expect("the rectangle is added");

    let handle = craftkit_sdk::Point::new(10.0, 0.0);
    assert_eq!(window.handle_at(handle, 0.5), Some(1), "the handle is under the pointer");
    assert_eq!(window.handle_at(craftkit_sdk::Point::new(5.0, 5.0), 0.5), None, "and not elsewhere");
}

/// A node **resizes**, through the registry.
///
/// The node keeps its place: a resize changes the size and not where
/// the node is.
#[test]
fn a_node_resizes_through_the_registry() {
    let mut window = window();
    window.open("geometry.rectangle");
    window.set("x", json!(0.0));
    window.set("y", json!(0.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.run().expect("the rectangle is added");

    window.resize_node(1, 20.0, 30.0).expect("the resize runs");
    let session = window.session();
    let rect = session.document().node(1).and_then(|node| node.rect).expect("it has a place");
    assert_eq!(rect.x, 0.0, "the node did not move across");
    assert_eq!(rect.y, 0.0, "and it did not move up");
    assert_eq!(rect.w, 20.0, "the width changed");
    assert_eq!(rect.h, 30.0, "and the height");
}

/// A **hidden** node is neither drawn nor picked.
///
/// It is in the document and not on the screen, and the window does not
/// decide that: the kit's walk of what is not hidden says so, and the
/// window draws and picks with that walk.
#[test]
fn a_hidden_node_is_neither_drawn_nor_picked() {
    let mut window = window();
    window.open("node.add");
    window.set("name", json!("a shape"));
    window.set("kind", json!("geometry.rect"));
    window.set("rect", json!({ "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }));
    window.run().expect("the command runs");

    assert_eq!(window.pick(craftkit_sdk::Point::new(5.0, 5.0)), Some(1), "the shape is picked");

    window.run_id("node.hide", json!({ "node": 1 })).expect("the node is hidden");
    assert_eq!(window.pick(craftkit_sdk::Point::new(5.0, 5.0)), None, "a hidden node is not picked");

    // And the tree still says it is there: it is in the document.
    let rows = window.tree();
    assert_eq!(rows.len(), 1, "the node is in the document");
    assert!(rows[0].hidden, "and the tree says it is hidden");
}

/// The **handle** is drawn where the drag finds it.
///
/// A handle that is drawn somewhere else is a handle that lies, and the
/// kit draws it: the session paints the document and the handle of the
/// selected node with it.
#[test]
fn the_handle_is_drawn_where_the_drag_finds_it() {
    let mut window = window();
    window.open("geometry.rectangle");
    window.set("x", json!(0.0));
    window.set("y", json!(0.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.run().expect("the rectangle is added");
    window.select_node(1).expect("the node is selected");

    // The handle is the corner opposite the anchor.
    let handle = window.handle_at(craftkit_sdk::Point::new(10.0, 0.0), 0.5);
    assert_eq!(handle, Some(1), "the handle is under the pointer");

    // And it is **drawn**: the picture has a black pixel where the
    // handle is.
    let picture = window.picture(64, 64).expect("the document is drawn");
    let (x, y) = (32 + 10, 32);
    let pixel = &picture.pixels[(y * 64 + x) * 4..(y * 64 + x) * 4 + 4];
    assert_eq!(pixel, &[0, 0, 0, 0xff], "the handle is drawn at the corner");
}

/// The **selection** moves as one.
///
/// A drag on a node that is selected moves the whole of the selection:
/// that is what a person means when they drag one of the things they
/// picked, and the command that does it is the registry's.
#[test]
fn the_selection_moves_as_one() {
    let mut window = window();
    for name in ["one", "two"] {
        window.open("geometry.rectangle");
        window.set("name", json!(name));
        window.set("x", json!(0.0));
        window.set("y", json!(0.0));
        window.set("width", json!(10.0));
        window.set("height", json!(10.0));
        window.run().expect("the rectangle is added");
    }
    window.run_id("select.all", json!({})).expect("everything is selected");
    assert!(window.is_selected(1), "the first node is selected");

    window.move_selection(7.0, 0.0).expect("the selection moves");
    for id in [1, 2] {
        let session = window.session();
        let rect = session.document().node(id).and_then(|node| node.rect).expect("it has a place");
        assert_eq!(rect.x, 7.0, "the node moved across");
    }
}

/// The **panels** are shown, and their commands are shown once.
///
/// A panel is a battery's own window of commands: it names the
/// commands it shows, and a front end shows them together. A command
/// that is shown twice is a command with two doors.
#[test]
fn the_panels_are_shown_and_their_commands_once() {
    let window = window();
    let panels = window.panels();
    assert!(!panels.is_empty(), "there are panels");

    let mut shown: Vec<String> = Vec::new();
    for panel in &panels {
        for id in &panel.commands {
            assert!(!shown.contains(id), "the command `{id}` is shown twice");
            shown.push(id.clone());
            assert!(window.command(id).is_some(), "the command `{id}` is linked in");
        }
    }

    // And the commands that no panel shows are the rest.
    let outside: Vec<&str> = window.commands_outside_panels().iter().map(|command| command.id).collect();
    for id in &shown {
        assert!(!outside.contains(&id.as_str()), "the command `{id}` is shown twice");
    }
}

/// The **control channel** drives the window's own document.
///
/// The channel serves the window's session and not a session of its
/// own: the two are one document, and a command that an agent runs
/// over the channel changes what the window is drawing.
#[test]
fn the_control_channel_drives_the_windows_document() {
    let window = window();
    let options = craftkit_control::Options {
        token: "a-secret".to_string(),
        address: "127.0.0.1:0".parse().expect("an address"),
        announce: None,
        render: None,
    };
    let mut server =
        craftkit_control::Server::serve_shared(window.shared_session(), options).expect("the channel opens");
    let mut client =
        craftkit_control::Client::connect(server.address(), "a-secret").expect("the client connects");

    client
        .call(
            craftkit_control::Method::Execute,
            json!({
                "command": "node.add",
                "params": { "name": "a shape", "kind": "geometry.rect" },
            }),
        )
        .expect("the command runs over the channel");

    // And the window sees it: one document, two doors.
    let rows = window.tree();
    assert_eq!(rows.len(), 1, "the node is in the window's document");
    assert_eq!(rows[0].name, "a shape", "and it is the node the agent added");
    server.shutdown();
}

/// The channel can **show** what the window is showing.
///
/// The channel knows the session and the window knows the screen, so
/// the window gives the channel a way to ask for the picture. An agent
/// that changes the document can then look at the result instead of
/// believing it.
#[test]
fn the_channel_can_show_the_windows_document() {
    let mut window = window();
    window.open("vector.rectangle");
    window.set("x", json!(-5.0));
    window.set("y", json!(-5.0));
    window.set("width", json!(10.0));
    window.set("height", json!(10.0));
    window.set("fill", json!("#3366cc"));
    window.run().expect("the command runs");

    let render = std::sync::Arc::new(std::sync::Mutex::new(craftkit_window::WindowPicture::new(
        window.shared_session(),
    )));
    let options = craftkit_control::Options {
        token: "a-secret".to_string(),
        address: "127.0.0.1:0".parse().expect("an address"),
        announce: None,
        render: Some(render),
    };
    let mut server =
        craftkit_control::Server::serve_shared(window.shared_session(), options).expect("the channel opens");
    let mut client =
        craftkit_control::Client::connect(server.address(), "a-secret").expect("the client connects");

    let answer = client.call(craftkit_control::Method::UiRender, json!({})).expect("the picture comes");
    let data = answer["data"].as_str().unwrap_or_else(|| panic!("the picture is there: {answer}"));
    assert_eq!(answer["format"], "png", "the picture is a PNG");
    let bytes = craftkit_sdk::bytes::decode(data).expect("the picture is base64");
    assert_eq!(&bytes[1..4], b"PNG", "and the bytes say so");
    server.shutdown();
}
