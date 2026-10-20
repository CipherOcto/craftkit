//! The commands themselves.
//!
//! Read this file as the manual. Every command is here, in one place, sorted,
//! with its parameters documented — and that is exactly what the generated docs,
//! the CLI listing and the MCP tool list are built from.
//!
//! # House rules for anything in this file
//!
//! - **Read parameters through [`craftkit_core::Params`].** Never index.
//! - **Return an error, do not clamp silently.** A refused command is a message
//!   a person can act on; a clamped one is a surprise ten minutes later.
//! - **Mutate through `Session`.** Never cache a document reference across a
//!   call; the snapshot in `Session::execute` is what makes the panic path safe.

use crate::command::{Area, CommandSpec, Icon, ParamDoc};
use crate::error::EngineError;
use crate::params::{self, Doc, node_by_id_or_name, value_for};
use crate::session::{Outcome, Session};
use craftkit_core::{FrameRate, Params, Tick};
use craftkit_model::{Easing, Key, Node, NodeKind, PropGroup, PropNode, PropPath, Property, limits};

// ---------------------------------------------------------------- document --

pub static DOC_RENAME: CommandSpec = CommandSpec {
    id: "doc.rename",
    label: "Rename plan",
    area: Area::Document,
    icon: Icon("pencil"),
    shortcut: None,
    doc: "Give the plan a new name.",
    params: &[ParamDoc::req("name", Doc::Text, "The new name.")],
    mutates: true,
    undoable: true,
    enabled: always,
    run: |s, p| {
        let name = required_text(p, "name")?;
        check_name(&name)?;
        let changed = s.doc_mut().set_name(name.clone());
        Ok(Outcome::with(format!("renamed to `{name}`"), serde_json::json!({ "name": name }))
            .unchanged_if(!changed))
    },
};

pub static DOC_RANGE: CommandSpec = CommandSpec {
    id: "doc.range",
    label: "Set range",
    area: Area::Document,
    icon: Icon("range"),
    shortcut: None,
    doc: "Set the plan's start and end instants and its frame rate.",
    params: &[
        ParamDoc::opt("start", Doc::Time, "First instant. Defaults to the current start."),
        ParamDoc::opt("end", Doc::Time, "One past the last instant. Defaults to the current end."),
        ParamDoc::opt("fps", Doc::Number, "Frame rate in frames per second."),
    ],
    mutates: true,
    undoable: true,
    enabled: always,
    run: |s, p| {
        // A range command with nothing in it changes nothing — and §44's rule
        // says a command that declares that it mutates and does not is an
        // error. That error is an **internal** one, which the tree documents
        // as "worth a crash report and a bug". But nothing to change is the
        // *user's* mistake, not the tree's, so it is refused here, with the
        // parameters in the message. §92.
        //
        // `null` counts as absent: a script that sends `{"fps": null}` means
        // "leave the rate alone", which is a thing to say — and it is a
        // *refusal*, not a failure.
        if p.raw("start").is_none() && p.raw("end").is_none() && p.raw("fps").is_none() {
            return Err(EngineError::Param {
                id: DOC_RANGE.id.to_string(),
                source: craftkit_core::ParamError::new(
                    "start",
                    "give at least one of `start`, `end` or `fps`",
                ),
            });
        }
        let fps = match p.int("fps")? {
            Some(f) => {
                // The bound is the model's, not a copy of it. The command used
                // to refuse anything above 240 while the reader accepted a
                // thousand, so a document could hold a rate that no command
                // could set — not even the rate it already had. §89.
                //
                // The comparison is on the `i64` the parameter is, not on a
                // cast of it: casting first would let 4 294 967 297 become
                // one frame per second.
                if f < 1 || f > i64::from(limits::MAX_FRAME_RATE) {
                    return Err(EngineError::Param {
                        id: DOC_RANGE.id.to_string(),
                        source: craftkit_core::ParamError::new(
                            "fps",
                            format!("{f} is not between 1 and {}", limits::MAX_FRAME_RATE),
                        ),
                    });
                }
                f as u32
            }
            None => s.document().meta.fps,
        };
        // After the new rate is known: `"48f"` must mean frame 48 *of this
        // document as it will be*, not of the one before the change.
        let rate = FrameRate(fps);
        let start = opt_time(p, "start", rate)?.unwrap_or(s.document().meta.start);
        let end = opt_time(p, "end", rate)?.unwrap_or(s.document().meta.end);
        if end <= start {
            // An empty range is not a document: `Document::validate` refuses
            // a range that is not strictly increasing, so a range that ended
            // *at* the start would leave a plan that could not be saved. §50.
            return Err(EngineError::Param {
                id: DOC_RANGE.id.to_string(),
                source: craftkit_core::ParamError::new("end", "the end must be after the start"),
            });
        }
        let changed = s.doc_mut().set_range(start, end, fps);
        Ok(Outcome::with(
            format!("range {:.3}s..{:.3}s at {fps} fps", start.seconds_f64(), end.seconds_f64()),
            serde_json::json!({ "start": start.raw(), "end": end.raw(), "fps": fps }),
        )
        .unchanged_if(!changed))
    },
};

// -------------------------------------------------------------------- node --

pub static NODE_ADD: CommandSpec = CommandSpec {
    id: "node.add",
    label: "Add node",
    area: Area::Node,
    icon: Icon("plus"),
    shortcut: Some("Ctrl+N"),
    doc: "Add a node to the end of the plan and return its id.",
    params: &[
        ParamDoc::opt("name", Doc::Text, "The node's name. Defaults to `<kind> <n>`."),
        ParamDoc::opt("kind", Doc::NodeKind, "One of source, filter, sink, cue, comment."),
        ParamDoc::opt("amount", Doc::Number, "Initial `amount`, within ±1e9. Defaults to 1."),
    ],
    mutates: true,
    undoable: true,
    enabled: always,
    run: |s, p| {
        let kind = opt_kind(p, "kind")?.unwrap_or(NodeKind::Filter);
        let amount = p
            .num_in("amount", -craftkit_core::MAX_MAGNITUDE, craftkit_core::MAX_MAGNITUDE, None)?
            .unwrap_or(1.0);
        let name = match p.text("name", limits::MAX_NAME)? {
            Some(n) => n,
            None => {
                let n = s.document().nodes.len();
                format!("{} {}", kind.label(), n + 1)
            }
        };
        check_name(&name)?;
        let id = s.doc_mut().add_node(name.clone(), kind)?;
        if let Some(node) = s.doc_mut().get_mut(id) {
            node.set_defaults(true, amount)?;
        }
        Ok(Outcome::with(
            format!("added {} `{name}`", kind.label()),
            serde_json::json!({ "id": id.get(), "name": name, "kind": kind.label() }),
        ))
    },
};

pub static NODE_REMOVE: CommandSpec = CommandSpec {
    id: "node.remove",
    label: "Delete node",
    area: Area::Node,
    icon: Icon("trash"),
    shortcut: Some("Del"),
    doc: "Delete a node. Returns what was removed.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let node = s.document().get(id).cloned().ok_or_else(|| EngineError::Param {
            id: NODE_REMOVE.id.to_string(),
            source: craftkit_core::ParamError::new("node", format!("there is no node with id {}", id.get())),
        })?;
        s.doc_mut().remove(id);
        Ok(Outcome::with(
            format!("deleted `{}`", node.name),
            serde_json::json!({ "id": id.get(), "name": node.name }),
        ))
    },
};

pub static NODE_RENAME: CommandSpec = CommandSpec {
    id: "node.rename",
    label: "Rename node",
    area: Area::Node,
    icon: Icon("pencil"),
    shortcut: Some("F2"),
    doc: "Rename a node. Names need not be unique; ids are what commands should address.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("name", Doc::Text, "The new name."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let name = required_text(p, "name")?;
        check_name(&name)?;
        let node = node_mut(s, id, NODE_RENAME.id)?;
        let changed = node.rename(name.clone());
        Ok(Outcome::with(format!("renamed to `{name}`"), serde_json::json!({ "id": id.get() }))
            .unchanged_if(!changed))
    },
};

pub static NODE_MUTE: CommandSpec = CommandSpec {
    id: "node.mute",
    label: "Mute node",
    area: Area::Node,
    icon: Icon("mute"),
    shortcut: Some("M"),
    doc: "Mute or unmute a node. A muted node passes the level through untouched.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::opt("muted", Doc::Bool, "True to mute, false to unmute. Defaults to the opposite."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let muted = match p.bool("muted")? {
            Some(b) => b,
            None => !node_mut(s, id, NODE_MUTE.id)?.muted,
        };
        let changed = node_mut(s, id, NODE_MUTE.id)?.set_muted(muted);
        Ok(Outcome::with(
            if muted { "muted" } else { "unmuted" },
            serde_json::json!({ "id": id.get(), "muted": muted }),
        )
        .unchanged_if(!changed))
    },
};

