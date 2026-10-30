//! `craftkit-plugin-text` — the **text battery**: a paragraph, laid out.
//!
//! # What is ported
//!
//! A word processor's paragraph is not a string and not a run of
//! glyphs: it is a **sequence of words** that has to be **broken into
//! lines** before anybody can read it. The breaking is the Knuth and
//! Plass model, and it lives in a **library**:
//! `crates/craftkit-text-layout` is the breaker — the optimal one and
//! the greedy one — and this crate is the **document model** around it:
//!
//! - a **paragraph** — a node of kind `text.paragraph`;
//! - the paragraph's **words** — the node's **payload**: a word is
//!   numerous, and what is numerous is in the payload;
//! - the **measure** — the width of the column, in millimetres, and it
//!   is in the payload because it is a property of the paragraph's
//!   place, not of the paragraph;
//! - the **ragged** flag — the paragraph's setting, in the payload;
//! - a **metric** — a node of kind `text.metric`: the widths of the
//!   characters, in millimetres. A metric is **few** — a typeface is
//!   shared by every paragraph in a document — so a metric is a node;
//! - the paragraph's **typeface** — an **edge**: the paragraph is set
//!   in the metric, and the edge says which one;
//! - the **lines** — **derived**: the paragraph is broken every time
//!   somebody asks, and the lines are in the plan, not in the document.
//!
//! # What the port found
//!
//! - **The kit has no font, so the widths come from the caller.** A
//!   line breaker needs the advance of every glyph, and an advance comes
//!   from a typeface. This tree has no typeface and no asset of any
//!   kind, so a **metric** is a table of widths and nothing else — no
//!   outlines, no glyphs, no kerning — and the caller says what the
//!   widths are. A metric is **numbers**, and numbers are data and not
//!   artwork, so the no-asset rule and the missing font are compatible.
//!   The moment a font arrives, it supplies the widths and nothing else
//!   changes.
//! - **The metric was in the paragraph's payload, and that was wrong.**
//!   The first version of this battery put the **advance** and the
//!   **space** in the paragraph's payload, because a paragraph's metric
//!   looked like a property of the paragraph. It is not: a typeface is
//!   shared by every paragraph in a document, and it has a name. So the
//!   metric moves to a **node** and the paragraph's typeface becomes an
//!   **edge** — the fifth kind of edge in this kit, after the PDF's
//!   reference, the CAD's layer, the prose's style and the layout's
//!   item's story.
//! - **A monospaced paragraph is not a special case; it is a metric
//!   with an empty table.** The general metric has a width for every
//!   character that has one and a default for the rest; the
//!   monospaced one has only the default. One code path, and the
//!   degenerate metric is the one everybody starts with.
//! - **A paragraph is words, not a string, and the words are the
//!   payload.** The word processor stores a paragraph's text once, as a
//!   string, and its runs over it. Here the string would be a second
//!   copy of what the words already say, so the **words are stored** and
//!   the string is not: the text is split on whitespace when the
//!   paragraph is made, and it is never put back together. That is the
//!   rule "what is numerous is in the payload" applied to a thing that
//!   is numerous *and* a list.
//! - **The measure is not the paragraph's.** A paragraph's measure is
//!   the width of the column it sits in, and the column is the
//!   **place**. So the measure is a **value the caller sets** and a
//!   **command the caller runs** — `text.paragraph.measure` — and a
//!   paragraph with no measure yet is a paragraph that has not been
//!   placed: it is in the document, and it has no lines.
//! - **The lines are derived, and the derivation is the library's.** The
//!   plugin does not choose the breaks: it turns the words into **boxes
//!   and glue** — a box of `characters × advance` for each word, a glue
//!   of `space` between them — and hands them to the breaker, which
//!   answers with the lines and says **which breaker ran**. The
//!   evaluation reports that answer: the lines, their widths, their
//!   ratios, and the method.
//! - **The item indices are not the word indices, and the difference is
//!   the glue.** The library's items alternate box and glue, so a line
//!   that ends at item `e` ends at word `e / 2`, and the plugin says
//!   so rather than leaving the caller to know it.
//!
//! # What is not here
//!
//! No font, no glyphs, no hyphenation dictionary and no varying
//! measure: the first is the library's reason for existing, the second
//! is a dictionary's business, and the third the library names as
//! deliberately out of scope.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{
    Bool, Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Relation, Session, Text,
};
use craftkit_text_layout::{Breaks, Item, LayoutError, Method, Params, Rag, break_paragraph};
use std::collections::HashMap;

/// The text layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "text", name = "Text", requires("core"))]
pub struct TextLayer;

/// A **paragraph**: a run of words that is broken into lines.
#[derive(NodeKind)]
#[node_kind(kind = "text.paragraph", label = "Paragraph", layer = "text", evaluates)]
pub struct ParagraphKind;

impl ParagraphKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a paragraph **means**: its words, its measure, its setting,
    /// and the number of lines it breaks into.
    ///
    /// The lines are **derived**: the paragraph is broken every time
    /// somebody asks, and a paragraph with no measure yet has **no
    /// lines at all** — the count is null, not zero.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let words = words_of(node);
        let measure = measure_of(node);
        let metric = metric_of(document, node);
        let lines = if measure > 0.0 && metric.is_some() {
            break_lines(node, metric.as_ref()).ok().map(|breaks| breaks.lines.len())
        } else {
            None
        };
        Some(json!({
            "words": words.len(),
            "measure_mm": measure,
            "ragged": ragged_of(node),
            "metric": metric.is_some(),
            "lines": lines,
        }))
    }
}

