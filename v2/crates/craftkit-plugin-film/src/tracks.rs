//! The **settings** of a track.
//!
//! A track is more than a line of clips: it may be **locked** (which
//! it already may be), it may be **enabled** or not, it may be
//! **muted**, it may be **solo**, it may have a **volume** and a
//! **pan**, and it may carry a **label** — the tree's own sixteen
//! colours, the same list the clips and the items carry.
//!
//! The tree's rules, and they are the finding:
//!
//! - the tree's **pan** is a number between minus a hundred and a
//!   hundred, and a pan outside it is **clamped** and says nothing.
//!   Here it is **refused**, and the refusal says which bound stopped
//!   it — a pan that is not the pan that was asked for is not a pan.
//! - the tree's **volume** is in **decibels** and may be any number.
//! - a **label** may be taken off with an empty name and with the
//!   name `None`, and a name that is not on the tree's list is
//!   refused.
//!
//! # What it does not do
//!
//! - **The tree's solo is not honoured here**: a solo is a fact of the
//!   track and the playing of the tracks is the host's business.
//! - **The tree's sync lock is not here**: a track that is locked with
//!   another is a fact this tree does not keep.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

/// The **pan** of a track: between minus a hundred and a hundred.
const MAX_PAN: f64 = 100.0;

/// The **track** a node is, if it is one.
fn the_track<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no track `{node}`"),
        });
    };
    if found.kind != "film.track" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a track", found.kind),
        });
    }
    Ok(found)
}

/// **Set** a **flag** of a track.
fn set_flag(
    session: &mut Session,
    id: &str,
    track: u64,
    key: &str,
    on: bool,
) -> Result<Outcome, EngineError> {
    the_track(session, id, track)?;
    let Some(node) = session.document_mut()?.node_mut(track) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no track `{track}`"),
        });
    };
    node.data[key] = serde_json::json!(on);
    Ok(Outcome::changed(format!("the track `{track}` is {}", if on { key } else { "not" }))
        .with_data(serde_json::json!({ "track": track, key: on })))
}

/// **Enable** a track, or **disable** it.
///
/// A disabled track is not played; its clips stay where they are.
#[derive(Command)]
#[command(id = "film.track.enable", area = "film", layer = "film", icon = "check", mutates, undoable)]
pub struct EnableTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether the track is enabled.
    pub on: Bool,
}

impl EnableTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "film.track.enable", self.track.0, "enabled", self.on.0)
    }
}

/// **Mute** a track, or **unmute** it.
#[derive(Command)]
#[command(id = "film.track.mute", area = "film", layer = "film", icon = "mute", mutates, undoable)]
pub struct MuteTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether the track is muted.
    pub on: Bool,
}

impl MuteTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "film.track.mute", self.track.0, "muted", self.on.0)
    }
}

/// **Solo** a track, or **unsolo** it.
#[derive(Command)]
#[command(id = "film.track.solo", area = "film", layer = "film", icon = "headphones", mutates, undoable)]
pub struct SoloTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether the track is soloed.
    pub on: Bool,
}

impl SoloTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        set_flag(session, "film.track.solo", self.track.0, "solo", self.on.0)
    }
}

/// **Set** the **volume** of a track, in decibels.
#[derive(Command)]
#[command(id = "film.track.volume", area = "film", layer = "film", icon = "speaker", mutates, undoable)]
pub struct SetVolume {
    /// The track.
    pub track: NodeRef,
    /// The volume, in decibels.
    pub db: Number,
}

impl SetVolume {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.volume".to_string();
        let track = self.track.0;
        the_track(session, &id, track)?;
        if !self.db.0.is_finite() {
            return Err(EngineError::Failed { id, message: "a volume in decibels is a number".to_string() });
        }
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "film.track.volume".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data["volume_db"] = serde_json::json!(self.db.0);
        Ok(Outcome::changed(format!("the track `{track}` is at {} decibels", self.db.0))
            .with_data(serde_json::json!({ "track": track, "db": self.db.0 })))
    }
}

/// **Set** the **pan** of a track.
///
/// The pan is a number between minus a hundred and a hundred, and a
/// pan outside it is **refused** — the tree clamps it and says
/// nothing.
#[derive(Command)]
#[command(id = "film.track.pan", area = "film", layer = "film", icon = "pan", mutates, undoable)]
pub struct SetPan {
    /// The track.
    pub track: NodeRef,
    /// The pan: between minus a hundred and a hundred.
    pub pan: Number,
}

impl SetPan {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.pan".to_string();
        let track = self.track.0;
        the_track(session, &id, track)?;
        let pan = self.pan.0;
        if !pan.is_finite() {
            return Err(EngineError::Failed { id, message: "a pan is a number".to_string() });
        }
        if pan < -MAX_PAN || pan > MAX_PAN {
            return Err(EngineError::Failed {
                id,
                message: format!("a pan is between -{MAX_PAN} and {MAX_PAN}, and {pan} is not"),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "film.track.pan".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data["pan"] = serde_json::json!(pan);
        Ok(Outcome::changed(format!("the track `{track}` is panned to {pan}"))
            .with_data(serde_json::json!({ "track": track, "pan": pan })))
    }
}

/// **Put** a **label** on a track, or **take it off**.
#[derive(Command)]
#[command(id = "film.track.label", area = "film", layer = "film", icon = "paint", mutates, undoable)]
pub struct SetTrackLabel {
    /// The track.
    pub track: NodeRef,
    /// The label: `Violet`, `Iris`, … — or nothing to take it off.
    pub label: Text,
}

impl SetTrackLabel {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.label".to_string();
        let track = self.track.0;
        the_track(session, &id, track)?;
        let wanted = self.label.0.trim().to_string();
        let label = if wanted.is_empty() || wanted.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(crate::labels::label_of(&wanted).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no label `{wanted}`: the labels are {}",
                    crate::labels::LABELS.join(", ")
                ),
            })?)
        };
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "film.track.label".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data["label"] = match label {
            Some(label) => serde_json::json!(label),
            None => serde_json::Value::Null,
        };
        Ok(Outcome::changed(format!("the track `{track}` is labelled"))
            .with_data(serde_json::json!({ "track": track, "label": label })))
    }
}

/// **Rename** a track.
#[derive(Command)]
#[command(id = "film.track.rename", area = "film", layer = "film", icon = "pencil", mutates, undoable)]
pub struct RenameTrack {
    /// The track.
    pub track: NodeRef,
    /// The name.
    pub name: Text,
}

impl RenameTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.rename".to_string();
        let track = self.track.0;
        the_track(session, &id, track)?;
        let name = self.name.0.clone();
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "film.track.rename".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.name = name.clone();
        Ok(Outcome::changed(format!("the track `{track}` is `{name}`"))
            .with_data(serde_json::json!({ "track": track, "name": name })))
    }
}
