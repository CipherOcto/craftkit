//! `craftkit-plugin-art` — the document model of an **AI image and video
//! IDE**, as v2 nodes, relations and tasks.
//!
//! # What is ported
//!
//! An artist's IDE is three programs in one, and they do not share a
//! model:
//!
//! - a **video editor**: a project of scenes, each scene of tracks, each
//!   track of elements, and every element a **clip** with a place on the
//!   timeline and a place in the media;
//! - a **3D stage**: the engine's own scene, which the application saves
//!   as **one opaque JSON string** and does not model;
//! - a **generation queue**: work that is sent to a provider, given an id
//!   the provider knows it by, and finished **hours later**, by an event
//!   that arrives from outside the process.
//!
//! All three are ported, and none of them needed a change in the core.
//!
//! # The mapping
//!
//! | the application | v2 |
//! |---|---|
//! | a project | a **node** of the kind `art.project` |
//! | a scene | a **node** of the kind `art.scene`, in the project |
//! | a track | a **node** of the kind `art.track.video` (or `.text`, `.audio`, `.graphic`, `.effect`) |
//! | an element | a **node** of the kind `art.element.video` (or `.image`, `.audio`, `.text`, `.sticker`, `.graphic`, `.effect`) |
//! | the media an element plays | the element's **payload**: a token, not a file |
//! | the params, the effects, the masks, the keyframes | the element's **payload** |
//! | the 3D scene | a **stream**: one opaque JSON file beside the document |
//! | a generation | a **task**, with the provider's own id as its handle |
//!
//! # The three things the port found
//!
//! - **A closed union becomes kinds, and the core learns nothing.** The
//!   application has five kinds of track and seven kinds of element, in a
//!   TypeScript union that a new kind would have to edit. Here each one
//!   is a **kind**, which v2's kinds are open strings: a new kind of
//!   element is another plugin's kind, and nothing in this crate or in
//!   the core changes. The price was **twelve commands** instead of one.
//! - **A document may carry a thing it does not model.** The 3D scene is
//!   the engine's own JSON, and modelling it would be a rewrite of the
//!   engine's half. The kit does not have to model it: a node may have a
//!   **stream**, the bytes travel beside the document in the bundle, and
//!   the editor hands them back to the engine untouched.
//! - **A job that lives outside the process is a task, and the outside
//!   world's name for it is a handle.** The provider's job id comes back
//!   when the job is done, so the task has to be findable **by that id** —
//!   which is one small method in the SDK, and the first thing a port has
//!   cost the core.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

/// The art layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "art", name = "Art", requires("core"))]
pub struct ArtLayer;

/// The slowest a rate may be.
pub const MIN_RATE: f64 = 0.01;

/// The fastest a rate may be.
pub const MAX_RATE: f64 = 5.0;

/// How many ticks are in a second.
///
/// The application's own time base is a count of **ticks**, and how many
/// of them make a second is a fact about the application, not about the
/// kit. So it is a number the project carries, and this is the default
/// when it does not say.
pub const DEFAULT_TICKS_PER_SECOND: u64 = 1_000_000;

/// The kind of a track, and the kind of an element.
///
/// The application has these in closed unions. Here each one is a
/// **kind**, and the kinds are open: the list below is what this plugin
/// knows, and it is not a limit on what the document may hold.
macro_rules! art_kind {
    ($struct:ident, $kind:literal, $label:literal) => {
        /// A kind of the application.
        #[derive(NodeKind)]
        #[node_kind(kind = $kind, label = $label, layer = "art", evaluates)]
        pub struct $struct;

        impl $struct {
            /// Nothing here is drawn.
            fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

            /// Nothing here is hit.
            fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
                false
            }

            /// What it **means**: its own place on the timeline, how long
            /// it is there, and the media it plays.
            fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
                Some(serde_json::json!({
                    "type": node.data.get("type").cloned().unwrap_or(serde_json::Value::Null),
                    "start": node.data.get("start").cloned().unwrap_or(serde_json::Value::Null),
                    "duration": timeline_duration(node),
                    "source_duration": node.data.get("source_duration").cloned().unwrap_or(serde_json::Value::Null),
                    "trim_start": node.data.get("trim_start").cloned().unwrap_or(serde_json::Value::Null),
                    "trim_end": node.data.get("trim_end").cloned().unwrap_or(serde_json::Value::Null),
                    "rate": node.data.get("rate").cloned().unwrap_or(serde_json::Value::Null),
                    "media": node.data.get("media").cloned().unwrap_or(serde_json::Value::Null),
                }))
            }
        }
    };
}

