//! The **page** of a document.
//!
//! A document is printed on **pages**, and a page has a **size** and
//! **margins**. The size and the margins are in **points** — a
//! seventy-second of an inch — because that is the tree's own unit.
//!
//! The page is a fact of the **document**, and the tree's default page
//! is the tree's default page: US Letter, with an inch of margin.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **width** of the tree's default page, in points: US Letter.
pub const DEFAULT_WIDTH: f64 = 612.0;

/// The **height** of the tree's default page, in points: US Letter.
pub const DEFAULT_HEIGHT: f64 = 792.0;

/// The **margin** of the tree's default page, in points: an inch.
pub const DEFAULT_MARGIN: f64 = 72.0;

/// The most a page may be, in points: a yard and a half of paper.
pub const MAX_POINTS: f64 = 20_000.0;

/// The **page** of a document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Page {
    /// The width of the page, in points.
    pub width: f64,
    /// The height of the page, in points.
    pub height: f64,
    /// The margin at the top, in points.
    pub top: f64,
    /// The margin at the bottom, in points.
    pub bottom: f64,
    /// The margin on the left, in points.
    pub left: f64,
    /// The margin on the right, in points.
    pub right: f64,
}

impl Default for Page {
    /// The tree's own default page.
    fn default() -> Self {
        Page {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            top: DEFAULT_MARGIN,
            bottom: DEFAULT_MARGIN,
            left: DEFAULT_MARGIN,
            right: DEFAULT_MARGIN,
        }
    }
}

impl Page {
    /// The page of a document, as it is kept.
    pub fn of(node: &craftkit_sdk::Node) -> Page {
        let mut page = Page::default();
        let Some(stored) = node.data.get("page") else { return page };
        let number = |name: &str, fallback: f64| {
            stored
                .get(name)
                .and_then(|value| value.as_f64())
                .filter(|value| value.is_finite())
                .unwrap_or(fallback)
        };
        page.width = number("width", page.width);
        page.height = number("height", page.height);
        page.top = number("top", page.top);
        page.bottom = number("bottom", page.bottom);
        page.left = number("left", page.left);
        page.right = number("right", page.right);
        page
    }

    /// The page, as it is kept.
    pub fn to_json(self) -> Json {
        json!({
            "width": self.width,
            "height": self.height,
            "top": self.top,
            "bottom": self.bottom,
            "left": self.left,
            "right": self.right,
        })
    }

    /// Whether the page is a **landscape** one.
    ///
    /// The tree keeps a flag for it and a width and a height that are
    /// already swapped for it — two facts where one is enough. Here
    /// the page is landscape when it is wider than it is tall, and the
    /// flag is **derived**.
    pub fn is_landscape(self) -> bool {
        self.width > self.height
    }
}

/// The **document** a page belongs to.
fn the_document(session: &Session, command: &str, document: u64) -> Result<(), EngineError> {
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no document `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    Ok(())
}

/// A **measurement** that is a measurement.
fn points(value: f64, name: &str, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a {name} is a number of points, and {value} is not a number"),
        });
    }
    if value < 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a {name} is not negative, and {value} is"),
        });
    }
    if value > MAX_POINTS {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a {name} is at most {MAX_POINTS} points, and {value} is not"),
        });
    }
    Ok(value)
}

/// **Set** the page of a document.
///
/// The size is in **points**, and a margin that is not given keeps the
/// one the page has.
#[derive(Command)]
#[command(id = "prose.page.set", area = "prose", layer = "prose", icon = "rect", mutates, undoable)]
pub struct SetPage {
    /// The document.
    pub document: NodeRef,
    /// The width of the page, in points.
    pub width: Number,
    /// The height of the page, in points.
    pub height: Number,
    /// The margin at the top, in points.
    pub top: Option<Number>,
    /// The margin at the bottom, in points.
    pub bottom: Option<Number>,
    /// The margin on the left, in points.
    pub left: Option<Number>,
    /// The margin on the right, in points.
    pub right: Option<Number>,
}

impl SetPage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.page.set".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let width = points(self.width.0, "width", &id)?;
        let height = points(self.height.0, "height", &id)?;
        if width == 0.0 || height == 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a page is not nothing wide and nothing high".to_string(),
            });
        }
        let node = session.document().node(document);
        let mut page = match node {
            Some(node) => Page::of(node),
            None => Page::default(),
        };
        page.width = width;
        page.height = height;
        for (value, name) in [
            (self.top.map(|margin| margin.0), "top"),
            (self.bottom.map(|margin| margin.0), "bottom"),
            (self.left.map(|margin| margin.0), "left"),
            (self.right.map(|margin| margin.0), "right"),
        ] {
            let Some(value) = value else { continue };
            let value = points(value, name, &id)?;
            match name {
                "top" => page.top = value,
                "bottom" => page.bottom = value,
                "left" => page.left = value,
                _ => page.right = value,
            }
        }
        let stored = page.to_json();
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        node.data["page"] = stored.clone();
        Ok(Outcome::changed(format!("the page of the document `{document}` is {width} by {height}"))
            .with_data(json!({ "document": document, "page": stored })))
    }
}

/// **Read** the page of a document.
///
/// The page is a fact of the document, and whether it is a **landscape**
/// one is derived: it is wider than it is tall.
#[derive(Command)]
#[command(id = "prose.page.get", area = "prose", layer = "prose", icon = "rect")]
pub struct GetPage {
    /// The document.
    pub document: NodeRef,
}

impl GetPage {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.page.get".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let page = match session.document().node(document) {
            Some(node) => Page::of(node),
            None => Page::default(),
        };
        let mut stored = page.to_json();
        stored["landscape"] = json!(page.is_landscape());
        // The **background** of the page: the colour it is painted
        // with, and the watermark it carries. Both are facts of the
        // document, and a document that has neither says nothing.
        let background = session.document().node(document);
        let colour = background.as_ref().and_then(|node| crate::background::background_of(node));
        let watermark = background.as_ref().and_then(|node| crate::background::watermark_of(node));
        let borders = background.as_ref().and_then(|node| crate::borders::borders_of(node));
        // The **columns** of the page: how many there are, the space
        // between them, the line that may be drawn down the middle,
        // and the width of a column, which is derived.
        let columns = background.as_ref().map(|node| crate::columns::columns_of(node));
        Ok(Outcome::new(format!("the page of the document `{document}`")).with_data(json!({
            "document": document,
            "page": stored,
            "colour": colour,
            "watermark": watermark,
            "borders": borders,
            "columns": columns,
            // The **numbering** of the pages: the format the numbers
            // are written in, the number the first page has, and the
            // first page's number, which is derived.
            "numbers": background.as_ref().map(|node| crate::pagenumbers::read_of(node)),
        })))
    }
}
