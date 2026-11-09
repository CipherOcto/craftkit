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

/// The **cues** of a track: the start, the end, the text and the
/// speaker of each.
fn cues_of(node: &Node) -> Vec<(u64, i64, i64, String, Option<String>)> {
    let mut cues: Vec<(u64, i64, i64, String, Option<String>)> = Vec::new();
    for child in node.children.iter().filter(|child| child.kind == "film.caption") {
        let start = child.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let end = child.data.get("end").and_then(|end| end.as_i64()).unwrap_or(0);
        let text = child.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        let speaker =
            child.data.get("speaker").and_then(|speaker| speaker.as_str()).map(|speaker| speaker.to_string());
        cues.push((child.id, start, end, text, speaker));
    }
    cues.sort_by_key(|(_, start, _, _, _)| *start);
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
        for (_, held_start, held_end, _, _) in &cues {
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
            .map(|(id, start, end, text, speaker)| {
                json!({ "id": id, "start": start, "end": end, "text": text, "speaker": speaker })
            })
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

/// **Import** a caption file as a new caption track of a sequence.
///
/// The tree's own rule: the cues of the file are **snapped to the
/// frames of the sequence**, and the track that is made is a new one —
/// named after the file, unless a name is given. And the cues of the
/// file that overlap each other are an error of the project, so the
/// whole import is refused and the sequence is left as it was.
#[derive(Command)]
#[command(id = "film.caption.import", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportCaptions {
    /// The sequence the track goes on.
    pub sequence: NodeRef,
    /// The file to read.
    pub path: Text,
    /// The format of the file: `subrip` when it is not given.
    pub format: Option<Text>,
    /// The name of the track: the name of the file when it is not
    /// given.
    pub name: Option<Text>,
}

/// The **frames** a number of milliseconds is, at the rate of a
/// sequence.
///
/// The tree's own rule: the cues of a caption file are snapped to the
/// frames of the sequence they go on.
fn frames_of(milliseconds: i64, num: i64, den: i64) -> i64 {
    // (ms * num) / (1000 * den), rounded to the nearest frame.
    let numerator = milliseconds.saturating_mul(num);
    let denominator = 1000i64.saturating_mul(den);
    if denominator <= 0 {
        return 0;
    }
    (numerator + denominator / 2) / denominator
}

impl ImportCaptions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.caption.import";
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no sequence `{sequence}`"),
            });
        };
        // The **rate** of the sequence: the frames a second.
        let rate = node.data.get("rate").cloned().unwrap_or(Json::Null);
        let (Some(num), Some(den)) =
            (rate.get("num").and_then(|num| num.as_i64()), rate.get("den").and_then(|den| den.as_i64()))
        else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the sequence `{sequence}` has no rate, and the frames are not known"),
            });
        };
        if num <= 0 || den <= 0 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the rate of the sequence `{sequence}` is not a rate"),
            });
        }
        // The **file** and the cues in it.
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), crate::captions::MAX_BYTES)
                .map_err(|message| EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the file cannot be read: {message}"),
                })?;
        // The **format** of the file: the one the command names, and
        // else the one the extension and the content of the file say.
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") => match crate::detect::detect(&path, &bytes) {
                Some(format) => format,
                None => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: format!("`{path}` is not a caption file of this tree"),
                    });
                }
            },
            Some("subrip") | Some("srt") => "subrip",
            Some("webvtt") | Some("vtt") => "webvtt",
            Some("stl") | Some("ebu") => "stl",
            Some("scc") => "scc",
            Some("mcc") => "mcc",
            Some("ttml") | Some("dfxp") | Some("xml") => "ttml",
            Some(other) => {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "`{other}` is not a format of a caption file: the formats are {}",
                        crate::captions::IMPORTED.join(", ")
                    ),
                });
            }
        };
        // The **cues** of the file: the formats that are read may be
        // imported, and the reading is the one of the reader.
        let (cues, warnings) = crate::captions::cues_of_file(format, &bytes).map_err(|message| {
            EngineError::Failed { id: command.to_string(), message: format!("{message}: `{path}`") }
        })?;
        let cues: Vec<(i64, i64, String)> =
            cues.into_iter().map(|cue| (cue.start, cue.end, cue.text)).collect();
        // The cues are **snapped to the frames** of the sequence.
        let mut snapped: Vec<(i64, i64, String)> = cues
            .into_iter()
            .map(|(start, end, text)| (frames_of(start, num, den), frames_of(end, num, den), text))
            .collect();
        snapped.sort_by_key(|(start, _, _)| *start);
        // The **overlap**: the tree checks the project when the track
        // is made, and a project with overlapping captions is not a
        // project. So the whole import is refused.
        for pair in snapped.windows(2) {
            if pair[0].1 > pair[1].0 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "the captions of the file overlap: one ends at {} and the next starts at {}",
                        pair[0].1, pair[1].0
                    ),
                });
            }
        }
        // The **track** is a new one, named after the file unless a
        // name is given, and it goes to the front of the sequence.
        let stem = std::path::Path::new(&path)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| "Captions".to_string());
        let name = self
            .name
            .as_ref()
            .map(|name| name.0.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or(stem);
        let track = session.document_mut()?.add_node_with_under(
            Some(sequence),
            name.clone(),
            "film.caption.track",
            None,
            json!({ "format": DEFAULT_FORMAT, "language": "en" }),
        );
        // The track goes to the **front** of the sequence: the tree's
        // own rule.
        if let Some(node) = session.document_mut()?.node_mut(sequence) {
            let mut children = node.children.clone();
            if let Some(index) = children.iter().position(|child| child.id == track) {
                let moved = children.remove(index);
                children.insert(0, moved);
            }
            node.children = children;
        }
        // The **cues** are on the track.
        for (start, end, text) in &snapped {
            session.document_mut()?.add_node_with_under(
                Some(track),
                format!("{start}-{end}"),
                "film.caption",
                None,
                json!({ "start": start, "end": end, "text": text }),
            );
        }
        Ok(Outcome::changed(format!("imported {} caption(s) into `{name}`", snapped.len())).with_data(
            json!({
                "sequence": sequence,
                "track": track,
                "name": name,
                "cues": snapped.len(),
                "warnings": warnings,
            }),
        ))
    }
}

