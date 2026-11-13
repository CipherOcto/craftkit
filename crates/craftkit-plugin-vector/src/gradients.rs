//! The **stops** of a gradient.
//!
//! A gradient is a list of **stops**, and a stop says where it is
//! along the gradient — its **offset**, from zero to one — and what
//! **colour** it is there, and how **solid** it is.
//!
//! The stops are a **list in order**: a stop at a smaller offset comes
//! before a stop at a bigger one, and that is the order the gradient
//! is read in. A gradient has **two stops or more**: one colour is a
//! fill and not a gradient.
//!
//! # What the port found
//!
//! - **The tree's stops have a midpoint and a swatch of their own**:
//!   a stop is a colour that may be shared with another stop, and the
//!   colour it shows is at a place along it. Here a stop is an
//!   **offset, a colour and an opacity**, and it is its own: there is
//!   no shared swatch and no midpoint, so there is nothing to keep in
//!   step with anything else.
//! - **The tree edits a gradient in place**: the stops are picked and
//!   moved and recoloured, and the gradient is the same gradient
//!   afterwards. Here the same: a stop is added, changed or taken away
//!   and the path keeps its gradient.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{
    Colour, EngineError, Fill, Gradient, GradientKind, GradientStop, NodeRef, Number, Outcome, Session, Text,
};

use crate::{parse_colour, vector_of, write_back};

/// The **gradient** a path is filled with.
fn gradient_of(session: &Session, id: &str, path: u64) -> Result<Gradient, EngineError> {
    let vector = vector_of(session, path)?;
    match vector.fill {
        Fill::Gradient(gradient) => Ok(gradient),
        Fill::Solid(_) => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the path `{path}` is filled with one colour and not with a gradient"),
        }),
    }
}

/// Put a **gradient** back into a path.
fn write_gradient(session: &mut Session, id: &str, path: u64, gradient: Gradient) -> Result<(), EngineError> {
    let mut vector = vector_of(session, path)?;
    vector.fill = Fill::Gradient(gradient);
    write_back(session, path, &vector)
        .map_err(|reason| EngineError::Failed { id: id.to_string(), message: reason.to_string() })
}

/// The **offset** of a stop: between zero and one.
fn offset_of(id: &str, offset: Number) -> Result<f64, EngineError> {
    let offset = offset.0;
    if !offset.is_finite() || !(0.0..=1.0).contains(&offset) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a stop's offset is between zero and one, and this one is {offset}"),
        });
    }
    Ok(offset)
}

/// The **opacity** of a stop: from zero to one.
fn opacity_of(id: &str, opacity: Option<Number>) -> Result<f64, EngineError> {
    match opacity {
        None => Ok(1.0),
        Some(opacity) => {
            let opacity = opacity.0;
            if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!("a stop's opacity is between zero and one, and this one is {opacity}"),
                });
            }
            Ok(opacity)
        }
    }
}

/// The **index** of a stop.
fn index_of(id: &str, stop: Number) -> Result<usize, EngineError> {
    let stop = stop.0;
    if !stop.is_finite() || stop < 0.0 || stop.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a stop is a whole number".to_string(),
        });
    }
    Ok(stop as usize)
}

/// The stops, in order.
fn stops_of(gradient: &Gradient) -> Vec<GradientStop> {
    gradient.stops.clone()
}

/// A colour, written as `#rrggbb`.
fn colour_hex(colour: Colour) -> String {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(colour.0[0]), byte(colour.0[1]), byte(colour.0[2]))
}

/// The **kind** of a gradient, as it is reported.
fn kind_of(kind: GradientKind) -> &'static str {
    match kind {
        GradientKind::Linear => "linear",
        GradientKind::Radial => "radial",
    }
}

/// The stops, as they are reported.
fn stops_json(stops: &[GradientStop]) -> Vec<serde_json::Value> {
    stops
        .iter()
        .map(|stop| {
            serde_json::json!({
                "offset": stop.offset,
                "colour": colour_hex(stop.colour),
                "opacity": stop.opacity,
            })
        })
        .collect()
}