pub static NODE_KIND: CommandSpec = CommandSpec {
    id: "node.kind",
    label: "Change kind",
    area: Area::Node,
    icon: Icon("swap"),
    shortcut: None,
    doc: "Change what a node does.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("kind", Doc::NodeKind, "One of source, filter, sink, cue, comment."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let kind = opt_kind(p, "kind")?.ok_or_else(|| EngineError::Param {
            id: NODE_KIND.id.to_string(),
            source: craftkit_core::ParamError::new("kind", "this parameter is required"),
        })?;
        let changed = node_mut(s, id, NODE_KIND.id)?.set_kind(kind);
        Ok(Outcome::with(
            format!("is now a {}", kind.label()),
            serde_json::json!({ "id": id.get(), "kind": kind.label() }),
        )
        .unchanged_if(!changed))
    },
};

pub static NODE_MOVE: CommandSpec = CommandSpec {
    id: "node.move",
    label: "Move node",
    area: Area::Node,
    icon: Icon("arrows"),
    shortcut: None,
    doc: "Move a node to another position. Position matters: a level flows from node to node.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("index", Doc::Number, "Destination index, 0-based."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let index = p.int("index")?.ok_or_else(|| EngineError::Param {
            id: NODE_MOVE.id.to_string(),
            source: craftkit_core::ParamError::new("index", "this parameter is required"),
        })?;
        if index < 0 {
            return Err(EngineError::Param {
                id: NODE_MOVE.id.to_string(),
                source: craftkit_core::ParamError::new("index", format!("{index} is negative")),
            });
        }
        let changed = s.doc_mut().move_to(id, index as usize)?;
        Ok(Outcome::with(format!("moved to {index}"), serde_json::json!({ "id": id.get(), "index": index }))
            .unchanged_if(!changed))
    },
};

pub static NODE_DUPLICATE: CommandSpec = CommandSpec {
    id: "node.duplicate",
    label: "Duplicate node",
    area: Area::Node,
    icon: Icon("copy"),
    shortcut: Some("Ctrl+D"),
    doc: "Copy a node, its properties and its keyframes, directly below the original.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let src = s.document().get(id).cloned().ok_or_else(|| EngineError::Param {
            id: NODE_DUPLICATE.id.to_string(),
            source: craftkit_core::ParamError::new("node", format!("there is no node with id {}", id.get())),
        })?;
        let at = s.document().index_of(id).map(|i| i + 1).unwrap_or(0);
        let new_id = s.doc_mut().add_node(format!("{} copy", src.name), src.kind)?;
        if let Some(node) = s.doc_mut().get_mut(new_id) {
            node.props = src.props.clone();
            node.muted = src.muted;
        }
        s.doc_mut().move_to(new_id, at)?;
        Ok(Outcome::with(
            format!("duplicated `{}`", src.name),
            serde_json::json!({ "id": new_id.get(), "source": id.get() }),
        ))
    },
};

// ---------------------------------------------------------------- property --

pub static PROP_SET: CommandSpec = CommandSpec {
    id: "prop.set",
    label: "Set property",
    area: Area::Property,
    icon: Icon("value"),
    shortcut: None,
    doc: "Set one property of a node by path. Creates the property if it does not exist.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req(
            "path",
            Doc::PropertyPath,
            "The property path, e.g. `amount` or `trigger/lead`. A path that names an id \
             (`@1f4a`) or a position (`#2`) names something that already exists, so it can \
             change a property but not create one.",
        ),
        ParamDoc::req(
            "value",
            Doc::Text,
            "A number, text, `true`/`false`, or `#rrggbb`. A numeric string is stored as a number.",
        ),
        ParamDoc::opt("animated", Doc::Bool, "Also key the property at the playhead. Defaults to false."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let value = value_for(p, "value")?.ok_or_else(|| EngineError::Param {
            id: PROP_SET.id.to_string(),
            source: craftkit_core::ParamError::new("value", "this parameter is required"),
        })?;
        let animated = p.bool("animated")?.unwrap_or(false);

        let at = s.at();
        let node = node_mut(s, id, PROP_SET.id)?;
        let parsed = parse_path(&path, PROP_SET.id)?;
        let changed = match node.props.find_mut(&parsed) {
            Some(existing) => {
                if existing.locked {
                    return Err(EngineError::Param {
                        id: PROP_SET.id.to_string(),
                        source: craftkit_core::ParamError::new("path", format!("`{path}` is locked")),
                    });
                }
                let mut changed = existing.set_value(value.clone());
                if animated {
                    key_now(existing, at, PROP_SET.id)?;
                    changed = true;
                }
                changed
            }
            None => {
                // The model creates it: the path is the model's grammar, and
                // the model refuses a path that cannot name what it is asked
                // to create. §85.
                let fresh_id = next_prop_id(&node.props);
                node.props.insert(&parsed, fresh_id, value.clone()).map_err(|e| EngineError::Param {
                    id: PROP_SET.id.to_string(),
                    source: craftkit_core::ParamError::new("path", e.to_string()),
                })?;
                if animated {
                    let created = node.props.find_mut(&parsed).ok_or_else(|| EngineError::Internal {
                        id: PROP_SET.id.to_string(),
                        detail: "the property was just created and could not be found".to_string(),
                    })?;
                    key_now(created, at, PROP_SET.id)?;
                }
                true
            }
        };
        Ok(Outcome::with(
            format!("`{path}` = {}", value.display()),
            serde_json::json!({ "id": id.get(), "path": path, "value": value }),
        )
        .unchanged_if(!changed))
    },
};

pub static PROP_RESET: CommandSpec = CommandSpec {
    id: "prop.reset",
    label: "Clear animation",
    area: Area::Property,
    icon: Icon("undo"),
    shortcut: None,
    doc: "Remove a property's keyframes, leaving its stored value.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("path", Doc::PropertyPath, "The property path."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let node = node_mut(s, id, PROP_RESET.id)?;
        let parsed = parse_path(&path, PROP_RESET.id)?;
        let prop = node.props.find_mut(&parsed).ok_or_else(|| EngineError::Param {
            id: PROP_RESET.id.to_string(),
            source: craftkit_core::ParamError::new("path", format!("there is no property `{path}`")),
        })?;
        let n = prop.keys.len();
        prop.keys.clear();
        Ok(Outcome::with(
            format!("removed {n} keyframe(s) from `{path}`"),
            serde_json::json!({ "id": id.get(), "path": path, "removed": n }),
        )
        .unchanged_if(n == 0))
    },
};

// --------------------------------------------------------------- keyframes --

pub static KEY_SET: CommandSpec = CommandSpec {
    id: "key.set",
    label: "Set keyframe",
    area: Area::Keyframes,
    icon: Icon("key"),
    shortcut: None,
    doc: "Key a property at an instant, creating it if absent. The stored value is \
          set to the same value, so clearing the keys later leaves that number rather \
          than whatever the property happened to hold.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("path", Doc::PropertyPath, "The property path."),
        ParamDoc::req("value", Doc::Text, "The value at this key."),
        ParamDoc::opt("time", Doc::Time, "The instant. Defaults to the playhead."),
        ParamDoc::opt(
            "easing",
            Doc::Easing,
            "How this key is left: linear, hold, easeIn, easeOut or easeInOut. Defaults to linear.",
        ),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let value = value_for(p, "value")?.ok_or_else(|| EngineError::Param {
            id: KEY_SET.id.to_string(),
            source: craftkit_core::ParamError::new("value", "this parameter is required"),
        })?;
        let rate = s.document().meta.frame_rate();
        let time = opt_time(p, "time", rate)?.unwrap_or(s.at());
        let easing = opt_easing(p, "easing")?.unwrap_or(Easing::Linear);

        let node = node_mut(s, id, KEY_SET.id)?;
        let parsed = parse_path(&path, KEY_SET.id)?;
        let prop = match node.props.find_mut(&parsed) {
            Some(p) => p,
            None => {
                // The model creates it, by the model's grammar. §85.
                let fresh_id = next_prop_id(&node.props);
                node.props.insert(&parsed, fresh_id, value.clone()).map_err(|e| EngineError::Param {
                    id: KEY_SET.id.to_string(),
                    source: craftkit_core::ParamError::new("path", e.to_string()),
                })?;
                node.props.find_mut(&parsed).ok_or_else(|| EngineError::Internal {
                    id: KEY_SET.id.to_string(),
                    detail: "the property was just created and could not be found".to_string(),
                })?
            }
        };
        // What the key at that instant held before, so the command can say
        // whether it changed anything. §93.
        let before = prop.keys.keys().iter().find(|k| k.time == time).map(|k| k.value.clone());
        prop.keys.set(Key::new(time, value.clone(), easing)?)?;
        // The stored value follows the key. Without this, `prop.reset` — which
        // clears the keys and leaves the stored value — would snap a property
        // back to a number the user set three edits ago.
        let value_changed = prop.set_value(value.clone());
        let after = prop.keys.keys().iter().find(|k| k.time == time).map(|k| k.value.clone());
        let changed = value_changed || before != after;
        Ok(Outcome::with(
            format!("keyed `{path}` at {:.3}s", time.seconds_f64()),
            serde_json::json!({
                "id": id.get(),
                "path": path,
                "time": time.raw(),
                "value": value,
                "keys": prop.keys.len(),
            }),
        )
        .unchanged_if(!changed))
    },
};

