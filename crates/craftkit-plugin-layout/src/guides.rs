//! The **guides** of a page.
//!
//! A guide is a **line** that is drawn on a page to help with the
//! placing of things: a ruler guide that is dragged from the ruler, or
//! a liquid guide that the page tool draws from the ruler.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A guide is **horizontal** or **vertical**, and it stands at a
//!   **position**. A position is a number, and it is finite and no
//!   further than a million either side of the origin; a position that
//!   is not is refused.
//! - A guide's position is a **spread coordinate**: the place of the
//!   guide is measured across the whole spread, and not across the
//!   page it is on.
//! - A guide is put on a **page**. A page that is not named is found:
//!   it is the page that stands under the coordinate of the guide, and
//!   a **spread guide** — one that crosses the pasteboard — stands on
//!   the **first page** of its spread.
//! - A guide may be **moved** and **deleted**, and a guide that is
//!   **locked** or **hidden** — or that stands on a layer that is —
//!   may not be: it is refused by name.
//! - The guides of a **spread** may all be deleted at once, and the
//!   ones that are locked go with them.
//!
//! # What it does not do
//!
//! - **The tree's guides are addressed by their index** in the page
//!   they are on, and the tree's selection holds them. Here a guide is
//!   a **node**, and a command takes that node.
//! - **The tree's locked column guides** — a document setting that
//!   says a vertical guide may not be dragged — are not here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The farthest a guide may be from the origin, either way.
const MAX_GUIDE_POSITION: f64 = 1_000_000.0;

/// The most **divisions** a page may be cut into, one way.
const MAX_GUIDE_DIVISIONS: u64 = 1_000;

/// The widest a **gutter** may be.
const MAX_GUIDE_GUTTER: f64 = 1_000_000.0;

/// The most guides **one command** may make.
const MAX_GUIDES_CREATED: usize = 10_000;

/// The **pages** of a spread, in the order they stand in.
fn page_ids(spread: &Node) -> Vec<u64> {
    spread.children.iter().filter(|child| child.kind == "layout.page").map(|child| child.id).collect()
}

/// The **page** of a spread, as the spread holds it.
fn page_node(spread: &Node, page: u64) -> Option<Node> {
    spread.children.iter().find(|child| child.id == page).map(|child| (**child).clone())
}

/// The **width** of a page, as the page says it.
fn width_of(page: &Node) -> f64 {
    page.data.get("width").and_then(|width| width.as_f64()).filter(|width| width.is_finite()).unwrap_or(0.0)
}

/// The **place** a page stands at in its spread: the width of the
/// pages before it.
fn place_of(spread: &Node, page: u64) -> f64 {
    let mut place = 0.0;
    for child in spread.children.iter() {
        if child.id == page {
            break;
        }
        if child.kind == "layout.page" {
            place += width_of(child);
        }
    }
    place
}

/// The **divisions** a page may be cut into, one way: a whole number,
/// and no more than a thousand.
fn divisions_of(command: &str, key: &str, value: Option<Number>) -> Result<u64, EngineError> {
    let Some(value) = value else { return Ok(0) };
    let number = value.0;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("`{key}` must be a whole number"),
        });
    }
    let count = number as u64;
    if count > MAX_GUIDE_DIVISIONS {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("`{key}` must not exceed {MAX_GUIDE_DIVISIONS}"),
        });
    }
    Ok(count)
}

/// The **gutter** between two divisions: a number between nothing and
/// a million.
fn gutter_of(command: &str, key: &str, value: Option<Number>) -> Result<f64, EngineError> {
    let Some(value) = value else { return Ok(12.0) };
    let gutter = value.0;
    if !gutter.is_finite() || !(0.0..=MAX_GUIDE_GUTTER).contains(&gutter) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("`{key}` must be between 0 and {MAX_GUIDE_GUTTER}"),
        });
    }
    Ok(gutter)
}

/// The **guides** of a division: how many they are.
fn guide_count(divisions: u64, gutter: f64) -> usize {
    let edges = divisions.saturating_sub(1);
    let count = if gutter > 0.0 { edges.saturating_mul(2) } else { edges };
    match usize::try_from(count) {
        Ok(count) => count,
        Err(_) => usize::MAX,
    }
}

