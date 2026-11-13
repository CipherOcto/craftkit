//! The **key** of the session: what the music is in.
//!
//! The tree's sound editor has a **key change**: a place in the
//! session where the key changes, and the key it changes to. The
//! editor's ruler draws it, and nothing else in the tree reads it — it
//! is a fact for a person to look at.
//!
//! Here the key is a **node** of its own, in the ruler's row, with a
//! **place** on the timeline, and two commands reach it: one sets the
//! key at a place, and one reads the keys of the session.
//!
//! # The key is a number and a mode
//!
//! A key signature in a MIDI file is a **number of sharps** and a
//! **mode**: five sharps and a minor mode is B minor, and five sharps
//! and a major mode is B major. The number is what is kept, because it
//! is what a file carries; the **name** is derived from it, because a
//! name is for a person to read and the number is not.
//!
//! # Where the key sits
//!
//! The key is in **ticks**, like the tempo and the meter (§ the events
//! of the session), because a key is a musical fact and the music is
//! counted in ticks. The tree's key changes are in **samples**, and
//! that is the difference between the two trees (§ the time of the
//! session): the tree's keys do not move when time is inserted, and
//! these do not move either — for the same reason.
//!
//! # What this does not do
//!
//! - **It does not change the notes.** A key signature says what the
//!   music is in; it does not transpose anything, and it is not a
//!   command on a note. The tree's is the same, and the tree's own
//!   ruler is the only thing that reads it.
//! - **It does not name every spelling.** The table holds the fifteen
//!   keys of the circle of fifths and no more: a key spelled some other
//!   way — D sharp major as C sharp major — is a key the table does
//!   not have, and it is refused rather than guessed at.

use craftkit_derive::{NodeKind, commands};

use craftkit_sdk::document::{Canvas, Document, Fill, Node, Path, Point, Stroke};
use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{BlendMode, Colour, EngineError, Number, Outcome, Text};

use crate::timeline;

/// What a **key change** says.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KeyPayload {
    /// Where it happens, in ticks.
    pub(crate) tick: i64,
    /// How many sharps it carries: less than zero means flats.
    pub(crate) sharps: i8,
    /// Whether the key is a minor one.
    pub(crate) minor: bool,
}

impl KeyPayload {
    pub(crate) fn to_json(&self) -> Json {
        serde_json::json!({
            "tick": self.tick,
            "sharps": self.sharps,
            "minor": self.minor,
        })
    }

    pub(crate) fn from_json(value: &Json) -> Result<Self, String> {
        let sharps = value.get("sharps").and_then(|sharps| sharps.as_i64()).unwrap_or(0);
        if !(-7..=7).contains(&sharps) {
            return Err(format!(
                "a key signature carries from seven flats to seven sharps, and {sharps} is not one"
            ));
        }
        Ok(KeyPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            sharps: sharps as i8,
            minor: value.get("minor").and_then(|minor| minor.as_bool()).unwrap_or(false),
        })
    }
}

/// The kind of a **key change**.
#[derive(NodeKind)]
#[node_kind(kind = "sound.key", label = "Key", layer = "sound", evaluates, place)]
pub struct KeyKind;

impl KeyKind {
    /// How a key change is drawn: a **mark in the ruler**.
    ///
    /// The ruler is the row above the tracks, and it is where the
    /// things that are not on a track live: the tempo, the meter and
    /// the key.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::event_rect(document, node) else { return };
        let line = Path::polygon(&[Point::new(rect.x, rect.y), Point::new(rect.x, rect.y + rect.h)]);
        canvas.path(
            &line,
            Fill::Solid(Colour::NONE),
            BlendMode::Normal,
            Some(Stroke { colour: Colour::BLACK, width: 0.01, ..Default::default() }),
        );
    }

    /// Whether a point is inside a key change.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::event_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a key change **is**: a mark in the ruler.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::event_rect(document, node)
    }

    /// What a key change **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let key = KeyPayload::from_json(&node.data).ok()?;
        let name = craftkit_midi::names::key_name(key.sharps, key.minor);
        Some(serde_json::json!({
            "tick": key.tick,
            "sharps": key.sharps,
            "minor": key.minor,
            "name": name,
        }))
    }
}

