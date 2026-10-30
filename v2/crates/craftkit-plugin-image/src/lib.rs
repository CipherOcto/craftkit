//! `craftkit-plugin-image` — the document model of a **picture**.
//!
//! # What is ported
//!
//! The family has pictures everywhere — an artist's stage, a publishing
//! application's placed objects, a spreadsheet's cell pictures — and in
//! every one of them a picture is **a file beside the document** and a
//! **size** inside it. The pixels are never in the document, and the
//! kit's own **stream** (§25) is what says so.
//!
//! - a **picture** — a node whose **payload** holds the size and the
//!   format, and whose **stream** is the file;
//! - the **size** — read from the picture's header, and never from the
//!   pixels;
//! - the **aspect ratio** and the **pixel count** — derived.
//!
//! # The three things the port found
//!
//! - **A picture is a size and a file, and never the pixels.** The
//!   family keeps the bytes beside the document and writes them into the
//!   package on save — so the document that an application edits holds
//!   two numbers and a name, and the file holds the rest.
//! - **The header is the only part of the picture the document needs.**
//!   So the reader reads the **head** of the file and stops: a picture
//!   of forty megabytes is measured by reading a few kilobytes of it,
//!   and the rest of it is never in memory.
//! - **A picture is an *open* kind.** The family has a vector of
//!   picture kinds — raster, vector, PDF — and every new one is a
//!   change in the model, the reader, the writer and the panel. Here
//!   the **format is data**: the payload says which format it is, and
//!   the kit says nothing about what a raster is.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome, Session, Text};

/// The image layer.
///
/// It needs the core and the size reader, and it says so.
#[derive(Layer)]
#[layer(id = "image", name = "Image", requires("core"))]
pub struct ImageLayer;

/// A **picture**: a size, a format, and a file.
///
/// The payload holds the two numbers a layout needs, and the stream holds
/// the name of the file the bytes are in. The pixels are in neither.
#[derive(NodeKind)]
#[node_kind(kind = "image.picture", label = "Picture", layer = "image", evaluates)]
pub struct PictureKind;

impl PictureKind {
    /// Not drawn.
    ///
    /// A picture is a file, and the kit cannot draw a file: what it can
    /// do is say how big it is.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a picture **means**: its size, and what follows from it.
    ///
    /// The aspect ratio and the pixel count are **derived**: they are
    /// worked out from the two numbers, and they are never stored.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let width = node.data.get("width").and_then(|width| width.as_u64()).unwrap_or(0);
        let height = node.data.get("height").and_then(|height| height.as_u64()).unwrap_or(0);
        let aspect = if height == 0 { None } else { Some(width as f64 / height as f64) };
        Some(json!({
            "width": width,
            "height": height,
            "pixels": width * height,
            "aspect": aspect,
            "format": node.data.get("format").cloned().unwrap_or(Json::Null),
            "stream": node.stream.clone(),
        }))
    }
}

/// Put a picture in the document.
///
/// The file is read **within a limit**, and only its **head** is read:
/// the size is in the header, and the pixels are in the file.
#[derive(Command)]
#[command(id = "image.add", area = "image", layer = "image", icon = "image", mutates, undoable)]
pub struct AddPicture {
    /// The picture file.
    pub path: Text,
    /// What to call it.
    pub name: Option<Text>,
}

impl AddPicture {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let size = measure(&path)?;
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => file_name(&path),
        };
        let id = session.document_mut()?.add_node(name.clone(), "image.picture", None);
        if let Some(node) = session.document_mut()?.node_mut(id) {
            node.data = json!({
                "width": size.width,
                "height": size.height,
                "format": size.format.name(),
            });
            // The bytes are in the file, and the document says where.
            node.stream = Some(path.clone());
        }
        Ok(Outcome::changed(format!("added a picture `{name}`")).with_data(json!({
            "id": id,
            "kind": "image.picture",
            "width": size.width,
            "height": size.height,
            "format": size.format.name(),
            "path": path,
        })))
    }
}

/// How big a picture is.
///
/// It changes nothing, so it is not an undo step: a measurement is not
/// an edit.
#[derive(Command)]
#[command(id = "image.measure", area = "image", layer = "image", icon = "image", no_journal)]
pub struct MeasurePicture {
    /// The picture file.
    pub path: Text,
}

impl MeasurePicture {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let size = measure(&path)?;
        Ok(Outcome::new(format!("{} by {}", size.width, size.height)).with_data(json!({
            "width": size.width,
            "height": size.height,
            "pixels": size.pixels(),
            "aspect": size.aspect(),
            "format": size.format.name(),
            "path": path,
        })))
    }
}

/// The panel that shows the pictures.
#[derive(Panel)]
#[panel(id = "panel.image", label = "Image", layer = "image", commands("image.add", "image.measure"))]
pub struct ImagePanel;

/// The most bytes of a picture's **head** that are read.
///
/// The header is at the front of the file, and a megabyte of it is more
/// than any picture needs. The limit is this program's, and the kit's
/// own number is beside it.
const MAX_HEAD_BYTES: u64 = 1024 * 1024;

/// The size of the picture in that file.
///
/// Only the **head** of the file is read: the size is in the header, and
/// the pixels are not needed to know how big the picture is.
fn measure(path: &str) -> Result<craftkit_image::Size, EngineError> {
    let head = craftkit_sdk::files::read_prefix_bounded(std::path::Path::new(path), MAX_HEAD_BYTES)
        .map_err(|message| EngineError::Failed { id: "image".to_string(), message })?;
    craftkit_image::size(&head).map_err(|message| EngineError::Failed { id: "image".to_string(), message })
}

/// The name of a file, without the folders it is in.
fn file_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(ImageLayer, PictureKind, AddPicture, MeasurePicture, ImagePanel);
