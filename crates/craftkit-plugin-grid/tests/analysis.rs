//! The **analysis reports**: a statistic of a range, written as a table.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **workbook** with a **sheet** in it, and their ids.
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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **cell**: what it holds.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// The **value** a cell holds or computes.
fn computed(session: &mut Session, sheet: u64, cell: &str) -> Json {
    cell_of(session, sheet, cell)["value"].clone()
}

/// The **formula** a cell holds.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    cell_of(session, sheet, cell)["formula"].as_str().unwrap_or_default().to_string()
}

/// The **text** a cell holds.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    cell_of(session, sheet, cell)["value"].as_str().unwrap_or_default().to_string()
}

/// Five numbers in a column: `1, 2, 3, 4, 5`.
fn a_column(session: &mut Session, sheet: u64) {
    for row in 1..=5 {
        value(session, sheet, &format!("A{row}"), json!(row));
    }
}

/// The **report**: the outcome of the descriptive statistics.
fn report(session: &mut Session, sheet: u64, extra: Json) -> Json {
    let mut params = json!({ "sheet": sheet, "range": "A1:A5" });
    if let (Some(target), Some(source)) = (params.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    session.execute("grid.analysis.descriptive", &params).expect("the report is made").data.clone()
}

/// The **number** a cell computes.
fn number(session: &mut Session, sheet: u64, cell: &str) -> f64 {
    computed(session, sheet, cell).as_f64().unwrap_or(f64::NAN)
}

#[test]
fn the_report_of_a_column_goes_to_a_new_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = report(&mut session, sheet, json!({}));
    assert_eq!(data["name"], "Descriptive");
    let report_sheet = data["sheet"].as_u64().expect("the report has a sheet");
    // The **name** of the variable, and the **mean** of it.
    assert_eq!(text_of(&mut session, report_sheet, "A1"), "Column1");
    assert_eq!(text_of(&mut session, report_sheet, "A3"), "Mean");
    assert_eq!(number(&mut session, report_sheet, "B3"), 3.0);
}

#[test]
fn the_report_is_live() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = report(&mut session, sheet, json!({}));
    let report_sheet = data["sheet"].as_u64().expect("the report has a sheet");
    // The value is a **formula** over the data, and it names the sheet
    // the data is on.
    assert_eq!(formula_of(&mut session, report_sheet, "B3"), "=AVERAGE(Sheet1!$A$1:$A$5)");
    // An edit of the data is an edit of the report.
    value(&mut session, sheet, "A1", json!(10.0));
    assert_eq!(number(&mut session, report_sheet, "B3"), 4.8);
}

#[test]
fn the_report_goes_to_a_range_of_the_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = report(&mut session, sheet, json!({ "output": "D1" }));
    assert_eq!(data["sheet"].as_u64(), Some(sheet));
    assert_eq!(text_of(&mut session, sheet, "D3"), "Mean");
    assert_eq!(number(&mut session, sheet, "E3"), 3.0);
    // The formula is **not** qualified: the data is on the same sheet.
    assert_eq!(formula_of(&mut session, sheet, "E3"), "=AVERAGE($A$1:$A$5)");
}

#[test]
fn the_report_refuses_to_write_over_what_is_there() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    value(&mut session, sheet, "D3", json!("something"));
    let outcome = session
        .execute("grid.analysis.descriptive", &json!({ "sheet": sheet, "range": "A1:A5", "output": "D1" }));
    let error = outcome.expect_err("the report is refused").to_string();
    assert!(error.contains("holds something"), "the error names the cells: {error}");
    // With `overwrite` it is written.
    report(&mut session, sheet, json!({ "output": "D1", "overwrite": true }));
    assert_eq!(text_of(&mut session, sheet, "D3"), "Mean");
}

#[test]
fn the_names_of_the_variables_are_the_first_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("Heights"));
    value(&mut session, sheet, "B1", json!("Weights"));
    for row in 2..=5 {
        value(&mut session, sheet, &format!("A{row}"), json!(row));
        value(&mut session, sheet, &format!("B{row}"), json!(2.0 * f64::from(row)));
    }
    let data = report(&mut session, sheet, json!({ "range": "A1:B5", "labels": true }));
    let report_sheet = data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report_sheet, "A1"), "Heights");
    assert_eq!(text_of(&mut session, report_sheet, "C1"), "Weights");
    // The **numbers** are the rows below the names.
    assert_eq!(number(&mut session, report_sheet, "B3"), 3.5);
    assert_eq!(number(&mut session, report_sheet, "D3"), 7.0);
}

