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
}

impl Window {
    /// A window over the kit.
    ///
    /// The ecosystem is the application's own — every battery that was
    /// linked in — and the session is built from it.
    pub fn new() -> Result<Window, EcosystemError> {
        let ecosystem = craftkit_studio::ecosystem()?;
        Ok(Window { session: ecosystem.session(), form: None, log: Vec::new() })
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
        match self.session.execute(&id, &params) {
            Ok(outcome) => {
                self.log.push(outcome.message.clone());
                self.form = None;
                Ok(outcome)
            }
            Err(error) => {
                self.log.push(format!("{id} refused: {error}"));
                Err(error)
            }
        }
    }

    /// What the window has to say.
    pub fn log(&self) -> &[String] {
        &self.log
    }
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
