//! The **dash** of a path: the line that is broken into pieces.
//!
//! A line may be **dashed**: a pattern of lengths — a dash, a gap, a
//! dash, a gap — and an **offset** that says where in the pattern the
//! line starts. The tree's own rule is the rule of every drawing
//! format that has dashes: a pattern of an **odd** number of lengths
//! is run **twice**, so that the dash and the gap keep their places.
//!
//! So the **pattern** is the fact and the **run** is derived: the
//! pattern that is run is the pattern that is written, doubled when
//! it is odd, and the length of a round of it is the sum of the run.
//!
//! The rules:
//!
//! - a length that is **not a number**, or that is **negative**, is
//!   refused;
//! - a pattern that is **empty** takes the dash away — the line is
//!   whole again, and that is the tree's own bargain: an empty value
//!   takes it away;
//! - an **offset** that is not a number is refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

/// A **dash**: the pattern of the line and where it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct Dash {
    /// The lengths of the pattern: a dash, a gap, a dash, a gap.
    pub pattern: Vec<f64>,
    /// Where in the pattern the line starts.
    pub offset: f64,
}

impl Dash {
    /// The **pattern that is run**: the pattern as it is written,
    /// doubled when it has an odd number of lengths.
    pub fn run(&self) -> Vec<f64> {
        let mut run = self.pattern.clone();
        if run.len() % 2 == 1 {
            run.extend(self.pattern.iter().copied());
        }
        run
    }

    /// The **length of a round** of the pattern: the sum of the run.
    pub fn round(&self) -> f64 {
        self.run().iter().sum()
    }

    /// The dash, as it is kept.
    pub fn to_json(&self) -> Json {
        json!({ "pattern": self.pattern.clone(), "offset": self.offset })
    }

    /// The dash, as it is read.
    pub fn from_json(value: &Json) -> Result<Self, String> {
        let pattern = value
            .get("pattern")
            .and_then(|pattern| pattern.as_array())
            .ok_or_else(|| "a dash has a pattern".to_string())?
            .iter()
            .map(|length| {
                length
                    .as_f64()
                    .filter(|length| length.is_finite() && *length >= 0.0)
                    .ok_or_else(|| "a dash is a list of lengths that are not negative".to_string())
            })
            .collect::<Result<Vec<f64>, String>>()?;
        let offset = value.get("offset").and_then(|offset| offset.as_f64()).unwrap_or(0.0);
        if !offset.is_finite() {
            return Err("the offset of a dash is a number".to_string());
        }
        Ok(Dash { pattern, offset })
    }
}

commands! {
    /// **Dash** the line of a path.
    ///
    /// A pattern that is **empty** takes the dash away.
    vector.path.dash {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Paint"),
        params: {
            /// The path.
            path: NodeRef,
            /// The pattern: a dash, a gap, a dash, a gap — the lengths in the
            /// world's units. Without it the line is whole.
            pattern: Option<Json>,
            /// Where in the pattern the line starts.
            offset: Option<Number>,
        },
        run: {
            let id = "vector.path.dash".to_string();
            let mut lengths = Vec::new();
            if let Some(pattern) = &self.pattern {
                let Some(list) = pattern.as_array() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the pattern of a dash is a list of lengths".to_string(),
                    });
                };
                for length in list {
                    let Some(value) = length.as_f64() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the pattern of a dash is a list of numbers".to_string(),
                        });
                    };
                    if !value.is_finite() || value < 0.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "{value} is not a length of a dash: a length is a number and not a negative one"
                            ),
                        });
                    }
                    lengths.push(value);
                }
            }
            let offset = self.offset.map(|offset| offset.0).unwrap_or(0.0);
            if !offset.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{offset} is not an offset: an offset is a number"),
                });
            }
            let mut vector = crate::vector_of(session, self.path.0)?;
            vector.dash = if lengths.is_empty() { None } else { Some(Dash { pattern: lengths.clone(), offset }) };
            crate::write_back(session, self.path.0, &vector)?;
            let dash = vector.dash.as_ref().map(|dash| dash.to_json()).unwrap_or(Json::Null);
            Ok(Outcome::changed(match vector.dash {
                Some(_) => "the line of the path is dashed".to_string(),
                None => "the line of the path is whole".to_string(),
            })
            .with_data(json!({ "id": self.path.0, "dash": dash })))

        }
    }
}
