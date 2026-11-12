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

/// **A program with an integer variable is solved in whole numbers.**
#[test]
fn a_program_with_an_integer_variable_is_solved_in_whole_numbers() {
    let mut session = session();
    // Two and a half of one thing and half of another: the whole
    // numbers that fit are three and one.
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0, 1.0],
            "constraints": [
                { "coefficients": [1.0, 1.0], "sense": ">=", "rhs": 4.0 },
                { "coefficients": [2.0, 1.0], "sense": "<=", "rhs": 7.0 },
            ],
            "bounds": [[0.0, null], [0.0, null]],
            "integer": [true, true],
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    for value in values {
        let value = value.as_f64().expect("a value");
        assert!((value - value.round()).abs() < 1e-9, "the values are whole: {values:?}");
    }
    let objective = answer["objective"].as_f64().expect("the objective");
    assert!((objective - 4.0).abs() < 1e-6, "and the objective is four: {answer:?}");
    let nodes = answer["nodes"].as_u64().expect("the nodes are said");
    assert!(nodes > 0, "and the branch and bound visited a node: {answer:?}");
}

/// **A program whose relaxation is fractional is cut in two, and the
/// whole numbers are found.**
#[test]
fn a_program_whose_relaxation_is_fractional_is_cut_in_two() {
    let mut session = session();
    // The most of one thing that twice fits in three: the relaxed
    // answer is one and a half, and the whole number is one.
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "constraints": [
                { "coefficients": [2.0], "sense": "<=", "rhs": 3.0 },
            ],
            "bounds": [[0.0, null]],
            "integer": [true],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    let value = values[0].as_f64().expect("a value");
    assert!((value - 1.0).abs() < 1e-6, "and the value is one: {values:?}");
    let objective = answer["objective"].as_f64().expect("the objective");
    assert!((objective - 1.0).abs() < 1e-6, "and the objective is one: {answer:?}");
    let nodes = answer["nodes"].as_u64().expect("the nodes are said");
    assert!(nodes > 1, "and the branch and bound cut it: {answer:?}");
}

/// **A flag that is not one for each variable is refused.**
#[test]
fn a_flag_that_is_not_one_for_each_variable_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0, 1.0], "integer": [true] }))
        .expect_err("the flags are refused");
    let message = refusal.to_string();
    assert!(message.contains("flag"), "the refusal says so: {message}");
}

/// **The nodes of the branch and bound are bounded, and a number
/// outside is refused.**
#[test]
fn the_nodes_of_the_branch_and_bound_are_refused_when_they_are_outside() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "integer": [true], "max_nodes": 0.0 }))
        .expect_err("the nodes are refused");
    let message = refusal.to_string();
    assert!(message.contains("between 1 and"), "the refusal says the bound: {message}");
}
