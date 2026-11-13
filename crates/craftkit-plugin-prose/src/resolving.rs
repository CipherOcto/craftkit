//! The **resolution** of a revision: accepting it, or rejecting it.
//!
//! A revision is a **mark on a run**: the run was inserted, it was
//! deleted, or its format was changed. Resolving the revision is the
//! decision about the mark:
//!
//! - **accept** it, and the change becomes part of the text: an
//!   insertion is ordinary text, a deletion is really deleted, and a
//!   format change is kept;
//! - **reject** it, and the change never happened: an insertion is
//!   taken away, a deletion is restored, and a format change is
//!   undone.
//!
//! # What the port found
//!
//! - **The tree resolves a revision by editing the text.** An accepted
//!   deletion deletes the words; a rejected insertion deletes them
//!   instead. Here the same, and the block's **text and runs are
//!   rewritten together**: the runs describe slices of the text, and a
//!   text and a set of runs that disagree are a block that says two
//!   things at once.
//! - **The tree keeps the format a run had before the change**, so it
//!   can put it back. Here the old format is **not kept**, and so a
//!   rejected format change is **refused**: there is no old format to
//!   put back, and a resolution that guesses is a resolution that is
//!   wrong.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// The kind of a revision, if the run carries one.
fn revision_of(run: &Json) -> Option<&str> {
    run.get("revision")?.get("kind")?.as_str()
}

/// The **text** of a block.
fn text_of(node: &Node) -> String {
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **runs** of a block.
fn runs_of(node: &Node) -> Vec<Json> {
    node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default()
}

/// Where a run's words **begin** in the text.
///
/// The runs cover the text in order, each one saying how many
/// characters it covers, so the start of a run is the sum of the
/// lengths of the ones before it.
fn start_of(runs: &[Json], index: usize) -> usize {
    runs.iter().take(index).map(|run| run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize).sum()
}

/// A run without its **revision**: the mark is gone and the run is
/// ordinary text.
fn unmarked(run: &Json) -> Json {
    let mut out = run.clone();
    if let Some(map) = out.as_object_mut() {
        map.remove("revision");
    }
    out
}

/// The **resolution** of one run: the new text and the new runs.
fn resolve(
    text: &str,
    runs: &[Json],
    index: usize,
    accept: bool,
    id: &str,
) -> Result<(String, Vec<Json>), EngineError> {
    let Some(run) = runs.get(index) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the block has no run `{index}`"),
        });
    };
    let Some(kind) = revision_of(run) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the run `{index}` has no revision"),
        });
    };
    let mut new_runs = runs.to_vec();
    match (accept, kind) {
        // An insertion that is accepted is ordinary text, and a
        // deletion that is rejected is text that was never deleted.
        (true, "insert") | (true, "format") | (false, "delete") => {
            new_runs[index] = unmarked(run);
            Ok((text.to_string(), new_runs))
        }
        // An insertion that is rejected is taken away, and a deletion
        // that is accepted is really deleted.
        (false, "insert") | (true, "delete") => {
            let characters: Vec<char> = text.chars().collect();
            let start = start_of(runs, index).min(characters.len());
            let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            let end = start.saturating_add(length).min(characters.len());
            let mut kept: String = characters[..start].iter().collect();
            kept.extend(characters[end..].iter());
            new_runs.remove(index);
            Ok((kept, new_runs))
        }
        // The format a run had before the change is not kept, and so
        // it cannot be put back.
        (false, "format") => Err(EngineError::Failed {
            id: id.to_string(),
            message: "not supported yet: the format a run had before the change is not kept".to_string(),
        }),
        _ => {
            Err(EngineError::Failed { id: id.to_string(), message: format!("there is no revision `{kind}`") })
        }
    }
}

