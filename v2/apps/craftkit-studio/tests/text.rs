//! The text battery: a paragraph, laid out into lines.
//!
//! What is proved here: the words are the payload, the measure is the
//! paragraph's **place** and not the paragraph, the **metric** is a node
//! and the paragraph's typeface is an **edge**, the lines are
//! **derived**, and the kit has no font — so the widths come from the
//! caller.

use craftkit_sdk::Ecosystem;
#[cfg(feature = "text")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "text")]
#[test]
fn the_text_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("text.paragraph.add").is_some(), "the battery is in");
    assert!(ecosystem.commands.get("text.paragraph.measure").is_some());
    assert!(ecosystem.commands.get("text.metric.add").is_some());
    assert!(ecosystem.commands.get("text.paragraph.metric").is_some());
    assert!(ecosystem.kinds.get("text.paragraph").is_some());
    assert!(ecosystem.kinds.get("text.metric").is_some());
    assert!(ecosystem.evaluators.get("text.lines").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "text"))]
#[test]
fn the_text_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("text.paragraph.add").is_none(), "the battery is out");
}

/// The words are the paragraph's **payload**: they are numerous, and
/// what is numerous is in the payload.
#[cfg(feature = "text")]
#[test]
fn the_words_are_the_payload() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "the quick brown fox", "metric": 1 }))
        .expect("the paragraph is added");
    let node = session.document().node(2).expect("the paragraph is there");
    assert_eq!(node.kind, "text.paragraph");
    assert_eq!(node.data["words"], json!(["the", "quick", "brown", "fox"]));
}

/// A paragraph with **no measure** has no lines: it has not been
/// placed. The count is null, not zero.
#[cfg(feature = "text")]
#[test]
fn a_paragraph_with_no_measure_has_no_lines() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "text.paragraph").expect("the paragraph is in the plan");
    assert_eq!(item.value["lines"], json!(null), "no measure, no lines");
}

/// Measured, the paragraph is broken into lines — and the lines are
/// **derived**, not stored.
#[cfg(feature = "text")]
#[test]
fn a_measured_paragraph_is_broken_into_lines() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "aa bb cc dd", "metric": 1 }))
        .expect("the paragraph is added");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 10.0 }))
        .expect("the paragraph is measured");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    assert_eq!(item.value["line_count"], json!(2), "two words of four millimetres each");
    assert_eq!(item.value["method"], json!("optimal"));
    let lines = item.value["lines"].as_array().expect("the lines are a list");
    assert_eq!(lines[0]["start"], json!(0));
    assert_eq!(lines[0]["end"], json!(2), "the first line holds the first two words");
    assert_eq!(lines[1]["start"], json!(2));
    assert_eq!(lines[1]["end"], json!(4));
    // The lines are derived: the document does not hold them.
    let node = session.document().node(2).expect("the paragraph is there");
    assert!(node.data.get("lines").is_none(), "the lines are not in the document");
}

/// The **measure** is the paragraph's place, so it is set by a command
/// and not by the paragraph's own text.
#[cfg(feature = "text")]
#[test]
fn the_measure_is_set_by_a_command() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 20.0 }))
        .expect("the paragraph is measured");
    let node = session.document().node(2).expect("the paragraph is there");
    assert_eq!(node.data["measure_mm"], json!(20.0));
    assert_eq!(node.data["ragged"], json!(false), "justified by default");
}

/// A **negative** measure is refused: a measure is a width.
#[cfg(feature = "text")]
#[test]
fn a_negative_measure_is_refused() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    let error = session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": -5.0 }))
        .expect_err("that is not a measure");
    assert!(error.to_string().contains("measure"), "the refusal says what is wrong: {error}");
}

/// A node that is not a paragraph is refused.
#[cfg(feature = "text")]
#[test]
fn a_node_that_is_not_a_paragraph_is_refused() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    let error = session
        .execute("text.paragraph.measure", &json!({ "paragraph": 99, "measure_mm": 20.0 }))
        .expect_err("there is no such paragraph");
    assert!(error.to_string().contains("paragraph"), "the refusal says what is wrong: {error}");
}

