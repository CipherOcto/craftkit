//! The **caption track** of a sequence.
//!
//! A caption track holds **cues**: the text that is on the screen and
//! the two times it is on it. The tree's caption track is a track of
//! its own, with a **format** and a **language**, and the captions on
//! it never overlap.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - The **format** of a caption track is one of the tree's: a
//!   subtitle, a CEA-608, a CEA-708 or a teletext. A format that is
//!   not one of them is refused by name, and the default is the
//!   subtitle.
//! - The **cues** of a track never **overlap**: a cue that ends after
//!   the next one starts is refused. A cue that ends exactly when the
//!   next one starts is not an overlap.
//! - A cue has a **positive duration**: it ends after it starts.
//! - The cues are kept in the order they **start** in.
//!
//! # The unit
//!
//! The times of a cue are **frames**, as the times of a clip are. The
//! tree keeps them in ticks, and the tree's tick is a fraction of a
//! frame.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The **formats** of a caption track: the tree's own.
pub const FORMATS: [&str; 4] = ["subtitle", "cea608", "cea708", "teletext"];

/// The format a caption track has when it is not given one: the
/// tree's own.
pub const DEFAULT_FORMAT: &str = "subtitle";

/// The most a cue's **text** may have.
pub const MAX_TEXT: usize = 4096;

/// The most **cues** a track may hold.
pub const MAX_CUES: usize = 10_000;

/// The **caption track** a command is about.
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

/// The **cues** of a track: the start, the end and the text of each.
fn cues_of(node: &Node) -> Vec<(u64, i64, i64, String)> {
    let mut cues: Vec<(u64, i64, i64, String)> = Vec::new();
    for child in node.children.iter().filter(|child| child.kind == "film.caption") {
        let start = child.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = child.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let text = child.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        cues.push((child.id, start, end, text));
    }
    cues.sort_by_key(|(_, start, _, _)| *start);
    cues
}

/// Add a **caption track** to a sequence.
#[derive(Command)]
#[command(id = "film.caption.track.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddCaptionTrack {
    /// What to call it.
    pub name: Text,
    /// The sequence it is in.
    pub sequence: NodeRef,
    /// The format of the track: `subtitle` when it is not given.
    pub format: Option<Text>,
}

impl AddCaptionTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.track.add";
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a track has a name".to_string(),
            });
        }
        let sequence = self.sequence.0;
        if session.document().node(sequence).is_none() {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no sequence `{sequence}`"),
            });
        }
        // The **format**: the tree's own, and a format that is not
        // one of them is refused by name.
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") => DEFAULT_FORMAT.to_string(),
            Some(format) if FORMATS.contains(&format) => format.to_string(),
            Some(other) => {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "`{other}` is not a format of a caption track: the formats are {}",
                        FORMATS.join(", ")
                    ),
                });
            }
        };
        let id = session.document_mut()?.add_node_with_under(
            Some(sequence),
            name.clone(),
            "film.caption.track",
            None,
            json!({ "format": format, "language": "en" }),
        );
        Ok(Outcome::changed(format!("added the caption track `{name}`"))
            .with_data(json!({ "id": id, "name": name, "format": format })))
    }
}

/// Add a **cue** to a caption track.
///
/// The cues of a track never overlap, and a cue that ends before it
/// starts is refused.
#[derive(Command)]
#[command(id = "film.caption.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddCaption {
    /// The caption track.
    pub track: NodeRef,
    /// The frame the cue starts at.
    pub start: Number,
    /// The frame the cue ends at.
    pub end: Number,
    /// The text of the cue.
    pub text: Text,
}

impl AddCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.add";
        let track = self.track.0;
        the_track(session, command, track)?;
        // The **times**: whole frames, and the end after the start.
        let frames = |value: Number| -> Result<i64, EngineError> {
            let number = value.0;
            if !number.is_finite() || number.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("a time is a whole number of frames: {number} is not one"),
                });
            }
            if number < 0.0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("a time is not before the start: {number} is"),
                });
            }
            Ok(number as i64)
        };
        let start = frames(self.start)?;
        let end = frames(self.end)?;
        if end <= start {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a cue ends after it starts".to_string(),
            });
        }
        let text = self.text.0.clone();
        if text.chars().count() > MAX_TEXT {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the text of a cue is not longer than {MAX_TEXT} characters"),
            });
        }
        // The **cues** of the track, and the overlap: the tree's own
        // rule.
        let node = the_track(session, command, track)?;
        let cues = cues_of(node);
        if cues.len() >= MAX_CUES {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("a track holds at most {MAX_CUES} cues"),
            });
        }
        for (_, held_start, held_end, _) in &cues {
            if start < *held_end && *held_start < end {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the cue from {start} to {end} overlaps a cue of the track"),
                });
            }
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(track),
            format!("{start}–{end}"),
            "film.caption",
            None,
            json!({ "start": start, "end": end, "text": text }),
        );
        Ok(Outcome::changed(format!("added a cue to the track `{track}`"))
            .with_data(json!({ "id": id, "track": track, "start": start, "end": end, "text": text })))
    }
}

/// The **cues** of a caption track, in the order they start in.
#[derive(Command)]
#[command(id = "film.caption.read", area = "film", layer = "film", icon = "list")]
pub struct ReadCaptions {
    /// The caption track.
    pub track: NodeRef,
}

impl ReadCaptions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.read";
        let track = self.track.0;
        let node = the_track(session, command, track)?;
        let format = node.data.get("format").and_then(|format| format.as_str()).unwrap_or(DEFAULT_FORMAT);
        let cues: Vec<Json> = cues_of(node)
            .into_iter()
            .map(|(id, start, end, text)| json!({ "id": id, "start": start, "end": end, "text": text }))
            .collect();
        Ok(Outcome::new(format!("the caption track `{track}` holds {} cue(s)", cues.len())).with_data(
            json!({
                "track": track,
                "format": format,
                "cues": cues,
                "count": cues.len(),
            }),
        ))
    }
}

/// Take a **cue** away from a caption track.
#[derive(Command)]
#[command(id = "film.caption.remove", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct RemoveCaption {
    /// The cue.
    pub caption: NodeRef,
}

impl RemoveCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.remove";
        let caption = self.caption.0;
        let Some(node) = session.document().node(caption) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no cue `{caption}`"),
            });
        };
        if node.kind != "film.caption" {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the node `{caption}` is a `{}` and not a cue", node.kind),
            });
        }
        session.document_mut()?.remove_node(caption);
        Ok(Outcome::changed(format!("the cue `{caption}` is gone")).with_data(json!({ "caption": caption })))
    }
}
