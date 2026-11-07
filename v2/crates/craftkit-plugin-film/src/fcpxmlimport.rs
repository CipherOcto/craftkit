//! The **FCPXML** read back: a document made into a sequence.
//!
//! FCPXML is an XML text that says what is on the timeline, what it
//! came from and how long it is. This module reads one and makes the
//! **sequence** it names.
//!
//! # The rules
//!
//! - The file is read through a **bound**: at most **sixteen
//!   mebibytes**.
//! - A file that is **not FCPXML** is refused: it is not XML, or its
//!   root is not `fcpxml`, or it holds no **sequence** — the tree's
//!   own refusals.
//! - The **rate** of the sequence is the rate of its format: the
//!   `frameDuration` of the format is the length of a frame in
//!   seconds, and the frames of a second are the rate of the format.
//!   A sequence with no format has the default rate of the battery.
//! - The **spine** is the first video track of the sequence, and the
//!   other tracks are **connected clips**: they sit over the spine, on
//!   a **lane** — the video lanes count up from one and the audio
//!   lanes count down from minus one — the tree's own rule.
//! - The **media** of a clip is an item of a bin, with the name and
//!   the file of the asset, and the **clip** points at it — the same
//!   shape as a clip made by hand.
//! - A **gap** is a hole: the clips after it move along, because the
//!   offsets of the text say where every clip stands.
//! - A **transition**, a **marker** and a **nested media** are left
//!   out, and said.
//!
//! # What it does not do
//!
//! - The **time map** of a retimed clip is not read, and the **roles**
//!   of the assets are not read: an asset is a media, and that is all
//!   it is.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Relation, Session, Text};

use craftkit_sdk::xml::Element;

/// The most an FCPXML file may be, in bytes.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// The **frames** a rational time of the format holds.
///
/// A time is a number of seconds: `250/25s` is ten seconds, and at
/// twenty-five frames a second that is two hundred and fifty frames.
fn frames_of(text: &str, rate: (i64, i64)) -> u64 {
    let text = text.trim();
    let text = text.strip_suffix('s').unwrap_or(text).trim();
    let (num, den) = match text.split_once('/') {
        Some((num, den)) => (num.trim(), den.trim()),
        None => (text, "1"),
    };
    let Ok(num) = num.parse::<f64>() else { return 0 };
    let Ok(den) = den.parse::<f64>() else { return 0 };
    if den == 0.0 || !num.is_finite() || !den.is_finite() {
        return 0;
    }
    let frames = num * rate.0 as f64 / (den * rate.1 as f64);
    if frames.is_finite() && frames > 0.0 { frames.round() as u64 } else { 0 }
}

/// The **rate** a format says, from the length of its frame.
fn rate_of(format: Option<&str>, formats: &[(String, String)]) -> (i64, i64) {
    let Some(id) = format else { return DEFAULT_RATE };
    let Some((_, duration)) = formats.iter().find(|(other, _)| other == id) else {
        return DEFAULT_RATE;
    };
    let text = duration.trim().strip_suffix('s').unwrap_or(duration.trim()).trim();
    let (den, num) = match text.split_once('/') {
        Some((den, num)) => (den.trim(), num.trim()),
        None => ("1", text),
    };
    let Ok(num) = num.parse::<i64>() else { return DEFAULT_RATE };
    let Ok(den) = den.parse::<i64>() else { return DEFAULT_RATE };
    if num <= 0 || den <= 0 {
        return DEFAULT_RATE;
    }
    (num, den)
}

/// An **asset** of the resources: the media it plays.
struct Asset {
    name: String,
    path: String,
    duration: String,
}

/// The **assets** of the resources, by their id.
fn assets_of(resources: Option<&Element>) -> Vec<(String, Asset)> {
    let mut assets = Vec::new();
    let Some(resources) = resources else { return assets };
    for asset in resources.children_named("asset") {
        let Some(id) = asset.attribute("id") else { continue };
        let path = asset
            .child("media-rep")
            .and_then(|rep| rep.attribute("src"))
            .map(|src| src.strip_prefix("file://").unwrap_or(src).to_string())
            .unwrap_or_default();
        assets.push((
            id.to_string(),
            Asset {
                name: asset.attribute("name").unwrap_or("the media").to_string(),
                path,
                duration: asset.attribute("duration").unwrap_or("0s").to_string(),
            },
        ));
    }
    assets
}

/// The **media** of the resources: the nested clips, by their id.
fn media_of(resources: Option<&Element>) -> Vec<String> {
    let mut media = Vec::new();
    let Some(resources) = resources else { return media };
    for nested in resources.children_named("media") {
        if let Some(id) = nested.attribute("id") {
            media.push(id.to_string());
        }
    }
    media
}

