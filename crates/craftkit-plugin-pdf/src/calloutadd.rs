//! The **callout** on a page: the tree's own free text with a leader.
//!
//! The tree's own: a **callout** is a text box with a **leader** —
//! a line that points at something on the page. The leader has three
//! points: the **point** it starts at, the **knee** it turns at, and
//! the place where it **meets** the box.
//!
//! # The rules
//!
//! - The **box** is four numbers, the corners may come in any order,
//!   and it must be at least a point wide and a point high: the
//!   tree's own rule.
//! - The **knee** and the **point** are two numbers each.
//! - The place where the leader **meets** the box is the middle of
//!   the side of the box that faces the knee: the tree's own
//!   `callout_attach`.
//! - The **rectangle** of the annotation is the box and the leader
//!   together, grown by half the width of the line, by the **size of
//!   the arrow** and by one point more: the tree's own rectangle.
//! - The **leader** is the three points — the `/CL` of the annotation
//!   — and its end is an **open arrow**: the tree's own `/LE`.
//! - The **text box** is inside the rectangle by the `/RD` of the
//!   annotation: the tree's own inset.
//! - The **intent** is the tree's own: `FreeTextCallout`.
//!
//! # What it does not settle
//!
//! - The **drawing** of the callout is not here: it is the appearance
//!   command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.callout.add";

/// The tree's own **width** of the line of a callout: one point.
const WIDTH: f64 = 1.0;

/// The tree's own **colour** of the text of a callout: the black.
const COLOUR: [f64; 3] = [0.0, 0.0, 0.0];

/// The tree's own **size** of the text: twelve points.
const SIZE: f64 = 12.0;

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// A **point**: two numbers.
fn a_point(value: &Json) -> Option<[f64; 2]> {
    let numbers = numbers_of(value);
    if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
        return None;
    }
    Some([numbers[0], numbers[1]])
}

/// The **size of an arrowhead**: the tree's own `arrow_size` — six
/// points plus three times the width of the line.
fn the_arrow_size(width: f64) -> f64 {
    6.0 + 3.0 * width.max(0.0)
}

/// Where the leader **meets** the box: the tree's own
/// `callout_attach` — the middle of the side that faces the knee.
fn the_attach(box_of: [f64; 4], knee: [f64; 2]) -> [f64; 2] {
    let [x0, y0, x1, y1] = box_of;
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    if knee[0] < x0 {
        [x0, cy]
    } else if knee[0] > x1 {
        [x1, cy]
    } else if knee[1] > y1 {
        [cx, y1]
    } else {
        [cx, y0]
    }
}

commands! {
    /// **Put** a **callout** on a page.
    pdf.annot.callout.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the callout is on.

            page: NodeRef,
            /// The object number the annotation has in the file.

            object: Number,
            /// The box the text is in: four numbers.

            rect: Json,
            /// The point the leader turns at: two numbers.

            knee: Json,
            /// The point the leader starts at: two numbers.

            point: Json,
            /// The text of the callout.

            contents: Text,
            /// The size of the text, in points.

            size: Option<Number>,
        },
        run: {

            // The **box**: four numbers, and the corners may come in any
            // order.
            let numbers = numbers_of(&self.rect);
            if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the box of a callout is four numbers".to_string(),
                });
            }
            let box_of = [
                numbers[0].min(numbers[2]),
                numbers[1].min(numbers[3]),
                numbers[0].max(numbers[2]),
                numbers[1].max(numbers[3]),
            ];
            // A box that is too small is refused: the tree's own rule.
            if box_of[2] - box_of[0] < 1.0 || box_of[3] - box_of[1] < 1.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the box of a callout is at least a point wide and a point high".to_string(),
                });
            }
            // The **knee** and the **point**.
            let Some(knee) = a_point(&self.knee) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the knee of a callout is two numbers".to_string(),
                });
            };
            let Some(point) = a_point(&self.point) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the point of a callout is two numbers".to_string(),
                });
            };
            // The **size** of the text: the tree's own twelve points when
            // nothing is said, and the tree's own bounds.
            let size = match self.size {
                Some(size) if !size.0.is_finite() => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of the text is a number".to_string(),
                    });
                }
                Some(size) if !(1.0..=400.0).contains(&size.0) => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the size of the text is between one point and four hundred".to_string(),
                    });
                }
                Some(size) => size.0,
                None => SIZE,
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
            // The place where the leader **meets** the box.
            let attach = the_attach(box_of, knee);
            // The **rectangle**: the box and the leader together, grown by
            // half the width of the line, by the size of the arrow and by
            // one point more.
            let pad = WIDTH / 2.0 + the_arrow_size(WIDTH) + 1.0;
            // The **leader's** own bounds, grown by the pad: the tree's own
            // `grow` of the bounds of the knee and the point.
            let lead = [
                knee[0].min(point[0]) - pad,
                knee[1].min(point[1]) - pad,
                knee[0].max(point[0]) + pad,
                knee[1].max(point[1]) + pad,
            ];
            // The **rectangle** is the box and the leader's bounds together:
            // the tree's own rectangle. The box is not shrunk — only the
            // leader is grown.
            let rect =
                [box_of[0].min(lead[0]), box_of[1].min(lead[1]), box_of[2].max(lead[2]), box_of[3].max(lead[3])];
            // The **inset** of the text box inside the rectangle.
            let inset = [box_of[0] - rect[0], box_of[1] - rect[1], rect[2] - box_of[2], rect[3] - box_of[3]];
            // The **annotation** itself: the tree's own callout.
            let entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("FreeText")),
                ("Rect".to_string(), json!(rect)),
                ("Contents".to_string(), json!({ "string": self.contents.0 })),
                (
                    "DA".to_string(),
                    json!(format!(
                        "{} {} {} rg /Helv {} Tf",
                        crate::watermark::number(COLOUR[0]),
                        crate::watermark::number(COLOUR[1]),
                        crate::watermark::number(COLOUR[2]),
                        crate::watermark::number(size)
                    )),
                ),
                ("Q".to_string(), json!(0)),
                ("IT".to_string(), json!("FreeTextCallout")),
                ("CL".to_string(), json!([point[0], point[1], knee[0], knee[1], attach[0], attach[1]])),
                ("LE".to_string(), json!("OpenArrow")),
                ("RD".to_string(), json!(inset)),
                ("BS".to_string(), json!({ "entries": [ ["W", WIDTH], ["S", "S"] ] })),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
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
                "subtype": "FreeText",
                "rect": rect,
                "box": box_of,
                "knee": knee,
                "point": point,
                "attach": attach,
                "inset": inset,
                "size": size,
            })))

        }
    }
}
