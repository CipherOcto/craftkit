//! The **FCPXML** of a sequence.
//!
//! FCPXML is the interchange format of a non-linear editor: an XML
//! text that says what is on the timeline, what it came from and how
//! long it is. It is **derived** — worked out from the sequence as it
//! is now — and it is returned as a text, because a command about the
//! machine is not a command about the document (§457).
//!
//! The shape of the text is the tree's shape:
//!
//! ```text
//! <fcpxml version="1.9">
//!   <resources>
//!     <format id="r1" name="FFVideoFormat1080p25" frameDuration="1/25s" width="1920" height="1080"/>
//!     <asset id="r2" name="the clip" start="0s" duration="250/25s" format="r1">
//!       <media-rep kind="original-media" src="file:///the/clip.mov"/>
//!     </asset>
//!   </resources>
//!   <library>
//!     <event name="the film">
//!       <project name="the sequence">
//!         <sequence format="r1" duration="500/25s" tcStart="0s" tcFormat="NDF">
//!           <spine>
//!             <clip ref="r2" name="the clip" offset="0s" start="0s" duration="250/25s"/>
//!           </spine>
//!         </sequence>
//!       </project>
//!     </event>
//!   </library>
//! </fcpxml>
//! ```
//!
//! - the **spine** is the first video track of the sequence, and the
//!   other tracks are **connected clips**: they sit over the spine and
//!   are written inside the element they sit over, on a **lane** — the
//!   video tracks count up from one and the audio tracks count down
//!   from minus one;
//! - the **holes** between the clips of the spine are **gaps**, and
//!   the gaps keep the timing;
//! - the **times** are rational and counted in frames: a duration of
//!   five frames at twenty-five frames a second is `5/25s`;
//! - a **clip that is retimed** — sped up or reversed — carries a
//!   `timeMap`, which says where the source goes;
//! - the **transitions** of the spine are `transition` elements, and
//!   the **markers** of the sequence are written inside the element
//!   they sit on.
//!
//! The rules:
//!
//! - a clip with **no item** is refused: the list would name a source
//!   that is not there;
//! - a **transition** on a track that is not the spine is **left out**,
//!   and the answer says what was left out — the tree drops those too,
//!   and here the dropping is said rather than done quietly;
//! - the **size** of the sequence is not a fact of the document here,
//!   so the caller says it.
//!
//! # What the port found
//!
//! - **The tree's export writes a file** and the tree's asset knows
//!   whether it has video and audio, because the tree probes the
//!   media. Here the item knows its name, its path and its length,
//!   and nothing else: the asset says what it knows and does not say
//!   what it does not.
//! - **The tree's export writes the keyframes, the transforms, the
//!   blend modes and the pan of a clip.** Here a clip is its place on
//!   the timeline, its source and its length — the rest of what a clip
//!   may carry is not carried yet.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// The version of the format this tree writes: the tree's own.
const VERSION: &str = "1.9";

/// The most characters the text may have.
const MAX_XML: usize = 4_000_000;

/// The rate a sequence with **no rate** runs at: the tree's own.
const DEFAULT_RATE: (i64, i64) = (25, 1);

/// The most a **size** may be, in pixels.
const MAX_SIZE: u64 = 100_000;

/// **Escape** a text for an XML attribute.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// A **time**, in frames, as the rational time of the format.
fn time(frames: u64, rate: (i64, i64)) -> String {
    if frames == 0 {
        return "0s".to_string();
    }
    let (num, den) = rate;
    let value = i128::from(frames) * i128::from(den);
    format!("{value}/{num}s")
}

/// The **label** of a rate: the frames of a second, as the tree
/// writes them — `25` and `2997`.
fn rate_label(rate: (i64, i64)) -> String {
    let value = rate.0 as f64 / rate.1 as f64;
    let mut text = format!("{value:.2}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text.replace('.', "")
}

/// The **rate** of a sequence: the frames of a second it counts.
fn rate_of(sequence: &Node) -> (i64, i64) {
    let stored = sequence.data.get("rate");
    let number = |name: &str| stored.and_then(|rate| rate.get(name)).and_then(|value| value.as_i64());
    match (number("num"), number("den")) {
        (Some(num), Some(den)) if num > 0 && den > 0 => (num, den),
        _ => DEFAULT_RATE,
    }
}

/// The **item** a clip plays.
fn item_of(document: &craftkit_sdk::Document, clip: u64) -> Option<Node> {
    let item = document
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)?;
    document.node(item).cloned()
}

/// The **tracks** of a sequence: the video ones first, then the audio
/// ones, and each in the order it is in the sequence.
fn tracks_of(sequence: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut tracks: Vec<std::sync::Arc<Node>> =
        sequence.children.iter().filter(|child| child.kind == "film.track").cloned().collect();
    let is_audio =
        |track: &Node| track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default() == "audio";
    tracks.sort_by_key(|track| is_audio(track.as_ref()));
    tracks
}

