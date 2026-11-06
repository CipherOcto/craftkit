//! The **crop** and the **trim** of two paths.
//!
//! The tree's pathfinder has more than the booleans: it has the
//! operations that take the **hidden** parts away and the ones that
//! keep what lies inside.
//!
//! - The **crop** keeps the part of the path **behind** that lies
//!   inside the path in **front**, and the path in front is taken
//!   away. What is left is what the front path was covering.
//! - The **trim** takes the hidden parts away: the path in front keeps
//!   all of itself, and the path behind loses the part the front was
//!   covering, so that the two do not overlap.
//!
//! Both are the tree's own operations, and both are made here out of
//! the boolean this tree has: the crop is the **intersection**, and
//! the trim is the **difference** of the path behind from the path in
//! front.
//!
//! The rules:
//!
//! - the path in front and the path behind must both be **paths**;
//! - a crop of two paths that do not meet is **refused**: the result
//!   would be nothing, and nothing is not a path.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, FormatValue, NodeRef, Outcome, Path, Session};

use crate::VectorNode;
use crate::geom;

/// The path in front and the path behind.
///
/// The ones the command was **given**, or the ones that are
/// **selected** — and the selection is in the order of the document,
/// so that the last one is the one in front.
fn front_and_back(
    session: &Session,
    id: &str,
    front: Option<NodeRef>,
    back: Option<NodeRef>,
) -> Result<(u64, u64), EngineError> {
    let need = |what: &str| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is nothing {what}: name two paths or select two"),
    };
    match (front, back) {
        (Some(front), Some(back)) => Ok((front.0, back.0)),
        (Some(front), None) => {
            let back = session
                .selected_whole_ids()
                .into_iter()
                .find(|other| *other != front.0)
                .ok_or_else(|| need("selected"))?;
            Ok((front.0, back))
        }
        (None, Some(back)) => {
            let front = session
                .selected_whole_ids()
                .into_iter()
                .find(|other| *other != back.0)
                .ok_or_else(|| need("selected"))?;
            Ok((front, back.0))
        }
        (None, None) => {
            let mut selected = session.selected_whole_ids();
            if selected.len() < 2 {
                return Err(need("selected"));
            }
            // The order of the document decides: the last is in front.
            let order: Vec<u64> = session.document().walk().map(|node| node.id).collect();
            selected.sort_by_key(|id| order.iter().position(|other| other == id).unwrap_or(0));
            let back = selected[0];
            let front = selected[selected.len() - 1];
            Ok((front, back))
        }
    }
}

/// **Crop** a path with another: the part of the path behind that
/// lies inside the path in front.
///
/// The path in front is taken away, and what it was covering is what
/// is left.
#[derive(Command)]
#[command(
    id = "vector.path.crop",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct CropPaths {
    /// The path in front.
    ///
    /// Leave it out and the **selection** is used instead.
    pub front: Option<NodeRef>,
    /// The path behind.
    ///
    /// Leave it out and the **selection** is used instead.
    pub back: Option<NodeRef>,
}

impl CropPaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.crop".to_string();
        let (front, back) = front_and_back(session, &id, self.front, self.back)?;
        let one = crate::vector_of(session, front)?;
        let two = crate::vector_of(session, back)?;
        if one.path.is_empty() || two.path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "one of the paths is empty".to_string(),
            });
        }
        let path = geom::boolean(&one.path, &two.path, geom::BoolOp::Intersect)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the paths do not meet, and there is nothing to crop".to_string(),
            });
        }
        let mut result = two.clone();
        result.path = path;
        crate::write_back(session, back, &result)?;
        session.document_mut()?.remove_node(front);
        Ok(Outcome::changed("the path in front was cropped away")
            .with_data(json!({ "id": back, "removed": front })))
    }
}

/// **Trim** two paths: the path behind loses the part the path in
/// front was covering.
///
/// The path in front keeps all of itself, and the two do not overlap
/// afterwards.
#[derive(Command)]
#[command(
    id = "vector.path.trim",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct TrimPaths {
    /// The path in front.
    ///
    /// Leave it out and the **selection** is used instead.
    pub front: Option<NodeRef>,
    /// The path behind.
    ///
    /// Leave it out and the **selection** is used instead.
    pub back: Option<NodeRef>,
}

impl TrimPaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.trim".to_string();
        let (front, back) = front_and_back(session, &id, self.front, self.back)?;
        let one = crate::vector_of(session, front)?;
        let two = crate::vector_of(session, back)?;
        if one.path.is_empty() || two.path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "one of the paths is empty".to_string(),
            });
        }
        let path = geom::boolean(&two.path, &one.path, geom::BoolOp::Difference)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        let mut result = two.clone();
        result.path = path;
        crate::write_back(session, back, &result)?;
        Ok(Outcome::changed("the path behind was trimmed").with_data(json!({ "id": back, "front": front })))
    }
}

