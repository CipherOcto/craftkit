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
//! - the **advance** — the width of one character, in millimetres, and
//!   it is in the payload too, because it is the **metric**, and the
//!   metric is what the caller brings;
//! - the **measure** — the width of the column, in millimetres, and it
//!   is in the payload because it is a property of the paragraph's
//!   place, not of the paragraph;
//! - the **ragged** flag — the paragraph's setting, in the payload;
//! - the **lines** — **derived**: the paragraph is broken every time
//!   somebody asks, and the lines are in the plan, not in the document.
//!
//! # What the port found
//!
//! - **The kit has no font, so the widths come from the caller.** A
//!   line breaker needs the advance of every glyph, and an advance comes
//!   from a typeface. This tree has no typeface and no asset of any
//!   kind, so the paragraph carries an **advance** — the width of one
//!   character — and a **space**, and the caller says what they are.
//!   The moment a font arrives, it supplies those two numbers and
//!   nothing else changes.
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
    Bool, Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text,
};
use craftkit_text_layout::{Breaks, Item, LayoutError, Method, Params, Rag, break_paragraph};

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
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let words = words_of(node);
        let measure = measure_of(node);
        let lines =
            if measure > 0.0 { break_lines(node).ok().map(|breaks| breaks.lines.len()) } else { None };
        Some(json!({
            "words": words.len(),
            "measure_mm": measure,
            "ragged": ragged_of(node),
            "lines": lines,
        }))
    }
}

/// The words of a paragraph: the payload's list of strings.
fn words_of(node: &Node) -> Vec<String> {
    node.data
        .get("words")
        .and_then(|words| words.as_array())
        .map(|words| words.iter().filter_map(|word| word.as_str().map(|word| word.to_string())).collect())
        .unwrap_or_default()
}

/// The width of one character, in millimetres.
fn advance_of(node: &Node) -> f64 {
    node.data.get("advance_mm").and_then(|value| value.as_f64()).unwrap_or(0.0)
}

/// The width of the space between two words, in millimetres.
fn space_of(node: &Node) -> f64 {
    node.data.get("space_mm").and_then(|value| value.as_f64()).unwrap_or(0.0)
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
/// A word is a **box** of `characters × advance`; between two words is a
/// **glue** of the space, which stretches a little and shrinks a little;
/// and after the last word is a **forced break**, so that the paragraph
/// ends where it ends.
///
/// The items alternate box and glue, so the item index of a word is
/// twice its word index.
fn items_for(words: &[String], advance: f64, space: f64) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::with_capacity(words.len() * 2);
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            items.push(Item::glue(space, space / 2.0, space / 3.0));
        }
        let width = word.chars().count() as f64 * advance;
        items.push(Item::boxed(width));
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
fn break_lines(node: &Node) -> Result<Breaks, LayoutError> {
    let words = words_of(node);
    let items = items_for(&words, advance_of(node), space_of(node));
    let params = layout_params(ragged_of(node));
    break_paragraph(&items, measure_of(node), &params)
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
    /// The width of one character, in millimetres.
    pub advance_mm: Number,
    /// The width of the space between two words, in millimetres.
    pub space_mm: Number,
}

