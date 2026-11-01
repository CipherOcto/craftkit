//! The mixer: buses, routes, inserts, sends and automation lanes.
//!
//! # Why this is a separate file
//!
//! Because this is the part of a sound document that is **not a tree**.
//!
//! A track holds its clips, and that is containment: the tree says it. But
//! a track *feeds* a bus, a track *sends* to a bus, and a clip *plays* a
//! source — and none of those is containment. A mixer is a **graph**, and
//! the tree cannot be a mixer.
//!
//! So the mixer is written in **relations**: a typed edge from one node to
//! another, and the edge's kind is a string. The core holds them
//! ([`craftkit_sdk::Relation`]) and knows nothing about them; the sound
//! battery reads the ones it cares about — `route`, `send`, `source` — and
//! ignores the rest.
//!
//! # What a mixer is here
//!
//! Not the mixing: there is no audio thread and no block of audio
//! anywhere in v2, and that is a blocker in the README rather than a
//! silence here. What is here is the **graph** — who feeds whom, what is
//! inserted where, what is automated — and the **derived order** of it,
//! which is what a router needs before it can route anything.

use craftkit_derive::{Command, NodeKind};

use craftkit_sdk::{
    Bool, Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Point, Session, Text,
};

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;

use crate::timeline;

/// The kind of a bus.
///
/// A bus is a **destination**: the mixer sends to it. It is a node, and
/// the tracks that feed it say so with a relation.
#[derive(NodeKind)]
#[node_kind(kind = "sound.bus", label = "Bus", layer = "sound", evaluates, place)]
pub struct BusKind;

impl BusKind {
    /// How a bus is drawn: the **line along the top** of its row.
    ///
    /// The bus is a row of the **mixer**, and the mixer is under the
    /// timeline: the tracks are the rows above and the buses are the
    /// rows below.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::bus_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.w, rect.y),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a bus.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::bus_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a bus **is**: its row in the mixer.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::bus_rect(document, node)
    }

    /// What a bus **means**, in the plan.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let bus = BusPayload::from_json(&node.data).ok()?;
        let fed_by: Vec<Json> = document
            .relations_to(node.id, Some("route"))
            .into_iter()
            .map(|relation| serde_json::json!({ "from": relation.from }))
            .collect();
        Some(serde_json::json!({
            "name": bus.name,
            "channels": bus.channels,
            "fed_by": fed_by,
        }))
    }
}

/// The kind of an insert: a plugin in a slot on a track.
///
/// The plugin is a **string**, not an enum: the tree this ports has thirty
/// built-in plugins in a static table and a hard-coded match on the id, so
/// a plugin that is not in the table cannot be created at all. Here the
/// insert says which plugin it is and the parameters are a map, and what
/// happens to them is the audio engine's business — which v2 does not
/// have yet.
#[derive(NodeKind)]
#[node_kind(kind = "sound.insert", label = "Insert", layer = "sound", evaluates, place)]
pub struct InsertKind;

impl InsertKind {
    /// How an insert is drawn: a **box at the left** of the track's row.
    ///
    /// An insert is not at a time, so it is at the **start** of the
    /// track — the way a mixer shows the slots of a strip.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::insert_rect(document, node) else { return };
        let fill = if node.muted {
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE)
        } else {
            document
                .appearance_of(node)
                .map(|appearance| appearance.fill)
                .unwrap_or(craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::BLACK))
        };
        canvas.rect(rect, fill, craftkit_sdk::document::BlendMode::Normal);
    }

    /// Whether a point is inside an insert.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::insert_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where an insert **is**: a box at the left of the track's row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::insert_rect(document, node)
    }

    /// What an insert **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let insert = InsertPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "plugin": insert.plugin,
            "bypass": insert.bypass,
            "params": insert.params,
        }))
    }
}

/// The kind of a send: a level that goes from a track to a bus.
///
/// The send is a node under the track, and the bus it goes to is a
/// **relation**: the send says *what it is* and the relation says *where
/// it goes*.
#[derive(NodeKind)]
#[node_kind(kind = "sound.send", label = "Send", layer = "sound", evaluates, place)]
pub struct SendKind;

impl SendKind {
    /// How a send is drawn: the **line** from the track down to the bus.
    ///
    /// The mixer is a **graph**, and this is the graph: the send is
    /// the edge, and the edge is drawn where it goes.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::send_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a send.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::send_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a send **is**: the line from the track to the bus.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::send_rect(document, node)
    }

    /// What a send **means**, in the plan.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let send = SendPayload::from_json(&node.data).ok()?;
        let to: Vec<u64> =
            document.relations_from(node.id, Some("send")).into_iter().map(|relation| relation.to).collect();
        Some(serde_json::json!({
            "level_db": send.level_db,
            "pan": send.pan,
            "pre_fader": send.pre_fader,
            "to": to,
        }))
    }
}

