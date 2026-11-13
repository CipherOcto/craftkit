//! The **loudness** of a clip: how loud it is to a listener.
//!
//! The tree's own meter: the **ITU-R BS.1770** loudness, as **EBU R128**
//! asks for it, with the **true peak** beside it.
//!
//! # What was built
//!
//! - The **meter** is the tree's own: the K-weighting of BS.1770, the
//!   blocks of a tenth of a second, and the **gate** that decides which
//!   of them count towards the integrated loudness.
//! - The three loudnesses are said:
//!   - the **momentary** loudness — the last four hundred milliseconds;
//!   - the **short-term** loudness — the last three seconds;
//!   - the **integrated** loudness — every block that is above the
//!     gate.
//! - The **true peak** is said beside them: the highest point of the
//!   sound **between** its samples, which is found by looking at the
//!   sound **four times over**.
//!
//! # What it does not settle
//!
//! - The **loudness range** of the tree — the spread between the quiet
//!   and the loud parts of a whole programme — is not here: it is a
//!   measure of a programme and not of a clip.
//! - The tree's meter also has a **peak meter** and an **RMS meter**
//!   for a display; this one measures loudness and nothing else.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{NodeRef, Outcome};

commands! {
    /// The **loudness** of a clip.
    ///
    /// The loudness is **derived**: it is measured in the sound of the clip
    /// every time it is asked for.
    sound.clip.loudness.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
            /// The clip to measure.
            clip: NodeRef,
        },
        run: {
            let id = "sound.clip.loudness.read".to_string();
            let clip_id = self.clip.0;
            let (channels, rate) = crate::planar_of(&id, session, clip_id)?;
            let mut meter = craftkit_audio::LoudnessMeter::new(rate as f32, channels.len().max(1));
            meter.process(&channels);
            Ok(Outcome::new(format!("the clip `{clip_id}` is measured")).with_data(json!({
                "clip": clip_id,
                "rate": rate,
                "channels": channels.len(),
                "momentary_lufs": meter.momentary_lufs(),
                "short_term_lufs": meter.short_term_lufs(),
                "integrated_lufs": meter.integrated_lufs(),
                "max_momentary_lufs": meter.max_momentary_lufs(),
                "true_peak_db": meter.true_peak_db(),
            })))

        }
    }
}
