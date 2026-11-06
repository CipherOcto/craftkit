//! The **tab order** of a form.
//!
//! The order in which a person reaches the fields of a form is not
//! the order they were made in: it is the order the **page** says.
//! A page's `/Tabs` says how its annotations are reached:
//!
//! - `R` — **rows**: the annotations in rows, from the top of the page
//!   down, and in a row from the left to the right;
//! - `C` — **columns**: the annotations in columns, from the left of
//!   the page across, and in a column from the top down;
//! - `S` — the **structure** of the document;
//! - nothing — the **annotation order**: the order the page lists its
//!   annotations in.
//!
//! The **rules**:
//!
//! - the pages come in the **document's order**, and each page's
//!   annotations are reached before the next page's;
//! - an annotation that is **on no page** has no place in the order:
//!   the answer is nothing and not a place that means something else;
//! - two annotations are in the same **row** when their top edges are
//!   within half the height of the first one — and never nearer than
//!   two — and in the same **column** when their left edges are within
//!   half the width of the first one.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session};

/// The **order** a page's annotations are reached in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabOrder {
    /// The rows of the page, from the top down.
    Row,
    /// The columns of the page, from the left across.
    Column,
    /// The structure of the document.
    Structure,
    /// The order the page lists its annotations in.
    Annotations,
}

impl TabOrder {
    /// The order, from the name the file says.
    pub fn of(name: Option<&str>) -> TabOrder {
        match name {
            Some("R") => TabOrder::Row,
            Some("C") => TabOrder::Column,
            Some("S") => TabOrder::Structure,
            _ => TabOrder::Annotations,
        }
    }

    /// The name of the order.
    pub fn name(self) -> &'static str {
        match self {
            TabOrder::Row => "rows",
            TabOrder::Column => "columns",
            TabOrder::Structure => "structure",
            TabOrder::Annotations => "annotations",
        }
    }
}

/// One **annotation** of the form, and where it stands.
#[derive(Debug, Clone)]
pub struct Item {
    /// The name of the field the annotation belongs to.
    pub field: String,
    /// The number of the object of the annotation.
    pub widget: u64,
    /// The number of the object of the page it is on.
    pub page: u64,
    /// Where it is on the page.
    pub rect: [f64; 4],
    /// The place of the annotation in the page's own list of them.
    pub index: usize,
}

/// The **annotations** of the form: every widget of every field.
///
/// The **pages** of the document, in the document's order.
fn pages_of(document: &Document) -> Vec<u64> {
    document
        .walk()
        .filter(|node| {
            (node.kind == "pdf.object" || node.kind == "pdf.stream")
                && crate::entries_of(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
        })
        .filter_map(crate::the_number_of)
        .collect()
}

/// The **page** an annotation is on: the page whose annotations hold
/// it.
fn page_of(document: &Document, widget: u64) -> Option<u64> {
    document
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "ref"
                && relation.to == widget
                && relation
                    .payload
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(|key| key == "Annots")
                    .unwrap_or(false)
        })
        .and_then(|relation| document.node(relation.from))
        .and_then(crate::the_number_of)
}

/// The **place** of an annotation in the page's own list of them.
fn index_of(document: &Document, page: u64, widget: u64) -> Option<usize> {
    let page_node = document.walk().find(|node| crate::the_number_of(node) == Some(page))?;
    let annots = crate::entries_of(&page_node.data)
        .into_iter()
        .find(|(key, _)| key == "Annots")
        .and_then(|(_, value)| value.as_array().cloned())
        .unwrap_or_default();
    annots.iter().position(|annot| {
        annot
            .get("ref")
            .and_then(|pair| pair.as_array())
            .and_then(|pair| pair.first())
            .and_then(|number| number.as_u64())
            == Some(widget)
    })
}

/// The **order** a page's annotations are reached in, from the name
/// the page says.
fn order_of(document: &Document, page: u64) -> TabOrder {
    let Some(page_node) = document.walk().find(|node| crate::the_number_of(node) == Some(page)) else {
        return TabOrder::Annotations;
    };
    let name = crate::entries_of(&page_node.data)
        .into_iter()
        .find(|(key, _)| key == "Tabs")
        .and_then(|(_, value)| value.as_str().map(|name| name.to_string()));
    TabOrder::of(name.as_deref())
}

