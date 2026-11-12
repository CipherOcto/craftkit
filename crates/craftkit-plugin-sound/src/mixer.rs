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

use craftkit_sdk::serde_json::json;

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

/// **Take away** a route: the track stops feeding the bus.
///
/// A route is a **relation** and not a node, and a relation that
/// cannot be taken away is a mistake that cannot be undone: the track
/// keeps feeding the bus and the mixer keeps mixing it in.
///
/// A route that is not there is **refused**, and it is refused by
/// name: a link that was never made is not a link that was taken away.
#[derive(Command)]
#[command(
    id = "sound.route.remove",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Track", "Route"),
    mutates,
    undoable
)]
pub struct RemoveRoute {
    /// The track that fed the bus.
    pub track: NodeRef,
    /// The bus it fed.
    pub bus: NodeRef,
}

impl RemoveRoute {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.route.remove".to_string();
        let track = self.track.0;
        let bus = self.bus.0;
        for (node, what) in [(track, "track"), (bus, "bus")] {
            if session.document().node(node).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no {what} `{node}`"),
                });
            }
        }
        let removed = session.document_mut()?.remove_relations(|relation| {
            relation.from == track && relation.to == bus && relation.kind == "route"
        });
        if removed == 0 {
            return Err(EngineError::Failed {
                id,
                message: format!("the track `{track}` does not feed the bus `{bus}`"),
            });
        }
        Ok(Outcome::changed(format!("the track `{track}` no longer feeds the bus `{bus}`"))
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

/// **Take away** an insert: the plugin is not on the thing any more.
///
/// The tree's insert lives in a **slot** on the track, and taking it
/// away empties the slot. Here the insert is a **node** under the
/// thing it is on, and taking it away takes the node away: there is
/// no slot left empty, because there was never a slot.
///
/// A node that is **not an insert** is refused.
#[derive(Command)]
#[command(
    id = "sound.insert.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Inserts"),
    mutates,
    undoable
)]
pub struct RemoveInsert {
    /// The insert to take away.
    pub insert: NodeRef,
}

impl RemoveInsert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.insert.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.insert.remove".to_string(),
                message: format!("there is no node `{id}` in the document"),
            });
        };
        if node.kind != "sound.insert" {
            return Err(EngineError::Failed {
                id: "sound.insert.remove".to_string(),
                message: format!("the node `{id}` is a `{}` and not an insert", node.kind),
            });
        }
        let owner = crate::timeline::parent_of(session.document(), id).map(|owner| owner.id);
        session.document_mut()?.remove_node(id);
        Ok(Outcome::changed(format!("took the insert `{id}` away"))
            .with_data(serde_json::json!({ "id": id, "owner": owner })))
    }
}

/// **Move** an insert: it and another one swap their places in the
/// chain.
///
/// The chain is the **order** of the inserts on the thing they are on,
/// and the order is the order they are run in. The tree's insert lives
/// in a **slot** and the move swaps two slots; here the insert is a
/// **node** and the move swaps two nodes.
///
/// Two inserts of **different** things are refused: a chain is the
/// chain of one thing, and there is no chain that holds both.
#[derive(Command)]
#[command(
    id = "sound.insert.move",
    area = "sound",
    layer = "sound",
    icon = "move",
    menu("Track", "Inserts"),
    mutates,
    undoable
)]
pub struct MoveInsert {
    /// The insert to move.
    pub insert: NodeRef,
    /// The insert it swaps places with.
    pub other: NodeRef,
}