commands! {
    /// **Read** the gradient of a path.
    vector.gradient.read {
        area: "vector",
        layer: "vector",
        icon: "paint",
        params: {
            /// The path.
            path: NodeRef,
        },
        run: {
            let id = "vector.gradient.read".to_string();
            let gradient = gradient_of(session, &id, self.path.0)?;
            let stops = stops_of(&gradient);
            Ok(Outcome::new(format!("the path `{}` has {} stop(s)", self.path.0, stops.len())).with_data(
                serde_json::json!({
                    "path": self.path.0,
                    "kind": kind_of(gradient.kind),
                    "stops": stops_json(&stops),
                }),
            ))

        }
    }

    /// **Add** a stop to a gradient.
    vector.gradient.stop.add {
        area: "vector",
        layer: "vector",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The path.
            path: NodeRef,
            /// Where the stop is along the gradient: from zero to one.
            offset: Number,
            /// The colour of the stop.
            colour: Text,
            /// How solid the stop is: from zero to one. The default is one.
            opacity: Option<Number>,
        },
        run: {
            let id = "vector.gradient.stop.add".to_string();
            let offset = offset_of(&id, self.offset)?;
            let opacity = opacity_of(&id, self.opacity)?;
            let colour: Colour = parse_colour(&self.colour.0)?;
            let mut gradient = gradient_of(session, &id, self.path.0)?;
            let mut stops = stops_of(&gradient);
            // The stops are a **list in order**: a stop at a smaller
            // offset comes first.
            let at = stops.iter().position(|stop| stop.offset > offset).unwrap_or(stops.len());
            stops.insert(at, GradientStop { offset, colour, opacity });
            gradient.stops = stops.clone();
            write_gradient(session, &id, self.path.0, gradient)?;
            Ok(Outcome::changed(format!(
                "the gradient of the path `{}` has {} stop(s)",
                self.path.0,
                stops.len()
            ))
            .with_data(serde_json::json!({
                "path": self.path.0,
                "at": at,
                "stops": stops_json(&stops),
            })))

        }
    }

    /// **Change** a stop of a gradient.
    vector.gradient.stop.set {
        area: "vector",
        layer: "vector",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The path.
            path: NodeRef,
            /// The stop to change: its number in the list.
            stop: Number,
            /// Where the stop is along the gradient: from zero to one. It stays
            /// where it is when it is not said.
            offset: Option<Number>,
            /// The colour of the stop. It stays what it is when it is not
            /// said.
            colour: Option<Text>,
            /// How solid the stop is: from zero to one. It stays what it is
            /// when it is not said.
            opacity: Option<Number>,
        },
        run: {
            let id = "vector.gradient.stop.set".to_string();
            let index = index_of(&id, self.stop)?;
            let offset = match &self.offset {
                Some(offset) => Some(offset_of(&id, *offset)?),
                None => None,
            };
            let opacity = match &self.opacity {
                Some(opacity) => Some(opacity_of(&id, Some(*opacity))?),
                None => None,
            };
            let colour = match &self.colour {
                Some(colour) => Some(parse_colour(&colour.0)?),
                None => None,
            };
            let mut gradient = gradient_of(session, &id, self.path.0)?;
            let mut stops = stops_of(&gradient);
            let Some(stop) = stops.get_mut(index) else {
                return Err(EngineError::Failed { id, message: format!("the gradient has no stop `{index}`") });
            };
            if let Some(offset) = offset {
                stop.offset = offset;
            }
            if let Some(colour) = colour {
                stop.colour = colour;
            }
            if let Some(opacity) = opacity {
                stop.opacity = opacity;
            }
            // The stops are a **list in order** whatever happens to them.
            stops.sort_by(|left, right| {
                left.offset.partial_cmp(&right.offset).unwrap_or(std::cmp::Ordering::Equal)
            });
            gradient.stops = stops.clone();
            write_gradient(session, &id, self.path.0, gradient)?;
            Ok(Outcome::changed(format!("the stop `{index}` of the path `{}` is changed", self.path.0))
                .with_data(serde_json::json!({
                    "path": self.path.0,
                    "stop": index,
                    "stops": stops_json(&stops),
                })))

        }
    }

    /// **Take away** a stop of a gradient.
    vector.gradient.stop.remove {
        area: "vector",
        layer: "vector",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The path.
            path: NodeRef,
            /// The stop that goes: its number in the list.
            stop: Number,
        },
        run: {
            let id = "vector.gradient.stop.remove".to_string();
            let index = index_of(&id, self.stop)?;
            let mut gradient = gradient_of(session, &id, self.path.0)?;
            let mut stops = stops_of(&gradient);
            if index >= stops.len() {
                return Err(EngineError::Failed { id, message: format!("the gradient has no stop `{index}`") });
            }
            // A gradient has **two stops or more**: one colour is a fill
            // and not a gradient.
            if stops.len() <= 2 {
                return Err(EngineError::Failed { id, message: "a gradient has two stops or more".to_string() });
            }
            stops.remove(index);
            gradient.stops = stops.clone();
            write_gradient(session, &id, self.path.0, gradient)?;
            Ok(Outcome::changed(format!("the stop `{index}` of the path `{}` is gone", self.path.0)).with_data(
                serde_json::json!({
                    "path": self.path.0,
                    "stop": index,
                    "stops": stops_json(&stops),
                }),
            ))

        }
    }
}