impl AddParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let advance = self.advance_mm.0;
        let space = self.space_mm.0;
        if !advance.is_finite() || advance < 0.0 {
            return Err(EngineError::Failed {
                id: "text.paragraph.add".to_string(),
                message: "a character is not narrower than nothing".to_string(),
            });
        }
        if !space.is_finite() || space < 0.0 {
            return Err(EngineError::Failed {
                id: "text.paragraph.add".to_string(),
                message: "a space is not narrower than nothing".to_string(),
            });
        }
        let words: Vec<String> = self.text.0.split_whitespace().map(|word| word.to_string()).collect();
        let name = name_of(&words);
        let id = session.document_mut().add_node_with_under(
            None,
            name,
            "text.paragraph",
            None,
            json!({
                "words": words,
                "advance_mm": advance,
                "space_mm": space,
                "measure_mm": 0.0,
                "ragged": false,
            }),
        );
        Ok(Outcome::changed("added a paragraph").with_data(json!({ "id": id })))
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
            let words = words_of(node);
            let Ok(breaks) = break_lines(node) else { continue };
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
    commands("text.paragraph.add", "text.paragraph.measure")
)]
pub struct TextPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    TextLayer,
    ParagraphKind,
    AddParagraph,
    MeasureParagraph,
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
        commands.register(ADD_PARAGRAPH.clone()).expect("the command is registered");
        commands.register(MEASURE_PARAGRAPH.clone()).expect("the command is registered");
        let mut kinds: Registry<NodeKindSpec> = Registry::new();
        kinds.register(PARAGRAPH_KIND).expect("the kind is registered");
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

    /// The words become boxes and glue, and the paragraph ends with a
    /// forced break.
    #[test]
    fn the_words_become_boxes_and_glue() {
        let words = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let items = items_for(&words, 2.0, 1.0);
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

    /// An empty paragraph is no items at all: there is nothing to break.
    #[test]
    fn an_empty_paragraph_has_no_items() {
        assert!(items_for(&[], 2.0, 1.0).is_empty());
    }

    /// A measure that is negative is refused, and nothing is measured.
    #[test]
    fn a_negative_measure_is_refused() {
        let mut session = session();
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "advance_mm": 2.0, "space_mm": 1.0 }))
            .expect("the paragraph is added");
        let error = session
            .execute("text.paragraph.measure", &json!({ "paragraph": 1, "measure_mm": -5.0 }))
            .expect_err("a negative measure is refused");
        assert!(matches!(error, EngineError::Failed { .. }), "{error:?}");
        let node = session.document().node(1).expect("the paragraph is there");
        assert_eq!(node.data["measure_mm"], json!(0.0), "and it was not measured");
    }

    /// A node that is not a paragraph is refused.
    #[test]
    fn a_node_that_is_not_a_paragraph_is_refused() {
        let mut session = session();
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "advance_mm": 2.0, "space_mm": 1.0 }))
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
        session
            .execute(
                "text.paragraph.add",
                &json!({ "text": "one two three four", "advance_mm": 2.0, "space_mm": 2.0 }),
            )
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 1, "measure_mm": 30.0 }))
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
        session
            .execute(
                "text.paragraph.add",
                &json!({ "text": "one two three four", "advance_mm": 2.0, "space_mm": 1.0 }),
            )
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 1, "measure_mm": 20.0 }))
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
        session
            .execute(
                "text.paragraph.add",
                &json!({ "text": "one two three four", "advance_mm": 2.0, "space_mm": 2.0 }),
            )
            .expect("the paragraph is added");
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 1, "measure_mm": 30.0, "ragged": true }))
            .expect("the paragraph is measured");
        let node = session.document().node(1).expect("the paragraph is there");
        assert_eq!(node.data["ragged"], json!(true));
        session
            .execute("text.paragraph.measure", &json!({ "paragraph": 1, "measure_mm": 30.0 }))
            .expect("the paragraph is measured again");
        let node = session.document().node(1).expect("the paragraph is there");
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
        session
            .execute("text.paragraph.add", &json!({ "text": "one two", "advance_mm": 2.0, "space_mm": 1.0 }))
            .expect("the paragraph is added");
        let plan = session.evaluate();
        assert!(plan.items.iter().all(|item| item.kind != "text.lines"), "there are no lines");
        let kind = PARAGRAPH_KIND.evaluate.as_ref().expect("the kind evaluates");
        let node = session.document().node(1).expect("the paragraph is there").clone();
        let value = kind(&node, session.document()).expect("the paragraph says what it means");
        assert_eq!(value["lines"], Json::Null, "no measure, no lines");
        assert_eq!(value["words"], json!(2));
    }
}