/// A **track**: a lane of the timeline.
macro_rules! track_kind {
    ($struct:ident, $kind:literal, $label:literal, $type:literal) => {
        /// A track of the timeline.
        #[derive(NodeKind)]
        #[node_kind(kind = $kind, label = $label, layer = "art", evaluates)]
        pub struct $struct;

        impl $struct {
            /// A track is not drawn.
            fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

            /// A track is not hit.
            fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
                false
            }

            /// What a track **means**: what kind it is, and how many
            /// elements are on it.
            fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
                Some(serde_json::json!({
                    "type": $type,
                    "elements": node.children.iter().filter(|child| child.kind.starts_with("art.element.")).count(),
                }))
            }
        }
    };
}

art_kind!(ElementVideoKind, "art.element.video", "Video");
art_kind!(ElementImageKind, "art.element.image", "Image");
art_kind!(ElementAudioKind, "art.element.audio", "Audio");
art_kind!(ElementTextKind, "art.element.text", "Text");
art_kind!(ElementStickerKind, "art.element.sticker", "Sticker");
art_kind!(ElementGraphicKind, "art.element.graphic", "Graphic");
art_kind!(ElementEffectKind, "art.element.effect", "Effect");

track_kind!(TrackVideoKind, "art.track.video", "Video", "video");
track_kind!(TrackTextKind, "art.track.text", "Text", "text");
track_kind!(TrackAudioKind, "art.track.audio", "Audio", "audio");
track_kind!(TrackGraphicKind, "art.track.graphic", "Graphic", "graphic");
track_kind!(TrackEffectKind, "art.track.effect", "Effect", "effect");

/// A **project**: the whole of the artist's work.
#[derive(NodeKind)]
#[node_kind(kind = "art.project", label = "Project", layer = "art", evaluates)]
pub struct ProjectKind;

impl ProjectKind {
    /// A project is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// A project is not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a project **means**: its scenes, its frame rate and its time
    /// base. The **duration** is not here: it is the length of the main
    /// scene, and it is derived (§33).
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "scenes": node.children.iter().filter(|child| child.kind == "art.scene").count(),
            "fps": node.data.get("fps").cloned().unwrap_or(serde_json::Value::Null),
            "ticks_per_second": node.data.get("ticks_per_second").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// A **scene**: one segment of the work.
#[derive(NodeKind)]
#[node_kind(kind = "art.scene", label = "Scene", layer = "art", evaluates)]
pub struct SceneKind;

impl SceneKind {
    /// A scene is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// A scene is not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a scene **means**: its tracks, and whether it is the **main**
    /// scene — the one the project's length comes from.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "tracks": node.children.iter().filter(|child| child.kind.starts_with("art.track.")).count(),
            "main": node.data.get("main").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// A **stage**: the 3D scene, as one opaque file.
///
/// The engine owns it and the application does not model it, so the kit
/// does not either: the node says **which file** the scene is in, and the
/// bytes travel beside the document.
#[derive(NodeKind)]
#[node_kind(kind = "art.stage", label = "Stage", layer = "art", evaluates)]
pub struct StageKind;

impl StageKind {
    /// A stage is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// A stage is not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a stage **means**: the file its scene is in.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "scene": match node.stream.clone() {
                Some(path) => serde_json::Value::String(path),
                None => serde_json::Value::Null,
            },
        }))
    }
}

/// Put a **project** in the document.
#[derive(Command)]
#[command(id = "art.project.add", area = "art", layer = "art", icon = "plus", mutates, undoable)]
pub struct AddProject {
    /// What to call it.
    pub name: Text,
    /// The frame rate, in frames a second.
    pub fps: Option<Number>,
    /// How many ticks are in a second.
    pub ticks_per_second: Option<Number>,
}

