//! The **surround pan** of a track.
//!
//! A surround pan says where a sound is **in the room**: how far to the
//! left or the right, how far back or forward, and how high. It is a
//! position and not a balance — the balance says how much of the sound
//! is on the left, and the surround pan says where the sound is.
//!
//! The tree's finding is that the position is a **puck** in a square
//! room, and that the puck's angle is **warped**: the square's corners
//! land on the front speakers and on the layout's rear-most surrounds,
//! so a puck in a corner is exactly on one speaker and not between two.
//!
//! The **gains** are the tree's: the power of the sound on each
//! speaker. The gains of the speakers that are not the LFE always add
//! up to one, so the sound is not quieter when it is spread.
//!
//! # What it does not do
//!
//! - **The mix does not use the gains yet.** The gains are derived
//!   here and reported, and the mix still balances the track between
//!   the two sides.
//! - **The tree's thirty-four layouts are five here.** The tree has a
//!   layout for every surround format there is, and the **Ambisonics**
//!   are not here at all. Here there are `stereo`, `5.0`, `5.1`,
//!   `7.1` and `7.1.4`, and a layout that is not one of them is
//!   refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

use crate::TrackPayload;

/// A **speaker**: where it is, and whether it is the LFE.
///
/// The **azimuth** is in degrees — zero at the front, and to the right
/// positive — and the **elevation** is in degrees above the ear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Speaker {
    /// Where it is around the room.
    pub azimuth: f64,
    /// How high it is.
    pub elevation: f64,
    /// Whether it is the LFE, which is never panned to.
    pub lfe: bool,
}

/// The **layouts** this tree has, and the speakers of each.
///
/// The tree's own numbers: the front speakers are at thirty degrees,
/// the surrounds of a five at a hundred and ten, the surrounds of a
/// seven at a hundred and fifty, and the height speakers at
/// forty-five degrees up.
pub const LAYOUTS: [(&str, &[Speaker]); 5] = [
    (
        "stereo",
        &[
            Speaker { azimuth: -30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 30.0, elevation: 0.0, lfe: false },
        ],
    ),
    (
        "5.0",
        &[
            Speaker { azimuth: -30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: -110.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 110.0, elevation: 0.0, lfe: false },
        ],
    ),
    (
        "5.1",
        &[
            Speaker { azimuth: -30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: true },
            Speaker { azimuth: -110.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 110.0, elevation: 0.0, lfe: false },
        ],
    ),
    (
        "7.1",
        &[
            Speaker { azimuth: -30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: true },
            Speaker { azimuth: -150.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 150.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: -90.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 90.0, elevation: 0.0, lfe: false },
        ],
    ),
    (
        "7.1.4",
        &[
            Speaker { azimuth: -30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 30.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 0.0, elevation: 0.0, lfe: true },
            Speaker { azimuth: -150.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 150.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: -90.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: 90.0, elevation: 0.0, lfe: false },
            Speaker { azimuth: -45.0, elevation: 45.0, lfe: false },
            Speaker { azimuth: 45.0, elevation: 45.0, lfe: false },
            Speaker { azimuth: -90.0, elevation: 45.0, lfe: false },
            Speaker { azimuth: 90.0, elevation: 45.0, lfe: false },
        ],
    ),
];

/// The speakers of a layout, if this tree has that layout.
pub fn layout(name: &str) -> Option<&'static [Speaker]> {
    let name = name.trim();
    LAYOUTS.iter().find(|(written, _)| *written == name).map(|(_, speakers)| *speakers)
}

/// The **names** of the layouts this tree has.
pub fn layout_names() -> Vec<&'static str> {
    LAYOUTS.iter().map(|(name, _)| *name).collect()
}