/// The **formats** of the resources, by their id.
fn formats_of(resources: Option<&Element>) -> Vec<(String, String)> {
    let mut formats = Vec::new();
    let Some(resources) = resources else { return formats };
    for format in resources.children_named("format") {
        let Some(id) = format.attribute("id") else { continue };
        let Some(duration) = format.attribute("frameDuration") else { continue };
        formats.push((id.to_string(), duration.to_string()));
    }
    formats
}

/// The **sequence** of the document: the one of the first project.
///
/// A **media** holds a sequence of its own — it is a nested clip —
/// and it is not the sequence of the document.
fn sequence_of(root: &Element) -> Option<&Element> {
    if let Some(project) = root.find("project")
        && let Some(sequence) = project.child("sequence")
    {
        return Some(sequence);
    }
    find_sequence(root, false)
}

/// The first **sequence** that is not inside a media.
fn find_sequence(element: &Element, inside_media: bool) -> Option<&Element> {
    let inside_media = inside_media || element.name == "media";
    if element.name == "sequence" && !inside_media {
        return Some(element);
    }
    for child in &element.children {
        if let Some(found) = find_sequence(child, inside_media) {
            return Some(found);
        }
    }
    None
}

/// The **reader**: it makes the nodes of a sequence.
struct Reader<'a> {
    session: &'a mut Session,
    bin: u64,
    number: u64,
    rate: (i64, i64),
    media: Vec<String>,
    clips: usize,
    said: Vec<String>,
}

impl Reader<'_> {
    /// A **node** under a parent.
    fn node(
        &mut self,
        parent: Option<u64>,
        name: String,
        kind: &str,
        data: Json,
    ) -> Result<u64, EngineError> {
        let document = self.session.document_mut()?;
        Ok(document.add_node_with_under(parent, name, kind, None, data))
    }

    /// The **item** of the bin for a piece of media.
    fn item(&mut self, name: &str, path: &str, duration: u64) -> Result<u64, EngineError> {
        self.number += 1;
        let number = self.number;
        let data = json!({
            "item": number,
            "path": if path.is_empty() { Json::Null } else { Json::String(path.to_string()) },
            "duration": duration,
            "offline": path.is_empty(),
        });
        self.node(Some(self.bin), name.to_string(), "film.item", data)
    }

    /// The **clip** of a track, with the media it plays.
    fn clip(
        &mut self,
        track: u64,
        name: &str,
        start: u64,
        duration: u64,
        source_in: u64,
        asset: Option<&Asset>,
    ) -> Result<u64, EngineError> {
        let item = match asset {
            Some(asset) => {
                let duration = frames_of(&asset.duration, self.rate);
                self.item(&asset.name, &asset.path, duration)?
            }
            None => self.item("the media", "", 0)?,
        };
        let clip = self.node(
            Some(track),
            if name.is_empty() { "a clip".to_string() } else { name.to_string() },
            "film.clip",
            json!({
                "start": start,
                "duration": duration,
                "source_in": source_in,
                "speed": 1.0,
                "link": Json::Null,
            }),
        )?;
        let relation = Relation::with_payload(clip, item, "clip", json!({}));
        let document = self.session.document_mut()?;
        document.add_relation(relation);
        self.clips += 1;
        Ok(clip)
    }
}

/// **Read** an FCPXML file and make the **sequence** it names.
///
/// The media of the file are items, and the clips of its spine are
/// clips on the first track of the sequence.
#[derive(Command)]
#[command(id = "film.import.fcpxml", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct ImportFcpxml {
    /// The file to read.
    pub path: Text,
    /// What to call the sequence.
    ///
    /// Without it, the name of the file.
    pub name: Option<Text>,
}

