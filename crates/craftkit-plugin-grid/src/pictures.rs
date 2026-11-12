//! The **picture** of a cell.
//!
//! A cell may hold a **picture**: a small image drawn in the cell.
//! The picture is a **file beside the document** and a **size** inside
//! it (§25): the bytes are never in the document, and the document
//! says where they are.
//!
//! The size is **measured** from the head of the file and never from
//! the pixels: a picture of forty megabytes is measured by reading a
//! little of it, and the rest of it is never in memory.
//!
//! The rules:
//!
//! - a file that is **not a picture** is refused, and so is a file
//!   that is not there;
//! - a cell that is **covered by a merge** is refused: the picture
//!   would be drawn under another cell's;
//! - a cell that is **locked** is refused;
//! - the **alt text** is what a reader says when it cannot show the
//!   picture, and it is the tree's own.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// The most bytes of a picture that are read to measure it.
const MAX_HEAD_BYTES: u64 = 1024 * 1024;

/// The **pictures** of a sheet: the cell and what it holds.
pub fn pictures_of(sheet: &Node) -> Vec<(String, Json)> {
    let Some(map) = sheet.data.get("pictures").and_then(|pictures| pictures.as_object()) else {
        return Vec::new();
    };
    map.iter().map(|(cell, picture)| (cell.clone(), picture.clone())).collect()
}

/// The **picture** of a cell, if it has one.
pub fn picture_of(sheet: &Node, cell: &str) -> Option<Json> {
    sheet.data.get("pictures").and_then(|pictures| pictures.get(cell)).cloned()
}

/// The **size** of a picture file, read from its head.
fn measure(path: &str) -> Result<craftkit_image::Size, String> {
    let head = craftkit_sdk::files::read_prefix_bounded(std::path::Path::new(path), MAX_HEAD_BYTES)?;
    craftkit_image::size(&head)
}

/// The **format** of a picture, from the name of the file.
fn format_of(path: &str) -> Option<&'static str> {
    let name = path.rsplit('/').next().unwrap_or_default().to_ascii_lowercase();
    if name.ends_with(".png") {
        Some("png")
    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        Some("jpeg")
    } else {
        None
    }
}

commands! {
    /// **Set** the picture of a cell.
    ///
    /// The picture is a file beside the document: the cell says where the
    /// bytes are, and how big the picture is.
    grid.picture.set {
        area: "grid",
        layer: "grid",
        icon: "image",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The cell the picture is drawn in.

            cell: Text,
            /// The **file** the bytes are in.

            path: Text,
            /// The **alt text**: what a reader says when it cannot show the
            /// picture.

            alt: Option<Text>,
        },
        run: {

            let id = "grid.picture.set".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let cell = crate::cell_at(self.cell.0.trim(), &id)?;
            crate::merges::refuse_covered(&node, &[cell], &id)?;
            crate::protection::refuse_locked(&node, &crate::style_locks(session, sheet), &[cell], &id)?;
            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a picture is a file, and a file has a name".to_string(),
                });
            }
            let size = measure(&path).map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            let format = format_of(&path).unwrap_or("unknown");
            let alt = self.alt.as_ref().map(|alt| alt.0.clone()).unwrap_or_default();
            let picture = json!({
                "path": path,
                "width": size.width,
                "height": size.height,
                "format": format,
                "alt": alt,
            });
            let address = cell.to_text();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                if !node.data["pictures"].is_object() {
                    node.data["pictures"] = json!({});
                }
                if let Some(map) = node.data["pictures"].as_object_mut() {
                    map.insert(address.clone(), picture.clone());
                }
            }
            Ok(Outcome::changed(format!("the cell `{address}` holds a picture")).with_data(json!({
                "sheet": sheet,
                "cell": address,
                "picture": picture,
            })))

        }
    }
    /// **Read** the picture of a cell: where its bytes are and how big it
    /// is.
    ///
    /// Without a cell, every picture of the sheet is read.
    ///
    /// The **pixels** and the **aspect ratio** are derived.
    grid.picture.read {
        area: "grid",
        layer: "grid",
        icon: "image",
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The cell. Without it, every picture of the sheet is read.

            cell: Option<Text>,
        },
        run: {

            let id = "grid.picture.read".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let read = |address: &str, picture: &Json| -> Json {
                let width = picture.get("width").and_then(|width| width.as_u64()).unwrap_or(0);
                let height = picture.get("height").and_then(|height| height.as_u64()).unwrap_or(0);
                let aspect = if height == 0 { None } else { Some(width as f64 / height as f64) };
                json!({
                    "cell": address,
                    "picture": picture,
                    "pixels": width * height,
                    "aspect": match aspect {
                        Some(aspect) => json!(aspect),
                        None => Json::Null,
                    },
                })
            };
            match &self.cell {
                Some(cell) => {
                    let address = crate::cell_at(cell.0.trim(), &id)?.to_text();
                    let Some(picture) = picture_of(&node, &address) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the cell `{address}` holds no picture"),
                        });
                    };
                    let read = read(&address, &picture);
                    let mut data = json!({ "sheet": sheet });
                    if let Some(map) = data.as_object_mut()
                        && let Some(read) = read.as_object()
                    {
                        for (key, value) in read {
                            map.insert(key.clone(), value.clone());
                        }
                    }
                    Ok(Outcome::new(format!("the cell `{address}` holds a picture")).with_data(data))
                }
                None => {
                    let mut pictures: Vec<Json> = Vec::new();
                    for (address, picture) in pictures_of(&node) {
                        pictures.push(read(&address, &picture));
                    }
                    Ok(Outcome::new(format!("the sheet holds {} picture(s)", pictures.len()))
                        .with_data(json!({ "sheet": sheet, "pictures": pictures })))
                }
            }

        }
    }
    /// **Take** the picture of a cell away.
    grid.picture.remove {
        area: "grid",
        layer: "grid",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The cell.

            cell: Text,
        },
        run: {

            let id = "grid.picture.remove".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let address = crate::cell_at(self.cell.0.trim(), &id)?.to_text();
            if picture_of(&node, &address).is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the cell `{address}` holds no picture"),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet)
                && let Some(map) = node.data["pictures"].as_object_mut()
            {
                map.remove(&address);
            }
            Ok(Outcome::changed(format!("the cell `{address}` holds no picture"))
                .with_data(json!({ "sheet": sheet, "cell": address })))

        }
    }
}
