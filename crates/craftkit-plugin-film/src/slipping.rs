//! The **slip** of a clip.
//!
//! A clip has two clocks — where it starts on the timeline and where it
//! begins in the media — and a **slip** moves the second one and leaves
//! the first one alone. The clip stays where it is on the timeline and
//! how long it is, and it shows a **different part of its media**.
//!
//! It is the third way of changing a clip, and the other two are not
//! like it: a **trim** moves an edge and changes how long the clip is,
//! and a **move** changes where it is and not what it shows. A slip
//! changes **what it shows** and neither of the others.
//!
//! The tree **clamps** a slip: it works out the biggest delta the slip
//! may have, uses that, and the editor never says that it did. This
//! tree **refuses**: a slip that would run past the end of the media
//! is an error that says how long the media is, and the clip is as it
//! was.
//!
//! The bounds are two:
//!
//! - a clip may not begin **before the media** it plays;
//! - a clip may not run **past the end** of it — and the media a clip
//!   plays is its length multiplied by its **speed** (the module
//!   `speed`).
//!
//! And the **link** is here, and the slip honours it: the tree slips
//! the linked clips **together**, so that the picture and the sound
//! stay in step, and this tree does the same (the module `linking`).
//!
//! # What the port found
//!
//! - **The tree's slip moves the media whatever the speed is.** The
//!   tree's `slip` adds the delta to the clip's place in the media and
//!   does not count the speed through it, while its own **trim** does.
//!   So a clip that plays twice as fast slips twice as far in the
//!   media as a clip that plays as it is, and the picture it shows
//!   moves half as far. Here the slip counts the media through the
//!   speed, like the trim does: a slip of ten frames shows ten frames
//!   of picture later, whatever the speed is.
//! - **The tree's linked slip is scaled and this one is refused.** The
//!   tree's `slip_together` divides the delta of the group by the
//!   fraction the most constrained member could take, so the whole
//!   group slips a little and nothing says so. Here every member of
//!   the group takes the same delta, and a member that cannot take it
//!   refuses the slip (the module `linking`).
//!
//! # What it does not do
//!
//! - **The roll and the slide are not here.** The tree's **roll**
//!   moves the cut between two clips and the tree's **slide** moves a
//!   clip between its neighbours, and both are edits of the **place**
//!   of a clip and not of its content. They are commands of their own.
//! - **The tree's subclip restriction is not here.** The tree's slip
//!   may not leave the range of a subclip. Here the media may carry
//!   **marks** and the slip stays between them (the module
//!   `mediamarks`), but a subclip is not here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

commands! {
    /// **Slip** a clip: show a different part of its media.
    ///
    /// The clip stays where it is on the timeline and how long it is, and
    /// its place in the media moves.
    ///
    /// The slip is refused when it would run past the end of the media or
    /// begin before it: the tree clamps such a slip and says nothing, and
    /// here the refusal says how long the media is.
    film.clip.slip {
        area: "film",
        layer: "film",
        icon: "move",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
            /// How far the clip slips, in frames of the timeline. A negative
            /// number slips it the other way.
            delta: Number,
        },
        run: {
            let id = "film.clip.slip".to_string();
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            // A **locked** track cannot be edited.
            if let Some(track) = crate::track_of(session, clip)
                && crate::is_locked(session, track)
            {
                return Err(crate::locked_error(&id, track));
            }
            let delta = self.delta.0;
            if !delta.is_finite() || delta.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a number of frames is a whole number".to_string(),
                });
            }
            let delta = delta as i64;
            if delta == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a slip of zero frames is not a slip".to_string(),
                });
            }
            // The **link** is here: the tree slips the linked clips
            // together, so that the picture and the sound stay in step.
            let members = crate::linking::group(session, clip);
            let mut planned: Vec<(u64, u64)> = Vec::new();
            let mut skipped: Vec<u64> = Vec::new();
            for member in &members {
                // A partner on a **locked** track stays where it is: the
                // tree's rule, and the rule of the move as well.
                if *member != clip
                    && let Some(track) = crate::track_of(session, *member)
                    && crate::is_locked(session, track)
                {
                    skipped.push(*member);
                    continue;
                }
                let source_in = slipped(session, *member, delta).map_err(|message| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{member}`: {message}"),
                })?;
                planned.push((*member, source_in));
            }
            for (member, source_in) in &planned {
                if let Some(node) = session.document_mut()?.node_mut(*member) {
                    node.data["source_in"] = serde_json::json!(*source_in);
                }
            }
            let source_in =
                planned.iter().find(|(member, _)| *member == clip).map(|(_, source_in)| *source_in).unwrap_or(0);
            let partners: Vec<u64> = members.iter().copied().filter(|member| *member != clip).collect();
            Ok(Outcome::changed(format!("the clip `{clip}` is slipped by {delta} frames")).with_data(
                serde_json::json!({
                    "id": clip,
                    "source_in": source_in,
                    "group": members,
                    "partners": partners,
                    "skipped": skipped,
                }),
            ))

        }
    }
}

/// The **new place in the media** of a clip slipped by a delta.
///
/// The answer is the place, or the **reason** the slip cannot be made.
fn slipped(session: &Session, clip: u64, delta: i64) -> Result<u64, String> {
    let Some(node) = session.document().node(clip) else {
        return Err(format!("there is no clip `{clip}`"));
    };
    let Some(source_in) = node.data.get("source_in").and_then(|source_in| source_in.as_u64()) else {
        return Err(format!("the clip `{clip}` does not say where it begins in the media"));
    };
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    // The **speed**: the media a frame of the clip plays is the
    // speed's worth of the media, so a slip of ten frames moves the
    // media by ten frames' worth of it.
    let speed = crate::speed::speed_of(&node.data);
    // The **frame hold**: the frame of the media the clip holds to the
    // end of it. A held clip plays its material up to that frame, so
    // the material is shorter than the clip.
    let hold = crate::hold::hold_of(&node.data);
    let shift = (delta as f64 * speed).round() as i64;
    let moved = source_in as i64 + shift;
    // The **media** the clip plays: the range of its frames a clip may
    // play, which is between its marks when it has them.
    let media = crate::media_range_of(session, clip);
    let media_start = media.map(|media| media.start as i64).unwrap_or(0);
    let media_marked = media.map(|media| media.marked).unwrap_or(false);
    if moved < media_start {
        return Err(if media_marked {
            format!("the clip would begin at frame {moved}, and the media begins at frame {media_start}")
        } else {
            format!("the clip would begin at frame {} before the media begins", -moved)
        });
    }
    let moved = moved as u64;
    match hold {
        // A **held** clip plays its material up to the frame it holds
        // and holds that frame to the end of the clip, so the material
        // is the part between the place in the media and the hold.
        Some(at) => {
            if moved >= at {
                return Err(format!(
                    "the clip would begin at frame {moved} of the media, and the frame it holds is at frame {at}"
                ));
            }
            // The material the clip plays before the hold has to fit
            // in the clip: the hold is at the end of the clip. The
            // material is counted in the media's own frames, and so is
            // what the clip plays.
            let material = at - moved;
            let played = crate::speed::media_frames(duration, speed);
            if material > played {
                return Err(format!(
                    "the clip would play {material} frames of the media before the frame it holds, and the clip plays {played} frames of the media"
                ));
            }
        }
        None => {
            let played = crate::speed::media_frames(duration, speed);
            if let Some(media) = media
                && moved + played > media.end
            {
                return Err(format!(
                    "the clip would run to frame {} of the media, and {}",
                    moved + played,
                    media.bound()
                ));
            }
        }
    }
    Ok(moved)
}
