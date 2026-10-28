//! The output of evaluation: a plan at one instant.
//!
//! A [`Plan`] is a *derived* value. Nothing writes it back to the document, it
//! holds no references into it, and it is cheap to throw away. That is what lets
//! the editor re-evaluate on every pointer move.

use craftkit_core::{NodeId, Tick, Value};
use craftkit_model::NodeKind;
use serde::{Deserialize, Serialize};

/// One property of a node, resolved at an instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resolved {
    /// The path that addresses this property, for a message that quotes it.
    pub path: String,
    /// The stable property id.
    pub id: u64,
    /// The property's value.
    ///
    /// **Published in the spelling the tree's commands take.** §269: the plan
    /// published the value the way the *file* spells it — a tagged object —
    /// and `prop.set`, the command that writes the very property the plan
    /// names, refused it: "a object cannot be a property value".
    #[serde(serialize_with = "value_as_parameter", deserialize_with = "value_from_parameter")]
    pub value: Value,
    /// Whether the value came from the keyframe curve rather than the stored
    /// default. The UI marks these so an animated value is never mistaken for
    /// a constant one.
    pub animated: bool,
}

/// The value as the tree's commands take it.
///
/// **The core's spelling.** §269.
fn value_as_parameter<S: serde::Serializer>(value: &Value, serializer: S) -> Result<S::Ok, S::Error> {
    value.as_parameter().serialize(serializer)
}

/// The value as the tree's commands take it, read back.
///
/// **The core's reader.** §269.
fn value_from_parameter<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    let json = serde_json::Value::deserialize(deserializer)?;
    Value::from_parameter("value", &json).map_err(serde::de::Error::custom)
}

/// What one node contributed to the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub muted: bool,
    /// The level entering this node.
    pub level_in: f64,
    /// The level leaving it. Equal to `level_in` for a sink, a cue, a comment,
    /// or a muted node.
    pub level_out: f64,
    /// Every property, resolved at the instant. Sorted by path so two plans for
    /// the same instant compare equal regardless of tree order.
    pub props: Vec<Resolved>,
}

impl NodeState {
    /// The value of one property, by path.
    pub fn prop(&self, path: &str) -> Option<&Value> {
        self.props.iter().find(|r| r.path == path).map(|r| &r.value)
    }
}

/// How bad a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The plan is still usable; something is probably a mistake.
    Warning,
    /// The plan is usable but this node did not do what it says.
    Error,
}

/// One thing worth telling the user about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The node it concerns, when it concerns one.
    pub node: Option<NodeId>,
    /// The node's name, carried here so a message survives the node being
    /// deleted between evaluation and display.
    pub node_name: String,
    pub message: String,
}

impl Diagnostic {
    pub fn warn(node: Option<NodeId>, name: &str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Warning, node, node_name: name.to_string(), message: message.into() }
    }

    pub fn error(node: Option<NodeId>, name: &str, message: impl Into<String>) -> Self {
        Self { severity: Severity::Error, node, node_name: name.to_string(), message: message.into() }
    }

    /// The line a person reads in the problems panel.
    pub fn render(&self) -> String {
        let who = if self.node_name.is_empty() {
            "plan".to_string()
        } else {
            format!("{} `{}`", self.node_name, self.id_text())
        };
        let tag = match self.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        format!("{tag}: {who}: {}", self.message)
    }

    fn id_text(&self) -> String {
        self.node.map(|n| n.get().to_string()).unwrap_or_else(|| "?".to_string())
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

/// Everything evaluation wants to tell the caller about.
///
/// Diagnostics are **collected, not returned**. One broken node must not hide
/// the eleven behind it, so evaluation continues and reports. The only thing
/// that stops an evaluation is a document that is too large to evaluate at all.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
    /// How many diagnostics were dropped because the list was full.
    pub dropped: usize,
}

impl Diagnostics {
    pub fn push(&mut self, d: Diagnostic) {
        self.push_capped(d, crate::MAX_DIAGNOSTICS);
    }

    /// Push, refusing past `cap`. The cap exists because a document with a
    /// broken property on every node would otherwise produce a megabyte of
    /// messages per frame.
    pub fn push_capped(&mut self, d: Diagnostic, cap: usize) {
        if self.items.len() < cap {
            self.items.push(d);
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.items.iter().filter(|d| d.severity == Severity::Error).count()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.dropped == 0
    }
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for d in &self.items {
            writeln!(f, "{d}")?;
        }
        if self.dropped > 0 {
            write!(f, "{} further problem(s) not listed", self.dropped)?;
        }
        Ok(())
    }
}

/// The evaluated document at one instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub at: Tick,
    pub nodes: Vec<NodeState>,
    pub diagnostics: Diagnostics,
    /// How many nodes were taken from the previous evaluation unchanged.
    pub reused: usize,
    /// How many were evaluated afresh.
    pub evaluated: usize,
}

impl Plan {
    /// The state of one node, by id.
    ///
    /// **This is a scan, not a lookup.** It walks the plan's nodes until it
    /// finds the one asked for, so a caller that asks about *every* node pays
    /// a scan per node — quadratic.
    ///
    /// The node list did exactly that, and paid 43.7 ms a frame on a document
    /// at the model's own limit of ten thousand nodes; it now collects the
    /// levels once instead. §127.
    ///
    /// So: ask about the node you need, or collect what you need in one walk
    /// and look it up in what you collected.
    pub fn state(&self, id: NodeId) -> Option<&NodeState> {
        self.nodes.iter().find(|s| s.id == id)
    }

    /// The level leaving the last node that produced one.
    pub fn output(&self) -> f64 {
        self.nodes.last().map(|s| s.level_out).unwrap_or(0.0)
    }

    /// A one-line summary for a status bar.
    pub fn summary(&self) -> String {
        format!(
            "{} at {:.3}s — {} reused, {} evaluated, {} problem(s)",
            self.at,
            self.at.seconds_f64(),
            self.reused,
            self.evaluated,
            self.diagnostics.items.len() + self.diagnostics.dropped
        )
    }
}