/// The kind of an automation lane: breakpoints on a parameter.
///
/// A lane says **which parameter** of **which node** it moves, and the
/// parameter is a **string**. The tree this ports has a closed enum of
/// mixer parameters with a plugin escape, so a lane can only automate what
/// the enum knows; here it can automate anything a node's payload names.
#[derive(NodeKind)]
#[node_kind(kind = "sound.lane", label = "Automation", layer = "sound", evaluates, place)]
pub struct LaneKind;

impl LaneKind {
    /// How a lane is drawn: the **curve** on the track's row.
    ///
    /// The curve is drawn to **fill its row**, so that what is seen is
    /// the shape of the automation and not the absolute values: the kit
    /// does not know what range a parameter takes.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let points = timeline::lane_points(document, node);
        if points.len() < 2 {
            return;
        }
        // The curve is an **open** line: it starts at the first
        // breakpoint and ends at the last, and it does not join back.
        let mut segments = Vec::with_capacity(points.len());
        for (x, y) in points.iter().skip(1) {
            segments.push(craftkit_sdk::Segment::Line { to: Point::new(*x, *y) });
        }
        let (x, y) = points[0];
        let path = craftkit_sdk::Path::single(craftkit_sdk::SubPath {
            start: Point::new(x, y),
            segments,
            closed: false,
        });
        canvas.path(
            &path,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a lane.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::lane_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a lane **is**: the curve's box on the track's row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::lane_rect(document, node)
    }

    /// What a lane **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let lane = LanePayload::from_json(&node.data).ok()?;
        let points: Vec<Json> =
            lane.points.iter().map(|(at, value)| serde_json::json!({ "at": at, "value": value })).collect();
        Some(serde_json::json!({ "param": lane.param, "points": points }))
    }
}

/// What a bus says.
#[derive(Debug, Clone, PartialEq)]
pub struct BusPayload {
    /// Its name.
    pub name: String,
    /// How many channels it has.
    pub channels: u32,
}

impl BusPayload {
    /// The bus as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({ "name": self.name, "channels": self.channels })
    }

    /// The bus as it is read.
    pub fn from_json(value: &Json) -> Result<Self, String> {
        Ok(BusPayload {
            name: value.get("name").and_then(|name| name.as_str()).unwrap_or("bus").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
        })
    }
}

/// What an insert says.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertPayload {
    /// Which plugin it is.
    pub plugin: String,
    /// Its parameters, by name.
    pub params: Json,
    /// Whether it is bypassed.
    pub bypass: bool,
}

impl InsertPayload {
    /// The insert as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({ "plugin": self.plugin, "params": self.params, "bypass": self.bypass })
    }

    /// The insert as it is read.
    pub fn from_json(value: &Json) -> Result<Self, String> {
        Ok(InsertPayload {
            plugin: value.get("plugin").and_then(|plugin| plugin.as_str()).unwrap_or("").to_string(),
            params: value.get("params").cloned().unwrap_or(Json::Object(Default::default())),
            bypass: value.get("bypass").and_then(|bypass| bypass.as_bool()).unwrap_or(false),
        })
    }
}

/// What a send says.
#[derive(Debug, Clone, PartialEq)]
pub struct SendPayload {
    /// Its level, in decibels.
    pub level_db: f64,
    /// Its pan, from -1 to 1.
    pub pan: f64,
    /// Whether it is silent.
    ///
    /// A send is a **copy** of the track's signal, and a muted send is
    /// a copy of nothing: the track is still heard on its own and the
    /// bus hears nothing.
    pub mute: bool,
    /// Whether it is before the fader.
    pub pre_fader: bool,
}

impl SendPayload {
    /// The send as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "level_db": self.level_db,
            "pan": self.pan,
            "mute": self.mute,
            "pre_fader": self.pre_fader,
        })
    }

    /// The send as it is read.
    pub fn from_json(value: &Json) -> Result<Self, String> {
        Ok(SendPayload {
            level_db: value.get("level_db").and_then(|level| level.as_f64()).unwrap_or(0.0),
            pan: value.get("pan").and_then(|pan| pan.as_f64()).unwrap_or(0.0),
            mute: value.get("mute").and_then(|mute| mute.as_bool()).unwrap_or(false),
            pre_fader: value.get("pre_fader").and_then(|pre| pre.as_bool()).unwrap_or(false),
        })
    }
}

/// What an automation lane says.
#[derive(Debug, Clone, PartialEq)]
pub struct LanePayload {
    /// Which parameter it moves.
    pub param: String,
    /// Its breakpoints: where, and what value.
    pub points: Vec<(i64, f64)>,
}

impl LanePayload {
    /// The lane as it is written.
    pub fn to_json(&self) -> Json {
        let points: Vec<Json> =
            self.points.iter().map(|(at, value)| serde_json::json!([at, value])).collect();
        serde_json::json!({ "param": self.param, "points": points })
    }

