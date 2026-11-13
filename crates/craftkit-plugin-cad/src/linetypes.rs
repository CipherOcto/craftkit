//! The **line type** of a drawing: the table of the patterns a line
//! may be drawn with.
//!
//! A line type is a **name**, a **description** and a **pattern**: the
//! lengths of the dashes and the gaps of the line. A dash is a
//! positive length, a gap is a negative one, and a dot is a length of
//! nothing — the tree's own way of writing a pattern.
//!
//! The table is the **tree's own**: the industry-common names users
//! expect, and the values the tree wrote for them. The tree's library
//! is written in inches and scaled for a metric drawing, and this tree
//! is in **millimetres**, so the lengths here are the tree's own
//! lengths multiplied by twenty-five and four tenths — `DASHED` is
//! 12.7 millimetres on and 6.35 off.
//!
//! So a layer may name **one of these** and not any text at all: a
//! name that is not in the table is a name nothing defines, and the
//! tree's own rule is that a layer's line type is a line type.

use craftkit_derive::commands;

use craftkit_sdk::Outcome;
use craftkit_sdk::serde_json::{Value as Json, json};

/// A **line type**: a name, a description and a pattern.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineType {
    /// The name of it.
    pub name: &'static str,
    /// What it says about itself.
    pub description: &'static str,
    /// The lengths of the pattern: a dash, a gap, a dash, a gap.
    pub pattern: &'static [f64],
}

/// The **line types** of this tree: the tree's own table.
///
/// The lengths are in **millimetres**: the tree's own inches
/// multiplied by twenty-five and four tenths.
const TYPES: [LineType; 18] = [
    LineType {
        name: "BORDER",
        description: "Border __ __ . __ __ . __ __ .",
        pattern: &[12.7, -6.35, 12.7, -6.35, 0.0, -6.35],
    },
    LineType {
        name: "CENTER",
        description: "Center ____ _ ____ _ ____",
        pattern: &[31.75, -6.35, 6.35, -6.35],
    },
    LineType {
        name: "CENTER2",
        description: "Center (.5x) ___ _ ___ _",
        pattern: &[19.05, -3.175, 3.175, -3.175],
    },
    LineType {
        name: "CENTERX2",
        description: "Center (2x) ________  __  ________",
        pattern: &[63.5, -12.7, 12.7, -12.7],
    },
    LineType { name: "DASHDOT", description: "Dash dot __ . __ . __ .", pattern: &[12.7, -6.35, 0.0, -6.35] },
    LineType {
        name: "DASHDOT2",
        description: "Dash dot (.5x) _._._._.",
        pattern: &[6.35, -3.175, 0.0, -3.175],
    },
    LineType { name: "DASHED", description: "Dashed __ __ __ __", pattern: &[12.7, -6.35] },
    LineType { name: "DASHED2", description: "Dashed (.5x) _ _ _ _ _", pattern: &[6.35, -3.175] },
    LineType { name: "DASHEDX2", description: "Dashed (2x) ____  ____  ____", pattern: &[25.4, -12.7] },
    LineType {
        name: "DIVIDE",
        description: "Divide ____ . . ____ . .",
        pattern: &[12.7, -6.35, 0.0, -6.35, 0.0, -6.35],
    },
    LineType { name: "DOT", description: "Dot . . . . . . . .", pattern: &[0.0, -6.35] },
    LineType { name: "DOT2", description: "Dot (.5x) ........", pattern: &[0.0, -3.175] },
    LineType { name: "HIDDEN", description: "Hidden __ __ __ __", pattern: &[6.35, -3.175] },
    LineType { name: "HIDDEN2", description: "Hidden (.5x) _ _ _ _ _", pattern: &[3.175, -1.5875] },
    LineType { name: "HIDDENX2", description: "Hidden (2x) ____ ____ ____", pattern: &[12.7, -6.35] },
    LineType {
        name: "PHANTOM",
        description: "Phantom ______  __  __  ______",
        pattern: &[31.75, -6.35, 6.35, -6.35, 6.35, -6.35],
    },
    LineType {
        name: "PHANTOM2",
        description: "Phantom (.5x) ___ _ _ ___",
        pattern: &[15.875, -3.175, 3.175, -3.175, 3.175, -3.175],
    },
    LineType {
        name: "ZIGZAG",
        description: "Zig zag /\\/\\/\\/\\/",
        pattern: &[0.00254, -5.08, 0.00254, -5.08],
    },
];

/// The **line type** a name says, if it says one this tree has.
///
/// The name is found without regard to its case, the way the tree
/// finds a line type.
pub fn of(name: &str) -> Option<&'static LineType> {
    let wanted = name.trim();
    TYPES.iter().find(|line_type| line_type.name.eq_ignore_ascii_case(wanted))
}

/// The **names** of the line types, for a refusal that says what they
/// are.
pub fn names() -> String {
    TYPES.iter().map(|line_type| line_type.name).collect::<Vec<&str>>().join(", ")
}

/// A line type, as it is reported.
pub fn to_json(line_type: &LineType) -> Json {
    json!({
        "name": line_type.name,
        "description": line_type.description,
        "pattern": line_type.pattern.to_vec(),
    })
}

commands! {
    /// **Read** the line types of this tree.
    cad.linetype.list {
        area: "cad",
        layer: "cad",
        icon: "line",
        params: {
        },
        run: {
            let types: Vec<Json> = TYPES.iter().map(to_json).collect();
            Ok(Outcome::new(format!("this tree has {} line type(s)", types.len()))
                .with_data(json!({ "linetypes": types })))

        }
    }
}