/// The **surround pan** of a track: where its sound is in the room.
///
/// The tree's fields, and the tree's rules: the **x** is left to
/// right, the **y** is back to front, the **z** is ear level to
/// overhead, the **divergence** spreads the sound from a point source
/// to every speaker, and the **centre** is the share of the centre
/// speaker's sound that stays on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurroundPan {
    /// How far to the left or the right: minus one to one.
    pub x: f64,
    /// How far back or forward: minus one at the back, one at the
    /// front.
    pub y: f64,
    /// How high: zero at the ear, one overhead.
    pub z: f64,
    /// How spread out the sound is: zero is a point source and one is
    /// every speaker.
    pub divergence: f64,
    /// The share of the centre speaker's sound that stays on it, as a
    /// percentage: the rest goes to a phantom centre on the left and
    /// the right.
    pub center: f64,
}

impl SurroundPan {
    /// The surround pan a track starts from: the middle of the room,
    /// in front, at the ear.
    pub fn middle() -> SurroundPan {
        SurroundPan { x: 0.0, y: 1.0, z: 0.0, divergence: 0.0, center: 100.0 }
    }

    /// The surround pan as it is written.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "x": self.x,
            "y": self.y,
            "z": self.z,
            "divergence": self.divergence,
            "center": self.center,
        })
    }

    /// The surround pan as it is read.
    pub fn from_json(value: &serde_json::Value) -> Option<SurroundPan> {
        let number = |key: &str| value.get(key).and_then(|number| number.as_f64());
        Some(SurroundPan {
            x: number("x")?,
            y: number("y")?,
            z: number("z")?,
            divergence: number("divergence")?,
            center: number("center")?,
        })
    }
}

/// The **azimuth** of a puck in the square room, in degrees.
///
/// The square's front corners land on the front speakers, its side
/// midpoints at ninety degrees and its rear corners on the layout's
/// rear-most surrounds — so a puck in a corner is exactly on one
/// speaker.
pub fn puck_azimuth(layout: &[Speaker], x: f64, y: f64) -> f64 {
    let x = if x.is_finite() { x.clamp(-1.0, 1.0) } else { 0.0 };
    let y = if y.is_finite() { y.clamp(-1.0, 1.0) } else { 1.0 };
    if x.abs() < 1e-9 && y.abs() < 1e-9 {
        return 0.0;
    }
    let rear = layout
        .iter()
        .filter(|speaker| !speaker.lfe && speaker.elevation.abs() < 1.0 && speaker.azimuth.abs() > 90.5)
        .map(|speaker| speaker.azimuth.abs())
        .fold(0.0_f64, f64::max);
    let rear = if rear > 0.0 { rear } else { 135.0 };
    let front = if layout.iter().any(|speaker| {
        !speaker.lfe && speaker.elevation.abs() < 1.0 && (speaker.azimuth.abs() - 30.0).abs() < 0.5
    }) {
        30.0
    } else {
        45.0
    };
    let phi = x.atan2(y).to_degrees();
    let angle = phi.abs();
    let warped = if angle <= 45.0 {
        angle / 45.0 * front
    } else if angle <= 90.0 {
        front + (angle - 45.0) / 45.0 * (90.0 - front)
    } else if angle <= 135.0 {
        90.0 + (angle - 90.0) / 45.0 * (rear - 90.0)
    } else {
        rear + (angle - 135.0) / 45.0 * (180.0 - rear)
    };
    warped.copysign(phi)
}

