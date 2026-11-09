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

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The farthest a guide may be from the origin, either way.
const MAX_GUIDE_POSITION: f64 = 1_000_000.0;

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

/// **Add** a guide to a page.
///
/// The position is a spread coordinate. A guide that is named by no
/// page is put on the page that stands under its coordinate, and a
/// spread guide is put on the first page of its spread.
#[derive(Command)]
#[command(id = "layout.guide.add", area = "layout", layer = "layout", icon = "ruler", mutates, undoable)]
pub struct AddGuide {
    /// The page it goes on.
    pub page: Option<NodeRef>,
    /// The spread it goes on, when the page is not named.
    pub spread: Option<NodeRef>,
    /// `horizontal`, `vertical` or `both`.
    pub orientation: Text,
    /// Where it stands.
    pub position: Option<Number>,
    /// The x of a guide in **both** directions.
    pub x: Option<Number>,
    /// The y of a guide in **both** directions.
    pub y: Option<Number>,
    /// The x of a horizontal guide, which is where the page is found.
    pub at: Option<Number>,
    /// Whether it crosses the pasteboard.
    pub spread_guide: Option<Bool>,
    /// Whether it is a liquid guide.
    pub liquid: Option<Bool>,
}

impl AddGuide {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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

/// **Move** a guide to another place.
#[derive(Command)]
#[command(id = "layout.guide.move", area = "layout", layer = "layout", icon = "ruler", mutates, undoable)]
pub struct MoveGuide {
    /// The guide.
    pub guide: NodeRef,
    /// Where it stands now.
    pub position: Number,
}

impl MoveGuide {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
#[derive(Command)]
#[command(id = "layout.guide.delete", area = "layout", layer = "layout", icon = "minus", mutates, undoable)]
pub struct DeleteGuide {
    /// The guide.
    pub guide: NodeRef,
}

impl DeleteGuide {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.guide.delete";
        let guide = self.guide.0;
        let node = the_guide(session, command, guide)?.clone();
        may_edit(session, &node, command)?;
        session.document_mut()?.remove_node(guide);
        Ok(Outcome::changed(format!("the guide `{guide}` is gone")).with_data(json!({ "guide": guide })))
    }
}

/// **Delete** every guide of a spread.
#[derive(Command)]
#[command(
    id = "layout.guide.delete.all",
    area = "layout",
    layer = "layout",
    icon = "minus",
    mutates,
    undoable
)]
pub struct DeleteAllGuides {
    /// The spread.
    pub spread: NodeRef,
}

impl DeleteAllGuides {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
#[derive(Command)]
#[command(id = "layout.page.guides", area = "layout", layer = "layout", icon = "list")]
pub struct ReadGuides {
    /// The page.
    pub page: NodeRef,
}

impl ReadGuides {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
