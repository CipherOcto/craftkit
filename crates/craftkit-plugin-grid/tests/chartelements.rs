//! The **elements** of a chart: its title, its legend, its data
//! labels, its gridlines, its axis titles and its smooth line.

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

/// **A chart is made with the tree's own elements.**
#[test]
fn a_chart_is_made_with_the_trees_own_elements() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["legend"], json!("bottom"), "the legend is at the bottom: {read:?}");
    assert_eq!(read["dataLabels"], json!(false), "the data labels are off: {read:?}");
    assert_eq!(read["gridlines"], json!(true), "the gridlines are on: {read:?}");
    assert_eq!(read["xTitle"], json!(null), "the axes have no titles: {read:?}");
    assert_eq!(read["smooth"], json!(false), "the line is not smooth: {read:?}");
}

/// **A pie has its legend at the right and no gridlines.**
#[test]
fn a_pie_has_its_legend_at_the_right_and_no_gridlines() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:B2", "kind": "pie" }))
        .expect("the chart is added");
    let chart = outcome.data["chart"].as_u64().expect("the chart has an id");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["legend"], json!("right"), "the legend is at the right: {read:?}");
    assert_eq!(read["gridlines"], json!(false), "and there are no gridlines: {read:?}");
}

/// **The legend is moved, and it is taken away.**
#[test]
fn the_legend_is_moved_and_it_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "legend", "position": "top" }))
        .expect("the legend is moved");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["legend"], json!("top"), "the legend is at the top: {read:?}");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "legend", "on": false }))
        .expect("the legend is taken away");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["legend"], json!("none"), "and there is no legend: {read:?}");
}

/// **The data labels are turned on and the gridlines off.**
#[test]
fn the_data_labels_are_turned_on_and_the_gridlines_off() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "dataLabels", "on": true }))
        .expect("the data labels are on");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "gridlines", "on": false }))
        .expect("the gridlines are off");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["dataLabels"], json!(true), "the data labels are on: {read:?}");
    assert_eq!(read["gridlines"], json!(false), "the gridlines are off: {read:?}");
}

/// **The title is taken away and given back.**
#[test]
fn the_title_is_taken_away_and_given_back() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "title", "on": false }))
        .expect("the title is taken away");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["title"], json!(null), "the title is gone: {read:?}");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "title", "on": true }))
        .expect("the title is given back");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["title"], json!("Chart Title"), "and it is the tree's: {read:?}");
}

/// **The titles of the axes are given and taken away.**
#[test]
fn the_titles_of_the_axes_are_given_and_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "axisTitles", "on": true }))
        .expect("the axes have titles");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["xTitle"], json!("Axis Title"), "the axis of the categories: {read:?}");
    assert_eq!(read["yTitle"], json!("Axis Title"), "and the axis of the values: {read:?}");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "axisTitles", "on": false }))
        .expect("the axes have no titles");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["xTitle"], json!(null), "the titles are gone: {read:?}");
    assert_eq!(read["yTitle"], json!(null), "and both of them: {read:?}");
}

/// **The line is made smooth.**
#[test]
fn the_line_is_made_smooth() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "smooth", "on": true }))
        .expect("the line is smooth");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["smooth"], json!(true), "the line is smooth: {read:?}");
}

/// **An element that is not one of the tree's own is refused.**
#[test]
fn an_element_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let refusal = session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "shadow" }))
        .expect_err("the element is refused");
    let message = refusal.to_string();
    assert!(message.contains("element"), "the refusal says so: {message}");
}

/// **A place for the legend that is not one of the tree's own is
/// refused.**
#[test]
fn a_place_for_the_legend_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let refusal = session
        .execute("grid.chart.element", &json!({ "chart": chart, "element": "legend", "position": "middle" }))
        .expect_err("the place is refused");
    let message = refusal.to_string();
    assert!(message.contains("legend"), "the refusal says so: {message}");
}

/// **A chart may be made with its elements said.**
#[test]
fn a_chart_may_be_made_with_its_elements_said() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "B1", 2.0);
    a_number(&mut session, sheet, "A2", 3.0);
    a_number(&mut session, sheet, "B2", 4.0);
    let outcome = session
        .execute(
            "grid.chart.add",
            &json!({
                "sheet": sheet,
                "range": "A1:B2",
                "legend": "left",
                "data_labels": true,
                "gridlines": false,
                "x_title": "the categories",
                "y_title": "the values",
            }),
        )
        .expect("the chart is added");
    let chart = outcome.data["chart"].as_u64().expect("the chart has an id");
    let read = chart_of(&mut session, chart);
    assert_eq!(read["legend"], json!("left"), "the legend is at the left: {read:?}");
    assert_eq!(read["dataLabels"], json!(true), "the data labels are on: {read:?}");
    assert_eq!(read["gridlines"], json!(false), "the gridlines are off: {read:?}");
    assert_eq!(read["xTitle"], json!("the categories"), "the axis is named: {read:?}");
    assert_eq!(read["yTitle"], json!("the values"), "and so is the other: {read:?}");
}
