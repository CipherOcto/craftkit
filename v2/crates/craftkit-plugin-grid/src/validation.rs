//! The **validation** of a range of cells.
//!
//! A validation rule says what may be typed into a range: a number of
//! a kind, a word from a list, a length of text — and a rule says so
//! with a **kind**, an **operator** and one or two **formulas**.
//!
//! The tree's finding is that a rule is **written for the first cell
//! of its range** and the relative references in it move with the
//! cell: a rule on `A1:A9` whose formula is `A1>10` is checked at
//! `A5` as `A5>10`. That is the tree's own rule, and it is the tree's
//! `shift_relative`.
//!
//! And the tree's finding is that an **unknown kind is refused**: the
//! tree's command used to give an "any value" rule for a kind it did
//! not know, and it does not any more — a rule that is not a rule
//! cannot be checked.
//!
//! # What it does not do
//!
//! - **The tree's custom rule is not here.** The tree's rule of the
//!   kind `custom` is a formula that says yes or no about the value
//!   being typed; the formula engine here cannot see a value that is
//!   not in a cell yet, and a rule that cannot be checked is not a
//!   rule.
//! - **The tree's error style is not here**: the tree's rule says
//!   whether a bad value is an error, a warning or a note, and that is
//!   about a host's message and not about the rule.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{self, Area, Book, CellRef, Expr, Value};

/// The **kinds** of a validation rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Anything at all: the tree's "any value".
    Any,
    /// A whole number.
    Whole,
    /// A number with a fraction.
    Decimal,
    /// A word from a list.
    List,
    /// A date.
    Date,
    /// A time.
    Time,
    /// A number of characters.
    TextLength,
}

impl Kind {
    /// The kind a name says, if it is one.
    pub fn parse(name: &str) -> Option<Kind> {
        Some(match name {
            "any" => Kind::Any,
            "whole" => Kind::Whole,
            "decimal" => Kind::Decimal,
            "list" => Kind::List,
            "date" => Kind::Date,
            "time" => Kind::Time,
            "textLength" | "text_length" => Kind::TextLength,
            _ => return None,
        })
    }

    /// The name of the kind.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Any => "any",
            Kind::Whole => "whole",
            Kind::Decimal => "decimal",
            Kind::List => "list",
            Kind::Date => "date",
            Kind::Time => "time",
            Kind::TextLength => "textLength",
        }
    }

    /// The kinds, for the message that refuses one.
    pub const NAMES: [&'static str; 7] = ["any", "whole", "decimal", "list", "date", "time", "textLength"];
}

/// The **operators** of a validation rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// Between the two: the tree's default.
    Between,
    /// Not between the two.
    NotBetween,
    /// Equal to the first.
    Equal,
    /// Not equal to the first.
    NotEqual,
    /// Greater than the first.
    Greater,
    /// Less than the first.
    Less,
    /// Greater than or equal to the first.
    GreaterOrEqual,
    /// Less than or equal to the first.
    LessOrEqual,
}

impl Operator {
    /// The operator a name says, if it is one.
    ///
    /// The tree's names and the tree's aliases: `greaterThan` is
    /// `greater` and `lessThanOrEqual` is `lessOrEqual`.
    pub fn parse(name: &str) -> Option<Operator> {
        Some(match name {
            "between" => Operator::Between,
            "notBetween" | "not_between" => Operator::NotBetween,
            "equal" => Operator::Equal,
            "notEqual" | "not_equal" => Operator::NotEqual,
            "greater" | "greaterThan" | "greater_than" => Operator::Greater,
            "less" | "lessThan" | "less_than" => Operator::Less,
            "greaterOrEqual" | "greaterThanOrEqual" | "greater_or_equal" => Operator::GreaterOrEqual,
            "lessOrEqual" | "lessThanOrEqual" | "less_or_equal" => Operator::LessOrEqual,
            _ => return None,
        })
    }

    /// The name of the operator.
    pub fn name(self) -> &'static str {
        match self {
            Operator::Between => "between",
            Operator::NotBetween => "notBetween",
            Operator::Equal => "equal",
            Operator::NotEqual => "notEqual",
            Operator::Greater => "greater",
            Operator::Less => "less",
            Operator::GreaterOrEqual => "greaterOrEqual",
            Operator::LessOrEqual => "lessOrEqual",
        }
    }

    /// The operators, for the message that refuses one.
    pub const NAMES: [&'static str; 8] =
        ["between", "notBetween", "equal", "notEqual", "greater", "less", "greaterOrEqual", "lessOrEqual"];
}

