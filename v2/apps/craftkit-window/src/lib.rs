//! `craftkit-window` — a **window** over the kit.
//!
//! # What it is
//!
//! Three things, and nothing else:
//!
//! 1. **the registry** — every command that was linked in, by area,
//!    with the parameters it wants;
//! 2. **the document** — the tree of nodes, as it is now;
//! 3. **the plan** — what the document means, as the kinds say it.
//!
//! # What it is not
//!
//! It is **thin**, and the rule is the tree's rule: a front end reads
//! the engine's state and acts through the registry. So the window
//!
//! - never writes the document: everything goes through
//!   [`Session::execute`];
//! - never evaluates the document itself: it asks the session for the
//!   plan;
//! - keeps no value the plan computes: the plan is read when it is
//!   drawn.
//!
//! The gate in `craftkit-gates` checks the first two, because a front
//! end that grows a second path is a front end nobody can reason
//! about.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{CommandSpec, Doc, EcosystemError, EngineError, Outcome, Session};

/// The window's own state.
///
/// The session is the whole of it: the document, the history, the
/// registry and the plan are all the session's, and the window holds
/// nothing else but the form being filled in and what it has to say.
pub struct Window {
    /// The session: the document, the history and the registry.
    session: Session,
    /// The command being filled in, if one is.
    form: Option<Form>,
    /// What the window has to say.
    log: Vec<String>,
    /// How many commands the window has run.
    ///
    /// It is a **counter**, and it is what the drawing knows: the
    /// picture is drawn again when the document or the camera has
    /// changed, and the counter is how the window knows that a command
    /// has run.
    runs: u64,
}

impl Window {
    /// A window over the kit.
    ///
    /// The ecosystem is the application's own — every battery that was
    /// linked in — and the session is built from it.
    pub fn new() -> Result<Window, EcosystemError> {
        let ecosystem = craftkit_studio::ecosystem()?;
        Ok(Window { session: ecosystem.session(), form: None, log: Vec::new(), runs: 0 })
    }

    /// The session the window draws.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Every **command** in the registry, in the order they are named.
    pub fn commands(&self) -> Vec<&CommandSpec> {
        let mut found: Vec<&CommandSpec> = self.session.registry().all().iter().collect();
        found.sort_by(|left, right| left.id.cmp(right.id));
        found
    }

    /// The **areas** the commands belong to, in order.
    pub fn areas(&self) -> Vec<&str> {
        let mut areas: Vec<&str> = self.commands().iter().map(|command| command.area).collect();
        areas.sort_unstable();
        areas.dedup();
        areas
    }

    /// The **document**, as a list of rows: one a node.
    pub fn tree(&self) -> Vec<NodeRow> {
        self.session
            .document()
            .walk()
            .map(|node| NodeRow { id: node.id, name: node.name.clone(), kind: node.kind.clone() })
            .collect()
    }

    /// The **plan**: what the document means, as the kinds say it.
    ///
    /// It is read from the session and not worked out here: a front end
    /// that evaluated the document itself would have two answers to
    /// every question.
    pub fn plan(&self) -> Vec<PlanRow> {
        self.session
            .evaluate()
            .items
            .iter()
            .map(|item| PlanRow { node: item.node, kind: item.kind.clone(), value: item.value.clone() })
            .collect()
    }

    /// The **form** being filled in, if one is.
    pub fn form(&self) -> Option<&Form> {
        self.form.as_ref()
    }

    /// **Open** a command: the form of what it wants.
    ///
    /// A command that is not in the registry opens nothing.
    pub fn open(&mut self, id: &str) -> bool {
        let Some(spec) = self.session.registry().get(id) else { return false };
        let fields = spec
            .params
            .iter()
            .map(|param| Field {
                name: param.name.to_string(),
                kind: param.kind,
                required: param.required,
                help: param.help.to_string(),
                value: zero_of(param.kind),
                // An optional parameter that nobody has filled in is
                // **not sent**: the zero of a node reference is zero,
                // and zero is a node that is not there.
                set: param.required,
            })
            .collect();
        self.form = Some(Form { id: spec.id.to_string(), label: spec.label.to_string(), fields });
        true
    }

