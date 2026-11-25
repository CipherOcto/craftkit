//! The **cuts of the grid battery**: the places where it stops early
//! and says so.
//!
//! A reader that stops at a bound must say that it stopped: an answer
//! that lists five samples of a field with nine is an answer that says
//! there are five. Every cut of this battery is named here, and the
//! bound it stops at is named in **code**, so that the proof and the
//! bound cannot drift apart.

use craftkit_plugin_grid::{
    MAX_EXPORT_CHARS, MAX_EXPORT_LINES, MAX_HEADER_LINES, MAX_ISSUES, MAX_RELATED, MAX_SAMPLES,
    MAX_SPARK_POINTS, MAX_SPELLING_SUGGESTIONS, MAX_SPELLING_WORD, MAX_VALUE_STEPS,
};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A **sheet** with a workbook above it, and their ids.
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **word list** of those words, one a line.
fn a_word_list(words: &[String]) -> String {
    let path = craftkit_scratch::file("craftkit-v2-grid-words.txt").expect("the scratch gives a path");
    std::fs::write(&path, words.join("\n")).expect("the word list is written");
    path.to_string_lossy().to_string()
}

/// **Words** the word list does not hold: `count` of them, each one
/// different and each one a word of three letters.
///
/// A word of three letters has no inflection the list could hold: the
/// stem of a word of three letters is two letters, and the tree does
/// not look for those.
fn unknown_words(count: usize) -> Vec<String> {
    let mut words = Vec::with_capacity(count);
    for index in 0..count {
        let first = (b'a' + (index / 26) as u8) as char;
        let second = (b'a' + (index % 26) as u8) as char;
        words.push(format!("q{first}{second}"));
    }
    words
}

/// **Words** that share a stem with `running`: `count` of them.
///
/// The stem of `running` is `runni`, and a word of the list is related
/// when it begins with the stem and is not longer than the word.
fn related_words(count: usize) -> Vec<String> {
    let mut words = Vec::with_capacity(count);
    for index in 0..count {
        let first = (b'a' + (index / 26) as u8) as char;
        let second = (b'a' + (index % 26) as u8) as char;
        words.push(format!("runni{first}{second}"));
    }
    words
}

/// The **issues** of a check of a sheet, and the answer it gave.
fn a_check(session: &mut Session, sheet: u64, dictionary: &str) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.spelling.check", &json!({ "sheet": sheet, "dictionary": dictionary }))
        .expect("the sheet is checked");
    outcome.data.clone()
}

/// The **issues** a check found are cut at `MAX_ISSUES`, and the
/// answer says so.
///
/// A sheet of more misspelled words than the bound has an answer that
/// lists the bound's number of them and says that it stopped: an
/// answer that did not say so would say the sheet has five hundred
/// issues, and it does not.
#[test]
fn the_issues_of_a_check_are_cut_and_say_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let words = unknown_words(MAX_ISSUES + 1);
    a_text(&mut session, sheet, "A1", &words.join(" "));
    let list = a_word_list(&["the".to_string(), "cat".to_string()]);
    let data = a_check(&mut session, sheet, &list);
    let issues = data["issues"].as_array().expect("the issues are a list");
    assert_eq!(issues.len(), MAX_ISSUES, "the issues are cut at the bound: {data:?}");
    assert_eq!(data["truncated"], json!(true), "the answer says it stopped: {data:?}");
}

/// A sheet of **fewer** issues than the bound is not cut, and the
/// answer says so.
///
/// The other half of the test above: a bound that is always reached
/// proves nothing, because it cannot tell a cut from a whole answer.
#[test]
fn a_check_with_few_issues_is_not_cut() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let words = unknown_words(MAX_ISSUES);
    a_text(&mut session, sheet, "A1", &words.join(" "));
    let list = a_word_list(&["the".to_string(), "cat".to_string()]);
    let data = a_check(&mut session, sheet, &list);
    let issues = data["issues"].as_array().expect("the issues are a list");
    assert_eq!(issues.len(), MAX_ISSUES, "every issue is listed: {data:?}");
    assert_eq!(data["truncated"], json!(false), "the answer says it did not stop: {data:?}");
}

