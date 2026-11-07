//! The **ALE** read back: an Avid log exchange made into a sequence.
//!
//! An ALE is a **clip log**: a tab-delimited text with three
//! sections — the **heading**, the **columns** and the **data** — and
//! each row of the data is a piece of media with the place in the
//! media it was used from and to.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - A file that is **not an ALE** is refused: the text has no
//!   `Heading` section, or no `Column` section — the tree's own
//!   refusals.
//! - The line ends are **CR LF**, **CR** or **LF**: the tree's own
//!   rule.
//! - The **rate** of the sequence is the `FPS` of the heading, and a
//!   rate that is not one of the tree's own common rates is worked
//!   out as a fraction of a thousand — the tree's own rule. A log
//!   with no `FPS` has the default rate of the battery.
//! - A row whose **start** or **end** cannot be read is **left out**:
//!   the tree leaves such a row out too.
//! - The clips are laid **end to end**: a log says what media there
//!   is, and not where it goes.
//! - The **media** of a row is an item of its own, and the **clip**
//!   points at it — the same shape as a clip that was made by hand.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session, Text};

/// The most an ALE file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The parsed **log**: the heading, the columns and the rows.
struct Ale {
    heading: Vec<(String, String)>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Ale {
    /// The value of the **heading**, by name.
    fn heading_of(&self, name: &str) -> Option<&str> {
        self.heading.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
    }

    /// The value of a **column** in a row.
    fn get(&self, row: usize, column: &str) -> Option<&str> {
        let at = self.columns.iter().position(|held| held.eq_ignore_ascii_case(column))?;
        self.rows.get(row)?.get(at).map(String::as_str)
    }
}

/// **Read** a log: the three sections of it, the tree's own rules.
fn parse(text: &str) -> Result<Ale, String> {
    #[derive(PartialEq)]
    enum Section {
        None,
        Heading,
        Column,
        Data,
    }
    let mut heading: Vec<(String, String)> = Vec::new();
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut section = Section::None;
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    for line in text.lines() {
        match line.trim() {
            "Heading" => {
                section = Section::Heading;
                continue;
            }
            "Column" => {
                section = Section::Column;
                continue;
            }
            "Data" => {
                section = Section::Data;
                continue;
            }
            "" => continue,
            _ => {}
        }
        match section {
            Section::None => return Err("not an ALE file (no Heading section)".to_string()),
            Section::Heading => {
                let mut parts = line.splitn(2, '\t');
                let key = parts.next().unwrap_or_default().trim().to_string();
                let value = parts.next().unwrap_or_default().trim().to_string();
                heading.push((key, value));
            }
            Section::Column => columns.extend(line.split('\t').map(|column| column.trim().to_string())),
            Section::Data => rows.push(line.split('\t').map(str::to_string).collect()),
        }
    }
    if columns.is_empty() {
        return Err("the ALE file has no Column section".to_string());
    }
    Ok(Ale { heading, columns, rows })
}

/// The **rate** of a heading's `FPS`, as the tree works it out: one
/// of the common rates when it is near one, and a fraction of a
/// thousand otherwise.
fn rate_of(fps: &str) -> (i64, i64) {
    let common: [(i64, i64); 11] = [
        (24000, 1001),
        (24, 1),
        (25, 1),
        (30000, 1001),
        (30, 1),
        (48, 1),
        (50, 1),
        (60000, 1001),
        (60, 1),
        (120000, 1001),
        (120, 1),
    ];
    let Ok(fps) = fps.trim().parse::<f64>() else { return (25, 1) };
    for (num, den) in common {
        let rate = num as f64 / den as f64;
        if (rate - fps).abs() < 0.005 {
            return (num, den);
        }
    }
    ((fps * 1000.0).round() as i64, 1000)
}

/// The **frames** a timecode says, at a rate.
fn frames_of(timecode: &str, per_second: u64) -> Option<u64> {
    let parts: Vec<&str> = timecode.trim().split(':').collect();
    if parts.len() != 4 {
        return None;
    }
    let hours: u64 = parts[0].trim().parse().ok()?;
    let minutes: u64 = parts[1].trim().parse().ok()?;
    let seconds: u64 = parts[2].trim().parse().ok()?;
    let frames: u64 = parts[3].trim().parse().ok()?;
    if minutes > 59 || seconds > 59 || frames >= per_second.max(1) {
        return None;
    }
    Some(((hours * 60 + minutes) * 60 + seconds) * per_second.max(1) + frames)
}

/// **Read** an Avid log exchange and make the **sequence** it names.
///
/// The media of the log are **items**, and the rows are **clips** on
/// a video track of the sequence.
#[derive(Command)]
#[command(id = "film.import.ale", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportAle {
    /// The file to read.
    pub path: Text,
    /// What to call the sequence.
    ///
    /// Without it, the name of the file.
    pub name: Option<Text>,
}