/// The **annotations** of the form, in the order they are reached.
pub fn tab_order(document: &Document) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    for field in crate::fields::fields_of(document) {
        for widget in field.widgets {
            let Some(node) = document.walk().find(|node| crate::the_number_of(node) == Some(widget)) else {
                continue;
            };
            let Some(page) = page_of(document, node.id) else { continue };
            let index = index_of(document, page, widget).unwrap_or(usize::MAX);
            items.push(Item {
                field: field.name.clone(),
                widget,
                page,
                rect: crate::fields::rect_of(&crate::entries_of(&node.data)),
                index,
            });
        }
    }
    let mut ordered: Vec<Item> = Vec::new();
    for page in pages_of(document) {
        let mut on: Vec<Item> = items.iter().filter(|item| item.page == page).cloned().collect();
        if on.is_empty() {
            continue;
        }
        match order_of(document, page) {
            TabOrder::Row => {
                on.sort_by(|one, other| other.rect[3].total_cmp(&one.rect[3]));
                for group in groups(&on, true) {
                    let mut group = group;
                    group.sort_by(|one, other| one.rect[0].total_cmp(&other.rect[0]));
                    ordered.extend(group);
                }
            }
            TabOrder::Column => {
                on.sort_by(|one, other| one.rect[0].total_cmp(&other.rect[0]));
                for group in groups(&on, false) {
                    let mut group = group;
                    group.sort_by(|one, other| other.rect[3].total_cmp(&one.rect[3]));
                    ordered.extend(group);
                }
            }
            TabOrder::Structure | TabOrder::Annotations => {
                on.sort_by_key(|item| item.index);
                ordered.extend(on);
            }
        }
    }
    ordered
}

/// The **groups** of annotations: the rows of a page, or its columns.
///
/// An annotation is in the group of the first one when their edges are
/// within half of its size — and never nearer than two.
fn groups(items: &[Item], row: bool) -> Vec<Vec<Item>> {
    let mut groups: Vec<Vec<Item>> = Vec::new();
    for item in items {
        let fits = groups.last().is_some_and(|group| {
            let first = &group[0];
            if row {
                (first.rect[3] - item.rect[3]).abs() < ((first.rect[3] - first.rect[1]) / 2.0).max(2.0)
            } else {
                (item.rect[0] - first.rect[0]).abs() < ((first.rect[2] - first.rect[0]) / 2.0).max(2.0)
            }
        });
        match groups.last_mut() {
            Some(group) if fits => group.push(item.clone()),
            _ => groups.push(vec![item.clone()]),
        }
    }
    groups
}

/// **Read** the **tab order** of a form.
///
/// The order in which the fields of a form are reached: the
/// annotations of a page in the order the page says, and the pages in
/// the document's order.
#[derive(Command)]
#[command(id = "pdf.tab.order", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadTabOrder {
    /// The page whose order is read.
    ///
    /// Leave it out and the whole document is read.
    pub page: Option<NodeRef>,
}

impl ReadTabOrder {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.tab.order".to_string();
        let document = session.document();
        let only = match self.page {
            Some(page) => {
                let node = document.node(page.0).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{}`", page.0),
                })?;
                Some(crate::the_number_of(node).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{}` has no object number", page.0),
                })?)
            }
            None => None,
        };
        let order = tab_order(document);
        let mut pages: Vec<Json> = Vec::new();
        for page in pages_of(document) {
            if only.is_some_and(|only| only != page) {
                continue;
            }
            pages.push(json!({
                "page": page,
                "order": order_of(document, page).name(),
            }));
        }
        let listed: Vec<Json> = order
            .iter()
            .enumerate()
            .filter(|(_, item)| only.is_none_or(|only| only == item.page))
            .map(|(rank, item)| {
                json!({
                    "rank": rank,
                    "field": item.field.clone(),
                    "widget": item.widget,
                    "page": item.page,
                })
            })
            .collect();
        let count = listed.len();
        Ok(Outcome::new(format!("the form has {count} annotation(s) in its tab order"))
            .with_data(json!({ "pages": pages, "order": listed })))
    }
}