/// A **metric**: the widths of the characters, in millimetres.
///
/// A metric is **few** — a typeface is shared by every paragraph in a
/// document — so a metric is a **node**, and a paragraph says which
/// metric it is set in with an **edge**.
#[derive(NodeKind)]
#[node_kind(kind = "text.metric", label = "Metric", layer = "text", evaluates)]
pub struct MetricKind;

impl MetricKind {
    /// Not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a metric **means**: its default width, its space, and how
    /// many characters have a width of their own.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let metric = Metric::of(node)?;
        Some(json!({
            "default_mm": metric.default_mm,
            "space_mm": metric.space_mm,
            "characters": metric.advances.len(),
            "runs": metric.runs.len(),
            "pairs": metric.pairs.len(),
            "monospaced": metric.advances.is_empty() && metric.runs.is_empty() && metric.pairs.is_empty(),
        }))
    }
}

/// The kind of the edge that says a paragraph is set in a metric.
const SET_IN: &str = "text.set_in";

/// A **metric**, read from a node's payload.
///
/// A metric is **three tables**, and the three are applied in three
/// different places:
///
/// - the **advances**: the width of a character that has one;
/// - the **runs**: the width of a *run* of characters that is one glyph —
///   a ligature. A run **replaces** its characters;
/// - the **pairs**: the **adjustment** between two characters — a
///   kerning pair. A pair does not replace anything: it changes the gap
///   between two characters that are still two glyphs.
///
/// The reader keeps what it does not understand: a width that is not a
/// number, a key that is not a single character, a run of one character
/// and a pair of three are **left out**. A metric that was made by the
/// command cannot hold such a thing; a metric that arrived in a file
/// can.
struct Metric {
    /// The width of a character that has no width of its own.
    default_mm: f64,
    /// The widths of the characters that have one.
    advances: HashMap<char, f64>,
    /// The widths of the runs that have one.
    runs: HashMap<String, f64>,
    /// The adjustments of the pairs that have one.
    pairs: HashMap<String, f64>,
    /// The width of the space between two words.
    space_mm: f64,
}

/// The longest run a metric may hold: eight characters.
///
/// A run is looked for at every position of a word, and the longest one
/// wins, so the number of characters a run may have is the number of
/// places that are looked at. Eight is enough for the clusters a
/// typeface has and small enough to bound the work.
pub const MAX_RUN: usize = 8;

impl Metric {
    /// Read the metric out of a node's payload.
    fn of(node: &Node) -> Option<Metric> {
        let default_mm = node.data.get("default_mm").and_then(|value| value.as_f64())?;
        let space_mm = node.data.get("space_mm").and_then(|value| value.as_f64())?;
        if !default_mm.is_finite() || !space_mm.is_finite() || default_mm < 0.0 || space_mm < 0.0 {
            return None;
        }
        let mut advances = HashMap::new();
        for (key, value) in table_of(node, "advances") {
            let Some(character) = one_character(key) else { continue };
            let Some(width) = width_of(value) else { continue };
            advances.insert(character, width);
        }
        let mut runs = HashMap::new();
        for (key, value) in table_of(node, "runs") {
            // A run is **two** characters or more: one character is an
            // advance, and the metric has a table for those.
            if key.chars().count() < 2 {
                continue;
            }
            let Some(width) = width_of(value) else { continue };
            runs.insert(key.clone(), width);
        }
        let mut pairs = HashMap::new();
        for (key, value) in table_of(node, "pairs") {
            // A pair is **exactly** two characters: more is a run, and
            // fewer is not a pair.
            if key.chars().count() != 2 {
                continue;
            }
            let Some(adjust) = value.as_f64() else { continue };
            if !adjust.is_finite() {
                continue;
            }
            pairs.insert(key.clone(), adjust);
        }
        Some(Metric { default_mm, advances, runs, pairs, space_mm })
    }

    /// The width of one character.
    fn advance(&self, character: char) -> f64 {
        self.advances.get(&character).copied().unwrap_or(self.default_mm)
    }

    /// The longest run that starts at `index`, and its width.
    fn longest_run(&self, characters: &[char], index: usize) -> Option<(usize, f64)> {
        let mut length = characters.len().min(index + MAX_RUN) - index;
        while length >= 2 {
            let text: String = characters[index..index + length].iter().collect();
            if let Some(width) = self.runs.get(&text) {
                return Some((length, *width));
            }
            length -= 1;
        }
        None
    }

    /// The width of a word: its runs, its characters and its pairs.
    ///
    /// The word is read from the left: at every place the **longest
    /// run** that matches is taken and replaces its characters, and at
    /// every **boundary** the pair's adjustment is added.
    fn word_width(&self, word: &str) -> f64 {
        let characters: Vec<char> = word.chars().collect();
        let mut width = 0.0;
        let mut index = 0;
        while index < characters.len() {
            match self.longest_run(&characters, index) {
                Some((length, run_width)) => {
                    width += run_width;
                    index += length;
                }
                None => {
                    width += self.advance(characters[index]);
                    index += 1;
                }
            }
            // The adjustment at the boundary, if the pair has one.
            if index < characters.len() {
                let pair: String = characters[index - 1..index + 1].iter().collect();
                if let Some(adjust) = self.pairs.get(&pair) {
                    width += adjust;
                }
            }
        }
        width
    }
}

