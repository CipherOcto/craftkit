//! The **bounds** this README promises are the bounds the tree has.
//!
//! # What this checks
//!
//! The README states a number for each bound the kit publishes, and this
//! test says whether that number is the number the code has.
//!
//! # Why the code hands the value over
//!
//! The check used to work the other way round: the tree's own gate
//! **re-parsed the code's text**, read the constant out of it, worked out
//! what it was, and compared that with the number the prose named. Two
//! parsers — the tree's and the compiler's — and a mistake in either was
//! invisible: a bound written in a form the tree's evaluator did not know
//! was not a failure, it was a **skip**, and a skip looks exactly like a
//! pass.
//!
//! So the numbers come from the **code** now. The SDK publishes each of
//! its bounds as a value, and the value is the **constant itself**, so the
//! compiler ties the two together: the published number cannot be stale.
//! This test compares the README with that.
//!
//! And a number that **cannot be worked out is a failure**, not a skip:
//! "ten thousand" is a number a reader understands and a checker cannot
//! evaluate, so the README states the number as an expression and the
//! words stay in the column beside it.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use craftkit_sdk::bounds::Bound;
use craftkit_sdk::contribution::Registry;

/// The marker that opens the table of the bounds.
const BEGIN: &str = "<!-- the bounds of the tree: begin -->";

/// The marker that closes it.
const END: &str = "<!-- the bounds of the tree: end -->";

/// The **root** of the tree: the folder that holds the crates and this
/// README.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// The **README** of the tree.
fn readme(root: &Path) -> String {
    fs::read_to_string(root.join("README.md")).expect("the README is there")
}

/// The bounds the tree publishes.
fn bounds() -> Vec<Bound> {
    let registry = Registry::<Bound>::from_inventory().expect("the bounds are not a mess");
    registry.all().to_vec()
}

/// A row of the table of the bounds.
struct Row {
    /// The bound's name.
    name: String,
    /// The crate that declares it.
    crate_name: String,
    /// The number, as the README states it.
    expression: String,
}

/// The rows of the marked table.
///
/// The table has a fixed shape: the bound, the crate, the number.
fn the_table(readme: &str) -> Option<Vec<Row>> {
    let start = readme.find(BEGIN)? + BEGIN.len();
    let end = readme[start..].find(END)? + start;
    let mut rows = Vec::new();
    for line in readme[start..end].lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<String> = line.split('|').map(|cell| cell.trim().to_string()).collect();
        if cells.len() < 4 {
            continue;
        }
        let name = cells[1].trim_matches('`').to_string();
        if !is_a_name(&name) {
            continue;
        }
        rows.push(Row {
            name,
            crate_name: cells[2].trim_matches('`').to_string(),
            expression: cells[3].trim_matches('`').to_string(),
        });
    }
    Some(rows)
}

/// The name in the first cell of a table row, if it is a bound's name.
fn the_bound_name(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let cells: Vec<&str> = line.split('|').map(|cell| cell.trim()).collect();
    let first = cells.get(1)?;
    let name = first.trim_matches('`');
    if !name.starts_with("MAX_") {
        return None;
    }
    Some(name.to_string())
}

