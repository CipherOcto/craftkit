//! The **commit** of a track: its sound written to a file, and a new
//! track of that file.
//!
//! The tree's commit renders the sound of a track, writes it to a file,
//! makes a **new track** of that file, and turns the **voice** of the
//! old track off. The editing is over: the old track is still there,
//! with its clips and its settings, and it is quiet.
//!
//! # What the sound is
//!
//! The sound of the commit is the sound of the track **before its
//! fader**: the clips, through the track's **trim** and through the
//! track's **inserts**, and not through its **mute**, not through its
//! **fader** and not through its **pan**. The new track carries the
//! old track's fader and pan, so the sound of the document is the
//! sound it was — the level is not counted twice.
//!
//! # What the new track carries
//!
//! The tree's commit copies the **mixer** of the old track onto the
//! new one: the **volume**, the **output** and the **sends**. Here the
//! new track carries the old track's **fader**, its **pan** and its
//! **sends**, and it **feeds the same bus** when the old track fed
//! one.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::mixer::SendPayload;

/// **Commit** a track: write its sound to a file and make a new track
/// of it.
///
/// The old track is not taken away: its **voice** is turned off, so it
/// is quiet and it is still in the document with its clips and its
/// settings.
///
/// A track with **no sound** is refused, and so is a track whose voice
/// is already off: there is nothing to write down.
#[derive(Command)]
#[command(
    id = "sound.track.commit",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Track", "Commit"),
    mutates,
    undoable
)]
pub struct CommitTrack {
    /// The track to commit.
    pub track: NodeRef,
    /// The directory the file is written into.
    pub directory: Text,
}

impl CommitTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.commit".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if crate::is_master(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the master holds the fader of the mix and not a sound of its own".to_string(),
            });
        }
        let payload = crate::TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if payload.inactive {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the track `{track}` is inactive: its voice is off and it has no sound"),
            });
        }
        let directory = self.directory.0.clone();
        if directory.trim().is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a directory is a path".to_string() });
        }
        std::fs::create_dir_all(&directory).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the directory `{directory}` cannot be made: {error}"),
        })?;
        // The **material** of the track: its clips say where it ends,
        // and a track with no clips has no sound to commit.
        let document = session.document().clone();
        let mut end = 0_i64;
        for clip in document.walk().filter(|node| node.kind == "sound.clip") {
            if crate::timeline::track_of(&document, clip).map(|owner| owner.id) != Some(track) {
                continue;
            }
            let Ok(clip) = crate::ClipPayload::from_json(&clip.data) else { continue };
            end = end.max(clip.start.max(0) + clip.length.max(0));
        }
        if end <= 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the track `{track}` has no sound to commit"),
            });
        }
        // The **sound**: the track before its fader.
        let mix = crate::mix_of(&document, &id, Some(track))?;
        let channels = mix.channels.max(1) as usize;
        let Some((_, signal)) = mix.pre_fader else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the track `{track}` has no sound to commit"),
            });
        };
        let to = end as usize * channels;
        if signal.len() < to {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sound of the track is {} frames long and its clips ask for {end}",
                    signal.len() / channels
                ),
            });
        }
        // The **file**: the sound of the track, written into the
        // directory.
        let name = format!("{}.cm", node.name);
        let path = crate::a_stem_path(&directory, &name);
        let bytes = craftkit_wav::write(mix.channels, mix.rate, &signal[..to]);
        crate::write_file(&path, &bytes)?;
        // The **source** of the file, and the **asset** that carries it.
        let frames = end;
        let source_payload = crate::SourcePayload {
            path: path.clone(),
            channels: mix.channels as u32,
            frames,
            sample_rate: mix.rate,
            peaks: Vec::new(),
        };
        let source = session.document_mut()?.add_node_with(
            path.clone(),
            "sound.source",
            None,
            source_payload.to_json(),
        );
        session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path.clone(), "audio", None));
        // The **new track**: the old track's fader and pan, and a clip
        // of the file.
        let new_payload = crate::TrackPayload {
            track_kind: "audio".to_string(),
            channels: mix.channels as u32,
            solo: false,
            trim_db: 0.0,
            fader_db: payload.fader_db,
            pan: payload.pan,
            mute: false,
            solo_safe: payload.solo_safe,
            phase_invert: false,
            active_playlist: None,
            inactive: false,
            elastic: None,
            surround: None,
            record_arm: false,
            input_monitor: false,
        };
        let new_track =
            session.document_mut()?.add_node_with(name.clone(), "sound.track", None, new_payload.to_json());
        let playlist = session.document_mut()?.add_node_with_under(
            Some(new_track),
            "A".to_string(),
            "sound.playlist",
            None,
            craftkit_sdk::serde_json::Value::Null,
        );
        let mut clip = crate::ClipPayload::new();
        clip.start = 0;
        clip.length = frames;
        clip.source = Some(source);
        let clip_id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            name.clone(),
            "sound.clip",
            None,
            clip.to_json(),
        );
        session.document_mut()?.add_relation(craftkit_sdk::Relation::new(clip_id, source, "source"));
        // The **sends** of the old track: the new track sends to the
        // same buses, at the same levels.
        let mut sends = 0_usize;
        for send in crate::sends_of(&document) {
            if send.track != track {
                continue;
            }
            let send_payload = SendPayload {
                level_db: send.level_db,
                pan: send.pan,
                mute: send.mute,
                pre_fader: send.pre_fader,
            };
            let send_id = session.document_mut()?.add_node_with_under(
                Some(new_track),
                format!("send to {}", send.bus),
                "sound.send",
                None,
                send_payload.to_json(),
            );
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(send_id, send.bus, "send"));
            sends += 1;
        }
        // The **route**: a track that feeds a bus feeds it after the
        // commit too, or the new track would be heard in the mix and
        // the bus would hear nothing.
        let route = crate::mixer::route_of(&document, track);
        if let Some(bus) = route {
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(new_track, bus, "route"));
        }
        // The **old track** is quiet: its voice is off.
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{track}`"),
            });
        };
        let mut old = crate::TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        old.inactive = true;
        node.data = old.to_json();
        // The **place** of the new track: where the old one was, so
        // that the mix is in the same order as it was.
        crate::place_after(session, new_track, track);
        Ok(Outcome::changed(format!("committed the track `{track}` into `{path}`")).with_data(json!({
            "track": track,
            "committed": new_track,
            "clip": clip_id,
            "source": source,
            "path": path,
            "frames": frames,
            "sends": sends,
            "bus": route,
        })))
    }
}
