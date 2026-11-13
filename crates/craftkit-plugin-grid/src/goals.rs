//! The **goal** of a sheet: the input that gives a result.
//!
//! A goal seek asks the other question: not "what does the sheet say
//! for this input?" but "**what input gives this result?**"
//!
//! It is found by **narrowing**: the sheet is asked at the two ends of
//! the range, and if the target is between what they say, the range is
//! halved and the sheet is asked again in the middle — and again, and
//! again, until the range is small enough to call the answer.
//!
//! # What the port found
//!
//! - **The tree's goal seek has its own limits and its own way of
//!   finding the answer**: it may scan the range in steps, or bisect
//!   it, and it stops when the answer is close enough. Here the range
//!   is **given** and the answer is found by **halving**: the work is
//!   one evaluation of the sheet for each halving, and the number of
//!   halvings is bounded.
//! - **The tree's goal seek says what it did.** Here it says the
//!   **steps** it took and the **width** of the range it was left
//!   with, so that a reader knows how close the answer is.
//!
//! And the assumption is named, because it is an assumption: the sheet
//! is taken to move **one way** as the input moves. A sheet that does
//! not is not found, and the command says so rather than giving an
//! answer that is a coincidence.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::formula::Value;

/// How many times a goal seek may halve the range.
const MAX_STEPS: usize = 64;