impl AddProject {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "art.project.add".to_string(),
                message: "a project has a name".to_string(),
            });
        }
        let fps = match &self.fps {
            Some(fps) => whole(*fps, "art.project.add", "a frame rate")?,
            None => 30,
        };
        let ticks = match &self.ticks_per_second {
            Some(ticks) => whole(*ticks, "art.project.add", "a tick rate")?,
            None => DEFAULT_TICKS_PER_SECOND,
        };
        let data = serde_json::json!({ "fps": fps, "ticks_per_second": ticks });
        let id = session.document_mut()?.add_node_with_under(None, name.clone(), "art.project", None, data);
        Ok(Outcome::changed(format!("added the project `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "fps": fps })))
    }
}

/// Put a **scene** in a project.
#[derive(Command)]
#[command(id = "art.scene.add", area = "art", layer = "art", icon = "plus", mutates, undoable)]
pub struct AddScene {
    /// The project it goes in.
    pub project: NodeRef,
    /// What to call it.
    pub name: Text,
    /// Whether it is the **main** scene: the one the project's length
    /// comes from.
    pub main: Option<craftkit_sdk::Bool>,
}

impl AddScene {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "art.scene.add".to_string(),
                message: "a scene has a name".to_string(),
            });
        }
        let project = self.project.0;
        if session.document().node(project).is_none() {
            return Err(EngineError::Failed {
                id: "art.scene.add".to_string(),
                message: format!("there is no project `{project}`"),
            });
        }
        let main = self.main.map(|main| main.0).unwrap_or(false);
        let data = serde_json::json!({ "main": main });
        let id =
            session.document_mut()?.add_node_with_under(Some(project), name.clone(), "art.scene", None, data);
        Ok(Outcome::changed(format!("added the scene `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "main": main })))
    }
}

/// Put a **track** of one kind in a scene.
///
/// The kinds are open, so each one has its own command: a new kind of
/// track is a new command, and nothing here changes.
macro_rules! track_command {
    ($struct:ident, $id:literal, $label:literal, $kind:literal) => {
        /// Put a track in a scene.
        #[derive(Command)]
        #[command(id = $id, area = "art", layer = "art", icon = "plus", mutates, undoable)]
        pub struct $struct {
            /// The scene it goes in.
            pub scene: NodeRef,
            /// What to call it.
            pub name: Text,
        }

        impl $struct {
            fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
                let name = self.name.0.clone();
                if name.trim().is_empty() {
                    return Err(EngineError::Failed {
                        id: $id.to_string(),
                        message: "a track has a name".to_string(),
                    });
                }
                let scene = self.scene.0;
                if session.document().node(scene).is_none() {
                    return Err(EngineError::Failed {
                        id: $id.to_string(),
                        message: format!("there is no scene `{scene}`"),
                    });
                }
                let id = session
                    .document_mut()?
                    .add_node_with_under(Some(scene), name.clone(), $kind, None, serde_json::json!({}));
                Ok(Outcome::changed(format!("added the track `{name}`"))
                    .with_data(serde_json::json!({ "id": id, "name": name, "kind": $kind })))
            }
        }
    };
}

track_command!(AddTrackVideo, "art.track.video.add", "Video", "art.track.video");
track_command!(AddTrackText, "art.track.text.add", "Text", "art.track.text");
track_command!(AddTrackAudio, "art.track.audio.add", "Audio", "art.track.audio");
track_command!(AddTrackGraphic, "art.track.graphic.add", "Graphic", "art.track.graphic");
track_command!(AddTrackEffect, "art.track.effect.add", "Effect", "art.track.effect");

