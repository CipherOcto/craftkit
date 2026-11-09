//! The **solver** of the grid: a linear program is solved.

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

/// **Solve** a program and give the answer back.
fn solve(session: &mut Session, program: craftkit_sdk::serde_json::Value) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.solver.solve", &program).expect("the program is solved");
    outcome.data.clone()
}

/// **A program is minimised.**
#[test]
fn a_program_is_minimised() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0, 1.0],
            "constraints": [
                { "coefficients": [1.0, 1.0], "sense": ">=", "rhs": 10.0 },
            ],
            "bounds": [[0.0, null], [0.0, null]],
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    let first = values[0].as_f64().expect("a value");
    let second = values[1].as_f64().expect("a value");
    assert!((first + second - 10.0).abs() < 1.0e-6, "the sum is ten: {values:?}");
    let objective = answer["objective"].as_f64().expect("the objective");
    assert!((objective - 10.0).abs() < 1.0e-6, "and the objective is ten: {answer:?}");
}

/// **A program that is maximised is minimised with its costs turned
/// about.**
#[test]
fn a_program_that_is_maximised_is_turned_about() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [3.0, 4.0],
            "constraints": [
                { "coefficients": [1.0, 1.0], "sense": "<=", "rhs": 10.0 },
            ],
            "bounds": [[0.0, null], [0.0, null]],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let objective = answer["objective"].as_f64().expect("the objective");
    assert!((objective - 40.0).abs() < 1.0e-6, "the best of it is forty: {answer:?}");
}

/// **A program with no solution is infeasible.**
#[test]
fn a_program_with_no_solution_is_infeasible() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "constraints": [
                { "coefficients": [1.0], "sense": ">=", "rhs": 10.0 },
                { "coefficients": [1.0], "sense": "<=", "rhs": 5.0 },
            ],
        }),
    );
    assert_eq!(answer["status"], json!("infeasible"), "and it says so: {answer:?}");
    assert!(answer["objective"].is_null(), "and there is no objective: {answer:?}");
}

/// **A program that can grow without end is unbounded.**
#[test]
fn a_program_that_can_grow_without_end_is_unbounded() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [-1.0],
            "constraints": [
                { "coefficients": [1.0], "sense": ">=", "rhs": 0.0 },
            ],
        }),
    );
    assert_eq!(answer["status"], json!("unbounded"), "and it says so: {answer:?}");
}

/// **The bounds of a variable are the tree's own.**
#[test]
fn the_bounds_of_a_variable_are_the_trees_own() {
    let mut session = session();
    // The bound of the variable is below the bound it must reach: the
    // program has no solution.
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "constraints": [
                { "coefficients": [1.0], "sense": ">=", "rhs": 5.0 },
            ],
            "bounds": [[0.0, 3.0]],
        }),
    );
    assert_eq!(answer["status"], json!("infeasible"), "and it says so: {answer:?}");
    // The same program with a bound that is above the one it must
    // reach: it is solved.
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "constraints": [
                { "coefficients": [1.0], "sense": ">=", "rhs": 5.0 },
            ],
            "bounds": [[0.0, 8.0]],
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    let value = values[0].as_f64().expect("a value");
    assert!((value - 5.0).abs() < 1.0e-6, "and the value is five: {values:?}");
}

/// **A constraint with the wrong number of coefficients is refused.**
#[test]
fn a_constraint_with_the_wrong_number_of_coefficients_is_refused() {
    let mut session = session();
    let refusal = session
        .execute(
            "grid.solver.solve",
            &json!({
                "objective": [1.0, 1.0],
                "constraints": [
                    { "coefficients": [1.0], "sense": ">=", "rhs": 10.0 },
                ],
            }),
        )
        .expect_err("the constraint is refused");
    let message = refusal.to_string();
    assert!(message.contains("coefficient"), "the refusal says so: {message}");
}

/// **A sense that is not one of the tree's own is refused.**
#[test]
fn a_sense_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let refusal = session
        .execute(
            "grid.solver.solve",
            &json!({
                "objective": [1.0],
                "constraints": [
                    { "coefficients": [1.0], "sense": "<>", "rhs": 10.0 },
                ],
            }),
        )
        .expect_err("the sense is refused");
    let message = refusal.to_string();
    assert!(message.contains("`<=`, `>=` or `=`"), "the refusal says what a sense is: {message}");
}

/// **A budget outside the tree's own is refused.**
#[test]
fn a_budget_outside_the_trees_own_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "budget": 0.0 }))
        .expect_err("the budget is refused");
    let message = refusal.to_string();
    assert!(message.contains("between 1 and"), "the refusal says the bound: {message}");
}
