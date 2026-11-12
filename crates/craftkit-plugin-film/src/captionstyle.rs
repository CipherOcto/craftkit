//! The **style** of a caption track, and the **track** itself.
//!
//! A caption track carries a **style**: the font the cues are written
//! in, the colour, the box behind them and where they stand. The
//! track itself carries a **name**, a **format**, a **language** and
//! three **flags**.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **format** of a track is one of the tree's four, and a
//!   format that is not one of them is refused by name.
//! - The **language** of a track is a tag, and it says something.
//! - A track may be **enabled** — shown in the monitor — and it may be
//!   **locked**, and it may **follow** the inserts and the extracts of
//!   the other tracks.
//! - The **style** has a **font**, a **size**, a **colour**, a
//!   **background** and the colour of it, an **align** — left, centre
//!   or right — an **anchor** — top, middle or bottom — a **margin**,
//!   a **line spacing**, an **outline** and the colour of it. An
//!   align and an anchor that are not one of the tree's are refused by
//!   name.
//! - A **reset** puts the style back as it was: the font `Inter`, the
//!   size fifty-four, white on a black box, centred at the bottom.
//!
//! # What it does not do
//!
//! - **The tree clamps the numbers** of the style — the size, the
//!   outline, the margin and the line spacing — into their ranges.
//!   Here a number outside its range is **refused**, as every other
//!   number of this tree is.
//! - **Nothing draws the captions here.** The style is a fact of the
//!   track, and the tree's renderer is the thing that draws it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The **smallest** a caption may be, in the tree's pixels.
const MIN_SIZE: f64 = 4.0;

/// The **largest** a caption may be, in the tree's pixels.
const MAX_SIZE: f64 = 400.0;

/// The **widest** an outline may be, in the tree's pixels.
const MAX_OUTLINE: f64 = 40.0;

/// The **largest** a margin may be, as a fraction of the height of the
/// frame.
const MAX_MARGIN: f64 = 0.45;

/// The **smallest** a line spacing may be.
const MIN_LINE_SPACING: f64 = 0.8;

/// The **largest** a line spacing may be.
const MAX_LINE_SPACING: f64 = 4.0;

/// The **aligns** of a caption: the tree's own.
pub const ALIGNS: [&str; 3] = ["left", "center", "right"];

/// The **anchors** of a caption: the tree's own.
pub const ANCHORS: [&str; 3] = ["top", "middle", "bottom"];

/// The style a caption track has when it has none: the tree's own.
pub fn default_style() -> Json {
    json!({
        "font": "Inter",
        "size": 54.0,
        "colour": "#ffffffff",
        "background": true,
        "background_colour": "#000000bf",
        "align": "center",
        "anchor": "bottom",
        "margin": 0.08,
        "line_spacing": 1.25,
        "outline": 0.0,
        "outline_colour": "#000000ff",
    })
}

/// The **style** of a track.
fn style_of(track: &Node) -> Json {
    track.data.get("style").cloned().unwrap_or_else(default_style)
}

/// The **track** a command is about.
fn the_track<'a>(session: &'a Session, command: &str, track: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(track) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no caption track `{track}`"),
        });
    };
    if node.kind != "film.caption.track" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{track}` is a `{}` and not a caption track", node.kind),
        });
    }
    Ok(node)
}

/// A **colour**, read the way a person writes one: `#rrggbb` or
/// `#rrggbbaa`.
fn colour_of(text: &str, command: &str) -> Result<String, EngineError> {
    let trimmed = text.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    let letters = hex.chars().all(|character| character.is_ascii_hexdigit());
    if !letters || !(hex.len() == 6 || hex.len() == 8) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("`{trimmed}` is not a colour: a colour is written #rrggbb or #rrggbbaa"),
        });
    }
    Ok(format!("#{}", hex.to_ascii_lowercase()))
}

/// A **number** inside its range, or refused by name.
fn number_in(command: &str, name: &str, value: f64, lowest: f64, highest: f64) -> Result<f64, EngineError> {
    if !value.is_finite() || value < lowest || value > highest {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("`{name}` is between {lowest} and {highest}, and {value} is not one"),
        });
    }
    Ok(value)
}

