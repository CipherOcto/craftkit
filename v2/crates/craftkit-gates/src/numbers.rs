//! The **numbers**: an expression, and what it comes to.
//!
//! # Why it is here
//!
//! The gate reads two expressions — the one in the code and the one in
//! the prose — and has to know whether they are the **same number**. A
//! gate that compares the *text* of two expressions catches a typo and
//! nothing else: `32 * 1024 * 1024` and `32 * 1024 * 1024` are the same
//! and `33554432` and `32 * 1024 * 1024` are the same, and a reader
//! cannot tell that from the text.
//!
//! So the expressions are **evaluated**, with the tree's own constants
//! as the names they may use. The evaluator is small on purpose: it
//! knows the four operators an arithmetic bound is written with, and it
//! refuses everything else with a name.
//!
//! # What it does not do
//!
//! It does not evaluate a function call, a cast or a shift by a
//! variable: a bound whose number cannot be worked out from arithmetic
//! is a bound the gate **cannot check**, and it says so rather than
//! guessing.

/// A number that could not be worked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved(pub String);

impl std::fmt::Display for Unresolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The names an expression may use, and what they are worth.
pub trait Names {
    /// The value of a name, if the tree has one.
    ///
    /// The **depth** is how many names deep the reader already is: a
    /// name that is written in terms of itself would recurse for ever,
    /// and the compiler would not catch it, because the gate reads the
    /// text.
    fn value_of(&self, name: &str, depth: usize) -> Option<u64>;
}

/// A map of names to numbers, and to the expressions they are written
/// with.
#[derive(Debug, Clone, Default)]
pub struct Table {
    expressions: Vec<(String, String)>,
}

impl Table {
    /// A table with nothing in it.
    pub fn new() -> Self {
        Table { expressions: Vec::new() }
    }

    /// Say what a name is **written with**.
    pub fn set_expression(&mut self, name: &str, expression: &str) {
        self.expressions.retain(|(other, _)| other != name);
        self.expressions.push((name.to_string(), expression.to_string()));
    }

    /// The expression a name is written with.
    pub fn expression_of(&self, name: &str) -> Option<String> {
        self.expressions.iter().find(|(other, _)| other == name).map(|(_, expression)| expression.clone())
    }
}

impl Names for Table {
    fn value_of(&self, name: &str, depth: usize) -> Option<u64> {
        // A name that is written as an expression is worked out from it.
        let expression = self.expression_of(name)?;
        let table = self.clone();
        evaluate_at(&expression, &table, depth + 1).ok()
    }
}

/// The value of an expression, or why it has none.
///
/// The operators are `*`, `+`, `-` and `<<`, and `*` binds tightest.
/// The names are the tree's own constants, and a name that is not one
/// of them is a name the expression cannot be worked out from.
pub fn evaluate(expression: &str, names: &impl Names) -> Result<u64, Unresolved> {
    evaluate_at(expression, names, 0)
}

/// How deep the names may go.
///
/// A constant that is written in terms of itself would recurse for ever,
/// and the compiler would not catch it: the gate reads the **text**, and
/// the text may say anything. So the depth is bounded, and a name that
/// is too deep is a name the gate cannot work out.
const MAX_DEPTH: usize = 32;

/// The value of an expression, at that depth.
fn evaluate_at(expression: &str, names: &impl Names, depth: usize) -> Result<u64, Unresolved> {
    if depth > MAX_DEPTH {
        return Err(Unresolved(format!("`{expression}` is too deep")));
    }
    let tokens = tokenise(expression)?;
    let mut at = 0;
    let value = sum(&tokens, &mut at, names, depth)?;
    if at != tokens.len() {
        return Err(Unresolved(format!("`{expression}` has something left over")));
    }
    Ok(value)
}

/// One piece of an expression.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(u64),
    Name(String),
    Star,
    Plus,
    Minus,
    Shift,
    Open,
    Close,
}

