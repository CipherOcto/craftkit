//! The **pointer**: what a hand is doing to the document.
//!
//! # The rule
//!
//! A drag that starts on a **handle** resizes the node, a drag that
//! starts on a **node** moves it, and a drag that starts on the
//! **background** moves the camera. That is the rule every drawing
//! program uses, and it is one rule and not three: what is under the
//! pointer says which of them it is.
//!
//! # Why it is here and not in the toolkit
//!
//! The pointer is the **front end's** business: it is a hand, and a
//! hand belongs to the program the person is looking at. The toolkit
//! knows about pixels and the kit knows about the world, and the rule
//! that joins them — a pixel is a point of the world, once the camera
//! has had its say — is the front end's to apply.
//!
//! So the pointer takes its points in the **world's** units, and every
//! front end — the window, and the channel that drives it — uses these
//! three functions and gets the same behaviour.

use craftkit_sdk::{EngineError, Outcome, Point, Rect, Session};

/// How near the **resize handle** is, in pixels of the screen.
///
/// It is a thing on the screen and not a thing in the world: it is the
/// same size whatever the zoom, and how near it is in the world's units
/// comes from the camera.
pub const HANDLE_PIXELS: f64 = 8.0;

/// The smallest a node may be, in the world's units.
///
/// A node with no area cannot be drawn and cannot be picked, and the
/// command that resizes it refuses such a size: the drag stops here
/// instead of asking for something the kit will not do.
pub const MIN_SIZE: f64 = 1.0;

/// What a drag is doing.
#[derive(Debug, Clone, PartialEq)]
pub enum Drag {
    /// A node is being moved.
    Move {
        /// The node.
        node: u64,
        /// Where the node was when the drag began.
        place: Rect,
    },
    /// A node is being resized.
    Resize {
        /// The node.
        node: u64,
        /// Where the node was when the drag began.
        place: Rect,
    },
    /// The camera is being moved.
    Camera,
}

/// **Begin** a drag at a point of the world.
///
/// What is under the pointer says what the drag is: the **handle**
/// first — it is a small thing on top of a node — then the node, and
/// then the background, which is the camera.
///
/// A node with **no place** is not dragged: there is nowhere to move
/// it to.
pub fn down(session: &Session, at: Point) -> Drag {
    // The handle is a **square** of HANDLE_PIXELS across, centred on
    // the corner, so the pointer is on it when it is within **half** of
    // it: a tolerance of the whole handle would make the handle reach
    // twice as far as it is drawn, and on a small document it would be
    // under most of the node.
    let tolerance = HANDLE_PIXELS / 2.0 / session.camera().zoom();
    if let Some(node) = handle_at(session, at, tolerance) {
        let place = session.document().node(node).and_then(|node| node.rect);
        if let Some(place) = place {
            return Drag::Resize { node, place };
        }
    }
    if let Some(node) = pick(session, at) {
        let place = session.document().node(node).and_then(|node| node.rect);
        if let Some(place) = place {
            return Drag::Move { node, place };
        }
    }
    Drag::Camera
}

/// **Continue** a drag: the camera moves, if the drag is the camera's.
///
/// The movement is in the **world's** units, so the camera moves with
/// the document and not with the screen: a zoom does not change how far
/// the world slides under the hand.
pub fn dragged(session: &mut Session, drag: &Drag, last: Point, at: Point) {
    if !matches!(drag, Drag::Camera) {
        return;
    }
    session.camera_mut().pan(last.x - at.x, last.y - at.y);
}

/// **End** a drag: the node moves or resizes.
///
/// It goes through the **registry**, like every other change: a drag
/// that moved a node by writing to the document would be a second way
/// into the design.
///
/// A drag that starts on a node that is **selected** moves the whole
/// selection: that is what a person means when they drag one of the
/// things they picked.
pub fn up(session: &mut Session, drag: &Drag, from: Point, at: Point) -> Result<Outcome, EngineError> {
    match drag {
        Drag::Camera => Ok(Outcome::new("the camera moved")),
        Drag::Move { node, place } => {
            let across = at.x - from.x;
            let up = at.y - from.y;
            if session.selected_ids().contains(node) {
                session.execute("select.move", &craftkit_sdk::serde_json::json!({ "x": across, "y": up }))
            } else {
                session.execute(
                    "node.move",
                    &craftkit_sdk::serde_json::json!({ "node": node, "x": place.x + across, "y": place.y + up }),
                )
            }
        }
        Drag::Resize { node, place } => {
            let resized = place.resized_to(at);
            session.execute(
                "node.resize",
                &craftkit_sdk::serde_json::json!({
                    "node": node,
                    "w": resized.w.max(MIN_SIZE),
                    "h": resized.h.max(MIN_SIZE),
                }),
            )
        }
    }
}

/// The node **under** a point of the world, if there is one.
///
/// Every node is offered to the kind that says what it is, and the
/// **deepest** hit wins: a thing in a group is in front of the group
/// that holds it.
pub fn pick(session: &Session, at: Point) -> Option<u64> {
    let document = session.document();
    document
        .walk_visible()
        .filter(|node| {
            session.kinds().get(&node.kind).map(|kind| (kind.hit)(node, document, at)).unwrap_or(false)
        })
        .last()
        .map(|node| node.id)
}

/// The node whose **resize handle** is under a point, if there is one.
///
/// The handle is the corner opposite the anchor — the kit's own
/// [`craftkit_sdk::Rect::resize_handle`] — and the tolerance is in the
/// **world's** units.
pub fn handle_at(session: &Session, at: Point, tolerance: f64) -> Option<u64> {
    let tolerance = tolerance.max(0.0);
    session
        .document()
        .walk_visible()
        .filter(|node| {
            node.rect
                .map(|rect| {
                    let handle = rect.resize_handle();
                    (handle.x - at.x).abs() <= tolerance && (handle.y - at.y).abs() <= tolerance
                })
                .unwrap_or(false)
        })
        .last()
        .map(|node| node.id)
}
