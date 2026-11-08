//! The **hidden information** of a file.
//!
//! A file hides things: the information of the document, the files
//! inside it, the comments on its pages, the fields of its form, the
//! text nobody can see, the layers that are off, its bookmarks, its
//! links and its scripts, and the private data of other applications.
//!
//! The tree has a panel that says all of that, and this module counts
//! the nine things the tree counts — the tree's own categories, in the
//! tree's own order.
//!
//! # The rules
//!
//! - The **tree's own rules** are the rules:
//!   - the **metadata** is the information of the file and the
//!     metadata streams of the catalog and of the pages;
//!   - the **attachments** are the files inside the file and the
//!     annotations that attach them;
//!   - the **comments** are the annotations that are none of a link, a
//!     widget, a pop-up, an attachment or a mark;
//!   - the **fields** are the fields of the form;
//!   - the **hidden text** is the text that is not seen (§642);
//!   - the **layers that are off** are the layers the file shows
//!     turned off;
//!   - the **bookmarks** are the items of the outline;
//!   - the **links, actions and scripts** are the links, the open
//!     action, the additional actions and the JavaScript;
//!   - the **private data** is the `/PieceInfo` of the catalog and of
//!     the pages.
//!
//! # What it does not do
//!
//! - **Nothing is taken away.** This counts what a file hides; the
//!   taking away is another thing.
//! - **The content of a layer that is off** is not counted: the tree
//!   counts the content of the off layers as well, and that is not
//!   here.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.hidden.scan";

/// The **categories** of the hidden information, in the tree's own
/// order.
pub const CATEGORIES: [&str; 9] = [
    "Metadata",
    "Attachments",
    "Comments",
    "FormFields",
    "HiddenText",
    "HiddenLayers",
    "Bookmarks",
    "LinksActionsScripts",
    "PrivateData",
];