#[test]
fn the_variables_may_be_the_rows() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = report(&mut session, sheet, json!({ "range": "A1:A5", "grouped_by": "rows" }));
    let report_sheet = data["sheet"].as_u64().expect("the report has a sheet");
    // Five variables, one for each row of the range.
    assert_eq!(data["variables"], 5);
    assert_eq!(number(&mut session, report_sheet, "B3"), 1.0);
    assert_eq!(number(&mut session, report_sheet, "J3"), 5.0);
}

#[test]
fn text_in_the_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    value(&mut session, sheet, "A3", json!("three"));
    let outcome = session.execute("grid.analysis.descriptive", &json!({ "sheet": sheet, "range": "A1:A5" }));
    let error = outcome.expect_err("the report is refused").to_string();
    assert!(error.contains("text"), "the error names the text: {error}");
}

#[test]
fn the_report_may_hold_the_largest_the_smallest_and_the_confidence() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = report(
        &mut session,
        sheet,
        json!({ "summary": false, "kth_largest": 1, "kth_smallest": 2, "confidence": 95 }),
    );
    let report_sheet = data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report_sheet, "A3"), "Largest(1)");
    assert_eq!(number(&mut session, report_sheet, "B3"), 5.0);
    assert_eq!(text_of(&mut session, report_sheet, "A4"), "Smallest(2)");
    assert_eq!(number(&mut session, report_sheet, "B4"), 2.0);
    assert_eq!(text_of(&mut session, report_sheet, "A5"), "Confidence Level(95.0%)");
    // The margin of error of the mean, at 95%.
    let margin = number(&mut session, report_sheet, "B5");
    assert!((margin - 1.9632431614775585).abs() < 1e-9, "the margin is {margin}");
}

#[test]
fn the_report_needs_something_to_write() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let outcome = session
        .execute("grid.analysis.descriptive", &json!({ "sheet": sheet, "range": "A1:A5", "summary": false }));
    let error = outcome.expect_err("the report is refused").to_string();
    assert!(error.contains("summary"), "the error names the choice: {error}");
}

#[test]
fn the_report_goes_to_one_place_or_the_other() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let outcome = session.execute(
        "grid.analysis.descriptive",
        &json!({ "sheet": sheet, "range": "A1:A5", "output": "D1", "new_sheet": "Report" }),
    );
    let error = outcome.expect_err("the report is refused").to_string();
    assert!(error.contains("not to both"), "the error names the choice: {error}");
}

/// The **histogram**: the outcome of the command.
fn histogram(session: &mut Session, sheet: u64, extra: Json) -> Json {
    let mut params = json!({ "sheet": sheet, "range": "A1:A5" });
    if let (Some(target), Some(source)) = (params.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    session.execute("grid.analysis.histogram", &params).expect("the histogram is made").data.clone()
}

#[test]
fn the_histogram_counts_the_numbers_of_each_bin() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = histogram(&mut session, sheet, json!({}));
    let report = data["sheet"].as_u64().expect("the report has a sheet");
    // Five numbers, two intervals, three edges.
    assert_eq!(data["bins"], 3);
    assert_eq!(text_of(&mut session, report, "A1"), "Bin");
    assert_eq!(text_of(&mut session, report, "B1"), "Frequency");
    // The bins are evenly spaced from the smallest to the largest.
    assert_eq!(number(&mut session, report, "A2"), 1.0);
    assert_eq!(number(&mut session, report, "A3"), 3.0);
    assert_eq!(number(&mut session, report, "A4"), 5.0);
    // And the counts are the numbers of each interval.
    assert_eq!(number(&mut session, report, "B2"), 1.0);
    assert_eq!(number(&mut session, report, "B3"), 2.0);
    assert_eq!(number(&mut session, report, "B4"), 2.0);
    // The numbers past the last bin.
    assert_eq!(text_of(&mut session, report, "A5"), "More");
    assert_eq!(number(&mut session, report, "B5"), 0.0);
}