commands! {
    /// The **goal**: the input that gives a result.
    grid.goal {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The cell whose value is **varied**.
            input: Text,
            /// The cell whose value is **read**.
            result: Text,
            /// The result to reach.
            target: Number,
            /// Where the search **starts**.
            from: Number,
            /// Where the search **ends**.
            to: Number,
        },
        run: {
            let id = "grid.goal".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let input = crate::whatifs::cell_of(&self.input.0, &id, "input")?;
            let result = crate::whatifs::cell_of(&self.result.0, &id, "result")?;
            if input == result {
                return Err(EngineError::Failed {
                    id,
                    message: "the input and the result are the same cell".to_string(),
                });
            }
            let target = self.target.0;
            let from = self.from.0;
            let to = self.to.0;
            for number in [target, from, to] {
                if !number.is_finite() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the target and the ends of the range are numbers".to_string(),
                    });
                }
            }
            if from == to {
                return Err(EngineError::Failed {
                    id,
                    message: "the range of a goal seek has two ends".to_string(),
                });
            }
            // The **copy** of the workbook: the search is made in it and
            // the sheet itself is not touched.
            let mut book = crate::book_of(session.document());
            let name = node.name.clone();
            let low = from.min(to);
            let high = from.max(to);
            let at_low = crate::whatifs::trial(&mut book, &name, input, result, low, &id)?;
            let at_high = crate::whatifs::trial(&mut book, &name, input, result, high, &id)?;
            let low_value = number_of(&at_low, &id, low)?;
            let high_value = number_of(&at_high, &id, high)?;
            // The target must be **between** the two ends: a sheet that
            // does not reach it there does not reach it at all.
            let reaches = |value: f64| (value - target).abs() <= f64::EPSILON * target.abs().max(1.0);
            if reaches(low_value) {
                return Ok(Outcome::new(format!("the goal is `{low}`"))
                    .with_data(json!({ "sheet": sheet, "value": low, "result": low_value, "steps": 0 })));
            }
            if reaches(high_value) {
                return Ok(Outcome::new(format!("the goal is `{high}`"))
                    .with_data(json!({ "sheet": sheet, "value": high, "result": high_value, "steps": 0 })));
            }
            let between = (low_value - target).abs() + (high_value - target).abs();
            if between == 0.0 || (low_value - target).signum() == (high_value - target).signum() {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "the sheet does not reach {target} between {low} and {to}: it says {low_value} and {high_value}"
                    ),
                });
            }
            // The **halving**: the range is cut in two and the half the
            // target is in is the one that is kept.
            let mut low = low;
            let mut high = high;
            let mut low_value = low_value;
            let mut steps = 0usize;
            while steps < MAX_STEPS {
                let middle = (low + high) / 2.0;
                let at_middle = crate::whatifs::trial(&mut book, &name, input, result, middle, &id)?;
                let middle_value = number_of(&at_middle, &id, middle)?;
                steps += 1;
                if (middle_value - target).abs() <= f64::EPSILON * target.abs().max(1.0) * 1e6 {
                    return Ok(Outcome::new(format!("the goal is `{middle}`")).with_data(json!({
                        "sheet": sheet,
                        "value": middle,
                        "result": middle_value,
                        "steps": steps,
                    })));
                }
                let low_side = (middle_value - target).signum() == (low_value - target).signum();
                if low_side {
                    low = middle;
                    low_value = middle_value;
                } else {
                    high = middle;
                }
                if (high - low).abs() <= f64::EPSILON * high.abs().max(1.0) * 1e6 {
                    break;
                }
            }
            let value = (low + high) / 2.0;
            let at_value = crate::whatifs::trial(&mut book, &name, input, result, value, &id)?;
            let found = number_of(&at_value, &id, value)?;
            Ok(Outcome::new(format!("the goal is `{value}`")).with_data(json!({
                "sheet": sheet,
                "value": value,
                "result": found,
                "steps": steps,
                "width": (high - low).abs(),
            })))

        }
    }

    /// The **crossings** of a sheet: every place the result goes past the
    /// target.
    ///
    /// The goal seek finds **one** answer and it assumes the sheet moves
    /// one way. The scan asks **every** place: the range is walked in
    /// steps and the places where the result crosses the target are
    /// reported, and a sheet that moves one way and then the other is
    /// found.
    grid.goal.scan {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The cell whose value is **varied**.
            input: Text,
            /// The cell whose value is **read**.
            result: Text,
            /// The result to reach.
            target: Number,
            /// Where the scan **starts**.
            from: Number,
            /// Where the scan **ends**.
            to: Number,
            /// How far the scan steps. The default is one.
            step: Option<Number>,
        },
        run: {
            let id = "grid.goal.scan".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let input = crate::whatifs::cell_of(&self.input.0, &id, "input")?;
            let result = crate::whatifs::cell_of(&self.result.0, &id, "result")?;
            if input == result {
                return Err(EngineError::Failed {
                    id,
                    message: "the input and the result are the same cell".to_string(),
                });
            }
            let target = self.target.0;
            let from = self.from.0;
            let to = self.to.0;
            for number in [target, from, to] {
                if !number.is_finite() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the target and the ends of the range are numbers".to_string(),
                    });
                }
            }
            if from == to {
                return Err(EngineError::Failed { id, message: "the range of a scan has two ends".to_string() });
            }
            let step = self.step.map(|step| step.0).unwrap_or(1.0);
            if !step.is_finite() || step <= 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{step} is not a step: a step is a number above zero"),
                });
            }
            let low = from.min(to);
            let high = from.max(to);
            let trials = ((high - low) / step).floor() as usize + 1;
            if trials > MAX_TRIALS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the scan would ask the sheet {trials} times, and a scan asks it at most {MAX_TRIALS} times"
                    ),
                });
            }
            // The **copy** of the workbook: the scan is made in it and the
            // sheet itself is not touched.
            let mut book = crate::book_of(session.document());
            let name = node.name.clone();
            let reaches = |value: f64| (value - target).abs() <= f64::EPSILON * target.abs().max(1.0) * 1e6;
            let mut crossings: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
            let mut hits: Vec<f64> = Vec::new();
            let mut previous: Option<(f64, f64)> = None;
            for step_index in 0..trials {
                let value = (low + step_index as f64 * step).min(high);
                let at_value = crate::whatifs::trial(&mut book, &name, input, result, value, &id)?;
                let result_value = number_of(&at_value, &id, value)?;
                if reaches(result_value) {
                    hits.push(value);
                }
                if let Some((before_value, before_result)) = previous {
                    let before = (before_result - target).signum();
                    let now = (result_value - target).signum();
                    if before != 0.0 && now != 0.0 && before != now {
                        crossings.push(json!({
                            "from": before_value,
                            "to": value,
                            "result_from": before_result,
                            "result_to": result_value,
                        }));
                    }
                }
                previous = Some((value, result_value));
            }
            Ok(Outcome::new(format!("the sheet crosses {target} in {} place(s)", crossings.len())).with_data(
                json!({
                    "sheet": sheet,
                    "target": target,
                    "step": step,
                    "trials": trials,
                    "hits": hits,
                    "crossings": crossings,
                }),
            ))

        }
    }
}

/// The **number** a value is, if it is one.
fn number_of(value: &Value, id: &str, at: f64) -> Result<f64, EngineError> {
    match value {
        Value::Number(number) => Ok(*number),
        Value::Error(message) => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the sheet says `{message}` at {at}"),
        }),
        other => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the sheet says `{:?}` at {at}, and a goal is a number", other),
        }),
    }
}

/// How many times a **scan** may ask the sheet.
const MAX_TRIALS: usize = 4096;