/// The **power** of a direction on one layer of the layout.
///
/// The power is added into the speakers: the squared gain, so that the
/// gains of the speakers that are not the LFE add up to one.
fn pair_power(layout: &[Speaker], height: bool, azimuth: f64, weight: f64, power: &mut [f64]) -> bool {
    let ids: Vec<usize> = layout
        .iter()
        .enumerate()
        .filter(|(_, speaker)| !speaker.lfe && (speaker.elevation > 1.0) == height)
        .map(|(index, _)| index)
        .collect();
    match ids.as_slice() {
        [] => false,
        [only] => {
            if let Some(power) = power.get_mut(*only) {
                *power += weight;
            }
            true
        }
        [first, ..] => {
            let azimuth = wrap180(azimuth);
            // The nearest speaker **clockwise** — to the right — and
            // the nearest one **anticlockwise**.
            let mut clockwise = *first;
            let mut clockwise_distance = f64::MAX;
            let mut anticlockwise = *first;
            let mut anticlockwise_distance = f64::MAX;
            for index in &ids {
                let speaker = layout.get(*index).map(|speaker| speaker.azimuth).unwrap_or(0.0);
                let distance = (speaker - azimuth).rem_euclid(360.0);
                if distance < clockwise_distance {
                    clockwise_distance = distance;
                    clockwise = *index;
                }
                let distance = (azimuth - speaker).rem_euclid(360.0);
                if distance < anticlockwise_distance {
                    anticlockwise_distance = distance;
                    anticlockwise = *index;
                }
            }
            if clockwise == anticlockwise || clockwise_distance < 1e-4 {
                if let Some(power) = power.get_mut(clockwise) {
                    *power += weight;
                }
                return true;
            }
            let span = (clockwise_distance + anticlockwise_distance).max(1e-6);
            let t = (anticlockwise_distance / span).clamp(0.0, 1.0) * std::f64::consts::FRAC_PI_2;
            if let Some(power) = power.get_mut(anticlockwise) {
                *power += weight * t.cos() * t.cos();
            }
            if let Some(power) = power.get_mut(clockwise) {
                *power += weight * t.sin() * t.sin();
            }
            true
        }
    }
}

/// The **gains** of a surround pan on a layout.
///
/// The gains are the power of the sound on each speaker, and the LFE
/// gets none: the caller sends to it separately.
pub fn surround_gains(layout: &[Speaker], pan: &SurroundPan) -> Vec<f64> {
    let mut gains = vec![0.0_f64; layout.len()];
    if layout.is_empty() {
        return gains;
    }
    let azimuth = puck_azimuth(layout, pan.x, pan.y);
    // How far the puck is from the middle of the room: the sound
    // there is more of a point source.
    let distance = pan.x.abs().max(pan.y.abs()).clamp(0.0, 1.0);
    let divergence = if pan.divergence.is_finite() { pan.divergence.clamp(0.0, 1.0) } else { 0.0 };
    let spread = 1.0 - distance * (1.0 - divergence);
    // The sound is spread over **point sources** across the spread,
    // and their powers are averaged.
    let count = 1 + (spread * 24.0).round() as usize;
    let width = spread * 180.0;
    let has_height = layout.iter().any(|speaker| !speaker.lfe && speaker.elevation > 1.0);
    let z = if has_height { if pan.z.is_finite() { pan.z.clamp(0.0, 1.0) } else { 0.0 } } else { 0.0 };
    let (bed, top) = {
        let angle = z * std::f64::consts::FRAC_PI_2;
        (angle.cos() * angle.cos(), angle.sin() * angle.sin())
    };
    let mut power = vec![0.0_f64; layout.len()];
    for index in 0..count {
        let here = if count == 1 {
            azimuth
        } else {
            azimuth - width + 2.0 * width * (index as f64 + 0.5) / count as f64
        };
        let weight = 1.0 / count as f64;
        if bed > 1e-9 && !pair_power(layout, false, here, weight * bed, &mut power) {
            pair_power(layout, true, here, weight * bed, &mut power);
        }
        if top > 1e-9 {
            pair_power(layout, true, here, weight * top, &mut power);
        }
    }
    // The **centre**: the share that stays on the centre speaker, and
    // the rest that goes to a phantom centre on the left and the
    // right.
    let find = |azimuth: f64| {
        layout.iter().position(|speaker| {
            !speaker.lfe && speaker.elevation.abs() < 1.0 && (speaker.azimuth - azimuth).abs() < 0.5
        })
    };
    let share = if pan.center.is_finite() { (pan.center / 100.0).clamp(0.0, 1.0) } else { 1.0 };
    if let (Some(centre), Some(left), Some(right)) = (find(0.0), find(-30.0), find(30.0))
        && share < 1.0
    {
        let centre_power = power.get(centre).copied().unwrap_or(0.0);
        let moved = centre_power * (1.0 - share);
        if let Some(slot) = power.get_mut(centre) {
            *slot = centre_power - moved;
        }
        for index in [left, right] {
            if let Some(slot) = power.get_mut(index) {
                *slot += 0.5 * moved;
            }
        }
    }
    for (gain, power) in gains.iter_mut().zip(power.iter()) {
        *gain = power.max(0.0).sqrt();
    }
    gains
}