/// A **word list** longer than `MAX_SPELLING_LINES` lines is cut, and
/// the answer says so.
///
/// The words beyond the cut are not read, and a word of the list that
/// is not read would be called misspelled: the answer says the list
/// was cut, so the reader knows.
#[test]
fn a_word_list_over_the_lines_is_cut_and_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "the");
    let path = craftkit_scratch::file("craftkit-v2-grid-long-words.txt").expect("the scratch gives a path");
    // One word, over and over: the **lines** are what is counted, and
    // a list of one word a line is the shortest one that has them.
    let lines = "the\n".repeat(craftkit_plugin_grid::MAX_SPELLING_LINES + 1);
    std::fs::write(&path, lines).expect("the word list is written");
    let list = path.to_string_lossy().to_string();
    let data = a_check(&mut session, sheet, &list);
    assert_eq!(data["dictionary_truncated"], json!(true), "the answer says the list was cut: {data:?}");
}

/// The **related words** of the thesaurus are cut at `MAX_RELATED`,
/// and the answer says so.
#[test]
fn the_related_words_are_cut_and_say_so() {
    let mut session = session();
    let list = a_word_list(&related_words(MAX_RELATED + 1));
    let outcome = session
        .execute("grid.thesaurus", &json!({ "word": "running", "dictionary": list }))
        .expect("the thesaurus is asked");
    let related = outcome.data["related"].as_array().expect("the related words are a list");
    assert_eq!(related.len(), MAX_RELATED, "the words are cut at the bound: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// The **samples** of a field of a pivot are cut at `MAX_SAMPLES`, and
/// the answer says so.
#[test]
fn the_samples_of_a_field_are_cut_and_say_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    for row in 2..=(MAX_SAMPLES as u32 + 2) {
        a_text(&mut session, sheet, &format!("A{row}"), &format!("name{row}"));
        a_text(&mut session, sheet, &format!("B{row}"), &format!("{row}"));
    }
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who",
                "range": format!("A1:B{}", MAX_SAMPLES + 2),
                "row": "A",
                "value": "B",
            }),
        )
        .expect("the pivot is made");
    let outcome = session
        .execute("grid.pivot.fields", &json!({ "sheet": sheet, "name": "by who" }))
        .expect("the fields are read");
    let fields = outcome.data["fields"].as_array().expect("the fields are a list");
    let who =
        fields.iter().find(|field| field["column"] == json!("A")).expect("there is a field for the column A");
    let samples = who["samples"].as_array().expect("the samples are a list");
    assert_eq!(samples.len(), MAX_SAMPLES, "the samples are cut at the bound: {who:?}");
    assert_eq!(who["samples_truncated"], json!(true), "the answer says it stopped: {who:?}");
}

