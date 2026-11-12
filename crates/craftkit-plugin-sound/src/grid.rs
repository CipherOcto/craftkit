//! The **grid**: the lines a clip is put on.
//!
//! The tree's sound editor has a grid — a division of the bar — and a
//! command that puts a clip on it. v2 had a tempo map and a beat and no
//! grid, so nothing here snapped a clip to anything.
//!
//! # Where the grid lives
//!
//! The tree keeps the grid in the **edit state**: beside the tool, the
//! edit mode and the selection. And the edit state is saved with the
//! session, so the tree's file does say what grid the session was made
//! on — in a place where a reader would not think to look, among the
//! things the editor was doing.
//!
//! Here the grid is a setting of the **session**, with the sample rate
//! and the timecode, so a file says what it is without saying what
//! tool was selected when it was written.
//!
//! # What the grid is
//!
//! A **note value** — a bar, a half, a quarter, an eighth, a sixteenth,
//! a thirty-second, a sixty-fourth — and two flags: **dotted** and
//! **triplet**. The lines are **musical**, so they move with the tempo
//! and follow the meter, and they are counted **from the bar line**, so
//! that a bar of an odd number of beats does not make the grid drift.
//!
//! # What the tree's absolute grids are not
//!
//! The tree's grid can also be a number of seconds, a number of frames
//! or a number of samples — grids for a timeline that is not musical.
//! There is no such grid here: a sound document is a bar of beats and
//! a grid in it is a division of the bar.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, Outcome, Text};
use craftkit_sound_time::{Grid, NoteValue};

use crate::locks;
use crate::{ClipPayload, session_of, tempo_map};

/// How many grid lines a read gives back.
///
/// The lines are a **bounded** answer: a song on a grid of sixty-fourths
/// has a great many lines in it, and a reader that asked for all of
/// them would be asking for a number that fills the memory.
const LINES_READ: usize = 8;

commands! {
    /// **Set the grid** of the session.
    ///
    /// The grid is a note value: `bar`, `1/2`, `1/4`, `1/8`, `1/16`, `1/32`
    /// or `1/64`. A step may be **dotted** (one and a half of the note) or
    /// a **triplet** (two thirds of it), and the flags say which.
    ///
    /// A value that is not a note value is **refused**: a grid of `1/5` is
    /// not a grid, and a grid that is quietly a quarter note is a grid the
    /// caller did not ask for.
    sound.grid.set {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Grid", "Set Grid"),
        params: {
            /// The note value the grid divides the bar into.

            value: Text,
            /// Whether the step is dotted. Without it, it is not.

            dotted: Option<Bool>,
            /// Whether the step is a triplet. Without it, it is not.

            triplet: Option<Bool>,
        },
        run: {

            let id = "sound.grid.set".to_string();
            let value = NoteValue::parse(&self.value.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no note value called `{}`: a grid is bar, 1/2, 1/4, 1/8, 1/16, 1/32 or 1/64",
                    self.value.0.trim()
                ),
            })?;
            let mut settings = session_of(session.document());
            settings.grid = Grid {
                value,
                dotted: self.dotted.map(|dotted| dotted.0).unwrap_or(false),
                triplet: self.triplet.map(|triplet| triplet.0).unwrap_or(false),
            };
            crate::set_session(session.document_mut()?, &settings);
            Ok(Outcome::changed(format!("the grid is {}", settings.grid.label())).with_data(serde_json::json!({
                "value": settings.grid.value.name(),
                "dotted": settings.grid.dotted,
                "triplet": settings.grid.triplet,
                "label": settings.grid.label(),
            })))

        }
    }
    /// **Read the grid** of the session.
    ///
    /// The grid is read and the **step** of it is worked out: the step in
    /// ticks is a number, and the step in samples depends on the tempo
    /// where it is, so it is worked out here and not kept. The first lines
    /// of the grid are given back as well.
    sound.grid.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
            /// The place to read the lines from, in samples.
            ///
            /// Without it, the lines from the beginning of the document.

            at: Option<craftkit_sdk::Number>,
        },
        run: {

            let settings = session_of(session.document());
            let map = tempo_map(session.document());
            let rate = settings.sample_rate;
            let grid = settings.grid;
            let at = match self.at {
                Some(at) => {
                    if !at.0.is_finite() {
                        return Err(EngineError::Failed {
                            id: "sound.grid.read".to_string(),
                            message: "a place is made of numbers".to_string(),
                        });
                    }
                    at.0.round() as i64
                }
                None => 0,
            };
            // The lines: the ones at or after the place, and no more than
            // the bound.
            let mut lines = Vec::with_capacity(LINES_READ);
            let mut line = grid.line_at_or_before(at, &map, rate);
            for _ in 0..LINES_READ {
                lines.push(line);
                line = grid.next_line(line, &map, rate);
            }
            Ok(Outcome::new(format!("the grid is {}", grid.label())).with_data(serde_json::json!({
                "value": grid.value.name(),
                "dotted": grid.dotted,
                "triplet": grid.triplet,
                "label": grid.label(),
                "step_ticks": grid.step_ticks(&map, 0),
                "step_samples": grid.step_samples(0, &map, rate),
                "lines": lines,
            })))

        }
    }
    /// **Put clips on the grid**.
    ///
    /// The clip's **beat** is put on the nearest grid line, and the clip
    /// moves so that its beat is there: a clip that is a quarter of a beat
    /// late is late because of its beat and not because of its head, and
    /// the beat is what the grid lines up.
    ///
    /// A clip with no beat is put on the grid by its **head**: there is
    /// nothing inside it that says where it belongs.
    ///
    /// A clip whose **place** is locked does not move, and the command
    /// refuses it: the locks are asked by the commands that would break
    /// them (§ the clip's locks).
    sound.clip.quantize {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Quantize to Grid"),
        params: {
            /// The clips to put on the grid: their ids.

            clips: serde_json::Value,
        },
        run: {

            let id = "sound.clip.quantize".to_string();
            let clips = crate::clip_ids_of(&self.clips, &id)?;
            let settings = session_of(session.document());
            let grid = settings.grid;
            let rate = settings.sample_rate;
            let map = tempo_map(session.document());
            let mut moved = 0;
            for clip in &clips {
                let node = session.document().node(*clip).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                })?;
                if node.kind != "sound.clip" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                    });
                }
                let payload = ClipPayload::from_json(&node.data)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
                // A clip whose **place** is locked stays where it is.
                if locks::time_is_locked(&payload) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{clip}` is locked and does not move"),
                    });
                }
                // The **anchor**: the beat inside the clip, and the head of
                // the clip when there is no beat inside it.
                let beat = payload.sync.filter(|beat| *beat >= 0 && *beat <= payload.length).unwrap_or(0);
                let anchor = payload.start + beat;
                let line = grid.snap(anchor, &map, rate);
                let start = line - beat;
                if start < 0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the grid line at {line} would put the clip `{clip}` before the beginning"
                        ),
                    });
                }
                if start == payload.start {
                    continue;
                }
                let mut payload = payload;
                payload.start = start;
                if let Some(node) = session.document_mut()?.node_mut(*clip) {
                    node.data = payload.to_json();
                }
                moved += 1;
            }
            if moved == 0 {
                return Ok(Outcome::new("the clips are on the grid already"));
            }
            Ok(Outcome::changed(format!("{moved} clips are on the grid"))
                .with_data(serde_json::json!({ "clips": moved, "grid": grid.label() })))

        }
    }
}