/// An angle in the range minus one hundred and eighty to one hundred
/// and eighty.
fn wrap180(angle: f64) -> f64 {
    let mut angle = if angle.is_finite() { angle % 360.0 } else { 0.0 };
    if angle > 180.0 {
        angle -= 360.0;
    } else if angle <= -180.0 {
        angle += 360.0;
    }
    angle
}

commands! {
    /// lie about where the sound is.
    sound.track.surround {
        area: "sound",
        layer: "sound",
        icon: "sliders",
        mutates,
        undoable,
        menu: ("Track", "Surround"),
        params: {
            /// The track to set.
            track: NodeRef,
            /// How far to the left or the right: from minus one to one.
            x: Option<Number>,
            /// How far back or forward: from minus one at the back to one at
            /// the front.
            y: Option<Number>,
            /// How high: from zero at the ear to one overhead.
            z: Option<Number>,
            /// How spread out the sound is: from zero, a point source, to one,
            /// every speaker.
            divergence: Option<Number>,
            /// The share of the centre speaker's sound that stays on it, as a
            /// percentage: the rest goes to a phantom centre on the left and
            /// the right.
            center: Option<Number>,
        },
        run: {
            let id = "sound.track.surround".to_string();
            let track = self.track.0;
            let Some(node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            let mut payload = TrackPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            let mut pan = payload.surround.unwrap_or(SurroundPan::middle());
            if let Some(x) = &self.x {
                pan.x = within(x.0, -1.0, 1.0, "a position to the left or the right", &id)?;
            }
            if let Some(y) = &self.y {
                pan.y = within(y.0, -1.0, 1.0, "a position back or forward", &id)?;
            }
            if let Some(z) = &self.z {
                pan.z = within(z.0, 0.0, 1.0, "a height", &id)?;
            }
            if let Some(divergence) = &self.divergence {
                pan.divergence =
                    within(divergence.0, 0.0, 1.0, "a spread from a point source to every speaker", &id)?;
            }
            if let Some(center) = &self.center {
                pan.center = within(center.0, 0.0, 100.0, "a share of the centre speaker's sound", &id)?;
            }
            payload.surround = Some(pan);
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            node.data = data;
            // The **gains**: what the pan says about each speaker of the
            // session's layout. They are derived, and they are not stored.
            let layout = crate::session_of(session.document()).layout.clone();
            let speakers = layout_speakers(&layout);
            let gains = surround_gains(speakers, &pan);
            Ok(Outcome::changed(format!("the track `{track}` is in the room")).with_data(serde_json::json!({
                "id": track,
                "surround": pan.to_json(),
                "layout": layout,
                "gains": gains,
            })))

        }
    }

    /// Take a track's **surround pan** away.
    ///
    /// A track with no surround pan is balanced, and a balance is not a
    /// position: the track's sound is in the middle of the room.
    sound.track.surround.delete {
        area: "sound",
        layer: "sound",
        icon: "minus",
        mutates,
        undoable,
        menu: ("Track", "Surround"),
        params: {
            /// The track to take it from.
            track: NodeRef,
        },
        run: {
            let id = "sound.track.surround.delete".to_string();
            let track = self.track.0;
            let Some(node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            let mut payload = TrackPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            payload.surround = None;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            node.data = data;
            Ok(Outcome::changed(format!("the track `{track}` is in the middle of the room"))
                .with_data(serde_json::json!({ "id": track })))

        }
    }
}

/// A number within a range, or the reason it is not.
fn within(value: f64, low: f64, high: f64, what: &str, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() || value < low || value > high {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a number between {low} and {high}"),
        });
    }
    Ok(value)
}

/// The speakers of a layout, if this tree has that layout.
fn layout_speakers(name: &str) -> &'static [Speaker] {
    layout(name).unwrap_or(&[])
}