impl MoveInsert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.insert.move".to_string();
        let (insert, other) = (self.insert.0, self.other.0);
        for node_id in [insert, other] {
            match session.document().node(node_id) {
                Some(node) if node.kind == "sound.insert" => {}
                Some(node) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{node_id}` is a `{}` and not an insert", node.kind),
                    });
                }
                None => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{node_id}` in the document"),
                    });
                }
            }
        }
        let owner = crate::timeline::parent_of(session.document(), insert).map(|owner| owner.id);
        let other_owner = crate::timeline::parent_of(session.document(), other).map(|owner| owner.id);
        if owner != other_owner {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the two inserts are not on the same thing: a chain is the chain of one thing"
                    .to_string(),
            });
        }
        let Some(owner_id) = owner else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the insert `{insert}` is on nothing"),
            });
        };
        let Some(node) = session.document_mut()?.node_mut(owner_id) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{owner_id}` in the document"),
            });
        };
        let children = &mut node.children;
        let Some(from) = children.iter().position(|child| child.id == insert) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the insert `{insert}` is not on `{owner_id}`"),
            });
        };
        let Some(to) = children.iter().position(|child| child.id == other) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the insert `{other}` is not on `{owner_id}`"),
            });
        };
        children.swap(from, to);
        Ok(Outcome::changed(format!("moved the insert `{insert}` and `{other}`"))
            .with_data(serde_json::json!({ "insert": insert, "other": other, "owner": owner_id })))
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

/// **Take away** the reading of a bus: the track stops reading it.
///
/// A bus is silent on its own: something must read it and take it into
/// the mix, and the reading is a **relation**. A relation that cannot
/// be taken away is a mistake that cannot be undone.
#[derive(Command)]
#[command(
    id = "sound.track.input.remove",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Track", "Input"),
    mutates,
    undoable
)]
pub struct RemoveInput {
    /// The track that read the bus.
    pub track: NodeRef,
    /// The bus it read.
    pub bus: NodeRef,
}

impl RemoveInput {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.input.remove".to_string();
        let track = self.track.0;
        let bus = self.bus.0;
        for (node, what) in [(track, "track"), (bus, "bus")] {
            if session.document().node(node).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no {what} `{node}`"),
                });
            }
        }
        let removed = session.document_mut()?.remove_relations(|relation| {
            relation.from == track && relation.to == bus && relation.kind == "input"
        });
        if removed == 0 {
            return Err(EngineError::Failed {
                id,
                message: format!("the track `{track}` does not read the bus `{bus}`"),
            });
        }
        Ok(Outcome::changed(format!("the track `{track}` no longer reads the bus `{bus}`"))
            .with_data(serde_json::json!({ "track": track, "bus": bus })))
    }
}

/// **Change** a send: its level, its pan, whether it is silent, and
/// whether it is taken before the track's fader.
///
/// The tree's send has a command for its **level** and another for the
/// rest, and this one has a command for the send's own settings: the
/// send is a thing, and the things about it are the send's.
///
/// A level outside the tree's range is **refused** and not pulled to
/// the edge, and a pan outside minus one and one is refused too.
#[derive(Command)]
#[command(
    id = "sound.send.set",
    area = "sound",
    layer = "sound",
    icon = "sliders",
    menu("Track", "Send"),
    mutates,
    undoable
)]
pub struct SetSend {
    /// The send to change.
    pub send: NodeRef,
    /// Its level, in decibels: from minus one hundred and forty-four to
    /// twelve.
    pub level_db: Option<Number>,
    /// Its pan, from minus one to one.
    pub pan: Option<Number>,
    /// Whether the send is silent.
    pub mute: Option<craftkit_sdk::Bool>,
    /// Whether the send is taken before the track's fader.
    pub pre_fader: Option<craftkit_sdk::Bool>,
}

impl SetSend {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.send.set".to_string();
        let send = self.send.0;
        let Some(node) = session.document().node(send) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{send}` in the document"),
            });
        };
        if node.kind != "sound.send" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{send}` is a `{}` and not a send", node.kind),
            });
        }
        let mut payload = SendPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if let Some(level) = &self.level_db {
            let decibels = level.0;
            if !decibels.is_finite() || !(-144.0..=12.0).contains(&decibels) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message:
                        "a level is a number of decibels from minus one hundred and forty-four to twelve"
                            .to_string(),
                });
            }
            payload.level_db = decibels;
        }
        if let Some(pan) = &self.pan {
            let value = pan.0;
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a pan is a number between minus one and one".to_string(),
                });
            }
            payload.pan = value;
        }
        if let Some(mute) = &self.mute {
            payload.mute = mute.0;
        }
        if let Some(pre_fader) = &self.pre_fader {
            payload.pre_fader = pre_fader.0;
        }
        let Some(node) = session.document_mut()?.node_mut(send) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{send}` in the document"),
            });
        };
        node.data = payload.to_json();
        Ok(Outcome::changed(format!("changed the send `{send}`")).with_data(serde_json::json!({
            "id": send,
            "level_db": payload.level_db,
            "pan": payload.pan,
            "mute": payload.mute,
            "pre_fader": payload.pre_fader,
        })))
    }
}

