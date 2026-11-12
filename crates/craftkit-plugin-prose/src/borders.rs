//! The **borders** of a page.
//!
//! A page may be drawn with a border, and the border has a **style**, a
//! **width** and a **colour**, and there is one of it for each of the
//! four sides of the page.
//!
//! # The rules
//!
//! - **The styles are the tree's own**: `single`, `double`, `dotted`,
//!   `dashed`, `thick`, `triple`, `dotDash` and `wave`, and a style
//!   that is not one of them is refused by name.
//! - **The width is in points**, from a **quarter** of a point to
//!   **six**. The tree clamps a width to that range and says nothing;
//!   here a width outside it is refused.
//! - **A colour is `#rrggbb`**, and a colour that is not one is
//!   refused by name. An **empty** colour takes the colour away, and
//!   the border has no colour of its own.
//! - **A border is set part by part.** A style that is not named
//!   leaves the style as it was, and a border that is new takes the
//!   tree's: `single`, a point wide, and no colour of its own.
//! - **A side may be named** — `top`, `bottom`, `left` or `right` —
//!   and a side that is not one of them is refused by name. A side
//!   that is given **nothing** takes no border at all.
//! - **The space between the border and the text is not here.** The
//!   tree's border keeps the text a certain distance away, and that
//!   distance is not a fact of this tree.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The **styles** a border may have: the tree's own.
pub const STYLES: [&str; 8] = ["single", "double", "dotted", "dashed", "thick", "triple", "dotDash", "wave"];

/// The **sides** a page has.
pub const SIDES: [&str; 4] = ["top", "bottom", "left", "right"];

/// The narrowest border, in points: a quarter of a point.
pub const THINNEST: f64 = 0.25;

/// The thickest border, in points.
pub const THICKEST: f64 = 6.0;

/// The width of a border that is not named, in points.
pub const DEFAULT_WIDTH: f64 = 1.0;

/// The **document** a border belongs to.
fn the_document(session: &Session, command: &str, document: u64) -> Result<(), EngineError> {
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no document `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    Ok(())
}

/// The **style** a name says, if it is one.
fn style_of(name: &str, id: &str) -> Result<&'static str, EngineError> {
    STYLES.iter().find(|style| style.eq_ignore_ascii_case(name.trim())).copied().ok_or_else(|| {
        EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no style `{name}`: the styles are {}", STYLES.join(", ")),
        }
    })
}

/// The **side** a name says, if it is one.
fn side_of(name: &str, id: &str) -> Result<&'static str, EngineError> {
    SIDES.iter().find(|side| side.eq_ignore_ascii_case(name.trim())).copied().ok_or_else(|| {
        EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no side `{name}`: the sides are {}", SIDES.join(", ")),
        }
    })
}

/// The **width** a number says, in points.
fn width_of(value: f64, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a width of points is a number that is there: {value}"),
        });
    }
    if !(THINNEST..=THICKEST).contains(&value) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a border is from {THINNEST} points to {THICKEST} points, and {value} is not"),
        });
    }
    Ok(value)
}

/// The **colour** a name says, if it is one. An empty colour is no
/// colour of its own.
fn colour_of(name: &str, id: &str) -> Result<Option<String>, EngineError> {
    let colour = name.trim();
    if colour.is_empty() {
        return Ok(None);
    }
    if let Err(why) = craftkit_sdk::Colour::from_hex(colour) {
        return Err(EngineError::Failed { id: id.to_string(), message: why });
    }
    Ok(Some(colour.to_string()))
}

/// The **borders** of a document, as they are kept.
pub fn borders_of(node: &craftkit_sdk::Node) -> Option<Json> {
    let borders = node.data.get("page_borders")?;
    let map = borders.as_object()?;
    if map.is_empty() {
        return None;
    }
    Some(borders.clone())
}

/// The border a side has now: the tree's own, when the side has none.
fn the_border(borders: &Json, side: &str) -> Json {
    let held = borders.get(side);
    let mut border = json!({ "style": "single", "width": DEFAULT_WIDTH });
    if let Some(held) = held {
        if let Some(style) = held.get("style").and_then(|style| style.as_str()) {
            border["style"] = json!(style);
        }
        if let Some(width) = held.get("width").and_then(|width| width.as_f64()) {
            border["width"] = json!(width);
        }
        if let Some(colour) = held.get("colour").and_then(|colour| colour.as_str()) {
            border["colour"] = json!(colour);
        }
    }
    border
}

/// **Set** the **borders** of the page of a document.
///
/// The `kind` gives every side of the page a border (`box`) or takes
/// them all away (`none`), and the `sides` names the sides one by one.
#[derive(Command)]
#[command(id = "prose.page.borders", area = "prose", layer = "prose", icon = "rect", mutates, undoable)]
pub struct SetBorders {
    /// The document.
    pub document: NodeRef,
    /// `box` for a border on every side, `none` for none at all.
    pub kind: Option<Text>,
    /// The style of the border.
    pub style: Option<Text>,
    /// The width of the border, in points.
    pub width: Option<Number>,
    /// The colour of the border, as `#rrggbb`.
    ///
    /// An empty colour takes the colour away.
    pub colour: Option<Text>,
    /// The borders of the sides named: `{"top": {…}, "left": null}`.
    pub sides: Option<Json>,
}