    /// The lane as it is read.
    pub fn from_json(value: &Json) -> Result<Self, String> {
        let param = value.get("param").and_then(|param| param.as_str()).unwrap_or("").to_string();
        let mut points = Vec::new();
        if let Some(list) = value.get("points").and_then(|points| points.as_array()) {
            for point in list {
                let at = point.get(0).and_then(|at| at.as_i64()).unwrap_or(0);
                let amount = point.get(1).and_then(|amount| amount.as_f64()).unwrap_or(0.0);
                if at < 0 || !amount.is_finite() {
                    return Err("a breakpoint is at a time and holds a number".to_string());
                }
                points.push((at, amount));
            }
        }
        Ok(LanePayload { param, points })
    }
}

/// Make a bus.
#[derive(Command)]
#[command(
    id = "sound.bus.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "New"),
    mutates,
    undoable
)]
pub struct AddBus {
    /// What to call it.
    pub name: Text,
    /// How many channels it has.
    pub channels: Option<Number>,
}

impl AddBus {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let channels = match &self.channels {
            Some(channels) => channels.0 as u32,
            None => 2,
        };
        let payload = BusPayload { name: name.clone(), channels };
        let id = session.document_mut()?.add_node_with(name.clone(), "sound.bus", None, payload.to_json());
        Ok(Outcome::changed(format!("added a bus `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name })))
    }
}

/// Send a track to a bus.
///
/// This is a **relation**: the track keeps the bus in its routing, and the
/// two nodes are not in each other.
#[derive(Command)]
#[command(
    id = "sound.route.add",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Track", "Route"),
    mutates,
    undoable
)]
pub struct AddRoute {
    /// The track that feeds the bus.
    pub track: NodeRef,
    /// The bus it feeds.
    pub bus: NodeRef,
}

impl AddRoute {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let bus = self.bus.0;
        for (id, what) in [(track, "track"), (bus, "bus")] {
            if session.document().node(id).is_none() {
                return Err(EngineError::Failed {
                    id: "sound.route.add".to_string(),
                    message: format!("there is no {what} `{id}`"),
                });
            }
        }
        let relation = craftkit_sdk::Relation::new(track, bus, "route");
        if !session.document_mut()?.add_relation(relation) {
            return Err(EngineError::Failed {
                id: "sound.route.add".to_string(),
                message: format!("the track `{track}` already feeds the bus `{bus}`"),
            });
        }
        Ok(Outcome::changed(format!("the track `{track}` feeds the bus `{bus}`"))
            .with_data(serde_json::json!({ "track": track, "bus": bus })))
    }
}

/// Make a track **read** a bus.
///
/// A bus is a **destination**: sound collects there and is not heard
/// there. Something must read it and take it into the mix — the tree's
/// aux is a strip that reads a bus, and the bus is silent on its own.
///
/// The reading is a **relation**: the node says what it is and the
/// relation says what it reads.
#[derive(Command)]
#[command(
    id = "sound.track.input",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Track", "Input"),
    mutates,
    undoable
)]
pub struct AddInput {
    /// The track that reads the bus.
    pub track: NodeRef,
    /// The bus it reads.
    pub bus: NodeRef,
}