/// **Take away** a send.
///
/// The send is a **node** under the track, and it goes with its link
/// to the bus: a send that is there and points at nothing is a send
/// that sends nothing, and it is litter.
#[derive(Command)]
#[command(
    id = "sound.send.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Send"),
    mutates,
    undoable
)]
pub struct RemoveSend {
    /// The send to take away.
    pub send: NodeRef,
}

impl RemoveSend {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.send.remove".to_string();
        let send = self.send.0;
        let node = session.document().node(send).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no send `{send}`"),
        })?;
        if node.kind != "sound.send" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{send}` is a `{}` and not a send", node.kind),
            });
        }
        session.document_mut()?.remove_node(send);
        Ok(Outcome::changed(format!("took the send `{send}` away"))
            .with_data(serde_json::json!({ "send": send })))
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

/// The **lane** of a command, read.
fn lane_of(session: &Session, id: &str, lane: u64) -> Result<LanePayload, EngineError> {
    let node = session.document().node(lane).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no lane `{lane}`"),
    })?;
    if node.kind != "sound.lane" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{lane}` is a `{}` and not a lane", node.kind),
        });
    }
    LanePayload::from_json(&node.data)
        .map_err(|error| EngineError::Failed { id: id.to_string(), message: error })
}

/// Write a lane's points back into the document.
fn write_lane(session: &mut Session, lane: u64, payload: &LanePayload) -> Result<(), EngineError> {
    if let Some(node) = session.document_mut()?.node_mut(lane) {
        node.data = payload.to_json();
    }
    Ok(())
}

/// The value a lane's parameter may have.
///
/// The **pan** is between minus one and one and the **mute** is
/// between zero and one, and a value outside is **refused**: the tree
/// clamps it into the range, and a value the caller did not ask for
/// is a value the caller does not know about.
///
/// The **volume** is any number of decibels, and a lane of any other
/// parameter is **drawn** and not heard (§ the automation), so its
/// value is only a number.
fn check_value(id: &str, param: &str, value: f64) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed { id: id.to_string(), message: "a value is a number".to_string() });
    }
    match param {
        "pan" if !(-1.0..=1.0).contains(&value) => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the pan is a number between minus one and one, and {value} is not one"),
        }),
        "mute" if !(0.0..=1.0).contains(&value) => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the mute is a number between zero and one, and {value} is not one"),
        }),
        _ => Ok(value),
    }
}

/// **Set a breakpoint** on a lane.
///
/// A breakpoint that is already there takes the new value, and a
/// breakpoint that is not there is added: the lane's points are in
/// order of their time, so "the breakpoint at that time" is a thing
/// that can be named.
#[derive(Command)]
#[command(
    id = "sound.lane.point.set",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct SetLanePoint {
    /// The lane the breakpoint goes on.
    pub lane: NodeRef,
    /// Where the breakpoint is, in the owner's samples.
    pub at: Number,
    /// The value at that time.
    pub value: Number,
}

impl SetLanePoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.point.set".to_string();
        let lane = self.lane.0;
        let mut payload = lane_of(session, &id, lane)?;
        let at = crate::samples(self.at.0, "where the breakpoint is")?;
        let value = check_value(&id, &payload.param, self.value.0)?;
        match payload.points.iter_mut().find(|(when, _)| *when == at) {
            Some(point) => point.1 = value,
            None => payload.points.push((at, value)),
        }
        payload.points.sort_by_key(|(at, _)| *at);
        write_lane(session, lane, &payload)?;
        Ok(Outcome::changed(format!("set a breakpoint on the lane `{lane}`"))
            .with_data(serde_json::json!({ "lane": lane, "at": at, "value": value })))
    }
}

/// **Take a breakpoint** off a lane.
///
/// A time that has no breakpoint on it is **refused**: a breakpoint
/// that is not there is not one that was taken away.
#[derive(Command)]
#[command(
    id = "sound.lane.point.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct RemoveLanePoint {
    /// The lane the breakpoint is on.
    pub lane: NodeRef,
    /// Where the breakpoint is, in the owner's samples.
    pub at: Number,
}