/// A **validation rule**: a kind, an operator and the formulas that
/// go with it, over a range of cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// The range the rule is over, as it is written: `A1:A9`.
    pub range: String,
    /// What may be typed.
    pub kind: Kind,
    /// How the value is compared.
    pub operator: Operator,
    /// The first formula: the number, the word or the list.
    pub formula1: String,
    /// The second formula, for the operators that have two.
    pub formula2: Option<String>,
    /// Whether a blank is allowed: the tree's default is that it is.
    pub allow_blank: bool,
}

impl Rule {
    /// The rule as it is written.
    pub fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "range": self.range,
            "kind": self.kind.name(),
            "operator": self.operator.name(),
            "formula1": self.formula1,
            "formula2": self.formula2,
            "allow_blank": self.allow_blank,
        })
    }

    /// The rule as it is read.
    pub fn from_json(value: &craftkit_sdk::serde_json::Value) -> Option<Rule> {
        let kind = Kind::parse(value.get("kind")?.as_str()?)?;
        let operator = Operator::parse(value.get("operator")?.as_str()?)?;
        Some(Rule {
            range: value.get("range")?.as_str()?.to_string(),
            kind,
            operator,
            formula1: value.get("formula1").and_then(|text| text.as_str()).unwrap_or("").to_string(),
            formula2: value.get("formula2").and_then(|text| text.as_str()).map(|text| text.to_string()),
            allow_blank: value.get("allow_blank").and_then(|blank| blank.as_bool()).unwrap_or(true),
        })
    }
}

/// The **rules** of a sheet, as the document holds them.
pub fn rules_of(node: &Node) -> Vec<Rule> {
    let Some(list) = node.data.get("validations").and_then(|list| list.as_array()) else {
        return Vec::new();
    };
    list.iter().filter_map(Rule::from_json).collect()
}

/// The **rules** of a sheet, into the document.
fn rules_into(node: &mut Node, rules: &[Rule]) {
    node.data["validations"] =
        craftkit_sdk::serde_json::Value::Array(rules.iter().map(|rule| rule.to_json()).collect());
}

/// The area a rule's range names.
fn area_of(rule: &Rule) -> Option<Area> {
    let text = rule.range.trim();
    let mut parts = text.split(':');
    let first = CellRef::parse(parts.next()?)?;
    let second = match parts.next() {
        Some(second) => CellRef::parse(second)?,
        None => first,
    };
    Some(Area::new(first, second))
}

/// The rule that covers a cell, if one does.
///
/// The **first** rule that covers the cell is the one that applies:
/// the tree's rules are in the order they were set, and the first one
/// that covers a cell is the one it is checked against.
pub fn rule_for(rules: &[Rule], cell: CellRef) -> Option<&Rule> {
    rules.iter().find(|rule| area_of(rule).map(|area| area.covers(cell)).unwrap_or(false))
}

/// The **items** of a list rule.
///
/// The tree's rule: a list written in quotes is a list of words, and
/// a list written without quotes is one too, and a list that is a
/// reference is a range of cells.
pub fn list_items(book: &Book, sheet: &str, rule: &Rule) -> Vec<String> {
    let text = rule.formula1.trim();
    if text.starts_with('"') {
        let body = text.strip_prefix('"').unwrap_or_default();
        let body = body.strip_suffix('"').unwrap_or(body);
        return body
            .replace("\"\"", "\"")
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect();
    }
    if let Ok(Expr::Range { start, end, .. }) = formula::parse(text) {
        let area = Area::new(start.cell, end.cell);
        let mut items = Vec::new();
        for row in area.start.row..=area.end.row {
            for col in area.start.col..=area.end.col {
                let value = book.value_of(sheet, CellRef { row, col });
                let text = value_text(&value);
                if text.is_empty() {
                    continue;
                }
                items.push(text);
            }
        }
        return items;
    }
    text.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
}

/// The text of a value, as a cell shows it.
fn value_text(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Error(message) => message.clone(),
    }
}

/// The **expression** a rule's formula says, moved to the cell it is
/// checked at.
///
/// The tree's rule: a rule is written for the first cell of its
/// range, and the relative references in it move with the cell — a
/// rule on `A1:A9` whose formula is `A1>10` is checked at `A5` as
/// `A5>10`. The tree's `shift_relative`, and the shift is here.
fn expression_at(text: &str, first: CellRef, at: CellRef) -> Option<Expr> {
    let expression = formula::parse(text.trim().trim_start_matches('=')).ok()?;
    if first == at {
        return Some(expression);
    }
    let dr = at.row as i64 - first.row as i64;
    let dc = at.col as i64 - first.col as i64;
    formula::shift(&expression, dr, dc).ok()
}