/// The **pages** of the document.
fn pages_of(document: &Document) -> Vec<u64> {
    document
        .walk()
        .filter(|node| {
            (node.kind == "pdf.object" || node.kind == "pdf.stream")
                && crate::entries_of(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
        })
        .map(|node| node.id)
        .collect()
}

/// The **annotations** of a page: the node of each and its subtype.
fn annotations_of(document: &Document, page: u64) -> Vec<(u64, String)> {
    let mut out: Vec<(u64, String)> = Vec::new();
    for relation in document.relations.iter().filter(|relation| {
        relation.kind == "ref"
            && relation.from == page
            && relation
                .payload
                .get("key")
                .and_then(|key| key.as_str())
                .map(|key| key == "Annots")
                .unwrap_or(false)
    }) {
        let Some(node) = document.node(relation.to) else { continue };
        let subtype = crate::entries_of(&node.data)
            .into_iter()
            .find(|(key, _)| key == "Subtype")
            .and_then(|(_, value)| value.as_str().map(|text| text.to_string()))
            .unwrap_or_default();
        out.push((node.id, subtype));
    }
    out
}

/// Whether a dictionary holds an **entry** of that name.
fn holds(entries: &[(String, Json)], key: &str) -> bool {
    entries.iter().any(|(name, _)| name == key)
}

/// The **hidden information** of a file: what each category holds.
///
/// The answer is the count of each category, in the tree's own order.
pub fn scan(session: &Session) -> Vec<(String, usize)> {
    let document = session.document();
    let pages = pages_of(document);
    let annotations: Vec<(u64, String)> =
        pages.iter().flat_map(|page| annotations_of(document, *page)).collect();
    let catalog = crate::preflight::the_catalog(document);
    let catalog_entries = catalog.map(|node| crate::entries_of(&node.data)).unwrap_or_default();

    // The **metadata**: the information of the file and the metadata
    // streams of the catalog and of the pages.
    let information = crate::information::the_information(document)
        .map(|node| crate::entries_of(&node.data).len())
        .unwrap_or(0);
    let mut streams = 0usize;
    if holds(&catalog_entries, "Metadata") {
        streams += 1;
    }
    for page in &pages {
        let entries = document.node(*page).map(|node| crate::entries_of(&node.data)).unwrap_or_default();
        if holds(&entries, "Metadata") {
            streams += 1;
        }
    }
    let metadata = information + streams;

    // The **attachments**: the files inside the file and the
    // annotations that attach them.
    let embedded = crate::attachments::attachments_of(document).len();
    let attached = annotations.iter().filter(|(_, subtype)| subtype == "FileAttachment").count();
    let attachments = embedded + attached;

    // The **comments**: the annotations that are none of the tree's
    // own five kinds.
    let comments = annotations
        .iter()
        .filter(|(_, subtype)| {
            !matches!(subtype.as_str(), "Link" | "Widget" | "Popup" | "FileAttachment" | "Redact")
        })
        .count();

    // The **fields**: the fields of the form.
    let fields = crate::form::form_of(document).map(|form| form.fields.len()).unwrap_or(0);

    // The **hidden text**: the text of the pages that is not seen.
    let mut hidden_text = 0usize;
    for page in &pages {
        hidden_text += crate::pagetext::hidden_pieces(session, *page).len();
    }

    // The **layers that are off**.
    let layers = crate::optional::layers_of(document).iter().filter(|layer| !layer.visible).count();

    // The **bookmarks**: the items of the outline.
    let bookmarks = crate::outline::count_of(session);

    // The **links, actions and scripts**.
    let links = annotations.iter().filter(|(_, subtype)| subtype == "Link").count();
    let open_action = usize::from(holds(&catalog_entries, "OpenAction"));
    let mut actions = usize::from(holds(&catalog_entries, "AA"));
    for page in &pages {
        let entries = document.node(*page).map(|node| crate::entries_of(&node.data)).unwrap_or_default();
        if holds(&entries, "AA") {
            actions += 1;
        }
    }
    for (id, _) in &annotations {
        let entries = document.node(*id).map(|node| crate::entries_of(&node.data)).unwrap_or_default();
        if holds(&entries, "AA") {
            actions += 1;
        }
        let is_a_widget = entries
            .iter()
            .find(|(key, _)| key == "Subtype")
            .and_then(|(_, value)| value.as_str())
            .map(|subtype| subtype == "Widget")
            .unwrap_or(false);
        if is_a_widget && holds(&entries, "A") {
            actions += 1;
        }
    }
    let scripts = crate::javascript::scripts_of(document).0.len();
    let links_actions_scripts = links + open_action + actions + scripts;

    // The **private data**: the `/PieceInfo` of the catalog and of the
    // pages.
    let mut private = usize::from(holds(&catalog_entries, "PieceInfo"));
    for page in &pages {
        let entries = document.node(*page).map(|node| crate::entries_of(&node.data)).unwrap_or_default();
        if holds(&entries, "PieceInfo") {
            private += 1;
        }
    }

    // The counts in the tree's own order.
    let counts = [
        metadata,
        attachments,
        comments,
        fields,
        hidden_text,
        layers,
        bookmarks,
        links_actions_scripts,
        private,
    ];
    CATEGORIES
        .iter()
        .enumerate()
        .map(|(at, name)| (name.to_string(), counts.get(at).copied().unwrap_or_default()))
        .collect()
}

/// The **hidden information** of a file is counted.
///
/// A query: it does not change the document. A file that hides nothing
/// says so, and every category is named.
#[derive(Command)]
#[command(id = "pdf.hidden.scan", area = "pdf", layer = "pdf", icon = "list")]
pub struct ScanHidden {
    /// The category to count.
    ///
    /// Leave it out and every category is counted.
    pub category: Option<craftkit_sdk::Text>,
}

impl ScanHidden {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let counts = scan(session);
        let listed: Vec<Json> = match &self.category {
            None => counts
                .iter()
                .map(|(category, count)| json!({ "category": category.clone(), "count": count }))
                .collect(),
            Some(category) => {
                let wanted = category.0.trim();
                let Some((name, count)) = counts.iter().find(|(name, _)| name == wanted) else {
                    return Err(EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("there is no category called `{wanted}`"),
                    });
                };
                vec![json!({ "category": name.clone(), "count": count })]
            }
        };
        let total: usize = counts.iter().map(|(_, count)| count).sum();
        Ok(Outcome::new(format!("the file hides {total} thing(s)")).with_data(json!({
            "categories": listed,
            "total": total,
        })))
    }
}