impl RemoveLanePoint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.point.remove".to_string();
        let lane = self.lane.0;
        let mut payload = lane_of(session, &id, lane)?;
        let at = crate::samples(self.at.0, "where the breakpoint is")?;
        let before = payload.points.len();
        payload.points.retain(|(when, _)| *when != at);
        if payload.points.len() == before {
            return Err(EngineError::Failed {
                id,
                message: format!("the lane `{lane}` has no breakpoint at {at}"),
            });
        }
        write_lane(session, lane, &payload)?;
        Ok(Outcome::changed(format!("took a breakpoint off the lane `{lane}`"))
            .with_data(serde_json::json!({ "lane": lane, "at": at })))
    }
}

/// **Clear** the breakpoints of a lane.
///
/// Without a range every breakpoint goes, and with one the breakpoints
/// in the range go and the rest stay: the tree's command clears a
/// range or the whole lane, and the range is the tree's selection.
#[derive(Command)]
#[command(
    id = "sound.lane.clear",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct ClearLane {
    /// The lane to clear.
    pub lane: NodeRef,
    /// The start of the range, in the owner's samples. The whole lane
    /// without it.
    pub start: Option<Number>,
    /// The end of the range, in the owner's samples.
    pub end: Option<Number>,
}

impl ClearLane {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.clear".to_string();
        let lane = self.lane.0;
        let mut payload = lane_of(session, &id, lane)?;
        let range = match (self.start, self.end) {
            (Some(start), Some(end)) => {
                let start = crate::samples(start.0, "where the range starts")?;
                let end = crate::samples(end.0, "where the range ends")?;
                Some((start, end))
            }
            (None, None) => None,
            _ => {
                return Err(EngineError::Failed { id, message: "a range is a start and an end".to_string() });
            }
        };
        let before = payload.points.len();
        match range {
            Some((start, end)) => payload.points.retain(|(at, _)| *at < start || *at >= end),
            None => payload.points.clear(),
        }
        let removed = before - payload.points.len();
        write_lane(session, lane, &payload)?;
        Ok(Outcome::changed(format!("cleared {removed} breakpoints of the lane `{lane}`"))
            .with_data(serde_json::json!({ "lane": lane, "removed": removed })))
    }
}

/// **Write** a value over a range of a lane.
///
/// The lane is **flat** over the range: a breakpoint at each end of it
/// with the value, and the curve outside the range is kept — with an
/// anchor just before and just after, so that the curve does not jump
/// where the range begins and ends.
#[derive(Command)]
#[command(
    id = "sound.lane.write",
    area = "sound",
    layer = "sound",
    icon = "pencil",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct WriteLane {
    /// The lane to write on.
    pub lane: NodeRef,
    /// Where the range starts, in the owner's samples.
    pub start: Number,
    /// Where the range ends, in the owner's samples.
    pub end: Number,
    /// The value over the range.
    pub value: Number,
}

impl WriteLane {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.write".to_string();
        let lane = self.lane.0;
        let mut payload = lane_of(session, &id, lane)?;
        let start = crate::samples(self.start.0, "where the range starts")?;
        let end = crate::samples(self.end.0, "where the range ends")?;
        let value = check_value(&id, &payload.param, self.value.0)?;
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range from {start} to {end} is empty"),
            });
        }
        // The curve before and after the range, so that the anchors
        // keep it.
        let before = crate::automate::value_at(&payload.points, start.saturating_sub(1));
        let after = crate::automate::value_at(&payload.points, end);
        payload.points.retain(|(at, _)| *at < start || *at > end);
        fn set(points: &mut Vec<(i64, f64)>, at: i64, value: f64) {
            match points.iter_mut().find(|(when, _)| *when == at) {
                Some(point) => point.1 = value,
                None => points.push((at, value)),
            }
        }
        if start > 0 {
            set(&mut payload.points, start - 1, before);
        }
        set(&mut payload.points, start, value);
        set(&mut payload.points, end, value);
        // A breakpoint that is already just after the range keeps its
        // own value.
        if !payload.points.iter().any(|(at, _)| *at == end + 1) {
            set(&mut payload.points, end + 1, after);
        }
        payload.points.sort_by_key(|(at, _)| *at);
        write_lane(session, lane, &payload)?;
        Ok(Outcome::changed(format!("wrote the lane `{lane}` over a range"))
            .with_data(serde_json::json!({ "lane": lane, "start": start, "end": end, "value": value })))
    }
}

