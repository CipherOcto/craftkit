//! The **transitions** of a track.
//!
//! A transition is a thing that sits **over the cut** between two
//! clips: the tree's `Transition`, with a span on the timeline, a
//! duration and an alignment on the cut.
//!
//! The tree's rules, and they are the finding:
//!
//! - a transition joins **two clips that are next to each other**: the
//!   outgoing clip ends exactly where the incoming clip starts;
//! - it sits on the **cut** by its alignment: centred on it, starting
//!   at it, or ending at it;
//! - it may not **leave the clips it joins** — the tree's bounds are
//!   the outgoing clip's start and the incoming clip's end, and the
//!   tree does not check the material the clips have beyond the cut;
//! - it is a thing of the **track**, and not of either clip: it is
//!   removed when the clip it joins is gone, and a locked track
//!   refuses it.
//!
//! So a transition here is a **node** in the track, with a payload:
//! the two clips, the span, the duration, the alignment and the kind.
//!
//! And a **split** answers for them: a transition that follows the cut
//! joins the piece after it, and one that spans the cut keeps the piece
//! before it (the module `splitting`).

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::{is_locked, locked_error};

/// A **transition**.
#[derive(craftkit_derive::NodeKind)]
#[node_kind(kind = "film.transition", label = "Transition", layer = "film", evaluates)]
pub struct TransitionKind;

impl TransitionKind {
    /// A transition is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a transition: never.
    fn hit(_node: &Node, _document: &craftkit_sdk::Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a transition **means**: the two clips it joins and the span
    /// it covers.
    fn evaluate(node: &Node, _document: &craftkit_sdk::Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "from": node.data.get("from").and_then(|from| from.as_u64()),
            "to": node.data.get("to").and_then(|to| to.as_u64()),
            "start": node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0),
            "duration": node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0),
            "align": node.data.get("align").and_then(|align| align.as_str()).unwrap_or("center"),
            "effect": node.data.get("effect").and_then(|effect| effect.as_str()).unwrap_or("cross_dissolve"),
        }))
    }
}

/// Where a transition sits on its **cut**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    /// Centred on the cut.
    Center,
    /// Starting at the cut.
    Start,
    /// Ending at the cut.
    End,
}

impl Align {
    /// The alignment a name says, if it says one this tree knows.
    fn parse(name: &str) -> Option<Align> {
        match name.trim().to_ascii_lowercase().as_str() {
            "center" | "centre" => Some(Align::Center),
            "start" => Some(Align::Start),
            "end" => Some(Align::End),
            _ => None,
        }
    }

    /// The name of the alignment.
    fn name(self) -> &'static str {
        match self {
            Align::Center => "center",
            Align::Start => "start",
            Align::End => "end",
        }
    }
}

/// The only transition kind this tree has.
///
/// The tree has a **registry** of transitions and a cross dissolve is
/// its default for a video track. A registry is a thing of its own and
/// it is not here, so this tree has the one the tree's default names.
const CROSS_DISSOLVE: &str = "cross_dissolve";

/// The **span** of a clip: where it starts and how long it is.
fn span_of(session: &Session, clip: u64) -> Option<(u64, u64)> {
    let node = session.document().node(clip)?;
    if node.kind != "film.clip" {
        return None;
    }
    let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
    Some((start, duration))
}

/// The **cut** between two clips: where the outgoing clip ends.
fn the_cut(session: &Session, from: u64, to: u64) -> Option<u64> {
    let (from_start, from_duration) = span_of(session, from)?;
    let (to_start, _) = span_of(session, to)?;
    if from_start + from_duration != to_start {
        return None;
    }
    Some(to_start)
}

/// Where a transition of that duration **starts**, by its alignment.
///
/// The tree's rule: a centred transition is half its duration before
/// the cut, one that starts at the cut starts at it, and one that ends
/// at the cut is its whole duration before it.
fn start_for(align: Align, cut: u64, duration: u64) -> u64 {
    match align {
        Align::Center => cut.saturating_sub(duration / 2),
        Align::Start => cut,
        Align::End => cut.saturating_sub(duration),
    }
}

