//! The **mark of a redaction** on a page: the tree's own redaction
//! mark.
//!
//! The tree's own: a **redaction mark** is an annotation that says
//! "this text is going to be taken away", and it is drawn as the
//! outline of the text it covers. The tree writes the text that is
//! over it — the **overlay** — and the file says so in the `/DA` of
//! the annotation.
//!
//! # The rules
//!
//! - The **region** is four numbers, and the **quads** of the mark are
//!   the lines of the **text under the region** — the tree's own
//!   rule, and the text under the region is what the mark covers.
//! - A region with **no text under it** is **refused**: the tree's own
//!   rule, and a mark over nothing marks nothing.
//! - The **colour** of the mark is the tree's own red when the caller
//!   says nothing, and it is three numbers between zero and one.
//! - The **fill** of the mark is the tree's own black: the tree fills
//!   the area it takes away.
//! - The **rectangle** of the annotation is the **bounds** of the
//!   quads: the tree's own rectangle, and it is not grown — a mark
//!   covers the text and nothing else.
//! - The **overlay text** is the text over the area, and the **size**
//!   of it is the tree's own ten points when the caller says nothing,
//!   and it is between one point and four hundred: the tree's own
//!   bounds.
//! - The **appearance of the overlay** is the tree's own string: the
//!   colour and the font and the size in one string, the `/DA` of the
//!   annotation.
//! - The **alignment** of the overlay is the tree's own three: left
//!   (`0`), centre (`1`) and right (`2`).
//! - The overlay **repeats** when the caller says so, and the file
//!   says so with the `/Repeat` of the annotation.
//!
//! # What it does not settle
//!
//! - The **drawing** of the mark is not here: it is the appearance
//!   command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.redact.add";

/// The tree's own **colour** of a redaction mark: the red.
const COLOUR: [f64; 3] = [0.89, 0.13, 0.13];

/// The tree's own **fill** of a redaction mark: the black.
const FILL: [f64; 3] = [0.0, 0.0, 0.0];

/// The tree's own **size** of the overlay text: ten points.
const SIZE: f64 = 10.0;

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

commands! {
    /// **Put** a **mark of a redaction** on a page.
    pdf.annot.redact.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the mark is on.
            page: NodeRef,
            /// The object number the annotation has in the file.
            object: Number,
            /// The region the mark covers: four numbers.
            region: Json,
            /// The text that is written over the area.
            text: Option<Text>,
            /// The colour of the mark: three numbers.
            colour: Option<Json>,
            /// The size of the overlay text, in points.
            size: Option<Number>,
            /// The alignment of the overlay text: `0` left, `1` centre, `2`
            /// right.
            align: Option<Number>,
            /// Whether the overlay text **repeats** over the area.
            repeat: Option<Bool>,
        },
        run: {
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
                quads.push(crate::markup::quad_of(bounds));
            }
            if quads.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "there is no text under the region".to_string(),
                });
            }
            // The **colour** of the mark: the tree's own red when nothing
            // is said.
            let colour = match &self.colour {
                Some(colour) => {
                    let numbers = numbers_of(colour);
                    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a mark is three numbers".to_string(),
                        });
                    }
                    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the colour of a mark is between zero and one".to_string(),
                        });
                    }
                    [numbers[0], numbers[1], numbers[2]]
                }
                None => COLOUR,
            };
            // The **size** of the overlay: the tree's own ten points when
            // nothing is said, and the tree's own bounds.
            let size = match self.size {
                Some(size) if !size.0.is_finite() => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of the overlay text is a number".to_string(),
                    });
                }
                Some(size) if !(1.0..=400.0).contains(&size.0) => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of the overlay text is between one point and four hundred".to_string(),
                    });
                }
                Some(size) => size.0,
                None => SIZE,
            };
            // The **alignment** of the overlay: the tree's own three.
            let align = match self.align {
                Some(align) if !align.0.is_finite() || align.0.fract() != 0.0 => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the alignment of the overlay text is a whole number".to_string(),
                    });
                }
                Some(align) if !(0.0..=2.0).contains(&align.0) => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the alignment of the overlay text is 0, 1 or 2".to_string(),
                    });
                }
                Some(align) => align.0 as i64,
                None => 0,
            };
            // The overlay **repeats** when the caller says so.
            let repeat = self.repeat.as_ref().map(|flag| flag.0).unwrap_or(false);
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
            // The page's own **annotations**, before the new one is made.
            let annots = crate::entries_of(&page.data)
                .into_iter()
                .find(|(key, _)| key == "Annots")
                .and_then(|(_, value)| value.as_array().cloned())
                .unwrap_or_default();
            // The **rectangle** of the mark: the bounds of the quads, and
            // it is not grown.
            let rect = crate::markup::bounds_of(&quads);
            // The **annotation** itself: the tree's own redaction mark.
            let flat: Vec<f64> = quads.iter().flat_map(|quad| quad.iter().copied()).collect();
            let mut entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Redact")),
                ("Rect".to_string(), json!(rect)),
                ("C".to_string(), json!(colour)),
                ("IC".to_string(), json!(FILL)),
                ("QuadPoints".to_string(), json!(flat)),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
            if let Some(text) = &self.text {
                entries.push(("OverlayText".to_string(), json!({ "string": text.0 })));
                // The **appearance of the overlay** is the tree's own
                // string.
                entries.push((
                    "DA".to_string(),
                    json!(format!(
                        "{} {} {} rg /Helv {} Tf",
                        crate::watermark::number(colour[0]),
                        crate::watermark::number(colour[1]),
                        crate::watermark::number(colour[2]),
                        crate::watermark::number(size)
                    )),
                ));
                entries.push(("Q".to_string(), json!(align)));
                if repeat {
                    entries.push(("Repeat".to_string(), json!(true)));
                }
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
                "subtype": "Redact",
                "rect": rect,
                "quads": quads,
                "colour": colour,
                "fill": FILL,
                "size": size,
                "align": align,
                "repeat": repeat,
            })))

        }
    }
}