/// **Thin** a lane: take away the breakpoints the curve does not need.
///
/// A breakpoint is taken away when the curve through the ones that
/// remain changes by less than the **tolerance** — the same shape of
/// curve without it. A tolerance of nothing takes away nothing, and a
/// large one takes away everything the curve can lose.
#[derive(Command)]
#[command(
    id = "sound.lane.thin",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct ThinLane {
    /// The lane to thin.
    pub lane: NodeRef,
    /// How much the curve may change, in the value of the parameter.
    pub tolerance: Option<Number>,
}

impl ThinLane {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.thin".to_string();
        let lane = self.lane.0;
        let mut payload = lane_of(session, &id, lane)?;
        let tolerance = match self.tolerance {
            Some(tolerance) if tolerance.0.is_finite() => tolerance.0.clamp(0.0, 100.0),
            Some(_) => {
                return Err(EngineError::Failed { id, message: "the tolerance is a number".to_string() });
            }
            None => 0.1,
        };
        let removed = thin_points(&mut payload.points, tolerance);
        write_lane(session, lane, &payload)?;
        Ok(Outcome::changed(format!("took {removed} breakpoints off the lane `{lane}`"))
            .with_data(serde_json::json!({ "lane": lane, "removed": removed })))
    }
}

/// The lanes of a **track**, by their parameter.
///
/// The lanes are the ones under the track: a lane automates one thing
/// and a track's lanes are under the track.
pub(crate) fn lanes_of(document: &Document, track: u64) -> Vec<(u64, LanePayload)> {
    let mut lanes: Vec<(u64, LanePayload)> = document
        .walk()
        .filter(|node| node.kind == "sound.lane")
        .filter(|node| crate::timeline::parent_of(document, node.id).map(|parent| parent.id) == Some(track))
        .filter_map(|node| LanePayload::from_json(&node.data).ok().map(|payload| (node.id, payload)))
        .collect();
    lanes.sort_by_key(|(_, payload)| payload.param.clone());
    lanes
}

/// **Thin** every lane of a track: take away the breakpoints the curves
/// do not need.
///
/// The tree's editor has a "Thin All": it thins every lane of every
/// track at once. Here it is **one track's** lanes, because a track is
/// a node and a node is something a command can be given.
#[derive(Command)]
#[command(
    id = "sound.lane.thin.all",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct ThinAllLanes {
    /// The track whose lanes are thinned.
    pub track: NodeRef,
    /// How much the curve may change, in the value of the parameter.
    pub tolerance: Option<Number>,
}

impl ThinAllLanes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.thin.all".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let tolerance = match self.tolerance {
            Some(tolerance) if tolerance.0.is_finite() => tolerance.0.clamp(0.0, 100.0),
            Some(_) => {
                return Err(EngineError::Failed { id, message: "the tolerance is a number".to_string() });
            }
            None => 0.1,
        };
        let lanes = lanes_of(session.document(), track);
        let mut removed = 0;
        for (lane, mut payload) in lanes {
            removed += thin_points(&mut payload.points, tolerance);
            write_lane(session, lane, &payload)?;
        }
        Ok(Outcome::changed(format!("took {removed} breakpoints off the lanes of the track `{track}`"))
            .with_data(serde_json::json!({ "track": track, "removed": removed })))
    }
}

/// **Write** the static value of every lane of a track over a range.
///
/// The tree's editor has a "Write to Current": it takes the **static**
/// value of each parameter — the fader, the pan, the mute — and writes
/// it over a range, so that the curve is flat over it and the value
/// before and after it is kept.
///
/// Here the static values are the **defaults of their lanes** (§ the
/// fader), and the tree's own mixer keeps a mute of its own beside the
/// mute lane: here the lane is the mute's default, and a lane without
/// breakpoints is the mute.
#[derive(Command)]
#[command(
    id = "sound.lane.write.all",
    area = "sound",
    layer = "sound",
    icon = "pencil",
    menu("Track", "Automation"),
    mutates,
    undoable
)]
pub struct WriteAllLanes {
    /// The track whose lanes are written.
    pub track: NodeRef,
    /// Where the range starts, in the owner's samples.
    pub start: Number,
    /// Where the range ends, in the owner's samples.
    pub end: Number,
}