/// The table in a node's payload under a name, if it is a table.
fn table_of<'a>(node: &'a Node, name: &str) -> Vec<(&'a String, &'a Json)> {
    node.data
        .get(name)
        .and_then(|value| value.as_object())
        .map(|table| table.iter().collect())
        .unwrap_or_default()
}

/// The width in a table, if it is one.
fn width_of(value: &Json) -> Option<f64> {
    let width = value.as_f64()?;
    if !width.is_finite() || width < 0.0 { None } else { Some(width) }
}

/// The one character of a key, if the key is one character.
fn one_character(key: &str) -> Option<char> {
    let mut characters = key.chars();
    let first = characters.next()?;
    if characters.next().is_some() { None } else { Some(first) }
}

/// The metric a paragraph is set in, if it is set in one.
fn metric_of(document: &Document, paragraph: &Node) -> Option<Metric> {
    let relation = document.relations_from(paragraph.id, Some(SET_IN)).into_iter().next()?;
    let node = document.node(relation.to)?;
    Metric::of(node)
}

/// The words of a paragraph: the payload's list of strings.
fn words_of(node: &Node) -> Vec<String> {
    node.data
        .get("words")
        .and_then(|words| words.as_array())
        .map(|words| words.iter().filter_map(|word| word.as_str().map(|word| word.to_string())).collect())
        .unwrap_or_default()
}

/// The width of the column, in millimetres. Zero means **not set yet**.
fn measure_of(node: &Node) -> f64 {
    node.data.get("measure_mm").and_then(|value| value.as_f64()).unwrap_or(0.0)
}

/// Whether the paragraph is set ragged.
fn ragged_of(node: &Node) -> bool {
    node.data.get("ragged").and_then(|value| value.as_bool()).unwrap_or(false)
}

/// The paragraph's words, as the library's **items**.
///
/// A word is a **box** as wide as the **metric** says its characters
/// are; between two words is a **glue** of the space, which stretches a
/// little and shrinks a little; and after the last word is a **forced
/// break**, so that the paragraph ends where it ends.
///
/// The items alternate box and glue, so the item index of a word is
/// twice its word index.
fn items_for(words: &[String], metric: &Metric) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::with_capacity(words.len() * 2);
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            items.push(Item::glue(metric.space_mm, metric.space_mm / 2.0, metric.space_mm / 3.0));
        }
        items.push(Item::boxed(metric.word_width(word)));
    }
    if !words.is_empty() {
        items.push(Item::forced());
    }
    items
}

/// The library's parameters for a paragraph: ragged, or justified.
fn layout_params(ragged: bool) -> Params {
    if ragged { Params { rag: Some(Rag::default()), ..Params::default() } } else { Params::default() }
}

/// Break a paragraph into lines, with the library.
///
/// The metric is the paragraph's, and the caller has found it: the
/// paragraph says which metric it is set in with an edge.
fn break_lines(node: &Node, metric: Option<&Metric>) -> Result<Breaks, LayoutError> {
    let Some(metric) = metric else { return Err(LayoutError::BadMeasure) };
    let words = words_of(node);
    let items = items_for(&words, metric);
    let params = layout_params(ragged_of(node));
    break_paragraph(&items, measure_of(node), &params)
}

/// Put a **metric** in the document: the widths of the characters.
///
/// A metric is **few**, so it is a **node**: a typeface is shared by
/// every paragraph in a document, and it has a name.
///
/// The widths are the metric's payload: they are numerous — one per
/// character that has one — and they hold nothing but numbers.
#[derive(Command)]
#[command(id = "text.metric.add", area = "text", layer = "text", icon = "plus", mutates, undoable)]
pub struct AddMetric {
    /// What to call it: the typeface's name.
    pub name: Text,
    /// The width of a character that has no width of its own, in
    /// millimetres. A metric with no table at all is a **monospaced**
    /// one.
    pub default_mm: Number,
    /// The width of the space between two words, in millimetres.
    pub space_mm: Number,
    /// The widths of the characters that have one: a table of character
    /// to millimetres.
    pub advances: Option<Json>,
    /// The widths of the **runs** that have one: a table of two or more
    /// characters to millimetres. A run is one glyph — a ligature — and
    /// it replaces its characters.
    pub runs: Option<Json>,
    /// The **adjustments** of the pairs that have one: a table of two
    /// characters to millimetres. A pair is a kerning pair: it changes
    /// the gap between two characters and replaces nothing.
    pub pairs: Option<Json>,
}

impl AddMetric {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let default_mm = self.default_mm.0;
        let space_mm = self.space_mm.0;
        if !default_mm.is_finite() || default_mm < 0.0 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: "a character is not narrower than nothing".to_string(),
            });
        }
        if !space_mm.is_finite() || space_mm < 0.0 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: "a space is not narrower than nothing".to_string(),
            });
        }
        let advances = read_advances(self.advances.as_ref())?;
        let runs = read_runs(self.runs.as_ref())?;
        let pairs = read_pairs(self.pairs.as_ref())?;
        let id = session.document_mut().add_node_with_under(
            None,
            self.name.0.clone(),
            "text.metric",
            None,
            json!({
                "default_mm": default_mm,
                "space_mm": space_mm,
                "advances": Json::Object(advances),
                "runs": Json::Object(runs),
                "pairs": Json::Object(pairs),
            }),
        );
        Ok(Outcome::changed(format!("added the metric `{}`", self.name.0)).with_data(json!({ "id": id })))
    }
}