/// Whether a cell holds a bound's name.
///
/// A name is an identifier: a header says "the bound" and a separator says
/// "---", and neither is a promise.
fn is_a_name(cell: &str) -> bool {
    let mut characters = cell.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// What is wrong with the README, if anything.
///
/// The problems are the sentences a reader of the gate would want: what
/// is promised, what the tree says, and what the two are.
fn problems(readme: &str, bounds: &[Bound]) -> Vec<String> {
    let mut out = Vec::new();
    let names: BTreeMap<&str, usize> = bounds.iter().map(|bound| (bound.name, bound.value)).collect();

    let Some(rows) = the_table(readme) else {
        return vec!["the README has no table of the bounds".to_string()];
    };
    if rows.is_empty() {
        out.push("the table of the bounds is empty".to_string());
    }

    let mut promised: Vec<&str> = Vec::new();
    for row in &rows {
        promised.push(&row.name);
        let declared =
            bounds.iter().find(|bound| bound.name == row.name && bound.crate_name == row.crate_name);
        let Some(declared) = declared else {
            out.push(format!(
                "the README promises `{}` of `{}`, and the tree declares no such bound",
                row.name, row.crate_name
            ));
            continue;
        };
        // A number that cannot be worked out is a **failure**. The old
        // check skipped it, and a skip looks like a pass.
        let Some(number) = evaluate(&row.expression, &names) else {
            out.push(format!(
                "the README states the number of `{}` as `{}`, which is not a number that can be worked out",
                row.name, row.expression
            ));
            continue;
        };
        if number != declared.value {
            out.push(format!(
                "the README says `{}` is `{}`, which is {number}, and the tree says {}",
                row.name, row.expression, declared.value
            ));
        }
    }

    // Every table of the README that promises a bound has to promise it in
    // the table of the bounds, and has to state a number. The table of the
    // bounds is checked above, so it is not walked again here.
    for line in outside_the_table(readme).lines() {
        let Some(name) = the_bound_name(line) else { continue };
        if !promised.contains(&name.as_str()) {
            out.push(format!(
                "the README promises `{name}` in a table, and the table of the bounds does not list it"
            ));
            continue;
        }
        let Some(number) = the_number_of(line, &names) else {
            out.push(format!("the README states no number for `{name}` that can be worked out"));
            continue;
        };
        let Some(declared) = names.get(name.as_str()) else { continue };
        if number != *declared {
            out.push(format!("the README says `{name}` is {number}, and the tree says {declared}"));
        }
    }

    out
}

/// The README without the table of the bounds.
///
/// The table is checked on its own — it has a shape of its own — so the
/// walk over the rest of the README does not walk it again.
fn outside_the_table(readme: &str) -> String {
    let Some(start) = readme.find(BEGIN) else { return readme.to_string() };
    let Some(end) = readme[start..].find(END) else { return readme.to_string() };
    let end = end + start + END.len();
    let mut out = String::new();
    out.push_str(&readme[..start]);
    out.push_str(&readme[end..]);
    out
}

/// The number a table row states, if it states one.
///
/// The number is the **first cell after the name that can be worked out**:
/// the row may also carry prose beside it, and prose is not a number.
fn the_number_of(line: &str, names: &BTreeMap<&str, usize>) -> Option<usize> {
    let cells: Vec<&str> = line.split('|').map(|cell| cell.trim()).collect();
    for cell in cells.iter().skip(2) {
        let cell = cell.trim_matches('`');
        if cell.is_empty() {
            continue;
        }
        if let Some(number) = evaluate(cell, names) {
            return Some(number);
        }
    }
    None
}

/// Work out the number an expression states.
///
/// The four operators a bound is written with — `*`, `+`, `-` and `<<` —
/// and the tree's own bounds as the names it may use. **`None` means the
/// expression could not be worked out**, and that is a failure, not a
/// skip.
fn evaluate(expression: &str, names: &BTreeMap<&str, usize>) -> Option<usize> {
    let tokens = tokenise(expression)?;
    let mut parser = Parser { tokens, names, at: 0 };
    let value = parser.sum()?;
    if parser.at != parser.tokens.len() {
        return None;
    }
    Some(value)
}

/// One token of an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A number.
    Number(usize),
    /// The name of a bound.
    Name(String),
    /// `*`
    Times,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `<<`
    Shift,
    /// `(`
    Open,
    /// `)`
    Close,
}

/// The tokens of an expression, or `None` when it is not one.
fn tokenise(expression: &str) -> Option<Vec<Token>> {
    let bytes: Vec<char> = expression.chars().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let character = bytes[at];
        if character.is_whitespace() {
            at += 1;
            continue;
        }
        if character.is_ascii_digit() {
            let mut text = String::new();
            while at < bytes.len() && (bytes[at].is_ascii_digit() || bytes[at] == '_') {
                if bytes[at] != '_' {
                    text.push(bytes[at]);
                }
                at += 1;
            }
            out.push(Token::Number(text.parse().ok()?));
            continue;
        }
        if character.is_ascii_alphabetic() || character == '_' {
            let mut text = String::new();
            while at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == '_') {
                text.push(bytes[at]);
                at += 1;
            }
            out.push(Token::Name(text));
            continue;
        }
        match character {
            '*' => {
                out.push(Token::Times);
                at += 1;
            }
            '+' => {
                out.push(Token::Plus);
                at += 1;
            }
            '-' => {
                out.push(Token::Minus);
                at += 1;
            }
            '(' => {
                out.push(Token::Open);
                at += 1;
            }
            ')' => {
                out.push(Token::Close);
                at += 1;
            }
            '<' if at + 1 < bytes.len() && bytes[at + 1] == '<' => {
                out.push(Token::Shift);
                at += 2;
            }
            _ => return None,
        }
    }
    Some(out)
}

/// The parser of an expression.
struct Parser<'a> {
    /// The tokens.
    tokens: Vec<Token>,
    /// The names an expression may use.
    names: &'a BTreeMap<&'a str, usize>,
    /// Where the parser is.
    at: usize,
}

