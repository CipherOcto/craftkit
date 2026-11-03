//! The **sections** of a format.
//!
//! A format may have a different code for the **positive** numbers, the
//! **negative** ones, the **zero** and the **text**, and the codes are
//! separated by a **semicolon**.
//!
//! - one section — every number, and the text;
//! - two sections — the first for the positive numbers and the zero,
//!   the second for the negative ones;
//! - three sections — the first for the positive numbers, the second
//!   for the negative ones and the third for the zero.
//!
//! The tree's rule about the **negative** section is the one that
//! matters: a negative section shows the **absolute value** and the
//! sign is **not** written, because the section is the one that says
//! how a negative number looks. A format with one section has no
//! second section, so the sign is written.
//!
//! # What it does not do
//!
//! - **The text section is not here.** The tree's fourth section is
//!   the text's own, and here a text is written as it is.
//! - **The conditions are not here.** The tree's sections may carry a
//!   condition — `[>100]` — and pick the section by the number's
//!   value. Here the sections are picked by the number's **sign** and
//!   by whether it is **zero**, and nothing else.
//! - **An empty section is refused.** The tree's empty section shows
//!   nothing for the numbers it covers; here a code with an empty
//!   section is not a format this tree has.

/// The **sections** a code has.
///
/// The codes are separated by a semicolon, and a semicolon inside a
/// literal in quotes is not a separator. `None` when the code has a
/// literal that is never closed.
pub fn split(code: &str) -> Option<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut literal = false;
    let mut chars = code.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '"' => {
                current.push(character);
                // Two quotes in a row are an escaped quote, and the
                // literal goes on.
                if literal && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    literal = !literal;
                }
            }
            ';' if !literal => {
                out.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(character),
        }
    }
    if literal {
        return None;
    }
    out.push(current.trim().to_string());
    Some(out)
}