pub static KEY_REMOVE: CommandSpec = CommandSpec {
    id: "key.remove",
    label: "Remove keyframe",
    area: Area::Keyframes,
    icon: Icon("key-off"),
    shortcut: None,
    doc: "Remove the keyframe at an instant. Refuses when there is no key there: \
          a command that reports success without changing anything is how an \
          agent concludes it has done something it has not.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("path", Doc::PropertyPath, "The property path."),
        ParamDoc::opt("time", Doc::Time, "The instant. Defaults to the playhead."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let rate = s.document().meta.frame_rate();
        let time = opt_time(p, "time", rate)?.unwrap_or(s.at());
        let node = node_mut(s, id, KEY_REMOVE.id)?;
        let parsed = parse_path(&path, KEY_REMOVE.id)?;
        let prop = node.props.find_mut(&parsed).ok_or_else(|| EngineError::Param {
            id: KEY_REMOVE.id.to_string(),
            source: craftkit_core::ParamError::new("path", format!("there is no property `{path}`")),
        })?;
        let removed = prop.keys.remove_at(time);
        if !removed {
            return Err(EngineError::Param {
                id: KEY_REMOVE.id.to_string(),
                source: craftkit_core::ParamError::new(
                    "time",
                    format!("`{path}` has no keyframe at {:.3}s", time.seconds_f64()),
                ),
            });
        }
        Ok(Outcome::with(
            format!("removed the key on `{path}`"),
            serde_json::json!({ "id": id.get(), "path": path, "removed": removed, "keys": prop.keys.len() }),
        ))
    },
};

pub static KEY_CLEAR: CommandSpec = CommandSpec {
    id: "key.clear",
    label: "Clear keyframes",
    area: Area::Keyframes,
    icon: Icon("broom"),
    shortcut: None,
    doc: "Remove every keyframe from a property.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("path", Doc::PropertyPath, "The property path."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let node = node_mut(s, id, KEY_CLEAR.id)?;
        let parsed = parse_path(&path, KEY_CLEAR.id)?;
        let prop = node.props.find_mut(&parsed).ok_or_else(|| EngineError::Param {
            id: KEY_CLEAR.id.to_string(),
            source: craftkit_core::ParamError::new("path", format!("there is no property `{path}`")),
        })?;
        let n = prop.keys.len();
        prop.keys.clear();
        Ok(Outcome::with(
            format!("cleared {n} keyframe(s)"),
            serde_json::json!({ "id": id.get(), "removed": n }),
        )
        .unchanged_if(n == 0))
    },
};

pub static KEY_EASE: CommandSpec = CommandSpec {
    id: "key.ease",
    label: "Set easing",
    area: Area::Keyframes,
    icon: Icon("curve"),
    shortcut: None,
    doc: "Change how a keyframe is left.",
    params: &[
        ParamDoc::req("node", Doc::NodeRef, "The node's id."),
        ParamDoc::opt("node_name", Doc::Text, "…or its name."),
        ParamDoc::req("path", Doc::PropertyPath, "The property path."),
        ParamDoc::opt("time", Doc::Time, "Which key. Defaults to the playhead."),
        ParamDoc::req("easing", Doc::Easing, "linear, hold, easeIn, easeOut or easeInOut."),
    ],
    mutates: true,
    undoable: true,
    enabled: has_nodes,
    run: |s, p| {
        let id = node_by_id_or_name(s.document(), p)?;
        let path = required_path(p, "path")?;
        let rate = s.document().meta.frame_rate();
        let time = opt_time(p, "time", rate)?.unwrap_or(s.at());
        let easing = opt_easing(p, "easing")?.ok_or_else(|| EngineError::Param {
            id: KEY_EASE.id.to_string(),
            source: craftkit_core::ParamError::new("easing", "this parameter is required"),
        })?;
        let node = node_mut(s, id, KEY_EASE.id)?;
        let parsed = parse_path(&path, KEY_EASE.id)?;
        let prop = node.props.find_mut(&parsed).ok_or_else(|| EngineError::Param {
            id: KEY_EASE.id.to_string(),
            source: craftkit_core::ParamError::new("path", format!("there is no property `{path}`")),
        })?;
        // The easing goes in through the model's own door, so the list keeps
        // its rules: the easing is checked, and the key is found by its time
        // in a list that is sorted. §91.
        //
        // What the key held before, so the command can say whether it changed
        // anything: easing a key it already eases changes nothing, and that
        // is a success, not a failure. §93.
        let before = prop.keys.keys().iter().find(|k| k.time == time).map(|k| k.ease);
        if before.is_none() {
            return Err(EngineError::Param {
                id: KEY_EASE.id.to_string(),
                source: craftkit_core::ParamError::new("time", format!("`{path}` has no key there")),
            });
        }
        prop.keys.set_ease(time, easing).map_err(|e| EngineError::Param {
            id: KEY_EASE.id.to_string(),
            source: craftkit_core::ParamError::new("easing", e.to_string()),
        })?;
        let changed = before != Some(easing);
        Ok(Outcome::with(format!("`{path}` eased {}", easing.name()), serde_json::json!({ "id": id.get() }))
            .unchanged_if(!changed))
    },
};

// ---------------------------------------------------------------- playback --

pub static PLAYBACK_SEEK: CommandSpec = CommandSpec {
    id: "playback.seek",
    label: "Go to",
    area: Area::Playback,
    icon: Icon("play"),
    shortcut: None,
    doc: "Move the playhead. Accepts seconds, a frame number, or raw ticks.",
    params: &[
        ParamDoc::req("seconds", Doc::Time, "The instant to go to, in seconds."),
        ParamDoc::opt("frame", Doc::Number, "…or a frame number at the plan's rate."),
    ],
    mutates: false,
    undoable: false,
    enabled: always,
    run: |s, p| {
        let rate = s.document().meta.frame_rate();
        let to = match opt_time(p, "seconds", rate)? {
            Some(t) => t,
            None => match p.int("frame")? {
                Some(f) => Tick::from_frames_i64(f, rate),
                None => {
                    return Err(EngineError::Param {
                        id: PLAYBACK_SEEK.id.to_string(),
                        source: craftkit_core::ParamError::new("seconds", "pass `seconds` or `frame`"),
                    });
                }
            },
        };
        let clamped = to.clamp(s.document().meta.start, s.document().meta.end);
        let was = s.at();
        s.set_at(clamped);
        Ok(Outcome::with(
            format!("{:.3}s", clamped.seconds_f64()),
            serde_json::json!({ "from": was.raw(), "to": clamped.raw(), "seconds": clamped.seconds_f64() }),
        ))
    },
};

// ----------------------------------------------------------------- history --

pub static HISTORY_UNDO: CommandSpec = CommandSpec {
    id: "history.undo",
    label: "Undo",
    area: Area::History,
    icon: Icon("undo"),
    shortcut: Some("Ctrl+Z"),
    doc: "Step back one change.",
    params: &[],
    mutates: true,
    undoable: false,
    enabled: |s| s.can_undo(),
    run: |s, _p| {
        let label = s.history_mut().undo_label().unwrap_or("change").to_string();
        let current = s.document().clone();
        let restored = s.history_mut().pop_undo(current).ok_or_else(|| EngineError::Disabled {
            id: HISTORY_UNDO.id.into(),
            reason: "there is nothing to undo".into(),
        })?;
        let n = restored.nodes.len();
        s.replace_document(restored);
        Ok(Outcome::with(format!("undid {label}"), serde_json::json!({ "nodes": n })))
    },
};

