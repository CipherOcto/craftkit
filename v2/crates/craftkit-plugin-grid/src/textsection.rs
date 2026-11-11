//! The **text section** of a format.
//!
//! A format may end with a section that writes the **text** of a cell:
//! the `@` is where the text goes and the literals around it are
//! written as they are. The tree's own code for it is `@`, and the
//! tree's ribbon names it `Text`.
//!
//! The tree's rule for **which** section is the text's own:
//!
//! - a code with **four** sections: the fourth is the text's own, and
//!   the first three are the positive numbers, the negative ones and
//!   the zero;
//! - a code with **fewer** than four: the **last** one is the text's
//!   own when it is a text section — a section with an `@` in it and
//!   no digits — and the ones before it are the numbers' sections.
//!
//! A text section is a sequence of **literals** and **`@`**s, and the
//! literals are in **quotes**, as they are everywhere in a format
//! code. A section with anything else in it is not a text section: it
//! is a number section that happens to hold an `@`, and the tree says
//! so by looking for digits in it.

/// A **text section**: the parts around the `@`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSection {
    /// The literals: the text is written between them, and there is
    /// one more of them than there are `@`s.
    pub parts: Vec<String>,
}

impl TextSection {
    /// The text section a code says, if it says one.
    ///
    /// `None` when the code has no `@` in it, or when it has
    /// something in it that is neither a literal in quotes nor an `@`.
    pub fn parse(code: &str) -> Option<TextSection> {
        let code = code.trim();
        let mut parts: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut places = 0usize;
        let mut characters = code.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '"' => {
                    let mut literal = String::new();
                    loop {
                        let character = characters.next()?;
                        if character == '"' {
                            // Two quotes in a row are an escaped quote,
                            // and the literal goes on.
                            if characters.peek() == Some(&'"') {
                                literal.push('"');
                                characters.next();
                                continue;
                            }
                            break;
                        }
                        literal.push(character);
                    }
                    current.push_str(&literal);
                }
                '@' => {
                    parts.push(std::mem::take(&mut current));
                    places += 1;
                }
                _ => return None,
            }
        }
        // A section with no `@` in it is not a text section.
        if places == 0 {
            return None;
        }
        parts.push(current);
        Some(TextSection { parts })
    }

    /// The **text** a value is written with.
    ///
    /// The text is written between the literals, once for every `@`.
    pub fn text(&self, text: &str) -> String {
        let mut out = String::new();
        for (index, part) in self.parts.iter().enumerate() {
            out.push_str(part);
            if index + 1 < self.parts.len() {
                out.push_str(text);
            }
        }
        out
    }
}
