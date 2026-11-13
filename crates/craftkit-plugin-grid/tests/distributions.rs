//! The **shape** of a distribution.
//!
//! A formula may ask how a set of numbers is spread out.

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

/// The **normal** distribution.
#[test]
fn the_normal_distribution_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(NORM.DIST(42,40,1.5,TRUE),7)"),
        json!(0.9087888),
        "the chance"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(NORM.DIST(42,40,1.5,FALSE),8)"),
        json!(0.10934005),
        "and the density"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(NORM.INV(0.908789,40,1.5),6)"),
        json!(42.000002),
        "and the number that makes it"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(NORM.S.DIST(1.333333,TRUE),9)"),
        json!(0.908788726),
        "and the standard one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(NORMSINV(0.975),8)"),
        json!(1.95996398),
        "and its inverse"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=ROUND(PHI(0.75),9)"),
        json!(0.301137432),
        "and the density of the standard one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A7", "=ROUND(GAUSS(2),9)"),
        json!(0.477249868),
        "and the chance between the mean and a number"
    );
}

/// The **gamma** family.
#[test]
fn the_gamma_family_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=ROUND(GAMMA(2.5),9)"), json!(1.329340388), "the gamma");
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(GAMMALN(4),9)"),
        json!(1.791759469),
        "and its logarithm"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(GAMMA.DIST(10.00001131,9,2,TRUE),6)"),
        json!(0.068094),
        "and the distribution"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(GAMMA.INV(0.068094,9,2),6)"),
        json!(10.000011),
        "and its inverse"
    );
}

/// The **error function**.
#[test]
fn the_error_function_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(ERF(1),9)"),
        json!(0.842700793),
        "the error function"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(ERF.PRECISE(0.3),9)"),
        json!(0.328626759),
        "and the precise one"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(ERFC(3),12)"),
        json!(0.000022090497),
        "and its complement"
    );
}

/// **Student's t**.
#[test]
fn students_t_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(T.DIST(60,1,TRUE),8)"),
        json!(0.99469533),
        "the chance"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(T.DIST(8,3,FALSE),8)"),
        json!(0.00073691),
        "and the density"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(T.DIST.2T(1.959999998,60),9)"),
        json!(0.05464493),
        "and the two tails"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(T.DIST.RT(1.959999998,60),9)"),
        json!(0.027322465),
        "and the right one"
    );
}

/// The **chi-square** and the **F**.
#[test]
fn the_chi_square_and_the_f_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(CHISQ.DIST(0.5,1,TRUE),8)"),
        json!(0.52049988),
        "the chance"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(CHISQ.DIST.RT(18.307,10),6)"),
        json!(0.050001),
        "and the right tail"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(CHISQ.INV(0.93,1),8)"),
        json!(3.28302029),
        "and its inverse"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(CHISQ.INV.RT(0.050001,10),6)"),
        json!(18.306973),
        "and the inverse of the tail"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(F.DIST(15.2069,6,4,TRUE),6)"),
        json!(0.99),
        "and the F"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=ROUND(F.DIST.RT(15.2069,6,4),6)"),
        json!(0.01),
        "and its right tail"
    );
}

/// The **counts**: the binomial, the Poisson, the exponential, the
/// Weibull, the lognormal and the beta.
#[test]
fn the_counts_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(BINOM.DIST(6,10,0.5,FALSE),9)"),
        json!(0.205078125),
        "the chance of six"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(BINOM.DIST(6,10,0.5,TRUE),9)"),
        json!(0.828125),
        "and the chance of six or more"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=BINOM.INV(6,0.5,0.75)"), json!(4.0), "and its inverse");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(POISSON.DIST(2,5,TRUE),6)"),
        json!(0.124652),
        "the chance of two things"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(EXPON.DIST(0.2,10,TRUE),8)"),
        json!(0.86466472),
        "and the exponential"
    );
    assert_eq!(
        computed(&mut session, sheet, "A6", "=ROUND(WEIBULL.DIST(105,20,100,TRUE),6)"),
        json!(0.929581),
        "and the Weibull"
    );
    assert_eq!(
        computed(&mut session, sheet, "A7", "=ROUND(LOGNORM.DIST(4,3.5,1.2,TRUE),7)"),
        json!(0.0390836),
        "and the lognormal"
    );
    assert_eq!(
        computed(&mut session, sheet, "A8", "=ROUND(BETA.DIST(2,8,10,TRUE,1,3),7)"),
        json!(0.6854706),
        "and the beta"
    );
}

/// The tree's **errors**: a deviation of nothing, a probability of
/// one, degrees of freedom below one and a gamma of nothing.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=NORM.DIST(42,40,0,TRUE)"),
        json!({ "error": "#NUM!" }),
        "a deviation of nothing is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=NORM.INV(1,0,1)"),
        json!({ "error": "#NUM!" }),
        "and a probability of one is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=T.DIST(1,0,TRUE)"),
        json!({ "error": "#NUM!" }),
        "and degrees of freedom below one is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=GAMMA(0)"),
        json!({ "error": "#NUM!" }),
        "and a gamma of nothing is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=BINOM.DIST(11,10,0.5,FALSE)"),
        json!({ "error": "#NUM!" }),
        "and more successes than trials is an error"
    );
}

/// The tree's **other names** for the functions that are here.
#[test]
fn the_trees_other_names_are_here() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(CHIINV(0.050001,10),5)"),
        json!(18.30697),
        "the old name of the upper tail of the chi-square"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(FDIST(15.2069,6,4),6)"),
        json!(0.01),
        "the old name of the right tail of the F"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=CRITBINOM(6,0.5,0.75)"),
        json!(4.0),
        "the old name of the inverse of the binomial"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=ROUND(BETAINV(0.685470581,8,10,1,3),8)"),
        json!(2.0),
        "the old name of the inverse of the beta"
    );
    assert_eq!(
        computed(&mut session, sheet, "A5", "=ROUND(F.INV.RT(0.01,6,4),5)"),
        json!(15.20686),
        "and the other name of the inverse of the right tail of the F"
    );
}

/// The **negative binomial**: how many failures come before a number
/// of successes.
#[test]
fn the_negative_binomial_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(NEGBINOM.DIST(10,5,0.25,TRUE),7)"),
        json!(0.3135141),
        "the cumulative"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(NEGBINOM.DIST(10,5,0.25,FALSE),7)"),
        json!(0.0550487),
        "and the one term"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=ROUND(NEGBINOMDIST(10,5,0.25),7)"),
        json!(0.0550487),
        "and the old name of the one term"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=NEGBINOM.DIST(10,5,2)"),
        json!({ "error": "#NUM!" }),
        "and a probability that is not one is an error"
    );
}

/// The **range** of the binomial: the probability of a number of
/// successes between two numbers.
#[test]
fn the_range_of_the_binomial_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=ROUND(BINOM.DIST.RANGE(6,0.5,2,3),8)"),
        json!(0.546875),
        "the two middle numbers"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=ROUND(BINOM.DIST.RANGE(6,0.5,2),8)"),
        json!(0.234375),
        "and one number of the range is the same number"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=BINOM.DIST.RANGE(6,0.5,2,7)"),
        json!({ "error": "#NUM!" }),
        "and a range that is not inside the trials is an error"
    );
}
