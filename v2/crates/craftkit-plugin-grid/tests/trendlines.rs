//! The **trendline** of a chart: the fit of a kind of fit to the values
//! of its series.

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

/// A sheet with two series over the categories `1`, `2`, `3` and `4`.
///
/// The first series is `sales` and it is a straight line; the second is
/// `squares` and it is a curve.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    a_text(session, sheet, "A1", "month");
    a_text(session, sheet, "B1", "sales");
    a_text(session, sheet, "C1", "squares");
    for (index, month) in ["1", "2", "3", "4"].iter().enumerate() {
        let row = index + 2;
        let step = (index + 1) as f64;
        a_text(session, sheet, &format!("A{row}"), month);
        a_number(session, sheet, &format!("B{row}"), 10.0 * step);
        a_number(session, sheet, &format!("C{row}"), step * step);
    }
    sheet
}

/// A chart over the range of a sheet.
fn a_chart(session: &mut Session, sheet: u64, range: &str) -> u64 {
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": range, "kind": "line" }))
        .expect("the chart is added");
    outcome.data["chart"].as_u64().expect("the chart has an id")
}

/// The **fit** of a kind of fit to a chart.
fn fit(
    session: &mut Session,
    chart: u64,
    kind: &str,
    extra: craftkit_sdk::serde_json::Value,
) -> craftkit_sdk::serde_json::Value {
    let mut params = json!({ "chart": chart, "kind": kind });
    if let Some(map) = extra.as_object() {
        for (key, value) in map {
            params[key.clone()] = value.clone();
        }
    }
    let outcome = session.execute("grid.chart.trendline", &params).expect("the chart is fitted");
    outcome.data["series"][0].clone()
}

/// **A straight line is fitted by a straight line.**
#[test]
fn a_straight_line_is_fitted_by_a_straight_line() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "linear", json!({}));
    assert_eq!(fit["name"], json!("sales"), "the first series is fitted");
    assert_eq!(fit["equation"], json!("y = 10x"), "the line is said: {fit:?}");
    let r2 = fit["r_squared"].as_f64().expect("the fit has an R²");
    assert!((r2 - 1.0).abs() < 1.0e-9, "a straight line is explained wholly: {r2}");
    assert_eq!(fit["used"], json!(4), "the four points are used");
    assert_eq!(fit["skipped"], json!(0), "and none is left out");
}

/// **A curve is fitted by a polynomial.**
#[test]
fn a_curve_is_fitted_by_a_polynomial() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "polynomial", json!({ "order": 2, "series": "squares" }));
    let equation = fit["equation"].as_str().expect("the fit has an equation");
    assert!(equation.starts_with("y = x²"), "the curve is said: {equation}");
    let r2 = fit["r_squared"].as_f64().expect("the fit has an R²");
    assert!((r2 - 1.0).abs() < 1.0e-9, "and it is explained wholly: {r2}");
}

/// **Every series of a chart is fitted.**
#[test]
fn every_series_of_a_chart_is_fitted() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let outcome = session
        .execute("grid.chart.trendline", &json!({ "chart": chart, "kind": "linear" }))
        .expect("the chart is fitted");
    let series = outcome.data["series"].as_array().expect("the fits are a list");
    assert_eq!(series.len(), 2, "both series are fitted: {series:?}");
    assert_eq!(series[1]["name"], json!("squares"), "the second one too");
}

/// **One series may be asked for.**
#[test]
fn one_series_may_be_asked_for() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "linear", json!({ "series": "squares" }));
    assert_eq!(fit["name"], json!("squares"), "the one that was asked for");
    let r2 = fit["r_squared"].as_f64().expect("the fit has an R²");
    assert!(r2 < 1.0, "and a curve is not explained by a line: {r2}");
}

/// **A fit of a logarithm is a fit of the logarithms.**
#[test]
fn a_fit_of_a_logarithm_is_a_fit_of_the_logarithms() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "logarithmic", json!({}));
    assert_eq!(fit["name"], json!("sales"), "the first series is fitted");
    assert_eq!(fit["skipped"], json!(0), "every point is used: {fit:?}");
}

/// **The points a fit cannot use are left out, and said.**
#[test]
fn the_points_a_fit_cannot_use_are_left_out() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    // A negative number cannot be a logarithm.
    a_number(&mut session, sheet, "B3", -5.0);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "exponential", json!({}));
    assert_eq!(fit["skipped"], json!(1), "the point is left out: {fit:?}");
    assert_eq!(fit["used"], json!(3), "and the three that can be used are used");
}

/// **A moving average is the average of the last points.**
#[test]
fn a_moving_average_is_the_average_of_the_last_points() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let fit = fit(&mut session, chart, "moving_average", json!({ "period": 2 }));
    assert_eq!(fit["equation"], json!(null), "a moving average has no equation");
    let points = fit["points"].as_array().expect("the points are a list");
    assert_eq!(points.len(), 3, "three of the four: {points:?}");
    assert_eq!(points[0]["y"], json!(15.0), "the average of ten and twenty");
    assert_eq!(points[1]["y"], json!(25.0), "the average of twenty and thirty");
    assert_eq!(points[2]["y"], json!(35.0), "and the average of thirty and forty");
}

/// **An order outside the tree's own is refused.**
#[test]
fn an_order_outside_the_trees_own_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let refusal = session
        .execute("grid.chart.trendline", &json!({ "chart": chart, "kind": "polynomial", "order": 9 }))
        .expect_err("the order is refused");
    let message = refusal.to_string();
    assert!(message.contains("between 2 and 6"), "the refusal says the bound: {message}");
}

/// **A period outside the tree's own is refused.**
#[test]
fn a_period_outside_the_trees_own_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let refusal = session
        .execute("grid.chart.trendline", &json!({ "chart": chart, "kind": "moving_average", "period": 1000 }))
        .expect_err("the period is refused");
    let message = refusal.to_string();
    assert!(message.contains("between 2 and 255"), "the refusal says the bound: {message}");
}

/// **A kind that is not one of the tree's own is refused.**
#[test]
fn a_kind_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:C5");
    let refusal = session
        .execute("grid.chart.trendline", &json!({ "chart": chart, "kind": "sine" }))
        .expect_err("the kind is refused");
    let message = refusal.to_string();
    assert!(message.contains("is not a kind of fit"), "the refusal says so: {message}");
}

/// **A series of one point cannot carry a fit, and there is no fit.**
#[test]
fn a_series_of_one_point_cannot_carry_a_fit() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet, "A1:B2");
    let refusal = session
        .execute("grid.chart.trendline", &json!({ "chart": chart, "kind": "linear" }))
        .expect_err("the fit is refused");
    let message = refusal.to_string();
    assert!(message.contains("cannot carry"), "the refusal says so: {message}");
}
