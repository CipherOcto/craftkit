//! The **limits of the grid battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The text that comes in is from
//! outside — a file, a paste, an agent — so it is checked before it is
//! used, and a limit that no test names is a limit nothing proves holds.

use craftkit_plugin_grid::{
    MAX_COLS, MAX_COLUMNS, MAX_CONSTRAINTS, MAX_DENOMINATOR_DIGITS, MAX_ERRORS, MAX_FIT_PAGES,
    MAX_FIXED_DENOMINATOR, MAX_FORMAT_CHARS, MAX_FORMULA_ROWS, MAX_IMPORT_ROWS, MAX_MARGIN, MAX_NODES,
    MAX_NODES_LP, MAX_ORDER, MAX_PERIOD, MAX_PIVOTS, MAX_SCALE, MAX_SERIAL, MAX_TEXT, MAX_VARS,
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

/// A **workbook**, and its id.
fn a_workbook(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    outcome.data["id"].as_u64().expect("the workbook has an id")
}

/// The **address** of the cell in that column, counted from zero.
///
/// A column is named in letters, the way the format names them: `A` is
/// the first and `XFD` is the last of a sheet.
fn a_column(column: usize) -> String {
    let mut name = String::new();
    let mut left = column + 1;
    while left > 0 {
        let rest = left - 1;
        name.insert(0, (b'A' + (rest % 26) as u8) as char);
        left = rest / 26;
    }
    name
}

/// A text with **more rows** than `MAX_IMPORT_ROWS` allows is refused.
///
/// The text of an import comes from outside, and a text of ten thousand
/// and one rows is refused rather than half-read.
#[test]
fn a_text_with_too_many_rows_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let text = "a\n".repeat(MAX_IMPORT_ROWS + 1);
    let error = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": text }))
        .expect_err("that text has too many rows");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A text with **more columns** than `MAX_COLUMNS` allows is refused.
#[test]
fn a_text_with_too_many_columns_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let text = format!("a{}", ",a".repeat(MAX_COLUMNS));
    let error = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": text }))
        .expect_err("that text has too many columns");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **sheet** in a workbook, and its id.
fn a_sheet(session: &mut Session) -> u64 {
    let workbook = a_workbook(session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **chart** on a sheet, and its id.
fn a_chart(session: &mut Session, sheet: u64) -> u64 {
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:C10" }))
        .expect("the chart is added");
    outcome.data["chart"].as_u64().expect("the chart has an id")
}

/// A cell in a column **beyond** `MAX_COLS` is not an address.
///
/// A sheet is sixteen thousand columns wide, and the address past the
/// last one is not a cell the battery has.
#[test]
fn a_cell_beyond_the_columns_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.cell.set",
            &json!({ "sheet": sheet, "cell": format!("{}1", a_column(MAX_COLS as usize)), "value": 1 }),
        )
        .expect_err("that column is past the sheet");
    assert!(error.to_string().contains("is not a cell"), "{error}");
}

/// A cell in a row **beyond** `MAX_FORMULA_ROWS` is not an address.
#[test]
fn a_cell_beyond_the_rows_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.cell.set",
            &json!({ "sheet": sheet, "cell": format!("A{}", MAX_FORMULA_ROWS + 1), "value": 1 }),
        )
        .expect_err("that row is past the sheet");
    assert!(error.to_string().contains("is not a cell"), "{error}");
}

/// A **format** over `MAX_FORMAT_CHARS` long is refused.
#[test]
fn a_format_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let format = "0".repeat(MAX_FORMAT_CHARS + 1);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": format }))
        .expect_err("that format is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A fraction with a denominator of **more digits** than
/// `MAX_DENOMINATOR_DIGITS` is refused.
///
/// The tree refuses the shape, and so it refuses the format: a code
/// that is not a format is not written down.
#[test]
fn a_fraction_with_too_many_denominator_digits_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": format!("# ??/{}", "?".repeat(MAX_DENOMINATOR_DIGITS + 1)) }))
        .expect_err("that denominator has too many digits");
    assert!(error.to_string().contains("is not a format"), "{error}");
}

/// A fraction with a **fixed denominator** over `MAX_FIXED_DENOMINATOR`
/// is refused.
#[test]
fn a_fraction_with_too_large_a_denominator_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.cell.format",
            &json!({ "sheet": sheet, "cell": "A1", "format": format!("# ?/{}", MAX_FIXED_DENOMINATOR + 1) }),
        )
        .expect_err("that denominator is too large");
    assert!(error.to_string().contains("is not a format"), "{error}");
}

/// A fraction with a denominator **within** the limits is a fraction.
///
/// This is the other half of the two tests above: a cap that refuses
/// everything is not a cap, it is a wall.
#[test]
fn a_fraction_within_the_limits_is_a_fraction() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0.5 }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": format!("# ??/{}", "?".repeat(MAX_DENOMINATOR_DIGITS)) }))
        .expect("the format is kept");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    let text = outcome.data["text"].as_str().expect("the cell has a text");
    assert!(text.contains('/'), "the code is a fraction: {text}");
}