/// The **steps** of an audit are cut at `MAX_VALUE_STEPS`, and the
/// answer says how many there were.
#[test]
fn the_steps_of_an_audit_are_cut_and_say_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // A chain of additions: every addition is a step of the audit.
    let formula = format!("={}", vec!["1"; MAX_VALUE_STEPS + 2].join("+"));
    let outcome = session
        .execute("grid.formula.steps", &json!({ "sheet": sheet, "formula": formula }))
        .expect("the formula is audited");
    let steps = outcome.data["steps"].as_array().expect("the steps are a list");
    assert_eq!(steps.len(), MAX_VALUE_STEPS, "the steps are cut at the bound: {outcome:?}");
    let total = outcome.data["steps_total"].as_u64().expect("the answer says how many there were");
    assert!(total > MAX_VALUE_STEPS as u64, "the answer says the whole number: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// The **lines of a header** are cut at `MAX_HEADER_LINES`, and the
/// export says how many were cut.
#[test]
fn the_lines_of_a_header_are_cut_and_the_export_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "ann");
    let lines =
        (0..MAX_HEADER_LINES + 2).map(|number| format!("line{number}")).collect::<Vec<String>>().join("\n");
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": lines }))
        .expect("the header is set");
    let path = craftkit_scratch::file("craftkit-v2-grid-header.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let cut = outcome.data["header_lines_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 2, "the lines beyond the bound are cut: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// The **text of a cell** is cut at `MAX_EXPORT_CHARS`, and the export
/// says how many cells were cut short.
#[test]
fn the_text_of_a_cell_is_cut_and_the_export_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", &"x".repeat(MAX_EXPORT_CHARS + 1));
    let path = craftkit_scratch::file("craftkit-v2-grid-text.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let cut = outcome.data["cell_texts_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 1, "the cell is cut short: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// The **points of a sparkline** are cut at `MAX_SPARK_POINTS`, and
/// the export says how many were cut.
#[test]
fn the_points_of_a_sparkline_are_cut_and_the_export_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "ann");
    // A **column** of points: every cell of the range is a point, and
    // a cell that holds nothing is a point that is missing.
    session
        .execute(
            "grid.sparkline.set",
            &json!({ "sheet": sheet, "cell": "A1", "range": format!("A1:A{}", MAX_SPARK_POINTS + 1) }),
        )
        .expect("the sparkline is set");
    let path = craftkit_scratch::file("craftkit-v2-grid-spark.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let cut = outcome.data["spark_points_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 1, "the point beyond the bound is cut: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// The **lines of a cell** are cut at `MAX_EXPORT_LINES`, and the export
/// says how many were cut.
///
/// The text of a cell is broken into lines, and the lines are cut at the
/// bound: a cell whose text is not all there is not the cell the user
/// wrote, and the export says how many lines are missing.
#[test]
fn the_lines_of_a_cell_are_cut_and_the_export_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let lines =
        (0..MAX_EXPORT_LINES + 3).map(|number| format!("line{number}")).collect::<Vec<String>>().join("\n");
    a_text(&mut session, sheet, "A1", &lines);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "wrap": true }))
        .expect("the text of the cell wraps");
    let path = craftkit_scratch::file("craftkit-v2-grid-lines.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let cut = outcome.data["cell_lines_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 3, "the lines beyond the bound are cut: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
}

/// A cell of **as many lines** as the bound allows is drawn whole, and
/// nothing is cut.
///
/// The other half of the test above: a bound that is always reached
/// proves nothing.
#[test]
fn the_lines_of_a_cell_at_the_bound_are_drawn_whole() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let lines =
        (0..MAX_EXPORT_LINES).map(|number| format!("line{number}")).collect::<Vec<String>>().join("\n");
    a_text(&mut session, sheet, "A1", &lines);
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "wrap": true }))
        .expect("the text of the cell wraps");
    let path = craftkit_scratch::file("craftkit-v2-grid-lines-whole.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let cut = outcome.data["cell_lines_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 0, "nothing was cut: {outcome:?}");
}

/// The **suggestions** of an issue are cut at
/// `MAX_SPELLING_SUGGESTIONS`, and the issue says so.
///
/// The suggestions are the alternatives the checker offers for a word,
/// and a reader that is offered six of them is not told that there were
/// more.
#[test]
fn the_suggestions_of_an_issue_are_cut_and_the_issue_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "cat");
    // The **alternatives** of `cat`: every one of them is one edit away
    // from it, so the checker finds them all.
    let alternatives = ["at", "ct", "bat", "cot", "hat", "fat", "mat"];
    let list = a_word_list(&alternatives.iter().map(|word| word.to_string()).collect::<Vec<String>>());
    let data = a_check(&mut session, sheet, &list);
    let issues = data["issues"].as_array().expect("the issues are a list");
    let issue = issues.first().expect("there is an issue for the word");
    let suggestions = issue["suggestions"].as_array().expect("the suggestions are a list");
    assert_eq!(
        suggestions.len(),
        MAX_SPELLING_SUGGESTIONS,
        "the suggestions are cut at the bound: {issue:?}"
    );
    assert_eq!(issue["suggestions_truncated"], json!(true), "the issue says it stopped: {issue:?}");
    assert_eq!(data["suggestions_truncated"], json!(true), "the answer says it stopped: {data:?}");
}

