//! The **binary** variables of a program: a variable that is zero or one.

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

/// **A binary program is solved in zeroes and ones.**
///
/// The knapsack of the tree: the biggest value that fits in the bag.
#[test]
fn a_binary_program_is_solved_in_zeroes_and_ones() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [10.0, 6.0, 4.0],
            "constraints": [
                { "coefficients": [4.0, 3.0, 2.0], "sense": "<=", "rhs": 11.0 },
            ],
            "binary": [true, true, true],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    assert_eq!(values.len(), 3, "there are three values: {values:?}");
    for value in values {
        let value = value.as_f64().expect("a value");
        assert!(value == 0.0 || value == 1.0, "a binary value is zero or one: {values:?}");
    }
    let objective = answer["objective"].as_f64().expect("the objective");
    assert!((objective - 20.0).abs() < 1.0e-6, "the best of it is twenty: {answer:?}");
}

/// **A binary variable with no value in its bounds has no solution.**
#[test]
fn a_binary_variable_with_no_value_in_its_bounds_has_no_solution() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "bounds": [[3.0, 7.0]],
            "binary": [true],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("infeasible"), "there is no answer: {answer:?}");
}

/// **A binary variable that cannot reach one is zero.**
///
/// The tree's rule: the bounds of a binary variable meet the pair zero
/// and one, and one that is left out of them is not a value.
#[test]
fn a_binary_variable_that_cannot_reach_one_is_zero() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0],
            "bounds": [[0.0, 0.5]],
            "binary": [true],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    let value = values[0].as_f64().expect("a value");
    assert_eq!(value, 0.0, "the only value left is zero: {values:?}");
}

/// **A binary program with no solution is infeasible.**
#[test]
fn a_binary_program_with_no_solution_is_infeasible() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [1.0, 1.0],
            "constraints": [
                { "coefficients": [1.0, 1.0], "sense": ">=", "rhs": 2.5 },
            ],
            "binary": [true, true],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("infeasible"), "there is no answer: {answer:?}");
}

/// **A binary variable is an integer variable.**
#[test]
fn a_binary_variable_that_is_not_an_integer_variable_is_refused() {
    let mut session = session();
    let refusal = session
        .execute(
            "grid.solver.solve",
            &json!({
                "objective": [1.0],
                "integer": [false],
                "binary": [true],
            }),
        )
        .expect_err("the contradiction is refused");
    let message = refusal.to_string();
    assert!(message.contains("binary"), "the refusal says so: {message}");
}

/// **A flag that is not one for each variable is refused.**
#[test]
fn a_flag_that_is_not_one_for_each_variable_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0, 1.0], "binary": [true] }))
        .expect_err("the flags are refused");
    let message = refusal.to_string();
    assert!(message.contains("flag"), "the refusal says so: {message}");
}

/// **A flag that is not true or false is refused.**
#[test]
fn a_flag_of_a_binary_variable_that_is_not_a_flag_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "binary": ["yes"] }))
        .expect_err("the flag is refused");
    let message = refusal.to_string();
    assert!(message.contains("true or false"), "the refusal says so: {message}");
}

/// **A binary list that is not a list is refused.**
#[test]
fn a_binary_list_that_is_not_a_list_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("grid.solver.solve", &json!({ "objective": [1.0], "binary": 1.0 }))
        .expect_err("the list is refused");
    let message = refusal.to_string();
    assert!(message.contains("list of flags"), "the refusal says so: {message}");
}

/// **A binary variable and a whole one live in one program.**
#[test]
fn a_binary_variable_and_a_whole_one_live_in_one_program() {
    let mut session = session();
    let answer = solve(
        &mut session,
        json!({
            "objective": [6.0, 2.0],
            "constraints": [
                { "coefficients": [2.0, 1.0], "sense": "<=", "rhs": 5.5 },
            ],
            "integer": [true, true],
            "binary": [true, false],
            "maximise": true,
        }),
    );
    assert_eq!(answer["status"], json!("optimal"), "the program is solved: {answer:?}");
    let values = answer["values"].as_array().expect("the values are a list");
    let first = values[0].as_f64().expect("a value");
    let second = values[1].as_f64().expect("a value");
    assert_eq!(first, 1.0, "the binary variable is one: {values:?}");
    assert_eq!(second, 3.0, "the whole one is three: {values:?}");
}