/// A **header** over `MAX_TEXT` long is refused.
#[test]
fn a_header_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let text = "h".repeat(MAX_TEXT + 1);
    let error = session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": text }))
        .expect_err("that header is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **scale** over `MAX_SCALE` is refused.
#[test]
fn a_scale_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.print.scale", &json!({ "sheet": sheet, "percent": MAX_SCALE + 1.0 }))
        .expect_err("that scale is too big");
    assert!(error.to_string().contains("percent"), "{error}");
}

/// A **fit** over `MAX_FIT_PAGES` pages is refused.
#[test]
fn a_fit_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.print.fit",
            &json!({ "sheet": sheet, "wide": MAX_FIT_PAGES as f64 + 1.0, "tall": 1.0 }),
        )
        .expect_err("that fit is too many pages");
    assert!(error.to_string().contains("pages"), "{error}");
}

/// A **margin** over `MAX_MARGIN` is refused.
#[test]
fn a_margin_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.print.margins", &json!({ "sheet": sheet, "left": MAX_MARGIN + 1.0 }))
        .expect_err("that margin is too big");
    assert!(error.to_string().contains("margin"), "{error}");
}

/// A **trendline** of an order over `MAX_ORDER` is refused.
#[test]
fn a_trendline_order_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet);
    let error = session
        .execute(
            "grid.chart.trendline",
            &json!({ "chart": chart, "kind": "polynomial", "order": MAX_ORDER as f64 + 1.0 }),
        )
        .expect_err("that order is too deep");
    assert!(error.to_string().contains("order"), "{error}");
}

/// A **moving average** of a period over `MAX_PERIOD` is refused.
#[test]
fn a_trendline_period_over_the_limit_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let chart = a_chart(&mut session, sheet);
    let error = session
        .execute(
            "grid.chart.trendline",
            &json!({ "chart": chart, "kind": "moving_average", "period": MAX_PERIOD as f64 + 1.0 }),
        )
        .expect_err("that period is too long");
    assert!(error.to_string().contains("period"), "{error}");
}

/// A program with **more variables** than `MAX_VARS` is refused.
#[test]
fn a_program_with_too_many_variables_is_refused() {
    let mut session = session();
    let objective = json!(vec![1.0; MAX_VARS + 1]);
    let error = session
        .execute("grid.solver.solve", &json!({ "objective": objective }))
        .expect_err("that program has too many variables");
    assert!(error.to_string().contains("variables"), "{error}");
}

/// A program with **more constraints** than `MAX_CONSTRAINTS` is refused.
#[test]
fn a_program_with_too_many_constraints_is_refused() {
    let mut session = session();
    let row = json!({ "coefficients": [1.0], "sense": "<=", "rhs": 1.0 });
    let constraints = json!(vec![row; MAX_CONSTRAINTS + 1]);
    let error = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "constraints": constraints }))
        .expect_err("that program has too many constraints");
    assert!(error.to_string().contains("constraints"), "{error}");
}

/// A solver whose **budget** of pivots is over `MAX_PIVOTS` is refused.
#[test]
fn a_solver_budget_over_the_limit_is_refused() {
    let mut session = session();
    let error = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "budget": MAX_PIVOTS as f64 + 1.0 }))
        .expect_err("that budget is too many pivots");
    assert!(error.to_string().contains("pivots"), "{error}");
}

/// A solver that may visit **more nodes** than `MAX_NODES` is refused.
///
/// The branch and bound is bounded by `MAX_NODES_LP`, which is
/// `MAX_NODES`: the two are one number under two names.
#[test]
fn a_solver_node_budget_over_the_limit_is_refused() {
    let mut session = session();
    assert_eq!(MAX_NODES, MAX_NODES_LP, "the two names are one number");
    let error = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "max_nodes": MAX_NODES as f64 + 1.0 }))
        .expect_err("that budget is too many nodes");
    assert!(error.to_string().contains("nodes"), "{error}");
}

/// A **serial** over `MAX_SERIAL` is not a day that happened.
///
/// The calendar ends, and a date past its end is an error and not a
/// date.
#[test]
fn a_serial_beyond_the_calendar_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute(
            "grid.formula.evaluate",
            &json!({ "sheet": sheet, "formula": format!("=YEAR({})", MAX_SERIAL + 1.0) }),
        )
        .expect("the formula is worked out");
    assert_eq!(outcome.data["type"], "error", "{outcome:?}");
    assert_eq!(outcome.data["value"]["error"], json!("#NUM!"), "{outcome:?}");
}

/// A sheet with **more errors** than `MAX_ERRORS` is refused.
///
/// The list of the errors of a sheet is a list, and a list of ten
/// thousand and one errors is refused rather than half-written.
#[test]
fn a_sheet_with_too_many_errors_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1/0" }))
        .expect("the formula is set");
    session
        .execute(
            "grid.range.fill",
            &json!({ "sheet": sheet, "from": "A1", "to": format!("A1:A{}", MAX_ERRORS + 1), "mode": "copy" }),
        )
        .expect("the formula is filled down");
    let error = session
        .execute("grid.errors", &json!({ "sheet": sheet }))
        .expect_err("that sheet has too many errors");
    assert!(error.to_string().contains("errors"), "{error}");
}
