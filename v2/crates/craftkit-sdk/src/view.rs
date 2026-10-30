//! The **view state**: the camera's place, written down or not.
//!
//! # What this is for
//!
//! The camera is the **session's** state (§29): it is where the person
//! is looking, it has its own caps, and the undo does not move it. That
//! is the right rule for a program that is open on a screen.
//!
//! But an application that wants a document to open **where it was left**
//! needs the view written down. The artist's IDE keeps the viewport in
//! the document (§33's port), and it is right to: the board is a thing
//! you come back to.
//!
//! So there are two commands, and the difference between them is the
//! whole of it:
//!
//! - `view.save` — write the camera into the document, under **the
//!   SDK's own key**. It changes the document, so the undo takes it
//!   back;
//! - `view.restore` — read it back and move the camera. It changes
//!   **the session**, so the undo does not take it back, and neither
//!   does a save.
//!
//! # Why the key is the SDK's
//!
//! The document's payload belongs to whoever writes it, and two plugins
//! must not collide by accident. So the SDK writes under one name of
//! its own — `craftkit/view` — and a plugin that wants a view of its
//! own writes under a name of its own, the way the sound battery keeps
//! its session settings.
//!
//! # What is not here
//!
//! The view state is **not** a second camera, and the camera is not
//! restored by the undo. Saving the view and restoring it are two
//! commands, and nothing happens in between unless somebody asks.

use craftkit_derive::Command;

use crate::{EngineError, Outcome, Session};

/// The key the SDK writes the view under.
///
/// It is namespaced, because the payload is shared: a plugin that writes
/// a view of its own writes under a name of its own.
pub const VIEW_KEY: &str = "craftkit/view";

/// Write the camera into the document.
///
/// It changes the document and not the camera: the camera is where it
/// is, and this only says where it was.
#[derive(Command)]
#[command(id = "view.save", area = "view", icon = "save", menu("View"), mutates, undoable)]
pub struct SaveView {}

impl SaveView {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let camera = session.camera();
        let zoom = camera.zoom();
        let offset = camera.offset();
        let view = crate::serde_json::json!({
            "zoom": zoom,
            "offset": { "x": offset.x, "y": offset.y },
        });
        session.document_mut()?.data[VIEW_KEY] = view.clone();
        Ok(Outcome::changed(format!("the view is written down at {zoom}"))
            .with_data(crate::serde_json::json!({ "view": view })))
    }
}

/// Move the camera to where the document says it was.
///
/// It changes the session and not the document: the camera moves, and
/// the document is as it was.
#[derive(Command)]
#[command(id = "view.restore", area = "view", icon = "open", menu("View"))]
pub struct RestoreView {}

impl RestoreView {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let view = session.document().data.get(VIEW_KEY).cloned().unwrap_or(crate::serde_json::Value::Null);
        if view.is_null() {
            return Err(EngineError::Failed {
                id: "view.restore".to_string(),
                message: "the document does not say where the view was".to_string(),
            });
        }
        let zoom = view.get("zoom").and_then(|zoom| zoom.as_f64()).unwrap_or(1.0);
        let x = view.get("offset").and_then(|offset| offset.get("x")).and_then(|x| x.as_f64()).unwrap_or(0.0);
        let y = view.get("offset").and_then(|offset| offset.get("y")).and_then(|y| y.as_f64()).unwrap_or(0.0);
        if !zoom.is_finite() || !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Failed {
                id: "view.restore".to_string(),
                message: "the view in the document is not a view".to_string(),
            });
        }
        session.camera_mut().set_zoom(zoom);
        // The view says where the middle of the screen is, so it is put
        // there: `look_at` is absolute and `pan` is not.
        session.camera_mut().look_at(crate::Point::new(x, y));
        let camera = session.camera();
        Ok(Outcome::new(format!("the view is at {}", camera.zoom())).with_data(crate::serde_json::json!({
            "zoom": camera.zoom(),
            "offset": { "x": camera.offset().x, "y": camera.offset().y },
        })))
    }
}
