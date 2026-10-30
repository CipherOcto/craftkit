//! The **assets**: the files the document needs, and the commands that
//! say so.
//!
//! # What an asset is
//!
//! A document that points at a file is **not finished**: it is a set of
//! instructions that only work where the files are. The document says
//! which files it needs — its **assets** — and the bytes travel beside
//! it in a bundle (see [`crate::bundle`]).
//!
//! # Why the commands are here
//!
//! The list belongs to the document, and the commands that change it
//! belong with it: so they are in the SDK rather than in an
//! application, and every front end gets the same ones. A plugin that
//! points at a file — the sound battery points at audio — adds an asset
//! through the same list, and the document then says what it needs in
//! one place instead of in every payload that happens to hold a path.

use craftkit_derive::Command;

use crate::document::Asset;
use crate::{EngineError, Number, Outcome, Session, Text};

/// Say that the document needs a file.
#[derive(Command)]
#[command(id = "doc.asset.add", area = "doc", icon = "plus", menu("File"), mutates, undoable)]
pub struct AddAsset {
    /// Where the file is, relative to the document.
    pub path: Text,
    /// What kind of file it is: `audio`, `font`, `image`, …
    pub kind: Option<Text>,
    /// How long it is, in bytes, if anybody knows.
    pub length: Option<Number>,
}

impl AddAsset {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "doc.asset.add".to_string(),
                message: "an asset has a path".to_string(),
            });
        }
        let kind = match &self.kind {
            Some(kind) => kind.0.clone(),
            None => "file".to_string(),
        };
        let length = match &self.length {
            Some(length) if length.0.is_finite() && length.0 >= 0.0 => Some(length.0 as u64),
            Some(_) => {
                return Err(EngineError::Failed {
                    id: "doc.asset.add".to_string(),
                    message: "a length is a number of bytes".to_string(),
                });
            }
            None => None,
        };
        let asset = Asset::new(path.clone(), kind.clone(), length);
        if !session.document_mut()?.add_asset(asset) {
            return Err(EngineError::Failed {
                id: "doc.asset.add".to_string(),
                message: format!("the document already needs `{path}`"),
            });
        }
        Ok(Outcome::changed(format!("the document now needs `{path}`"))
            .with_data(crate::serde_json::json!({ "path": path, "kind": kind })))
    }
}

/// The document does not need a file any more.
#[derive(Command)]
#[command(id = "doc.asset.remove", area = "doc", icon = "minus", menu("File"), mutates, undoable)]
pub struct RemoveAsset {
    /// The path of the file.
    pub path: Text,
}

impl RemoveAsset {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let removed = session.document_mut()?.remove_assets(|asset| asset.path == path);
        if removed == 0 {
            return Err(EngineError::Failed {
                id: "doc.asset.remove".to_string(),
                message: format!("the document does not need `{path}`"),
            });
        }
        Ok(Outcome::changed(format!("the document does not need `{path}` any more"))
            .with_data(crate::serde_json::json!({ "path": path, "removed": removed })))
    }
}

/// The files the document needs.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "doc.assets", area = "doc", icon = "list", menu("File"))]
pub struct ListAssets {}

impl ListAssets {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let assets: Vec<crate::serde_json::Value> = session
            .document()
            .assets
            .iter()
            .map(|asset| {
                crate::serde_json::json!({
                    "path": asset.path,
                    "kind": asset.kind,
                    "length": asset.length,
                })
            })
            .collect();
        let count = assets.len();
        Ok(Outcome::new(format!("the document needs {count} file(s)"))
            .with_data(crate::serde_json::json!({ "assets": assets })))
    }
}
