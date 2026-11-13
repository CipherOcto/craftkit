//! The **generated** items of a project.
//!
//! The tree's generators: an item that plays something the project
//! **makes** and not a file it reads — the black video, the
//! transparent video, the bars and the tone of a test card, the
//! universal counting leader and the colour matte.
//!
//! The tree's own rules:
//!
//! - a generator is **five seconds** long unless it says otherwise,
//!   and the bars and tone are **ten**;
//! - the colour matte is the tree's own **dark grey** unless it says
//!   otherwise;
//! - a generator is made at the **rate of the sequence**, and the
//!   tree's own default rate is `23.976`.
//!
//! The generator is an **item** of a bin, like any other item, and it
//! carries the kind it is: a clip that plays it says what it plays.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Colour, EngineError, NodeRef, Number, Outcome, Text};

/// The **generators** this tree has, and the name of each.
///
/// The names are the tree's own: `Black Video`, `Transparent Video`,
/// `Bars and Tone`, `Universal Counting Leader` and `Color Matte`. The
/// matte is also written `Colour Matte`, which is the same name in
/// another spelling.
pub const GENERATORS: [&str; 5] =
    ["Black Video", "Transparent Video", "Bars and Tone", "Universal Counting Leader", "Color Matte"];

/// The generator a name says, if it says one.
///
/// The name is read without regard to case, and `Colour Matte` is
/// `Color Matte` written the way this tree writes it.
pub fn generator_of(text: &str) -> Option<&'static str> {
    let text = text.trim();
    if text.eq_ignore_ascii_case("Colour Matte") {
        return Some("Color Matte");
    }
    GENERATORS.iter().find(|generator| generator.eq_ignore_ascii_case(text)).copied()
}

/// The **seconds** a generator is long, when it says nothing.
///
/// The tree's rule: five seconds, and the bars and tone are ten.
fn default_seconds(kind: &str) -> f64 {
    if kind == "Bars and Tone" { 10.0 } else { 5.0 }
}

/// The colour of the matte when it says nothing: the tree's own dark
/// grey.
const DEFAULT_MATTE: &str = "#1a1a1a";