/// A word with **as many suggestions** as the bound allows is not cut,
/// and the issue says so.
#[test]
fn the_suggestions_at_the_bound_are_not_cut() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "cat");
    let alternatives = ["at", "ct", "bat", "cot", "hat", "fat"];
    let list = a_word_list(&alternatives.iter().map(|word| word.to_string()).collect::<Vec<String>>());
    let data = a_check(&mut session, sheet, &list);
    let issues = data["issues"].as_array().expect("the issues are a list");
    let issue = issues.first().expect("there is an issue for the word");
    let suggestions = issue["suggestions"].as_array().expect("the suggestions are a list");
    assert_eq!(suggestions.len(), MAX_SPELLING_SUGGESTIONS, "every suggestion is offered: {issue:?}");
    assert_eq!(issue["suggestions_truncated"], json!(false), "nothing was cut: {issue:?}");
    assert_eq!(data["suggestions_truncated"], json!(false), "nothing was cut: {data:?}");
}

/// A word **longer** than `MAX_SPELLING_WORD` gets no suggestions at
/// all, and the issue says so.
///
/// The candidates of a word grow with the square of its length, so a
/// long word is not looked for: a word with no suggestions because none
/// were looked for is not a word with no alternatives.
#[test]
fn a_long_word_gets_no_suggestions_and_the_issue_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let word = "a".repeat(MAX_SPELLING_WORD + 1);
    a_text(&mut session, sheet, "A1", &word);
    let list = a_word_list(&["the".to_string(), "cat".to_string()]);
    let data = a_check(&mut session, sheet, &list);
    let issues = data["issues"].as_array().expect("the issues are a list");
    let issue = issues.first().expect("there is an issue for the word");
    let suggestions = issue["suggestions"].as_array().expect("the suggestions are a list");
    assert!(suggestions.is_empty(), "no suggestions are offered: {issue:?}");
    assert_eq!(issue["suggestions_omitted"], json!(true), "the issue says they were not made: {issue:?}");
    assert_eq!(data["suggestions_omitted"], json!(true), "the answer says so: {data:?}");
}

/// The **solver** that runs out of its budget says so.
///
/// The search of the solver stops at the bound of the tree, and a
/// solution that was not proved to be the best one is not "optimal":
/// the answer says that the search stopped.
#[test]
fn the_solver_that_runs_out_of_its_budget_says_so() {
    let mut session = session();
    let outcome = session
        .execute(
            "grid.solver.solve",
            &json!({
                "objective": [3.0, 2.0, 2.0, 1.0],
                "binary": [true, true, true, true],
                "constraints": [{ "coefficients": [2.0, 2.0, 2.0, 2.0], "sense": "<=", "rhs": 5.0 }],
                "maximise": true,
                "max_nodes": 2.0,
            }),
        )
        .expect("the program is solved");
    assert_ne!(outcome.data["status"], json!("optimal"), "the answer is not a lie: {outcome:?}");
    assert_eq!(outcome.data["converged"], json!(false), "the answer says it stopped: {outcome:?}");
}

/// The **solver** that proves its answer says so.
#[test]
fn the_solver_that_proves_its_answer_says_so() {
    let mut session = session();
    let outcome = session
        .execute(
            "grid.solver.solve",
            &json!({
                "objective": [3.0, 2.0, 2.0, 1.0],
                "binary": [true, true, true, true],
                "constraints": [{ "coefficients": [1.0, 1.0, 1.0, 1.0], "sense": "<=", "rhs": 3.0 }],
                "maximise": true,
            }),
        )
        .expect("the program is solved");
    assert_eq!(outcome.data["status"], json!("optimal"), "the answer is proved: {outcome:?}");
    assert_eq!(outcome.data["converged"], json!(true), "the answer says it stopped: {outcome:?}");
}