/// The pieces of an expression.
fn tokenise(expression: &str) -> Result<Vec<Token>, Unresolved> {
    let mut out = Vec::new();
    let bytes: Vec<char> = expression.chars().collect();
    let mut at = 0;
    while at < bytes.len() {
        let character = bytes[at];
        match character {
            ' ' | '\t' | '\n' | '_' => at += 1,
            '*' => {
                out.push(Token::Star);
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
            '<' => {
                if bytes.get(at + 1) != Some(&'<') {
                    return Err(Unresolved(format!("`{expression}` has a `<` that is not a shift")));
                }
                out.push(Token::Shift);
                at += 2;
            }
            '0'..='9' => {
                let start = at;
                while at < bytes.len() && bytes[at].is_ascii_digit() {
                    at += 1;
                }
                let text: String = bytes[start..at].iter().collect();
                let value =
                    text.parse::<u64>().map_err(|_| Unresolved(format!("`{text}` is not a number")))?;
                out.push(Token::Number(value));
            }
            'a'..='z' | 'A'..='Z' => {
                let start = at;
                while at < bytes.len()
                    && (bytes[at].is_ascii_alphanumeric() || bytes[at] == '_' || bytes[at] == ':')
                {
                    at += 1;
                }
                let text: String = bytes[start..at].iter().collect();
                // A path is a name: the last segment is the name.
                let name = text.rsplit("::").next().unwrap_or(&text).to_string();
                out.push(Token::Name(name));
            }
            other => {
                return Err(Unresolved(format!("`{expression}` has a `{other}` in it")));
            }
        }
    }
    Ok(out)
}

/// A sum: an addition or a subtraction of products.
fn sum(tokens: &[Token], at: &mut usize, names: &impl Names, depth: usize) -> Result<u64, Unresolved> {
    let mut value = product(tokens, at, names, depth)?;
    while *at < tokens.len() {
        match tokens[*at] {
            Token::Plus => {
                *at += 1;
                value = value
                    .checked_add(product(tokens, at, names, depth)?)
                    .ok_or_else(|| Unresolved("the expression is too big".to_string()))?;
            }
            Token::Minus => {
                *at += 1;
                let right = product(tokens, at, names, depth)?;
                value = value
                    .checked_sub(right)
                    .ok_or_else(|| Unresolved("the expression goes below zero".to_string()))?;
            }
            _ => break,
        }
    }
    Ok(value)
}

/// A product: a multiplication of shifts.
fn product(tokens: &[Token], at: &mut usize, names: &impl Names, depth: usize) -> Result<u64, Unresolved> {
    let mut value = shift(tokens, at, names, depth)?;
    while *at < tokens.len() {
        match tokens[*at] {
            Token::Star => {
                *at += 1;
                let right = shift(tokens, at, names, depth)?;
                value = value
                    .checked_mul(right)
                    .ok_or_else(|| Unresolved("the expression is too big".to_string()))?;
            }
            _ => break,
        }
    }
    Ok(value)
}

/// A shift: a value and the bits it moves.
fn shift(tokens: &[Token], at: &mut usize, names: &impl Names, depth: usize) -> Result<u64, Unresolved> {
    let value = atom(tokens, at, names, depth)?;
    if *at < tokens.len() && tokens[*at] == Token::Shift {
        *at += 1;
        let bits = atom(tokens, at, names, depth)?;
        if bits > 63 {
            return Err(Unresolved("the expression shifts too far".to_string()));
        }
        return Ok(value << bits);
    }
    Ok(value)
}

/// A value: a number, a name or a parenthesised expression.
fn atom(tokens: &[Token], at: &mut usize, names: &impl Names, depth: usize) -> Result<u64, Unresolved> {
    let Some(token) = tokens.get(*at).cloned() else {
        return Err(Unresolved("the expression ends early".to_string()));
    };
    *at += 1;
    match token {
        Token::Number(value) => Ok(value),
        Token::Name(name) => names
            .value_of(&name, depth)
            .ok_or_else(|| Unresolved(format!("`{name}` is not a number this tree knows"))),
        Token::Open => {
            let value = sum(tokens, at, names, depth)?;
            if tokens.get(*at) != Some(&Token::Close) {
                return Err(Unresolved("a bracket is not closed".to_string()));
            }
            *at += 1;
            Ok(value)
        }
        other => Err(Unresolved(format!("`{other:?}` is not a value"))),
    }
}