/// Put an **element** on a track.
///
/// The kinds are open, so each one has its own command: a new kind of
/// element is a new command, and nothing here changes.
macro_rules! element_command {
    (
        $struct:ident, $id:literal, $label:literal, $kind:literal, $type:literal,
        $extra:ident, $extra_key:literal, $extra_doc:literal
    ) => {
        /// Put an element on a track.
        #[derive(Command)]
        #[command(id = $id, area = "art", layer = "art", icon = "plus", mutates, undoable)]
        pub struct $struct {
            /// The track it goes on.
            pub track: NodeRef,
            /// What to call it.
            pub name: Text,
            /// Where it starts on the timeline, in ticks.
            pub start: Number,
            /// How long the **media** is, in ticks.
            pub duration: Number,
            /// Where in the media it begins, in ticks.
            pub trim_start: Option<Number>,
            /// Where in the media it ends, in ticks.
            pub trim_end: Option<Number>,
            /// The extra thing this kind of element has.
            pub $extra: Option<Text>,
        }

        impl $struct {
            fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
                let name = self.name.0.clone();
                if name.trim().is_empty() {
                    return Err(EngineError::Failed {
                        id: $id.to_string(),
                        message: "an element has a name".to_string(),
                    });
                }
                let track = self.track.0;
                if session.document().node(track).is_none() {
                    return Err(EngineError::Failed {
                        id: $id.to_string(),
                        message: format!("there is no track `{track}`"),
                    });
                }
                let start = whole(self.start, $id, "a start")?;
                let duration = whole(self.duration, $id, "a duration")?;
                if duration == 0 {
                    return Err(EngineError::Failed {
                        id: $id.to_string(),
                        message: "an element is not zero ticks long".to_string(),
                    });
                }
                let rate = rate(1.0, $id)?;
                let trim_start = match &self.trim_start {
                    Some(trim) => whole(*trim, $id, "a trim")?,
                    None => 0,
                };
                // No trim means the whole media is played.
                let trim_end = match &self.trim_end {
                    Some(trim) => whole(*trim, $id, "a trim")?,
                    None => duration,
                };
                let extra = self.$extra.as_ref().map(|extra| extra.0.clone());
                let data = serde_json::json!({
                    "type": $type,
                    "start": start,
                    "source_duration": duration,
                    "trim_start": trim_start,
                    "trim_end": trim_end,
                    "rate": rate,
                    $extra_key: match &extra {
                        Some(extra) => serde_json::Value::String(extra.clone()),
                        None => serde_json::Value::Null,
                    },
                });
                let id = session
                    .document_mut()?
                    .add_node_with_under(Some(track), name.clone(), $kind, None, data);
                Ok(Outcome::changed(format!("added the element `{name}`"))
                    .with_data(serde_json::json!({ "id": id, "name": name, "start": start, "duration": duration })))
            }
        }
    };
}

element_command!(
    AddElementVideo,
    "art.element.video.add",
    "Video",
    "art.element.video",
    "video",
    media,
    "media",
    "The media it plays: a token, not a file."
);
element_command!(
    AddElementImage,
    "art.element.image.add",
    "Image",
    "art.element.image",
    "image",
    media,
    "media",
    "The media it plays: a token, not a file."
);
element_command!(
    AddElementAudio,
    "art.element.audio.add",
    "Audio",
    "art.element.audio",
    "audio",
    media,
    "media",
    "The media it plays: a token, not a file."
);
element_command!(
    AddElementText,
    "art.element.text.add",
    "Text",
    "art.element.text",
    "text",
    content,
    "content",
    "What it says."
);
element_command!(
    AddElementSticker,
    "art.element.sticker.add",
    "Sticker",
    "art.element.sticker",
    "sticker",
    sticker,
    "sticker",
    "Which sticker it is."
);
element_command!(
    AddElementGraphic,
    "art.element.graphic.add",
    "Graphic",
    "art.element.graphic",
    "graphic",
    definition,
    "definition",
    "Which graphic it is."
);
element_command!(
    AddElementEffect,
    "art.element.effect.add",
    "Effect",
    "art.element.effect",
    "effect",
    effect_type,
    "effect_type",
    "Which effect it is."
);

/// Put a **stage** in the document: the 3D scene, as one file.
///
/// The scene is the engine's own JSON, and the application does not model
/// it. So the node does not either: it says **which file** the scene is
/// in, and the bytes travel beside the document in the bundle.
#[derive(Command)]
#[command(id = "art.stage.add", area = "art", layer = "art", icon = "plus", mutates, undoable)]
pub struct AddStage {
    /// What to call it.
    pub name: Text,
    /// Where the scene's file is, relative to the document.
    pub path: Text,
}

