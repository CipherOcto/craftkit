//! The **comments** of a file: the annotations that are notes.
//!
//! The tree's own: a **comment** is an annotation that is **none of a
//! link, a widget and a popup** — a link is not a note, a widget is
//! the appearance of a field, and a popup is the window of a note
//! rather than the note itself.
//!
//! # The rules
//!
//! - the **pages** are the pages of the tree of pages, in the order
//!   the tree holds them;
//! - the **annotations** of a page are the ones the `/Annots` of the
//!   page holds, in the order the file holds them, and the **index**
//!   of an annotation is its place in that list — a link counts as a
//!   place, even though it is not a comment;
//! - a comment says:
//!   - the **page** it is on, and its **index** on that page;
//!   - the **subtype** — the kind of note it is;
//!   - the **author** (`/T`), the **contents** (`/Contents`), the
//!     **day** it was changed (`/M`) and its **name** (`/NM`);
//!   - the comment it **answers** (`/IRT` — the name of that one);
//!   - its **rectangle** (`/Rect`), which is two corners in order;
//!   - its **colour** (`/C`);
//!   - its **quads** (`/QuadPoints`), which are the lines of text it
//!     lies over, eight numbers at a time;
//!   - whether it is **locked** (the flag `Locked`);
//!   - its **intent** (`/IT`) — what it means, when it says.
//! - the **colour** of a comment that is written free (`FreeText`) is
//!   the colour of its **default appearance** (`/DA`) — the tree's own
//!   rule, and the rule here: the three numbers before the `rg` of
//!   the appearance.
//! - the **list** is **sorted**: by the page, and then from the **top
//!   of the page down** — the comment whose rectangle reaches highest
//!   comes first.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.comment.read";

/// How deep the tree of pages may be walked.
pub(crate) const MAX_DEPTH: usize = 64;

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **pages** of the file, in the order of the tree of pages.
fn pages_of(document: &Document) -> Vec<u64> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(pages) = entry(&catalog_entries, "Pages") else { return Vec::new() };
    let Some(node) = crate::preflight::node_of(document, pages) else { return Vec::new() };
    let mut out: Vec<u64> = Vec::new();
    walk_pages(document, node.id, 0, &mut out);
    out
}

/// Walk the **tree of pages**, and take the pages of it in order.
fn walk_pages(document: &Document, node: u64, depth: usize, out: &mut Vec<u64>) {
    if depth >= MAX_DEPTH {
        return;
    }
    let Some(current) = document.node(node) else { return };
    let entries = crate::entries_of(&current.data);
    let is_page = entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page"));
    if is_page {
        out.push(node);
        return;
    }
    let Some(kids) = entry(&entries, "Kids").and_then(|kids| kids.as_array()).cloned() else {
        return;
    };
    for kid in kids {
        let Some(kid_node) = crate::preflight::node_of(document, &kid) else { continue };
        let id = kid_node.id;
        walk_pages(document, id, depth + 1, out);
    }
}

/// The **annotations** of a page, in the order the file holds them.
fn annots_of(document: &Document, page: u64) -> Vec<u64> {
    let Some(node) = document.node(page) else { return Vec::new() };
    let entries = crate::entries_of(&node.data);
    let Some(annots) = entry(&entries, "Annots").and_then(|annots| annots.as_array()).cloned() else {
        return Vec::new();
    };
    let mut out: Vec<u64> = Vec::new();
    for annot in annots {
        let Some(node) = crate::preflight::node_of(document, &annot) else { continue };
        out.push(node.id);
    }
    out
}

/// The **numbers** of a list of numbers.
fn numbers_of(value: &Json) -> Vec<f64> {
    value
        .as_array()
        .map(|list| list.iter().map(|item| item.as_f64().unwrap_or(0.0)).collect())
        .unwrap_or_default()
}

/// The **rectangle** of a comment: two corners in order, and the
/// origin when the file says nothing that is a rectangle.
fn rect_of(entries: &[(String, Json)]) -> [f64; 4] {
    let numbers = entry(entries, "Rect").map(numbers_of).unwrap_or_default();
    if numbers.len() != 4 {
        return [0.0; 4];
    }
    [
        numbers[0].min(numbers[2]),
        numbers[1].min(numbers[3]),
        numbers[0].max(numbers[2]),
        numbers[1].max(numbers[3]),
    ]
}