/// Read the table of **advances**: a character to millimetres.
///
/// A key that is not one character is refused, because the table of
/// runs is the table for the longer ones.
fn read_advances(table: Option<&Json>) -> Result<craftkit_sdk::serde_json::Map<String, Json>, EngineError> {
    let mut out = craftkit_sdk::serde_json::Map::new();
    let Some(table) = table else { return Ok(out) };
    let Some(table) = table.as_object() else {
        return Err(EngineError::Failed {
            id: "text.metric.add".to_string(),
            message: "the widths are a table of character to millimetres".to_string(),
        });
    };
    for (key, value) in table {
        let Some(character) = one_character(key) else {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("`{key}` is not one character"),
            });
        };
        let Some(width) = value.as_f64() else {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the width of `{character}` is not a number"),
            });
        };
        if !width.is_finite() || width < 0.0 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the width of `{character}` is not a width"),
            });
        }
        out.insert(key.clone(), json!(width));
    }
    Ok(out)
}

/// Read the table of **runs**: two or more characters to millimetres.
///
/// A run of one character is refused: that is an advance, and the
/// metric has a table for those.
fn read_runs(table: Option<&Json>) -> Result<craftkit_sdk::serde_json::Map<String, Json>, EngineError> {
    let mut out = craftkit_sdk::serde_json::Map::new();
    let Some(table) = table else { return Ok(out) };
    let Some(table) = table.as_object() else {
        return Err(EngineError::Failed {
            id: "text.metric.add".to_string(),
            message: "the runs are a table of characters to millimetres".to_string(),
        });
    };
    for (key, value) in table {
        if key.chars().count() < 2 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("`{key}` is one character, and that is an advance"),
            });
        }
        let Some(width) = value.as_f64() else {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the width of `{key}` is not a number"),
            });
        };
        if !width.is_finite() || width < 0.0 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the width of `{key}` is not a width"),
            });
        }
        out.insert(key.clone(), json!(width));
    }
    Ok(out)
}

/// Read the table of **pairs**: exactly two characters to millimetres.
///
/// The adjustment may be **negative**: that is what a kerning pair is
/// for.
fn read_pairs(table: Option<&Json>) -> Result<craftkit_sdk::serde_json::Map<String, Json>, EngineError> {
    let mut out = craftkit_sdk::serde_json::Map::new();
    let Some(table) = table else { return Ok(out) };
    let Some(table) = table.as_object() else {
        return Err(EngineError::Failed {
            id: "text.metric.add".to_string(),
            message: "the pairs are a table of two characters to millimetres".to_string(),
        });
    };
    for (key, value) in table {
        if key.chars().count() != 2 {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("`{key}` is not two characters, and that is not a pair"),
            });
        }
        let Some(adjust) = value.as_f64() else {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the adjustment of `{key}` is not a number"),
            });
        };
        if !adjust.is_finite() {
            return Err(EngineError::Failed {
                id: "text.metric.add".to_string(),
                message: format!("the adjustment of `{key}` is not a number"),
            });
        }
        out.insert(key.clone(), json!(adjust));
    }
    Ok(out)
}

/// Put a **paragraph** in the document.
///
/// The words are the text split on whitespace, and the **advance** and
/// the **space** are the metric: this kit has no font, so the caller
/// says how wide a character and a space are.
#[derive(Command)]
#[command(id = "text.paragraph.add", area = "text", layer = "text", icon = "plus", mutates, undoable)]
pub struct AddParagraph {
    /// The paragraph's text: the words are split on whitespace.
    pub text: Text,
    /// The metric it is set in: the widths of the characters.
    pub metric: NodeRef,
}

impl AddParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let metric = self.metric.0;
        let Some(metric_node) = session.document().node(metric) else {
            return Err(EngineError::Failed {
                id: "text.paragraph.add".to_string(),
                message: format!("there is no metric `{metric}`"),
            });
        };
        if metric_node.kind != "text.metric" {
            return Err(EngineError::Failed {
                id: "text.paragraph.add".to_string(),
                message: format!("the node `{metric}` is not a metric"),
            });
        }
        let metric_name = metric_node.name.clone();
        let words: Vec<String> = self.text.0.split_whitespace().map(|word| word.to_string()).collect();
        let name = name_of(&words);
        let id = session.document_mut().add_node_with_under(
            None,
            name,
            "text.paragraph",
            None,
            json!({
                "words": words,
                "measure_mm": 0.0,
                "ragged": false,
            }),
        );
        // The paragraph is set **in** the metric: an edge, and the edge
        // says which metric, so that a reader can say so without
        // resolving it.
        session.document_mut().add_relation(Relation::with_payload(
            id,
            metric,
            SET_IN,
            json!({ "metric": metric_name }),
        ));
        Ok(Outcome::changed("added a paragraph").with_data(json!({ "id": id, "metric": metric })))
    }
}

/// Set the **metric** a paragraph is set in.
///
/// The metric is a **node** and the paragraph's typeface is an **edge**,
/// so setting it is adding an edge — and taking the old one away.
#[derive(Command)]
#[command(id = "text.paragraph.metric", area = "text", layer = "text", icon = "list", mutates, undoable)]
pub struct SetMetric {
    /// The paragraph.
    pub paragraph: NodeRef,
    /// The metric it is set in.
    pub metric: NodeRef,
}