impl ImportAle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.import.ale".to_string();
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
        }
        let bytes =
            craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES).map_err(|message| {
                EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
            })?;
        let text = String::from_utf8(bytes).map_err(|_| EngineError::Failed {
            id: id.clone(),
            message: "the file is not a text".to_string(),
        })?;
        let ale = parse(&text).map_err(|message| EngineError::Failed {
            id: id.clone(),
            message: format!("the file is not an ALE file: {message}"),
        })?;
        let (num, den) = match ale.heading_of("FPS") {
            Some(fps) => rate_of(fps),
            None => (25, 1),
        };
        let per_second = ((num as f64 / den as f64).round() as u64).max(1);
        // The **name** of the sequence.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => std::path::Path::new(&path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
                .unwrap_or_else(|| "the sequence".to_string()),
        };
        // The **bin** the media goes in.
        let bin = session.document_mut()?.add_node_under(None, format!("{name} media"), "film.bin", None);
        // The **sequence** itself, and the **track** of the clips.
        let mut rows: Vec<(u64, u64, usize)> = Vec::new();
        for index in 0..ale.rows.len() {
            let (Some(start), Some(end)) = (
                ale.get(index, "Start").and_then(|start| frames_of(start, per_second)),
                ale.get(index, "End").and_then(|end| frames_of(end, per_second)),
            ) else {
                continue;
            };
            if end <= start {
                continue;
            }
            rows.push((start, end - start, index));
        }
        let total: u64 = rows.iter().map(|(_, duration, _)| duration).sum();
        let sequence = session.document_mut()?.add_node_with_under(
            Some(bin),
            name.clone(),
            "film.item",
            None,
            json!({
                "item": 0,
                "path": Json::Null,
                "duration": total,
                "offline": false,
                "rate": { "num": num, "den": den },
            }),
        );
        let track = session.document_mut()?.add_node_with_under(
            Some(sequence),
            "the track".to_string(),
            "film.track",
            None,
            json!({ "kind": "video" }),
        );
        // The **clips**: the rows of the log, end to end.
        let mut clips: Vec<Json> = Vec::new();
        let mut at = 0u64;
        for (source_in, duration, index) in rows {
            let clip_name = ale.get(index, "Name").unwrap_or("a clip").to_string();
            let path = ale.get(index, "Source File").unwrap_or_default().to_string();
            let item = session.document_mut()?.add_node_with_under(
                Some(bin),
                clip_name.clone(),
                "film.item",
                None,
                json!({
                    "item": index as u64 + 1,
                    "path": path,
                    "duration": duration,
                    "offline": false,
                }),
            );
            let clip = session.document_mut()?.add_node_with_under(
                Some(track),
                clip_name.clone(),
                "film.clip",
                None,
                json!({
                    "start": at,
                    "duration": duration,
                    "source_in": source_in,
                    "speed": 1.0,
                    "link": Json::Null,
                }),
            );
            let relation = craftkit_sdk::Relation::with_payload(clip, item, "clip", json!({}));
            session.document_mut()?.add_relation(relation);
            clips.push(json!({ "id": clip, "item": item, "start": at, "duration": duration }));
            at += duration;
        }
        let count = clips.len();
        Ok(Outcome::changed(format!("the log of `{path}` made {count} clip(s)")).with_data(json!({
            "path": path,
            "bin": bin,
            "sequence": sequence,
            "track": track,
            "clips": clips,
        })))
    }
}
