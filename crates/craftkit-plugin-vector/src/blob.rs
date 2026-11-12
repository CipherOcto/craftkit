//! The **blob brush**: a shape painted by a brush.
//!
//! The tree's Blob Brush paints a **stroke** of a brush's width onto
//! the shapes it touches, and the shape grows where the brush went.
//!
//! The rules are the tree's own:
//!
//! - the brush's **size** is the width of the stroke, and it must be a
//!   positive number;
//! - with a **fidelity**, the brush follows the stroke fitted by the
//!   tree's own fitting, and without one it follows every point;
//! - the stroke is **round** at its ends and **round** at its corners;
//! - the shape is the shape and the stroke **joined**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Cap, EngineError, Join, NodeRef, Number, Outcome, Point};

use craftkit_vector_geom as geom;

use crate::freehand::{read_samples, stroke};
use crate::{vector_of, write_back};

/// The brush's **size** when the caller names none.
const DEFAULT_SIZE: f64 = 10.0;

/// The **smallest** fidelity a brush may have.
const MIN_FIDELITY: f64 = 0.01;

/// The **largest** fidelity a brush may have.
const MAX_FIDELITY: f64 = 100.0;

commands! {
    /// Paint a brush stroke onto a shape.
    vector.path.blob {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Paint"),
        params: {
            /// The path the brush paints on.

            path: NodeRef,
            /// The brush's stroke: a run of points, `[[x, y], …]`.

            points: Json,
            /// How wide the brush is. 10 if it is not given.

            size: Option<Number>,
            /// The fidelity the brush's stroke is fitted at.
            ///
            /// Without one the brush follows every point.

            fidelity: Option<Number>,
        },
        run: {

            let id = "vector.path.blob".to_string();
            let size = self.size.as_ref().map(|size| size.0).unwrap_or(DEFAULT_SIZE);
            if !size.is_finite() || size <= 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "the size of the brush is a number and not a negative one".to_string(),
                });
            }
            if let Some(fidelity) = self.fidelity.as_ref()
                && (!fidelity.0.is_finite() || fidelity.0 < MIN_FIDELITY || fidelity.0 > MAX_FIDELITY)
            {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "a fidelity is a number between {MIN_FIDELITY} and {MAX_FIDELITY}, and the brush \\
                         follows the stroke fitted at it"
                    ),
                });
            }
            let samples =
                read_samples(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            // The brush's **stroke**: the fitted one when a fidelity is
            // named, and every point when it is not. A stroke of **one
            // point** is no line: the tree's own rule is a line of a
            // hundredth of a point there.
            let mut samples = samples;
            if samples.len() == 1 {
                let only = samples[0];
                samples.push(Point::new(only.x + 0.01, only.y));
            }
            let fitted = match self.fidelity.as_ref() {
                Some(fidelity) => stroke(&samples, fidelity.0, false, true),
                None => stroke(&samples, 0.0, false, false),
            };
            if fitted.is_empty() {
                return Err(EngineError::Failed {
                    id,
                    message: "the brush's stroke has nothing in it".to_string(),
                });
            }
            // The outline of a stroke closes on itself, and the side
            // where it closes is a point: it is taken away before the
            // shape and the brush are joined.
            let region =
                geom::without_degenerate(&geom::outline_stroke(&fitted, size, Cap::Round, Join::Round, 4.0));
            if region.is_empty() {
                return Err(EngineError::Failed {
                    id,
                    message: "the brush's stroke has nothing in it".to_string(),
                });
            }
            let mut vector = vector_of(session, self.path.0)?;
            if vector.path.is_empty() {
                return Err(EngineError::Failed { id, message: "the path has no shape to paint on".to_string() });
            }
            // The shape and the stroke **joined**: the shape is bigger
            // where the brush went.
            let painted = geom::boolean(&vector.path, &region, geom::BoolOp::Union)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            if painted.is_empty() {
                return Err(EngineError::Failed { id, message: "the brush left no shape".to_string() });
            }
            vector.path = painted;
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the brush painted on the shape").with_data(json!({ "id": self.path.0 })))

        }
    }
}