    /// **Set** one of the form's fields.
    ///
    /// A field that is not there is refused: the form is the command's
    /// own, and the command's parameters are what it is.
    pub fn set(&mut self, name: &str, value: Json) -> bool {
        let Some(form) = self.form.as_mut() else { return false };
        let Some(field) = form.fields.iter_mut().find(|field| field.name == name) else {
            return false;
        };
        field.value = value;
        field.set = true;
        true
    }

    /// **Run** the command the form is for.
    ///
    /// Everything goes through the session's one way in, and what the
    /// command said is what the window says.
    pub fn run(&mut self) -> Result<Outcome, EngineError> {
        let Some(form) = self.form.as_ref() else {
            self.log.push("there is no command to run".to_string());
            return Err(EngineError::Failed {
                id: "window".to_string(),
                message: "there is no command to run".to_string(),
            });
        };
        let id = form.id.clone();
        let params = form.params();
        let outcome = self.run_id(&id, params)?;
        self.form = None;
        self.runs += 1;
        Ok(outcome)
    }

    /// **Run** a command by its id, with the parameters given.
    ///
    /// It is the same way in as the form's: the session's own, and
    /// nothing else.
    pub fn run_id(&mut self, id: &str, params: Json) -> Result<Outcome, EngineError> {
        match self.session.execute(id, &params) {
            Ok(outcome) => {
                self.log.push(outcome.message.clone());
                Ok(outcome)
            }
            Err(error) => {
                self.log.push(format!("{id} refused: {error}"));
                Err(error)
            }
        }
    }

    /// The node **under** a point of the world, if there is one.
    ///
    /// Every node is offered to the kind that says what it is, and the
    /// **deepest** hit wins: a thing in a group is in front of the group
    /// that holds it.
    pub fn pick(&self, point: craftkit_sdk::Point) -> Option<u64> {
        self.session
            .document()
            .walk()
            .filter(|node| {
                self.session.kinds().get(&node.kind).map(|kind| (kind.hit)(node, point)).unwrap_or(false)
            })
            .last()
            .map(|node| node.id)
    }

    /// **Select** a node, through the registry.
    ///
    /// The selection is the session's state, and the command that sets
    /// it is in the registry like every other: the window does not have
    /// a second way in.
    pub fn select_node(&mut self, node: u64) -> Result<Outcome, EngineError> {
        self.run_id("select.node", json!({ "node": node }))
    }

    /// **Move** a node, through the registry.
    ///
    /// The node keeps its size: a move is a move.
    pub fn move_node(&mut self, node: u64, x: f64, y: f64) -> Result<Outcome, EngineError> {
        self.run_id("node.move", json!({ "node": node, "x": x, "y": y }))
    }

    /// **Resize** a node, through the registry.
    ///
    /// The node keeps its place: a resize changes the size and not
    /// where the node is.
    pub fn resize_node(&mut self, node: u64, w: f64, h: f64) -> Result<Outcome, EngineError> {
        self.run_id("node.resize", json!({ "node": node, "w": w, "h": h }))
    }

    /// The node whose **resize handle** is under a point, if there is
    /// one.
    ///
    /// The handle is the corner opposite the anchor — the kit's own
    /// [`craftkit_sdk::Rect::resize_handle`] — and the tolerance is in
    /// the **world's** units: the caller says how near is near, in the
    /// units it is working in.
    ///
    /// The **deepest** handle wins, as it does for a pick: a thing in a
    /// group is in front of the group that holds it.
    pub fn handle_at(&self, point: craftkit_sdk::Point, tolerance: f64) -> Option<u64> {
        let tolerance = tolerance.max(0.0);
        self.session
            .document()
            .walk()
            .filter(|node| {
                node.rect
                    .map(|rect| {
                        let handle = rect.resize_handle();
                        (handle.x - point.x).abs() <= tolerance && (handle.y - point.y).abs() <= tolerance
                    })
                    .unwrap_or(false)
            })
            .last()
            .map(|node| node.id)
    }

    /// **Zoom** the camera.
    ///
    /// The camera is the session's state, and it has its own caps: a
    /// zoom outside them is clamped, because the wheel can send
    /// anything.
    pub fn zoom_by(&mut self, factor: f64) {
        self.session.camera_mut().zoom_by(factor);
    }

