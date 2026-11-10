//! The **look** of a field: the tree's own Appearance tab.
//!
//! The tree's editor has a **dialog** for a field, and its second tab
//! is the **Appearance** one: the border of the field, its fill, the
//! width of its line, the style of that line, the colour of its text,
//! the font of that text, the size of it, and where the field sits on
//! the page.
//!
//! The rules of the tree, ported by name:
//!
//! - `set_props` — the **locked** field takes only **unlocking**;
//! - the **look** is written on the **widgets** of the field — the
//!   annotation that shows the field on the page — and the **default
//!   appearance** is written on the **field**;
//! - the **colours** are the three numbers of a colour, and the
//!   **width of the line** is the tree's own bound;
//! - the **style of the line** is one of the tree's own five: the
//!   solid one, the dashed one, the beveled one, the inset one and the
//!   underline;
//! - the **check style** — the mark of a check box or of a radio
//!   button — belongs to those two kinds of field and to nothing else;
//! - the **rectangle** is four numbers, the corners may come in any
//!   order, and a field smaller than four points by four is refused.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::checkbox::CheckStyle;
use crate::properties::is_locked;

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.field.look.set";

/// The **smallest** a field may be: the tree's own rule, four points
/// by four.
const SMALLEST: f64 = 4.0;

/// The **most** the width of a line may be: the tree's own bound, and
/// a line that is wider than this is refused and not made thinner.
const WIDEST: f64 = 12.0;

/// The **most** the size of the text of a field may be: the tree's own
/// bound, and a size that is bigger than this is refused and not made
/// smaller.
const BIGGEST: f64 = 100.0;

/// The **style** of the line of a field: the tree's own five.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderStyle {
    /// The solid line.
    Solid,
    /// The dashed line.
    Dashed,
    /// The beveled line.
    Beveled,
    /// The inset line.
    Inset,
    /// The underline.
    Underline,
}

impl BorderStyle {
    /// Every style, in the tree's own order.
    pub const ALL: [BorderStyle; 5] = [
        BorderStyle::Solid,
        BorderStyle::Dashed,
        BorderStyle::Beveled,
        BorderStyle::Inset,
        BorderStyle::Underline,
    ];

    /// The **code** of the style: the name the file holds.
    pub fn code(self) -> &'static str {
        match self {
            BorderStyle::Solid => "S",
            BorderStyle::Dashed => "D",
            BorderStyle::Beveled => "B",
            BorderStyle::Inset => "I",
            BorderStyle::Underline => "U",
        }
    }

    /// The **label** of the style: what the tree's own menu says.
    pub fn label(self) -> &'static str {
        match self {
            BorderStyle::Solid => "Solid",
            BorderStyle::Dashed => "Dashed",
            BorderStyle::Beveled => "Beveled",
            BorderStyle::Inset => "Inset",
            BorderStyle::Underline => "Underline",
        }
    }

    /// The style a **label** names.
    pub fn parse(label: &str) -> Option<BorderStyle> {
        let label = label.trim();
        BorderStyle::ALL.into_iter().find(|style| style.label().eq_ignore_ascii_case(label))
    }
}

/// The **font** of the text of a field: the tree's own three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldFont {
    /// Helvetica.
    Helvetica,
    /// Times.
    Times,
    /// Courier.
    Courier,
}

impl FieldFont {
    /// Every font, in the tree's own order.
    pub const ALL: [FieldFont; 3] = [FieldFont::Helvetica, FieldFont::Times, FieldFont::Courier];

