//! The **crop**, the **trim**, the **divide** and the **merge**.
//!
//! The tree's pathfinder has more than the booleans: it has the
//! operations that take the **hidden** parts away and the ones that
//! keep what lies inside.
//!
//! - The **crop** keeps the part of the paths **behind** that lies
//!   inside the path in **front**, and the path in front is taken
//!   away. What is left is what the front path was covering.
//! - The **trim** takes the hidden parts away: the path in front keeps
//!   all of itself, and the paths behind lose the parts the paths in
//!   front were covering, so that they do not overlap.
//!
//! Both are the tree's own operations, and both are made here out of
//! the boolean this tree has: the crop is the **intersection**, and
//! the trim is the **difference** of a path from what is in front of
//! it.
//!
//! The crop and the trim work on a whole **selection**, as the tree's
//! do — the paths in the order they are in the document, the last in
//! front — and the divide and the merge are of **two** paths, and a
//! selection of more than two is refused for them.
//!
//! The rules:
//!
//! - every path must be a **path**;
//! - a crop whose paths do not meet is **refused**: the result would
//!   be nothing, and nothing is not a path;
//! - a path that has **nothing left** after a crop or a trim is
//!   **gone**: it is not a path that covers nothing;
//! - the divide and the merge are of **two** paths, and a selection of
//!   more than two is **refused** for them.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, FormatValue, NodeRef, Outcome, Path, Session};

use crate::VectorNode;
use crate::geom;

/// The paths a command works on, **in front first**.
///
/// The ones the command was **given**, or the ones that are
/// **selected** — and the selection is in the order of the document,
/// so that the last one is the one in front.
///
/// A path that was named keeps the place it was named for: the path
/// named as the front is in front of all the others, and the path
/// named as the back is behind all of them.
fn the_paths(
    session: &Session,
    id: &str,
    front: Option<NodeRef>,
    back: Option<NodeRef>,
) -> Result<Vec<u64>, EngineError> {
    let need = |what: &str| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is nothing {what}: name two paths or select two"),
    };
    // The selection, in the order of the document: the last is in
    // front.
    let order: Vec<u64> = session.document().walk().map(|node| node.id).collect();
    let mut selected = session.selected_whole_ids();
    selected.sort_by_key(|id| order.iter().position(|other| other == id).unwrap_or(0));
    match (front, back) {
        (Some(front), Some(back)) => Ok(vec![front.0, back.0]),
        (Some(front), None) => {
            let mut paths: Vec<u64> = selected.into_iter().filter(|other| *other != front.0).collect();
            if paths.is_empty() {
                return Err(need("selected"));
            }
            paths.insert(0, front.0);
            Ok(paths)
        }
        (None, Some(back)) => {
            let mut paths: Vec<u64> = selected.into_iter().filter(|other| *other != back.0).collect();
            if paths.is_empty() {
                return Err(need("selected"));
            }
            paths.push(back.0);
            Ok(paths)
        }
        (None, None) => {
            if selected.len() < 2 {
                return Err(need("selected"));
            }
            selected.reverse();
            Ok(selected)
        }
    }
}

/// The path in front and the path behind.
///
/// The two the command was **given**, or the two of the selection —
/// and a selection of more than two is refused, because the divide
/// and the merge are of two paths.
fn front_and_back(
    session: &Session,
    id: &str,
    front: Option<NodeRef>,
    back: Option<NodeRef>,
) -> Result<(u64, u64), EngineError> {
    let paths = the_paths(session, id, front, back)?;
    if paths.len() != 2 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("this works on two paths, and there are {}", paths.len()),
        });
    }
    Ok((paths[0], paths[1]))
}

