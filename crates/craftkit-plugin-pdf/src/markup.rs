//! The **text markup** of a document: the highlight, the underline, the
//! strikeout and the squiggly underline.
//!
//! The tree's own: a text markup is an **annotation** whose
//! `/QuadPoints` are the lines of the text it covers, and whose `/C` is
//! the colour of the kind.
//!
//! # The rules
//!
//! - The **kinds** are the tree's own four: `highlight`, `underline`,
//!   `strikeout` and `squiggly`; a kind that is not one of them is
//!   **refused**.
//! - The **quads** are the lines of the **text under a region**: the
//!   tree takes the lines of the text that is selected, and here the
//!   region says which text it is.
//! - A region with **no text under it** has no quads, and the markup
//!   is **refused**: a markup over nothing marks nothing.
//! - The **rectangle** of the annotation is the bounds of the quads.
//! - The **highlight multiplies over the text**, the tree's own rule:
//!   the file says so with the blend mode `/Multiply`.
//!
//! # What it does not settle
//!
//! - The **author** and the **date** of the tree's markup are not here:
//!   this tree has no user to name and no clock to read.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.markup.add";

/// The **kinds** of markup: the tree's own four, with the colour of
/// each.
pub fn kind_of(name: &str) -> Option<(&'static str, [f64; 3])> {
    match name.trim().to_ascii_lowercase().as_str() {
        "highlight" => Some(("Highlight", [1.0, 0.94, 0.0])),
        "underline" => Some(("Underline", [0.0, 0.47, 0.84])),
        "strikeout" | "strikethrough" => Some(("StrikeOut", [0.89, 0.13, 0.13])),
        "squiggly" => Some(("Squiggly", [0.18, 0.62, 0.36])),
        _ => None,
    }
}

/// The **kinds**, as they are said.
pub const KINDS: [&str; 4] = ["highlight", "underline", "strikeout", "squiggly"];

/// The **quad** of a line of text: the upper left, the upper right,
/// the lower left and the lower right, the tree's own order.
pub(crate) fn quad_of(bounds: [f64; 4]) -> Vec<f64> {
    let [left, bottom, right, top] = bounds;
    vec![left, top, right, top, left, bottom, right, bottom]
}

/// The **bounds** of the quads: the tree's own rectangle.
pub(crate) fn bounds_of(quads: &[Vec<f64>]) -> [f64; 4] {
    let mut left = f64::INFINITY;
    let mut bottom = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut top = f64::NEG_INFINITY;
    for quad in quads {
        for index in (0..4).rev() {
            let x = quad[index * 2];
            let y = quad[index * 2 + 1];
            left = left.min(x);
            bottom = bottom.min(y);
            right = right.max(x);
            top = top.max(y);
        }
    }
    [left, bottom, right, top]
}

commands! {
    /// **Mark** the text of a page: a highlight, an underline, a strikeout
    /// or a squiggly underline.
    pdf.markup.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the markup is on.

            page: NodeRef,
            /// The object number the annotation has in the file.

            object: Number,
            /// The kind of markup: `highlight`, `underline`, `strikeout` or
            /// `squiggly`.

            kind: Text,
            /// The region the markup covers: four numbers, the left, the
            /// bottom, the right and the top of it.

            region: Json,
        },
        run: {

            // The **kind**: one of the tree's own four.
            let Some((subtype, colour)) = kind_of(&self.kind.0) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!(
                        "`{}` is not a kind of markup: the kinds are {}",
                        self.kind.0.trim(),
                        KINDS.join(", ")
                    ),
                });
            };
            // The **page**: a page of the document.
            let Some(page) = session.document().node(self.page.0) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no node `{}`", self.page.0),
                });
            };
            let page_number = crate::the_number_of(page).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{}` has no object number", self.page.0),
            })?;
            // The page's own **annotations**, before the new one is made.
            let annots = crate::entries_of(&page.data)
                .into_iter()
                .find(|(key, _)| key == "Annots")
                .and_then(|(_, value)| value.as_array().cloned())
                .unwrap_or_default();
            // The **region**: four numbers.
            let Some(region) = crate::boxes::rect_of(&self.region) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the region is not a rectangle of four numbers".to_string(),
                });
            };
            // The **quads**: the lines of the text under the region.
            let text = crate::pagetext::text_of_page(session.document(), self.page.0)?;
            let mut quads: Vec<Vec<f64>> = Vec::new();
            for piece in &text.pieces {
                let Some(bounds) = piece.bounds else { continue };
                let origin = match (piece.x, piece.y) {
                    (Some(x), Some(y)) => (x, y),
                    _ => continue,
                };
                if !crate::region::under(region, bounds, origin) {
                    continue;
                }
                quads.push(quad_of(bounds));
            }
            if quads.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "there is no text under the region".to_string(),
                });
            }
            // The **object number** of the annotation: a whole number, and
            // one that nothing else has.
            let object = self.object.0;
            if !object.is_finite() || object < 0.0 || object.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "an object number is a whole number".to_string(),
                });
            }
            let number = object as u64;
            if number == 0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "object numbers start at one".to_string(),
                });
            }
            if session.document().walk().any(|node| crate::the_number_of(node) == Some(number)) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the object {number} is already in the document"),
                });
            }
            // The **annotation** itself.
            let rect = bounds_of(&quads);
            let flat: Vec<f64> = quads.iter().flat_map(|quad| quad.iter().copied()).collect();
            let mut entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!(subtype)),
                ("Rect".to_string(), json!(rect)),
                ("QuadPoints".to_string(), json!(flat)),
                ("C".to_string(), json!(colour)),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
            // The tree's highlight **multiplies** over the text, and the
            // file says so.
            if subtype == "Highlight" {
                entries.push(("BM".to_string(), json!("Multiply")));
            }
            let data = json!({ "object": number, "entries": entries });
            let name = format!("annotation {number}");
            let annot = session.document_mut()?.add_node_with(name.clone(), "pdf.object", None, data);
            // The page **names** the annotation.
            let mut annots = annots;
            annots.push(json!({ "ref": [number, 0] }));
            if let Some(node) = session.document_mut()?.node_mut(self.page.0) {
                node.data = crate::with_entry(&node.data, "Annots", json!(annots));
            }
            // And the **edge**: the page names the annotation.
            let relation = craftkit_sdk::Relation::with_payload(
                self.page.0,
                annot,
                "ref",
                json!({ "key": "Annots", "generation": 0 }),
            );
            if !session.document_mut()?.add_relation(relation) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the page already refers to the annotation `{annot}`"),
                });
            }
            Ok(Outcome::changed(format!("added the `{name}` to the page")).with_data(json!({
                "id": annot,
                "object": number,
                "page": self.page.0,
                "subtype": subtype,
                "kind": self.kind.0.trim().to_ascii_lowercase(),
                "rect": rect,
                "quads": quads,
                "colour": colour,
            })))

        }
    }
}