impl AddStage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "art.stage.add".to_string(),
                message: "a stage has a name".to_string(),
            });
        }
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "art.stage.add".to_string(),
                message: "a stage says which file its scene is in".to_string(),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            None,
            name.clone(),
            "art.stage",
            None,
            serde_json::json!({}),
        );
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "art.stage.add".to_string(),
                message: format!("the stage `{name}` is not in the document"),
            });
        };
        node.stream = Some(path.clone());
        Ok(Outcome::changed(format!("added the stage `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "path": path })))
    }
}

/// The timeline of the document.
///
/// A query: the elements in the order they start, and the length of every
/// track, every scene and the project. All of it is **derived**: none of
/// it is written down.
#[derive(Command)]
#[command(id = "art.timeline", area = "art", layer = "art", icon = "list")]
pub struct ReadTimeline {}

impl ReadTimeline {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let document = session.document();
        let mut scenes: Vec<serde_json::Value> = Vec::new();
        for scene in document.walk().filter(|node| node.kind == "art.scene") {
            let mut tracks: Vec<serde_json::Value> = Vec::new();
            let mut scene_end = 0u64;
            for track in &scene.children {
                if !track.kind.starts_with("art.track.") {
                    continue;
                }
                let mut elements: Vec<(u64, u64, String)> = track
                    .children
                    .iter()
                    .filter(|child| child.kind.starts_with("art.element."))
                    .map(|child| {
                        let start = child.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                        let duration = timeline_duration(child);
                        (start, duration, child.kind.clone())
                    })
                    .collect();
                // The order of a timeline is not the order of the tree.
                elements.sort_by_key(|(start, _, _)| *start);
                let end = elements.iter().map(|(start, duration, _)| start + duration).max().unwrap_or(0);
                scene_end = scene_end.max(end);
                tracks.push(serde_json::json!({
                    "track": track.id,
                    "kind": track.kind,
                    "elements": elements
                        .iter()
                        .map(|(start, duration, kind)| serde_json::json!({
                            "kind": kind,
                            "start": start,
                            "duration": duration,
                        }))
                        .collect::<Vec<_>>(),
                    "end": end,
                }));
            }
            scenes.push(serde_json::json!({
                "scene": scene.id,
                "name": scene.name,
                "main": scene.data.get("main").cloned().unwrap_or(serde_json::Value::Null),
                "tracks": tracks,
                "end": scene_end,
            }));
        }
        // The project is as long as its **main** scene, and as long as
        // the longest of the others if it has no main scene.
        let main_end = scenes
            .iter()
            .filter(|scene| scene.get("main").and_then(|main| main.as_bool()) == Some(true))
            .filter_map(|scene| scene.get("end").and_then(|end| end.as_u64()))
            .max();
        let longest = scenes.iter().filter_map(|scene| scene.get("end").and_then(|end| end.as_u64())).max();
        let project_end = main_end.or(longest).unwrap_or(0);
        let count = scenes.len();
        Ok(Outcome::new(format!("{count} scene(s)"))
            .with_data(serde_json::json!({ "scenes": scenes, "project_end": project_end })))
    }
}

/// The evaluator that measures the **timeline**.
///
/// A track's length is the end of its last element, a scene's length is
/// the end of the longest of its tracks, and the project's length is the
/// length of its main scene. None of them is stored: they change when an
/// element moves, and a thing that changes when something else changes
/// is derived.
#[derive(Evaluator)]
#[evaluator(id = "art.timeline", label = "Timeline", layer = "art")]
pub struct TimelineEvaluator;

impl TimelineEvaluator {
    /// The length of every scene, and of the project.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        let mut project_end = 0u64;
        let mut main_end = None;
        for node in document.walk() {
            if node.kind != "art.scene" {
                continue;
            }
            let end = node
                .children
                .iter()
                .filter(|child| child.kind.starts_with("art.track."))
                .filter_map(|track| {
                    track
                        .children
                        .iter()
                        .filter(|child| child.kind.starts_with("art.element."))
                        .filter_map(|child| {
                            let start = child.data.get("start").and_then(|start| start.as_u64())?;
                            Some(start + timeline_duration(child))
                        })
                        .max()
                })
                .max()
                .unwrap_or(0);
            if node.data.get("main").and_then(|main| main.as_bool()) == Some(true) {
                main_end = Some(end);
            }
            project_end = project_end.max(end);
            plan.items.push(PlanItem {
                node: node.id,
                kind: "art.timeline".to_string(),
                value: serde_json::json!({ "end": end }),
            });
        }
        let project = document.walk().find(|node| node.kind == "art.project");
        if let Some(project) = project {
            plan.items.push(PlanItem {
                node: project.id,
                kind: "art.timeline".to_string(),
                value: serde_json::json!({ "end": main_end.unwrap_or(project_end) }),
            });
        }
        plan
    }
}

