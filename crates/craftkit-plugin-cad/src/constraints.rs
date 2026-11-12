//! The **constraints** of a drawing.
//!
//! A constraint is a **fact** of a drawing: it says that two entities
//! of the drawing stand in some relation to one another — that they
//! are parallel, that one of them is fixed, that they are the same
//! length. The constraint is recorded, and **whether it holds is
//! derived**: it is worked out from the geometry of the entities.
//!
//! The tree's constraint is a solver: it moves the geometry until the
//! constraint holds, and it moves it again after every edit. Here the
//! constraint is a fact of the drawing and the geometry is not moved —
//! and the answer says whether the constraint holds as it stands.
//!
//! # The rules
//!
//! - **A constraint is between two entities.** Every kind here needs
//!   two, and a constraint that names one entity or three is refused.
//! - **The kinds are named.** `coincident`, `parallel`,
//!   `perpendicular`, `horizontal`, `vertical`, `tangent`,
//!   `concentric` and `equal` — and a kind that is not one of those is
//!   refused by name.
//! - **A constraint that cannot be worked out is not held.** A
//!   constraint between two things whose geometry this tree cannot
//!   read is *unknown*, and unknown is not the same as false.
//!
//! # What it does not do
//!
//! - **The geometry is not moved.** The tree's solver moves it, and
//!   this tree does not: the constraint says what the drawing means,
//!   and the drawing is as it is.
//! - **The degrees of freedom are not counted.** The tree counts them
//!   and shows them in the constraint bar.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Relation, Session, Text};

/// The kinds of **constraint** this tree reads.
const KINDS: [&str; 8] =
    ["coincident", "parallel", "perpendicular", "horizontal", "vertical", "tangent", "concentric", "equal"];

/// The **relation** a constraint is: from one entity to another.
const CONSTRAINS: &str = "cad.constrains";