impl ImportFcpxml {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.import.fcpxml".to_string();
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
        let root = craftkit_sdk::xml::parse(&text).map_err(|message| EngineError::Failed {
            id: id.clone(),
            message: format!("the file is not FCPXML: {message}"),
        })?;
        if root.name != "fcpxml" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the file is not FCPXML: its root is `{}` and not `fcpxml`", root.name),
            });
        }
        let Some(sequence) = sequence_of(&root) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file is not FCPXML: there is no sequence in it".to_string(),
            });
        };
        let resources = root.child("resources");
        let formats = formats_of(resources);
        let assets = assets_of(resources);
        let rate = rate_of(sequence.attribute("format"), &formats);
        // The **name** of the sequence.
        let name = match &self.name {
            Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
            _ => {
                let from_file = std::path::Path::new(&path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_default();
                if from_file.is_empty() {
                    sequence.attribute("name").unwrap_or("the sequence").to_string()
                } else {
                    from_file
                }
            }
        };
        // The **bin** the media goes in, and the **sequence**.
        let bin = session.document_mut()?.add_node_under(None, format!("{name} media"), "film.bin", None);
        // The **bin** the sequence goes in: a sequence is not a media.
        let sequences =
            session.document_mut()?.add_node_under(None, format!("{name} sequences"), "film.bin", None);
        let sequence_id = session.document_mut()?.add_node_with_under(
            Some(sequences),
            name.clone(),
            "film.item",
            None,
            json!({
                "item": 0,
                "path": Json::Null,
                "duration": 0,
                "offline": false,
                "rate": { "num": rate.0, "den": rate.1 },
            }),
        );
        let media = media_of(resources);
        let mut reader = Reader { session, bin, number: 0, rate, media, clips: 0, said: Vec::new() };
        // The **spine** is the first video track.
        let spine = reader.node(
            Some(sequence_id),
            "the spine".to_string(),
            "film.track",
            json!({ "kind": "video" }),
        )?;
        let mut end = 0u64;
        let mut tracks = 1usize;
        let mut lanes: Vec<(i64, u64)> = Vec::new();
        if let Some(spine_element) = sequence.child("spine") {
            for element in spine_element.children_named("clip") {
                let start = frames_of(element.attribute("offset").unwrap_or("0s"), rate);
                let duration = frames_of(element.attribute("duration").unwrap_or("0s"), rate);
                let source_in = frames_of(element.attribute("start").unwrap_or("0s"), rate);
                let name = element.attribute("name").unwrap_or("a clip");
                let reference = element.attribute("ref").unwrap_or_default();
                if reader.media.iter().any(|id| id == reference) {
                    reader.said.push(format!("the nested media of the clip `{name}` is left out"));
                    continue;
                }
                let asset = assets.iter().find(|(other, _)| other == reference).map(|(_, asset)| asset);
                reader.clip(spine, name, start, duration, source_in, asset)?;
                end = end.max(start + duration);
                // The **connected clips** over this one: the other
                // tracks of the sequence.
                for connected in element.children_named("clip") {
                    let Some(lane) =
                        connected.attribute("lane").and_then(|lane| lane.trim().parse::<i64>().ok())
                    else {
                        continue;
                    };
                    if lanes.iter().any(|(other, _)| *other == lane) {
                        continue;
                    }
                    let kind = if lane < 0 { "audio" } else { "video" };
                    let track = reader.node(
                        Some(sequence_id),
                        format!("the lane {lane}"),
                        "film.track",
                        json!({ "kind": kind }),
                    )?;
                    lanes.push((lane, track));
                    tracks += 1;
                }
                // The **connected clips** themselves.
                for connected in element.children_named("clip") {
                    let Some(lane) =
                        connected.attribute("lane").and_then(|lane| lane.trim().parse::<i64>().ok())
                    else {
                        continue;
                    };
                    let Some((_, track)) = lanes.iter().find(|(other, _)| *other == lane) else { continue };
                    let offset = frames_of(connected.attribute("offset").unwrap_or("0s"), rate);
                    let duration = frames_of(connected.attribute("duration").unwrap_or("0s"), rate);
                    let source_in = frames_of(connected.attribute("start").unwrap_or("0s"), rate);
                    let name = connected.attribute("name").unwrap_or("a clip");
                    let asset = connected
                        .attribute("ref")
                        .and_then(|id| assets.iter().find(|(other, _)| other == id))
                        .map(|(_, asset)| asset);
                    reader.clip(*track, name, start + offset, duration, source_in, asset)?;
                    end = end.max(start + offset + duration);
                }
                // The **transitions**, the **markers** and the
                // **nested media** are left out, and said.
                for transition in element.children_named("transition") {
                    reader.said.push(format!(
                        "the transition `{}` is left out",
                        transition.attribute("name").unwrap_or("a transition")
                    ));
                }
                for marker in element.children_named("marker") {
                    reader.said.push(format!(
                        "the marker `{}` is left out",
                        marker.attribute("name").unwrap_or("a marker")
                    ));
                }
            }
        }
        // The **length** of the sequence is what its tracks hold.
        if let Some(node) =
            reader.session.document_mut().ok().and_then(|document| document.node_mut(sequence_id))
        {
            node.data["duration"] = Json::from(end);
        }
        let said = reader.said.clone();
        let clips = reader.clips;
        Ok(Outcome::changed(format!("the document of `{path}` made {tracks} track(s) and {clips} clip(s)"))
            .with_data(json!({
                "path": path,
                "bin": bin,
                "sequence_bin": sequences,
                "sequence": sequence_id,
                "tracks": tracks,
                "clips": clips,
                "rate": { "num": rate.0, "den": rate.1 },
                "left_out": said,
            })))
    }
}