/// The **ragged** setting is the paragraph's, and it is kept when the
/// measure is set again without it.
#[cfg(feature = "text")]
#[test]
fn the_ragged_setting_is_kept() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two three", "metric": 1 }))
        .expect("the paragraph is added");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 20.0, "ragged": true }))
        .expect("the paragraph is measured");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 30.0 }))
        .expect("the paragraph is measured again");
    let node = session.document().node(2).expect("the paragraph is there");
    assert_eq!(node.data["ragged"], json!(true), "the setting is kept");
    assert_eq!(node.data["measure_mm"], json!(30.0));
}

/// The kit has **no font**: the widths come from the caller, and the
/// **metric** is where they live.
#[cfg(feature = "text")]
#[test]
fn the_widths_come_from_the_caller() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    let node = session.document().node(1).expect("the metric is there");
    assert_eq!(node.kind, "text.metric");
    assert_eq!(node.data["default_mm"], json!(2.0), "the advance is the metric");
    assert_eq!(node.data["space_mm"], json!(2.0));
}

/// A **metric** is a node and a paragraph's typeface is an **edge**: the
/// paragraph does not hold the widths.
#[cfg(feature = "text")]
#[test]
fn the_metric_is_a_node_and_the_paragraph_is_set_in_it() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "one two", "metric": 1 }))
        .expect("the paragraph is added");
    let node = session.document().node(2).expect("the paragraph is there");
    assert!(node.data.get("advance_mm").is_none(), "the widths are not in the paragraph");
    let relations = session.document().relations_from(2, Some("text.set_in"));
    assert_eq!(relations.len(), 1, "the paragraph is set in the metric");
    assert_eq!(relations[0].to, 1);
    assert_eq!(relations[0].payload["metric"], json!("mono"), "and the edge says which one");
}

/// **Changing the metric changes the lines, and the paragraph is not
/// touched.** The lines are derived, and the dependency is not inside
/// the node but **between** two nodes.
#[cfg(feature = "text")]
#[test]
fn changing_the_metric_changes_the_lines() {
    let mut session = ecosystem().session();
    session
        .execute("text.metric.add", &json!({ "name": "mono", "default_mm": 2.0, "space_mm": 2.0 }))
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "aa bb cc dd", "metric": 1 }))
        .expect("the paragraph is added");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 10.0 }))
        .expect("the paragraph is measured");
    let before = session.evaluate();
    let item = before.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    assert_eq!(item.value["line_count"], json!(2), "two words of four millimetres fit on a line");

    // The metric is narrowed: the same paragraph now fits more on a
    // line, and the paragraph itself has not been touched.
    let Some(node) = session.document_mut().node_mut(1) else { panic!("the metric is there") };
    node.data["default_mm"] = json!(0.5);
    let after = session.evaluate();
    let item = after.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    assert_eq!(item.value["line_count"], json!(1), "the same words now fit on one line");
    let node = session.document().node(2).expect("the paragraph is there");
    assert_eq!(node.data["words"], json!(["aa", "bb", "cc", "dd"]), "and the paragraph is the same");
}

/// A character with a **width of its own** is as wide as it says, and
/// the others are as wide as the default.
#[cfg(feature = "text")]
#[test]
fn a_character_with_a_width_of_its_own_is_wider() {
    let mut session = ecosystem().session();
    session
        .execute(
            "text.metric.add",
            &json!({
                "name": "mixed",
                "default_mm": 2.0,
                "space_mm": 2.0,
                "advances": { "i": 1.0 },
            }),
        )
        .expect("the metric is added");
    session
        .execute("text.paragraph.add", &json!({ "text": "iii abc", "metric": 1 }))
        .expect("the paragraph is added");
    session
        .execute("text.paragraph.measure", &json!({ "paragraph": 2, "measure_mm": 8.0 }))
        .expect("the paragraph is measured");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "text.lines").expect("the lines are in the plan");
    let lines = item.value["lines"].as_array().expect("the lines are a list");
    // "iii" is three millimetres wide, "abc" is six.
    assert_eq!(lines[0]["width"], json!(3.0), "the narrow letters are narrow");
}
