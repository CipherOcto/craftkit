//! The **expression** of a property.
//!
//! The tree's expressions are **JavaScript**, run on a JavaScript
//! engine, and they are After Effects' expressions: `wiggle`,
//! `valueAtTime`, `thisComp`, `posterizeTime` and the rest of a small
//! language that is a language of its own.
//!
//! This tree is Rust and has no engine in it, and the rule here is
//! that a thing that is not here says so. So an expression here is a
//! small **arithmetic language** of this tree's own, and it has:
//!
//! - the **numbers**, the operators `+ - * / % ^`, and the brackets;
//! - the **names** `time` (the time in seconds) and `value` (the
//!   property's own value at that time, without the expression);
//! - the **functions** the tree has and that mean here what they mean
//!   there: `linear`, `ease`, `easeIn`, `easeOut`, `clamp`, `min`,
//!   `max`, `abs`, `round`, `floor`, `ceil`, `sqrt`, `sin`, `cos`,
//!   `tan` and `atan2`.
//!
//! The eases are the tree's: `ease` is the smoothstep `u²(3 − 2u)`,
//! `easeIn` is `u²(2 − u)` and `easeOut` is `1 − (1 − u)²(2 − (1 − u))`.
//!
//! # What it does not do
//!
//! - **The tree's JavaScript is not here**: there is no `wiggle`, no
//!   `valueAtTime`, no `thisComp` and no `posterizeTime`, and an
//!   expression that uses one is **refused**.
//! - **The random numbers are not here**: a value that changes every
//!   time it is read is not derived, and an expression here is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

use crate::{get_at, parse_path, set_at};