#[test]
fn the_histogram_is_live() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = histogram(&mut session, sheet, json!({}));
    let report = data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(
        formula_of(&mut session, report, "B3"),
        "=COUNTIFS(Sheet1!$A$1:$A$5,\">\"&$A$2,Sheet1!$A$1:$A$5,\"<=\"&$A$3)"
    );
    // An edit of the data is an edit of the histogram.
    value(&mut session, sheet, "A1", json!(4.0));
    assert_eq!(number(&mut session, report, "B3"), 1.0);
    assert_eq!(number(&mut session, report, "B4"), 3.0);
}

#[test]
fn the_histogram_may_hold_the_cumulative_percentage() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = histogram(&mut session, sheet, json!({ "cumulative": true }));
    let report = data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report, "C1"), "Cumulative %");
    assert_eq!(number(&mut session, report, "C2"), 0.2);
    assert_eq!(number(&mut session, report, "C5"), 1.0);
}

#[test]
fn the_histogram_may_hold_the_pareto() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    let data = histogram(&mut session, sheet, json!({ "pareto": true }));
    let report = data["sheet"].as_u64().expect("the report has a sheet");
    // The bins of the pareto, in the order of their frequencies.
    assert_eq!(text_of(&mut session, report, "C1"), "Bin");
    assert_eq!(text_of(&mut session, report, "D1"), "Frequency");
    assert_eq!(number(&mut session, report, "D2"), 2.0);
    assert_eq!(number(&mut session, report, "D3"), 2.0);
    assert_eq!(number(&mut session, report, "D4"), 1.0);
    // The bin of the largest frequency is the third one (3 to 5).
    assert_eq!(number(&mut session, report, "C2"), 3.0);
}

#[test]
fn the_bins_may_be_given() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_column(&mut session, sheet);
    value(&mut session, sheet, "C1", json!(10.0));
    value(&mut session, sheet, "C2", json!(0.0));
    value(&mut session, sheet, "C3", json!(5.0));
    let data = histogram(&mut session, sheet, json!({ "bins": "C1:C3" }));
    let report = data["sheet"].as_u64().expect("the report has a sheet");
    // The bins are the range, in order.
    assert_eq!(number(&mut session, report, "A2"), 0.0);
    assert_eq!(number(&mut session, report, "A3"), 5.0);
    assert_eq!(number(&mut session, report, "A4"), 10.0);
    // The count of each interval.
    assert_eq!(number(&mut session, report, "B2"), 0.0);
    assert_eq!(number(&mut session, report, "B3"), 5.0);
    assert_eq!(number(&mut session, report, "B4"), 0.0);
}

#[test]
fn the_histogram_wants_a_line_of_numbers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for row in 1..=3 {
        for col in ["A", "B"] {
            value(&mut session, sheet, &format!("{col}{row}"), json!(row));
        }
    }
    let outcome = session.execute("grid.analysis.histogram", &json!({ "sheet": sheet, "range": "A1:B3" }));
    let error = outcome.expect_err("the histogram is refused").to_string();
    assert!(error.contains("one row or one column"), "the error names the shape: {error}");
}

/// Two samples: `1, 2, 3, 4, 5` and `2, 4, 6, 8, 10`.
fn two_samples(session: &mut Session, sheet: u64) {
    for row in 1..=5 {
        value(session, sheet, &format!("A{row}"), json!(row));
        value(session, sheet, &format!("B{row}"), json!(2.0 * f64::from(row)));
    }
}

/// The **sample variance** of a list of numbers.
fn var_s(numbers: &[f64]) -> f64 {
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    numbers.iter().map(|value| (value - mean) * (value - mean)).sum::<f64>() / (numbers.len() - 1) as f64
}

#[test]
fn the_correlation_of_the_variables_is_a_matrix() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    two_samples(&mut session, sheet);
    let outcome = session
        .execute("grid.analysis.correlation", &json!({ "sheet": sheet, "range": "A1:B5" }))
        .expect("the correlation is made");
    let report = outcome.data["sheet"].as_u64().expect("the report has a sheet");
    // The names of the variables are in the first row and the first
    // column.
    assert_eq!(text_of(&mut session, report, "B1"), "Column1");
    assert_eq!(text_of(&mut session, report, "C1"), "Column2");
    assert_eq!(text_of(&mut session, report, "A2"), "Column1");
    assert_eq!(text_of(&mut session, report, "A3"), "Column2");
    // The diagonal is one, and the correlation of the two variables is
    // under it.
    assert_eq!(number(&mut session, report, "B2"), 1.0);
    assert_eq!(number(&mut session, report, "C3"), 1.0);
    assert_eq!(number(&mut session, report, "B3"), 1.0);
    assert_eq!(formula_of(&mut session, report, "B3"), "=CORREL(Sheet1!$B$1:$B$5,Sheet1!$A$1:$A$5)");
}