impl Parser<'_> {
    /// A sum: the things added and taken away.
    fn sum(&mut self) -> Option<usize> {
        let mut value = self.product()?;
        loop {
            match self.tokens.get(self.at) {
                Some(Token::Plus) => {
                    self.at += 1;
                    value = value.checked_add(self.product()?)?;
                }
                Some(Token::Minus) => {
                    self.at += 1;
                    value = value.checked_sub(self.product()?)?;
                }
                _ => return Some(value),
            }
        }
    }

    /// A product: the things multiplied and shifted.
    fn product(&mut self) -> Option<usize> {
        let mut value = self.factor()?;
        loop {
            match self.tokens.get(self.at) {
                Some(Token::Times) => {
                    self.at += 1;
                    value = value.checked_mul(self.factor()?)?;
                }
                Some(Token::Shift) => {
                    self.at += 1;
                    value = value.checked_shl(u32::try_from(self.factor()?).ok()?)?;
                }
                _ => return Some(value),
            }
        }
    }

    /// A factor: a number, a name, or a sum in brackets.
    fn factor(&mut self) -> Option<usize> {
        match self.tokens.get(self.at)? {
            Token::Number(number) => {
                self.at += 1;
                Some(*number)
            }
            Token::Name(name) => {
                self.at += 1;
                self.names.get(name.as_str()).copied()
            }
            Token::Open => {
                self.at += 1;
                let value = self.sum()?;
                match self.tokens.get(self.at)? {
                    Token::Close => {
                        self.at += 1;
                        Some(value)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// The README's own numbers are the tree's numbers.
#[test]
fn the_numbers_agree_with_the_prose() {
    let tree = root();
    let readme = readme(&tree);
    let found = problems(&readme, &bounds());
    assert!(found.is_empty(), "the README and the tree disagree: {found:#?}");
}

/// The tree publishes the bounds the README promises.
#[test]
fn every_promised_bound_is_published() {
    let tree = root();
    let readme = readme(&tree);
    let rows = the_table(&readme).expect("the table of the bounds is there");
    let bounds = bounds();
    for row in &rows {
        assert!(
            bounds.iter().any(|bound| bound.name == row.name && bound.crate_name == row.crate_name),
            "the README promises `{}` of `{}`, and the tree publishes no such bound",
            row.name,
            row.crate_name
        );
    }
}

/// A number that cannot be worked out is a **failure**, not a skip.
///
/// This is the promise §849 made: the rows that said "ten thousand" were
/// skipped, and a skip looks exactly like a pass.
#[test]
fn a_number_in_words_is_a_failure() {
    let readme = "\
<!-- the bounds of the tree: begin -->
| the bound | the crate | the number |
|---|---|---|
| `MAX_NODES` | `craftkit-sdk` | ten thousand |
<!-- the bounds of the tree: end -->
";
    let bounds = vec![Bound { name: "MAX_NODES", crate_name: "craftkit-sdk", value: 10_000 }];
    let found = problems(readme, &bounds);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("not a number that can be worked out"), "{found:#?}");
}

/// A number that is not the tree's number is a failure.
#[test]
fn a_wrong_number_is_a_failure() {
    let readme = "\
<!-- the bounds of the tree: begin -->
| the bound | the crate | the number |
|---|---|---|
| `MAX_NODES` | `craftkit-sdk` | `9_999` |
<!-- the bounds of the tree: end -->
";
    let bounds = vec![Bound { name: "MAX_NODES", crate_name: "craftkit-sdk", value: 10_000 }];
    let found = problems(readme, &bounds);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("the tree says 10000"), "{found:#?}");
}

/// A bound the tree does not publish is a failure.
#[test]
fn a_bound_the_tree_does_not_have_is_a_failure() {
    let readme = "\
<!-- the bounds of the tree: begin -->
| the bound | the crate | the number |
|---|---|---|
| `MAX_MYSTERY` | `craftkit-sdk` | `42` |
<!-- the bounds of the tree: end -->
";
    let found = problems(readme, &[]);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("declares no such bound"), "{found:#?}");
}

/// A table that promises a bound the table of the bounds does not list is
/// a failure: the table of the bounds is the tree's list of promises.
#[test]
fn a_promise_outside_the_table_is_a_failure() {
    let readme = "\
<!-- the bounds of the tree: begin -->
| the bound | the crate | the number |
|---|---|---|
| `MAX_NODES` | `craftkit-sdk` | `10_000` |
<!-- the bounds of the tree: end -->

| the bound | it is | and it is |
|---|---|---|
| `MAX_DEPTH` | `64` | sixty-four levels |
";
    let bounds = vec![Bound { name: "MAX_NODES", crate_name: "craftkit-sdk", value: 10_000 }];
    let found = problems(readme, &bounds);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("does not list it"), "{found:#?}");
}

/// The evaluator says what the arithmetic says.
#[test]
fn the_evaluator_works_out_the_arithmetic() {
    let names: BTreeMap<&str, usize> = BTreeMap::from([("MAX_NODES", 10_000)]);
    assert_eq!(evaluate("64", &names), Some(64));
    assert_eq!(evaluate("10_000", &names), Some(10_000));
    assert_eq!(evaluate("32 * 1024 * 1024", &names), Some(33_554_432));
    assert_eq!(evaluate("MAX_NODES * 2", &names), Some(20_000));
    assert_eq!(evaluate("(2 + 3) * 4", &names), Some(20));
    assert_eq!(evaluate("1 << 20", &names), Some(1_048_576));
    assert_eq!(evaluate("ten thousand", &names), None);
    assert_eq!(evaluate("MAX_MYSTERY", &names), None);
}
