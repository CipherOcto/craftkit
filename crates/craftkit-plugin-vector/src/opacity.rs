//! The **opacity** of a path.
//!
//! The opacity is how **solid** a path is: it is a percentage, and it
//! multiplies into everything the path is painted with — the fill, the
//! line on its outline, and the stops of a gradient.
//!
//! The tree's finding is that the tree **clamps** the percentage and
//! this tree **refuses** it: the tree's command takes any number and
//! puts it between zero and a hundred, and a percentage of a thousand
//! is a percentage of a hundred. Here an opacity outside zero to a
//! hundred is refused, because an opacity that is not a percentage is
//! not an opacity.
//!
//! And the tree's finding is that the opacity is **not** the fill's
//! own transparency: a fill may be half solid and the path may be
//! half opaque, and the two are multiplied — the tree's opacity is the
//! object's own and the fill's alpha is the paint's.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, Fill, NodeRef, Number, Outcome, Session};

use crate::{Stroke, vector_of, write_back};

/// Put the **opacity** of a path.
///
/// The opacity is a percentage, from zero to a hundred: zero is
/// nothing and a hundred is everything. A number outside that range is
/// refused — the tree's command clamps it, and a clamped number is a
/// number that was not asked for.
#[derive(Command)]
#[command(id = "vector.path.opacity", area = "vector", layer = "vector", icon = "paint", mutates, undoable)]
pub struct SetOpacity {
    /// The path.
    pub path: NodeRef,
    /// The opacity, as a percentage: from zero to a hundred.
    pub opacity: Number,
}

impl SetOpacity {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "vector.path.opacity".to_string();
        let opacity = self.opacity.0;
        if !opacity.is_finite() || !(0.0..=100.0).contains(&opacity) {
            return Err(EngineError::Failed {
                id,
                message: "an opacity is a percentage from zero to a hundred".to_string(),
            });
        }
        let mut vector = vector_of(session, self.path.0)?;
        vector.opacity = opacity / 100.0;
        write_back(session, self.path.0, &vector)?;
        Ok(Outcome::changed(format!("the path is {} per cent solid", opacity))
            .with_data(serde_json::json!({ "id": self.path.0, "opacity": opacity })))
    }
}

/// The **fill** of a path with the path's opacity put into it.
///
/// The tree's rule: the object's opacity multiplies the paint's own
/// transparency — a fill that is half solid under a path that is half
/// opaque is a quarter solid.
pub fn fill_at(opacity: f64, fill: &Fill) -> Fill {
    if opacity >= 1.0 {
        return fill.clone();
    }
    match fill {
        Fill::Solid(colour) => {
            let mut colour = *colour;
            colour.0[3] *= opacity as f32;
            Fill::Solid(colour)
        }
        Fill::Gradient(gradient) => {
            let mut gradient = gradient.clone();
            for stop in &mut gradient.stops {
                stop.opacity *= opacity;
            }
            Fill::Gradient(gradient)
        }
    }
}

/// The **line** of a path with the path's opacity put into it.
pub fn stroke_at(opacity: f64, stroke: &Stroke) -> Stroke {
    let mut stroke = *stroke;
    stroke.colour.0[3] *= opacity as f32;
    stroke
}
