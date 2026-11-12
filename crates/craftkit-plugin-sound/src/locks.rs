//! The **locks** on a clip: what may not be changed, and what may not
//! move.
//!
//! The tree's sound editor has two locks on a clip, and they are two
//! different claims:
//!
//! - the **edit lock**: the sound inside the clip is finished — the
//!   clip may not be trimmed, faded or stretched;
//! - the **time lock**: the clip stays where it is — it may not be
//!   spaced or snapped.
//!
//! And both of them keep the clip from **moving**, because a clip that
//! moves is a clip that changed.
//!
//! The tree keeps the locks as two flags on the clip and honours them
//! in a few places. Here they are two flags on the clip's **payload** —
//! the clip says what may be done to it — and the commands that would
//! do the forbidden thing **ask**.
//!
//! # What the kit's own lock is not
//!
//! The kit has a lock of its own (§115): `node.lock` locks a **node**,
//! and a locked node cannot be moved, resized or deleted. That is a
//! lock on the node, and a sound clip has no place to lock — the sound
//! kinds have none — so the kit's lock cannot say what the tree's
//! locks say. The clip's locks are the battery's own.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session};

use crate::ClipPayload;

commands! {
    /// Lock a clip's **content**, or take the lock off.
    ///
    /// The clip's sound is finished: it may not be trimmed, faded or
    /// stretched.
    ///
    /// The clip does not **move** either: the tree's editor holds a locked
    /// clip still — the tree's own word for it is that a locked clip
    /// "doesn't move" — and the two locks are two different claims about
    /// the same clip, and both of them keep it where it is.
    ///
    /// Without a lock given, the lock is **toggled**: a clip that is
    /// locked is unlocked, and a clip that is not is locked.
    sound.clip.lock.edit {
        area: "sound",
        layer: "sound",
        icon: "lock",
        mutates,
        undoable,
        menu: ("Clip", "Edit Lock"),
        params: {
            /// The clip to lock.

            clip: NodeRef,
            /// Whether the clip is locked. Without it, the lock is toggled.

            locked: Option<Bool>,
        },
        run: {

            let locked = self.locked.map(|locked| locked.0);
            set_lock(session, self.clip.0, Lock::Edit, locked)

        }
    }
    /// Lock a clip's **place**, or take the lock off.
    ///
    /// The clip stays where it is: it may not be spaced or snapped. The
    /// clip's sound may still be **changed** — the edit lock is a
    /// different lock, and a clip may carry both.
    ///
    /// Without a lock given, the lock is **toggled**.
    sound.clip.lock.time {
        area: "sound",
        layer: "sound",
        icon: "lock",
        mutates,
        undoable,
        menu: ("Clip", "Time Lock"),
        params: {
            /// The clip to lock.

            clip: NodeRef,
            /// Whether the clip is locked. Without it, the lock is toggled.

            locked: Option<Bool>,
        },
        run: {

            let locked = self.locked.map(|locked| locked.0);
            set_lock(session, self.clip.0, Lock::Time, locked)

        }
    }
}

/// Which of the two locks a command is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lock {
    /// The clip's **content**: the sound inside it.
    Edit,
    /// The clip's **place**: where it sits on the timeline.
    Time,
}

impl Lock {
    /// The name of the lock, for a person.
    fn name(self) -> &'static str {
        match self {
            Lock::Edit => "edit",
            Lock::Time => "time",
        }
    }
}

/// Set a lock on a clip, or take it off.
///
/// A lock that is not named is **toggled**: the clip says whether it
/// has the lock, and the opposite is what is asked for.
pub(crate) fn set_lock(
    session: &mut Session,
    clip: u64,
    lock: Lock,
    locked: Option<bool>,
) -> Result<Outcome, EngineError> {
    let id = match lock {
        Lock::Edit => "sound.clip.lock.edit".to_string(),
        Lock::Time => "sound.clip.lock.time".to_string(),
    };
    let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
        id: id.clone(),
        message: format!("there is no node `{clip}`"),
    })?;
    if node.kind != "sound.clip" {
        return Err(EngineError::Failed {
            id: id.clone(),
            message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
        });
    }
    let mut payload = ClipPayload::from_json(&node.data)
        .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
    let has = match lock {
        Lock::Edit => payload.edit_locked,
        Lock::Time => payload.time_locked,
    };
    let locked = locked.unwrap_or(!has);
    match lock {
        Lock::Edit => payload.edit_locked = locked,
        Lock::Time => payload.time_locked = locked,
    }
    if let Some(node) = session.document_mut()?.node_mut(clip) {
        node.data = payload.to_json();
    }
    Ok(Outcome::changed(if locked {
        format!("the clip `{clip}` has the {} lock", lock.name())
    } else {
        format!("the clip `{clip}` has no {} lock", lock.name())
    })
    .with_data(serde_json::json!({
        "id": clip,
        "lock": lock.name(),
        "locked": locked,
    })))
}

/// Whether a clip's **content** may not be changed.
///
/// A clip whose sound is finished is not trimmed, not faded and not
/// stretched: the sound inside it is what it is.
pub(crate) fn edit_is_locked(payload: &ClipPayload) -> bool {
    payload.edit_locked
}

/// Whether a clip's **place** may not change.
///
/// A clip that stays where it is is not spaced and not snapped, and a
/// clip whose content is locked does not move either: a clip that
/// moves is a clip that changed.
pub(crate) fn time_is_locked(payload: &ClipPayload) -> bool {
    payload.time_locked || payload.edit_locked
}
