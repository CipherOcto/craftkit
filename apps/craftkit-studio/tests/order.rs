//! The **order of the tree is the order of the drawing**.
//!
//! A node that is drawn later is on top. So the order is not a number
//! a node carries: it is where the node is among its brothers, and
//! bringing a node to the front is moving it to the end of the list it
//! is in.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The order the nodes are in.
fn order(session: &craftkit_sdk::Session) -> Vec<u64> {
    session.document().roots().iter().map(|node| node.id).collect()
}

/// A node that is brought to the front is drawn **last**.
///
/// And a node that is sent to the back is drawn first.
#[test]
fn the_front_is_drawn_last() {
    let mut session = ecosystem().session();
    for name in ["one", "two", "three"] {
        session
            .execute("node.add", &json!({ "name": name, "kind": "geometry.rect" }))
            .expect("the node is added");
    }
    assert_eq!(order(&session), vec![1, 2, 3], "the order is the order they were added in");

    session.execute("node.front", &json!({ "node": 1 })).expect("the node is at the front");
    assert_eq!(order(&session), vec![2, 3, 1], "the node is last, and so it is on top");

    session.execute("node.back", &json!({ "node": 3 })).expect("the node is at the back");
    assert_eq!(order(&session), vec![3, 2, 1], "the node is first, and so it is behind");
}

/// A node that is not there is **refused**.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = ecosystem().session();
    let refused = session.execute("node.front", &json!({ "node": 9 })).expect_err("the command refuses");
    assert!(refused.to_string().contains("no node"), "the error says why: {refused}");
}

/// The order is the order of the **drawing**, and the picture agrees.
///
/// Two rectangles on top of each other: the one that is later in the
/// tree is the one that is on top.
#[cfg(feature = "vector")]
#[test]
fn the_picture_agrees_with_the_order() {
    let mut session = ecosystem().session();
    for (name, fill) in [("under", "#3366cc"), ("over", "#cc3366")] {
        session
            .execute(
                "vector.rectangle",
                &json!({ "x": -5.0, "y": -5.0, "width": 10.0, "height": 10.0, "fill": fill, "name": name }),
            )
            .expect("the rectangle is added");
    }
    let picture = draw(&session, 64);
    assert!(picture.contains(&[0xcc, 0x33, 0x66, 0xff]), "the rectangle that is later in the tree is on top");

    // And bringing the other one to the front puts it on top.
    session.execute("node.front", &json!({ "node": 1 })).expect("the node is at the front");
    let picture = draw(&session, 64);
    assert!(
        picture.contains(&[0x33, 0x66, 0xcc, 0xff]),
        "the rectangle that was brought to the front is on top"
    );
}

/// The picture of a document, as the pixels it has.
#[cfg(feature = "vector")]
fn draw(session: &craftkit_sdk::Session, size: usize) -> Vec<[u8; 4]> {
    let camera = *session.camera();
    let mut raster = craftkit_raster::Raster::new(size, size, camera).expect("the picture is made");
    let document = session.document();
    for node in document.walk_visible() {
        let Some(kind) = session.kinds().get(&node.kind) else { continue };
        (kind.paint)(node, document, &mut raster);
    }
    let image = raster.image();
    let mut pixels = Vec::new();
    for y in 0..image.height() {
        for x in 0..image.width() {
            pixels.push(image.pixel(x, y).unwrap_or([0, 0, 0, 0]));
        }
    }
    pixels
}