commands! {
    /// Put a **generated** item in a bin.
    ///
    /// The item plays something the project makes and not a file it reads:
    /// the black video, the transparent video, the bars and the tone, the
    /// counting leader or the colour matte.
    film.item.generator {
        area: "film",
        layer: "film",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// What to call it.
            name: Text,
            /// The bin it goes in.
            bin: NodeRef,
            /// The item's own id, in the editor.
            item: Number,
            /// What kind of generator it is: `Black Video`, `Transparent
            /// Video`, `Bars and Tone`, `Universal Counting Leader` or `Color
            /// Matte`.
            kind: Text,
            /// How long it is, in seconds. The default is five, and the bars
            /// and the tone are ten.
            seconds: Option<Number>,
            /// The rate it is made at: `25`, `23.976`, … The default is the
            /// tree's own `23.976`.
            rate: Option<Text>,
            /// The colour of the matte: `#rrggbb`. The default is the tree's
            /// own dark grey, and a colour on another generator is refused.
            colour: Option<Text>,
        },
        run: {
            let id = "film.item.generator".to_string();
            let name = self.name.0.clone();
            if name.trim().is_empty() {
                return Err(EngineError::Failed { id, message: "an item has a name".to_string() });
            }
            let bin = self.bin.0;
            if session.document().node(bin).is_none() {
                return Err(EngineError::Failed { id, message: format!("there is no bin `{bin}`") });
            }
            let Some(kind) = generator_of(&self.kind.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "`{}` is not a generator: the generators are {}",
                        self.kind.0.trim(),
                        GENERATORS.join(", ")
                    ),
                });
            };
            // The **item's own id**, in the editor: it is a whole number
            // and it is there.
            let item = self.item.0;
            if !item.is_finite() || item.fract() != 0.0 || item < 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "an item's own id is a whole number".to_string(),
                });
            }
            // The **seconds**: the tree's default, and a number that is
            // there.
            let seconds = match self.seconds {
                Some(seconds) => {
                    let seconds = seconds.0;
                    if !seconds.is_finite() || seconds < 0.0 {
                        return Err(EngineError::Failed {
                            id,
                            message: format!(
                                "the length of a generator in seconds is a number that is there: {seconds}"
                            ),
                        });
                    }
                    seconds
                }
                None => default_seconds(kind),
            };
            // The **rate**: the tree's own, and the rate of the sequence
            // when it says one.
            let rate = match &self.rate {
                Some(rate) => crate::rates::rate_of(&rate.0),
                None => crate::rates::rate_of("23.976"),
            }
            .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            // The **duration** in frames: the seconds at the rate.
            let frames = (seconds * rate.0 as f64 / rate.1 as f64).round();
            if !frames.is_finite() || frames < 0.0 || frames > u64::MAX as f64 {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the length of a generator in seconds is a number that is there: {seconds}"),
                });
            }
            // The **colour**: the matte's own, and nobody else's.
            let colour = match &self.colour {
                Some(colour) => {
                    if kind != "Color Matte" {
                        return Err(EngineError::Failed {
                            id,
                            message: format!("a colour is the matte's own, and `{kind}` is not a matte"),
                        });
                    }
                    Colour::from_hex(&colour.0).map_err(|message| EngineError::Failed { id, message })?
                }
                None if kind == "Color Matte" => {
                    Colour::from_hex(DEFAULT_MATTE).map_err(|message| EngineError::Failed { id, message })?
                }
                None => Colour::NONE,
            };
            let data = json!({
                "item": item as u64,
                "path": Json::Null,
                "duration": frames as u64,
                "offline": false,
                "generator": {
                    "kind": kind,
                    "colour": format!("#{:02x}{:02x}{:02x}", (colour.0[0] * 255.0).round() as u8, (colour.0[1] * 255.0).round() as u8, (colour.0[2] * 255.0).round() as u8),
                },
            });
            let id_node =
                session.document_mut()?.add_node_with_under(Some(bin), name.clone(), "film.item", None, data);
            Ok(Outcome::changed(format!("added the generator `{name}`"))
                .with_data(json!({ "id": id_node, "item": item as u64, "name": name, "kind": kind, "duration": frames as u64 })))

        }
    }

    /// The **colour** of a colour matte is changed.
    ///
    /// The tree's rule: the colour must be `#rrggbb`, and the item must
    /// be a colour matte — an item that is not one is refused by name.
    ///
    /// The colour is the item's own, so every clip that plays the matte
    /// is changed with it.
    film.item.matte.colour {
        area: "film",
        layer: "film",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The item: the colour matte.
            item: NodeRef,
            /// The colour of the matte: `#rrggbb`.
            colour: Text,
        },
        run: {
            let id = "film.item.matte.colour".to_string();
            let item = self.item.0;
            let Some(node) = session.document().node(item) else {
                return Err(EngineError::Failed { id, message: format!("there is no item `{item}`") });
            };
            // The item must be a **colour matte**: the tree's own rule.
            let kind = node
                .data
                .get("generator")
                .and_then(|generator| generator.get("kind"))
                .and_then(|kind| kind.as_str());
            if kind != Some("Color Matte") {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the item `{item}` is not a Color Matte"),
                });
            }
            // The **colour**: `#rrggbb`, and nothing else.
            let colour =
                Colour::from_hex(&self.colour.0).map_err(|message| EngineError::Failed { id, message })?;
            let hex = format!(
                "#{:02x}{:02x}{:02x}",
                (colour.0[0] * 255.0).round() as u8,
                (colour.0[1] * 255.0).round() as u8,
                (colour.0[2] * 255.0).round() as u8
            );
            if let Some(node) = session.document_mut()?.node_mut(item) {
                node.data["generator"]["colour"] = json!(hex.clone());
            }
            Ok(Outcome::changed(format!("the matte is {hex}")).with_data(json!({ "item": item, "colour": hex })))

        }
    }
}