/// Start a **generation**: work that leaves the process and comes back.
///
/// The provider gives the work an id of its own, and that id is the
/// **handle**: the name the other end will call the work by when it is
/// done. The handle may come later, so it is optional here.
#[derive(Command)]
#[command(id = "art.generate", area = "art", layer = "art", icon = "play", mutates, undoable)]
pub struct Generate {
    /// The model asked for.
    pub model: Text,
    /// What to ask it for.
    pub prompt: Option<Text>,
    /// The provider's own id for the work, if it has one already.
    pub handle: Option<Text>,
}

impl Generate {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let model = self.model.0.clone();
        if model.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "art.generate".to_string(),
                message: "a generation names a model".to_string(),
            });
        }
        let id = session.tasks_mut().register("artcraft.generate");
        session
            .tasks_mut()
            .start(id)
            .map_err(|message| EngineError::Failed { id: "art.generate".to_string(), message })?;
        if let Some(handle) = &self.handle {
            let handle = handle.0.trim().to_string();
            if !handle.is_empty() {
                session
                    .tasks_mut()
                    .attach(id, handle.clone())
                    .map_err(|message| EngineError::Failed { id: "art.generate".to_string(), message })?;
            }
        }
        let handle = session.tasks().task(id).and_then(|task| task.handle.clone());
        Ok(Outcome::changed(format!("started the generation `{model}` as task {id}")).with_data(
            serde_json::json!({
                "task": id,
                "model": model,
                "prompt": self.prompt.as_ref().map(|prompt| prompt.0.clone()),
                "handle": handle,
            }),
        ))
    }
}

/// Say how far a generation has got.
#[derive(Command)]
#[command(id = "art.job.progress", area = "art", layer = "art", icon = "list", mutates, undoable)]
pub struct JobProgress {
    /// The task.
    pub task: Number,
    /// How far it has got, as a percent.
    pub percent: Number,
}

impl JobProgress {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = whole(self.task, "art.job.progress", "a task number")?;
        let percent = self.percent.0;
        if !percent.is_finite() {
            return Err(EngineError::Failed {
                id: "art.job.progress".to_string(),
                message: "a progress is a number".to_string(),
            });
        }
        let percent = percent.clamp(0.0, 100.0).round() as u8;
        session
            .tasks_mut()
            .progress(id, percent)
            .map_err(|message| EngineError::Failed { id: "art.job.progress".to_string(), message })?;
        Ok(Outcome::changed(format!("the task {id} is {percent}% done")))
    }
}

/// Finish a generation, **by the provider's own id**.
///
/// This is the door the outside world comes in by: the event that says
/// the work is done names the work by the id the provider knows it by,
/// and the task is found by that id.
#[derive(Command)]
#[command(id = "art.job.finish", area = "art", layer = "art", icon = "check", mutates, undoable)]
pub struct JobFinish {
    /// The provider's own id for the work.
    pub handle: Text,
    /// The media it produced: a token, not a file.
    pub media: Option<Text>,
}

impl JobFinish {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let handle = self.handle.0.trim().to_string();
        if handle.is_empty() {
            return Err(EngineError::Failed {
                id: "art.job.finish".to_string(),
                message: "a generation is finished by the provider's own id".to_string(),
            });
        }
        let media = self.media.as_ref().map(|media| media.0.clone());
        let result = serde_json::json!({
            "media": match &media {
                Some(media) => serde_json::Value::String(media.clone()),
                None => serde_json::Value::Null,
            },
        });
        session
            .tasks_mut()
            .complete_by_handle(&handle, result)
            .map_err(|message| EngineError::Failed { id: "art.job.finish".to_string(), message })?;
        Ok(Outcome::changed(format!("the generation `{handle}` is finished"))
            .with_data(serde_json::json!({ "handle": handle, "media": media })))
    }
}