impl SetBorders {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.page.borders".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        if self.kind.is_none() && self.sides.is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a border is named or it is not".to_string(),
            });
        }
        if self.kind.is_some() && self.sides.is_some() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the kind and the sides are not both given".to_string(),
            });
        }
        // The **parts** that are named, and they are the same for
        // every side that is named.
        let style = match &self.style {
            Some(style) => Some(style_of(&style.0, &id)?),
            None => None,
        };
        let width = match &self.width {
            Some(width) => Some(width_of(width.0, &id)?),
            None => None,
        };
        let colour = match &self.colour {
            Some(colour) => Some(colour_of(&colour.0, &id)?),
            None => None,
        };
        // The **borders** as they are now.
        let mut borders: Json = session
            .document()
            .node(document)
            .and_then(|node| node.data.get("page_borders"))
            .cloned()
            .filter(|borders| borders.is_object())
            .unwrap_or_else(|| json!({}));
        match &self.kind {
            Some(kind) => {
                let kind = kind.0.trim().to_lowercase();
                match kind.as_str() {
                    "box" => {
                        for side in SIDES {
                            let border = the_border(&borders, side);
                            borders[side] = with_parts(&border, style, width, colour.clone());
                        }
                    }
                    "none" => {
                        for side in SIDES {
                            borders.as_object_mut().map(|map| map.remove(side));
                        }
                    }
                    other => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no kind `{other}`: the kinds are box and none"),
                        });
                    }
                }
            }
            None => {
                let Some(sides) = &self.sides else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a border is named or it is not".to_string(),
                    });
                };
                let Some(list) = sides.as_object() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the sides are a list of pairs".to_string(),
                    });
                };
                for (name, value) in list {
                    let side = side_of(name, &id)?;
                    if value.is_null() {
                        borders.as_object_mut().map(|map| map.remove(side));
                        continue;
                    }
                    let Some(given) = value.as_object() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the border of the {side} is a list of pairs, or nothing"),
                        });
                    };
                    let side_style = match given.get("style") {
                        Some(style) => Some(style_of(style.as_str().unwrap_or_default(), &id)?),
                        None => None,
                    };
                    let side_width = match given.get("width") {
                        Some(width) => Some(width_of(width.as_f64().unwrap_or_default(), &id)?),
                        None => None,
                    };
                    let side_colour = match given.get("colour") {
                        Some(colour) => Some(colour_of(colour.as_str().unwrap_or_default(), &id)?),
                        None => None,
                    };
                    let border = the_border(&borders, side);
                    borders[side] = with_parts(&border, side_style, side_width, side_colour);
                }
            }
        }
        // The **borders** are kept, and a page with no border at all
        // has none.
        let empty = borders.as_object().map(|map| map.is_empty()).unwrap_or(true);
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        if empty {
            node.data.as_object_mut().map(|map| map.remove("page_borders"));
        } else {
            node.data["page_borders"] = borders.clone();
        }
        Ok(Outcome::changed(format!("the page of the document `{document}` has its borders"))
            .with_data(json!({ "document": document, "borders": borders })))
    }
}

/// A border with the **parts** that are named, and the rest as they
/// were.
fn with_parts(
    border: &Json,
    style: Option<&str>,
    width: Option<f64>,
    colour: Option<Option<String>>,
) -> Json {
    let mut border = border.clone();
    if let Some(style) = style {
        border["style"] = json!(style);
    }
    if let Some(width) = width {
        border["width"] = json!(width);
    }
    match colour {
        Some(Some(colour)) => border["colour"] = json!(colour),
        Some(None) => {
            if let Some(map) = border.as_object_mut() {
                map.remove("colour");
            }
        }
        None => {}
    }
    border
}