commands! {
    /// **Set** the track of the captions: its name, its format, its
    /// language and its flags.
    film.caption.track.set {
        area: "film",
        layer: "film",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The caption track.

            track: NodeRef,
            /// Its new name.

            name: Option<Text>,
            /// Its format: `subtitle`, `cea608`, `cea708` or `teletext`.

            format: Option<Text>,
            /// Its language: a tag such as `en` or `fr-CA`.

            language: Option<Text>,
            /// Whether it is shown in the monitor.

            enabled: Option<Bool>,
            /// Whether it may be changed.

            locked: Option<Bool>,
            /// Whether it follows the inserts and the extracts of the other
            /// tracks.

            sync_lock: Option<Bool>,
        },
        run: {

            let command = "film.caption.track.set";
            let track = self.track.0;
            the_track(session, command, track)?;
            // The **name**: it says something.
            let name = self.name.as_ref().map(|name| name.0.trim().to_string());
            if let Some(name) = name.as_ref()
                && name.is_empty()
            {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a caption track has a name".to_string(),
                });
            }
            // The **format**: one of the tree's four.
            let format = match self.format.as_ref() {
                Some(format) => {
                    let format = format.0.trim().to_ascii_lowercase();
                    if !crate::captiontracks::FORMATS.contains(&format.as_str()) {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!(
                                "`{format}` is not a format of a caption track: the formats are {}",
                                crate::captiontracks::FORMATS.join(", ")
                            ),
                        });
                    }
                    Some(format)
                }
                None => None,
            };
            // The **language**: a tag that says something.
            let language = self.language.as_ref().map(|language| language.0.trim().to_string());
            if let Some(language) = language.as_ref()
                && language.is_empty()
            {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a caption track has a language".to_string(),
                });
            }
            let enabled = self.enabled.map(|flag| flag.0);
            let locked = self.locked.map(|flag| flag.0);
            let sync_lock = self.sync_lock.map(|flag| flag.0);
            if let Some(node) = session.document_mut()?.node_mut(track) {
                if let Some(name) = name.clone() {
                    node.name = name.clone();
                    node.data["name"] = json!(name);
                }
                if let Some(format) = format.clone() {
                    node.data["format"] = json!(format);
                }
                if let Some(language) = language.clone() {
                    node.data["language"] = json!(language);
                }
                if let Some(enabled) = enabled {
                    node.data["enabled"] = json!(enabled);
                }
                if let Some(locked) = locked {
                    node.data["locked"] = json!(locked);
                }
                if let Some(sync_lock) = sync_lock {
                    node.data["sync_lock"] = json!(sync_lock);
                }
            }
            Ok(Outcome::changed(format!("the caption track `{track}` is set")).with_data(json!({
                "track": track,
                "name": name,
                "format": format,
                "language": language,
                "enabled": enabled,
                "locked": locked,
                "sync_lock": sync_lock,
            })))

        }
    }
    /// **Set** the style of a caption track.
    ///
    /// A number outside its range is refused, and a **reset** puts the
    /// style back as it was.
    film.caption.track.style {
        area: "film",
        layer: "film",
        icon: "text",
        mutates,
        undoable,
        params: {
            /// The caption track.

            track: NodeRef,
            /// The font the cues are written in.

            font: Option<Text>,
            /// The size of the cues, in the tree's pixels.

            size: Option<Number>,
            /// The colour of the cues.

            colour: Option<Text>,
            /// Whether a box is drawn behind the cues.

            background: Option<Bool>,
            /// The colour of the box.

            background_colour: Option<Text>,
            /// `left`, `center` or `right`.

            align: Option<Text>,
            /// `top`, `middle` or `bottom`.

            anchor: Option<Text>,
            /// The room from the edge, as a fraction of the height of the
            /// frame.

            margin: Option<Number>,
            /// The line spacing, as a multiple of the size.

            line_spacing: Option<Number>,
            /// The width of the outline, in the tree's pixels.

            outline: Option<Number>,
            /// The colour of the outline.

            outline_colour: Option<Text>,
            /// Whether the style is put back as it was.

            reset: Option<Bool>,
        },
        run: {

            let command = "film.caption.track.style";
            let track = self.track.0;
            let node = the_track(session, command, track)?;
            let mut style = style_of(node);
            // The **reset**: the style as it was.
            if self.reset.map(|flag| flag.0).unwrap_or(false) {
                style = default_style();
            }
            // The **font**: it says something.
            let font = self.font.as_ref().map(|font| font.0.trim().to_string());
            if let Some(font) = font.as_ref()
                && font.is_empty()
            {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a font says what it is".to_string(),
                });
            }
            // The **numbers**: each one inside its range.
            let size = match self.size {
                Some(size) => Some(number_in(command, "size", size.0, MIN_SIZE, MAX_SIZE)?),
                None => None,
            };
            let outline = match self.outline {
                Some(outline) => Some(number_in(command, "outline", outline.0, 0.0, MAX_OUTLINE)?),
                None => None,
            };
            let margin = match self.margin {
                Some(margin) => Some(number_in(command, "margin", margin.0, 0.0, MAX_MARGIN)?),
                None => None,
            };
            let line_spacing = match self.line_spacing {
                Some(spacing) => {
                    Some(number_in(command, "lineSpacing", spacing.0, MIN_LINE_SPACING, MAX_LINE_SPACING)?)
                }
                None => None,
            };
            // The **colours**.
            let colour = match self.colour.as_ref() {
                Some(colour) => Some(colour_of(&colour.0, command)?),
                None => None,
            };
            let background_colour = match self.background_colour.as_ref() {
                Some(colour) => Some(colour_of(&colour.0, command)?),
                None => None,
            };
            let outline_colour = match self.outline_colour.as_ref() {
                Some(colour) => Some(colour_of(&colour.0, command)?),
                None => None,
            };
            // The **align** and the **anchor**: one of the tree's.
            let align = match self.align.as_ref() {
                Some(align) => {
                    let align = align.0.trim().to_ascii_lowercase();
                    if !ALIGNS.contains(&align.as_str()) {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("unknown align `{align}`: the aligns are {}", ALIGNS.join(", ")),
                        });
                    }
                    Some(align)
                }
                None => None,
            };
            let anchor = match self.anchor.as_ref() {
                Some(anchor) => {
                    let anchor = anchor.0.trim().to_ascii_lowercase();
                    if !ANCHORS.contains(&anchor.as_str()) {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("unknown anchor `{anchor}`: the anchors are {}", ANCHORS.join(", ")),
                        });
                    }
                    Some(anchor)
                }
                None => None,
            };
            let background = self.background.map(|flag| flag.0);
            // The **style** is put back on the track.
            if let Some(map) = style.as_object_mut() {
                if let Some(font) = font {
                    map.insert("font".to_string(), json!(font));
                }
                if let Some(size) = size {
                    map.insert("size".to_string(), json!(size));
                }
                if let Some(colour) = colour {
                    map.insert("colour".to_string(), json!(colour));
                }
                if let Some(background) = background {
                    map.insert("background".to_string(), json!(background));
                }
                if let Some(background_colour) = background_colour {
                    map.insert("background_colour".to_string(), json!(background_colour));
                }
                if let Some(align) = align {
                    map.insert("align".to_string(), json!(align));
                }
                if let Some(anchor) = anchor {
                    map.insert("anchor".to_string(), json!(anchor));
                }
                if let Some(margin) = margin {
                    map.insert("margin".to_string(), json!(margin));
                }
                if let Some(line_spacing) = line_spacing {
                    map.insert("line_spacing".to_string(), json!(line_spacing));
                }
                if let Some(outline) = outline {
                    map.insert("outline".to_string(), json!(outline));
                }
                if let Some(outline_colour) = outline_colour {
                    map.insert("outline_colour".to_string(), json!(outline_colour));
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(track) {
                node.data["style"] = style.clone();
            }
            Ok(Outcome::changed(format!("the style of the caption track `{track}` is set")).with_data(json!({
                "track": track,
                "style": style,
            })))

        }
    }
}