/// Put a path in the document, with the **style** of another.
fn a_path(session: &mut Session, vector: &VectorNode, path: Path, name: String) -> Result<u64, EngineError> {
    let mut copy = vector.clone();
    copy.path = path;
    let rect = geom::bounds(&copy.placed());
    Ok(session.document_mut()?.add_node_with(name, "vector.path", Some(rect), copy.to_json()))
}

/// **Divide** two paths: every **face** of them.
///
/// The paths are cut at the places where they meet, and what is left
/// is one path for each face: the part of the path in front that the
/// path behind does not cover, the part of the path behind that the
/// path in front does not cover, and the part they share. The face
/// they share takes the **style of the path in front**, because the
/// path in front is what covers it.
#[derive(Command)]
#[command(
    id = "vector.path.divide",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct DividePaths {
    /// The path in front.
    ///
    /// Leave it out and the **selection** is used instead.
    pub front: Option<NodeRef>,
    /// The path behind.
    ///
    /// Leave it out and the **selection** is used instead.
    pub back: Option<NodeRef>,
}

impl DividePaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.divide".to_string();
        let (front, back) = front_and_back(session, &id, self.front, self.back)?;
        let one = crate::vector_of(session, front)?;
        let two = crate::vector_of(session, back)?;
        if one.path.is_empty() || two.path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "one of the paths is empty".to_string(),
            });
        }
        // The **faces**: what the path behind does not cover, what the
        // path in front does not cover, and what they share.
        let back_only = geom::boolean(&two.path, &one.path, geom::BoolOp::Difference)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        let front_only = geom::boolean(&one.path, &two.path, geom::BoolOp::Difference)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        let shared = geom::boolean(&one.path, &two.path, geom::BoolOp::Intersect)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        // The path behind keeps what is its own, and the path in front
        // keeps what is its own.
        let mut back_result = two.clone();
        back_result.path = back_only;
        crate::write_back(session, back, &back_result)?;
        let mut front_result = one.clone();
        front_result.path = front_only;
        crate::write_back(session, front, &front_result)?;
        // The face they share is a path of its own, and it takes the
        // style of the path in front.
        let shared_id = if shared.is_empty() {
            None
        } else {
            Some(a_path(session, &one, shared, "the shared face".to_string())?)
        };
        let count = 2 + usize::from(shared_id.is_some());
        Ok(Outcome::changed(format!("the paths were divided into {count} face(s)"))
            .with_data(json!({ "faces": count, "shared": shared_id })))
    }
}

/// **Merge** two paths: the parts that are the same are joined.
///
/// The tree's merge is the trim and then a join: the parts that are
/// left are joined into one path when they are painted the same way,
/// and they are kept apart when they are not.
#[derive(Command)]
#[command(
    id = "vector.path.merge",
    area = "vector",
    layer = "vector",
    icon = "path",
    menu("Path", "Combine"),
    mutates,
    undoable
)]
pub struct MergePaths {
    /// The path in front.
    ///
    /// Leave it out and the **selection** is used instead.
    pub front: Option<NodeRef>,
    /// The path behind.
    ///
    /// Leave it out and the **selection** is used instead.
    pub back: Option<NodeRef>,
}

impl MergePaths {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.merge".to_string();
        let (front, back) = front_and_back(session, &id, self.front, self.back)?;
        let one = crate::vector_of(session, front)?;
        let two = crate::vector_of(session, back)?;
        if one.path.is_empty() || two.path.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "one of the paths is empty".to_string(),
            });
        }
        if one.fill == two.fill {
            // The **same paint**: the two are one path.
            let path = geom::boolean(&one.path, &two.path, geom::BoolOp::Union)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            let mut result = one.clone();
            result.path = path;
            crate::write_back(session, front, &result)?;
            session.document_mut()?.remove_node(back);
            return Ok(
                Outcome::changed("the paths are one now").with_data(json!({ "id": front, "removed": back }))
            );
        }
        // **Different paint**: the parts are kept apart, and the part
        // of the path behind that the path in front covers is taken
        // away.
        let path = geom::boolean(&two.path, &one.path, geom::BoolOp::Difference)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
        let mut result = two.clone();
        result.path = path;
        crate::write_back(session, back, &result)?;
        Ok(Outcome::changed("the paths were merged and kept apart")
            .with_data(json!({ "id": back, "front": front })))
    }
}