/// A generation **failed**, by the provider's own id.
#[derive(Command)]
#[command(id = "art.job.fail", area = "art", layer = "art", icon = "cross", mutates, undoable)]
pub struct JobFail {
    /// The provider's own id for the work.
    pub handle: Text,
    /// Why it did not finish.
    pub message: Text,
}

impl JobFail {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let handle = self.handle.0.trim().to_string();
        if handle.is_empty() {
            return Err(EngineError::Failed {
                id: "art.job.fail".to_string(),
                message: "a generation is failed by the provider's own id".to_string(),
            });
        }
        let message = self.message.0.clone();
        session
            .tasks_mut()
            .fail_by_handle(&handle, message.clone())
            .map_err(|message| EngineError::Failed { id: "art.job.fail".to_string(), message })?;
        Ok(Outcome::changed(format!("the generation `{handle}` failed")))
    }
}

/// **Trim** an element: choose the part of the media it plays.
///
/// A trim is about a **range**, and not about a node: the element keeps
/// its place on the timeline and plays less of its media. The length it
/// takes on the timeline follows from the range, and nobody writes it
/// down.
#[derive(Command)]
#[command(id = "art.element.trim", area = "art", layer = "art", icon = "trim", mutates, undoable)]
pub struct TrimElement {
    /// The element to trim.
    pub element: NodeRef,
    /// Where in the media it begins, in ticks.
    pub from: Number,
    /// Where in the media it ends, in ticks.
    pub to: Number,
}

impl TrimElement {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.element.0;
        let from = whole(self.from, "art.element.trim", "a trim")?;
        let to = whole(self.to, "art.element.trim", "a trim")?;
        if to <= from {
            return Err(EngineError::Failed {
                id: "art.element.trim".to_string(),
                message: "a trim ends after it begins".to_string(),
            });
        }
        let source = {
            let Some(node) = session.document().node(id) else {
                return Err(EngineError::Failed {
                    id: "art.element.trim".to_string(),
                    message: format!("there is no element `{id}`"),
                });
            };
            node.data.get("source_duration").and_then(|source| source.as_u64()).unwrap_or(0)
        };
        if to > source {
            return Err(EngineError::Failed {
                id: "art.element.trim".to_string(),
                message: format!("the media is {source} ticks long and the trim ends at {to}"),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "art.element.trim".to_string(),
                message: format!("there is no element `{id}`"),
            });
        };
        node.data["trim_start"] = serde_json::json!(from);
        node.data["trim_end"] = serde_json::json!(to);
        let duration = timeline_duration(node);
        Ok(Outcome::changed(format!("trimmed the element {id} to {from}..{to}")).with_data(
            serde_json::json!({ "element": id, "trim_start": from, "trim_end": to, "duration": duration }),
        ))
    }
}

/// **Retime** an element: change how fast its media plays.
///
/// The rate is the application's own: it is clamped, not refused, and
/// the bounds are the ones the application uses. The length on the
/// timeline follows from the rate, and nobody writes it down.
#[derive(Command)]
#[command(id = "art.element.retime", area = "art", layer = "art", icon = "clock", mutates, undoable)]
pub struct RetimeElement {
    /// The element to retime.
    pub element: NodeRef,
    /// How fast the media plays: one is as fast as it was.
    pub rate: Number,
}

impl RetimeElement {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.element.0;
        let rate = rate(self.rate.0, "art.element.retime")?;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "art.element.retime".to_string(),
                message: format!("there is no element `{id}`"),
            });
        };
        node.data["rate"] = serde_json::json!(rate);
        let duration = timeline_duration(node);
        Ok(Outcome::changed(format!("the element {id} plays at {rate}"))
            .with_data(serde_json::json!({ "element": id, "rate": rate, "duration": duration })))
    }
}

/// **Ripple** a track: take the space out of one element and give it to
/// the ones after it.
///
/// The ripple is **per track**: it does not cross into another track,
/// and that is the application's rule. The elements after the cut move
/// earlier by the amount, and the ones before it do not move at all.
#[derive(Command)]
#[command(id = "art.ripple", area = "art", layer = "art", icon = "cut", mutates, undoable)]
pub struct RippleTrack {
    /// The track to ripple.
    pub track: NodeRef,
    /// Where the cut is, in ticks.
    pub after: Number,
    /// How much space to take out, in ticks.
    pub amount: Number,
}

