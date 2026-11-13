//! The **marks** on a clip: what a person has said about it.
//!
//! The tree's sound editor puts two marks on a clip: a **rating** —
//! stars, from none to five — and a **colour**, a tint for the clip's
//! bar on the timeline.
//!
//! Only one of the two is a mark in this tree, and the other one was
//! already here.
//!
//! **The rating is the clip's own.** The tree keeps it as a number on
//! the clip, from none to five, and its keyboard sets it with the
//! number keys. Here it is a field in the clip's **payload** and a
//! command sets it — the same shape as the clip's beat and the clip's
//! locks, because those are the clip's own facts too.
//!
//! **The colour is the kit's.** The kit already says how every node is
//! drawn: an **appearance** is the colour a node is filled with, and
//! `core.appearance.set` sets one on **any** node — a path, a
//! rectangle and a sound clip alike. The tree's clip colour is that
//! same fact under a narrower name, and a command of the battery's own
//! to set it would be **two paths to one thing**: the tree's way for a
//! clip and the kit's way for everything else.
//!
//! # The tree clamps a rating and this tree refuses one
//!
//! The tree's command takes any number and **clamps** it into the
//! range: a rating of nine becomes five, and a rating of minus one
//! becomes none. Its keyboard refuses the same thing — a press of the
//! nine key does not reach the command at all — so the tree has two
//! answers to one question, and which one you get depends on how you
//! asked.
//!
//! Here there is one answer, and it is the **refusal**: a rating is a
//! number of stars from none to five, and a number of nine stars is not
//! a rating. It is refused, and the clip keeps the rating it had —
//! because a rating that was changed without being asked for is a
//! rating the caller did not want (§ the tree's own rule: a refusal
//! never truncates).

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

use crate::ClipPayload;

/// The most stars a clip may be given.
///
/// A rating is a number of stars, and the tree's own range is none to
/// five.
pub(crate) const MAX_RATING: u8 = 5;

commands! {
    /// Set the **rating** of a clip.
    ///
    /// The rating is a number of stars, from none (zero) to five. A rating
    /// outside that range is **refused** and the clip keeps the rating it
    /// had: the tree's command clamps such a rating into the range, and a
    /// rating asked for as nine stars is not a rating.
    ///
    /// A rating of **zero** is a rating — the clip has none — and not the
    /// absence of one, so a clip's rating is always there.
    sound.clip.rating {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Rating"),
        params: {
            /// The clip to rate.
            clip: NodeRef,
            /// The rating, from none (zero) to five stars.
            rating: Number,
        },
        run: {
            let id = "sound.clip.rating".to_string();
            let clip = self.clip.0;
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
            // The rating: a whole number of stars, and no more than five.
            let asked = self.rating.0;
            if !asked.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a rating is a number of stars".to_string(),
                });
            }
            if asked.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a rating is a whole number of stars, and {asked} is not"),
                });
            }
            if asked < 0.0 || asked > MAX_RATING as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a rating is from none to {MAX_RATING} stars, and {asked} is not a rating"),
                });
            }
            let rating = asked as u8;
            let mut payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            payload.rating = rating;
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = payload.to_json();
            }
            Ok(Outcome::changed(format!("the clip `{clip}` is rated {rating}"))
                .with_data(serde_json::json!({ "id": clip, "rating": rating })))

        }
    }
}
