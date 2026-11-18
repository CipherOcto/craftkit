//! The **chart** of a sheet: a picture of a range, whose data is
//! derived.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **chart** of a range.
fn a_chart(session: &mut Session, sheet: u64, range: &str) -> u64 {
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": range }))
        .expect("the chart is added");
    outcome.data["chart"].as_u64().expect("the chart has an id")
}

/// The **chart**, as it is read.
fn chart_of(session: &mut Session, chart: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.chart.read", &json!({ "chart": chart })).expect("the chart is read");
    outcome.data.clone()
}

/// **A chart may be added, and it says what it reads.**
#[test]
fn a_chart_may_be_added() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:B4");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["range"], json!("A1:B4"));
    assert_eq!(read["kind"], json!("column"));
}

/// **The first row names the series and the first column holds the
/// categories.**
#[test]
fn the_data_is_the_shape_of_the_range() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    a_number(&mut session, sheet, "B2", 3.0);
    a_text(&mut session, sheet, "A3", "bob");
    a_number(&mut session, sheet, "B3", 5.0);
    let chart = a_chart(&mut session, sheet, "A1:B3");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["data"]["categories"], json!(["ann", "bob"]));
    let series = read["data"]["series"].as_array().expect("the series are a list");
    assert_eq!(series.len(), 1);
    assert_eq!(series[0]["name"], json!("amount"));
    assert_eq!(series[0]["values"], json!([3.0, 5.0]));
}

/// **The data is derived: it follows the sheet.**
#[test]
fn the_data_follows_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    a_number(&mut session, sheet, "B2", 3.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    a_number(&mut session, sheet, "B2", 7.0);
    let read = chart_of(&mut session, chart);
    let series = read["data"]["series"].as_array().expect("the series are a list");
    assert_eq!(series[0]["values"], json!([7.0]), "the chart is not out of date");
}

/// **A value that is not a number is nothing, and not a zero.**
#[test]
fn a_value_that_is_not_a_number_is_nothing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    a_text(&mut session, sheet, "B2", "many");
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let read = chart_of(&mut session, chart);
    let series = read["data"]["series"].as_array().expect("the series are a list");
    assert_eq!(series[0]["values"], json!([null]), "a text is not a zero");
}

/// **A range of one row or one column is refused.**
#[test]
fn a_range_of_one_row_or_one_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for range in ["A1:B1", "A1:A4"] {
        let outcome = session.execute("grid.chart.add", &json!({ "sheet": sheet, "range": range }));
        assert!(outcome.is_err(), "the range {range} is refused");
    }
}

/// **A kind that is not a kind is refused.**
#[test]
fn a_kind_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:B4", "kind": "doughnut" }));
    assert!(outcome.is_err(), "`doughnut` is not a kind of chart");
}

/// **The kind, the title and the range may be changed.**
#[test]
fn the_chart_may_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "amount");
    a_text(&mut session, sheet, "A2", "ann");
    a_number(&mut session, sheet, "B2", 3.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.set", &json!({ "chart": chart, "kind": "pie", "title": "the amounts" }))
        .expect("the chart is changed");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["kind"], json!("pie"));
    assert_eq!(read["title"], json!("the amounts"));
    session
        .execute("grid.chart.set", &json!({ "chart": chart, "range": "A1:B3" }))
        .expect("the range is changed");
    assert_eq!(chart_of(&mut session, chart)["range"], json!("A1:B3"));
}

/// **A chart may be taken away.**
#[test]
fn a_chart_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:B4");
    session.execute("grid.chart.remove", &json!({ "chart": chart })).expect("the chart is taken away");
    let outcome = session.execute("grid.chart.read", &json!({ "chart": chart }));
    assert!(outcome.is_err(), "the chart is not there");
}

/// **A node that is not a chart is refused.**
#[test]
fn a_node_that_is_not_a_chart_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.chart.read", &json!({ "chart": sheet }));
    assert!(outcome.is_err(), "a sheet is not a chart");
}

