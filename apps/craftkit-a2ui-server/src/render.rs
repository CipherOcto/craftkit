//! The **headless** render — a picture of the document, and a click.
//!
//! # What this is
//!
//! A [`Render`] that draws the document into a **software rasteriser** —
//! no window, no display, no event loop — and that turns a **click** into
//! a **command**.
//!
//! # Why the click is a command
//!
//! The house rule is that every change to the document is a command, and
//! that a front end has **no second path**. So a click is not a document
//! edit: it is `a2ui.component.invoke`, and the command does the work.
//! The renderer only says **what** was clicked.
//!
//! # The picture is the tree's own
//!
//! The picture is drawn with `Session::paint_view` — the **same** seam
//! every other front end uses — so what the door shows is what a window
//! would show.

use std::sync::{Arc, Mutex};

use crate::Outbound;
use craftkit_sdk::document::Rect;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Outcome, Point, Session};

/// The picture's **width**, in pixels.
pub const WIDTH: usize = 640;

/// The picture's **height**, in pixels.
pub const HEIGHT: usize = 480;

/// A **headless** render: a picture of the document, and a click.
pub struct Headless {
    /// The door's **own** session, behind its lock.
    ///
    /// The channel serves the door's document and not a copy of it, so a
    /// command an agent runs over the channel changes what the door is
    /// drawing.
    session: Arc<Mutex<Session>>,
    /// The messages that are not replies, shared with the door.
    outbound: Outbound,
    width: usize,
    height: usize,
    /// How many calls to the agent the render has made.
    ///
    /// The protocol says every call to an agent-side function carries an
    /// id of its own, and the agent copies it into the answer.
    calls: std::sync::atomic::AtomicU64,
}