impl WriteAllLanes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.lane.write.all".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let start = crate::samples(self.start.0, "where the range starts")?;
        let end = crate::samples(self.end.0, "where the range ends")?;
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range from {start} to {end} is empty"),
            });
        }
        // The **static values**: the fader, the pan and the mute are
        // the defaults of their lanes, and those are the values the
        // tree writes.
        let payload = crate::TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        let statics: Vec<(&str, f64)> = vec![
            ("volume", payload.fader_db),
            ("pan", payload.pan),
            ("mute", if payload.mute { 1.0 } else { 0.0 }),
        ];
        let lanes = lanes_of(session.document(), track);
        let mut written = 0;
        for (lane, mut lane_payload) in lanes {
            // The value this lane's parameter holds when it is not
            // automated: the track's own.
            let Some((_, value)) = statics.iter().find(|(param, _)| *param == lane_payload.param) else {
                continue;
            };
            let value = check_value(&id, &lane_payload.param, *value)?;
            // The curve before and after the range, so that the anchors
            // keep it.
            let before = crate::automate::value_at(&lane_payload.points, start.saturating_sub(1));
            let after = crate::automate::value_at(&lane_payload.points, end);
            lane_payload.points.retain(|(when, _)| *when < start || *when > end);
            fn set(points: &mut Vec<(i64, f64)>, at: i64, value: f64) {
                match points.iter_mut().find(|(when, _)| *when == at) {
                    Some(point) => point.1 = value,
                    None => points.push((at, value)),
                }
            }
            if start > 0 {
                set(&mut lane_payload.points, start - 1, before);
            }
            set(&mut lane_payload.points, start, value);
            set(&mut lane_payload.points, end, value);
            if !lane_payload.points.iter().any(|(when, _)| *when == end + 1) {
                set(&mut lane_payload.points, end + 1, after);
            }
            lane_payload.points.sort_by_key(|(at, _)| *at);
            write_lane(session, lane, &lane_payload)?;
            written += 1;
        }
        Ok(Outcome::changed(format!("wrote the lanes of the track `{track}` over a range"))
            .with_data(serde_json::json!({ "track": track, "lanes": written, "start": start, "end": end })))
    }
}

/// Take away the breakpoints of a curve that the curve does not need.
///
/// A breakpoint is taken away when the **slope** from the last
/// breakpoint that was kept to the next one can be reached from every
/// slope since then without leaving the tolerance: the curve through
/// the ones that remain is the same curve, within the tolerance.
///
/// The first and the last breakpoints are always kept: a curve needs
/// its ends.
fn thin_points(points: &mut Vec<(i64, f64)>, tolerance: f64) -> usize {
    if points.len() < 3 {
        return 0;
    }
    let mut keep = vec![true; points.len()];
    let mut last = 0_usize;
    let (mut low, mut high) = (f64::NEG_INFINITY, f64::INFINITY);
    for index in 1..points.len() - 1 {
        let (Some(a), Some(b), Some(c)) = (points.get(last), points.get(index), points.get(index + 1)) else {
            continue;
        };
        let span = b.0.saturating_sub(a.0) as f64;
        if span <= 0.0 {
            continue;
        }
        let rise = b.1 - a.1;
        low = low.max((rise - tolerance) / span);
        high = high.min((rise + tolerance) / span);
        let slope = (c.1 - a.1) / (c.0.saturating_sub(a.0) as f64).max(f64::EPSILON);
        if (low..=high).contains(&slope) {
            if let Some(slot) = keep.get_mut(index) {
                *slot = false;
            }
        } else {
            last = index;
            low = f64::NEG_INFINITY;
            high = f64::INFINITY;
        }
    }
    let before = points.len();
    let mut kept = Vec::with_capacity(before);
    for (index, point) in points.iter().enumerate() {
        if keep.get(index).copied().unwrap_or(true) {
            kept.push(*point);
        }
    }
    *points = kept;
    before - points.len()
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

/// The **mutes** of every track cleared.
///
/// The tree's Clear Mutes: every track of the document is unmuted at
/// once. A track that is **mixed** with others is unmuted with them —
/// the mix link is the tree's own, and a change to the mute of one of
/// them is a change to the mute of all of them.
#[derive(Command)]
#[command(
    id = "sound.track.mute.clear",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Mute"),
    mutates,
    undoable
)]
pub struct ClearMutes {}

