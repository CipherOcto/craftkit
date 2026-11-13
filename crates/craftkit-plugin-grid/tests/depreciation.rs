//! The **wearing out** of an asset.
//!
//! A formula may ask what a thing that wears out is worth now, a
//! period at a time.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// The **straight line**: the same every period.
#[test]
fn the_straight_line_is_the_same_every_period() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=SLN(30000,7500,10)"),
        json!(2250.0),
        "the depreciation of a period"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(SLN(30000,7500,10)*10,2)"),
        json!(22500.0),
        "and ten of them are the whole of the wear"
    );
}

/// The **sum of the years**: less and less, because the years are
/// added up and each one takes its share.
#[test]
fn the_sum_of_the_years_takes_less_and_less() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(SYD(30000,7500,10,1),6)"),
        json!(4090.909091),
        "the first year"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(SYD(30000,7500,10,10),7)"),
        json!(409.0909091),
        "and the last"
    );
}

/// The **declining balance**, with the month it is bought in.
#[test]
fn the_declining_balance_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(DB(1000000,100000,6,1,7),4)"),
        json!(186083.3333),
        "the first period"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(DB(1000000,100000,6,2,7),4)"),
        json!(259639.4167),
        "and the second"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(DB(1000000,100000,6,3,7),4)"),
        json!(176814.4428),
        "and the third"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(DB(1000000,100000,6,7,7),5)"),
        json!(15845.09847),
        "and the last"
    );
}

/// The **double declining balance**.
#[test]
fn the_double_declining_balance_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(DDB(2400,300,3650,1),9)"),
        json!(1.315068493),
        "the first period of a long life"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(DDB(2400,300,120,1,2),10)"),
        json!(40.0),
        "and the first of a shorter one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(DDB(2400,300,10,1,2),10)"),
        json!(480.0),
        "and of a short one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(DDB(2400,300,10,2,1.5),10)"),
        json!(306.0),
        "and with a factor of its own"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(DDB(2400,300,10,10),7)"),
        json!(22.1225472),
        "and the last period"
    );
}

/// The declining balance over a **span** of periods, with the switch
/// to the straight line left in.
#[test]
fn the_declining_balance_over_a_span_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(VDB(2400,300,3650,0,1),9)"),
        json!(1.315068493),
        "one period"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(VDB(2400,300,10,0,1),10)"),
        json!(480.0),
        "and one period of a short life"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(VDB(2400,300,120,6,18),7)"),
        json!(396.3060533),
        "and a span of them"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(VDB(2400,300,120,6,18,1.5),7)"),
        json!(311.8089367),
        "and a span with a factor of its own"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(VDB(2400,300,10,0,10),10)"),
        json!(2100.0),
        "and over the whole life the wear is the whole of it"
    );
}

/// The tree's **errors**: a life of no periods, a period beyond the
/// life and a span that ends before it starts.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=SLN(30000,7500,0)"),
        json!({ "error": "#DIV/0!" }),
        "a life of no periods is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=DB(1000000,100000,6,8,7)"),
        json!({ "error": "#NUM!" }),
        "and a period beyond the life is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=DDB(2400,300,10,11)"),
        json!({ "error": "#NUM!" }),
        "and a period beyond the life of a balance is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=VDB(2400,300,10,5,4)"),
        json!({ "error": "#NUM!" }),
        "and a span that ends before it starts is an error"
    );
}

/// A thing bought on a day wears out from that day — the tree's
/// `AMORLINC` and `AMORDEGRC`.
#[test]
fn a_thing_bought_on_a_day_wears_out_from_that_day() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // The tree's own example: a cost of two thousand four hundred,
    // bought in August and first settled in December.
    assert_eq!(
        computed(&mut session, sheet, "D1", "=AMORLINC(2400,DATE(2008,8,19),DATE(2008,12,31),300,1,0.15,1)"),
        json!(360.0),
        "the straight line of the first period"
    );
    assert_eq!(
        computed(&mut session, sheet, "D2", "=AMORDEGRC(2400,DATE(2008,8,19),DATE(2008,12,31),300,1,0.15,1)"),
        json!(776.0),
        "and the declining balance of the first period"
    );
    assert_eq!(
        computed(&mut session, sheet, "D3", "=AMORLINC(2400,DATE(2008,12,31),DATE(2008,8,19),300,1,0.15)"),
        json!({ "error": "#NUM!" }),
        "and a first period that ends before the thing was bought is an error"
    );
}