/// The **places** the guides of a division stand at, between `a` and
/// `b`: the edges of the cells, and — when there is a gutter — the
/// edge after it as well.
fn grid_positions(a: f64, b: f64, divisions: u64, gutter: f64) -> Result<Vec<f64>, EngineError> {
    let command = "layout.guide.create";
    if divisions < 2 {
        return Ok(Vec::new());
    }
    if !a.is_finite() || !b.is_finite() || !gutter.is_finite() || divisions > MAX_GUIDE_DIVISIONS {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "guide geometry must be finite and within the supported range".to_string(),
        });
    }
    let cell = ((b - a) - gutter * (divisions - 1) as f64) / divisions as f64;
    if !cell.is_finite() {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "guide geometry is outside the supported range".to_string(),
        });
    }
    let mut places = Vec::with_capacity(guide_count(divisions, gutter));
    for k in 1..divisions {
        let edge = a + k as f64 * cell + (k - 1) as f64 * gutter;
        if !edge.is_finite() || (gutter > 0.0 && !(edge + gutter).is_finite()) {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "guide geometry is outside the supported range".to_string(),
            });
        }
        places.push(edge);
        if gutter > 0.0 {
            places.push(edge + gutter);
        }
    }
    Ok(places)
}

/// The page that stands **under** a coordinate: the page that covers
/// it, and the nearest one when it covers none.
fn page_under(spread: &Node, x: f64) -> Option<u64> {
    let pages = page_ids(spread);
    if pages.is_empty() {
        return None;
    }
    let mut place = 0.0;
    let mut nearest = pages[0];
    let mut distance = f64::INFINITY;
    for page in pages {
        let Some(node) = page_node(spread, page) else { continue };
        let width = width_of(&node);
        if x >= place && x < place + width {
            return Some(page);
        }
        let away = if x < place { place - x } else { x - (place + width) };
        if away < distance {
            distance = away;
            nearest = page;
        }
        place += width;
    }
    Some(nearest)
}

/// The **spread** a page is in.
fn spread_of(session: &Session, page: u64) -> Option<u64> {
    crate::facing::spread_of(session.document(), page)
}

/// The **page** a command is about.
fn the_page<'a>(session: &'a Session, command: &str, page: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(page) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no page `{page}`"),
        });
    };
    if node.kind != "layout.page" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{page}` is a `{}` and not a page", node.kind),
        });
    }
    Ok(node)
}

/// The **spread** a command is about.
fn the_spread<'a>(session: &'a Session, command: &str, spread: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(spread) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no spread `{spread}`"),
        });
    };
    if node.kind != "layout.spread" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{spread}` is a `{}` and not a spread", node.kind),
        });
    }
    Ok(node)
}