pub static HISTORY_REDO: CommandSpec = CommandSpec {
    id: "history.redo",
    label: "Redo",
    area: Area::History,
    icon: Icon("redo"),
    shortcut: Some("Ctrl+Shift+Z"),
    doc: "Step forward one undone change.",
    params: &[],
    mutates: true,
    undoable: false,
    enabled: |s| s.can_redo(),
    run: |s, _p| {
        let label = s.history_mut().redo_label().unwrap_or("change").to_string();
        let current = s.document().clone();
        let restored = s.history_mut().pop_redo(current).ok_or_else(|| EngineError::Disabled {
            id: HISTORY_REDO.id.into(),
            reason: "there is nothing to redo".into(),
        })?;
        let n = restored.nodes.len();
        s.replace_document(restored);
        Ok(Outcome::with(format!("redid {label}"), serde_json::json!({ "nodes": n })))
    },
};

// ------------------------------------------------------------------- debug --

pub static PLAN_EVALUATE: CommandSpec = CommandSpec {
    id: "plan.evaluate",
    label: "Evaluate plan",
    area: Area::Debug,
    icon: Icon("play"),
    shortcut: None,
    doc: "Evaluate the document at an instant and return the whole plan.",
    params: &[ParamDoc::opt("at", Doc::Time, "The instant. Defaults to the playhead.")],
    mutates: false,
    undoable: false,
    enabled: always,
    run: |s, p| {
        let rate = s.document().meta.frame_rate();
        let at = opt_time(p, "at", rate)?.unwrap_or(s.at());
        let plan = craftkit_eval::evaluate_once(s.document(), at).map_err(EngineError::Eval)?;
        Ok(Outcome::with(plan.summary(), serde_json::to_value(&plan).unwrap_or(serde_json::Value::Null)))
    },
};

pub static DIAGNOSTICS_LIST: CommandSpec = CommandSpec {
    id: "diagnostics.list",
    label: "Problems",
    area: Area::Debug,
    icon: Icon("warning"),
    shortcut: None,
    doc: "List what the evaluator has to say about the current plan.",
    params: &[],
    mutates: false,
    undoable: false,
    enabled: always,
    run: |s, _p| {
        let items: Vec<serde_json::Value> = s
            .plan()
            .map(|p| {
                p.diagnostics
                    .items
                    .iter()
                    .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null))
                    .collect()
            })
            .unwrap_or_default();
        let dropped = s.plan().map(|p| p.diagnostics.dropped).unwrap_or(0);
        Ok(Outcome::with(
            format!("{} problem(s)", items.len() + dropped),
            serde_json::json!({ "items": items, "dropped": dropped }),
        ))
    },
};

pub static SESSION_STATS: CommandSpec = CommandSpec {
    id: "session.stats",
    label: "Session",
    area: Area::Debug,
    icon: Icon("info"),
    shortcut: None,
    doc: "Counters for the running session: commands, failures, undo depth, kernel recoveries (process-wide).",
    params: &[],
    mutates: false,
    undoable: false,
    enabled: always,
    run: |s, _p| {
        Ok(Outcome::with(
            format!("{} command(s), {} failure(s)", s.log().total(), s.log().failures()),
            serde_json::json!({
                "commands": s.log().total(),
                "failures": s.log().failures(),
                "undo_depth": s.undo_depth(),
                "kernel_recoveries": craftkit_kernel::recovered_count(),
                "nodes": s.document().nodes.len(),
            }),
        ))
    },
};

// ------------------------------------------------------------------ helpers --

fn always(_s: &Session) -> bool {
    true
}

fn has_nodes(s: &Session) -> bool {
    !s.document().nodes.is_empty()
}

fn required_text(p: &Params<'_>, key: &str) -> Result<String, EngineError> {
    required_text_capped(p, key, limits::MAX_NAME)
}

/// A required text parameter whose limit is the **model's**, not this file's.
///
/// A name is at most `MAX_NAME` bytes; a **path** is made of names, so it may
/// be as long as the grammar can express one. Capping a path at a name's
/// length was the tree refusing a path it had published itself: the listing
/// builds paths out of the model's names, and a path of two hundred-byte
/// groups is four hundred bytes, which no command would carry. §97.
fn required_path(p: &Params<'_>, key: &str) -> Result<String, EngineError> {
    required_text_capped(p, key, limits::MAX_PATH_BYTES)
}

fn required_text_capped(p: &Params<'_>, key: &str, max: usize) -> Result<String, EngineError> {
    let text = p.text(key, max).map_err(|e| EngineError::Param { id: String::new(), source: e })?;
    match text {
        None => Err(EngineError::Param {
            id: String::new(),
            source: craftkit_core::ParamError::new(key, "this parameter is required"),
        }),
        // The parameter **was** given. Saying "required" about a parameter
        // that arrived is a message that sends the reader looking for the
        // wrong mistake. §92.
        Some(s) if s.trim().is_empty() => Err(EngineError::Param {
            id: String::new(),
            source: craftkit_core::ParamError::new(key, "this parameter must not be empty"),
        }),
        Some(s) => Ok(s),
    }
}

/// Check a name against the model's rule.
///
/// The rule is the **model's** — `Node::why_not_a_name` — and the command
/// asks it, so the command and the document validator cannot disagree about
/// what a name is. They did: the command accepted an empty name and the
/// validator refused it, so the user's empty name came back as the tree's own
/// unexpected failure. §92.
///
/// The error is a **parameter** error, naming the parameter: a name the user
/// typed is the user's mistake, and the tree says so in the category that
/// means "a refusal", not the one that means "a crash and a bug".
fn check_name(name: &str) -> Result<(), EngineError> {
    if let Some(why) = craftkit_model::Node::why_not_a_name(name) {
        return Err(EngineError::Param {
            id: String::new(),
            source: craftkit_core::ParamError::new("name", why),
        });
    }
    Ok(())
}

/// Read an optional instant. Accepts a number of seconds, or a string like
/// `"12f"` for frames and `"120t"` for raw ticks.
///
/// `rate` is the document's own frame rate, so `"24f"` means the same frame in
/// a 24 fps plan and in a 30 fps one. Hard-coding 24 here would silently give a
/// 30 fps plan the wrong instant, which is the sort of bug that only shows up
/// in someone else's project.
fn opt_time(p: &Params<'_>, key: &str, rate: FrameRate) -> Result<Option<Tick>, EngineError> {
    let bad = |detail: String| EngineError::Param {
        id: String::new(),
        source: craftkit_core::ParamError::new(key, detail),
    };
    match p.raw(key) {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => {
            let secs = n.as_f64().ok_or_else(|| bad(format!("{n} is not a number of seconds")))?;
            Ok(Some(Tick::try_from_seconds_f64("time", secs).map_err(|e| bad(e.to_string()))?))
        }
        Some(serde_json::Value::String(s)) => parse_time_text(s, rate).map(Some).ok_or_else(|| {
            bad(format!("`{s}` is not a time; use seconds (1.5), frames (12f) or ticks (120t)"))
        }),
        Some(other) => Err(bad(format!("a {} is not a time", params::kind_of(other)))),
    }
}

fn parse_time_text(s: &str, rate: FrameRate) -> Option<Tick> {
    let t = s.trim();
    let (body, factor) = match t.chars().last() {
        Some('f') | Some('F') => (&t[..t.len() - 1], 'f'),
        Some('t') | Some('T') => (&t[..t.len() - 1], 't'),
        _ => (t, 's'),
    };
    let n: f64 = body.trim().parse().ok()?;
    match factor {
        's' => Tick::try_from_seconds_f64("time", n).ok(),
        'f' => Some(Tick::from_frames_i64(n as i64, rate)),
        _ => Some(Tick::from_raw(n as i128)),
    }
}

fn opt_kind(p: &Params<'_>, key: &str) -> Result<Option<NodeKind>, EngineError> {
    let bad = |detail: String| EngineError::Param {
        id: String::new(),
        source: craftkit_core::ParamError::new(key, detail),
    };
    let Some(raw) = p.raw(key) else { return Ok(None) };
    let text = match raw {
        serde_json::Value::String(s) => s.clone(),
        other => return Err(bad(format!("a {} is not a node kind; expected text", params::kind_of(other)))),
    };
    NodeKind::all().iter().find(|k| k.label() == text).copied().map(Some).ok_or_else(|| {
        bad(format!("`{text}` is not a kind; expected one of source, filter, sink, cue, comment"))
    })
}

fn opt_easing(p: &Params<'_>, key: &str) -> Result<Option<Easing>, EngineError> {
    let Some(raw) = p.raw(key) else { return Ok(None) };
    let serde_json::Value::String(s) = raw else {
        return Err(EngineError::Param {
            id: String::new(),
            source: craftkit_core::ParamError::new(key, "expected an easing name as text"),
        });
    };
    Easing::preset(s).map(Some).ok_or_else(|| EngineError::Param {
        id: String::new(),
        source: craftkit_core::ParamError::new(
            key,
            format!("`{s}` is not an easing; expected linear, hold, easeIn, easeOut or easeInOut"),
        ),
    })
}