impl RippleTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let after = whole(self.after, "art.ripple", "a cut")?;
        let amount = whole(self.amount, "art.ripple", "an amount")?;
        if amount == 0 {
            return Err(EngineError::Failed {
                id: "art.ripple".to_string(),
                message: "a ripple takes some space out or it takes none".to_string(),
            });
        }
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: "art.ripple".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        // The elements that move, and where they move to.
        let mut moved: Vec<(u64, u64)> = Vec::new();
        for child in &node.children {
            if !child.kind.starts_with("art.element.") {
                continue;
            }
            let start = child.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
            if start < after {
                continue;
            }
            if start < amount {
                return Err(EngineError::Failed {
                    id: "art.ripple".to_string(),
                    message: format!("the ripple would push the element at {start} before the start"),
                });
            }
            moved.push((child.id, start - amount));
        }
        let count = moved.len();
        for (element, start) in moved {
            let Some(node) = session.document_mut()?.node_mut(element) else { continue };
            node.data["start"] = serde_json::json!(start);
        }
        Ok(Outcome::changed(format!("rippled {count} element(s) on the track {track}")).with_data(
            serde_json::json!({ "track": track, "after": after, "amount": amount, "moved": count }),
        ))
    }
}

/// The panel that shows the project.
#[derive(Panel)]
#[panel(
    id = "panel.art",
    label = "Art",
    layer = "art",
    commands(
        "art.project.add",
        "art.scene.add",
        "art.track.video.add",
        "art.element.video.add",
        "art.timeline"
    )
)]
pub struct ArtPanel;

/// The **length of an element on the timeline**.
///
/// It is **derived**: the span of the media the element plays, divided
/// by the rate it plays at. Nobody writes it down, so a trim or a
/// retime cannot leave a stale number behind.
fn timeline_duration(node: &Node) -> u64 {
    let source = node.data.get("source_duration").and_then(|source| source.as_u64()).unwrap_or(0);
    let trim_start = node.data.get("trim_start").and_then(|trim| trim.as_u64()).unwrap_or(0);
    let trim_end = node.data.get("trim_end").and_then(|trim| trim.as_u64()).unwrap_or(source);
    let span = trim_end.saturating_sub(trim_start).min(source);
    let rate = node.data.get("rate").and_then(|rate| rate.as_f64()).unwrap_or(1.0);
    if rate <= 0.0 {
        return 0;
    }
    ((span as f64) / rate).round() as u64
}

/// A **rate**: how fast the media plays, and within the domain's bounds.
///
/// The bounds are the application's: a rate of a hundredth is slow
/// enough and a rate of five is fast enough, and a rate outside them is
/// clamped rather than refused — the same thing the application does.
fn rate(value: f64, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a rate is a number above zero".to_string(),
        });
    }
    Ok(value.clamp(MIN_RATE, MAX_RATE))
}

/// A whole number, and not a fraction and not a NaN.
///
/// The numbers that come in are from outside — a command line, a script,
/// an agent — so they are checked before they are used.
fn whole(value: Number, id: &str, what: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("{what} is a whole number") });
    }
    Ok(value.0 as u64)
}

// The plugin's anchor: one name that keeps every item above in the
// program, on every target.
craftkit_sdk::plugin_anchor!(
    ArtLayer,
    ElementVideoKind,
    ElementImageKind,
    ElementAudioKind,
    ElementTextKind,
    ElementStickerKind,
    ElementGraphicKind,
    ElementEffectKind,
    TrackVideoKind,
    TrackTextKind,
    TrackAudioKind,
    TrackGraphicKind,
    TrackEffectKind,
    ProjectKind,
    SceneKind,
    StageKind,
    AddProject,
    AddScene,
    AddTrackVideo,
    AddTrackText,
    AddTrackAudio,
    AddTrackGraphic,
    AddTrackEffect,
    AddElementVideo,
    AddElementImage,
    AddElementAudio,
    AddElementText,
    AddElementSticker,
    AddElementGraphic,
    AddElementEffect,
    AddStage,
    TrimElement,
    RetimeElement,
    RippleTrack,
    ReadTimeline,
    TimelineEvaluator,
    Generate,
    JobProgress,
    JobFinish,
    JobFail,
    ArtPanel
);