impl AddInput {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let bus = self.bus.0;
        match session.document().node(track) {
            Some(node) if node.kind == "sound.track" => {}
            Some(node) => {
                return Err(EngineError::Failed {
                    id: "sound.track.input".to_string(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            None => {
                return Err(EngineError::Failed {
                    id: "sound.track.input".to_string(),
                    message: format!("there is no track `{track}`"),
                });
            }
        }
        match session.document().node(bus) {
            Some(node) if node.kind == "sound.bus" => {}
            Some(node) => {
                return Err(EngineError::Failed {
                    id: "sound.track.input".to_string(),
                    message: format!("the node `{bus}` is a `{}` and not a bus", node.kind),
                });
            }
            None => {
                return Err(EngineError::Failed {
                    id: "sound.track.input".to_string(),
                    message: format!("there is no bus `{bus}`"),
                });
            }
        }
        // A track reads **one** bus: a reader with two inputs is a
        // reader with no meaning.
        for relation in session.document().relations_from(track, Some("input")) {
            if relation.to != bus {
                return Err(EngineError::Failed {
                    id: "sound.track.input".to_string(),
                    message: format!("the track `{track}` already reads the bus `{}`", relation.to),
                });
            }
        }
        let relation = craftkit_sdk::Relation::new(track, bus, "input");
        if !session.document_mut()?.add_relation(relation) {
            return Err(EngineError::Failed {
                id: "sound.track.input".to_string(),
                message: format!("the track `{track}` already reads the bus `{bus}`"),
            });
        }
        Ok(Outcome::changed(format!("the track `{track}` reads the bus `{bus}`"))
            .with_data(serde_json::json!({ "track": track, "bus": bus })))
    }
}

/// Whether a track **reads** a bus.
///
/// A reader takes the bus's sound into the mix, and the bus is not
/// heard by itself: the tree's bus is read by an aux and is silent on
/// its own.
pub fn reads_bus(document: &Document, track: u64) -> bool {
    !document.relations_from(track, Some("input")).is_empty()
}

/// Where a node **feeds**: the bus it sends its sound to.
///
/// A relation of the kind `route` says that the sound of one node
/// goes into another, and the node that goes nowhere is heard in the
/// main mix.
pub fn route_of(document: &Document, node: u64) -> Option<u64> {
    document.relations_from(node, Some("route")).into_iter().map(|relation| relation.to).next()
}

/// The **order of the buses**: a bus that feeds another bus comes
/// before it, so that a bus is fed before it is read.
///
/// The list is short and the order is worked out by walking it, so a
/// cycle in the routing is not a hang: a bus that is already in the
/// order is left where it is.
pub fn bus_order(document: &Document) -> Vec<u64> {
    let buses: Vec<&Node> = document.walk().filter(|node| node.kind == "sound.bus").collect();
    let mut order: Vec<u64> = Vec::new();
    for bus in &buses {
        if !order.contains(&bus.id) {
            order.push(bus.id);
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for bus in &buses {
            let feeds: Vec<u64> = document
                .relations_to(bus.id, Some("route"))
                .into_iter()
                .map(|relation| relation.from)
                .filter(|from| buses.iter().any(|node| node.id == *from))
                .collect();
            for from in feeds {
                let from_index = order.iter().position(|id| *id == from);
                let bus_index = order.iter().position(|id| *id == bus.id);
                if let (Some(from_index), Some(bus_index)) = (from_index, bus_index)
                    && from_index > bus_index
                {
                    order.remove(from_index);
                    let bus_index = order.iter().position(|id| *id == bus.id).unwrap_or(0);
                    order.insert(bus_index, from);
                    changed = true;
                }
            }
        }
    }
    order
}

/// The tracks that **read** a bus: the reader and the bus it reads.
pub fn readers_of(document: &Document) -> Vec<(u64, u64)> {
    let mut readers = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.track") {
        for relation in document.relations_from(node.id, Some("input")) {
            readers.push((node.id, relation.to));
        }
    }
    readers
}

/// A **stage** of the mix: a bus, or a reader and the bus it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// A bus, and the signal it holds.
    Bus(u64),
    /// A reader, and the bus it reads.
    Reader(u64, u64),
}

/// The **order of the mix**: the buses and the readers, in the order
/// they are mixed.
///
/// A bus is mixed before the readers that read it, and a reader is
/// mixed before the bus it feeds — so the order is worked out from the
/// routes, and a mix that loops is refused by name.
pub fn stage_order(document: &Document) -> Result<Vec<Stage>, String> {
    let mut stages: Vec<Stage> = Vec::new();
    for bus in document.walk().filter(|node| node.kind == "sound.bus") {
        stages.push(Stage::Bus(bus.id));
    }
    for (reader, bus) in readers_of(document) {
        stages.push(Stage::Reader(reader, bus));
    }
    let index_of = |id: u64| {
        stages.iter().position(|stage| match stage {
            Stage::Bus(bus) => *bus == id,
            Stage::Reader(reader, _) => *reader == id,
        })
    };
    // The **edges**: what must be mixed before what.
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (index, stage) in stages.iter().enumerate() {
        match stage {
            Stage::Bus(bus) => {
                // A reader that reads this bus is mixed after it.
                for (reader, read) in readers_of(document) {
                    if read == *bus
                        && let Some(after) = index_of(reader)
                    {
                        edges.push((index, after));
                    }
                }
                // A bus that feeds another bus is mixed before it.
                if let Some(destination) = route_of(document, *bus)
                    && let Some(after) = index_of(destination)
                {
                    edges.push((index, after));
                }
            }
            Stage::Reader(reader, _) => {
                // A reader that feeds a bus is mixed before it.
                if let Some(destination) = route_of(document, *reader)
                    && let Some(after) = index_of(destination)
                {
                    edges.push((index, after));
                }
            }
        }
    }
    // The **order**: the stages with nothing before them, one after
    // another.
    let mut done: Vec<bool> = vec![false; stages.len()];
    let mut order: Vec<Stage> = Vec::with_capacity(stages.len());
    for _ in 0..stages.len() {
        let mut chosen = None;
        for index in 0..stages.len() {
            if done[index] {
                continue;
            }
            let waiting = edges.iter().any(|(before, after)| *after == index && !done[*before]);
            if !waiting {
                chosen = Some(index);
                break;
            }
        }
        let Some(index) = chosen else {
            return Err("the mix loops: a bus and a reader feed each other".to_string());
        };
        done[index] = true;
        order.push(stages[index]);
    }
    Ok(order)
}

/// Put a plugin in a slot on a track.
#[derive(Command)]
#[command(
    id = "sound.insert.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Inserts"),
    mutates,
    undoable
)]
pub struct AddInsert {
    /// The track the insert is on.
    ///
    /// An insert is on **one** thing: a track or a clip, and not both.
    pub track: Option<NodeRef>,
    /// The clip the insert is on.
    ///
    /// A clip may carry effects of its own: the tree's clip effects are
    /// an equaliser and a compressor on one clip and not on the track.
    pub clip: Option<NodeRef>,
    /// Which plugin it is.
    pub plugin: Text,
    /// Whether it is bypassed.
    pub bypass: Option<craftkit_sdk::Bool>,
}