commands! {
    /// Set the **key** of the session at a place.
    ///
    /// The key is a **name** — "G major", "E flat minor" — and it is
    /// written into the document as the number it stands for: the sharps
    /// and the mode. A second call at the same place changes the key that
    /// is there, the way the tree's command does.
    ///
    /// A name that is not a key of the table is **refused**: the tree
    /// takes any string at all and shows it, and a string that is not a
    /// key is a string nobody can read.
    sound.key.set {
        area: "sound",
        layer: "sound",
        icon: "music",
        mutates,
        undoable,
        menu: ("Event", "Key"),
        params: {
            /// The key, by name: "C major", "G major", "E flat minor".
            key: Text,
            /// Where the key changes, in ticks.
            ///
            /// Without it, the key changes at the beginning of the session.
            tick: Option<Number>,
        },
        run: {
            let id = "sound.key.set".to_string();
            let Some((sharps, minor)) = craftkit_midi::names::key_from_name(&self.key.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{}` is not a key this battery knows", self.key.0),
                });
            };
            let tick = match &self.tick {
                Some(tick) => crate::ticks(tick.0)?,
                None => 0,
            };
            let name = craftkit_midi::names::key_name(sharps, minor).unwrap_or_default();
            let payload = KeyPayload { tick, sharps, minor };
            let existing = crate::find_event(session, "sound.key", tick);
            let id_node = match existing {
                Some(existing) => {
                    if let Some(node) = session.document_mut()?.node_mut(existing) {
                        node.data = payload.to_json();
                        node.name = format!("key {name}");
                    }
                    existing
                }
                None => session.document_mut()?.add_node_with(
                    format!("key {name}"),
                    "sound.key",
                    None,
                    payload.to_json(),
                ),
            };
            Ok(Outcome::changed(format!("the key at tick {tick} is {name}")).with_data(
                serde_json::json!({ "id": id_node, "tick": tick, "key": name, "sharps": sharps, "minor": minor }),
            ))

        }
    }

    /// Read the **keys** of the session.
    ///
    /// The read gives every key change the session has, in the order they
    /// happen, and — when a place is asked for — the key that is in effect
    /// there: the last change at or before it.
    ///
    /// A session that has **no** key change says so, and it is not a
    /// failure: the tree's session has none until one is added, and the
    /// tree's ruler shows nothing there.
    sound.key.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
            /// The place to read the key at, in ticks.
            ///
            /// Without it, every key change is read and no key is singled out.
            tick: Option<Number>,
        },
        run: {
            let mut keys: Vec<(i64, i8, bool, u64)> = session
                .document()
                .walk()
                .filter(|node| node.kind == "sound.key")
                .filter_map(|node| {
                    let payload = KeyPayload::from_json(&node.data).ok()?;
                    Some((payload.tick, payload.sharps, payload.minor, node.id))
                })
                .collect();
            keys.sort_by_key(|(tick, sharps, minor, _)| (*tick, *sharps, *minor));

            let changes: Vec<Json> = keys
                .iter()
                .map(|(tick, sharps, minor, node)| {
                    serde_json::json!({
                        "id": node,
                        "tick": tick,
                        "sharps": sharps,
                        "minor": minor,
                        "name": craftkit_midi::names::key_name(*sharps, *minor),
                    })
                })
                .collect();

            // The key **in effect** at a place: the last change at or
            // before it. A place before the first change has no key, and
            // none is made up for it.
            let at = match &self.tick {
                Some(tick) => Some(crate::ticks(tick.0)?),
                None => None,
            };
            let current = match at {
                Some(at) => {
                    keys.iter().rev().find(|(tick, _, _, _)| *tick <= at).map(|(tick, sharps, minor, node)| {
                        serde_json::json!({
                            "id": node,
                            "tick": tick,
                            "sharps": sharps,
                            "minor": minor,
                            "name": craftkit_midi::names::key_name(*sharps, *minor),
                        })
                    })
                }
                None => None,
            };
            let count = changes.len();
            Ok(Outcome::new(format!("the session has {count} key changes"))
                .with_data(serde_json::json!({ "keys": changes, "current": current })))

        }
    }
}