/// The **colour** of a comment.
///
/// The tree's own: the `/C` of the comment, and for a comment that is
/// written free — a `FreeText` — the colour of its **default
/// appearance**: the three numbers that come before the `rg` of the
/// `/DA`.
fn colour_of(entries: &[(String, Json)], subtype: &str) -> Option<[f64; 3]> {
    let numbers = entry(entries, "C").map(numbers_of).unwrap_or_default();
    if numbers.len() == 3 {
        return Some([numbers[0], numbers[1], numbers[2]]);
    }
    if subtype != "FreeText" {
        return None;
    }
    let appearance = entry(entries, "DA").and_then(crate::fields::text_of)?;
    let parts: Vec<&str> = appearance.split_whitespace().collect();
    let at = parts.iter().position(|part| *part == "rg")?;
    if at < 3 {
        return None;
    }
    let number = |index: usize| parts[index].parse::<f64>().unwrap_or(0.0);
    Some([number(at - 3), number(at - 2), number(at - 1)])
}

/// The **comment** an annotation answers: the name of the comment it
/// is a reply to.
fn reply_of(document: &Document, entries: &[(String, Json)]) -> Option<String> {
    let reply = entry(entries, "IRT")?;
    let node = crate::preflight::node_of(document, reply)?;
    let reply_entries = crate::entries_of(&node.data);
    entry(&reply_entries, "NM").and_then(crate::fields::text_of)
}

/// The **flags** of a comment: the number the file holds.
fn flags_of(entries: &[(String, Json)]) -> i64 {
    entry(entries, "F")
        .and_then(|flags| flags.as_f64())
        .filter(|flags| flags.is_finite() && *flags > 0.0)
        .map(|flags| flags as i64)
        .unwrap_or(0)
}

/// The **icons** of a note: the tree's own.
pub const ICONS: [&str; 7] = ["Comment", "Note", "Help", "Insert", "Key", "NewParagraph", "Paragraph"];

/// The **size** of the icon of a note: the tree's own, in points.
const NOTE_SIZE: f64 = 20.0;

/// The **flags** of a note: the tree's own — it prints, it does not
/// zoom and it does not rotate.
const NOTE_FLAGS: i64 = 28;

/// The **colour** of a note that says none: the tree's own.
const NOTE_COLOUR: [f64; 3] = [1.0, 0.82, 0.0];

/// A **number**, as the tree writes it in a content stream.
fn stream_number(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".to_string() } else { text.to_string() }
}

/// The **drawing** of the icon of a note: the tree's own.
///
/// The tree draws the **bubble** for the icon `Comment` and the
/// **folded page** for every other icon — so the name of the icon says
/// which kind of note it is, and the shape of the icon is the tree's.
fn note_icon(name: &str, colour: [f64; 3]) -> String {
    let mut out = format!(
        "{} {} {} rg\n0.25 0.25 0.25 RG 0.8 w 1 j 1 J\n",
        stream_number(colour[0]),
        stream_number(colour[1]),
        stream_number(colour[2])
    );
    if name == "Comment" {
        out.push_str("3 18.5 m 17 18.5 l 18.5 18.5 18.5 17 18.5 17 c 18.5 8 l 18.5 6.5 17 6.5 17 6.5 c 9.5 6.5 l 5 2 l 5.5 6.5 l 3 6.5 l 1.5 6.5 1.5 8 1.5 8 c 1.5 17 l 1.5 18.5 3 18.5 3 18.5 c h B\n");
        out.push_str("4.5 15 m 15.5 15 l 4.5 12.5 m 15.5 12.5 l 4.5 10 m 11.5 10 l S\n");
    } else {
        out.push_str("3 19 m 13 19 l 17 15 l 17 1 l 3 1 l h B\n13 19 m 13 15 l 17 15 l S\n");
        out.push_str("5.5 12 m 14.5 12 l 5.5 9 m 14.5 9 l 5.5 6 m 11.5 6 l S\n");
    }
    out
}