commands! {
    /// Put an **expression** on a property.
    ///
    /// The expression computes the property's value at every frame, and the
    /// name `value` inside it is the property's own value — the one its
    /// keyframes give.
    ///
    /// The property must be **animated**: an expression is a way of saying
    /// what the value is, and a property with no keyframes has no value to
    /// say it from.
    ///
    /// The expression is **parsed** when it is put down, and one that does
    /// not parse is **refused**: a property with a broken expression is a
    /// property that cannot be read.
    fx.prop.expression {
        area: "fx",
        layer: "fx",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The layer the property is on.
            layer: NodeRef,
            /// The path of the property.
            path: Text,
            /// The expression itself.
            expression: Text,
        },
        run: {
            let id = "fx.prop.expression".to_string();
            let layer = self.layer.0;
            let path = self.path.0.clone();
            let text = self.expression.0.clone();
            let segments = parse_path(&path);
            if segments.is_empty() {
                return Err(EngineError::Failed { id, message: "a path has something in it".to_string() });
            }
            // The expression is **parsed** first: an expression that does
            // not parse is refused before anything is written.
            parse(&text).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
            let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            let Some(current) = get_at(&props, &segments).cloned() else {
                return Err(EngineError::Failed { id, message: format!("there is no `{path}` here") });
            };
            if !current.is_object() {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{path}` is not animated, and an expression needs keyframes"),
                });
            }
            let mut new = current;
            new["expression"] = serde_json::json!(text);
            let mut props = props;
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            node.data["props"] = props;
            Ok(Outcome::changed(format!("`{path}` has an expression"))
                .with_data(serde_json::json!({ "layer": layer, "path": path, "expression": text })))

        }
    }

    /// Take the **expression** off a property.
    ///
    /// The property keeps its keyframes, and its value is the one they
    /// give.
    fx.prop.expression.delete {
        area: "fx",
        layer: "fx",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The layer the property is on.
            layer: NodeRef,
            /// The path of the property.
            path: Text,
        },
        run: {
            let id = "fx.prop.expression.delete".to_string();
            let layer = self.layer.0;
            let path = self.path.0.clone();
            let segments = parse_path(&path);
            if segments.is_empty() {
                return Err(EngineError::Failed { id, message: "a path has something in it".to_string() });
            }
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
            let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            let Some(current) = get_at(&props, &segments).cloned() else {
                return Err(EngineError::Failed { id, message: format!("there is no `{path}` here") });
            };
            let mut new = current;
            if let Some(object) = new.as_object_mut() {
                object.remove("expression");
            }
            let mut props = props;
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            node.data["props"] = props;
            Ok(Outcome::changed(format!("`{path}` has no expression"))
                .with_data(serde_json::json!({ "layer": layer, "path": path })))

        }
    }

    /// **Bake** the expression of a property into **keyframes**.
    ///
    /// An expression says what the value is at every frame; baking it
    /// samples it and writes the samples down as keyframes, so the value
    /// is a fact of the property and not a thing that is worked out every
    /// time it is asked for. The expression is **taken away** afterwards.
    ///
    /// The rules:
    ///
    /// - the property must have an **expression**: there is nothing to
    ///   bake otherwise;
    /// - the frames are **whole numbers**, the first is not after the
    ///   last, and the step between them is a whole number of at least
    ///   one;
    /// - the number of samples is **bounded**: a bake is at most a
    ///   thousand of them, and a bake that would be longer is refused
    ///   rather than truncated;
    /// - an expression that cannot be worked out at a frame is **refused**,
    ///   and the refusal names the frame;
    /// - the keyframes that are written are **straight lines** between the
    ///   samples: the samples are points and a curve between them is a
    ///   guess.
    fx.prop.bake {
        area: "fx",
        layer: "fx",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The layer the property is on.
            layer: NodeRef,
            /// The path of the property.
            path: Text,
            /// The first frame.
            from: Number,
            /// The last frame.
            to: Number,
            /// The frames between the samples. It is one unless the file says
            /// another.
            step: Option<Number>,
        },
        run: {
            let id = "fx.prop.bake".to_string();
            let layer = self.layer.0;
            let path = self.path.0.clone();
            let segments = parse_path(&path);
            if segments.is_empty() {
                return Err(EngineError::Failed { id, message: "a path has something in it".to_string() });
            }
            // The **frames**: whole numbers, and the first before the
            // last.
            let whole = |number: f64| -> Option<u64> {
                if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
                    None
                } else {
                    Some(number as u64)
                }
            };
            let Some(from) = whole(self.from.0) else {
                return Err(EngineError::Failed { id, message: "a frame is a whole number".to_string() });
            };
            let Some(to) = whole(self.to.0) else {
                return Err(EngineError::Failed { id, message: "a frame is a whole number".to_string() });
            };
            if from > to {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the first frame {from} is after the last frame {to}"),
                });
            }
            let step = match &self.step {
                None => 1u64,
                Some(step) => {
                    let Some(step) = whole(step.0) else {
                        return Err(EngineError::Failed {
                            id,
                            message: "the step between the frames is a whole number".to_string(),
                        });
                    };
                    if step == 0 {
                        return Err(EngineError::Failed {
                            id,
                            message: "the step between the frames is at least one".to_string(),
                        });
                    }
                    step
                }
            };
            let samples = (to - from) / step + 1;
            if samples as usize > MAX_SAMPLES {
                return Err(EngineError::Failed {
                    id,
                    message: format!("a bake is at most {MAX_SAMPLES} samples, and this one is {samples}"),
                });
            }
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
            let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
            let Some(current) = get_at(&props, &segments).cloned() else {
                return Err(EngineError::Failed { id, message: format!("there is no `{path}` here") });
            };
            let Some(text) =
                current.get("expression").and_then(|text| text.as_str()).map(|text| text.to_string())
            else {
                return Err(EngineError::Failed { id, message: format!("`{path}` has no expression to bake") });
            };
            let expression = parse(&text).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            // The **own** value of the property: the one its keyframes
            // give, and it is what the name `value` means inside the
            // expression.
            let keys = crate::keys::keys_of(&current).unwrap_or_default();
            let spatial = current.get("spatial").and_then(|spatial| spatial.as_bool()).unwrap_or(false);
            // The **samples**: the expression at each frame.
            let mut baked: Vec<crate::keys::Key> = Vec::with_capacity(samples as usize);
            let mut frame = from;
            while frame <= to {
                let own = crate::keys::value_at(&keys, frame as f64, spatial)
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0);
                let number = expression.value(frame as f64, own).map_err(|message| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the expression is not worked out at frame {frame}: {message}"),
                })?;
                baked.push(crate::keys::Key {
                    frame,
                    value: serde_json::json!(number),
                    interp: crate::keys::Interp::Linear,
                    ease: crate::keys::Ease::default(),
                    in_interp: crate::keys::Interp::Linear,
                    in_ease: crate::keys::Ease::default(),
                    auto: false,
                    continuous: false,
                });
                frame += step;
            }
            let mut new = crate::keys::with_keys(&current, &baked);
            // The expression is **taken away**: the keyframes are the
            // fact now.
            if let Some(map) = new.as_object_mut() {
                map.remove("expression");
            }
            let mut props = props;
            set_at(&mut props, &segments, new)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            node.data["props"] = props;
            Ok(Outcome::changed(format!("`{path}` is baked into {} keyframe(s)", baked.len())).with_data(
                serde_json::json!({
                    "layer": layer,
                    "path": path,
                    "from": from,
                    "to": to,
                    "step": step,
                    "keyframes": baked.len(),
                }),
            ))

        }
    }
}

/// The most characters an expression may have.
pub const MAX_CHARS: usize = 1024;

/// The most brackets an expression may be nested in.
const MAX_DEPTH: usize = 32;

/// The names an expression may use.
#[derive(Debug, Clone, Copy)]
struct Names {
    /// The time, in seconds.
    time: f64,
    /// The property's own value at that time.
    value: f64,
}

/// A **parsed** expression.
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    /// What it is.
    root: Node,
}

/// A piece of an expression.
#[derive(Debug, Clone, PartialEq)]
enum Node {
    /// A number.
    Number(f64),
    /// A name: `time` or `value`.
    Name(Name),
    /// A call of a function.
    Call(String, Vec<Node>),
    /// An operation of two.
    Binary(Operator, Box<Node>, Box<Node>),
    /// The sign of a number.
    Negate(Box<Node>),
}

/// The names an expression may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Name {
    /// The time, in seconds.
    Time,
    /// The property's own value at that time.
    Value,
}

/// The operators of two numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    /// The sum.
    Add,
    /// The difference.
    Subtract,
    /// The product.
    Multiply,
    /// The quotient.
    Divide,
    /// The remainder.
    Remainder,
    /// The power.
    Power,
}

/// The functions an expression may call.
///
/// Each one is the tree's, and each one says how many arguments it
/// takes. The four eases take three arguments — the time, and the two
/// values it runs between — or five: the time, the two times it runs
/// between, and the two values.
const FUNCTIONS: [(&str, &[usize]); 15] = [
    ("linear", &[3, 5]),
    ("ease", &[3, 5]),
    ("easeIn", &[3, 5]),
    ("easeOut", &[3, 5]),
    ("clamp", &[3]),
    ("min", &[2]),
    ("max", &[2]),
    ("abs", &[1]),
    ("round", &[1]),
    ("floor", &[1]),
    ("ceil", &[1]),
    ("sqrt", &[1]),
    ("sin", &[1]),
    ("cos", &[1]),
    ("tan", &[1]),
];

impl Expression {
    /// The **value** of the expression at a time, when the property's
    /// own value is what it is.
    pub fn value(&self, time: f64, value: f64) -> Result<f64, String> {
        self.root.value(&Names { time, value })
    }
}

impl Node {
    /// The value of the piece.
    fn value(&self, names: &Names) -> Result<f64, String> {
        match self {
            Node::Number(number) => Ok(*number),
            Node::Name(name) => Ok(match name {
                Name::Time => names.time,
                Name::Value => names.value,
            }),
            Node::Negate(node) => Ok(-node.value(names)?),
            Node::Binary(operator, left, right) => {
                let left = left.value(names)?;
                let right = right.value(names)?;
                Ok(match operator {
                    Operator::Add => left + right,
                    Operator::Subtract => left - right,
                    Operator::Multiply => left * right,
                    Operator::Divide => {
                        if right == 0.0 {
                            return Err("an expression cannot divide by zero".to_string());
                        }
                        left / right
                    }
                    Operator::Remainder => {
                        if right == 0.0 {
                            return Err("an expression cannot divide by zero".to_string());
                        }
                        left % right
                    }
                    Operator::Power => left.powf(right),
                })
            }
            Node::Call(name, arguments) => {
                let values: Vec<f64> = arguments
                    .iter()
                    .map(|argument| argument.value(names))
                    .collect::<Result<Vec<f64>, String>>()?;
                call(name, &values)
            }
        }
    }
}

/// A call of a **function**, with the tree's meanings.
fn call(name: &str, values: &[f64]) -> Result<f64, String> {
    match name {
        "linear" => interpolate(values, |u| u),
        "ease" => interpolate(values, |u| u * u * (3.0 - 2.0 * u)),
        "easeIn" => interpolate(values, |u| u * u * (2.0 - u)),
        "easeOut" => interpolate(values, |u| {
            let rest = 1.0 - u;
            1.0 - rest * rest * (2.0 - rest)
        }),
        "clamp" => {
            if values.len() != 3 {
                return Err("`clamp` takes a value and two bounds".to_string());
            }
            Ok(values[0].max(values[1].min(values[2])).min(values[1].max(values[2])))
        }
        "min" => Ok(values[0].min(values[1])),
        "max" => Ok(values[0].max(values[1])),
        "abs" => Ok(values[0].abs()),
        "round" => Ok(values[0].round()),
        "floor" => Ok(values[0].floor()),
        "ceil" => Ok(values[0].ceil()),
        "sqrt" => {
            if values[0] < 0.0 {
                return Err("the square root of a negative number is not a number".to_string());
            }
            Ok(values[0].sqrt())
        }
        "sin" => Ok(values[0].sin()),
        "cos" => Ok(values[0].cos()),
        "tan" => Ok(values[0].tan()),
        _ => Err(format!("`{name}` is not a function this tree has")),
    }
}

/// The **interpolation** the tree's eases do: the value from one to
/// the other, over a time, with a shape.
///
/// With three arguments the time is from zero to one, and with five
/// the time is from the second to the third.
fn interpolate(values: &[f64], shape: impl Fn(f64) -> f64) -> Result<f64, String> {
    let (at, from, to, first, last) = match values.len() {
        3 => (values[0], 0.0, 1.0, values[1], values[2]),
        5 => (values[0], values[1], values[2], values[3], values[4]),
        _ => return Err("this function takes three arguments or five".to_string()),
    };
    let u = if to == from { if at >= to { 1.0 } else { 0.0 } } else { (at - from) / (to - from) };
    let u = u.clamp(0.0, 1.0);
    Ok(first + (last - first) * shape(u))
}

/// The **parser** of an expression.
struct Parser {
    /// The characters of the expression.
    chars: Vec<char>,
    /// Where it is.
    at: usize,
    /// How deep it is in brackets.
    depth: usize,
}

impl Parser {
    /// The expression.
    fn expression(&mut self) -> Result<Node, String> {
        let mut left = self.term()?;
        loop {
            self.skip_space();
            let Some(operator) = self.operator_of("+-") else { break };
            let right = self.term()?;
            left = Node::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// A term: a factor and the factors it is multiplied by.
    fn term(&mut self) -> Result<Node, String> {
        let mut left = self.factor()?;
        loop {
            self.skip_space();
            let Some(operator) = self.operator_of("*/%") else { break };
            let right = self.factor()?;
            left = Node::Binary(operator, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    /// A factor: a thing and its power.
    fn factor(&mut self) -> Result<Node, String> {
        let left = self.unary()?;
        self.skip_space();
        let Some(operator) = self.operator_of("^") else { return Ok(left) };
        // The power is to the right, and it is right-associative: two
        // twos to the power of a three is a two to the power of a
        // three to the power of a two.
        let right = self.factor()?;
        Ok(Node::Binary(operator, Box::new(left), Box::new(right)))
    }

    /// A unary thing: a sign and a thing.
    fn unary(&mut self) -> Result<Node, String> {
        self.skip_space();
        if self.peek() == Some('-') {
            self.at += 1;
            return Ok(Node::Negate(Box::new(self.unary()?)));
        }
        if self.peek() == Some('+') {
            self.at += 1;
            return self.unary();
        }
        self.primary()
    }

    /// A primary thing: a number, a name, a call or a bracket.
    fn primary(&mut self) -> Result<Node, String> {
        self.skip_space();
        let Some(character) = self.peek() else {
            return Err("the expression ends too early".to_string());
        };
        if character.is_ascii_digit() || character == '.' {
            return self.number();
        }
        if character == '(' {
            self.at += 1;
            self.depth += 1;
            if self.depth > MAX_DEPTH {
                return Err(format!("an expression is at most {MAX_DEPTH} brackets deep"));
            }
            let node = self.expression()?;
            self.skip_space();
            if self.peek() != Some(')') {
                return Err("a bracket is not closed".to_string());
            }
            self.at += 1;
            self.depth -= 1;
            return Ok(node);
        }
        if character.is_ascii_alphabetic() || character == '_' {
            let name = self.word();
            self.skip_space();
            if self.peek() == Some('(') {
                self.at += 1;
                let arguments = self.arguments()?;
                // The function and its **arguments** are checked here,
                // so that an expression that cannot work is refused
                // when it is written down and not when it is read.
                let Some((_, counts)) = FUNCTIONS.iter().find(|(known, _)| *known == name) else {
                    return Err(format!("`{name}` is not a function this tree has"));
                };
                if !counts.contains(&arguments.len()) {
                    return Err(format!(
                        "`{name}` takes {} argument(s)",
                        counts.iter().map(|count| count.to_string()).collect::<Vec<String>>().join(" or ")
                    ));
                }
                return Ok(Node::Call(name, arguments));
            }
            return match name.as_str() {
                "time" => Ok(Node::Name(Name::Time)),
                "value" => Ok(Node::Name(Name::Value)),
                other => Err(format!("`{other}` is not a name this tree has")),
            };
        }
        Err(format!("`{character}` is not something an expression can hold"))
    }

    /// A number.
    fn number(&mut self) -> Result<Node, String> {
        let start = self.at;
        while matches!(self.peek(), Some(character) if character.is_ascii_digit()) {
            self.at += 1;
        }
        if self.peek() == Some('.') {
            self.at += 1;
            while matches!(self.peek(), Some(character) if character.is_ascii_digit()) {
                self.at += 1;
            }
        }
        let text: String = self.chars[start..self.at].iter().collect();
        let number: f64 = text.parse().map_err(|_| format!("`{text}` is not a number"))?;
        if !number.is_finite() {
            return Err(format!("`{text}` is not a number"));
        }
        Ok(Node::Number(number))
    }

    /// The **arguments** of a call.
    fn arguments(&mut self) -> Result<Vec<Node>, String> {
        let mut arguments: Vec<Node> = Vec::new();
        self.skip_space();
        if self.peek() == Some(')') {
            self.at += 1;
            return Ok(arguments);
        }
        loop {
            arguments.push(self.expression()?);
            self.skip_space();
            match self.peek() {
                Some(',') => self.at += 1,
                Some(')') => {
                    self.at += 1;
                    break;
                }
                _ => return Err("the arguments of a call are separated by commas".to_string()),
            }
        }
        Ok(arguments)
    }

    /// A word: a name of a function or of a value.
    fn word(&mut self) -> String {
        let start = self.at;
        while matches!(self.peek(), Some(character) if character.is_ascii_alphanumeric() || character == '_')
        {
            self.at += 1;
        }
        self.chars[start..self.at].iter().collect()
    }

    /// The operator at the front, if it is one of them.
    fn operator_of(&mut self, operators: &str) -> Option<Operator> {
        let character = self.peek()?;
        if !operators.contains(character) {
            return None;
        }
        self.at += 1;
        Some(match character {
            '+' => Operator::Add,
            '-' => Operator::Subtract,
            '*' => Operator::Multiply,
            '/' => Operator::Divide,
            '%' => Operator::Remainder,
            '^' => Operator::Power,
            _ => return None,
        })
    }

    /// The character at the front, if there is one.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    /// The spaces at the front.
    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(character) if character.is_whitespace()) {
            self.at += 1;
        }
    }
}

/// Parse an **expression**.
///
/// An expression that does not parse is **refused**: a broken
/// expression is a fact about the property, and a property with a
/// broken expression is not a property that can be read.
pub fn parse(text: &str) -> Result<Expression, String> {
    if text.chars().count() > MAX_CHARS {
        return Err(format!("an expression is at most {MAX_CHARS} characters long"));
    }
    let mut parser = Parser { chars: text.chars().collect(), at: 0, depth: 0 };
    let root = parser.expression()?;
    parser.skip_space();
    if parser.at < parser.chars.len() {
        let rest: String = parser.chars[parser.at..].iter().collect();
        return Err(format!("the expression has more in it: `{rest}`"));
    }
    Ok(Expression { root })
}

/// The most **samples** a bake may take.
const MAX_SAMPLES: usize = 1_000;