/// The **point** of a geometry, if it has one by that name.
fn point_in(geometry: &Json, key: &str) -> Option<(f64, f64)> {
    let value = geometry.get(key)?;
    let x = value.get("x")?.as_f64()?;
    let y = value.get("y")?.as_f64()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// The **ends** of a geometry: the points it is drawn between.
fn ends_of(geometry: &Json) -> Option<((f64, f64), (f64, f64))> {
    let from = point_in(geometry, "from")?;
    let to = point_in(geometry, "to")?;
    Some((from, to))
}

/// The **direction** of a geometry: the vector it runs along.
fn direction_of(geometry: &Json) -> Option<(f64, f64)> {
    let (from, to) = ends_of(geometry)?;
    Some((to.0 - from.0, to.1 - from.1))
}

/// The **radius** of a geometry, if it has one.
fn radius_of(geometry: &Json) -> Option<f64> {
    let radius = geometry.get("radius")?.as_f64()?;
    if !radius.is_finite() {
        return None;
    }
    Some(radius)
}

/// The **centre** of a geometry, if it has one.
fn centre_of(geometry: &Json) -> Option<(f64, f64)> {
    point_in(geometry, "center")
}

/// The **length** of a geometry, if it has one.
fn length_of(geometry: &Json) -> Option<f64> {
    let (from, to) = ends_of(geometry)?;
    Some(((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt())
}

/// Whether two numbers are **the same**, to the drawing's own measure.
///
/// The drawing is in millimetres, and a hundredth of a millimetre is
/// as close as a drawing needs.
fn same(one: f64, other: f64) -> bool {
    (one - other).abs() < 0.000_001
}

/// Whether a **constraint** holds between two geometries.
///
/// The answer is *unknown* when the geometry cannot be read, and that
/// is not the same as false.
fn holds(kind: &str, one: &Json, other: &Json) -> Option<bool> {
    match kind {
        "horizontal" => {
            let (from, to) = ends_of(one)?;
            Some(same(from.1, to.1))
        }
        "vertical" => {
            let (from, to) = ends_of(one)?;
            Some(same(from.0, to.0))
        }
        "parallel" => {
            let first = direction_of(one)?;
            let second = direction_of(other)?;
            // Two lines are parallel when their directions are: the
            // cross product of the two is nothing.
            let cross = first.0 * second.1 - first.1 * second.0;
            let lengths = (first.0 * first.0 + first.1 * first.1).sqrt()
                * (second.0 * second.0 + second.1 * second.1).sqrt();
            if lengths == 0.0 {
                return None;
            }
            Some(cross.abs() / lengths < 0.000_001)
        }
        "perpendicular" => {
            let first = direction_of(one)?;
            let second = direction_of(other)?;
            let dot = first.0 * second.0 + first.1 * second.1;
            let lengths = (first.0 * first.0 + first.1 * first.1).sqrt()
                * (second.0 * second.0 + second.1 * second.1).sqrt();
            if lengths == 0.0 {
                return None;
            }
            Some(dot.abs() / lengths < 0.000_001)
        }
        "coincident" => {
            let first = ends_of(one)?;
            let second = ends_of(other)?;
            Some(
                (same(first.0.0, second.0.0) && same(first.0.1, second.0.1))
                    || (same(first.0.0, second.1.0) && same(first.0.1, second.1.1))
                    || (same(first.1.0, second.0.0) && same(first.1.1, second.0.1))
                    || (same(first.1.0, second.1.0) && same(first.1.1, second.1.1)),
            )
        }
        "concentric" => {
            let first = centre_of(one)?;
            let second = centre_of(other)?;
            Some(same(first.0, second.0) && same(first.1, second.1))
        }
        "equal" => {
            if let (Some(first), Some(second)) = (length_of(one), length_of(other)) {
                return Some(same(first, second));
            }
            let first = radius_of(one)?;
            let second = radius_of(other)?;
            Some(same(first, second))
        }
        "tangent" => {
            let first = centre_of(one)?;
            let second = centre_of(other)?;
            let first_radius = radius_of(one)?;
            let second_radius = radius_of(other)?;
            let distance = ((first.0 - second.0).powi(2) + (first.1 - second.1).powi(2)).sqrt();
            Some(
                same(distance, first_radius + second_radius)
                    || same(distance, (first_radius - second_radius).abs()),
            )
        }
        _ => None,
    }
}

/// The **geometry** of an entity, if it has any.
fn geometry_of(document: &Document, entity: u64) -> Option<Json> {
    let node = document.node(entity)?;
    node.data.get("geometry").cloned()
}

/// The **entities** under a node, and the nodes under it.
fn gather_entities(node: &Node, out: &mut Vec<u64>) {
    for child in node.children.iter() {
        if child.kind.starts_with("cad.entity.") {
            out.push(child.id);
        }
        gather_entities(child, out);
    }
}

/// **Add** a **constraint** to a drawing.
///
/// A constraint is a fact of the drawing: it says that two entities
/// stand in some relation to one another. Whether it holds is worked
/// out from the geometry, and the answer says.
#[derive(Command)]
#[command(id = "cad.constraint.add", area = "cad", layer = "cad", icon = "link", mutates, undoable)]
pub struct AddConstraint {
    /// The first entity.
    pub first: NodeRef,
    /// The second entity.
    pub second: NodeRef,
    /// The kind of constraint: `coincident`, `parallel`,
    /// `perpendicular`, `horizontal`, `vertical`, `tangent`,
    /// `concentric` or `equal`.
    pub kind: Text,
}

impl AddConstraint {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.constraint.add".to_string();
        let first = self.first.0;
        let second = self.second.0;
        let kind = self.kind.0.trim().to_lowercase();
        if !KINDS.contains(&kind.as_str()) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a constraint is one of {}: `{kind}` is not one of them", KINDS.join(", ")),
            });
        }
        for entity in [first, second] {
            let Some(node) = session.document().node(entity) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no entity `{entity}`"),
                });
            };
            if !node.kind.starts_with("cad.entity.") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{entity}` is a `{}` and not an entity", node.kind),
                });
            }
        }
        if first == second {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a constraint is between two entities, and these are one".to_string(),
            });
        }
        // The **constraint** is a relation from one entity to the
        // other, and it says what kind it is.
        session.document_mut()?.add_relation(Relation::with_payload(
            first,
            second,
            CONSTRAINS,
            json!({ "kind": kind }),
        ));
        // Whether it **holds** is derived from the geometry.
        let one = geometry_of(session.document(), first);
        let other = geometry_of(session.document(), second);
        let held = match (&one, &other) {
            (Some(one), Some(other)) => holds(&kind, one, other),
            _ => None,
        };
        Ok(Outcome::changed(format!("the entity `{first}` is constrained to `{second}`")).with_data(json!({
            "first": first,
            "second": second,
            "kind": kind,
            "holds": held,
        })))
    }
}

/// **Read** the **constraints** of a drawing.
///
/// Whether each constraint holds is **derived**: it is worked out from
/// the geometry of the entities, and a constraint whose geometry
/// cannot be read is *unknown*.
#[derive(Command)]
#[command(id = "cad.constraint.list", area = "cad", layer = "cad", icon = "list")]
pub struct ListConstraints {
    /// The drawing.
    pub drawing: NodeRef,
}

impl ListConstraints {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.constraint.list".to_string();
        let drawing = self.drawing.0;
        let Some(node) = session.document().node(drawing) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no drawing `{drawing}`"),
            });
        };
        if node.kind != "cad.drawing" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{drawing}` is a `{}` and not a drawing", node.kind),
            });
        }
        // The **entities** of the drawing: the constraints are the
        // relations they hold.
        let mut entities: Vec<u64> = Vec::new();
        gather_entities(node, &mut entities);
        let mut listed: Vec<Json> = Vec::new();
        let mut relations: Vec<craftkit_sdk::Relation> = Vec::new();
        for entity in entities {
            for relation in session.document().relations_from(entity, Some(CONSTRAINS)) {
                relations.push(relation.clone());
            }
        }
        for relation in relations {
            let kind =
                relation.payload.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default().to_string();
            let one = geometry_of(session.document(), relation.from);
            let other = geometry_of(session.document(), relation.to);
            let held = match (&one, &other) {
                (Some(one), Some(other)) => holds(&kind, one, other),
                _ => None,
            };
            listed.push(json!({
                "first": relation.from,
                "second": relation.to,
                "kind": kind,
                "holds": held,
            }));
        }
        let count = listed.len();
        Ok(Outcome::new(format!("the drawing `{drawing}` holds {count} constraint(s)")).with_data(json!({
            "drawing": drawing,
            "constraints": listed,
            "count": count,
        })))
    }
}