/// The **block** a node is, if it is one.
fn the_block<'a>(session: &'a Session, id: &str, node: u64) -> Result<&'a Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no block `{node}`"),
        });
    };
    if !found.kind.starts_with("prose.block.") {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{node}` is a `{}` and not a block", found.kind),
        });
    }
    Ok(found)
}

commands! {
    /// **Accept** a revision.
    prose.revision.accept {
        area: "prose",
        layer: "prose",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
            /// The run the revision is on: its number, counting from nothing.
            run: Number,
        },
        run: {
            let id = "prose.revision.accept".to_string();
            let block = self.block.0;
            let node = the_block(session, &id, block)?;
            let text = text_of(node);
            let runs = runs_of(node);
            let index = self.run.0 as usize;
            let (text, runs) = resolve(&text, &runs, index, true, &id)?;
            write_back(session, block, &text, &runs, &id)?;
            Ok(Outcome::changed(format!("accepted the revision of the run `{index}` of the block `{block}`"))
                .with_data(json!({ "block": block, "run": index })))

        }
    }

    /// **Reject** a revision.
    prose.revision.reject {
        area: "prose",
        layer: "prose",
        icon: "undo",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
            /// The run the revision is on: its number, counting from nothing.
            run: Number,
        },
        run: {
            let id = "prose.revision.reject".to_string();
            let block = self.block.0;
            let node = the_block(session, &id, block)?;
            let text = text_of(node);
            let runs = runs_of(node);
            let index = self.run.0 as usize;
            let (text, runs) = resolve(&text, &runs, index, false, &id)?;
            write_back(session, block, &text, &runs, &id)?;
            Ok(Outcome::changed(format!("rejected the revision of the run `{index}` of the block `{block}`"))
                .with_data(json!({ "block": block, "run": index })))

        }
    }

    /// **Accept** every revision of a block.
    prose.revision.accept.all {
        area: "prose",
        layer: "prose",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
        },
        run: {
            let id = "prose.revision.accept.all".to_string();
            let block = self.block.0;
            let node = the_block(session, &id, block)?;
            let (text, runs, count) = resolve_all(node, true, &id)?;
            write_back(session, block, &text, &runs, &id)?;
            Ok(Outcome::changed(format!("accepted {count} revision(s) of the block `{block}`"))
                .with_data(json!({ "block": block, "count": count })))

        }
    }

    /// **Reject** every revision of a block.
    prose.revision.reject.all {
        area: "prose",
        layer: "prose",
        icon: "undo",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
        },
        run: {
            let id = "prose.revision.reject.all".to_string();
            let block = self.block.0;
            let node = the_block(session, &id, block)?;
            let (text, runs, count) = resolve_all(node, false, &id)?;
            write_back(session, block, &text, &runs, &id)?;
            Ok(Outcome::changed(format!("rejected {count} revision(s) of the block `{block}`"))
                .with_data(json!({ "block": block, "count": count })))

        }
    }
}

/// The **resolution** of every revision of a block.
///
/// The runs are resolved one at a time, and each one is found again
/// after the last, because resolving a deletion takes a run away and
/// the runs after it move.
fn resolve_all(node: &Node, accept: bool, id: &str) -> Result<(String, Vec<Json>, usize), EngineError> {
    let mut text = text_of(node);
    let mut runs = runs_of(node);
    let mut count = 0usize;
    // The work is bounded by the number of runs, and every round takes
    // one revision away.
    for _ in 0..runs.len() + 1 {
        let Some(index) = runs.iter().position(|run| revision_of(run).is_some()) else { break };
        let (new_text, new_runs) = resolve(&text, &runs, index, accept, id)?;
        text = new_text;
        runs = new_runs;
        count += 1;
    }
    Ok((text, runs, count))
}

/// Put a **text** and its **runs** back into a block.
fn write_back(
    session: &mut Session,
    block: u64,
    text: &str,
    runs: &[Json],
    id: &str,
) -> Result<(), EngineError> {
    let Some(node) = session.document_mut()?.node_mut(block) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no block `{block}`"),
        });
    };
    node.data["text"] = json!(text);
    node.data["runs"] = Json::Array(runs.to_vec());
    Ok(())
}
