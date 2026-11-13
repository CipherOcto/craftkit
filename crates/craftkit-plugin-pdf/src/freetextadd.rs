//! The **text box** on a page: the tree's own free text.
//!
//! The tree's own: a **text box** is an annotation that shows its own
//! text — the **contents** of the annotation — inside a rectangle, and
//! the text is set in the tree's Helvetica. The tree draws it when the
//! annotation is made; here the annotation is made here and the
//! drawing is the appearance command's own work.
//!
//! # The rules
//!
//! - The **rectangle** is four numbers, the corners may come in any
//!   order, and it must be at least a point wide and a point high:
//!   the tree's own rule.
//! - The **size** of the text is the tree's own twelve points when
//!   the caller says nothing, and it is between one point and four
//!   hundred: the tree's own bounds.
//! - The **colour** of the text is the tree's own black when the
//!   caller says nothing, and it is three numbers between zero and
//!   one.
//! - The **background** of the box is the colour of the fill when the
//!   caller says one, and there is none when the caller says nothing:
//!   the tree's own rule.
//! - The **width** of the border is the tree's own zero — a text box
//!   has no border unless one is asked for — and a width below zero
//!   is the tree's own zero.
//! - The **alignment** is the tree's own three: left (`0`), centre
//!   (`1`) and right (`2`), and an alignment that is not one of them
//!   is **refused**.
//! - The **appearance of the text** is the tree's own string: the
//!   colour and the font and the size in one string, the `/DA` of the
//!   annotation — the tree's own Helvetica under the name `Helv`.
//!
//! # What it does not settle
//!
//! - The **drawing** of the text box is not here: it is the appearance
//!   command's own work.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.annot.freetext.add";

/// The tree's own **size** of the text: twelve points.
const SIZE: f64 = 12.0;

/// The tree's own **colour** of the text: the black.
const COLOUR: [f64; 3] = [0.0, 0.0, 0.0];

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<f64> {
    value.as_array().map(|list| list.iter().filter_map(|item| item.as_f64()).collect()).unwrap_or_default()
}

/// A **colour**: three numbers between zero and one.
fn the_colour(value: &Json) -> Result<[f64; 3], EngineError> {
    let numbers = numbers_of(value);
    if numbers.len() != 3 || numbers.iter().any(|number| !number.is_finite()) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is three numbers".to_string(),
        });
    }
    if numbers.iter().any(|number| !(0.0..=1.0).contains(number)) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is between zero and one".to_string(),
        });
    }
    Ok([numbers[0], numbers[1], numbers[2]])
}

commands! {
    /// **Put** a **text box** on a page.
    pdf.annot.freetext.add {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The page the text box is on.
            page: NodeRef,
            /// The object number the annotation has in the file.
            object: Number,
            /// The rectangle the box covers: four numbers.
            rect: Json,
            /// The text of the box.
            contents: Text,
            /// The size of the text, in points.
            size: Option<Number>,
            /// The colour of the text: three numbers.
            colour: Option<Json>,
            /// The colour of the background: three numbers.
            background: Option<Json>,
            /// The width of the border, in points.
            width: Option<Number>,
            /// The alignment of the text: `0` left, `1` centre, `2` right.
            align: Option<Number>,
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
            // The **rectangle**: four numbers, and the corners may come in
            // any order.
            let numbers = numbers_of(&self.rect);
            if numbers.len() != 4 || numbers.iter().any(|number| !number.is_finite()) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the rectangle of a text box is four numbers".to_string(),
                });
            }
            let rect = [
                numbers[0].min(numbers[2]),
                numbers[1].min(numbers[3]),
                numbers[0].max(numbers[2]),
                numbers[1].max(numbers[3]),
            ];
            // A text box that is too small is refused: the tree's own rule.
            if rect[2] - rect[0] < 1.0 || rect[3] - rect[1] < 1.0 {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "a text box is at least a point wide and a point high".to_string(),
                });
            }
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
            // The **colour** of the text: the tree's own black when
            // nothing is said.
            let colour = match &self.colour {
                Some(colour) => the_colour(colour)?,
                None => COLOUR,
            };
            // The **background** of the box.
            let background = match &self.background {
                Some(background) => Some(the_colour(background)?),
                None => None,
            };
            // The **width** of the border: the tree's own zero, and never
            // less than zero.
            let width = match self.width {
                Some(width) if !width.0.is_finite() => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the width of the border is a number".to_string(),
                    });
                }
                Some(width) => width.0.max(0.0),
                None => 0.0,
            };
            // The **alignment**: the tree's own three.
            let align = match self.align {
                Some(align) if !align.0.is_finite() || align.0.fract() != 0.0 => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the alignment of the text is a whole number".to_string(),
                    });
                }
                Some(align) if !(0.0..=2.0).contains(&align.0) => {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the alignment of the text is 0, 1 or 2".to_string(),
                    });
                }
                Some(align) => align.0 as i64,
                None => 0,
            };
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
            // The **appearance of the text** is the tree's own string: the
            // colour and the font and the size in one string.
            let appearance = format!(
                "{} {} {} rg /Helv {} Tf",
                crate::watermark::number(colour[0]),
                crate::watermark::number(colour[1]),
                crate::watermark::number(colour[2]),
                crate::watermark::number(size)
            );
            // The **annotation** itself: the tree's own text box.
            let mut entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("FreeText")),
                ("Rect".to_string(), json!(rect)),
                ("Contents".to_string(), json!({ "string": self.contents.0 })),
                ("DA".to_string(), json!(appearance)),
                ("Q".to_string(), json!(align)),
                ("P".to_string(), json!({ "ref": [page_number, 0] })),
            ];
            if let Some(background) = background {
                entries.push(("C".to_string(), json!(background)));
            }
            if width > 0.0 {
                entries.push(("BS".to_string(), json!({ "entries": [ ["W", width], ["S", "S"] ] })));
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
                "subtype": "FreeText",
                "rect": rect,
                "contents": self.contents.0,
                "size": size,
                "colour": colour,
                "background": background,
                "width": width,
                "align": align,
            })))

        }
    }
}