impl Headless {
    /// A headless render over a session.
    ///
    /// The session is the door's **own**, behind its lock, and the queue
    /// is the door's: a click may raise an action, and the action
    /// belongs on the wire the agent is reading.
    pub fn new(session: Arc<Mutex<Session>>, outbound: Outbound) -> Self {
        Headless {
            session,
            outbound,
            width: WIDTH,
            height: HEIGHT,
            calls: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// The session the render draws.
    pub fn with_session<T>(&mut self, body: impl FnOnce(&mut Session) -> T) -> T {
        let mut session = match self.session.lock() {
            Ok(session) => session,
            Err(poisoned) => poisoned.into_inner(),
        };
        body(&mut session)
    }

    /// The **picture** of what the program is showing, as a PNG.
    ///
    /// It is drawn with `Session::paint_view` — the **same** seam every
    /// other front end uses — so what the door shows is what a window
    /// would show.
    pub fn render(&mut self) -> Result<Vec<u8>, String> {
        // The camera is the **session's** own — the one type, with its
        // own caps — and not a second one.
        let camera = self.with_session(|session| *session.camera());
        let mut raster = craftkit_raster::Raster::new(self.width, self.height, camera)?;
        self.with_session(|session| session.paint_view(&mut raster));
        Ok(craftkit_raster::png::write(raster.image()))
    }

    /// The **node** under a point, if there is one.
    ///
    /// The **last** one found, so that the node on top is the one that is
    /// under the pointer: the tree paints in order, and the thing a
    /// person sees on top is the thing they clicked.
    fn node_at(&mut self, at: Point) -> Option<u64> {
        // The **tree's own** hit-test: the kind says whether the point is
        // on the node, and a node of a kind the tree does not know is
        // never under the pointer.
        let mut found: Option<u64> = None;
        self.with_session(|session| {
            let document = session.document();
            for node in document.walk() {
                let Some(kind) = session.kinds().get(&node.kind) else { continue };
                if (kind.hit)(node, document, at) {
                    found = Some(node.id);
                }
            }
        });
        found
    }

    /// The **surface** a node is on, if there is one.
    ///
    /// The click is an action of a **component**, and a component is on
    /// a surface — and a component may be **nested**: a card's child is
    /// in the card, and the card is in the surface. So the surface is
    /// found by walking **up**.
    fn surface_of(&mut self, id: u64) -> Option<String> {
        let surfaces: Vec<craftkit_sdk::Node> =
            self.with_session(|session| session.document().nodes().cloned().collect());
        let document = self.with_session(|session| session.document().clone());
        let mut current = Some(id);
        while let Some(id) = current {
            if let Some(node) = surfaces.iter().find(|node| node.id == id)
                && node.kind == "a2ui.surface"
            {
                return Some(node.name.clone());
            }
            current = document.parent_of(id).flatten();
        }
        None
    }

    /// The **target** of a click: the node under the pointer, or the
    /// nearest **ancestor** that carries an action.
    ///
    /// A button's label is a node **inside** the button, and the label
    /// is drawn on top of it — so a click on the label is a click on the
    /// button, and not on a label with nothing to do.
    fn target_of(&mut self, id: u64) -> Option<u64> {
        let document = self.with_session(|session| session.document().clone());
        let mut current = Some(id);
        while let Some(id) = current {
            let node = document.node(id)?;
            if node.data.get("action").is_some() {
                return Some(id);
            }
            if node.kind == "a2ui.surface" {
                return None;
            }
            current = document.parent_of(id).flatten();
        }
        None
    }

    /// The **click**: what was clicked, and what it does.
    ///
    /// The click is turned into `a2ui.component.invoke` — a **command** —
    /// and the command's outcome is the answer.
    pub fn click(&mut self, at: Point) -> Result<Json, String> {
        let Some(id) = self.node_at(at) else {
            return Ok(json!({ "clicked": Json::Null }));
        };
        // And the **target**: the node, or the nearest ancestor that
        // carries an action.
        let Some(id) = self.target_of(id) else {
            return Ok(json!({ "clicked": Json::Null }));
        };
        let node = self
            .with_session(|session| session.document().node(id).cloned())
            .ok_or_else(|| format!("the node {id} is not there"))?;
        // The **action** is the node's own: a component carries one.
        let action = node.data.get("action").cloned().unwrap_or_else(|| Json::Object(Default::default()));
        // A node that is not on a surface is not a component: there is
        // nothing to invoke, and the click says so.
        let Some(surface) = self.surface_of(id) else {
            return Ok(json!({ "clicked": Json::Null }));
        };
        let outcome = self
            .with_session(|session| {
                session.execute(
                    "a2ui.component.invoke",
                    &json!({
                        "surface_id": surface,
                        "component_id": node.name,
                        "action": action,
                    }),
                )
            })
            .map_err(|error| error.to_string())?;
        // An action that is the **agent's** goes on the wire.
        self.queue(&outcome);
        Ok(json!({
            "clicked": id,
            "message": outcome.message,
            "changed": outcome.changed,
        }))
    }

    /// Queue an **action** the outcome raised, if it raised one.
    ///
    /// The message goes on the wire even if the agent says nothing more:
    /// the writer waits on the queue.
    fn queue(&self, outcome: &Outcome) {
        let count = self.calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let Some(message) = crate::message_of(outcome, &format!("call-{count}")) else { return };
        let _ = self.outbound.send(Some(message));
    }

    /// The **drag**: what a hand does to the document.
    ///
    /// The same three ways a person drags: a **handle** resizes, a
    /// **node** moves, and the **background** slides the camera. The
    /// change goes through the **registry**, like every other change.
    ///
    /// And an a2ui **component** is not dragged: its place is
    /// **derived**, and a person cannot drag a derived value about.
    pub fn drag(&mut self, from: Point, to: Point) -> Result<Json, String> {
        let picked = self.with_session(|session| {
            let document = session.document();
            document
                .walk_visible()
                .filter(|node| {
                    session
                        .kinds()
                        .get(&node.kind)
                        .map(|kind| (kind.hit)(node, document, from))
                        .unwrap_or(false)
                })
                .last()
                .map(|node| (node.id, node.kind.clone()))
        });
        let Some((_node, kind)) = picked else {
            // The **background**: the camera slides.
            self.with_session(|session| session.camera_mut().pan(from.x - to.x, from.y - to.y));
            return Ok(json!({ "message": "the camera moved" }));
        };
        // Everything this program draws is an a2ui node, and an a2ui
        // node's place is **derived**: a person cannot drag a derived
        // value about. So the drag says so, and nothing moves.
        debug_assert!(kind.starts_with("a2ui."), "the door holds a2ui nodes only");
        Ok(json!({
            "message": "an a2ui component's place is derived, and a drag does not change it",
            "moved": Json::Null,
        }))
    }
}

/// The **control channel**'s render: the door's headless one.
///
/// The trait is the channel's, and the door supplies it: a picture, a
/// click and a drag — no window anywhere.
impl craftkit_control::Render for Headless {
    fn render(&mut self) -> Result<Vec<u8>, String> {
        Headless::render(self)
    }

    fn click(&mut self, at: Point) -> Result<Json, String> {
        Headless::click(self, at)
    }

    fn drag(&mut self, from: Point, to: Point) -> Result<Json, String> {
        Headless::drag(self, from, to)
    }
}

/// The **bounds** of the document: the box that holds everything that is
/// somewhere.
///
/// It is the union of the places of the nodes, and a node with no place is
/// in none of it.
pub fn bounds(session: &Session) -> Option<Rect> {
    session.document().bounds()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A session with a node in it.
    fn session() -> Arc<Mutex<Session>> {
        let ecosystem = craftkit_sdk::Ecosystem::from_inventory().expect("the ecosystem is built");
        Arc::new(Mutex::new(ecosystem.session()))
    }

    /// The queue of the messages that are not replies.
    fn a_queue() -> std::sync::mpsc::Sender<Option<craftkit_sdk::serde_json::Value>> {
        let (sender, _reader) = std::sync::mpsc::channel();
        sender
    }

    /// A headless render over a session.
    fn render() -> Headless {
        Headless::new(session(), a_queue())
    }

    /// A click on **nothing** says nothing.
    #[test]
    fn a_click_on_nothing_says_nothing() {
        let mut render = render();
        let answer = render.click(Point::new(-10.0, -10.0)).expect("the click is answered");
        assert!(answer["clicked"].is_null(), "{answer}");
    }

    /// A drag on an a2ui **component** changes nothing: its place is
    /// derived, and a derived value is not dragged about.
    #[test]
    fn a_drag_on_a_component_does_not_move_it() {
        let mut render = render();
        render.with_session(|session| {
            session
                .execute("a2ui.surface.create", &json!({ "surface_id": "s1" }))
                .expect("the surface is made");
            session
                .execute(
                    "a2ui.surface.apply",
                    &json!({
                        "surface_id": "s1",
                        "components": [{ "component": "Text", "id": "root", "text": "hello" }],
                    }),
                )
                .expect("the text is drawn");
        });
        let before = render.with_session(|session| {
            session.document().walk().find(|node| node.kind == "a2ui.text").and_then(|node| node.rect)
        });
        let answer = render.drag(Point::new(1.0, 1.0), Point::new(50.0, 50.0)).expect("the drag is answered");
        assert!(answer["moved"].is_null(), "{answer}");
        let after = render.with_session(|session| {
            session.document().walk().find(|node| node.kind == "a2ui.text").and_then(|node| node.rect)
        });
        assert_eq!(before, after, "the text did not move");
    }

    /// And the **render** is the channel's: a picture, a click and a
    /// drag, with no window anywhere.
    #[test]
    fn the_headless_render_is_the_channels() {
        let mut render = render();
        let picture = craftkit_control::Render::render(&mut render).expect("the picture is drawn");
        assert!(!picture.is_empty(), "and it is not empty");
    }
}