    /// **Move** the camera, in the world's units.
    pub fn pan(&mut self, across: f64, down: f64) {
        self.session.camera_mut().pan(across, down);
    }

    /// **Zoom about a point of the screen.**
    ///
    /// The point is measured from the **middle** of the picture, the
    /// way the camera measures it, so a front end that has a pointer
    /// can say where it is without working out the world's mapping a
    /// second time.
    pub fn zoom_at(&mut self, factor: f64, at: craftkit_sdk::Point) {
        self.session.camera_mut().zoom_at(factor, at);
    }

    /// What the window has to say.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// How many commands the window has run.
    pub fn runs(&self) -> u64 {
        self.runs
    }

    /// A **picture** of the document, as pixels.
    ///
    /// The document is drawn **through the kinds**: every node is
    /// offered to the kind that says what it is, and a node whose kind
    /// is not linked in is not drawn — the core does not know which
    /// kinds those are.
    ///
    /// The camera is the **session's**, so the picture is seen through
    /// the same controls every other front end uses.
    pub fn picture(&self, width: usize, height: usize) -> Result<Picture, String> {
        let camera = *self.session.camera();
        let mut raster = craftkit_raster::Raster::new(width, height, camera)?;
        let document = self.session.document();
        for node in document.walk() {
            let Some(kind) = self.session.kinds().get(&node.kind) else { continue };
            (kind.paint)(node, document, &mut raster);
        }
        let image = raster.image();
        let mut pixels = Vec::with_capacity(image.width().saturating_mul(image.height()).saturating_mul(4));
        for y in 0..image.height() {
            for x in 0..image.width() {
                match image.pixel(x, y) {
                    Some(pixel) => pixels.extend_from_slice(&pixel),
                    None => pixels.extend_from_slice(&[0, 0, 0, 0]),
                }
            }
        }
        Ok(Picture { width: image.width(), height: image.height(), pixels })
    }
}

/// A **picture** of the document: pixels, and nothing else.
///
/// It is the window's own shape, and not the rasteriser's: the window
/// shows a picture and does not know what drew it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// How wide it is.
    pub width: usize,
    /// How tall it is.
    pub height: usize,
    /// The pixels: red, green, blue and alpha, four bytes each.
    pub pixels: Vec<u8>,
}

/// One **node** of the document, as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRow {
    /// Its id.
    pub id: u64,
    /// Its name.
    pub name: String,
    /// Its kind.
    pub kind: String,
}

/// One **item** of the plan, as the window shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanRow {
    /// The node it is about.
    pub node: u64,
    /// Its kind.
    pub kind: String,
    /// What it means.
    pub value: Json,
}

/// The **command** being filled in.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// The command's id.
    pub id: String,
    /// What the button says.
    pub label: String,
    /// What it wants.
    pub fields: Vec<Field>,
}

impl Form {
    /// The parameters, as the command wants them.
    pub fn params(&self) -> Json {
        let mut params = craftkit_sdk::serde_json::Map::new();
        for field in &self.fields {
            // An optional parameter that was never filled in is left
            // out: the command's own default is the right answer, and
            // the zero of a node reference is not.
            if !field.required && !field.set {
                continue;
            }
            params.insert(field.name.clone(), field.value.clone());
        }
        Json::Object(params)
    }
}

/// One **field** of a form.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The parameter's name.
    pub name: String,
    /// What kind of value it takes.
    pub kind: Doc,
    /// Whether the caller has to give it.
    pub required: bool,
    /// One line about it.
    pub help: String,
    /// What it is set to.
    pub value: Json,
    /// Whether it has been filled in.
    ///
    /// An optional parameter starts as **not** filled in, and it is
    /// left out of the command's parameters until it is.
    pub set: bool,
}

/// The value a field of that kind starts at.
///
/// It is the **zero** of the kind and not a guess: a text is empty, a
/// number is zero, a flag is off, and a node reference is nothing —
/// which the command will refuse, because nothing is not a node.
fn zero_of(kind: Doc) -> Json {
    match kind {
        Doc::Number => json!(0.0),
        Doc::Text => json!(""),
        Doc::Bool => json!(false),
        Doc::NodeRef => json!(0),
        Doc::NodeKind => json!(""),
        Doc::Json => Json::Null,
    }
}