impl AddInsert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // An insert is on **one** thing: a track or a clip.
        let (owner, owner_id, kind) = match (self.track, self.clip) {
            (Some(track), None) => ("track", track.0, "sound.track"),
            (None, Some(clip)) => ("clip", clip.0, "sound.clip"),
            (Some(_), Some(_)) => {
                return Err(EngineError::Failed {
                    id: "sound.insert.add".to_string(),
                    message: "an insert is on one thing: a track or a clip".to_string(),
                });
            }
            (None, None) => {
                return Err(EngineError::Failed {
                    id: "sound.insert.add".to_string(),
                    message: "an insert is on something: which track, and which clip?".to_string(),
                });
            }
        };
        match session.document().node(owner_id) {
            Some(node) if node.kind == kind => {}
            Some(node) => {
                return Err(EngineError::Failed {
                    id: "sound.insert.add".to_string(),
                    message: format!("the node `{owner_id}` is a `{}` and not a {owner}", node.kind),
                });
            }
            None => {
                return Err(EngineError::Failed {
                    id: "sound.insert.add".to_string(),
                    message: format!("there is no {owner} `{owner_id}`"),
                });
            }
        }
        let plugin = self.plugin.0.clone();
        if plugin.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "sound.insert.add".to_string(),
                message: "an insert says which plugin it is".to_string(),
            });
        }
        let bypass = self.bypass.map(|bypass| bypass.0).unwrap_or(false);
        let payload =
            InsertPayload { plugin: plugin.clone(), params: Json::Object(Default::default()), bypass };
        let id = session.document_mut()?.add_node_with_under(
            Some(owner_id),
            plugin.clone(),
            "sound.insert",
            None,
            payload.to_json(),
        );
        Ok(Outcome::changed(format!("added a `{plugin}` insert on the {owner} `{owner_id}`"))
            .with_data(serde_json::json!({ "id": id, owner: owner_id, "plugin": plugin })))
    }
}

/// Change an **insert**: its parameters, or whether it is bypassed.
///
/// An insert's parameters are how a plugin is told what to do, and a
/// document may not change them by itself: the change is a **command**,
/// like every other change.
#[derive(Command)]
#[command(
    id = "sound.insert.set",
    area = "sound",
    layer = "sound",
    icon = "pencil",
    menu("Track", "Inserts"),
    mutates,
    undoable
)]
pub struct SetInsert {
    /// The insert to change.
    pub insert: NodeRef,
    /// Its parameters, by name: a table of parameter to number.
    pub parameters: Option<Json>,
    /// Whether it is bypassed.
    pub bypass: Option<craftkit_sdk::Bool>,
}

impl SetInsert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.insert.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.insert.set".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        if node.kind != "sound.insert" {
            return Err(EngineError::Failed {
                id: "sound.insert.set".to_string(),
                message: format!("node {id} is not an insert"),
            });
        }
        // The parameters are **numbers**, and a value that is not a
        // number is **left out**: the reader's rule (§43), and the
        // parameters of a plugin are numbers.
        let params = match &self.parameters {
            Some(params) => {
                let mut values = serde_json::Map::new();
                if let Some(table) = params.as_object() {
                    for (name, value) in table {
                        if let Some(number) = value.as_f64()
                            && number.is_finite()
                        {
                            values.insert(name.clone(), serde_json::json!(number));
                        }
                    }
                }
                Some(values)
            }
            None => None,
        };
        let bypass = self.bypass.map(|bypass| bypass.0);
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.insert.set".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        if let Some(params) = params {
            node.data["params"] = serde_json::Value::Object(params);
        }
        if let Some(bypass) = bypass {
            node.data["bypass"] = serde_json::json!(bypass);
        }
        Ok(Outcome::changed(format!("changed the insert {id}")).with_data(serde_json::json!({ "id": id })))
    }
}

/// Send a track to a bus at a level.
///
/// The send is a node under the track, and the bus it goes to is a
/// relation: the node says what the send *is* and the relation says where
/// it goes.
#[derive(Command)]
#[command(
    id = "sound.send.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Sends"),
    mutates,
    undoable
)]
pub struct AddSend {
    /// The track the send is on.
    pub track: NodeRef,
    /// The bus it goes to.
    pub bus: NodeRef,
    /// Its level, in decibels.
    pub level_db: Option<Number>,
    /// Its pan, from minus one to one.
    pub pan: Option<Number>,
    /// Whether the send is silent.
    pub mute: Option<Bool>,
    /// Whether the send is taken before the track's fader.
    pub pre_fader: Option<Bool>,
}