/// The **bounds** of a transition: the clips it joins are the bounds,
/// and the tree does not look at the material beyond the cut.
fn within_the_clips(start: u64, duration: u64, from_start: u64, to_end: u64) -> bool {
    start >= from_start && start.saturating_add(duration) <= to_end
}

/// **Add** a transition over the cut between two clips.
///
/// The two clips must be **next to each other** on the track: the
/// outgoing clip ends exactly where the incoming one starts. The
/// transition sits on the cut by its **alignment**, and it may not
/// leave the clips it joins.
///
/// A transition that is already over the same cut and joins the same
/// clip is **replaced**: the tree's `add_transition` removes it, so
/// that there is one transition over a cut and not two.
#[derive(Command)]
#[command(id = "film.transition.add", area = "film", layer = "film", icon = "plus", mutates, undoable)]
pub struct AddTransition {
    /// The track the transition is on.
    pub track: NodeRef,
    /// The outgoing clip.
    pub from: NodeRef,
    /// The incoming clip.
    pub to: NodeRef,
    /// How long the transition is, in frames.
    pub duration: Number,
    /// Where it sits on the cut: `center`, `start` or `end`. The
    /// default is `center`.
    pub align: Option<Text>,
    /// The kind of transition: `cross_dissolve`. The default is that.
    pub effect: Option<Text>,
}