#[test]
fn the_covariance_of_the_variables_is_a_matrix() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    two_samples(&mut session, sheet);
    let outcome = session
        .execute("grid.analysis.covariance", &json!({ "sheet": sheet, "range": "A1:B5" }))
        .expect("the covariance is made");
    let report = outcome.data["sheet"].as_u64().expect("the report has a sheet");
    // The covariance of a variable with itself is its variance.
    assert_eq!(number(&mut session, report, "B2"), 2.0);
    // And the covariance of the two is twice that: the second is twice
    // the first.
    assert_eq!(number(&mut session, report, "B3"), 4.0);
}

#[test]
fn the_t_test_of_two_samples_holds_the_statistics() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    two_samples(&mut session, sheet);
    let outcome = session
        .execute(
            "grid.analysis.ttest",
            &json!({ "sheet": sheet, "kind": "unequal", "range1": "A1:A5", "range2": "B1:B5" }),
        )
        .expect("the test is made");
    let report = outcome.data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report, "A1"), "t-Test: Two-Sample Assuming Unequal Variances");
    // The **means**, the **variances** and the **observations**.
    assert_eq!(number(&mut session, report, "B4"), 3.0);
    assert_eq!(number(&mut session, report, "C4"), 6.0);
    assert_eq!(number(&mut session, report, "B5"), 2.5);
    assert_eq!(number(&mut session, report, "B6"), 5.0);
    // The **t statistic**: the difference of the means over the
    // standard error of it.
    let one = [1.0, 2.0, 3.0, 4.0, 5.0];
    let two = [2.0, 4.0, 6.0, 8.0, 10.0];
    let expected = (3.0 - 6.0) / (var_s(&one) / 5.0 + var_s(&two) / 5.0).sqrt();
    assert!((number(&mut session, report, "B9") - expected).abs() < 1e-9, "the t statistic");
    // And the **p-value** of it.
    let p = number(&mut session, report, "B10");
    assert!(p > 0.0 && p < 1.0, "the p-value is {p}");
}

#[test]
fn the_paired_t_test_holds_the_correlation() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    two_samples(&mut session, sheet);
    let outcome = session
        .execute(
            "grid.analysis.ttest",
            &json!({ "sheet": sheet, "kind": "paired", "range1": "A1:A5", "range2": "B1:B5" }),
        )
        .expect("the test is made");
    let report = outcome.data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report, "A1"), "t-Test: Paired Two Sample for Means");
    assert_eq!(number(&mut session, report, "B7"), 1.0);
    // The differences are `-1, -2, -3, -4, -5`.
    let expected = -3.0 / (var_s(&[1.0, 2.0, 3.0, 4.0, 5.0]) / 5.0).sqrt();
    assert!((number(&mut session, report, "B10") - expected).abs() < 1e-9, "the t statistic");
}

#[test]
fn the_f_test_holds_the_ratio_of_the_variances() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    two_samples(&mut session, sheet);
    let outcome = session
        .execute("grid.analysis.ftest", &json!({ "sheet": sheet, "range1": "A1:A5", "range2": "B1:B5" }))
        .expect("the test is made");
    let report = outcome.data["sheet"].as_u64().expect("the report has a sheet");
    assert_eq!(text_of(&mut session, report, "A1"), "F-Test Two-Sample for Variances");
    assert_eq!(number(&mut session, report, "B7"), 4.0);
    assert_eq!(number(&mut session, report, "B8"), 0.25);
    // The ratio is less than one, so the tail is the left one.
    let p = number(&mut session, report, "B9");
    assert!(p > 0.0 && p < 1.0, "the p-value is {p}");
}

#[test]
fn a_test_wants_a_line_of_numbers() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for row in 1..=3 {
        for col in ["A", "B"] {
            value(&mut session, sheet, &format!("{col}{row}"), json!(row));
        }
    }
    let outcome = session.execute(
        "grid.analysis.ttest",
        &json!({ "sheet": sheet, "kind": "equal", "range1": "A1:B3", "range2": "B1:B3" }),
    );
    let error = outcome.expect_err("the test is refused").to_string();
    assert!(error.contains("one row or one column"), "the error names the shape: {error}");
}