/// The number a rule's formula says, in a sheet, at a cell.
///
/// A formula that is a number is the number; a formula that is a
/// reference is what the cell holds.
fn number_of(book: &Book, sheet: &str, first: CellRef, at: CellRef, text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<f64>() {
        if number.is_finite() {
            return Some(number);
        }
        return None;
    }
    let expression = expression_at(text, first, at)?;
    let mut being = Vec::new();
    book.evaluate(sheet, &expression, &mut being).as_number()
}

/// The **number** a value is, for a rule of a kind.
///
/// A whole number and a decimal are numbers as they are written; a
/// date is the day it falls on, in the tree's calendar; a time is the
/// fraction of the day it is.
fn value_number(input: &str, kind: Kind, system: crate::dates::System) -> Option<f64> {
    match kind {
        Kind::Whole | Kind::Decimal => {
            let number = input.trim().parse::<f64>().ok()?;
            if !number.is_finite() {
                return None;
            }
            Some(number)
        }
        Kind::Date => {
            let (date, _) = crate::dates::parse(input, system)?;
            crate::dates::to_serial(date.year, date.month, date.day, system)
        }
        Kind::Time => {
            let (time, _) = crate::dates::parse_time(input)?;
            Some(crate::dates::time_fraction(time.hour, time.minute, time.second))
        }
        Kind::List | Kind::TextLength | Kind::Any => None,
    }
}

/// The value a rule says, when the **input** is checked at a cell.
///
/// `Ok(None)` is the value passing the rule.
pub fn check(
    book: &Book,
    sheet: &str,
    rules: &[Rule],
    cell: CellRef,
    input: &str,
    system: crate::dates::System,
) -> Option<(Rule, String)> {
    let rule = rule_for(rules, cell)?.clone();
    if rule.kind == Kind::Any {
        return None;
    }
    if input.is_empty() && rule.allow_blank {
        return None;
    }
    let first = area_of(&rule).map(|area| area.start).unwrap_or(cell);
    let ok = match rule.kind {
        Kind::List => {
            let items = list_items(book, sheet, &rule);
            items.iter().any(|item| item.eq_ignore_ascii_case(input))
        }
        Kind::TextLength => {
            let length = input.chars().count() as f64;
            compare(
                rule.operator,
                length,
                number_of(book, sheet, first, cell, &rule.formula1),
                number_of(book, sheet, first, cell, &rule.formula2.clone().unwrap_or_default()),
            )
        }
        Kind::Whole | Kind::Decimal | Kind::Date | Kind::Time => {
            let Some(number) = value_number(input, rule.kind, system) else {
                return Some((rule.clone(), message()));
            };
            if rule.kind == Kind::Whole && number.fract() != 0.0 {
                return Some((rule.clone(), message()));
            }
            compare(
                rule.operator,
                number,
                number_of(book, sheet, first, cell, &rule.formula1),
                number_of(book, sheet, first, cell, &rule.formula2.clone().unwrap_or_default()),
            )
        }
        Kind::Any => true,
    };
    if ok { None } else { Some((rule.clone(), message())) }
}

/// The message a rule says when a value does not pass it.
fn message() -> String {
    "This value doesn't match the data validation restrictions defined for this cell.".to_string()
}

/// Whether a number passes an operator.
fn compare(operator: Operator, number: f64, first: Option<f64>, second: Option<f64>) -> bool {
    let Some(first) = first else { return false };
    match operator {
        Operator::Between => {
            second.is_some_and(|second| number >= first.min(second) && number <= first.max(second))
        }
        Operator::NotBetween => {
            second.is_some_and(|second| number < first.min(second) || number > first.max(second))
        }
        Operator::Equal => number == first,
        Operator::NotEqual => number != first,
        Operator::Greater => number > first,
        Operator::Less => number < first,
        Operator::GreaterOrEqual => number >= first,
        Operator::LessOrEqual => number <= first,
    }
}