impl SetMetric {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let paragraph = self.paragraph.0;
        let metric = self.metric.0;
        let Some(node) = session.document().node(paragraph) else {
            return Err(EngineError::Failed {
                id: "text.paragraph.metric".to_string(),
                message: format!("there is no paragraph `{paragraph}`"),
            });
        };
        if node.kind != "text.paragraph" {
            return Err(EngineError::Failed {
                id: "text.paragraph.metric".to_string(),
                message: format!("the node `{paragraph}` is not a paragraph"),
            });
        }
        let Some(metric_node) = session.document().node(metric) else {
            return Err(EngineError::Failed {
                id: "text.paragraph.metric".to_string(),
                message: format!("there is no metric `{metric}`"),
            });
        };
        if metric_node.kind != "text.metric" {
            return Err(EngineError::Failed {
                id: "text.paragraph.metric".to_string(),
                message: format!("the node `{metric}` is not a metric"),
            });
        }
        let metric_name = metric_node.name.clone();
        let document = session.document_mut();
        document.remove_relations(|relation| relation.from == paragraph && relation.kind == SET_IN);
        document.add_relation(Relation::with_payload(
            paragraph,
            metric,
            SET_IN,
            json!({ "metric": metric_name.clone() }),
        ));
        Ok(Outcome::changed(format!("set the paragraph {paragraph} in the metric `{metric_name}`"))
            .with_data(json!({ "paragraph": paragraph, "metric": metric })))
    }
}

/// The name of a paragraph: its first few words.
fn name_of(words: &[String]) -> String {
    let mut name = String::new();
    for (index, word) in words.iter().take(8).enumerate() {
        if index > 0 {
            name.push(' ');
        }
        name.push_str(word);
    }
    if words.len() > 8 {
        name.push_str(" …");
    }
    if name.is_empty() { "a paragraph".to_string() } else { name }
}

/// **Measure** a paragraph: give it the width of its column.
///
/// The measure is the **place**, not the paragraph, so it is set here.
/// A paragraph with no measure has no lines, and the evaluation says
/// so with a null rather than a zero.
#[derive(Command)]
#[command(id = "text.paragraph.measure", area = "text", layer = "text", icon = "list", mutates, undoable)]
pub struct MeasureParagraph {
    /// The paragraph to measure.
    pub paragraph: NodeRef,
    /// The width of the column, in millimetres.
    pub measure_mm: Number,
    /// Whether the paragraph is set **ragged**: true leaves the lines
    /// ragged, false justifies them. Left out, the paragraph keeps the
    /// setting it has.
    pub ragged: Option<Bool>,
}

impl MeasureParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.paragraph.0;
        let measure = self.measure_mm.0;
        if !measure.is_finite() || measure < 0.0 {
            return Err(EngineError::Failed {
                id: "text.paragraph.measure".to_string(),
                message: "a measure is a width of a column, in millimetres".to_string(),
            });
        }
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "text.paragraph.measure".to_string(),
                message: format!("there is no paragraph `{id}`"),
            });
        };
        if node.kind != "text.paragraph" {
            return Err(EngineError::Failed {
                id: "text.paragraph.measure".to_string(),
                message: format!("the node `{id}` is not a paragraph"),
            });
        }
        let ragged = match self.ragged {
            Some(flag) => flag.0,
            None => ragged_of(node),
        };
        let Some(node) = session.document_mut().node_mut(id) else {
            return Err(EngineError::Failed {
                id: "text.paragraph.measure".to_string(),
                message: format!("there is no paragraph `{id}`"),
            });
        };
        node.data["measure_mm"] = json!(measure);
        node.data["ragged"] = json!(ragged);
        Ok(Outcome::changed(format!("measured the paragraph at {measure} mm"))
            .with_data(json!({ "id": id, "measure_mm": measure, "ragged": ragged })))
    }
}

/// The evaluator: the **lines** of every paragraph that has a measure.
///
/// A paragraph without a measure is not in the plan: it has no lines
/// yet, and a plan item with nothing in it is noise.
#[derive(Evaluator)]
#[evaluator(id = "text.lines", label = "Lines", layer = "text")]
pub struct LinesEvaluator;

impl LinesEvaluator {
    /// The lines of every measured paragraph.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if node.kind != "text.paragraph" {
                continue;
            }
            if measure_of(node) <= 0.0 {
                continue;
            }
            let Some(metric) = metric_of(document, node) else { continue };
            let words = words_of(node);
            let Ok(breaks) = break_lines(node, Some(&metric)) else { continue };
            let lines: Vec<Json> = breaks
                .lines
                .iter()
                .map(|line| {
                    json!({
                        "start": line.start / 2,
                        "end": (line.end / 2 + 1).min(words.len()),
                        "width": line.width,
                        "ratio": line.ratio,
                        "excess": line.excess,
                        "flagged": line.flagged,
                    })
                })
                .collect();
            plan.items.push(PlanItem {
                node: node.id,
                kind: "text.lines".to_string(),
                value: json!({
                    "lines": lines,
                    "line_count": breaks.lines.len(),
                    "method": match breaks.method {
                        Method::Optimal => "optimal",
                        Method::Greedy => "greedy",
                    },
                }),
            });
        }
        plan
    }
}