commands! {
    /// **Put** a **note** on a page.
    ///
    /// The tree's own: a note is an annotation of the subtype `Text` — a
    /// sticky note — and it is drawn as an **icon** that hangs below the
    /// point it is put at, with a **popup** beside it that holds the text
    /// when it is opened.
    ///
    /// The rules:
    ///
    /// - the **icon** is one of the tree's own seven — `Comment`, `Note`,
    ///   `Help`, `Insert`, `Key`, `NewParagraph` and `Paragraph` — and an
    ///   icon that is not one of them is **refused**;
    /// - the **point** is two numbers, and the **rectangle** of the note
    ///   is the tree's own: the icon is twenty points square and hangs
    ///   **below** the point;
    /// - the **popup** of the note is the tree's own: it is beside the
    ///   icon, two hundred points wide and a hundred and twenty high;
    /// - the note **prints**, and it does not zoom and it does not rotate;
    /// - the **colour** of a note that says none is the tree's own — the
    ///   yellow of a sticky note;
    /// - the **subject** of the note is `Sticky Note`;
    /// - the **day** the note was written is the **caller's**: the tree
    ///   keeps the clock of the machine, and this tree is given the day.
    pdf.comment.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The page the note is on.
            page: NodeRef,
            /// The point the note is put at: two numbers.
            at: serde_json::Value,
            /// The text of the note.
            contents: Option<Text>,
            /// The author of the note.
            author: Option<Text>,
            /// The icon of the note: `Comment`, `Note`, `Help`, `Insert`,
            /// `Key`, `NewParagraph` or `Paragraph`.
            icon: Option<Text>,
            /// The colour of the note: a hex text.
            colour: Option<Text>,
            /// The day the note was written: `D:YYYYMMDDHHmmSS`.
            date: Option<Text>,
            /// Where the bytes of the icon are written.
            to: Text,
        },
        run: {
            let id = "pdf.comment.add".to_string();
            // The **page**.
            let Some(page_node) = session.document().node(self.page.0).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{}`", self.page.0),
                });
            };
            let page_entries = crate::entries_of(&page_node.data);
            if !page_entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page")) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{}` is not a page", self.page.0),
                });
            }
            let Some(page_number) = crate::the_number_of(&page_node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{}` has no object number", self.page.0),
                });
            };
            // The **point**: two numbers.
            let numbers = numbers_of(&self.at);
            if numbers.len() != 2 || numbers.iter().any(|number| !number.is_finite()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the point of a note is two numbers".to_string(),
                });
            }
            // The **icon**.
            let icon = match &self.icon {
                Some(icon) => {
                    let name = icon.0.trim().to_string();
                    if !ICONS.contains(&name.as_str()) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "`{name}` is not an icon of a note: it is one of {}",
                                ICONS.join(", ")
                            ),
                        });
                    }
                    name
                }
                None => "Note".to_string(),
            };
            // The **colour**.
            let colour = match &self.colour {
                Some(colour) => {
                    let parsed = craftkit_sdk::document::Colour::from_hex(colour.0.trim())
                        .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                    [f64::from(parsed.0[0]), f64::from(parsed.0[1]), f64::from(parsed.0[2])]
                }
                None => NOTE_COLOUR,
            };
            // The **rectangle**: the icon hangs below the point.
            let rect = [numbers[0], numbers[1] - NOTE_SIZE, numbers[0] + NOTE_SIZE, numbers[1]];
            // The **popup**: beside the icon.
            let popup_rect = [rect[2] + 10.0, rect[3] - 120.0, rect[2] + 210.0, rect[3]];
            // The **icon**, drawn.
            let path = self.to.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the drawing of the icon goes somewhere".to_string(),
                });
            }
            let drawing = note_icon(&icon, colour);
            std::fs::write(&path, drawing.as_bytes()).map_err(|error| EngineError::Failed {
                id: id.clone(),
                message: format!("the icon cannot be written: {error}"),
            })?;
            // The **appearance**: the icon is a form XObject of its own.
            let appearance_number = crate::free::the_next_number(session.document(), 0);
            let appearance_data = json!({
                "object": appearance_number,
                "entries": [
                    ["Type", "XObject"],
                    ["Subtype", "Form"],
                    ["FormType", 1],
                    ["BBox", [0.0, 0.0, NOTE_SIZE, NOTE_SIZE]],
                    ["Resources", { "entries": [] }],
                ],
            });
            let appearance_name = format!("stream {appearance_number}");
            let appearance =
                session.document_mut()?.add_node_with(appearance_name, "pdf.stream", None, appearance_data);
            if let Some(node) = session.document_mut()?.node_mut(appearance) {
                node.stream = Some(path);
            }
            // The **note**.
            let note_number = crate::free::the_next_number(session.document(), 0);
            let mut entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Text")),
                ("Rect".to_string(), json!(rect)),
                (
                    "Contents".to_string(),
                    json!({ "string": self.contents.as_ref().map(|text| text.0.clone()).unwrap_or_default() }),
                ),
                ("Subj".to_string(), json!("Sticky Note")),
                ("C".to_string(), json!([colour[0], colour[1], colour[2]])),
                ("Name".to_string(), json!(icon.clone())),
                ("F".to_string(), json!(NOTE_FLAGS)),
                ("Open".to_string(), json!(false)),
                ("P".to_string(), json!({ "ref": [page_number as f64, 0.0] })),
                ("AP".to_string(), json!({ "entries": [["N", { "ref": [appearance_number as f64, 0.0] }]] })),
            ];
            if let Some(author) = &self.author {
                let author = author.0.trim();
                if !author.is_empty() {
                    entries.push(("T".to_string(), json!({ "string": author })));
                }
            }
            if let Some(date) = &self.date {
                let date = date.0.trim();
                if !date.is_empty() {
                    entries.push(("M".to_string(), json!({ "string": date })));
                    entries.push(("CreationDate".to_string(), json!({ "string": date })));
                }
            }
            let note_data = json!({ "object": note_number, "entries": crate::pairs_of(&entries) });
            let note_name = format!("annotation {note_number}");
            let note = session.document_mut()?.add_node_with(note_name, "pdf.object", None, note_data);
            // The **popup** of the note.
            let popup_number = crate::free::the_next_number(session.document(), 0);
            let popup_entries: Vec<(String, Json)> = vec![
                ("Type".to_string(), json!("Annot")),
                ("Subtype".to_string(), json!("Popup")),
                ("Rect".to_string(), json!(popup_rect)),
                ("Parent".to_string(), json!({ "ref": [note_number as f64, 0.0] })),
                ("Open".to_string(), json!(false)),
                ("F".to_string(), json!(NOTE_FLAGS)),
            ];
            let popup_data = json!({ "object": popup_number, "entries": crate::pairs_of(&popup_entries) });
            let popup_name = format!("annotation {popup_number}");
            let popup = session.document_mut()?.add_node_with(popup_name, "pdf.object", None, popup_data);
            // The note names its popup.
            if let Some(node) = session.document_mut()?.node_mut(note) {
                node.data = crate::with_entry(&node.data, "Popup", json!({ "ref": [popup_number as f64, 0.0] }));
            }
            // The page names them both.
            let page_entries = crate::entries_of(
                &session.document().node(self.page.0).map(|node| node.data.clone()).unwrap_or(Json::Null),
            );
            let mut annots =
                entry(&page_entries, "Annots").and_then(|annots| annots.as_array()).cloned().unwrap_or_default();
            annots.push(json!({ "ref": [note_number as f64, 0.0] }));
            annots.push(json!({ "ref": [popup_number as f64, 0.0] }));
            if let Some(node) = session.document_mut()?.node_mut(self.page.0) {
                node.data = crate::with_entry(&node.data, "Annots", json!(annots));
            }
            // And the **edges**.
            for to in [note, popup] {
                let payload = json!({ "key": "Annots", "generation": 0 });
                let relation = craftkit_sdk::Relation::with_payload(self.page.0, to, "ref", payload);
                session.document_mut()?.add_relation(relation);
            }
            Ok(Outcome::changed("the note is put on the page".to_string()).with_data(json!({
                "note": note,
                "object": note_number,
                "popup": popup,
                "appearance": appearance,
                "icon": icon,
                "rect": rect,
                "popup_rect": popup_rect,
            })))

        }
    }

    /// **Change** the **author**, the **subject** and the **icon** of a
    /// comment.
    ///
    /// The tree's own: the author is the `/T` of the comment, the subject
    /// is its `/Subj` and the icon is its `/Name` — and the icon may only
    /// be given to a **note**, and a comment that is **locked** is not
    /// changed at all.
    ///
    /// The rules:
    ///
    /// - the node must be a **comment** — an annotation that is none of a
    ///   link, a widget and a popup;
    /// - a comment that is **locked** — the flag `Locked` — is
    ///   **refused**;
    /// - the **icon** is one of the tree's own seven, and only a **note**
    ///   — an annotation of the subtype `Text` — has one: an icon for a
    ///   comment of another kind is **refused**;
    /// - an **author** or a **subject** that is empty is taken away;
    /// - when the icon changes, the **icon is drawn again** — the tree's
    ///   own drawing, in the content of a form XObject of its own.
    pdf.comment.info {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The comment.
            comment: NodeRef,
            /// The author of the comment.
            author: Option<Text>,
            /// The subject of the comment.
            subject: Option<Text>,
            /// The icon of the comment: `Comment`, `Note`, `Help`, `Insert`,
            /// `Key`, `NewParagraph` or `Paragraph`.
            icon: Option<Text>,
            /// Where the bytes of the icon are written, when the icon changes.
            to: Option<Text>,
        },
        run: {
            let id = "pdf.comment.info".to_string();
            let comment = self.comment.0;
            let Some(node) = session.document().node(comment).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{comment}`"),
                });
            };
            let entries = crate::entries_of(&node.data);
            let subtype = entry(&entries, "Subtype").and_then(|kind| kind.as_str()).unwrap_or_default();
            if matches!(subtype, "Link" | "Widget" | "Popup") || subtype.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{comment}` is not a comment"),
                });
            }
            // A **locked** comment is not changed.
            if flags_of(&entries) & crate::annot::LOCKED != 0 {
                return Err(EngineError::Failed { id: id.clone(), message: "the comment is locked".to_string() });
            }
            // The **icon**: only a note has one.
            let mut icon = None;
            if let Some(text) = &self.icon {
                let name = text.0.trim().to_string();
                if !ICONS.contains(&name.as_str()) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{name}` is not an icon of a note: it is one of {}", ICONS.join(", ")),
                    });
                }
                if subtype != "Text" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "only a note has an icon".to_string(),
                    });
                }
                icon = Some(name);
            }
            // The **author** and the **subject**.
            let mut author = entry(&entries, "T").cloned();
            if let Some(text) = &self.author {
                let text = text.0.trim().to_string();
                author = if text.is_empty() { None } else { Some(json!({ "string": text })) };
            }
            let mut subject = entry(&entries, "Subj").cloned();
            if let Some(text) = &self.subject {
                let text = text.0.trim().to_string();
                subject = if text.is_empty() { None } else { Some(json!(text)) };
            }
            // The **icon**, drawn again.
            let mut appearance = None;
            if let Some(name) = &icon {
                let path = match &self.to {
                    Some(path) => path.0.trim().to_string(),
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the drawing of the icon goes somewhere".to_string(),
                        });
                    }
                };
                if path.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the drawing of the icon goes somewhere".to_string(),
                    });
                }
                let colour = match entry(&entries, "C").map(numbers_of) {
                    Some(colour) if colour.len() == 3 => [colour[0], colour[1], colour[2]],
                    _ => NOTE_COLOUR,
                };
                let drawing = note_icon(name, colour);
                std::fs::write(&path, drawing.as_bytes()).map_err(|error| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the icon cannot be written: {error}"),
                })?;
                let number = crate::free::the_next_number(session.document(), 0);
                let data = json!({
                    "object": number,
                    "entries": [
                        ["Type", "XObject"],
                        ["Subtype", "Form"],
                        ["FormType", 1],
                        ["BBox", [0.0, 0.0, NOTE_SIZE, NOTE_SIZE]],
                        ["Resources", { "entries": [] }],
                    ],
                });
                let stream_name = format!("stream {number}");
                let stream = session.document_mut()?.add_node_with(stream_name, "pdf.stream", None, data);
                if let Some(node) = session.document_mut()?.node_mut(stream) {
                    node.stream = Some(path);
                }
                appearance = Some((number, stream));
            }
            // The **comment** itself.
            if let Some(node) = session.document_mut()?.node_mut(comment) {
                let mut entries = crate::entries_of(&node.data);
                match &author {
                    Some(author) => match entries.iter_mut().find(|(key, _)| key == "T") {
                        Some(entry) => entry.1 = author.clone(),
                        None => entries.push(("T".to_string(), author.clone())),
                    },
                    None => entries.retain(|(key, _)| key != "T"),
                }
                match &subject {
                    Some(subject) => match entries.iter_mut().find(|(key, _)| key == "Subj") {
                        Some(entry) => entry.1 = subject.clone(),
                        None => entries.push(("Subj".to_string(), subject.clone())),
                    },
                    None => entries.retain(|(key, _)| key != "Subj"),
                }
                if let Some(name) = &icon {
                    match entries.iter_mut().find(|(key, _)| key == "Name") {
                        Some(entry) => entry.1 = json!(name.clone()),
                        None => entries.push(("Name".to_string(), json!(name.clone()))),
                    }
                }
                if let Some((number, _)) = &appearance {
                    let value = json!({ "entries": [["N", { "ref": [*number as f64, 0.0] }]] });
                    match entries.iter_mut().find(|(key, _)| key == "AP") {
                        Some(entry) => entry.1 = value,
                        None => entries.push(("AP".to_string(), value)),
                    }
                }
                node.data = json!({
                    "object": node.data.get("object").cloned().unwrap_or(Json::Null),
                    "entries": crate::pairs_of(&entries),
                });
            }
            Ok(Outcome::changed(format!("the comment `{comment}` is changed")).with_data(json!({
                "comment": comment,
                "author": author,
                "subject": subject,
                "icon": icon,
                "appearance": appearance.map(|(number, _)| number),
            })))

        }
    }

    /// **Read** the comments of a file.
    ///
    /// The tree's own: the comments are the annotations that are none of a
    /// link, a widget and a popup, and the list is sorted by the page and
    /// then from the top of the page down.
    pdf.comment.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The page whose comments are read; every page of the file when
            /// it is not said.
            page: Option<craftkit_sdk::NodeRef>,
        },
        run: {
            let document = session.document();
            let pages = pages_of(document);
            // The **pages** that are asked for.
            let wanted: Vec<(usize, u64)> = match self.page {
                Some(page) => {
                    let Some(index) = pages.iter().position(|held| *held == page.0) else {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!("the node `{}` is not a page of the file", page.0),
                        });
                    };
                    vec![(index, page.0)]
                }
                None => pages.iter().copied().enumerate().collect(),
            };
            let mut comments: Vec<Json> = Vec::new();
            for (page_index, page) in wanted {
                for (index, annot) in annots_of(document, page).iter().enumerate() {
                    let Some(node) = document.node(*annot) else { continue };
                    let entries = crate::entries_of(&node.data);
                    let Some(subtype) = entry(&entries, "Subtype").and_then(|kind| kind.as_str()) else {
                        continue;
                    };
                    if matches!(subtype, "Link" | "Widget" | "Popup") {
                        continue;
                    }
                    let rect = rect_of(&entries);
                    let quads: Vec<Vec<f64>> = entry(&entries, "QuadPoints")
                        .map(numbers_of)
                        .unwrap_or_default()
                        .chunks(8)
                        .map(|chunk| chunk.to_vec())
                        .collect();
                    comments.push(json!({
                        "page": page_index,
                        "index": index,
                        "id": node.id,
                        "subtype": subtype,
                        "author": entry(&entries, "T").and_then(crate::fields::text_of),
                        "contents": entry(&entries, "Contents").and_then(crate::fields::text_of),
                        "modified": entry(&entries, "M").and_then(crate::fields::text_of),
                        "name": entry(&entries, "NM").and_then(crate::fields::text_of),
                        "in_reply_to": reply_of(document, &entries),
                        "rect": rect,
                        "colour": colour_of(&entries, subtype),
                        "quads": quads,
                        "locked": flags_of(&entries) & crate::annot::LOCKED != 0,
                        "intent": entry(&entries, "IT").and_then(|intent| intent.as_str()),
                    }));
                }
            }
            // The **order** of the tree's own: by the page, and then from
            // the top of the page down.
            comments.sort_by(|left, right| {
                let page = left["page"].as_u64().cmp(&right["page"].as_u64());
                if page != std::cmp::Ordering::Equal {
                    return page;
                }
                let top = |comment: &Json| {
                    comment["rect"].as_array().and_then(|rect| rect.get(3)).and_then(Json::as_f64).unwrap_or(0.0)
                };
                top(right).total_cmp(&top(left))
            });
            let count = comments.len();
            Ok(Outcome::new(format!("the file holds {count} comment(s)")).with_data(json!({
                "comments": comments,
                "count": count,
            })))

        }
    }
}
