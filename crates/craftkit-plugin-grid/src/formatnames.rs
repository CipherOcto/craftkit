//! The **names** of a format.
//!
//! The tree's ribbon offers its formats by **name**: `General`,
//! `Number`, `Currency`, `Accounting`, `Short Date`, `Long Date`,
//! `Time`, `Percentage`, `Fraction`, `Scientific`, `Text` and `Comma`.
//! Each name says a code, and the tree keeps the code and not the
//! name: the name is a way of saying the code.
//!
//! Here a format may be given either by its **code** or by its
//! **name**, and the name is read apart from the code: a code is a
//! code of this tree, and a name is one of the names below.
//!
//! The names this tree **honours** are twelve, and each says a code
//! this tree has:
//!
//! | the name | the code it says |
//! |---|---|
//! | `general` | `general` |
//! | `number` | `0.00` |
//! | `currency` | `"$"#,##0.00` |
//! | `accounting` | `accounting` |
//! | `comma` | `comma` |
//! | `percentage` | `0.00%` |
//! | `fraction` | `# ?/?` |
//! | `scientific` | `0.00E+00` |
//! | `short date` | `m/d/yyyy` |
//! | `long date` | `dddd, mmmm dd, yyyy` |
//! | `time` | `h:mm:ss AM/PM` |
//! | `text` | `@` |
//!
//! The **accounting** and the **comma** formats are the tree's own
//! **filling** ones: they write the symbol at the edge of the cell and
//! the number at the other edge, and the space between them is the
//! **fill**. The symbol of the accounting one may be chosen, and it is
//! written in quotes: `accounting "L"`.
//!
//! The names this tree **knows but does not honour** are **none**:
//! every name of the tree's ribbon is a name this tree writes.
//!
//! And a name that is **not one of these** is refused as a format that
//! this tree does not have: a name that is quietly ignored is a name
//! the user believes in and the program does not have.

use crate::format::Format;

/// The names this tree honours, and the code each one says.
pub const HONOURED: [(&str, &str); 12] = [
    ("general", "general"),
    ("number", "0.00"),
    ("currency", "\"$\"#,##0.00"),
    ("accounting", "accounting"),
    ("comma", "comma"),
    ("percentage", "0.00%"),
    ("fraction", "# ?/?"),
    ("scientific", "0.00E+00"),
    ("short date", "m/d/yyyy"),
    ("long date", "dddd, mmmm dd, yyyy"),
    ("time", "h:mm:ss AM/PM"),
    ("text", "@"),
];

/// The names this tree knows but does not honour, and the reason.
///
/// There are **none**: every name of the tree's ribbon is a name this
/// tree writes. The list is here because the refusal that names a
/// format and says why is a rule of this tree, and a rule with no
/// names in it is still a rule.
pub const REFUSED: [(&str, &str); 0] = [];

/// Why a format was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The input is neither a code of this tree nor a name of this
    /// tree.
    Unknown,
    /// The input is a name this tree knows but does not honour.
    Unsupported {
        /// The name, as it was written.
        name: String,
        /// Why this tree does not write it.
        reason: &'static str,
    },
}

/// The **code** a format is written with.
///
/// A code is a code of this tree, and a code wins: a name is read
/// only when the input is not a code.
pub fn resolve(input: &str) -> Result<String, Refusal> {
    let code = input.trim();
    if Format::parse(code).is_some() {
        return Ok(code.to_string());
    }
    let key = code.to_lowercase();
    if let Some((_, said)) = HONOURED.iter().find(|(name, _)| *name == key) {
        return Ok(said.to_string());
    }
    if let Some((name, reason)) = REFUSED.iter().find(|(name, _)| *name == key) {
        return Err(Refusal::Unsupported { name: name.to_string(), reason });
    }
    Err(Refusal::Unknown)
}

/// The **message** a refusal is written with.
///
/// The message says what a format is, so that a refusal tells the
/// user what there is to write and not only that there is a mistake.
pub fn message(input: &str, refusal: &Refusal) -> String {
    match refusal {
        Refusal::Unknown => format!("`{input}` is not a format this tree has: {FORMATS}"),
        Refusal::Unsupported { name, reason } => {
            format!("the format `{name}` is not one this tree writes: {reason}")
        }
    }
}

/// What a format is, said in full.
pub const FORMATS: &str = "a number format is `general`, a whole part of `0` and `#`, up to fifteen decimals and a `%`, or a number with a symbol in quotes; a fraction is `# ?/?`, `# ??/??`, `?/?` or `# ?/4`; a scientific format is a mantissa, an `E` and the exponent's digits, such as `0.00E+00`; a text format is `@`, which writes the text of a cell; and a format may have sections, separated by a semicolon; and a date format is `yyyy-mm-dd`, `dd/mm/yyyy`, `m/d/yyyy`, `d-mmm-yyyy`, `mmm`, `mmmm`, `ddd`, `dddd`, `yyyy` or `dddd, mmmm dd, yyyy`; and a time format is `h`, `hh`, `h:mm`, `hh:mm`, `h:mm:ss`, `hh:mm:ss`, `h AM/PM`, `h:mm AM/PM`, `h:mm:ss AM/PM`, `hh:mm:ss AM/PM`, `[h]:mm`, `[h]:mm:ss` or `[mm]:ss`; and a format may be named `general`, `number`, `currency`, `accounting`, `comma`, `percentage`, `fraction`, `scientific`, `short date`, `long date`, `time` or `text`";
