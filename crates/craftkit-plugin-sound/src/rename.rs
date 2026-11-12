//! The **name** of a track and of a clip.
//!
//! The tree's editor renames a track and renames a clip. The name is
//! what a person calls the thing, and it is not the thing's identity:
//! in this tree the **id** is the identity and the name is a label, so
//! two tracks may have the same name — the tree refuses it, and the
//! section says why.
//!
//! A name that is **empty** is refused, and so is a name that is
//! **longer than the tree's bound**: the tree takes the first
//! two hundred and fifty-six characters of a long name and throws the
//! rest away, and a name that was cut in half is a name that is not
//! the name that was asked for.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

/// The most characters a name may have.
///
/// The tree's bound is its own, and a name that is longer is a name
/// with a mistake in it.
const MAX_NAME_CHARS: usize = 256;

/// The name of a node, ready to be put on it.
///
/// The name is **trimmed** — a name of spaces is no name — and an
/// empty one is refused, and so is one that is longer than the bound.
fn a_name(id: &str, name: &str) -> Result<String, EngineError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a name is not an empty name".to_string(),
        });
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a name is at most {MAX_NAME_CHARS} characters long"),
        });
    }
    Ok(name.to_string())
}

commands! {
    /// **Rename** a track.
    ///
    /// The name is a label and not the track's identity: the track's id
    /// does not change, and two tracks may have the same name.
    sound.track.rename {
        area: "sound",
        layer: "sound",
        icon: "pencil",
        mutates,
        undoable,
        menu: ("Track", "Rename"),
        params: {
            /// The track to rename.

            track: NodeRef,
            /// Its new name.

            name: Text,
        },
        run: {

            let id = "sound.track.rename".to_string();
            let track = self.track.0;
            let Some(node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            let name = a_name(&id, &self.name.0)?;
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            node.name = name.clone();
            Ok(Outcome::changed(format!("the track `{track}` is named `{name}`"))
                .with_data(json!({ "id": track, "name": name })))

        }
    }
    /// **Rename** a clip.
    ///
    /// A clip that is in an **auto rename scheme** keeps its place in it:
    /// the scheme names the pieces of a cut, and a clip that is renamed by
    /// hand is still a clip of the scheme.
    sound.clip.rename {
        area: "sound",
        layer: "sound",
        icon: "pencil",
        mutates,
        undoable,
        menu: ("Track", "Rename Clip"),
        params: {
            /// The clip to rename.

            clip: NodeRef,
            /// Its new name.

            name: Text,
        },
        run: {

            let id = "sound.clip.rename".to_string();
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let name = a_name(&id, &self.name.0)?;
            let Some(node) = session.document_mut()?.node_mut(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            node.name = name.clone();
            Ok(Outcome::changed(format!("the clip `{clip}` is named `{name}`"))
                .with_data(json!({ "id": clip, "name": name })))

        }
    }
}