/// The **guide** a command is about.
fn the_guide<'a>(session: &'a Session, command: &str, guide: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(guide) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no guide `{guide}`"),
        });
    };
    if node.kind != "layout.guide" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{guide}` is a `{}` and not a guide", node.kind),
        });
    }
    Ok(node)
}

/// A guide that is **locked** or **hidden** — or that stands on a layer
/// that is — may not be edited. The tree's own rule.
fn may_edit(session: &Session, guide: &Node, command: &str) -> Result<(), EngineError> {
    if session.document().is_locked(guide) || !session.document().is_visible(guide) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the guide or its layer is locked or hidden".to_string(),
        });
    }
    Ok(())
}

/// A guide's **position**: finite and no further than a million either
/// side of the origin.
fn position_of(position: f64, command: &str) -> Result<f64, EngineError> {
    if !position.is_finite() || position.abs() > MAX_GUIDE_POSITION {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!(
                "a guide position must be between -{MAX_GUIDE_POSITION} and {MAX_GUIDE_POSITION}"
            ),
        });
    }
    Ok(position)
}

/// The **guides** of a page, in the order they stand in.
fn guide_ids(page: &Node) -> Vec<u64> {
    page.children.iter().filter(|child| child.kind == "layout.guide").map(|child| child.id).collect()
}

/// What a guide **is**, for the answer.
fn guide_json(guide: &Node, index: usize) -> Json {
    json!({
        "id": guide.id,
        "index": index,
        "orientation": guide.data.get("orientation").cloned().unwrap_or(Json::Null),
        "position": guide.data.get("position").cloned().unwrap_or(Json::Null),
        "spread": guide.data.get("spread").and_then(|spread| spread.as_bool()).unwrap_or(false),
        "liquid": guide.data.get("liquid").and_then(|liquid| liquid.as_bool()).unwrap_or(false),
    })
}

commands! {
    /// **Add** a guide to a page.
    ///
    /// The position is a spread coordinate. A guide that is named by no
    /// page is put on the page that stands under its coordinate, and a
    /// spread guide is put on the first page of its spread.
    layout.guide.add {
        area: "layout",
        layer: "layout",
        icon: "ruler",
        mutates,
        undoable,
        params: {
            /// The page it goes on.

            page: Option<NodeRef>,
            /// The spread it goes on, when the page is not named.

            spread: Option<NodeRef>,
            /// `horizontal`, `vertical` or `both`.

            orientation: Text,
            /// Where it stands.

            position: Option<Number>,
            /// The x of a guide in **both** directions.

            x: Option<Number>,
            /// The y of a guide in **both** directions.

            y: Option<Number>,
            /// The x of a horizontal guide, which is where the page is found.

            at: Option<Number>,
            /// Whether it crosses the pasteboard.

            spread_guide: Option<Bool>,
            /// Whether it is a liquid guide.

            liquid: Option<Bool>,
        },
        run: {

            let command = "layout.guide.add";
            let spread_guide = self.spread_guide.map(|flag| flag.0).unwrap_or(false);
            let liquid = self.liquid.map(|flag| flag.0).unwrap_or(false);
            // The **orientation**: what the guide is, and a guide in both
            // directions is two guides.
            let orientation = self.orientation.0.trim().to_ascii_lowercase();
            let wanted: Vec<(&str, f64)> = match orientation.as_str() {
                "horizontal" | "vertical" => {
                    let position =
                        self.position.map(|position| position.0).ok_or_else(|| EngineError::Failed {
                            id: command.to_string(),
                            message: "a guide needs a position".to_string(),
                        })?;
                    vec![(if orientation == "horizontal" { "horizontal" } else { "vertical" }, position)]
                }
                "both" => {
                    let x = self.x.map(|x| x.0).ok_or_else(|| EngineError::Failed {
                        id: command.to_string(),
                        message: "a guide in both directions needs an x and a y".to_string(),
                    })?;
                    let y = self.y.map(|y| y.0).ok_or_else(|| EngineError::Failed {
                        id: command.to_string(),
                        message: "a guide in both directions needs an x and a y".to_string(),
                    })?;
                    vec![("horizontal", y), ("vertical", x)]
                }
                _ => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "orientation: horizontal|vertical|both".to_string(),
                    });
                }
            };
            // The **position** of every guide: finite and in its bounds.
            let mut guides = Vec::new();
            for (orientation, position) in wanted {
                let position = position_of(position, command)?;
                guides.push((orientation, position));
            }
            // The **page** the guides go on, and the spread they are in.
            let page = match self.page {
                Some(page) => {
                    the_page(session, command, page.0)?;
                    if let Some(spread) = self.spread
                        && spread_of(session, page.0) != Some(spread.0)
                    {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("the page `{}` is not in the spread `{}`", page.0, spread.0),
                        });
                    }
                    page.0
                }
                None => {
                    let spread = self.spread.map(|spread| spread.0).ok_or_else(|| EngineError::Failed {
                        id: command.to_string(),
                        message: "a guide needs a page or a spread".to_string(),
                    })?;
                    let spread_node = the_spread(session, command, spread)?.clone();
                    if page_ids(&spread_node).is_empty() {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("the spread `{spread}` has no page"),
                        });
                    }
                    // The page that stands **under** the coordinate of the
                    // guide, and the first page for a spread guide.
                    if spread_guide {
                        page_ids(&spread_node)[0]
                    } else {
                        let x = match orientation.as_str() {
                            "vertical" => guides[0].1,
                            "horizontal" => self.at.map(|at| at.0).unwrap_or(0.0),
                            _ => self.x.map(|x| x.0).unwrap_or(0.0),
                        };
                        page_under(&spread_node, x).ok_or_else(|| EngineError::Failed {
                            id: command.to_string(),
                            message: format!("the spread `{spread}` has no page"),
                        })?
                    }
                }
            };
            // A **liquid** guide is never a spread guide, as in the tree.
            let liquid = liquid && !spread_guide;
            let mut made = Vec::new();
            for (orientation, position) in guides {
                let name = format!("{orientation} guide");
                let id = session.document_mut()?.add_node_with_under(
                    Some(page),
                    name,
                    "layout.guide",
                    None,
                    json!({
                        "orientation": orientation,
                        "position": position,
                        "spread": spread_guide,
                        "liquid": liquid,
                    }),
                );
                made.push(json!({
                    "id": id,
                    "page": page,
                    "orientation": orientation,
                    "position": position,
                    "spread": spread_guide,
                    "liquid": liquid,
                }));
            }
            Ok(Outcome::changed(format!("added {} guide(s)", made.len())).with_data(json!({
                "guides": made,
                "count": made.len(),
            })))

        }
    }
    /// **Create** the guides of a page: a grid of them, across the page
    /// or across its margins.
    ///
    /// The tree's own rules: a page may be cut into **rows** and
    /// **columns** — at most a thousand of either — and the cells may have
    /// a **gutter** between them. `n` divisions make `n - 1` guides, and
    /// with a gutter each of them is doubled: the edge of the cell and the
    /// edge after the gutter. One command may make at most **ten thousand**
    /// guides.
    layout.guide.create {
        area: "layout",
        layer: "layout",
        icon: "ruler",
        mutates,
        undoable,
        params: {
            /// The spread the pages are in.

            spread: NodeRef,
            /// The page, when it is not every page of the spread.

            page: Option<NodeRef>,
            /// The rows the page is cut into.

            rows: Option<Number>,
            /// The columns the page is cut into.

            columns: Option<Number>,
            /// The room between the rows.

            row_gutter: Option<Number>,
            /// The room between the columns.

            column_gutter: Option<Number>,
            /// `margins` — the default — or `page`.

            fit_to: Option<Text>,
            /// Whether the guides that are there are cleared first.

            remove_existing: Option<Bool>,
        },
        run: {

            let command = "layout.guide.create";
            let rows = divisions_of(command, "rows", self.rows)?;
            let columns = divisions_of(command, "columns", self.columns)?;
            let row_gutter = gutter_of(command, "rowGutter", self.row_gutter)?;
            let column_gutter = gutter_of(command, "columnGutter", self.column_gutter)?;
            // The **area** the guides stand across: the margins of the
            // page, or the page itself.
            let fit_to = self.fit_to.as_ref().map(|fit| fit.0.trim().to_ascii_lowercase());
            let to_page = match fit_to.as_deref() {
                None | Some("margins") => false,
                Some("page") => true,
                Some(_) => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "fitTo: margins|page".to_string(),
                    });
                }
            };
            let spread = self.spread.0;
            let spread_node = the_spread(session, command, spread)?.clone();
            // The **pages** the guides are made on.
            let pages: Vec<u64> = match self.page {
                Some(page) => {
                    if !page_ids(&spread_node).contains(&page.0) {
                        return Err(EngineError::Failed {
                            id: command.to_string(),
                            message: format!("there is no page `{}` in the spread `{spread}`", page.0),
                        });
                    }
                    vec![page.0]
                }
                None => page_ids(&spread_node),
            };
            // The **count**: at most ten thousand guides in one command.
            let per_page = guide_count(rows, row_gutter) + guide_count(columns, column_gutter);
            let total = per_page.saturating_mul(pages.len());
            if total > MAX_GUIDES_CREATED {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("a single command may create at most {MAX_GUIDES_CREATED} guides"),
                });
            }
            let remove = self.remove_existing.map(|flag| flag.0).unwrap_or(false);
            let mut made = 0usize;
            for page in pages {
                let Some(node) = page_node(&spread_node, page) else { continue };
                let width = width_of(&node);
                let height = node
                    .data
                    .get("height")
                    .and_then(|height| height.as_f64())
                    .filter(|height| height.is_finite())
                    .unwrap_or(0.0);
                let place = place_of(&spread_node, page);
                // The **area**: the margin box of the page, and the page
                // itself when the guides are to fit it.
                let margins = crate::parents::margins_of(session.document(), page);
                let (x0, x1, y0, y1) = if to_page {
                    (place, place + width, 0.0, height)
                } else {
                    (
                        place + margins.left,
                        (place + width - margins.right).max(place + margins.left),
                        margins.top,
                        (height - margins.bottom).max(margins.top),
                    )
                };
                // The **guides** themselves.
                let mut wanted: Vec<(&str, f64)> = Vec::new();
                for y in grid_positions(y0, y1, rows, row_gutter)? {
                    wanted.push(("horizontal", y));
                }
                for x in grid_positions(x0, x1, columns, column_gutter)? {
                    wanted.push(("vertical", x));
                }
                if remove {
                    for guide in guide_ids(&node) {
                        session.document_mut()?.remove_node(guide);
                    }
                }
                for (orientation, position) in wanted {
                    let name = format!("{orientation} guide");
                    session.document_mut()?.add_node_with_under(
                        Some(page),
                        name,
                        "layout.guide",
                        None,
                        json!({
                            "orientation": orientation,
                            "position": position,
                            "spread": false,
                            "liquid": false,
                        }),
                    );
                    made += 1;
                }
            }
            Ok(Outcome::changed(format!("made {made} guide(s)")).with_data(json!({
                "guides": made,
                "count": made,
                "spread": spread,
            })))

        }
    }
    /// **Move** a guide to another place.
    layout.guide.move {
        area: "layout",
        layer: "layout",
        icon: "ruler",
        mutates,
        undoable,
        params: {
            /// The guide.

            guide: NodeRef,
            /// Where it stands now.

            position: Number,
        },
        run: {

            let command = "layout.guide.move";
            let guide = self.guide.0;
            the_guide(session, command, guide)?;
            let position = position_of(self.position.0, command)?;
            let node = session.document().node(guide).cloned().ok_or_else(|| EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no guide `{guide}`"),
            })?;
            may_edit(session, &node, command)?;
            if let Some(node) = session.document_mut()?.node_mut(guide) {
                node.data["position"] = json!(position);
            }
            Ok(Outcome::changed(format!("the guide `{guide}` stands at {position}"))
                .with_data(json!({ "guide": guide, "position": position })))

        }
    }
    /// **Delete** a guide.
    layout.guide.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The guide.

            guide: NodeRef,
        },
        run: {

            let command = "layout.guide.delete";
            let guide = self.guide.0;
            let node = the_guide(session, command, guide)?.clone();
            may_edit(session, &node, command)?;
            session.document_mut()?.remove_node(guide);
            Ok(Outcome::changed(format!("the guide `{guide}` is gone")).with_data(json!({ "guide": guide })))

        }
    }
    /// **Delete** every guide of a spread.
    layout.guide.delete.all {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The spread.

            spread: NodeRef,
        },
        run: {

            let command = "layout.guide.delete.all";
            let spread = self.spread.0;
            let node = the_spread(session, command, spread)?.clone();
            let mut deleted = 0usize;
            for page in page_ids(&node) {
                let Some(page_node) = page_node(&node, page) else { continue };
                for guide in guide_ids(&page_node) {
                    deleted += 1;
                    session.document_mut()?.remove_node(guide);
                }
            }
            Ok(Outcome::changed(format!("deleted {deleted} guide(s)")).with_data(json!({
                "spread": spread,
                "deleted": deleted,
            })))

        }
    }
    /// The **guides** of a page: what they are and where they stand.
    layout.page.guides {
        area: "layout",
        layer: "layout",
        icon: "list",
        params: {
            /// The page.

            page: NodeRef,
        },
        run: {

            let command = "layout.page.guides";
            let page = self.page.0;
            let node = the_page(session, command, page)?;
            let guides: Vec<Json> = guide_ids(node)
                .into_iter()
                .enumerate()
                .filter_map(|(index, guide)| session.document().node(guide).map(|node| guide_json(node, index)))
                .collect();
            Ok(Outcome::new(format!("the page `{page}` holds {} guide(s)", guides.len())).with_data(json!({
                "page": page,
                "guides": guides,
                "count": guides.len(),
            })))

        }
    }
}