impl ClearMutes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let tracks: Vec<u64> =
            session.document().walk().filter(|node| node.kind == "sound.track").map(|node| node.id).collect();
        let mut cleared = 0usize;
        for track in tracks {
            let Some(node) = session.document().node(track) else {
                continue;
            };
            let Ok(mut payload) = crate::TrackPayload::from_json(&node.data) else {
                continue;
            };
            if !payload.mute {
                continue;
            }
            payload.mute = false;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                continue;
            };
            node.data = data;
            cleared += 1;
            // The **mix link**: the tracks that are mixed with this one
            // are unmuted with it.
            for sibling in crate::group::mix_siblings(session.document(), track) {
                if sibling == track {
                    continue;
                }
                let Some(node) = session.document().node(sibling) else {
                    continue;
                };
                let Ok(mut other) = crate::TrackPayload::from_json(&node.data) else {
                    continue;
                };
                if !other.mute {
                    continue;
                }
                other.mute = false;
                let data = other.to_json();
                if let Some(node) = session.document_mut()?.node_mut(sibling) {
                    node.data = data;
                    cleared += 1;
                }
            }
        }
        if cleared == 0 {
            return Ok(Outcome::new("no track was muted").with_data(json!({ "cleared": 0 })));
        }
        Ok(Outcome::changed(format!("cleared the mute of {cleared} track(s)"))
            .with_data(json!({ "cleared": cleared })))
    }
}

/// The **solos** of every track cleared.
///
/// The tree's Clear Solos: every track of the document is unsoloed at
/// once, and the document is heard whole again.
///
/// A solo is a rule about the **other** tracks, and so clearing it is
/// about every track and not about one: the tree's own rule. The flag
/// that says a track is heard even when another is soloed — the solo
/// safe — is not a solo and is left as it is.
#[derive(Command)]
#[command(
    id = "sound.track.solo.clear",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Solo"),
    mutates,
    undoable
)]
pub struct ClearSolos {}

impl ClearSolos {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let tracks: Vec<u64> =
            session.document().walk().filter(|node| node.kind == "sound.track").map(|node| node.id).collect();
        let mut cleared = 0usize;
        for track in tracks {
            let Some(node) = session.document().node(track) else {
                continue;
            };
            let Ok(mut payload) = crate::TrackPayload::from_json(&node.data) else {
                continue;
            };
            if !payload.solo {
                continue;
            }
            payload.solo = false;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                continue;
            };
            node.data = data;
            cleared += 1;
        }
        if cleared == 0 {
            return Ok(Outcome::new("no track was soloed").with_data(json!({ "cleared": 0 })));
        }
        Ok(Outcome::changed(format!("cleared the solo of {cleared} track(s)"))
            .with_data(json!({ "cleared": cleared })))
    }
}

/// **Mute** the sends of a track, or let them be heard.
///
/// The tree's Mute Sends: every send of the track is muted at once.
///
/// A send is a **copy** of the track's signal, and a muted send is a
/// copy of nothing: the track is still heard on its own and the bus
/// hears nothing.
#[derive(Command)]
#[command(
    id = "sound.track.sends.mute",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Sends"),
    mutates,
    undoable
)]
pub struct MuteTrackSends {
    /// The track.
    pub track: NodeRef,
    /// Whether the sends are silent.
    pub muted: Bool,
}

impl MuteTrackSends {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.sends.mute".to_string();
        let track = self.track.0;
        let Some(node) = session.document().node(track).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let muted = self.muted.0;
        let sends: Vec<u64> =
            node.children.iter().filter(|child| child.kind == "sound.send").map(|child| child.id).collect();
        let mut changed = 0usize;
        for send in &sends {
            let Some(node) = session.document().node(*send) else { continue };
            let Ok(mut payload) = crate::SendPayload::from_json(&node.data) else { continue };
            if payload.mute == muted {
                continue;
            }
            payload.mute = muted;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(*send) else { continue };
            node.data = data;
            changed += 1;
        }
        if changed == 0 {
            let count = sends.len();
            return Ok(Outcome::new(format!(
                "the track `{track}` has {count} send(s) and none of them moved"
            ))
            .with_data(json!({ "track": track, "muted": muted, "sends": sends.len() })));
        }
        Ok(Outcome::changed(format!(
            "the sends of the track `{track}` are {}",
            if muted { "muted" } else { "heard" }
        ))
        .with_data(json!({ "track": track, "muted": muted, "sends": sends.len(), "changed": changed })))
    }
}
