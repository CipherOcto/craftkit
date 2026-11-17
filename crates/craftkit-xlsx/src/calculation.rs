//! The **calculation settings** of a workbook: how the file says it
//! calculates.
//!
//! A file holds them in one place, `<calcPr>`, and they say four things
//! and a fifth: how the workbook calculates, whether it calculates
//! **circular references** by iterating, how many times it iterates, how
//! much a value may change before it stops iterating, and whether it
//! calculates with the precision of what is **shown**.
//!
//! The settings are read as they are written, and a value that is not
//! one this tree knows is **not read**: the tree says so and keeps the
//! default rather than guessing at what the file meant.

use craftkit_xml::El;

/// How a workbook calculates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// The workbook calculates by itself.
    #[default]
    Automatic,
    /// The workbook calculates by itself, except for its tables.
    AutomaticExceptTables,
    /// The workbook calculates when it is told to.
    Manual,
}

impl Mode {
    /// The **name** of the mode, as the tree says it.
    pub fn name(&self) -> &'static str {
        match self {
            Mode::Automatic => "automatic",
            Mode::AutomaticExceptTables => "automaticExceptTables",
            Mode::Manual => "manual",
        }
    }
}

/// The **calculation settings** of a workbook.
#[derive(Debug, Clone, PartialEq)]
pub struct Calculation {
    /// How the workbook calculates.
    pub mode: Mode,
    /// Whether the workbook calculates **circular references** by
    /// iterating.
    pub iterative: bool,
    /// The most times the workbook iterates.
    pub max_iterations: u32,
    /// The most a value may change before the workbook stops iterating.
    pub max_change: f64,
    /// Whether the workbook calculates with the precision of what is
    /// shown rather than with the precision of the number.
    pub precision_as_displayed: bool,
}

impl Default for Calculation {
    fn default() -> Self {
        Calculation {
            mode: Mode::default(),
            iterative: false,
            max_iterations: DEFAULT_ITERATIONS,
            max_change: DEFAULT_CHANGE,
            precision_as_displayed: false,
        }
    }
}

/// The most **iterations** a file may name.
pub const MAX_ITERATIONS: u32 = 32_767;

/// The most iterations a workbook has when it says nothing.
pub const DEFAULT_ITERATIONS: u32 = 100;

/// The most a value may change, when the workbook says nothing.
pub const DEFAULT_CHANGE: f64 = 0.001;

/// Read the **calculation settings** of a workbook, and say what was
/// not read.
///
/// A file with no settings has the default ones: a workbook that says
/// nothing calculates by itself, does not iterate, and calculates with
/// the precision of the number.
pub fn read(element: Option<&El>) -> (Calculation, Vec<String>) {
    let mut settings = Calculation::default();
    let mut warnings = Vec::new();
    let Some(element) = element else { return (settings, warnings) };
    match element.attr("calcMode") {
        None => {}
        Some("auto") => settings.mode = Mode::Automatic,
        Some("autoNoTable") => settings.mode = Mode::AutomaticExceptTables,
        Some("manual") => settings.mode = Mode::Manual,
        Some(other) => warnings.push(format!(
            "the mode of calculation `{other}` is not one this tree knows; the automatic mode is used"
        )),
    }
    settings.iterative = element.flag("iterate", false);
    if let Some(text) = element.attr("iterateCount") {
        match text.trim().parse::<u32>() {
            Ok(number) if (1..=MAX_ITERATIONS).contains(&number) => settings.max_iterations = number,
            _ => warnings.push(format!(
                "the most iterations of the file is `{text}`, and the most a file may name is {MAX_ITERATIONS}; the default is used"
            )),
        }
    }
    if let Some(text) = element.attr("iterateDelta") {
        match text.trim().parse::<f64>() {
            Ok(number) if number.is_finite() => settings.max_change = number.abs(),
            _ => warnings.push(format!(
                "the most a value may change in the file is `{text}`, and that is not a number; the default is used"
            )),
        }
    }
    settings.precision_as_displayed = !element.flag("fullPrecision", true);
    (settings, warnings)
}