/// Put a **validation rule** on a range of a sheet.
///
/// The rule is a kind, an operator and the formulas that go with it.
/// A kind that is **not a kind** is refused: the tree's command used
/// to give an "any value" rule for a kind it did not know, and it
/// does not any more.
///
/// A **list** rule needs a list: a rule with no list in it cannot be
/// checked, and it is refused.
#[derive(Command)]
#[command(id = "grid.validation.set", area = "grid", layer = "grid", icon = "check", mutates, undoable)]
pub struct SetValidation {
    /// The sheet the rule is on.
    pub sheet: NodeRef,
    /// The range the rule is over: `A1:A9`.
    pub range: Text,
    /// The kind: `any`, `whole`, `decimal`, `list`, `date`, `time`,
    /// `textLength`.
    pub kind: Text,
    /// The operator: `between` and the rest.
    pub operator: Option<Text>,
    /// The first formula: the number, or the list.
    pub formula1: Option<Text>,
    /// The second formula, for the operators that have two.
    pub formula2: Option<Text>,
    /// Whether a blank is allowed. The tree's default is that it is.
    pub ignore_blank: Option<Bool>,
}

impl SetValidation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.validation.set".to_string();
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
        let kind_name = self.kind.0.trim().to_string();
        let kind = Kind::parse(&kind_name).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("unknown type `{kind_name}` ({})", Kind::NAMES.join(", ")),
        })?;
        let operator_name = self.operator.as_ref().map(|operator| operator.0.trim().to_string());
        let operator = match &operator_name {
            Some(name) => Operator::parse(name).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("unknown operator `{name}` ({})", Operator::NAMES.join(", ")),
            })?,
            None => Operator::Between,
        };
        let formula1 = self.formula1.as_ref().map(|formula| formula.0.trim().to_string()).unwrap_or_default();
        if kind == Kind::List && formula1.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "a list validation needs `formula1`".to_string(),
            });
        }
        let rule = Rule {
            range: self.range.0.trim().to_string(),
            kind,
            operator,
            formula1,
            formula2: self.formula2.as_ref().map(|formula| formula.0.trim().to_string()),
            allow_blank: self.ignore_blank.map(|blank| blank.0).unwrap_or(true),
        };
        // The **range** must be a range: a rule over nothing is a rule
        // over no cells.
        if area_of(&rule).is_none() {
            return Err(EngineError::Failed { id, message: format!("`{}` is not a range", rule.range) });
        }
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let mut rules = rules_of(node);
        rules.push(rule.clone());
        rules_into(node, &rules);
        Ok(Outcome::changed(format!("put a validation rule on {}", rule.range)).with_data(json!({
            "sheet": sheet,
            "rule": rule.to_json(),
        })))
    }
}

/// Take the **validation rules** off a range of a sheet.
#[derive(Command)]
#[command(id = "grid.validation.clear", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct ClearValidation {
    /// The sheet.
    pub sheet: NodeRef,
    /// The range to clear.
    pub range: Text,
}

impl ClearValidation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.validation.clear".to_string();
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
        let text = self.range.0.trim().to_string();
        let area = crate::fill::area_at(&text, &id)?;
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let mut rules = rules_of(node);
        rules.retain(|rule| {
            let rule_area = area_of(rule);
            match rule_area {
                Some(rule_area) => !(rule_area.start == area.start && rule_area.end == area.end),
                None => true,
            }
        });
        rules_into(node, &rules);
        Ok(Outcome::changed(format!("cleared the validation rules on {text}"))
            .with_data(json!({ "sheet": sheet, "range": text })))
    }
}

/// Check a **value** against the validation rules of a cell.
///
/// The rule that covers the cell is the one that is checked, and the
/// first such rule is the one: the tree's rules are in the order they
/// were set.
///
/// The value passes when there is no rule, and it fails with the
/// tree's own message.
#[derive(Command)]
#[command(id = "grid.validation.check", area = "grid", layer = "grid", icon = "check")]
pub struct CheckValidation {
    /// The sheet.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The value to check.
    pub input: Text,
}

impl CheckValidation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.validation.check".to_string();
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
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let rules = rules_of(node);
        let name = node.name.clone();
        let book = crate::book_of(session.document());
        let input = self.input.0.clone();
        let system = crate::system_of(session, sheet);
        match check(&book, &name, &rules, cell, &input, system) {
            None => Ok(Outcome::new(format!("the value passes the rules of {}", cell.to_text()))
                .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "ok": true }))),
            Some((rule, message)) => Ok(Outcome::new("the value does not pass the rules".to_string())
                .with_data(json!({
                    "sheet": sheet,
                    "cell": cell.to_text(),
                    "ok": false,
                    "message": message,
                    "rule": rule.to_json(),
                }))),
        }
    }
}