/// Whether a track is an **audio** track.
fn is_audio(track: &Node) -> bool {
    track.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default() == "audio"
}

/// The **clips** of a track, in the order they are on the timeline.
fn clips_of(track: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut clips: Vec<std::sync::Arc<Node>> =
        track.children.iter().filter(|child| child.kind == "film.clip").cloned().collect();
    clips.sort_by_key(|clip| clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
    clips
}

/// A **transition** of a track: a dissolve over a cut.
struct Transition {
    /// The incoming clip.
    to: u64,
    /// Where it sits on the timeline.
    start: u64,
    /// How long it is.
    duration: u64,
    /// The name of its effect.
    name: String,
}

/// The **transitions** of a track.
fn transitions_of(track: &Node) -> Vec<Transition> {
    let mut transitions = Vec::new();
    for child in track.children.iter().filter(|child| child.kind == "film.transition") {
        let number = |name: &str| child.data.get(name).and_then(|value| value.as_u64());
        let (Some(_from), Some(to)) = (number("from"), number("to")) else { continue };
        transitions.push(Transition {
            to,
            start: number("start").unwrap_or(0),
            duration: number("duration").unwrap_or(0),
            name: child
                .data
                .get("effect")
                .and_then(|effect| effect.as_str())
                .unwrap_or("Cross Dissolve")
                .to_string(),
        });
    }
    transitions
}

/// The **markers** of a sequence, in the order they are on the
/// timeline.
fn markers_of(sequence: &Node) -> Vec<std::sync::Arc<Node>> {
    let mut markers: Vec<std::sync::Arc<Node>> =
        sequence.children.iter().filter(|child| child.kind == "film.marker").cloned().collect();
    markers.sort_by_key(|marker| marker.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
    markers
}

/// One element of the **spine**: a clip, a gap, or a hole.
struct Element {
    /// Where it sits on the timeline.
    start: u64,
    /// Where it ends.
    end: u64,
    /// The element itself, with its children.
    open: String,
    /// The children: the connected clips and the markers.
    children: Vec<String>,
    /// The clip this element is, when it is one.
    clip: Option<u64>,
    /// The name of the element, for the closing tag.
    close: String,
}

/// The **writer** of the text: it hands out the ids of the resources.
struct Writer {
    /// The next id.
    next: u32,
    /// The resources, in the order they were made.
    resources: Vec<String>,
    /// The id of the asset of each item.
    assets: Vec<(u64, String)>,
    /// The id of the effect of each name.
    effects: Vec<(String, String)>,
    /// The rate of the sequence.
    rate: (i64, i64),
    /// The width of the sequence, in pixels.
    width: u64,
    /// The height of the sequence, in pixels.
    height: u64,
    /// What was left out, and why.
    left_out: Vec<String>,
}

impl Writer {
    /// A new id.
    fn id(&mut self) -> String {
        let id = format!("r{}", self.next);
        self.next += 1;
        id
    }

    /// The **format** of the sequence.
    fn format(&mut self) -> String {
        let id = self.id();
        let (num, den) = self.rate;
        self.resources.push(format!(
            "    <format id=\"{id}\" name=\"FFVideoFormat{height}p{label}\" frameDuration=\"{den}/{num}s\" width=\"{width}\" height=\"{height}\"/>\n",
            height = self.height,
            width = self.width,
            label = rate_label(self.rate),
        ));
        id
    }

    /// The **asset** of an item: the media it plays.
    fn asset(&mut self, item: &Node) -> String {
        let id = item.id;
        if let Some((_, resource)) = self.assets.iter().find(|(other, _)| *other == id) {
            return resource.clone();
        }
        let resource = self.id();
        let name = escape(&item.name);
        let duration = item.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
        let path = item.data.get("path").and_then(|path| path.as_str()).unwrap_or_default();
        let mut asset = format!(
            "    <asset id=\"{resource}\" name=\"{name}\" start=\"0s\" duration=\"{}\"",
            time(duration, self.rate)
        );
        if path.is_empty() {
            asset.push_str("/>\n");
        } else {
            let source = if path.starts_with('/') { format!("file://{path}") } else { path.to_string() };
            asset.push_str(&format!(
                ">\n      <media-rep kind=\"original-media\" src=\"{}\"/>\n    </asset>\n",
                escape(&source)
            ));
        }
        self.resources.push(asset);
        self.assets.push((id, resource.clone()));
        resource
    }

    /// The **effect** of a transition.
    fn effect(&mut self, name: &str) -> String {
        if let Some((_, resource)) = self.effects.iter().find(|(other, _)| other == name) {
            return resource.clone();
        }
        let resource = self.id();
        self.resources.push(format!(
            "    <effect id=\"{resource}\" name=\"{}\" uid=\"craftkit.{}\"/>\n",
            escape(name),
            escape(&resource)
        ));
        self.effects.push((name.to_string(), resource.clone()));
        resource
    }
}

/// The **text** of the FCPXML of a sequence.
///
/// The text is derived from the sequence as it is now, and it is
/// returned as a text.
#[derive(Command)]
#[command(id = "film.export.fcpxml", area = "film", layer = "film", icon = "list")]
pub struct ExportFcpxml {
    /// The sequence.
    pub sequence: NodeRef,
    /// The **width** of the sequence, in pixels. The document does not
    /// say it, so the caller does.
    pub width: Number,
    /// The **height** of the sequence, in pixels.
    pub height: Number,
}

impl ExportFcpxml {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.export.fcpxml".to_string();
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{sequence}`"),
            });
        };
        if node.kind != "film.item" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sequence}` is a `{}`, and a sequence is an item", node.kind),
            });
        }
        let size = |value: f64, what: &str| -> Result<u64, EngineError> {
            if !value.is_finite() || value <= 0.0 || value > MAX_SIZE as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{value} is not {what}: it is a number from one to {MAX_SIZE}"),
                });
            }
            Ok(value.round() as u64)
        };
        let width = size(self.width.0, "a width")?;
        let height = size(self.height.0, "a height")?;
        let rate = rate_of(&node);
        let mut writer = Writer {
            next: 1,
            resources: Vec::new(),
            assets: Vec::new(),
            effects: Vec::new(),
            rate,
            width,
            height,
            left_out: Vec::new(),
        };
        let format = writer.format();
        let document = session.document();
        let tracks = tracks_of(&node);
        // The **spine** is the first video track; the other tracks are
        // connected clips.
        let spine = tracks.iter().find(|track| !is_audio(track.as_ref()));
        let mut elements: Vec<Element> = Vec::new();
        let mut at = 0u64;
        if let Some(spine) = spine {
            for clip in clips_of(spine) {
                let start = clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = clip.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                if start > at {
                    elements.push(Element {
                        start: at,
                        end: start,
                        open: format!(
                            "<gap name=\"Gap\" offset=\"{}\" start=\"{}\" duration=\"{}\"",
                            time(at, rate),
                            time(at, rate),
                            time(start - at, rate)
                        ),
                        children: Vec::new(),
                        clip: None,
                        close: "gap".to_string(),
                    });
                }
                let Some(item) = item_of(document, clip.id) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{}` has no item, and it has no source", clip.name),
                    });
                };
                let resource = writer.asset(&item);
                let source_in = clip.data.get("source_in").and_then(|source| source.as_u64()).unwrap_or(0);
                let speed = clip.data.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(1.0);
                let reverse = clip.data.get("reverse").and_then(|reverse| reverse.as_bool()).unwrap_or(false);
                let enabled = clip.data.get("enabled").and_then(|enabled| enabled.as_bool()).unwrap_or(true);
                let mut open = format!(
                    "<clip ref=\"{resource}\" name=\"{}\" offset=\"{}\" start=\"{}\" duration=\"{}\"",
                    escape(&clip.name),
                    time(start, rate),
                    time(source_in, rate),
                    time(duration, rate)
                );
                if !enabled {
                    open.push_str(" enabled=\"0\"");
                }
                let mut children: Vec<String> = Vec::new();
                // A clip that is **retimed** says where its source
                // goes: the time map.
                if speed != 1.0 || reverse {
                    let source = (duration as f64 * speed).round() as u64;
                    let (first, last) = if reverse {
                        (source_in + source, source_in)
                    } else {
                        (source_in, source_in + source)
                    };
                    children.push(format!(
                        "<timeMap>\n  <timept time=\"{}\" value=\"{}\" interp=\"linear\"/>\n  <timept time=\"{}\" value=\"{}\" interp=\"linear\"/>\n</timeMap>",
                        time(source_in, rate),
                        time(first, rate),
                        time(source_in + duration, rate),
                        time(last, rate)
                    ));
                }
                elements.push(Element {
                    start,
                    end: start + duration,
                    open,
                    children,
                    clip: Some(clip.id),
                    close: "clip".to_string(),
                });
                at = start + duration;
            }
        }
        // The **connected clips**: the other tracks, over the spine.
        for (index, track) in tracks.iter().enumerate() {
            let video = !is_audio(track.as_ref());
            let first_video = spine.map(|spine| spine.id == track.id).unwrap_or(false);
            if first_video {
                continue;
            }
            let lane = if video {
                index as i64
            } else {
                -((index - tracks.iter().filter(|other| is_audio(other.as_ref())).count() + 1) as i64)
            };
            for clip in clips_of(track) {
                let start = clip.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = clip.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                let Some(item) = item_of(document, clip.id) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{}` has no item, and it has no source", clip.name),
                    });
                };
                let resource = writer.asset(&item);
                let source_in = clip.data.get("source_in").and_then(|source| source.as_u64()).unwrap_or(0);
                let element =
                    match elements.iter_mut().find(|element| element.start <= start && start < element.end) {
                        Some(element) => element,
                        None => {
                            writer.left_out.push(format!(
                                "the clip `{}` of the track `{}` sits over nothing, and it is left out",
                                clip.name, track.name
                            ));
                            continue;
                        }
                    };
                element.children.push(format!(
                    "<clip ref=\"{resource}\" lane=\"{lane}\" name=\"{}\" offset=\"{}\" start=\"{}\" duration=\"{}\"/>",
                    escape(&clip.name),
                    time(start - element.start, rate),
                    time(source_in, rate),
                    time(duration, rate)
                ));
                at = at.max(start + duration);
            }
        }
        // The **transitions** of the spine.
        let mut transitions: Vec<(usize, String)> = Vec::new();
        if let Some(spine) = spine {
            for (index, transition) in transitions_of(spine).iter().enumerate() {
                let resource = writer.effect(&transition.name);
                let _ = index;
                // The transition comes **after** the clip it comes
                // into: it is the cut that follows that clip.
                let after = elements
                    .iter()
                    .position(|element| element.clip == Some(transition.to))
                    .map(|position| position + 1)
                    .unwrap_or(elements.len());
                transitions.push((
                    after,
                    format!(
                        "<transition name=\"{}\" offset=\"{}\" duration=\"{}\"><filter-video ref=\"{resource}\" name=\"{}\"/></transition>",
                        escape(&transition.name),
                        time(transition.start, rate),
                        time(transition.duration, rate),
                        escape(&transition.name)
                    ),
                ));
            }
        }
        // The **transitions** of the other tracks are left out, and the
        // answer says so.
        for track in tracks.iter() {
            if spine.map(|spine| spine.id == track.id).unwrap_or(false) {
                continue;
            }
            for transition in transitions_of(track) {
                writer.left_out.push(format!(
                    "the transition `{}` of the track `{}` is on a track that is not the spine, and it is left out",
                    transition.name, track.name
                ));
            }
        }
        // The **markers** of the sequence.
        for marker in markers_of(&node) {
            let start = marker.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
            let duration = marker.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
            let name = escape(&marker.name);
            let element =
                match elements.iter_mut().find(|element| element.start <= start && start < element.end) {
                    Some(element) => element,
                    None => {
                        writer.left_out.push(format!(
                            "the marker `{}` is not on the timeline, and it is left out",
                            marker.name
                        ));
                        continue;
                    }
                };
            element.children.push(format!(
                "<marker start=\"{}\" duration=\"{}\" name=\"{name}\"/>",
                time(start - element.start, rate),
                time(duration, rate)
            ));
        }
        // The **text**.
        let mut out = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE fcpxml>\n<fcpxml version=\"{VERSION}\">\n  <resources>\n"
        );
        for resource in &writer.resources {
            out.push_str(resource);
        }
        out.push_str("  </resources>\n");
        out.push_str(&format!("  <library>\n    <event name=\"{}\">\n", escape(&node.name)));
        out.push_str(&format!("      <project name=\"{}\">\n", escape(&node.name)));
        out.push_str(&format!(
            "        <sequence format=\"{format}\" duration=\"{}\" tcStart=\"0s\" tcFormat=\"NDF\">\n          <spine>\n",
            time(at, rate)
        ));
        let mut written = 0usize;
        for (index, element) in elements.iter().enumerate() {
            while written < transitions.len() && transitions[written].0 == index {
                out.push_str(&format!("            {}\n", transitions[written].1));
                written += 1;
            }
            if element.children.is_empty() {
                out.push_str(&format!("            {}/>\n", element.open));
            } else {
                out.push_str(&format!("            {}>\n", element.open));
                for child in &element.children {
                    for line in child.lines() {
                        out.push_str(&format!("              {line}\n"));
                    }
                }
                out.push_str(&format!("            </{}>\n", element.close));
            }
        }
        while written < transitions.len() {
            out.push_str(&format!("            {}\n", transitions[written].1));
            written += 1;
        }
        out.push_str("          </spine>\n        </sequence>\n      </project>\n    </event>\n  </library>\n</fcpxml>\n");
        if out.chars().count() > MAX_XML {
            return Err(EngineError::Failed {
                id,
                message: format!("the text is more than {MAX_XML} characters long, and it is not written"),
            });
        }
        Ok(Outcome::new(format!("the sequence is {} characters of FCPXML", out.chars().count())).with_data(
            json!({
                "sequence": sequence,
                "xml": out,
                "format": format,
                "left_out": writer.left_out,
            }),
        ))
    }
}
