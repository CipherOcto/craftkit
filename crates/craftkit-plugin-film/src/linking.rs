//! The **link** of a clip.
//!
//! Two clips may be **linked**: they are the same thing on two
//! tracks — a picture and its sound, a shot and its title — and they
//! move as one. The tree keeps a link group id in the clip and the
//! clips with the same id are the group.
//!
//! Here a clip's payload carries the **link**, and the link is the id
//! of the **first clip of the group** — the smallest id in it — so
//! that the group has a name that is not a number somebody had to
//! make up, and so that two groups can be merged by taking the
//! smallest of the two.
//!
//! The tree's rules, and they are the finding:
//!
//! - a **move** of a clip moves its partners by the **same offset** on
//!   their own tracks, so that the picture and the sound stay in step;
//! - a partner on a **locked** track stays where it is;
//! - a group of **one** is not a group: a clip left alone in its link
//!   is unlinked, because a link with nothing to follow is a link that
//!   reads as if it were something.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// The **link** of a clip: the group it is in.
///
/// A clip with no link is a clip of its own.
pub fn link_of(data: &serde_json::Value) -> Option<u64> {
    data.get("link").and_then(|link| link.as_u64())
}

/// The **group** a clip is in: the clip itself and its partners.
pub fn group(session: &Session, clip: u64) -> Vec<u64> {
    let Some(node) = session.document().node(clip) else {
        return Vec::new();
    };
    let Some(link) = link_of(&node.data) else {
        return vec![clip];
    };
    let mut group: Vec<u64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "film.clip")
        .filter(|node| link_of(&node.data) == Some(link))
        .map(|node| node.id)
        .collect();
    group.sort_unstable();
    group
}

/// The **partners** of a clip: the group without the clip itself.
pub fn partners(session: &Session, clip: u64) -> Vec<u64> {
    group(session, clip).into_iter().filter(|member| *member != clip).collect()
}

/// The **name** of a group: the smallest id in it.
///
/// The name of a group is the id of its first clip, so that a group
/// has a name that is in the document and that two groups may be
/// merged by taking the smallest of the two.
pub fn name_of(clips: &[u64]) -> Option<u64> {
    clips.iter().copied().min()
}

commands! {
    /// **Link** two clips together.
    ///
    /// The two clips are the same thing on two tracks: a move of one moves
    /// the other, and the group they are in is named by its first clip.
    film.clip.link {
        area: "film",
        layer: "film",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
            /// The clip it is linked to.
            with: NodeRef,
        },
        run: {
            let id = "film.clip.link".to_string();
            let clip = self.clip.0;
            let with = self.with.0;
            if clip == with {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip is not linked to itself".to_string(),
                });
            }
            for member in [clip, with] {
                let Some(node) = session.document().node(member) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no clip `{member}`"),
                    });
                };
                if node.kind != "film.clip" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{member}` is a `{}` and not a clip", node.kind),
                    });
                }
            }
            // The two clips must be in the same **sequence**: a link is a
            // relation between two things of one project.
            let sequence = crate::sequence_of(session, clip);
            if sequence != crate::sequence_of(session, with) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the two clips are not in the same sequence".to_string(),
                });
            }
            // The group is the **union** of the two groups, and its name
            // is the smallest id in it.
            let mut members = group(session, clip);
            for member in group(session, with) {
                if !members.contains(&member) {
                    members.push(member);
                }
            }
            let Some(name) = name_of(&members) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "there is nothing to link".to_string(),
                });
            };
            for member in &members {
                if let Some(node) = session.document_mut()?.node_mut(*member) {
                    node.data["link"] = serde_json::json!(name);
                }
            }
            Ok(Outcome::changed(format!("the clip `{clip}` is linked to the clip `{with}`"))
                .with_data(serde_json::json!({ "clip": clip, "with": with, "link": name, "group": members })))

        }
    }

    /// **Unlink** a clip from its group.
    ///
    /// A clip that is left alone in its group is unlinked too: a link
    /// with nothing to follow reads as if it were something.
    film.clip.unlink {
        area: "film",
        layer: "film",
        icon: "unlink",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
        },
        run: {
            let id = "film.clip.unlink".to_string();
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no clip `{clip}`"),
                });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            if link_of(&node.data).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` is not linked to anything"),
                });
            }
            let members = group(session, clip);
            let mut left: Vec<u64> = Vec::new();
            for member in &members {
                if *member != clip {
                    left.push(*member);
                }
            }
            // The clip goes, and a partner that is left alone goes too.
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data["link"] = serde_json::Value::Null;
            }
            let left = if left.len() == 1 {
                let lonely = left[0];
                if let Some(node) = session.document_mut()?.node_mut(lonely) {
                    node.data["link"] = serde_json::Value::Null;
                }
                Vec::new()
            } else {
                left
            };
            Ok(Outcome::changed(format!("the clip `{clip}` is not linked to anything"))
                .with_data(serde_json::json!({ "clip": clip, "group": members, "left": left })))

        }
    }
}
