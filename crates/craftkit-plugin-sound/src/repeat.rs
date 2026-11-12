//! The **repeating** of a range: the material of a range, again.
//!
//! The tree's editor repeats a selection: it copies the material of
//! the range and lays the copies after it, as many times as the caller
//! asks for.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

/// The most times a range may be repeated.
///
/// The tree's repeat is a number from one to nine hundred and
/// ninety-nine, and the bound is here too: a repeat of ten thousand
/// times is a track of ten thousand times the material.
const MAX_REPEATS: u32 = 999;

commands! {
    /// **Repeat** the material of a range of a track.
    ///
    /// The clips of the track that overlap the range are **copied** — each
    /// copy trimmed to the range and moved to the range's own head — and
    /// the copies are laid after the range, as many times as the caller
    /// asks for.
    ///
    /// The copies keep everything the clips had: their gain, their fades,
    /// their stretch, their pitch and their processes. A copy that is
    /// trimmed at its head loses its **fade in** — the fade was of the
    /// material before it — and a copy that is trimmed at its tail loses
    /// its **fade out**.
    ///
    /// The copies are laid **over** what is there, as the loop's copies
    /// are (§ the loop of a clip): the tree's paste clears what it covers,
    /// and this one does not, because a document that throws away
    /// material nobody asked it to throw away is a document that surprises.
    sound.track.repeat {
        area: "sound",
        layer: "sound",
        icon: "loop",
        mutates,
        undoable,
        menu: ("Track", "Repeat"),
        params: {
            /// The track whose material is repeated.

            track: NodeRef,
            /// Where the range starts, in samples.

            start: Option<Number>,
            /// Where the range ends, in samples.

            end: Option<Number>,
            /// How many times the range is repeated.
            ///
            /// One is none. The number is **clamped** into its bounds: it is a
            /// knob and not a fact about the material.

            count: Option<Number>,
        },
        run: {

            let id = "sound.track.repeat".to_string();
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
            // The **count**: how many times, and it is a knob.
            let count = match self.count {
                Some(count) if count.0.is_finite() => (count.0 as u32).clamp(1, MAX_REPEATS),
                Some(_) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "the count is a number of times".to_string(),
                    });
                }
                None => 1,
            };
            let (start, end) = crate::range_of(session, track, &id, self.start, self.end)?;
            let length = end - start;
            // The **copies**: each clip of the range, trimmed to the range
            // and moved to the range's head.
            let copies = crate::copies_of_range(session, track, start, end);
            if copies.is_empty() {
                return Ok(Outcome::new("there is no material in the range to repeat")
                    .with_data(serde_json::json!({ "repeats": 0, "clips": Vec::<u64>::new() })));
            }
            let playlist = crate::playlist_of(session, track, None)?;
            // The **repeats**: the copies are laid after the range, again
            // and again.
            let mut made = Vec::new();
            for step in 1..=count {
                let at = start + length * step as i64;
                for (name, copy) in &copies {
                    let mut placed = copy.clone();
                    placed.start = at + copy.start;
                    let placed_id = session.document_mut()?.add_node_with_under(
                        Some(playlist),
                        name.clone(),
                        "sound.clip",
                        None,
                        placed.to_json(),
                    );
                    // The **source** relation goes with the copy: a copy
                    // that plays nothing is a copy of nothing.
                    if let Some(source) = placed.source {
                        session
                            .document_mut()?
                            .add_relation(craftkit_sdk::Relation::new(placed_id, source, "source"));
                    }
                    made.push(placed_id);
                }
            }
            Ok(Outcome::changed(format!("repeated the range {count} time(s)"))
                .with_data(serde_json::json!({ "repeats": count, "clips": made })))

        }
    }
}