commands! {
    /// **Crop** a path with another: the part of the path behind that
    /// lies inside the path in front.
    ///
    /// The path in front is taken away, and what it was covering is what
    /// is left.
    vector.path.crop {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Combine"),
        params: {
            /// The path in front.
            ///
            /// Leave it out and the **selection** is used instead.
            front: Option<NodeRef>,
            /// The path behind.
            ///
            /// Leave it out and the **selection** is used instead.
            back: Option<NodeRef>,
        },
        run: {
            let id = "vector.path.crop".to_string();
            let paths = the_paths(session, &id, self.front, self.back)?;
            let front = paths[0];
            let one = crate::vector_of(session, front)?;
            if one.path.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the path in front is empty".to_string(),
                });
            }
            // The part of each path behind that lies inside the path in
            // front. A path with nothing inside it has nothing left: the
            // crop keeps what lies inside, and there is none of it.
            let mut kept: Vec<(u64, Path)> = Vec::new();
            let mut gone: Vec<u64> = Vec::new();
            for path in &paths[1..] {
                let two = crate::vector_of(session, *path)?;
                if two.path.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the path `{path}` is empty"),
                    });
                }
                let shared = geom::boolean(&one.path, &two.path, geom::BoolOp::Intersect)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
                if shared.is_empty() {
                    gone.push(*path);
                    continue;
                }
                kept.push((*path, shared));
            }
            if kept.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the paths do not meet, and there is nothing to crop".to_string(),
                });
            }
            // The path in front is taken away, and what it was covering
            // is what is left.
            for (path, shared) in kept {
                let mut result = crate::vector_of(session, path)?;
                result.path = shared;
                crate::write_back(session, path, &result)?;
            }
            // A path behind that did not meet the path in front has
            // nothing left of the crop: it is gone.
            for path in gone {
                session.document_mut()?.remove_node(path);
            }
            session.document_mut()?.remove_node(front);
            let count = paths.len() - 1;
            Ok(Outcome::changed(format!("the path in front was cropped away from {count} path(s)"))
                .with_data(json!({ "id": front, "cropped": count })))

        }
    }

    /// **Trim** two paths: the path behind loses the part the path in
    /// front was covering.
    ///
    /// The path in front keeps all of itself, and the two do not overlap
    /// afterwards.
    vector.path.trim {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Combine"),
        params: {
            /// The path in front.
            ///
            /// Leave it out and the **selection** is used instead.
            front: Option<NodeRef>,
            /// The path behind.
            ///
            /// Leave it out and the **selection** is used instead.
            back: Option<NodeRef>,
        },
        run: {
            let id = "vector.path.trim".to_string();
            let paths = the_paths(session, &id, self.front, self.back)?;
            // The paths in front of each path, one at a time: the hidden
            // part is taken away path by path, because a path in front is
            // a path and not a shape of many.
            let mut trimmed: Vec<(u64, Path)> = Vec::new();
            // From the back to the front, so that what is in front of a
            // path is known when the path is reached.
            for index in (0..paths.len()).rev() {
                let path = paths[index];
                let vector = crate::vector_of(session, path)?;
                if vector.path.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the path `{path}` is empty"),
                    });
                }
                let mut kept = vector.path.clone();
                for other in &paths[..index] {
                    let in_front = crate::vector_of(session, *other)?;
                    if in_front.path.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the path `{other}` is empty"),
                        });
                    }
                    kept = geom::boolean(&kept, &in_front.path, geom::BoolOp::Difference)
                        .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
                    if kept.is_empty() {
                        break;
                    }
                }
                trimmed.push((path, kept));
            }
            // The paths that have something left are kept, and the ones
            // that have nothing left are gone: a path that is hidden
            // altogether is not a path.
            let mut kept = 0usize;
            for (path, path_of) in trimmed {
                if path_of.is_empty() {
                    session.document_mut()?.remove_node(path);
                    continue;
                }
                let mut result = crate::vector_of(session, path)?;
                result.path = path_of;
                crate::write_back(session, path, &result)?;
                kept += 1;
            }
            Ok(Outcome::changed(format!("{kept} path(s) were trimmed")).with_data(json!({ "trimmed": kept })))

        }
    }

    /// **Divide** two paths: every **face** of them.
    ///
    /// The paths are cut at the places where they meet, and what is left
    /// is one path for each face: the part of the path in front that the
    /// path behind does not cover, the part of the path behind that the
    /// path in front does not cover, and the part they share. The face
    /// they share takes the **style of the path in front**, because the
    /// path in front is what covers it.
    vector.path.divide {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Combine"),
        params: {
            /// The path in front.
            ///
            /// Leave it out and the **selection** is used instead.
            front: Option<NodeRef>,
            /// The path behind.
            ///
            /// Leave it out and the **selection** is used instead.
            back: Option<NodeRef>,
        },
        run: {
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
    vector.path.merge {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Combine"),
        params: {
            /// The path in front.
            ///
            /// Leave it out and the **selection** is used instead.
            front: Option<NodeRef>,
            /// The path behind.
            ///
            /// Leave it out and the **selection** is used instead.
            back: Option<NodeRef>,
        },
        run: {
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
}

/// Put a path in the document, with the **style** of another.
fn a_path(session: &mut Session, vector: &VectorNode, path: Path, name: String) -> Result<u64, EngineError> {
    let mut copy = vector.clone();
    copy.path = path;
    let rect = geom::bounds(&copy.placed());
    Ok(session.document_mut()?.add_node_with(name, "vector.path", Some(rect), copy.to_json()))
}
