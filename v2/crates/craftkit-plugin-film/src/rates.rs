//! The **rate** of a sequence.
//!
//! A frame is a number, and the **rate** of a sequence says how many
//! frames there are in a second: 25, 24, 30, or the tree's rational
//! rates — `30000/1001` and the rest.
//!
//! A **sequence** is an **item** in this tree, and the rate belongs to
//! the timeline that hangs on it. A sequence with no track has an empty
//! timeline and the rate is still its rate.
//!
//! The rate is a **fact**, and the length of a sequence in **seconds**
//! is **derived**: it is the length in frames and the rate.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The tree's own rates: the name the tree shows, and the rational
/// behind it.
pub const RATES: [(&str, i64, i64); 11] = [
    ("23.976", 24000, 1001),
    ("24", 24, 1),
    ("25", 25, 1),
    ("29.97", 30000, 1001),
    ("30", 30, 1),
    ("48", 48, 1),
    ("50", 50, 1),
    ("59.94", 60000, 1001),
    ("60", 60, 1),
    ("119.88", 120000, 1001),
    ("120", 120, 1),
];

/// The most the **numerator** of a rate may be.
const MAX_NUMERATOR: i64 = 1_000_000;

/// The most the **denominator** of a rate may be.
const MAX_DENOMINATOR: i64 = 10_000;

/// The rate a text says, if it says one this tree has.
///
/// The tree's own **names** are the ones it shows — `25`, `23.976` —
/// and a rate may also be written as the fraction behind it:
/// `30000/1001`.
pub fn rate_of(text: &str) -> Result<(i64, i64), String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("a rate is not nothing".to_string());
    }
    if let Some((_, num, den)) = RATES.iter().find(|(name, _, _)| *name == text) {
        return Ok((*num, *den));
    }
    let not_a_rate = || {
        format!(
            "`{text}` is not a rate: a rate is a number of frames in a second, such as `25`, or a fraction such as `30000/1001`"
        )
    };
    let (num, den) = match text.split_once('/') {
        Some((num, den)) => (num.trim(), den.trim()),
        None => (text, "1"),
    };
    let Ok(num) = num.parse::<i64>() else { return Err(not_a_rate()) };
    let Ok(den) = den.parse::<i64>() else { return Err(not_a_rate()) };
    if num <= 0 || den <= 0 {
        return Err(format!("a rate is a positive number of frames in a second, and `{text}` is not"));
    }
    if num > MAX_NUMERATOR || den > MAX_DENOMINATOR {
        return Err(format!("a rate is at most {MAX_NUMERATOR} over {MAX_DENOMINATOR}, and `{text}` is not"));
    }
    Ok((num, den))
}

/// The rate a **number of frames a second** says.
///
/// The tree's own rates are the ones it has, and a rate that is near
/// one of them is that one — `23.976` is `24000/1001`. Any other
/// rate is worked out as a fraction of a thousand, because a rate
/// that is not a whole number of frames a second cannot be one
/// exactly.
pub fn rate_from_float(fps: f64) -> (i64, i64) {
    if !fps.is_finite() || fps <= 0.0 {
        return (25, 1);
    }
    for (_, num, den) in RATES {
        if ((num as f64 / den as f64) - fps).abs() < 0.005 {
            return (num, den);
        }
    }
    ((fps * 1000.0).round() as i64, 1000)
}

/// The **length** of a sequence in frames: the end of the longest of
/// its tracks.
///
/// It is **derived**: the tracks change when a clip moves, and the
/// length of the sequence changes with them.
fn length_in_frames(sequence: &craftkit_sdk::Node) -> u64 {
    sequence
        .children
        .iter()
        .filter(|child| child.kind == "film.track")
        .map(|track| {
            track
                .children
                .iter()
                .filter(|child| child.kind == "film.clip")
                .filter_map(|child| {
                    let start = child.data.get("start").and_then(|start| start.as_u64())?;
                    let duration = child.data.get("duration").and_then(|duration| duration.as_u64())?;
                    Some(start + duration)
                })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0)
}

/// The **sequence** a node is, if it is one.
///
/// A sequence is an **item** in this tree, and the item that holds the
/// tracks of a timeline is the sequence of them.
fn the_sequence<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no sequence `{node}`"),
        });
    };
    if found.kind != "film.item" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}`, and a sequence is an item", found.kind),
        });
    }
    Ok(found)
}

/// **Set** the rate of a sequence.
///
/// The rate is a number of frames in a second, and it may be the
/// tree's own name — `25`, `23.976` — or the fraction behind it,
/// `30000/1001`.
#[derive(Command)]
#[command(id = "film.sequence.rate.set", area = "film", layer = "film", icon = "clock", mutates, undoable)]
pub struct SetRate {
    /// The sequence.
    pub sequence: NodeRef,
    /// The rate: `25`, `23.976` or `30000/1001`.
    pub rate: Text,
}

impl SetRate {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.sequence.rate.set".to_string();
        let sequence = self.sequence.0;
        the_sequence(session, &id, sequence)?;
        let (num, den) =
            rate_of(&self.rate.0).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let rate = json!({ "num": num, "den": den });
        let Some(node) = session.document_mut()?.node_mut(sequence) else {
            return Err(EngineError::Failed { id, message: format!("there is no sequence `{sequence}`") });
        };
        node.data["rate"] = rate.clone();
        Ok(Outcome::changed(format!("the sequence `{sequence}` runs at {num}/{den}"))
            .with_data(json!({ "sequence": sequence, "rate": rate })))
    }
}

/// **Read** the rate of a sequence.
///
/// The answer is the rate, the length of the sequence in **frames** and
/// its length in **seconds**. The lengths are **derived**, and a
/// sequence with no rate has no length in seconds.
#[derive(Command)]
#[command(id = "film.sequence.rate.get", area = "film", layer = "film", icon = "list")]
pub struct GetRate {
    /// The sequence.
    pub sequence: NodeRef,
}

impl GetRate {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.sequence.rate.get".to_string();
        let sequence = self.sequence.0;
        let node = the_sequence(session, &id, sequence)?;
        let rate = node.data.get("rate").cloned().unwrap_or(Json::Null);
        let frames = length_in_frames(node);
        // The length in **seconds**: the frames and the rate.
        let seconds = match rate
            .get("num")
            .and_then(|num| num.as_i64())
            .zip(rate.get("den").and_then(|den| den.as_i64()))
        {
            Some((num, den)) if num > 0 => Json::from(frames as f64 * den as f64 / num as f64),
            _ => Json::Null,
        };
        Ok(Outcome::new(format!("the sequence `{sequence}`")).with_data(json!({
            "sequence": sequence,
            "rate": rate,
            "frames": frames,
            "seconds": seconds,
        })))
    }
}
