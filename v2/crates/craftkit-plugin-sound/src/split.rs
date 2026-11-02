//! The **split** of a track into **mono** tracks.
//!
//! A stereo track is two channels that travel together, and a track of
//! one channel is two things that can be heard apart. The tree's
//! editor splits a stereo track into a track for each of its channels,
//! and the two are panned to the sides so that the pair says what the
//! one said.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::{ClipPayload, SourcePayload, TrackPayload};

/// **Split** a track into **mono** tracks.
///
/// The tree's split makes one track for each channel of a stereo track,
/// and each of them plays **one channel** of the original's material:
/// the channel is written to a file of its own, and the clips of the
/// new track play that file.
///
/// The two new tracks are **panned to the sides** — the left one hard
/// left and the right one hard right — so that the pair says what the
/// one track said. The original track is left where it is: it is the
/// caller's to mute.
///
/// A track that is **not** stereo is **refused**: there is nothing to
/// split, and a command that quietly does nothing is a command that
/// answers a question nobody asked.
#[derive(Command)]
#[command(
    id = "sound.track.split.mono",
    area = "sound",
    layer = "sound",
    icon = "split",
    menu("Track", "Split into Mono"),
    mutates,
    undoable
)]
pub struct SplitMono {
    /// The track to split.
    pub track: NodeRef,
    /// The directory the mono files are written into.
    pub directory: Text,
}

impl SplitMono {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.split.mono".to_string();
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
        let payload = TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        let channels = payload.channels.max(1);
        if channels < 2 {
            return Err(EngineError::Failed {
                id,
                message: format!("the track `{}` has one channel and there is nothing to split", node.name),
            });
        }
        let directory = self.directory.0.clone();
        if directory.trim().is_empty() {
            return Err(EngineError::Failed { id, message: "a directory is a path".to_string() });
        }
        std::fs::create_dir_all(&directory).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the directory `{directory}` cannot be made: {error}"),
        })?;
        let name = node.name.clone();
        // The **clips** of the track: each of them is copied into every
        // new track, playing its own channel of the material.
        let document = session.document().clone();
        let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
        for clip in document.walk().filter(|node| node.kind == "sound.clip") {
            if crate::timeline::track_of(&document, clip).map(|owner| owner.id) != Some(track) {
                continue;
            }
            let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
            clips.push((clip.id, payload));
        }
        // The **channels**: one track for each, and the suffix that
        // names it.
        let mut made = Vec::with_capacity(channels as usize);
        for channel in 0..channels {
            let suffix = suffix_for(channels, channel);
            let track_name = format!("{name}.{suffix}");
            let new_track = crate::add_track(session, &track_name)?;
            // The track is **mono**, and it is panned to its side: the
            // pair says what the one track said.
            if let Some(node) = session.document_mut()?.node_mut(new_track) {
                let Ok(mut payload) = TrackPayload::from_json(&node.data) else { continue };
                payload.channels = 1;
                payload.pan = if channel == 0 { -1.0 } else { 1.0 };
                node.data = payload.to_json();
            }
            let playlist = crate::playlist_of(session, new_track, None)?;
            // The **clips**: copies of the track's clips, each playing
            // the channel of its own.
            for (clip_id, clip) in &clips {
                let mut clip = clip.clone();
                let mut source_id = clip.source;
                if let Some(source) = source_id {
                    let new_source = mono_source(session, source, channel, &suffix, &directory)?;
                    clip.source = Some(new_source);
                    source_id = Some(new_source);
                }
                let clip_name = format!("{}.{suffix}", clip_name_of(session, *clip_id));
                let new_clip = session.document_mut()?.add_node_with_under(
                    Some(playlist),
                    clip_name.clone(),
                    "sound.clip",
                    None,
                    clip.to_json(),
                );
                if let Some(source) = source_id {
                    session
                        .document_mut()?
                        .add_relation(craftkit_sdk::Relation::new(new_clip, source, "source"));
                }
            }
            made.push(serde_json::json!({ "track": new_track, "name": track_name, "channel": channel }));
        }
        Ok(Outcome::changed(format!("split the track `{name}` into {} mono tracks", made.len()))
            .with_data(serde_json::json!({ "tracks": made, "count": made.len() })))
    }
}

/// The **suffix** of a channel: "L" and "R" for a pair, and the
/// number of the channel for anything wider.
fn suffix_for(channels: u32, channel: u32) -> String {
    match (channels, channel) {
        (2, 0) => "L".to_string(),
        (2, 1) => "R".to_string(),
        _ => format!("{}", channel + 1),
    }
}

/// The **name** of a clip, if the document still has it.
fn clip_name_of(session: &Session, clip: u64) -> String {
    session.document().node(clip).map(|node| node.name.clone()).unwrap_or_else(|| format!("clip {clip}"))
}

/// The **mono** file of one channel of a source, written beside the
/// session.
///
/// The file holds the whole channel: the clips that play it keep their
/// own offsets, their stretches and their fades, and every reader sees
/// genuine mono media.
fn mono_source(
    session: &mut Session,
    source: u64,
    channel: u32,
    suffix: &str,
    directory: &str,
) -> Result<u64, EngineError> {
    let id = "sound.track.split.mono".to_string();
    let node = session.document().node(source).ok_or_else(|| EngineError::Failed {
        id: id.clone(),
        message: format!("there is no source `{source}`"),
    })?;
    let payload = SourcePayload::from_json(&node.data)
        .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
    let sound = crate::read_sound(&id, &payload.path)?;
    let width = sound.channels.max(1) as usize;
    let frames = sound.samples.len() / width;
    // The **channel**: the tree's renderer takes the source's channel
    // `c % n` for the track's channel `c`, and so does this.
    let mut mono: Vec<f32> = Vec::with_capacity(frames);
    for frame in 0..frames {
        mono.push(sound.samples[frame * width + (channel as usize % width)]);
    }
    let path = format!("{directory}/{}.{suffix}.wav", file_stem(&payload.path));
    let bytes = craftkit_wav::write(1, sound.rate, &mono);
    crate::write_file(&path, &bytes)?;
    // The **source** of the mono file, and the asset that carries it.
    let source_payload = SourcePayload {
        path: path.clone(),
        channels: 1,
        frames: frames as i64,
        sample_rate: sound.rate,
        peaks: Vec::new(),
    };
    let new_source =
        session.document_mut()?.add_node_with(path.clone(), "sound.source", None, source_payload.to_json());
    session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path, "audio", None));
    Ok(new_source)
}

/// The **stem** of a path: its name without the directory and without
/// the extension.
fn file_stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name.to_string(),
    }
}