fn node_mut<'a>(
    s: &'a mut Session,
    id: craftkit_core::NodeId,
    id_of: &'static str,
) -> Result<&'a mut Node, EngineError> {
    s.doc_mut().get_mut(id).ok_or_else(|| EngineError::Param {
        id: id_of.to_string(),
        source: craftkit_core::ParamError::new("node", format!("there is no node with id {}", id.get())),
    })
}

/// Parse a path with the model's own grammar.
///
/// One addressing scheme, one implementation. A second parser here would be a
/// second thing to keep in step with `craftkit-model`.
fn parse_path(path: &str, id_of: &'static str) -> Result<PropPath, EngineError> {
    PropPath::parse(path).ok_or_else(|| EngineError::Param {
        id: id_of.to_string(),
        source: craftkit_core::ParamError::new(
            "path",
            format!(
                "`{path}` is not a path this build accepts: a path has at most {} segments, \
                 and every segment must name a property or a group",
                craftkit_model::limits::MAX_PATH_SEGMENTS
            ),
        ),
    })
}

/// The next free property id inside a tree. Ids are per-node and stable, so a
/// new property must not reuse one that is already taken.
fn next_prop_id(group: &PropGroup) -> u64 {
    fn walk(group: &PropGroup, max: &mut u64) {
        for n in &group.nodes {
            match n {
                PropNode::Prop(p) => *max = (*max).max(p.id),
                PropNode::Group(g) => walk(g, max),
            }
        }
    }
    let mut max = 0u64;
    walk(group, &mut max);
    max.saturating_add(1)
}