impl AddTransition {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.transition.add".to_string();
        let track = self.track.0;
        let from = self.from.0;
        let to = self.to.0;
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "film.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        // A **locked** track cannot be edited, and a transition is an
        // edit.
        if is_locked(session, track) {
            return Err(locked_error(&id, track));
        }
        // The two clips must be on the track, and they must be **next
        // to each other**: a transition is over a cut.
        for clip in [from, to] {
            if !node.children.iter().any(|child| child.id == clip) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` is not on the track `{track}`"),
                });
            }
        }
        let Some(cut) = the_cut(session, from, to) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{from}` does not end where the clip `{to}` begins"),
            });
        };
        let duration = frames(self.duration, &id)?;
        if duration == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a transition is not zero frames long".to_string(),
            });
        }
        let align = match &self.align {
            Some(align) => Align::parse(&align.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not an alignment: a transition is `center`, `start` or `end`",
                    align.0.trim()
                ),
            })?,
            None => Align::Center,
        };
        let effect = match &self.effect {
            Some(effect) => {
                if effect.0.trim() != CROSS_DISSOLVE {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{}` is not a transition this tree has: the one it has is `{CROSS_DISSOLVE}`",
                            effect.0.trim()
                        ),
                    });
                }
                CROSS_DISSOLVE.to_string()
            }
            None => CROSS_DISSOLVE.to_string(),
        };
        let (from_start, _) = span_of(session, from).unwrap_or((0, 0));
        let (to_start, to_duration) = span_of(session, to).unwrap_or((0, 0));
        let start = start_for(align, cut, duration);
        if !within_the_clips(start, duration, from_start, to_start + to_duration) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "a transition of {duration} frames does not fit between the clip `{from}` and the clip `{to}`"
                ),
            });
        }
        // The tree's rule: a transition that is already over this cut
        // and joins one of these clips is replaced.
        let replaced: Vec<u64> = node
            .children
            .iter()
            .filter(|child| child.kind == "film.transition")
            .filter(|child| {
                let same_from = child.data.get("from").and_then(|value| value.as_u64()) == Some(from);
                let same_to = child.data.get("to").and_then(|value| value.as_u64()) == Some(to);
                same_from || same_to
            })
            .map(|child| child.id)
            .collect();
        for old in &replaced {
            session.document_mut()?.remove_node(*old);
        }
        let name = format!("transition at {start}");
        let data = serde_json::json!({
            "from": from,
            "to": to,
            "start": start,
            "duration": duration,
            "align": align.name(),
            "effect": effect,
        });
        let new =
            session.document_mut()?.add_node_with_under(Some(track), name, "film.transition", None, data);
        Ok(Outcome::changed(format!("added a {duration}-frame transition at {start}")).with_data(
            serde_json::json!({
                "id": new,
                "track": track,
                "from": from,
                "to": to,
                "start": start,
                "duration": duration,
                "align": align.name(),
                "effect": effect,
                "replaced": replaced,
            }),
        ))
    }
}

/// **Change** a transition: its duration and its alignment.
///
/// The start of the transition follows from its alignment, so a
/// transition that is centred on the cut is centred on it again, and
/// one that ends at the cut keeps its end there. The transition may
/// not leave the clips it joins.
#[derive(Command)]
#[command(id = "film.transition.set", area = "film", layer = "film", icon = "pencil", mutates, undoable)]
pub struct SetTransition {
    /// The transition.
    pub transition: NodeRef,
    /// How long it is, in frames. The default is the duration it has.
    pub duration: Option<Number>,
    /// Where it sits on the cut: `center`, `start` or `end`. The
    /// default is the alignment it has.
    pub align: Option<Text>,
}

impl SetTransition {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.transition.set".to_string();
        let transition = self.transition.0;
        let Some(node) = session.document().node(transition) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no transition `{transition}`"),
            });
        };
        if node.kind != "film.transition" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{transition}` is a `{}` and not a transition", node.kind),
            });
        }
        let track = crate::track_of(session, transition).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the transition `{transition}` is on no track"),
        })?;
        if is_locked(session, track) {
            return Err(locked_error(&id, track));
        }
        let from = node.data.get("from").and_then(|from| from.as_u64());
        let to = node.data.get("to").and_then(|to| to.as_u64());
        let (Some(from), Some(to)) = (from, to) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the transition `{transition}` joins no clips"),
            });
        };
        let duration = match &self.duration {
            Some(duration) => frames(*duration, &id)?,
            None => node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0),
        };
        if duration == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a transition is not zero frames long".to_string(),
            });
        }
        let align = match &self.align {
            Some(align) => Align::parse(&align.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not an alignment: a transition is `center`, `start` or `end`",
                    align.0.trim()
                ),
            })?,
            None => match node.data.get("align").and_then(|align| align.as_str()) {
                Some(name) => Align::parse(name).unwrap_or(Align::Center),
                None => Align::Center,
            },
        };
        let Some(cut) = the_cut(session, from, to) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{from}` does not end where the clip `{to}` begins"),
            });
        };
        let start = start_for(align, cut, duration);
        let (from_start, _) = span_of(session, from).unwrap_or((0, 0));
        let (to_start, to_duration) = span_of(session, to).unwrap_or((0, 0));
        if !within_the_clips(start, duration, from_start, to_start + to_duration) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "a transition of {duration} frames does not fit between the clip `{from}` and the clip `{to}`"
                ),
            });
        }
        let mut data = node.data.clone();
        data["start"] = serde_json::json!(start);
        data["duration"] = serde_json::json!(duration);
        data["align"] = serde_json::json!(align.name());
        if let Some(node) = session.document_mut()?.node_mut(transition) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the transition at {start} is {duration} frames long")).with_data(
            serde_json::json!({
                "transition": transition,
                "track": track,
                "start": start,
                "duration": duration,
                "align": align.name(),
            }),
        ))
    }
}

/// **Take away** a transition.
///
/// The clips it joins are not touched: a transition goes and the cut
/// is a cut again.
#[derive(Command)]
#[command(id = "film.transition.remove", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct RemoveTransition {
    /// The transition.
    pub transition: NodeRef,
}

impl RemoveTransition {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.transition.remove".to_string();
        let transition = self.transition.0;
        let Some(node) = session.document().node(transition) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no transition `{transition}`"),
            });
        };
        if node.kind != "film.transition" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{transition}` is a `{}` and not a transition", node.kind),
            });
        }
        let track = crate::track_of(session, transition);
        if let Some(track) = track
            && is_locked(session, track)
        {
            return Err(locked_error(&id, track));
        }
        session.document_mut()?.remove_node(transition);
        Ok(Outcome::changed(format!("took the transition {transition} away"))
            .with_data(serde_json::json!({ "transition": transition, "track": track })))
    }
}

/// A number of frames.
fn frames(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a number of frames is a whole number".to_string(),
        });
    }
    Ok(value.0 as u64)
}