    /// The **name** the file holds: the name of the resource.
    pub fn resource(self) -> &'static str {
        match self {
            FieldFont::Helvetica => "Helv",
            FieldFont::Times => "TiRo",
            FieldFont::Courier => "Cour",
        }
    }

    /// The **label** of the font: what the tree's own menu says.
    pub fn label(self) -> &'static str {
        match self {
            FieldFont::Helvetica => "Helvetica",
            FieldFont::Times => "Times",
            FieldFont::Courier => "Courier",
        }
    }

    /// The font a **label** or a **resource name** names.
    pub fn parse(label: &str) -> Option<FieldFont> {
        let label = label.trim();
        FieldFont::ALL.into_iter().find(|font| {
            font.label().eq_ignore_ascii_case(label) || font.resource().eq_ignore_ascii_case(label)
        })
    }
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **pairs** of an inline dictionary.
fn dictionary_of(value: &Json) -> Vec<(String, Json)> {
    let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            Some((pair.first()?.as_str()?.to_string(), pair.get(1)?.clone()))
        })
        .collect()
}

/// A **colour**: the three numbers of it.
///
/// The numbers come from outside, so they are read and checked: a
/// number that is not between zero and one is refused, and the tree's
/// own clamp of it is not used here.
fn the_colour(value: &serde_json::Value) -> Result<[f64; 3], EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: "a colour is three numbers".to_string(),
        });
    };
    if list.len() != 3 {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("a colour is three numbers, and this one is {}", list.len()),
        });
    }
    let mut colour = [0.0; 3];
    for (index, item) in list.iter().enumerate() {
        let Some(number) = item.as_f64() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "a colour is three numbers".to_string(),
            });
        };
        if !number.is_finite() || !(0.0..=1.0).contains(&number) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the numbers of a colour are between zero and one".to_string(),
            });
        }
        colour[index] = number;
    }
    Ok(colour)
}

/// The **default appearance** of a field: the font, the size and the
/// colour of its text.
///
/// The tree's own `parse_da`: the font is the name before the `Tf`,
/// the size is the number before it, and the colour is the three
/// numbers after it.
fn the_da(da: &str) -> (String, f64, [f64; 3]) {
    let tokens: Vec<&str> = da.split_whitespace().collect();
    let at = tokens.iter().position(|token| *token == "Tf").unwrap_or(0);
    let font = at
        .checked_sub(2)
        .and_then(|index| tokens.get(index))
        .and_then(|token| token.strip_prefix('/'))
        .unwrap_or("Helv")
        .to_string();
    let size = at
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .and_then(|token| token.parse::<f64>().ok())
        .filter(|size| size.is_finite() && *size > 0.0)
        .unwrap_or(12.0);
    let mut colour = [0.0; 3];
    for (index, slot) in colour.iter_mut().enumerate() {
        *slot = tokens
            .get(at + 1 + index)
            .and_then(|token| token.parse::<f64>().ok())
            .filter(|number| number.is_finite())
            .unwrap_or(0.0);
    }
    (font, size, colour)
}

/// The **default appearance** of a field, as the file writes it.
///
/// The tree's own `da_string`: the font, the size, and the colour of
/// the text.
fn the_da_string(font: &str, size: f64, colour: [f64; 3]) -> String {
    format!(
        "/{font} {} Tf {} {} {} rg",
        crate::watermark::number(size),
        crate::watermark::number(colour[0]),
        crate::watermark::number(colour[1]),
        crate::watermark::number(colour[2])
    )
}

/// **Set** the **look** of a field.
///
/// The look that is not said is left as it was.
///
/// The rules:
///
/// - the node must be a **field** of a form;
/// - a **locked** field takes only unlocking;
/// - a **colour** is three numbers, and each of them is between zero
///   and one;
/// - the **width of the line** is between zero and twelve points;
/// - the **style of the line** is one of the tree's own five;
/// - the **font** is one of the tree's own three;
/// - the **size of the text** is between zero and a hundred points;
/// - the **check style** belongs to a check box or a radio button and
///   to nothing else;
/// - the **rectangle** is four numbers, the corners may come in any
///   order, and a field smaller than four points by four is refused.
#[derive(Command)]
#[command(id = "pdf.field.look.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetLook {
    /// The field.
    pub field: NodeRef,
    /// The **colour of the border**.
    pub border: Option<serde_json::Value>,
    /// The **colour of the background**.
    pub fill: Option<serde_json::Value>,
    /// The **width of the line**, in points.
    pub width: Option<Number>,
    /// The **style of the line**: `Solid`, `Dashed`, `Beveled`,
    /// `Inset` or `Underline`.
    pub style: Option<Text>,
    /// The **colour of the text**.
    pub text_colour: Option<serde_json::Value>,
    /// The **font of the text**: `Helvetica`, `Times` or `Courier`.
    pub font: Option<Text>,
    /// The **size of the text**, in points.
    pub font_size: Option<Number>,
    /// The **mark** of a check box or of a radio button: `Check`,
    /// `Circle`, `Cross`, `Diamond`, `Square` or `Star`.
    pub check_style: Option<Text>,
    /// The **rectangle** the field covers on the page: four numbers.
    pub rect: Option<serde_json::Value>,
}