impl AddSend {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let bus = self.bus.0;
        for (id, what) in [(track, "track"), (bus, "bus")] {
            if session.document().node(id).is_none() {
                return Err(EngineError::Failed {
                    id: "sound.send.add".to_string(),
                    message: format!("there is no {what} `{id}`"),
                });
            }
        }
        let level_db = match &self.level_db {
            Some(level) => level.0,
            None => 0.0,
        };
        if !level_db.is_finite() {
            return Err(EngineError::Failed {
                id: "sound.send.add".to_string(),
                message: "a level is a number".to_string(),
            });
        }
        let pan = match &self.pan {
            Some(pan) => pan.0,
            None => 0.0,
        };
        if !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
            return Err(EngineError::Failed {
                id: "sound.send.add".to_string(),
                message: "a pan is a number between minus one and one".to_string(),
            });
        }
        let mute = self.mute.map(|mute| mute.0).unwrap_or(false);
        let pre_fader = self.pre_fader.map(|pre| pre.0).unwrap_or(false);
        // A **reader** carries the bus's sound and nothing else: it
        // takes a bus into the mix and does not copy itself anywhere.
        if reads_bus(session.document(), track) {
            return Err(EngineError::Failed {
                id: "sound.send.add".to_string(),
                message: format!("the track `{track}` reads a bus, and it has no sends"),
            });
        }
        let payload = SendPayload { level_db, pan, mute, pre_fader };
        let id = session.document_mut()?.add_node_with_under(
            Some(track),
            format!("send to {bus}"),
            "sound.send",
            None,
            payload.to_json(),
        );
        let relation = craftkit_sdk::Relation::new(id, bus, "send");
        session.document_mut()?.add_relation(relation);
        Ok(Outcome::changed(format!("the track `{track}` sends to the bus `{bus}`"))
            .with_data(serde_json::json!({ "id": id, "track": track, "bus": bus })))
    }
}

/// Automate a parameter of a track.
///
/// The lane says which parameter it moves, and the parameter is a
/// **string**: `volume`, `pan`, `mute`, or anything a node's payload
/// names.
#[derive(Command)]
#[command(
    id = "sound.lane.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct AddLane {
    /// The track whose parameter is automated. A lane automates **one**
    /// thing: a track or a clip, and not both.
    pub track: Option<NodeRef>,
    /// The clip whose parameter is automated.
    pub clip: Option<NodeRef>,
    /// Which parameter: `volume`, `pan`, `mute`, …
    pub param: Text,
    /// The breakpoints: a list of `[at, value]`, where `at` is a time in
    /// samples.
    pub points: Option<Json>,
}

impl AddLane {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // A lane automates **one** thing: a track or a clip. The
        // family's clip carries a curve of its own, and the tree's
        // lane is that curve when it is under a clip.
        let (owner, owner_id) = match (self.track, self.clip) {
            (Some(track), None) => ("track", track.0),
            (None, Some(clip)) => ("clip", clip.0),
            (Some(_), Some(_)) => {
                return Err(EngineError::Failed {
                    id: "sound.lane.add".to_string(),
                    message: "a lane automates one thing: a track or a clip".to_string(),
                });
            }
            (None, None) => {
                return Err(EngineError::Failed {
                    id: "sound.lane.add".to_string(),
                    message: "a lane automates something: which track, and which clip?".to_string(),
                });
            }
        };
        let kind = if owner == "track" { "sound.track" } else { "sound.clip" };
        match session.document().node(owner_id) {
            Some(node) if node.kind == kind => {}
            Some(node) => {
                return Err(EngineError::Failed {
                    id: "sound.lane.add".to_string(),
                    message: format!("the node `{owner_id}` is a `{}` and not a {owner}", node.kind),
                });
            }
            None => {
                return Err(EngineError::Failed {
                    id: "sound.lane.add".to_string(),
                    message: format!("there is no {owner} `{owner_id}`"),
                });
            }
        }
        let param = self.param.0.clone();
        if param.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "sound.lane.add".to_string(),
                message: "a lane says which parameter it moves".to_string(),
            });
        }
        let points = match &self.points {
            Some(points) => {
                let value = serde_json::json!({ "param": param, "points": points });
                LanePayload::from_json(&value)
                    .map_err(|error| EngineError::Failed {
                        id: "sound.lane.add".to_string(),
                        message: format!("the breakpoints are not breakpoints: {error}"),
                    })?
                    .points
            }
            None => Vec::new(),
        };
        let payload = LanePayload { param: param.clone(), points };
        let id = session.document_mut()?.add_node_with_under(
            Some(owner_id),
            format!("{param} automation"),
            "sound.lane",
            None,
            payload.to_json(),
        );
        Ok(Outcome::changed(format!("the {owner} `{owner_id}` automates `{param}`"))
            .with_data(serde_json::json!({ "id": id, "owner": owner_id, "param": param })))
    }
}