/// **Export** the cues of a caption track as a caption file.
///
/// The tree's own rule: the captions of a track are written back as a
/// caption file, and the times of the track are the times of the
/// sequence they are on — the frames of the timeline are the
/// milliseconds of the file.
#[derive(Command)]
#[command(id = "film.captions.export", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ExportCaptions {
    /// The caption track.
    pub track: NodeRef,
    /// The file to write.
    pub path: Text,
    /// The format of the file: `subrip` when it is not given.
    pub format: Option<Text>,
}

impl ExportCaptions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "film.captions.export";
        let track = self.track.0;
        let node = the_track(session, command, track)?;
        // The **sequence** the track is on, and its **rate**: the
        // frames of the track are the milliseconds of the file.
        let sequence = session
            .document()
            .walk()
            .find(|node| node.children.iter().any(|child| child.id == track))
            .map(|node| node.id);
        let rate = sequence
            .and_then(|sequence| session.document().node(sequence))
            .and_then(|node| node.data.get("rate").cloned())
            .unwrap_or(Json::Null);
        let (Some(num), Some(den)) =
            (rate.get("num").and_then(|num| num.as_i64()), rate.get("den").and_then(|den| den.as_i64()))
        else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the sequence has no rate, and the times are not known".to_string(),
            });
        };
        if num <= 0 || den <= 0 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the rate of the sequence is not a rate".to_string(),
            });
        }
        // The **cues** of the track, in the order they start in. Their
        // times are the **frames** of the sequence.
        let cues = cues_of(node);
        let cues: Vec<crate::captions::Cue> = cues
            .into_iter()
            .map(|(_, start, end, text, _)| crate::captions::Cue {
                start: milliseconds_of(start, num, den),
                end: milliseconds_of(end, num, den),
                text,
            })
            .collect();
        // The **file** is written.
        let named = self.format.as_ref().map(|format| format.0.trim().to_ascii_lowercase());
        let format = match named.as_deref() {
            None | Some("") | Some("subrip") | Some("srt") => "subrip",
            Some("webvtt") | Some("vtt") => "webvtt",
            Some("stl") | Some("ebu") => "stl",
            Some("scc") => "scc",
            Some("mcc") => "mcc",
            Some("ttml") => "ttml",
            Some("dfxp") => "dfxp",
            Some(other) => {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!(
                        "`{other}` is not a format of a caption file: the formats are {}",
                        crate::captions::WRITABLE.join(", ")
                    ),
                });
            }
        };
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a file has a path".to_string(),
            });
        }
        let written = if format == "stl" {
            // The **EBU STL file**: the frames of the track are the
            // frames of the file, at the rate the tree's rule says.
            let cues: Vec<crate::stl::StlCue> = cues_of(node)
                .into_iter()
                .map(|(_, start, end, text, _)| crate::stl::StlCue { start, end, text })
                .collect();
            crate::stl::write(&cues, num, den)
        } else if format == "webvtt" {
            let cues: Vec<crate::captions::VttCue> = cues
                .iter()
                .map(|cue| crate::captions::VttCue {
                    start: cue.start,
                    end: cue.end,
                    id: None,
                    settings: String::new(),
                    speaker: None,
                    text: cue.text.clone(),
                })
                .collect();
            crate::captions::write_vtt(&cues, &[]).into_bytes()
        } else if format == "scc" {
            // The **SCC file** carries the frames of the format, which
            // are twenty-nine and ninety-seven a second whatever the
            // rate of the sequence is: each cue is rounded to the
            // nearest frame of the format.
            let cues: Vec<(i64, i64, String)> = cues
                .iter()
                .map(|cue| (crate::scc::frame_of(cue.start), crate::scc::frame_of(cue.end), cue.text.clone()))
                .collect();
            crate::scc::write(&cues, false).into_bytes()
        } else if format == "mcc" {
            // The **MCC file**: the CEA-608 pairs of the field one, in
            // a caption distribution packet a frame.
            let cues: Vec<(i64, i64, String)> = cues
                .iter()
                .map(|cue| (crate::scc::frame_of(cue.start), crate::scc::frame_of(cue.end), cue.text.clone()))
                .collect();
            crate::mcc::write(&cues, false).into_bytes()
        } else if format == "ttml" || format == "dfxp" {
            // The **TTML file**: the cues of the track with the times of
            // the rate of the sequence.
            let flavour =
                if format == "dfxp" { crate::ttml::Flavour::Dfxp } else { crate::ttml::Flavour::Imsc1 };
            crate::ttml::write(&cues, Some((num, den)), flavour).into_bytes()
        } else {
            crate::captions::write_subrip(&cues).into_bytes()
        };
        std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
            id: command.to_string(),
            message: format!("the file cannot be written: {error}"),
        })?;
        Ok(Outcome::changed(format!("`{path}` was written with {} cue(s)", cues.len())).with_data(json!({
            "track": track,
            "path": path,
            "format": format,
            "count": cues.len(),
            "bytes": written.len(),
        })))
    }
}

/// The **milliseconds** a number of frames is, at a rate.
fn milliseconds_of(frames: i64, num: i64, den: i64) -> i64 {
    (frames * den * 1000 + num / 2) / num
}