impl SetLook {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let field = self.field.0;
        let Some(node) = session.document().node(field).cloned() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{field}`"),
            });
        };
        let Some(found) = crate::fields::field_of(session.document(), &node) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{field}` is not a field"),
            });
        };
        // A **locked** field takes only unlocking.
        if is_locked(session.document(), &node) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the field is locked: unlock it first".to_string(),
            });
        }
        // The **colours**.
        let border = match &self.border {
            Some(value) => Some(the_colour(value)?),
            None => None,
        };
        let fill = match &self.fill {
            Some(value) => Some(the_colour(value)?),
            None => None,
        };
        let text_colour = match &self.text_colour {
            Some(value) => Some(the_colour(value)?),
            None => None,
        };
        // The **width of the line**.
        let width = match self.width.as_ref().map(|value| value.0) {
            Some(value) => {
                if !value.is_finite() || !(0.0..=WIDEST).contains(&value) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the width of a line is between zero and {WIDEST} points"),
                    });
                }
                Some(value)
            }
            None => None,
        };
        // The **style of the line**.
        let style = match &self.style {
            Some(text) => Some(BorderStyle::parse(&text.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a style of a line; the tree's own are Solid, Dashed, Beveled, Inset and Underline",
                    text.0
                ),
            })?),
            None => None,
        };
        // The **font of the text**.
        let font = match &self.font {
            Some(text) => Some(FieldFont::parse(&text.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a font; the tree's own are Helvetica, Times and Courier",
                    text.0
                ),
            })?),
            None => None,
        };
        // The **size of the text**.
        let font_size = match self.font_size.as_ref().map(|value| value.0) {
            Some(value) => {
                if !value.is_finite() || !(0.0..=BIGGEST).contains(&value) {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!(
                            "the size of the text of a field is between zero and {BIGGEST} points"
                        ),
                    });
                }
                Some(value)
            }
            None => None,
        };
        // The **mark of a check box**: only a check box or a radio
        // button has one.
        let check_style = match &self.check_style {
            Some(text) => {
                let style = CheckStyle::parse(&text.0).ok_or_else(|| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!(
                        "`{}` is not a style of a mark; the tree's own are Check, Circle, Cross, Diamond, Square and Star",
                        text.0
                    ),
                })?;
                let is_box = found.kind == crate::fields::FieldKind::Button;
                if !is_box {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "only check boxes and radio buttons have a check style".to_string(),
                    });
                }
                Some(style)
            }
            None => None,
        };
        // The **rectangle**: the corners may come in any order.
        let rect = match &self.rect {
            Some(value) => {
                let rect = crate::the_rect(value, COMMAND)?;
                if rect[2] - rect[0] < SMALLEST || rect[3] - rect[1] < SMALLEST {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the field is too small".to_string(),
                    });
                }
                Some(rect)
            }
            None => None,
        };
        // The **widgets** of the field: the kids that have no name of
        // their own — and a field that has none is its own widget.
        let kids =
            entry(&entries_of(&node), "Kids").and_then(|kids| kids.as_array().cloned()).unwrap_or_default();
        let mut widgets: Vec<u64> = Vec::new();
        for kid in &kids {
            let Some(number) = crate::number_of(kid) else { continue };
            let Some(kid_node) = node_of_number(session.document(), number) else { continue };
            if entry(&entries_of(kid_node), "T").is_some() {
                continue;
            }
            widgets.push(number);
        }
        let own = widgets.is_empty();
        if own {
            widgets.push(node.data.get("object").and_then(|object| object.as_u64()).unwrap_or(0));
        }
        // The **look** is written on the widgets — and a field that is
        // its own widget has the look in its own dictionary.
        let mut entries = entries_of(&node);
        let put = |entries: &mut Vec<(String, Json)>, key: &str, value: Json| match entries
            .iter_mut()
            .find(|(name, _)| name == key)
        {
            Some(entry) => entry.1 = value,
            None => entries.push((key.to_string(), value)),
        };
        for widget in &widgets {
            let mut widget_entries = if own {
                entries.clone()
            } else {
                let Some(widget_node) = node_of_number(session.document(), *widget) else { continue };
                entries_of(widget_node)
            };
            // The **colours**: the border and the background.
            if border.is_some() || fill.is_some() || check_style.is_some() {
                let mut mark = match entry(&widget_entries, "MK") {
                    Some(value) => dictionary_of(value),
                    None => Vec::new(),
                };
                if let Some(border) = border {
                    put(&mut mark, "BC", json!(border));
                }
                if let Some(fill) = fill {
                    put(&mut mark, "BG", json!(fill));
                }
                if let Some(style) = check_style {
                    put(&mut mark, "CA", json!({ "string": style.code() }));
                }
                put(
                    &mut widget_entries,
                    "MK",
                    json!({ "entries": mark.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>() }),
                );
            }
            // The **line**: its width and its style.
            if width.is_some() || style.is_some() {
                let mut line = match entry(&widget_entries, "BS") {
                    Some(value) => dictionary_of(value),
                    None => Vec::new(),
                };
                if let Some(width) = width {
                    put(&mut line, "W", json!(width));
                }
                if let Some(style) = style {
                    put(&mut line, "S", json!(style.code()));
                    match style {
                        BorderStyle::Dashed => put(&mut line, "D", json!([3])),
                        _ => line.retain(|(key, _)| key != "D"),
                    }
                }
                put(
                    &mut widget_entries,
                    "BS",
                    json!({ "entries": line.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>() }),
                );
            }
            // The **rectangle**.
            if let Some(rect) = rect {
                put(&mut widget_entries, "Rect", json!(rect));
            }
            if own {
                entries = widget_entries;
                continue;
            }
            let Some(widget_node) = node_of_number(session.document(), *widget) else { continue };
            let id = widget_node.id;
            let data = json!({
                "object": widget_node.data.get("object").cloned().unwrap_or(Json::Null),
                "entries": widget_entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
            });
            if let Some(node) = session.document_mut()?.node_mut(id) {
                node.data = data;
            }
        }
        // The **default appearance** is written on the field.
        if font.is_some() || font_size.is_some() || text_colour.is_some() {
            let current =
                entry(&entries, "DA").and_then(text_of).unwrap_or_else(|| "/Helv 12 Tf 0 g".to_string());
            let (held_font, held_size, held_colour) = the_da(&current);
            let font = font.map(|font| font.resource().to_string()).unwrap_or(held_font);
            let size = font_size.unwrap_or(held_size);
            let colour = text_colour.unwrap_or(held_colour);
            let da = the_da_string(&font, size, colour);
            match entries.iter_mut().find(|(key, _)| key == "DA") {
                Some(entry) => entry.1 = json!({ "string": da }),
                None => entries.push(("DA".to_string(), json!({ "string": da }))),
            }
        }
        let data = json!({
            "object": node.data.get("object").cloned().unwrap_or(Json::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        if let Some(node) = session.document_mut()?.node_mut(field) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the field `{field}` has its look")).with_data(json!({
            "field": field,
            "border": border,
            "fill": fill,
            "width": width,
            "style": style.map(|style| style.label()),
            "text_colour": text_colour,
            "font": font.map(|font| font.label()),
            "font_size": font_size,
            "check_style": check_style.map(|style| style.label()),
            "rect": rect,
        })))
    }
}
