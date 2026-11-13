//! The **taking away** of a node: a clip, a marker and a track.
//!
//! The tree's editor can delete a clip, a marker and a track, and this
//! tree could do none of it: a thing that could be made but not
//! unmade is a thing that is there for ever.
//!
//! Each of them is a **command of its own**, because each of them is a
//! different thing: a clip is a piece of a track, a marker is a place
//! on the timeline and a track is a row of it.
//!
//! # What goes with what
//!
//! - A **clip** goes and its edges go with it: a clip that played a
//!   source is a clip that plays nothing, and the edge that said so
//!   is an edge to nothing.
//! - A **marker** goes and nothing else: a marker is a place and
//!   nothing points at it.
//! - A **track** goes with its **takes**, its **clips** and its
//!   **lanes**, and the **routes** and **sends** that named it go
//!   with it too — a route to a track that is not there is a route to
//!   nothing.
//!
//! # The locks
//!
//! A **locked** clip is finished: its content is locked or its place
//! is, and a clip whose content is locked is not taken away. The
//! tree's `edit.clear` removes a locked clip with the rest — it does
//! not ask — and here the lock is asked about first: a lock says the
//! clip stays where it is and as it is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// **Take away** a clip.
    ///
    /// The clip goes and its **edges** go with it: the relation that said
    /// the clip plays a source is a relation to a clip that is not there.
    ///
    /// A clip whose **content** is locked is **refused**: a lock says the
    /// clip is finished, and a finished clip is not thrown away.
    sound.clip.remove {
        area: "sound",
        layer: "sound",
        icon: "minus",
        mutates,
        undoable,
        menu: ("Clip", "Remove"),
        params: {
            /// The clip to take away.
            clip: NodeRef,
        },
        run: {
            let id = "sound.clip.remove".to_string();
            let clip = self.clip.0;
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no clip `{clip}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = crate::ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if crate::locks::edit_is_locked(&payload) {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the clip `{clip}` is locked: its sound is finished"),
                });
            }
            let name = node.name.clone();
            session.document_mut()?.remove_node(clip);
            Ok(Outcome::changed(format!("took the clip `{name}` away"))
                .with_data(serde_json::json!({ "clip": clip, "name": name })))

        }
    }

    /// **Take away** a marker.
    ///
    /// The marker goes and nothing else goes with it: a marker is a place
    /// on the timeline and nothing points at it.
    sound.marker.remove {
        area: "sound",
        layer: "sound",
        icon: "minus",
        mutates,
        undoable,
        menu: ("Marker", "Remove"),
        params: {
            /// The marker to take away.
            marker: NodeRef,
        },
        run: {
            let id = "sound.marker.remove".to_string();
            let marker = self.marker.0;
            let node = session.document().node(marker).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no marker `{marker}`"),
            })?;
            if node.kind != "sound.marker" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{marker}` is a `{}` and not a marker", node.kind),
                });
            }
            let name = node.name.clone();
            session.document_mut()?.remove_node(marker);
            Ok(Outcome::changed(format!("took the marker `{name}` away"))
                .with_data(serde_json::json!({ "marker": marker, "name": name })))

        }
    }

    /// **Take away** a track.
    ///
    /// The track goes with its **takes**, its **clips** and its **lanes**,
    /// and the **routes** and **sends** that named it go with it: a route
    /// to a track that is not there is a route to nothing.
    ///
    /// A **master** is not taken away: the master holds the fader of the
    /// whole mix, and a mix with no master has no fader.
    sound.track.remove {
        area: "sound",
        layer: "sound",
        icon: "minus",
        mutates,
        undoable,
        menu: ("Track", "Remove"),
        params: {
            /// The track to take away.
            track: NodeRef,
        },
        run: {
            let id = "sound.track.remove".to_string();
            let track = self.track.0;
            let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            })?;
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            if crate::is_master(node) {
                return Err(EngineError::Failed {
                    id,
                    message: "the master holds the fader of the mix and it does not go away".to_string(),
                });
            }
            let name = node.name.clone();
            // How much goes with it: the takes, the clips and the lanes.
            let mut clips = 0_usize;
            let mut lanes = 0_usize;
            for child in &node.children {
                match child.kind.as_str() {
                    "sound.playlist" => {
                        for clip in &child.children {
                            if clip.kind == "sound.clip" {
                                clips += 1;
                            }
                        }
                    }
                    "sound.lane" => lanes += 1,
                    _ => {}
                }
            }
            session.document_mut()?.remove_node(track);
            Ok(Outcome::changed(format!("took the track `{name}` away"))
                .with_data(serde_json::json!({ "track": track, "name": name, "clips": clips, "lanes": lanes })))

        }
    }
}