/// The panel that shows the text.
#[derive(Panel)]
#[panel(
    id = "panel.text",
    label = "Text",
    layer = "text",
    commands("text.metric.add", "text.paragraph.add", "text.paragraph.measure", "text.paragraph.metric")
)]
pub struct TextPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    TextLayer,
    ParagraphKind,
    MetricKind,
    AddMetric,
    AddParagraph,
    MeasureParagraph,
    SetMetric,
    LinesEvaluator,
    TextPanel
);

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::Registry;
    use craftkit_sdk::command::CommandSpec;
    use craftkit_sdk::document::NodeKindSpec;
    use craftkit_sdk::evaluator::EvaluatorSpec;
    use craftkit_sdk::format::{FormatSpec, MigrationSpec};

    /// A session with this battery's commands in it.
    fn session() -> Session {
        let mut commands: Registry<CommandSpec> = Registry::new();
        commands.register(ADD_METRIC.clone()).expect("the command is registered");
        commands.register(ADD_PARAGRAPH.clone()).expect("the command is registered");
        commands.register(MEASURE_PARAGRAPH.clone()).expect("the command is registered");
        commands.register(SET_METRIC.clone()).expect("the command is registered");
        let mut kinds: Registry<NodeKindSpec> = Registry::new();
        kinds.register(PARAGRAPH_KIND).expect("the kind is registered");
        kinds.register(METRIC_KIND).expect("the kind is registered");
        let mut evaluators: Registry<EvaluatorSpec> = Registry::new();
        evaluators.register(LINES_EVALUATOR).expect("the evaluator is registered");
        Session::with_formats(
            commands,
            kinds,
            Registry::<FormatSpec>::new(),
            Registry::<MigrationSpec>::new(),
            evaluators,
        )
    }

    /// A monospaced metric: one width for every character.
    fn monospaced(session: &mut Session, advance: f64) -> u64 {
        session
            .execute("text.metric.add", &json!({ "name": "mono", "default_mm": advance, "space_mm": 2.0 }))
            .expect("the metric is added");
        1
    }

    /// The words become boxes and glue, and the paragraph ends with a
    /// forced break.
    #[test]
    fn the_words_become_boxes_and_glue() {
        let metric = Metric {
            default_mm: 2.0,
            advances: HashMap::new(),
            runs: HashMap::new(),
            pairs: HashMap::new(),
            space_mm: 1.0,
        };
        let words = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let items = items_for(&words, &metric);
        assert_eq!(items.len(), 6, "a box, a glue, a box, a glue, a box and a forced break");
        assert_eq!(items[0].kind, craftkit_text_layout::Kind::Box);
        assert_eq!(items[0].width, 6.0, "three characters of two millimetres");
        assert_eq!(items[1].kind, craftkit_text_layout::Kind::Glue);
        assert_eq!(items[1].width, 1.0);
        assert_eq!(items[1].stretch, 0.5);
        assert_eq!(items[1].shrink, 1.0 / 3.0);
        assert_eq!(items[2].kind, craftkit_text_layout::Kind::Box);
        assert_eq!(items[2].width, 6.0);
        assert_eq!(items[4].kind, craftkit_text_layout::Kind::Box);
        assert_eq!(items[4].width, 10.0, "five characters of two millimetres");
        assert!(items[5].is_forced(), "the paragraph ends where it ends");
    }

    /// A character with a width of its own is as wide as it says, and
    /// the others are as wide as the default.
    #[test]
    fn a_character_with_a_width_of_its_own_is_wider() {
        let mut advances = HashMap::new();
        advances.insert('i', 1.0);
        let metric =
            Metric { default_mm: 2.0, advances, runs: HashMap::new(), pairs: HashMap::new(), space_mm: 1.0 };
        assert_eq!(metric.word_width("iii"), 3.0, "three narrow letters");
        assert_eq!(metric.word_width("abc"), 6.0, "three ordinary ones");
    }

    /// An empty paragraph is no items at all: there is nothing to break.
    #[test]
    fn an_empty_paragraph_has_no_items() {
        let metric = Metric {
            default_mm: 2.0,
            advances: HashMap::new(),
            runs: HashMap::new(),
            pairs: HashMap::new(),
            space_mm: 1.0,
        };
        assert!(items_for(&[], &metric).is_empty());
    }

    /// A measure that is negative is refused, and nothing is measured.
    #[test]
    fn a_negative_measure_is_refused() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": metric }))
            .expect("the paragraph is added");
        let error = session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": -5.0 }))
            .expect_err("a negative measure is refused");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
        let node = session.document().node(2).expect("the paragraph is there");
        assert_eq!(node.data["measure_mm"], json!(0.0), "and it was not measured");
    }

    /// A node that is not a paragraph is refused.
    #[test]
    fn a_node_that_is_not_a_paragraph_is_refused() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": metric }))
            .expect("the paragraph is added");
        let error = session
            .execute("text.paragraph.measure", &json!({ "paragraph": 99, "measure_mm": 50.0 }))
            .expect_err("there is no such paragraph");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// A measured paragraph is broken into lines, and the evaluation says
    /// which breaker ran.
    #[test]
    fn a_measured_paragraph_is_broken_into_lines() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two three four", "metric": metric }))
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 30.0 }))
            .expect("the paragraph is measured");
        let plan = session.evaluate();
        let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are there");
        assert_eq!(item.value["line_count"], json!(2), "four words of two millimetres do not fit on one");
        assert_eq!(item.value["method"], json!("optimal"));
        let lines = item.value["lines"].as_array().expect("a list of lines");
        assert_eq!(lines[0]["start"], json!(0));
        assert_eq!(lines[0]["end"], json!(3), "the first line holds three words");
        assert_eq!(lines[1]["start"], json!(3));
        assert_eq!(lines[1]["end"], json!(4), "and the second holds the last one");
    }

    /// A measure so tight that no justified line fits is broken by the
    /// **greedy** breaker, and the evaluation says which one ran.
    #[test]
    fn a_measure_that_cannot_be_justified_uses_the_greedy_breaker() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two three four", "metric": metric }))
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 20.0 }))
            .expect("the paragraph is measured");
        let plan = session.evaluate();
        let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are there");
        assert_eq!(item.value["method"], json!("greedy"));
        assert_eq!(item.value["line_count"], json!(2));
    }

    /// A ragged paragraph is broken the same way, and the setting is kept
    /// when the measure is set again without one.
    #[test]
    fn a_ragged_paragraph_is_broken_and_keeps_its_setting() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two three four", "metric": metric }))
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 30.0, "ragged": true }))
            .expect("the paragraph is measured");
        let node = session.document().node(2).expect("the paragraph is there");
        assert_eq!(node.data["ragged"], json!(true));
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 30.0 }))
            .expect("the paragraph is measured again");
        let node = session.document().node(2).expect("the paragraph is there");
        assert_eq!(node.data["ragged"], json!(true), "the setting is kept");
        let plan = session.evaluate();
        let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are there");
        assert_eq!(item.value["line_count"], json!(2));
    }

    /// A paragraph with no measure has no lines at all: the count is
    /// null, not zero.
    #[test]
    fn a_paragraph_with_no_measure_has_no_lines() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": metric }))
            .expect("the paragraph is added");
        let plan = session.evaluate();
        assert!(plan.items.iter().all(|item| item.kind != "text.lines"), "there are no lines");
        let kind = PARAGRAPH_KIND.evaluate.as_ref().expect("the kind evaluates");
        let node = session.document().node(2).expect("the paragraph is there").clone();
        let value = kind(&node, session.document()).expect("the paragraph says what it means");
        assert_eq!(value["lines"], Json::Null, "no measure, no lines");
        assert_eq!(value["words"], json!(2));
    }

    /// The metric is a **node** and the paragraph's typeface is an
    /// **edge**: the paragraph does not hold the widths.
    #[test]
    fn a_metric_is_a_node_and_a_paragraph_is_set_in_it() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": metric }))
            .expect("the paragraph is added");
        let node = session.document().node(2).expect("the paragraph is there");
        assert!(node.data.get("advance_mm").is_none(), "the widths are not in the paragraph");
        let relations = session.document().relations_from(2, Some(SET_IN));
        assert_eq!(relations.len(), 1, "the paragraph is set in the metric");
        assert_eq!(relations[0].to, metric);
        assert_eq!(relations[0].payload["metric"], json!("mono"), "and the edge says which one");
    }

    /// A paragraph that is set in another metric is set in it: the old
    /// edge goes and the new one comes.
    #[test]
    fn setting_the_metric_again_replaces_the_edge() {
        let mut session = session();
        let first = monospaced(&mut session, 2.0);
        session
            .execute("text.metric.add", &json!({ "name": "wide", "default_mm": 4.0, "space_mm": 1.0 }))
            .expect("the metric is added");
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": first }))
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.metric", &json!({ "paragraph": 3, "metric": 2 }))
            .expect("the metric is set");
        let relations = session.document().relations_from(3, Some(SET_IN));
        assert_eq!(relations.len(), 1, "there is one edge, not two");
        assert_eq!(relations[0].to, 2, "and it is the new one");
    }

    /// **Changing the metric changes the lines, and nothing in the
    /// paragraph is touched.** The lines are derived, and the dependency
    /// is not inside the node but **between** two nodes.
    #[test]
    fn changing_the_metric_changes_the_lines() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "aa bb cc dd", "metric": metric }))
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 10.0 }))
            .expect("the paragraph is measured");
        let before = session.evaluate();
        let item = before.items.iter().find(|item| item.kind == "text.lines").expect("the lines are there");
        assert_eq!(item.value["line_count"], json!(2), "two words of four millimetres fit on a line");

        // The metric is widened: the same paragraph now fits more on a
        // line, and the paragraph itself has not been touched.
        let Some(node) = session.document_mut().node_mut(metric) else { panic!("the metric is there") };
        node.data["default_mm"] = json!(0.5);
        let after = session.evaluate();
        let item = after.items.iter().find(|item| item.kind == "text.lines").expect("the lines are there");
        assert_eq!(item.value["line_count"], json!(1), "the same words now fit on one line");
        let node = session.document().node(2).expect("the paragraph is there");
        assert_eq!(node.data["words"], json!(["aa", "bb", "cc", "dd"]), "and the paragraph is the same");
    }

    /// A monospaced metric is a metric with no table: one width for
    /// every character, and no special case in the code.
    #[test]
    fn a_monospaced_metric_is_a_metric_with_an_empty_table() {
        let mut session = session();
        monospaced(&mut session, 2.0);
        let plan = session.evaluate();
        let item = plan.items.iter().find(|item| item.kind == "text.metric").expect("the metric is there");
        assert_eq!(item.value["monospaced"], json!(true));
        assert_eq!(item.value["characters"], json!(0));
    }

    /// A table's key is **one character**, and a key that is not is
    /// refused.
    #[test]
    fn a_key_that_is_not_one_character_is_refused() {
        let mut session = session();
        let error = session
            .execute(
                "text.metric.add",
                &json!({ "name": "bad", "default_mm": 2.0, "space_mm": 1.0, "advances": { "ab": 3.0 } }),
            )
            .expect_err("that is not one character");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// A width that is not a width is refused.
    #[test]
    fn a_width_that_is_not_a_width_is_refused() {
        let mut session = session();
        let error = session
            .execute(
                "text.metric.add",
                &json!({ "name": "bad", "default_mm": 2.0, "space_mm": 1.0, "advances": { "i": -1.0 } }),
            )
            .expect_err("that is not a width");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// A **run** replaces its characters: a ligature is one glyph and
    /// not two letters.
    #[test]
    fn a_run_replaces_its_characters() {
        let mut runs = HashMap::new();
        runs.insert("fi".to_string(), 3.0);
        let metric =
            Metric { default_mm: 2.0, advances: HashMap::new(), runs, pairs: HashMap::new(), space_mm: 1.0 };
        assert_eq!(metric.word_width("fi"), 3.0, "the ligature is one glyph");
        assert_eq!(metric.word_width("fl"), 4.0, "and the other pair is two letters");
    }

    /// The **longest** run wins: the reader looks for the longest one
    /// first, at every place.
    #[test]
    fn the_longest_run_wins() {
        let mut runs = HashMap::new();
        runs.insert("fi".to_string(), 3.0);
        runs.insert("fij".to_string(), 4.0);
        let metric =
            Metric { default_mm: 2.0, advances: HashMap::new(), runs, pairs: HashMap::new(), space_mm: 1.0 };
        assert_eq!(metric.word_width("fij"), 4.0, "the longer run is taken");
        assert_eq!(metric.word_width("fig"), 5.0, "and where there is none, the letters are");
    }

    /// A **pair** is an adjustment between two characters: it replaces
    /// nothing and it may be negative.
    #[test]
    fn a_pair_is_an_adjustment() {
        let mut pairs = HashMap::new();
        pairs.insert("AV".to_string(), -0.5);
        let metric =
            Metric { default_mm: 2.0, advances: HashMap::new(), runs: HashMap::new(), pairs, space_mm: 1.0 };
        assert_eq!(metric.word_width("AV"), 3.5, "the pair is closer together");
        assert_eq!(metric.word_width("VA"), 4.0, "and the other way round it is not");
    }

    /// A run and a pair may both be there: the run is taken first and
    /// the pair is applied at the boundary.
    #[test]
    fn a_run_and_a_pair_are_both_applied() {
        let mut runs = HashMap::new();
        runs.insert("fi".to_string(), 3.0);
        let mut pairs = HashMap::new();
        pairs.insert("iV".to_string(), -0.5);
        let metric = Metric { default_mm: 2.0, advances: HashMap::new(), runs, pairs, space_mm: 1.0 };
        assert_eq!(metric.word_width("fiV"), 4.5, "the ligature, and the pair after it");
    }

    /// A run of **one character** is refused: that is an advance.
    #[test]
    fn a_run_of_one_character_is_refused() {
        let mut session = session();
        let error = session
            .execute(
                "text.metric.add",
                &json!({ "name": "bad", "default_mm": 2.0, "space_mm": 1.0, "runs": { "i": 1.0 } }),
            )
            .expect_err("that is an advance, not a run");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// A pair of **three characters** is refused: that is a run.
    #[test]
    fn a_pair_of_three_characters_is_refused() {
        let mut session = session();
        let error = session
            .execute(
                "text.metric.add",
                &json!({ "name": "bad", "default_mm": 2.0, "space_mm": 1.0, "pairs": { "abc": -1.0 } }),
            )
            .expect_err("that is a run, not a pair");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }

    /// The **reader** leaves out what it does not understand: a run of
    /// one character and a pair of three are dropped, and the letters
    /// fall back to the default.
    #[test]
    fn the_reader_leaves_out_what_it_does_not_understand() {
        let mut session = session();
        session
            .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 1.0 }))
            .expect("the metric is added");
        {
            let Some(node) = session.document_mut().node_mut(1) else { panic!("the metric is there") };
            node.data["runs"] = json!({ "i": 1.0, "fi": 3.0 });
            node.data["pairs"] = json!({ "abc": -1.0, "AV": -0.5 });
        }
        let metric =
            Metric::of(session.document().node(1).expect("the metric is there")).expect("the metric is read");
        assert_eq!(metric.runs.len(), 1, "the run of one character is left out");
        assert_eq!(metric.pairs.len(), 1, "and the pair of three");
        assert_eq!(metric.word_width("fi"), 3.0, "the run that was understood is used");
        assert_eq!(metric.word_width("AV"), 3.5, "and so is the pair");
    }

    /// A paragraph that is set in a node that is not a metric is
    /// refused.
    #[test]
    fn a_paragraph_in_something_that_is_not_a_metric_is_refused() {
        let mut session = session();
        let metric = monospaced(&mut session, 2.0);
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "metric": metric }))
            .expect("the paragraph is added");
        let error = session
            .execute("text.paragraph.add", &json!({ "text": "three four", "metric": 2 }))
            .expect_err("a paragraph is not a metric");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
    }
}