/// **A chart may read its range the other way round**: the first row
/// holds the categories and the rows after it are the series.
#[test]
fn a_chart_may_read_its_range_by_rows() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "ann");
    a_text(&mut session, sheet, "C1", "bob");
    a_text(&mut session, sheet, "A2", "amount");
    a_number(&mut session, sheet, "B2", 3.0);
    a_number(&mut session, sheet, "C2", 5.0);
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:C2", "switch_row_column": true }))
        .expect("the chart is added");
    let chart = outcome.data["chart"].as_u64().expect("the chart has an id");
    let read = chart_of(&mut session, chart);
    // The **categories** are the cells of the first row, and the
    // **series** are the rows after it.
    assert_eq!(read["data"]["categories"], json!(["ann", "bob"]));
    let series = read["data"]["series"].as_array().expect("the series are a list");
    assert_eq!(series.len(), 1);
    assert_eq!(series[0]["name"], json!("amount"));
    assert_eq!(series[0]["values"], json!([3.0, 5.0]));
}

/// **The way a chart reads its range may be changed after it is made.**
#[test]
fn the_way_a_chart_reads_its_range_may_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "who");
    a_text(&mut session, sheet, "B1", "ann");
    a_text(&mut session, sheet, "A2", "amount");
    a_number(&mut session, sheet, "B2", 3.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    // By **columns**: the categories are the first column.
    let read = chart_of(&mut session, chart);
    assert_eq!(read["data"]["categories"], json!(["amount"]));
    // Switched: the categories are the first row.
    session
        .execute("grid.chart.set", &json!({ "chart": chart, "switch_row_column": true }))
        .expect("the chart is set");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["data"]["categories"], json!(["ann"]));
}

/// A chart's **series may have a colour**: the list is the tree's own,
/// and a chart with fewer colours than series takes them again.
#[test]
fn the_series_of_a_chart_may_have_a_colour() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "name");
    a_text(&mut session, sheet, "B1", "one");
    a_text(&mut session, sheet, "C1", "two");
    a_text(&mut session, sheet, "A2", "ann");
    a_number(&mut session, sheet, "B2", 1.0);
    a_number(&mut session, sheet, "C2", 2.0);
    let chart = a_chart(&mut session, sheet, "A1:C2");
    session
        .execute("grid.chart.set", &json!({ "chart": chart, "colours": ["#ff0000", "#00ff00"] }))
        .expect("the colours are set");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["colours"][0].as_str(), Some("#ff0000"), "the first colour is kept");
    assert_eq!(read["seriesColours"][0].as_str(), Some("#ff0000"), "the first series has it");
    assert_eq!(read["seriesColours"][1].as_str(), Some("#00ff00"), "and the second the other");
    // A chart with **one** colour and two series: the colour is taken
    // again.
    session
        .execute("grid.chart.set", &json!({ "chart": chart, "colours": ["#0000ff"] }))
        .expect("the colours are set");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["seriesColours"][0].as_str(), Some("#0000ff"), "the first series has it");
    assert_eq!(read["seriesColours"][1].as_str(), Some("#0000ff"), "and the second the same");
}

/// The colours of a chart are **colours**: a colour that is not one is
/// refused.
#[test]
fn the_colours_of_a_chart_are_colours() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "name");
    a_text(&mut session, sheet, "B1", "one");
    a_text(&mut session, sheet, "A2", "ann");
    a_number(&mut session, sheet, "B2", 1.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let failure = session
        .execute("grid.chart.set", &json!({ "chart": chart, "colours": ["red"] }))
        .expect_err("the colour is refused");
    let message = failure.to_string();
    assert!(message.contains("is not a colour"), "the refusal says why: {message}");
    let failure = session
        .execute("grid.chart.set", &json!({ "chart": chart, "colours": "red" }))
        .expect_err("the colours are refused");
    let message = failure.to_string();
    assert!(message.contains("a list of them"), "the refusal says why: {message}");
}