/// Add a keyframe at `at` with the property's current value.
fn key_now(prop: &mut Property, at: Tick, id_of: &'static str) -> Result<(), EngineError> {
    let value = prop.value.clone();
    prop.keys.set(Key::new(at, value, Easing::Linear).map_err(|e| EngineError::Param {
        id: id_of.to_string(),
        source: craftkit_core::ParamError::new("time", e.to_string()),
    })?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value as Json, json};

    fn session() -> Session {
        Session::sample()
    }

    /// Run a command, expecting it to succeed.
    fn ok(s: &mut Session, id: &str, params: Json) -> String {
        s.execute(id, &params).expect("command should succeed").message
    }

    #[test]
    fn a_frame_string_uses_the_documents_own_frame_rate() {
        let mut s = session();
        ok(&mut s, "doc.range", json!({ "fps": 30 }));
        // Frame 30 at 30 fps is one second; at 24 it would be 1.25 s.
        ok(&mut s, "playback.seek", json!({ "seconds": "30f" }));
        assert!((s.at().seconds_f64() - 1.0).abs() < 1e-6, "landed at {:.4}s", s.at().seconds_f64());
    }

    #[test]
    fn seconds_frames_and_ticks_all_name_the_same_instant() {
        let mut s = session();
        let mut landed = Vec::new();
        for seconds in [json!(1.0), json!("24f"), json!("254016000000t")] {
            ok(&mut s, "playback.seek", json!({ "seconds": seconds }));
            landed.push(s.at().raw());
        }
        assert_eq!(landed[0], landed[1], "1.0 s and 24f differ");
        assert_eq!(landed[0], landed[2], "1.0 s and raw ticks differ");
    }

    /// A hostile time must not panic the command that reports it.
    ///
    /// The `time` parameter accepts anything up to 10^9 seconds, and 10^9
    /// seconds is 2.5 × 10^20 ticks — more than a JSON number holds. The
    /// command that reported such a tick built its reply with `json!`, which
    /// converts an interpolated value with `to_value(…).unwrap()` **inside the
    /// macro**, where the lint that forbids `unwrap` cannot see it. The
    /// command panicked, and the user was told *"called `Result::unwrap()` on
    /// an `Err` value"*. §60.
    #[test]
    fn a_hostile_time_does_not_panic_the_command_that_reports_it() {
        let mut s = session();
        ok(&mut s, "node.add", json!({ "name": "n", "kind": "filter" }));
        // Every one of these is inside the parameter's accepted range.
        for time in [json!(1e8), json!(1e9), json!("18446744073709551616t"), json!("-99999999999999999999t")]
        {
            // The point is that this returns at all: before the fix it
            // panicked inside `json!` and the caller saw an internal error
            // with a serde message in it.
            let outcome =
                s.execute("key.set", &json!({ "node": 1, "path": "amount", "value": 1.0, "time": time }));
            if let Err(e) = &outcome {
                assert!(!e.to_string().contains("unwrap"), "the command panicked rather than refusing: {e}");
            }
        }
        // And the session is still usable.
        ok(&mut s, "session.stats", json!({}));
    }

    /// The same for the command that reports the document's range.
    #[test]
    fn a_hostile_range_does_not_panic_the_command_that_reports_it() {
        let mut s = session();
        let outcome = s.execute("doc.range", &json!({ "start": 0, "end": 1e9 }));
        if let Err(e) = &outcome {
            assert!(!e.to_string().contains("unwrap"), "the command panicked rather than refusing: {e}");
        }
        ok(&mut s, "session.stats", json!({}));
    }

    /// The statistic that reports kernel recoveries must count recoveries that
    /// happened on another thread.
    ///
    /// The document is evaluated on the UI thread; `session.stats` is read from
    /// the control channel's thread. With a per-thread counter the statistic
    /// could only ever report recoveries that happened on the thread asking —
    /// which, in the desktop app, is none of them: the counter was
    /// structurally incapable of reporting the thing it exists to report. §62.
    #[test]
    fn the_session_statistics_count_a_recovery_that_happened_on_another_thread() {
        let mut s = session();
        let before = craftkit_kernel::recovered_count();
        let worker = std::thread::spawn(|| {
            let r: Result<i32, craftkit_kernel::KernelError> =
                craftkit_kernel::guard("probe", || -> Result<i32, craftkit_kernel::KernelError> {
                    panic!("a panic on a thread that is not the one asking")
                });
            assert!(r.expect_err("the boundary should have caught it").is_internal());
        });
        worker.join().expect("the worker should not panic");

        let outcome = s.execute("session.stats", &json!({})).expect("the command should succeed");
        let reported = outcome
            .data
            .get("kernel_recoveries")
            .and_then(Json::as_u64)
            .expect("the statistics should report a number of recoveries");
        assert!(
            reported > before,
            "the statistics reported {reported} recoveries, but only {before} had happened before the worker ran"
        );
    }

    /// A path the grammar can express is a path a command can carry.
    ///
    /// §97: a path is made of names, and a name is at most `MAX_NAME` bytes,
    /// so a path of `MAX_PATH_SEGMENTS` names is longer than a name — and the
    /// commands capped a path at a *name's* length. So the tree published
    /// paths it would not accept: the listing builds them out of the model's
    /// names, and a path of two two-hundred-byte groups is four hundred bytes,
    /// which every command refused for its size.
    #[test]
    fn a_path_longer_than_a_name_is_still_carried() {
        let mut s = session();
        let first = "a".repeat(200);
        let second = "b".repeat(100);
        let path = format!("{first}/{second}");
        assert!(path.len() > craftkit_model::limits::MAX_NAME, "the path has to be longer than a name");

        let outcome = s.execute("prop.set", &json!({ "node": 1, "path": path.clone(), "value": 2.0 }));
        assert!(outcome.is_ok(), "a legal path was refused: {outcome:?}");
        assert!(s.document().validate().is_ok(), "the document the command left behind does not validate");
    }

    /// The statistics count the whole session, not the last 256 commands.
    ///
    /// §69: the command log is a bounded ring, and both counters were taken
    /// over the ring — so a session that had run three hundred commands
    /// reported 256 of them and 255 of the failures.
    #[test]
    fn the_session_statistics_count_every_command_and_not_only_the_last_two_hundred_and_fifty_six() {
        let mut s = session();
        for _ in 0..300 {
            // A command that refuses: the node is not there.
            let _ = s.execute("node.remove", &json!({ "node": 9999 }));
        }
        let outcome = s.execute("session.stats", &json!({})).expect("the command should succeed");
        assert_eq!(
            outcome.data.get("commands").and_then(Json::as_u64),
            Some(300),
            "the statistics reported {} command(s)",
            outcome.data.get("commands").and_then(Json::as_u64).unwrap_or_default()
        );
        assert_eq!(
            outcome.data.get("failures").and_then(Json::as_u64),
            Some(300),
            "the statistics reported {} failure(s)",
            outcome.data.get("failures").and_then(Json::as_u64).unwrap_or_default()
        );
        assert!(outcome.message.contains("300 command(s)"), "{}", outcome.message);
    }

    #[test]
    fn an_unparsable_time_is_refused_with_the_three_spellings() {
        let mut s = session();
        let err = s.execute("playback.seek", &json!({ "seconds": "soon" })).expect_err("refused");
        assert!(err.to_string().contains("12f"), "{err}");
        assert!(err.to_string().contains("120t"), "{err}");
    }

    #[test]
    fn seeking_is_clamped_to_the_document_range() {
        let mut s = session();
        ok(&mut s, "playback.seek", json!({ "seconds": 9999 }));
        assert_eq!(s.at(), s.document().meta.end);
        ok(&mut s, "playback.seek", json!({ "seconds": -9999 }));
        assert_eq!(s.at(), s.document().meta.start);
    }

    #[test]
    fn seeking_needs_seconds_or_frame() {
        let mut s = session();
        let err = s.execute("playback.seek", &json!({})).expect_err("refused");
        assert!(err.to_string().contains("frame"), "{err}");
    }

    #[test]
    fn adding_a_node_gives_it_the_standard_properties() {
        let mut s = session();
        ok(&mut s, "node.add", json!({ "name": "Standard", "kind": "filter" }));
        let node = s.document().nodes.last().expect("a node was added");
        let paths: Vec<String> = node.props.flatten().into_iter().map(|(p, _)| p.to_string()).collect();
        assert!(paths.contains(&"amount".to_string()), "{paths:?}");
        assert!(paths.contains(&"enabled".to_string()), "{paths:?}");
    }

    #[test]
    fn a_named_node_is_addressable_by_name() {
        let mut s = session();
        ok(&mut s, "node.rename", json!({ "node_name": "Taper", "name": "Slope" }));
        let msg = ok(&mut s, "node.mute", json!({ "node_name": "Slope" }));
        assert!(msg.contains("muted"), "{msg}");
    }

    #[test]
    fn setting_a_property_creates_it_when_absent() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "custom/thing", "value": 7 }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let paths: Vec<String> = node.props.flatten().into_iter().map(|(p, _)| p.to_string()).collect();
        assert!(paths.iter().any(|p| p.ends_with("thing")), "{paths:?}");
    }

    #[test]
    fn a_property_can_be_addressed_by_its_stable_id() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "amount", "value": 3 }));
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "@2", "value": 5 }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let by_id = craftkit_model::PropPath::parse("@2").expect("parses");
        let amount = node.props.find(&by_id).map(|prop| prop.value.clone());
        assert_eq!(amount.and_then(|v| v.as_num()), Some(5.0));
        // And the label path still names the same property.
        let by_label = craftkit_model::PropPath::parse("amount").expect("parses");
        assert_eq!(node.props.find(&by_label).map(|p| p.id), Some(2));
    }

    #[test]
    fn a_keyframe_sorts_itself_and_samples_in_between() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "amount", "value": 0 }));
        // Added out of order on purpose: the key list sorts itself.
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 10, "time": 1.0 }));
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 0, "time": 0.0 }));
        ok(&mut s, "playback.seek", json!({ "seconds": 1.0 }));
        let plan = s.execute("plan.evaluate", &json!({})).expect("plan");
        assert_eq!(plan.data["nodes"][0]["level_out"], json!(10.0));

        // Halfway between them the curve interpolates.
        ok(&mut s, "playback.seek", json!({ "seconds": 0.5 }));
        let mid = s.execute("plan.evaluate", &json!({})).expect("plan");
        let level = mid.data["nodes"][0]["level_out"].as_f64().unwrap_or(-1.0);
        assert!((4.9..=5.1).contains(&level), "halfway was {level}");

        // And before the first key it holds the first value.
        ok(&mut s, "playback.seek", json!({ "seconds": 0.0 }));
        let first = s.execute("plan.evaluate", &json!({})).expect("plan");
        assert_eq!(first.data["nodes"][0]["level_out"], json!(0.0));
    }

    #[test]
    fn removing_an_absent_keyframe_is_refused_rather_than_silently_succeeding() {
        let mut s = session();
        let err = s
            .execute("key.remove", &json!({ "node": 1, "path": "amount", "time": 3.0 }))
            .expect_err("refused");
        assert!(err.to_string().contains("no keyframe"), "{err}");
    }

    /// A property set by its id is the property the id names.
    ///
    /// The write side resolved a uid among the *direct children* of the group
    /// it was addressing, while the read side looked *anywhere below* it — so
    /// a nested property could be read by `@5` and not written by it. The
    /// engine writes, so `prop.set @5` answered "there is no such property"
    /// by inventing one: it reported "`@5` = 42", left the property the path
    /// named at its old value, and created a second property labelled `5` at
    /// the root. §85.
    #[test]
    fn a_property_set_by_its_id_is_the_property_the_id_names() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "a/b", "value": 1 }));
        let nested = s
            .document()
            .get(craftkit_core::NodeId(1))
            .expect("node")
            .props
            .flatten()
            .into_iter()
            .find(|(p, _)| p.to_string() == "a/b")
            .map(|(_, prop)| prop.id)
            .expect("the nested property");

        ok(&mut s, "prop.set", json!({ "node": 1, "path": format!("@{nested}"), "value": 42 }));

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let flat = node.props.flatten();
        // The property the path names holds the new value.
        let by_id = flat.iter().find(|(_, prop)| prop.id == nested).map(|(_, prop)| prop.value.clone());
        assert_eq!(by_id, Some(craftkit_core::Value::Num(42.0)), "the named property was not set");
        // And nothing was invented.
        assert_eq!(flat.len(), 5, "a property was invented: {flat:?}");
        assert!(!flat.iter().any(|(p, _)| p.to_string() == "5"), "a property labelled `5` was created");
    }

    /// A path too long to name is refused, and the document is untouched.
    ///
    /// The engine used to build the tree anyway: seventeen groups and a
    /// property the listing dropped and the parser refused — a property the
    /// file kept and nothing could see. §85.
    #[test]
    fn a_path_too_long_to_name_is_refused() {
        let mut s = session();
        // The shortest path that is too long: sixteen groups and a property.
        let deep: String = (0..16).map(|i| format!("g{i}")).collect::<Vec<_>>().join("/");
        let path = format!("{deep}/leaf");
        let err =
            s.execute("prop.set", &json!({ "node": 1, "path": path, "value": 1 })).expect_err("refused");
        assert!(err.to_string().contains("at most 16 segments"), "{err}");

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert_eq!(node.props.flatten().len(), 4, "something was created anyway");
    }

    /// A path that names a position cannot create anything.
    ///
    /// `#2` means "the second child", and a child that does not exist yet has
    /// no position to be second. The engine used to read the segment as a
    /// literal name and made a group whose id was `#2` — which the grammar
    /// could not describe, so the property it created could not be read back
    /// by the path that created it. §85.
    #[test]
    fn a_path_that_names_a_position_cannot_create_anything() {
        let mut s = session();
        let err = s
            .execute("prop.set", &json!({ "node": 1, "path": "effects/#2/amount", "value": 7 }))
            .expect_err("refused");
        assert!(err.to_string().contains("position"), "{err}");

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert_eq!(node.props.flatten().len(), 4, "something was created anyway");
    }

    /// An id that names no property cannot create one.
    ///
    /// The engine used to read the segment as a literal name and made a
    /// property labelled `99` — a property the grammar could not describe,
    /// because `@99` means "the property whose id is 99", and ids are the
    /// model's to assign. §85.
    #[test]
    fn an_id_that_names_no_property_cannot_create_anything() {
        let mut s = session();
        let err =
            s.execute("prop.set", &json!({ "node": 1, "path": "@99", "value": 7 })).expect_err("refused");
        assert!(err.to_string().contains("id"), "{err}");

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert_eq!(node.props.flatten().len(), 4, "something was created anyway");
    }

    /// A property created by a path can be read back by that path.
    #[test]
    fn a_created_property_is_readable_by_the_path_that_created_it() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "custom/thing", "value": 7 }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let path = craftkit_model::PropPath::parse("custom/thing").expect("parses");
        let found = node.props.find(&path).expect("the property the path names");
        assert_eq!(found.value.as_num(), Some(7.0));
        // And the listing publishes it under that path.
        let listed = node.props.flatten();
        assert!(listed.iter().any(|(p, _)| p.to_string() == "custom/thing"), "{listed:?}");
    }

    /// Setting a property that is already there changes that property, and
    /// creates nothing.
    #[test]
    fn setting_a_property_twice_does_not_create_a_second_one() {
        let mut s = session();
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "twice", "value": 1 }));
        ok(&mut s, "prop.set", json!({ "node": 1, "path": "twice", "value": 2 }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let twice: Vec<_> =
            node.props.flatten().into_iter().filter(|(p, _)| p.to_string() == "twice").collect();
        assert_eq!(twice.len(), 1, "{twice:?}");
        assert_eq!(twice[0].1.value.as_num(), Some(2.0));
    }

    /// A path the grammar cannot read is refused.
    ///
    /// `a#b` is not a name the grammar can write: it reads as the name `a` in
    /// the second place, or as nothing at all. A command that accepted it
    /// would create a property under a name the listing cannot publish, and
    /// the property it made could not be read back by the path that made it.
    /// §88.
    #[test]
    fn a_path_the_grammar_cannot_read_is_refused() {
        let mut s = session();
        let err =
            s.execute("prop.set", &json!({ "node": 1, "path": "a#b", "value": 1 })).expect_err("refused");
        assert!(err.to_string().contains("not a path this build accepts"), "{err}");

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert_eq!(node.props.flatten().len(), 4, "something was created anyway");
    }

    #[test]
    fn removing_a_keyframe_that_is_there_reports_how_many_are_left() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 2, "time": 0.0 }));
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 8, "time": 1.0 }));
        let out =
            s.execute("key.remove", &json!({ "node": 1, "path": "amount", "time": 0.0 })).expect("removed");
        assert_eq!(out.data["keys"], json!(1));
    }

    #[test]
    fn easing_an_absent_key_is_refused_with_the_key_in_the_message() {
        let mut s = session();
        let err = s
            .execute("key.ease", &json!({ "node": 1, "path": "amount", "time": 1.0, "easing": "hold" }))
            .expect_err("refused");
        assert!(err.to_string().contains("no key"), "{err}");
    }

    /// Easing a key changes the curve and nothing else.
    ///
    /// The easing goes in through the model's own door, so the list keeps its
    /// rules — the key is found by its time in a list that is sorted, and the
    /// easing is checked before it is stored. §91.
    #[test]
    fn easing_a_key_changes_the_curve_and_leaves_the_keys_alone() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 2, "time": 0.0 }));
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 8, "time": 1.0 }));
        ok(&mut s, "key.ease", json!({ "node": 1, "path": "amount", "time": 0.0, "easing": "easeOut" }));

        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let path = craftkit_model::PropPath::parse("amount").expect("a path");
        let prop = node.props.find(&path).expect("the property");
        assert_eq!(prop.keys.len(), 2, "easing a key must not add or remove keys");
        assert_eq!(
            prop.keys.keys()[0].ease,
            craftkit_model::keyframe::Easing::Bezier { out_x: 0.58, out_y: 1.0 }
        );
    }

    /// An easing the model refuses is refused.
    ///
    /// The presets are all legal, so this cannot be reached through the
    /// command's own parameter — the check is here because the door the
    /// easing comes through is the model's, and the model refuses it there
    /// too. §91.
    #[test]
    fn an_easing_the_model_refuses_is_refused() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 2, "time": 0.0 }));
        let err = s
            .execute("key.ease", &json!({ "node": 1, "path": "amount", "time": 0.0, "easing": "wobble" }))
            .expect_err("refused");
        assert!(err.to_string().contains("easing"), "{err}");
    }

    // ------------------------------------------------------------------ §92 --

    /// An empty name is the user's mistake, and it is reported as one.
    ///
    /// The command used to *accept* an empty name and let the document
    /// validator refuse it afterwards — so the tree's own backstop caught the
    /// user's empty name and reported it as *"failed unexpectedly"*, of the
    /// kind the tree documents as "worth a crash report and a bug". The
    /// document is unchanged either way, but the *kind* of the error is what
    /// an agent branches on. §92.
    #[test]
    fn an_empty_node_name_is_a_refusal_not_an_internal_failure() {
        let mut s = session();
        let before = s.document().nodes.len();
        let err = s.execute("node.add", &json!({ "name": "", "kind": "source" })).expect_err("refused");
        assert!(!err.is_internal(), "an empty name must not be reported as a crash: {err}");
        assert_eq!(err.kind(), "bad-parameter", "{err}");
        assert!(err.to_string().contains("name"), "{err}");
        assert_eq!(s.document().nodes.len(), before, "a refused node was added anyway");
    }

    /// The two commands that take a name agree about what a name is.
    ///
    /// `node.add { name: " " }` used to be accepted while `node.rename` with
    /// the same name was refused — two commands, one rule, two answers. §92.
    #[test]
    fn every_command_that_takes_a_name_agrees_about_what_a_name_is() {
        for bad in ["", "   "] {
            let mut s = session();
            let added = s.execute("node.add", &json!({ "name": bad, "kind": "source" }));
            assert!(added.is_err(), "node.add accepted the name {bad:?}");
            let mut s = session();
            let renamed = s.execute("node.rename", &json!({ "node": 1, "name": bad }));
            assert!(renamed.is_err(), "node.rename accepted the name {bad:?}");
            let mut s = session();
            let doc = s.execute("doc.rename", &json!({ "name": bad }));
            assert!(doc.is_err(), "doc.rename accepted the name {bad:?}");
        }
    }

    /// A range command with nothing in it is a refusal, not a failure.
    ///
    /// `doc.range {}` set every value to what it already was, changed
    /// nothing, and hit §44's backstop — *"the command declared that it
    /// mutates the document but changed nothing"* — which is an **internal**
    /// error. Asking a command to change nothing is the user's mistake. §92.
    #[test]
    fn a_range_command_with_nothing_to_change_is_a_refusal() {
        for params in [json!({}), json!({ "start": null }), json!({ "fps": null })] {
            let mut s = session();
            let err = s.execute("doc.range", &params).expect_err("refused");
            assert!(!err.is_internal(), "{params} must not be reported as a crash: {err}");
            assert_eq!(err.kind(), "bad-parameter", "{params}: {err}");
            assert!(err.to_string().contains("start"), "{params}: {err}");
        }
    }

    /// A parameter that was given but is empty says so.
    ///
    /// The message used to be "this parameter is required", which sends the
    /// reader looking for a parameter that was never sent. §92.
    #[test]
    fn a_parameter_that_was_given_but_is_empty_says_so() {
        let mut s = session();
        let err = s.execute("node.rename", &json!({ "node": 1, "name": "" })).expect_err("refused");
        assert!(err.to_string().contains("must not be empty"), "{err}");
        assert!(!err.to_string().contains("required"), "{err}");

        // And a parameter that was never given still says *that*.
        let mut s = session();
        let err = s.execute("node.rename", &json!({ "node": 1 })).expect_err("refused");
        assert!(err.to_string().contains("required"), "{err}");
    }

    // ------------------------------------------------------------------ §93 --

    /// A command asked to do what it has already done is a **success**.
    ///
    /// The engine compares the document with the snapshot it took before the
    /// command, and a command that declares that it mutates and changes
    /// nothing is refused — because that is usually a command that forgot to
    /// write. But the comparison cannot tell that from a command that wrote
    /// the value a field already held, and that is ordinary.
    ///
    /// Measured, before the fix: eight of the nine commands that can be asked
    /// to do what they have already done were refused as *"failed
    /// unexpectedly (the command declared that it mutates the document but
    /// changed nothing)"*. §93.
    #[test]
    fn a_command_asked_to_do_what_it_has_already_done_succeeds() {
        let cases: Vec<(&str, serde_json::Value)> = vec![
            ("prop.set", json!({ "node": 1, "path": "amount", "value": 1.0 })),
            ("prop.reset", json!({ "node": 1, "path": "amount" })),
            ("key.clear", json!({ "node": 1, "path": "amount" })),
            ("node.mute", json!({ "node": 1, "muted": false })),
            ("node.kind", json!({ "node": 1, "kind": "source" })),
            ("node.rename", json!({ "node": 1, "name": "Level" })),
            ("node.move", json!({ "node": 1, "index": 0 })),
            ("doc.rename", json!({ "name": "Sample plan" })),
            ("doc.range", json!({ "fps": 24 })),
        ];
        for (id, params) in cases {
            let mut s = session();
            let before = s.document().clone();
            let undo_before = s.undo_depth();
            let outcome = s.execute(id, &params).unwrap_or_else(|e| panic!("`{id}` refused: {e}"));
            assert!(!outcome.message.is_empty(), "`{id}` said nothing");
            assert_eq!(s.document(), &before, "`{id}` changed the document");
            assert_eq!(s.undo_depth(), undo_before, "`{id}` put a no-op step on the undo stack");
            assert!(!s.is_dirty(), "`{id}` marked the project dirty");
        }
    }

    /// The same, for the two commands that need a keyframe first.
    ///
    /// Keying a property at an instant it is already keyed at, with the value
    /// it already has, changes nothing — and easing a key it already eases
    /// changes nothing. Both are ordinary. §93.
    #[test]
    fn keying_and_easing_what_is_already_there_succeeds() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 3, "time": 1.0 }));
        // The first keying really did change the document, so this is the
        // state the no-ops are measured against.
        let before = s.document().clone();
        let undo_before = s.undo_depth();
        let dirty_before = s.is_dirty();

        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 3, "time": 1.0 }));
        assert_eq!(s.document(), &before, "keying a key that is already there changed the document");
        assert_eq!(s.undo_depth(), undo_before, "a no-op put a step on the undo stack");
        assert_eq!(s.is_dirty(), dirty_before, "a no-op changed the dirty flag");

        ok(&mut s, "key.ease", json!({ "node": 1, "path": "amount", "time": 1.0, "easing": "linear" }));
        assert_eq!(s.document(), &before, "easing a key that is already eased changed the document");
        assert_eq!(s.undo_depth(), undo_before, "a no-op put a step on the undo stack");
        assert_eq!(s.is_dirty(), dirty_before, "a no-op changed the dirty flag");
    }

    /// A command that claims to have changed something and did not is still a
    /// bug.
    ///
    /// The fix for §93 is that the **command** says whether it changed
    /// anything — and the default is "it did", which is the direction that
    /// keeps §44's check alive. A command that forgot to write still says
    /// nothing, so the engine still catches it. §44, §93.
    #[test]
    fn a_command_that_claims_to_change_something_and_does_not_is_still_refused() {
        let mut s = session();
        let err = s.execute("test.noop", &json!({})).expect_err("refused");
        assert!(err.is_internal(), "{err}");
        assert!(err.to_string().contains("changed nothing"), "{err}");
    }

    #[test]
    fn a_newer_frame_rate_changes_what_a_frame_means() {
        let mut s = session();
        ok(&mut s, "doc.range", json!({ "fps": 60 }));
        ok(&mut s, "playback.seek", json!({ "seconds": "60f" }));
        assert!((s.at().seconds_f64() - 1.0).abs() < 1e-6, "landed at {:.4}s", s.at().seconds_f64());
    }

    #[test]
    fn a_backwards_range_is_refused() {
        let mut s = session();
        let err = s.execute("doc.range", &json!({ "start": 2.0, "end": 1.0 })).expect_err("refused");
        assert!(err.to_string().contains("end"), "{err}");
    }

    #[test]
    fn an_impossible_frame_rate_is_refused_with_the_range() {
        let mut s = session();
        let err = s.execute("doc.range", &json!({ "fps": 0 })).expect_err("refused");
        assert!(err.to_string().contains("1000"), "{err}");
    }

    /// The command can set any rate a document may carry.
    ///
    /// The ceiling was written in five places and the two that decided anything
    /// disagreed: the reader accepted a thousand frames per second and the
    /// command accepted two hundred and forty. A document could therefore
    /// hold a rate that no command could set — not even the rate it already
    /// had. §89.
    #[test]
    fn the_command_can_set_any_rate_a_document_may_carry() {
        for fps in [0i64, 1, 24, 30, 240, 241, 500, 1000, 1001] {
            let mut doc = craftkit_model::Document::empty("t");
            doc.meta.fps = fps as u32;
            let model_ok = doc.validate().is_ok();

            let mut s = session();
            // Move the document off the rate under test, so the command has
            // something to change and the session's "changed nothing" guard
            // does not answer for it.
            let other = if fps == 24 { 25 } else { 24 };
            let _ = s.execute("doc.range", &json!({ "fps": other }));
            let command_ok = s.execute("doc.range", &json!({ "fps": fps })).is_ok();

            assert_eq!(
                model_ok, command_ok,
                "fps {fps}: the model says {model_ok}, the command says {command_ok}"
            );
        }
    }

    #[test]
    fn moving_a_node_beyond_the_end_is_refused_rather_than_panicking() {
        let mut s = session();
        let err = s.execute("node.move", &json!({ "node": 1, "index": 99 })).expect_err("refused");
        assert!(err.to_string().contains("index"), "{err}");
        assert_eq!(s.document().nodes.len(), 3);
    }

    #[test]
    fn duplicating_a_node_copies_its_properties_and_keys() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 4, "time": 0.0 }));
        let before = s.document().nodes.len();
        let out = s.execute("node.duplicate", &json!({ "node": 1 })).expect("duplicated");
        let new_id = craftkit_core::NodeId(out.data["id"].as_u64().expect("an id"));
        assert_eq!(s.document().nodes.len(), before + 1);

        let copy = s.document().get(new_id).expect("the copy");
        let original = s.document().nodes[0].clone();
        assert_eq!(copy.props.flatten().len(), original.props.flatten().len());
        assert_eq!(copy.kind, original.kind);
        assert!(copy.name.ends_with("copy"), "{}", copy.name);
        let amount = copy
            .props
            .flatten()
            .into_iter()
            .find(|(path, _)| path.to_string() == "amount")
            .map(|(_, prop)| prop.keys.len());
        assert_eq!(amount, Some(1), "the copy should carry the keyframes too");
        // Directly below the original, which is what "duplicate" means.
        let index = s.document().index_of(new_id).expect("the copy is in the list");
        assert_eq!(index, 1);
    }

    #[test]
    fn the_plan_command_returns_the_whole_plan() {
        let mut s = session();
        let out = s.execute("plan.evaluate", &json!({})).expect("plan");
        assert_eq!(out.data["nodes"].as_array().map(Vec::len), Some(3));
        assert!(out.data["diagnostics"].is_object());
        assert!(out.data["at"].is_number());
    }

    #[test]
    fn the_statistics_command_reports_real_counters() {
        let mut s = session();
        ok(&mut s, "node.add", json!({ "name": "Counted", "kind": "cue" }));
        let out = s.execute("session.stats", &json!({})).expect("stats");
        assert_eq!(out.data["nodes"], json!(4));
        assert!(out.data["commands"].as_u64().unwrap_or(0) >= 1);
        assert_eq!(out.data["failures"], json!(0));
    }

    #[test]
    fn the_diagnostics_command_lists_nothing_on_a_clean_document() {
        let mut s = session();
        let out = s.execute("diagnostics.list", &json!({})).expect("diagnostics");
        assert_eq!(out.data["items"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn the_diagnostics_command_lists_a_problem_once_there_is_one() {
        let mut s = session();
        // Force the type mismatch a hand-edited file produces.
        let id = s.document().nodes[0].id;
        {
            let node = s.doc_mut_for_test(id);
            node.props.nodes[0] =
                PropNode::Prop(Property::new(1, "amount", craftkit_core::Value::Bool(true)));
        }
        s.recompute_for_test();
        let out = s.execute("diagnostics.list", &json!({})).expect("diagnostics");
        let items = out.data["items"].as_array().expect("items");
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0]["severity"], json!("error"));
    }

    #[test]
    fn a_command_that_is_disabled_says_why_in_words() {
        let mut s = session();
        let err = s.execute("history.undo", &json!({})).expect_err("disabled");
        assert!(err.to_string().contains("nothing to undo"), "{err}");
    }

    #[test]
    fn clearing_keyframes_leaves_the_stored_value() {
        let mut s = session();
        ok(&mut s, "key.set", json!({ "node": 1, "path": "amount", "value": 6, "time": 0.0 }));
        ok(&mut s, "key.clear", json!({ "node": 1, "path": "amount" }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        let prop = node
            .props
            .flatten()
            .into_iter()
            .find(|(p, _)| p.to_string() == "amount")
            .map(|(_, p)| p)
            .expect("the property is still there");
        assert!(prop.keys.is_empty());
        assert_eq!(prop.value.as_num(), Some(6.0));
    }

    #[test]
    fn muting_is_a_toggle_unless_it_is_told_otherwise() {
        let mut s = session();
        ok(&mut s, "node.mute", json!({ "node": 1 }));
        ok(&mut s, "node.mute", json!({ "node": 1 }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert!(!node.muted, "two toggles should return to the start");
        ok(&mut s, "node.mute", json!({ "node": 1, "muted": true }));
        let node = s.document().get(craftkit_core::NodeId(1)).expect("node");
        assert!(node.muted);
    }

    #[test]
    fn a_node_becomes_a_different_kind_on_request() {
        let mut s = session();
        ok(&mut s, "node.kind", json!({ "node": 1, "kind": "sink" }));
        assert_eq!(
            s.document().get(craftkit_core::NodeId(1)).expect("node").kind,
            craftkit_model::NodeKind::Sink
        );
    }
}
