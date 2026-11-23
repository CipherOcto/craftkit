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

use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::Rect;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Point, Session};

/// The picture's **width**, in pixels.
pub const WIDTH: usize = 640;

/// The picture's **height**, in pixels.
pub const HEIGHT: usize = 480;

/// A **headless** render: a picture of the document, and a click.
pub struct Headless {
    session: Session,
    width: usize,
    height: usize,
}

impl Headless {
    /// A headless render over a session.
    pub fn new(session: Session) -> Self {
        Headless { session, width: WIDTH, height: HEIGHT }
    }

    /// The session the render draws.
    pub fn session(&mut self) -> &mut Session {
        &mut self.session
    }

    /// The camera the picture is seen through.
    ///
    /// The tree's **own** camera — the one type, with its own caps — and
    /// not a second one. A new canvas goes through that camera rather
    /// than hand-rolling its own input.
    fn camera(&self) -> Camera {
        Camera::new()
    }

    /// The **picture** of what the program is showing, as a PNG.
    ///
    /// It is drawn with `Session::paint_view` — the **same** seam every
    /// other front end uses — so what the door shows is what a window
    /// would show.
    pub fn render(&mut self) -> Result<Vec<u8>, String> {
        let mut raster = craftkit_raster::Raster::new(self.width, self.height, self.camera())?;
        self.session.paint_view(&mut raster);
        Ok(craftkit_raster::png::write(raster.image()))
    }

    /// The **node** under a point, if there is one.
    ///
    /// The **last** one found, so that the node on top is the one that is
    /// under the pointer: the tree paints in order, and the thing a
    /// person sees on top is the thing they clicked.
    fn node_at(&self, at: Point) -> Option<u64> {
        let mut found: Option<u64> = None;
        for node in self.session.document().walk() {
            let Some(rect) = node.rect else { continue };
            if rect.contains(at) {
                found = Some(node.id);
            }
        }
        found
    }

    /// The **surface** a node is on, if there is one.
    ///
    /// The click is an action of a **component**, and a component is on
    /// a surface — so the surface is found by looking for the one that
    /// holds the node.
    fn surface_of(&self, id: u64) -> Option<String> {
        for node in self.session.document().nodes() {
            if node.kind != "a2ui.surface" {
                continue;
            }
            if node.children.iter().any(|child| child.id == id) {
                return Some(node.name.clone());
            }
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
        let node =
            self.session.document().node(id).cloned().ok_or_else(|| format!("the node {id} is not there"))?;
        // The **action** is the node's own: a component carries one.
        let action = node.data.get("action").cloned().unwrap_or_else(|| Json::Object(Default::default()));
        let surface = self.surface_of(id).unwrap_or_default();
        let outcome = self
            .session
            .execute(
                "a2ui.component.invoke",
                &json!({
                    "surface_id": surface,
                    "component_id": node.id.to_string(),
                    "action": action,
                }),
            )
            .map_err(|error| error.to_string())?;
        Ok(json!({
            "clicked": id,
            "message": outcome.message,
            "changed": outcome.changed,
        }))
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
    fn session() -> Session {
        let ecosystem = craftkit_sdk::Ecosystem::from_inventory().expect("the ecosystem is built");
        ecosystem.session()
    }

    /// A click on **nothing** says nothing.
    #[test]
    fn a_click_on_nothing_says_nothing() {
        let mut render = Headless::new(session());
        let answer = render.click(Point::new(-10.0, -10.0)).expect("the click is answered");
        assert!(answer["clicked"].is_null(), "{answer}");
    }
}
