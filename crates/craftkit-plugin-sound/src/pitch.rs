//! The **pitch** of a clip: the notes its sound sings.
//!
//! The tree's own command, in the tree's own way: the sound of the
//! clip is read, it is mixed down to **one channel**, and the notes are
//! **heard** in it.
//!
//! # What was built
//!
//! - The **detector** is the tree's own: **YIN**, the cumulative mean
//!   normalised difference of de Cheveigné and Kawahara (2002), and the
//!   segmentation of the frames into **runs** of one note.
//! - The notes are said in **seconds**, because the tree says them in
//!   samples and this tree's timeline is in seconds.
//!
//! # What it does not settle
//!
//! - The detection is **monophonic**: a sound that sings two notes at
//!   once is heard as one of them.
//! - The tree copies the notes to the **clipboard** as a MIDI clip.
//!   Here the notes are **said**, and a MIDI clip is made by the
//!   commands that make MIDI clips.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **lowest** pitch the detector looks for, in hertz.
const MIN_HZ: f64 = 20.0;

/// The **highest** pitch the detector looks for, in hertz.
const MAX_HZ: f64 = 5_000.0;

/// The **pitch** of the notes of a clip.
///
/// The notes are **derived**: they are heard in the sound of the clip
/// every time they are asked for.
#[derive(Command)]
#[command(id = "sound.clip.pitch.read", area = "sound", layer = "sound", icon = "list")]
pub struct ReadPitch {
    /// The clip to listen to.
    pub clip: NodeRef,
    /// The lowest note to look for, in hertz.
    pub min_hz: Option<Number>,
    /// The highest note to look for, in hertz.
    pub max_hz: Option<Number>,
}

impl ReadPitch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.pitch.read".to_string();
        let clip_id = self.clip.0;
        let (channels, rate) = crate::planar_of(&id, session, clip_id)?;
        // The **bounds** of the search: the tree's own are fifty hertz
        // to sixteen hundred, and a number outside is refused.
        let min_hz = self.min_hz.map(|value| value.0).unwrap_or(50.0);
        let max_hz = self.max_hz.map(|value| value.0).unwrap_or(1600.0);
        if !(min_hz.is_finite() && max_hz.is_finite()) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the bounds of the search are numbers".to_string(),
            });
        }
        if min_hz < MIN_HZ || max_hz > MAX_HZ || max_hz <= min_hz {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the search runs from {MIN_HZ} hertz to {MAX_HZ} hertz, and {min_hz} to {max_hz} is not a range inside it"
                ),
            });
        }
        // The sound in **one channel**: the tree renders the range to
        // mono.
        let mono: Vec<f32> = if channels.is_empty() {
            Vec::new()
        } else {
            let count = channels.iter().map(Vec::len).min().unwrap_or(0);
            let mut out: Vec<f32> = Vec::with_capacity(count);
            for frame in 0..count {
                let mut sum = 0.0f32;
                for lane in &channels {
                    sum += lane.get(frame).copied().unwrap_or(0.0);
                }
                out.push(sum / channels.len() as f32);
            }
            out
        };
        let notes = craftkit_audio::audio_to_notes(&mono, rate as f32);
        let rate = rate.max(1) as f64;
        let notes: Vec<craftkit_sdk::serde_json::Value> = notes
            .iter()
            .filter(|note| {
                let hz = 440.0 * 2.0f64.powf((note.pitch as f64 - 69.0) / 12.0);
                hz >= min_hz && hz <= max_hz
            })
            .map(|note| {
                json!({
                    "start": note.start as f64 / rate,
                    "end": note.end as f64 / rate,
                    "pitch": note.pitch,
                    "velocity": note.velocity,
                })
            })
            .collect();
        Ok(Outcome::new(format!("the clip `{clip_id}` sings {} note(s)", notes.len())).with_data(json!({
            "clip": clip_id,
            "rate": rate,
            "notes": notes,
        })))
    }
}