/// The routing: who feeds whom, and in what order.
///
/// This is **derived state** over the relations: the document holds the
/// edges and the evaluator works out what they mean. The order is the
/// order the buses are fed in — a bus that nothing feeds comes first, and
/// a bus that feeds another comes before it.
pub fn routing(document: &Document) -> Vec<Json> {
    let order = bus_order(document);
    order
        .into_iter()
        .filter_map(|id| {
            let bus = document.node(id)?;
            let payload = BusPayload::from_json(&bus.data).ok()?;
            let fed_by: Vec<Json> = document
                .relations_to(id, Some("route"))
                .into_iter()
                .map(|relation| serde_json::json!({ "from": relation.from }))
                .collect();
            Some(serde_json::json!({
                "id": id,
                "name": payload.name,
                "channels": payload.channels,
                "fed_by": fed_by,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// A track and a bus, and the route between them.
    #[test]
    fn a_track_feeds_a_bus() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let bus = session.document().node_by_name("mix").expect("the bus is there").id;
        session
            .execute("sound.route.add", &json!({ "track": track, "bus": bus }))
            .expect("the route is added");
        let relations = &session.document().relations;
        assert_eq!(relations.len(), 1, "the route is a relation: {relations:?}");
        assert_eq!(relations[0].from, track);
        assert_eq!(relations[0].to, bus);
        assert_eq!(relations[0].kind, "route");
    }

    /// The **bus** is a row of the mixer, and the mixer is **under** the
    /// tracks.
    #[test]
    fn a_bus_is_a_row_under_the_tracks() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
        let bus = session.document().node_by_name("mix").expect("the bus is there");
        let rect = BusKind::place(bus, session.document()).expect("the bus has a place");
        assert_eq!(rect.y, timeline::ROW_HEIGHT, "the bus is one row below the track");
        assert_eq!(rect.h, timeline::ROW_HEIGHT, "and it is a row");
    }

    /// A **send** is the line from the track down to the bus.
    #[test]
    fn a_send_is_the_line_from_the_track_to_the_bus() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let bus = session.document().node_by_name("mix").expect("the bus is there").id;
        session.execute("sound.send.add", &json!({ "track": track, "bus": bus })).expect("the send is added");
        let send =
            session.document().walk().find(|node| node.kind == "sound.send").expect("the send is there");
        let rect = SendKind::place(send, session.document()).expect("the send has a place");
        assert_eq!(rect.y, 0.0, "the line starts on the track's row");
        assert_eq!(rect.h, timeline::ROW_HEIGHT, "and it reaches the bus's row");
    }

    /// An **insert** is a box at the left of the track's row.
    #[test]
    fn an_insert_is_a_box_at_the_left_of_the_row() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "reverb" }))
            .expect("the insert is added");
        let insert =
            session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert is there");
        let rect = InsertKind::place(insert, session.document()).expect("the insert has a place");
        assert_eq!(rect.x, 0.0, "the box is at the left");
        assert_eq!(rect.w, timeline::INSERT_WIDTH, "and it is as wide as an insert is");
        assert_eq!(rect.y, 0.0, "on the track's row");
    }

    /// An **automation lane** is a curve on the track's row.
    #[test]
    fn an_automation_lane_is_a_curve_on_the_row() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute(
                "sound.lane.add",
                &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [48000, 1.0]] }),
            )
            .expect("the lane is added");
        let lane =
            session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
        let rect = LaneKind::place(lane, session.document()).expect("the lane has a place");
        assert_eq!(rect.x, 0.0, "the curve starts at the beginning");
        assert_eq!(rect.w, 1.0, "and it is a second long");
        assert_eq!(rect.y, 0.0, "on the track's row");
        let points = timeline::lane_points(session.document(), lane);
        assert_eq!(points.len(), 2, "and it has two points");
        assert!((points[0].1 - 0.0).abs() < 0.001, "the first is at the bottom");
        assert!((points[1].1 - 1.0).abs() < 0.001, "and the second at the top");
    }

    /// The same route twice is refused: one edge is one relation.
    #[test]
    fn a_route_twice_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let bus = session.document().node_by_name("mix").expect("the bus is there").id;
        session
            .execute("sound.route.add", &json!({ "track": track, "bus": bus }))
            .expect("the route is added");
        let error = session
            .execute("sound.route.add", &json!({ "track": track, "bus": bus }))
            .expect_err("the second route is refused");
        assert!(error.to_string().contains("already feeds"), "{error}");
    }

    /// A route to a bus that is not there is refused.
    #[test]
    fn a_route_to_nothing_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.route.add", &json!({ "track": track, "bus": 9999 }))
            .expect_err("the route is refused");
        assert!(error.to_string().contains("no bus"), "{error}");
    }

    /// A clip plays its source, and that is a relation and not
    /// containment.
    #[test]
    fn a_clip_plays_its_source() {
        let mut session = session();
        session.execute("sound.source.add", &json!({ "path": "kit/kick.wav" })).expect("the source is added");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let source = session.document().node_by_name("kit/kick.wav").expect("the source is there").id;
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute(
                "sound.clip.add",
                &json!({ "track": track, "start": 0, "length": 48000, "source": source }),
            )
            .expect("the clip is added");
        let clip = session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip");
        assert!(clip.children.is_empty(), "the clip does not hold the source");
        assert_eq!(session.document().relations.len(), 1, "the clip is linked to the source");
        assert_eq!(session.document().relations[0].from, clip.id);
        assert_eq!(session.document().relations[0].to, source);
        assert_eq!(session.document().relations[0].kind, "source");
    }

    /// A send goes to a bus, and the send says what it is while the
    /// relation says where it goes.
    #[test]
    fn a_send_goes_to_a_bus() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let bus = session.document().node_by_name("reverb").expect("the bus is there").id;
        session
            .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": -12.0 }))
            .expect("the send is added");
        let send = session.document().walk().find(|node| node.kind == "sound.send").expect("the send");
        assert_eq!(send.data.get("level_db"), Some(&json!(-12.0)));
        assert_eq!(session.document().relations[0].from, send.id);
        assert_eq!(session.document().relations[0].to, bus);
        assert_eq!(session.document().relations[0].kind, "send");
    }

    /// An insert is a node on the track, and the plugin is a string.
    #[test]
    fn an_insert_is_a_node_on_the_track() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "comp" }))
            .expect("the insert is added");
        let insert = session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert");
        assert_eq!(insert.data.get("plugin"), Some(&json!("comp")));
        let track_node = session.document().node(track).expect("the track is there");
        assert!(
            track_node.children.iter().any(|child| child.kind == "sound.insert"),
            "the insert is on the track: {:?}",
            track_node.children.iter().map(|child| &child.kind).collect::<Vec<_>>()
        );
    }

    /// A lane automates a parameter, and the parameter is a string.
    #[test]
    fn a_lane_automates_a_parameter() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute(
                "sound.lane.add",
                &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [48000, -6.0]] }),
            )
            .expect("the lane is added");
        let lane = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane");
        assert_eq!(lane.data.get("param"), Some(&json!("volume")));
        assert_eq!(lane.data.get("points"), Some(&json!([[0, 0.0], [48000, -6.0]])));
    }

    /// A lane with a breakpoint that is not a breakpoint is refused.
    #[test]
    fn a_lane_with_a_bad_breakpoint_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.lane.add", &json!({ "track": track, "param": "volume", "points": [[-1, 0.0]] }))
            .expect_err("the lane is refused");
        assert!(error.to_string().contains("breakpoints"), "{error}");
    }

    /// The routing is derived: the buses come out in the order they are
    /// fed in.
    #[test]
    fn the_routing_is_derived() {
        let mut session = session();
        session.execute("sound.bus.add", &json!({ "name": "mix" })).expect("the bus is added");
        session.execute("sound.bus.add", &json!({ "name": "master" })).expect("the bus is added");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let mix = session.document().node_by_name("mix").expect("the bus is there").id;
        let master = session.document().node_by_name("master").expect("the bus is there").id;
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        // The track feeds the mix, and the mix feeds the master, so the
        // mix comes first.
        session.execute("sound.route.add", &json!({ "track": track, "bus": master })).expect("the route");
        session.execute("sound.route.add", &json!({ "track": track, "bus": mix })).expect("the route");
        let order = routing(session.document());
        let names: Vec<&str> =
            order.iter().filter_map(|bus| bus.get("name").and_then(|name| name.as_str())).collect();
        assert!(names.contains(&"mix"), "{names:?}");
        assert!(names.contains(&"master"), "{names:?}");
        let mix_index = names.iter().position(|name| *name == "mix").expect("the mix is there");
        let master_index = names.iter().position(|name| *name == "master").expect("the master is there");
        assert!(mix_index < master_index, "the mix is fed before the master: {names:?}");
    }

    /// The routing evaluator is in the ecosystem.
    #[test]
    fn the_routing_evaluator_is_there() {
        let ecosystem = ecosystem();
        assert!(ecosystem.evaluators.get("sound.routing").is_some());
        assert!(ecosystem.commands.get("sound.bus.add").is_some());
        assert!(ecosystem.commands.get("sound.route.add").is_some());
        assert!(ecosystem.commands.get("sound.send.add").is_some());
        assert!(ecosystem.commands.get("sound.insert.add").is_some());
        assert!(ecosystem.commands.get("sound.lane.add").is_some());
    }
}
