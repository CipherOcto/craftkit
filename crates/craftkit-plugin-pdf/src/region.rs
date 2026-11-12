//! The **text under a region**: what a rectangle of the page covers.
//!
//! A region is a rectangle of the page's own space, and the text of a
//! page has a box (§637). This module asks the tree's own question:
//! **is this box under that region?**
//!
//! # The rules
//!
//! - The tree's own rule is the rule: a box counts as under a region
//!   when **more than a fifth of it**, or a point, whichever is less,
//!   is under the region in **both** directions. A neighbour that only
//!   touches an edge survives, and a box that is partly covered goes.
//! - A **degenerate box** — one with no width or no height — counts
//!   when its **origin** is inside the region: there is nothing to
//!   overlap, so the place it begins is the answer.
//! - **A piece with no place** is under nothing: a text that is not
//!   anywhere is not under a region either.
//! - **A region that is not a rectangle** is refused: four finite
//!   numbers, and the corners are put in order.
//!
//! # What it does not do
//!
//! - **The text is not removed.** This module says what is under a
//!   region; taking it away is another thing.
//! - **The glyphs are not counted one by one**: the tree works glyph
//!   by glyph, and this tree works by the **piece** — the box of a
//!   piece is the box of its whole run.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.text.under";

/// A box with **no width** or **no height**: there is nothing to
/// overlap.
const DEGENERATE: f64 = 0.01;

/// The **part of a box** that must be under a region: a fifth of it.
const FIFTH: f64 = 0.2;

/// The **most** a box must be under a region: a point.
const A_POINT: f64 = 1.0;

/// Whether a **box** is under a **region**.
///
/// The tree's own rule: more than a fifth of the box, or a point,
/// whichever is less, must be under the region in both directions.
pub fn under(region: [f64; 4], bounds: [f64; 4], origin: (f64, f64)) -> bool {
    let width = bounds[2] - bounds[0];
    let height = bounds[3] - bounds[1];
    if width < DEGENERATE || height < DEGENERATE {
        // A box with nothing in it is under the region when it
        // begins inside it.
        return origin.0 >= region[0]
            && origin.0 <= region[2]
            && origin.1 >= region[1]
            && origin.1 <= region[3];
    }
    let least_x = (width * FIFTH).min(A_POINT);
    let least_y = (height * FIFTH).min(A_POINT);
    let over_x = bounds[2].min(region[2]) - bounds[0].max(region[0]);
    let over_y = bounds[3].min(region[3]) - bounds[1].max(region[1]);
    over_x > least_x && over_y > least_y
}

commands! {
    /// The **text** of a page that is **under a region**.
    ///
    /// A query: it does not change the document. A page with no text under
    /// the region has none, and the answer says so.
    pdf.page.text.under {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The page whose text is read.

            page: NodeRef,
            /// The region: four numbers, the left, the bottom, the right and
            /// the top of it.

            rect: Json,
        },
        run: {

            let page = self.page.0;
            let Some(region) = crate::boxes::rect_of(&self.rect) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the region is not a rectangle of four numbers".to_string(),
                });
            };
            let text = crate::pagetext::text_of_page(session.document(), page)?;
            let mut found: Vec<Json> = Vec::new();
            for piece in &text.pieces {
                let Some(bounds) = piece.bounds else { continue };
                let origin = match (piece.x, piece.y) {
                    (Some(x), Some(y)) => (x, y),
                    _ => continue,
                };
                if !under(region, bounds, origin) {
                    continue;
                }
                found.push(json!({
                    "text": piece.text.clone(),
                    "operator": piece.operator.clone(),
                    "font": piece.font.clone().map(|font| json!(font)).unwrap_or(Json::Null),
                    "x": piece.x.map(|x| json!(x)).unwrap_or(Json::Null),
                    "y": piece.y.map(|y| json!(y)).unwrap_or(Json::Null),
                    "box": json!(bounds),
                }));
            }
            let count = found.len();
            Ok(Outcome::new(format!("{count} piece(s) of text are under the region")).with_data(json!({
                "page": page,
                "region": json!(region),
                "pieces": found,
                "count": count,
            })))

        }
    }
}
