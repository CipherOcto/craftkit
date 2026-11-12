//! The **render** of a clip: the sound of a clip, written to a file.
//!
//! The tree's editor exports the clips as files: it renders the track
//! over the clip's range and writes what it hears to a WAV file named
//! after the clip. This tree's battery could render the **mix** and
//! the **stems**, and not one clip of them.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// **Render** a clip: the sound of it, written to a file.
///
/// The sound is the sound of the **track** over the clip's range: the
/// clips of the track, with their gains and their fades, and not the
/// track's trim, not the track's inserts and not the track's fader —
/// the same sound the tree's export writes, and the same sound the
/// consolidate writes (§ the consolidate of a track).
///
/// A **MIDI clip** is refused: it holds notes and not a sound, and a
/// file of notes is not a sound.
#[derive(Command)]
#[command(
    id = "sound.clip.render",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Track", "Render Clip"),
    mutates,
    undoable
)]
pub struct RenderClip {
    /// The clip to render.
    pub clip: NodeRef,
    /// The directory the file is written into.
    pub directory: Text,
}

impl RenderClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.render".to_string();
        let clip = self.clip.0;
        let document = session.document().clone();
        let Some(node) = document.node(clip) else {
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
        let payload = crate::ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if payload.source.is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` holds no sound: it holds notes"),
            });
        }
        let Some(track) = crate::timeline::track_of(&document, node) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` is on no track"),
            });
        };
        let directory = self.directory.0.clone();
        if directory.trim().is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a directory is a path".to_string() });
        }
        std::fs::create_dir_all(&directory).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the directory `{directory}` cannot be made: {error}"),
        })?;
        // The **sound** of the track over the clip's range.
        let mix = crate::mix_of(&document, &id, Some(track.id))?;
        let channels = mix.channels.max(1) as usize;
        let Some((_, signal)) = mix.raw else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the track `{}` has no sound to render", track.id),
            });
        };
        let start = payload.start.max(0) as usize * channels;
        let end = (payload.start.max(0) + payload.length.max(0)) as usize * channels;
        if signal.len() < end {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sound of the track is {} frames long and the clip asks for {}",
                    signal.len() / channels,
                    payload.start.max(0) + payload.length.max(0)
                ),
            });
        }
        // The **file**: the sound of the clip, named after it.
        let name = node.name.clone();
        let path = crate::a_stem_path(&directory, &name);
        let bytes = craftkit_wav::write(mix.channels, mix.rate, &signal[start..end]);
        crate::write_file(&path, &bytes)?;
        Ok(Outcome::changed(format!("rendered the clip `{clip}` into `{path}`"))
            .with_data(json!({ "clip": clip, "path": path, "track": track.id })))
    }
}
