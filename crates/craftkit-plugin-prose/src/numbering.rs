//! The **numbering** of a block.
//!
//! A **list** says what its items are numbered like — `1.`, `a)`, `i.`
//! or a bullet — and a block is in a list at a **level**.
//!
//! The **label** — the number shown beside the block — is **derived**:
//! it is the count of the blocks of the list before it, and the format
//! of the level. So a paragraph that is moved is renumbered, and
//! nothing is written down that could be wrong.
//!
//! The kinds and the levels are the **tree's**: a list kind says what
//! each of its nine levels looks like, and the levels of the kinds that
//! cycle — `1. a. i.` — cycle every three.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Number, Outcome, Relation, Session, Text};
use std::collections::BTreeMap;

/// The kinds of list the tree has.
pub const KINDS: [&str; 9] = [
    "bullet",
    "numbered",
    "numberedParen",
    "outline",
    "upperLetter",
    "lowerLetter",
    "lowerLetterDot",
    "lowerRoman",
    "legal",
];

/// The most levels a list has.
pub const MAX_LEVELS: usize = 9;

/// The **relation** that says a block is in a list.
pub const NUMBERED_BY: &str = "prose.numbered_by";

/// The **format** a level counts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// `1`, `2`, `3`.
    Decimal,
    /// `I`, `II`, `III`.
    UpperRoman,
    /// `i`, `ii`, `iii`.
    LowerRoman,
    /// `A`, `B`, `C`.
    UpperLetter,
    /// `a`, `b`, `c`.
    LowerLetter,
    /// A bullet: a character and not a number.
    Bullet,
}

/// A **level** of a list: the format it counts in and the text it
/// shows.
#[derive(Debug, Clone)]
pub struct Level {
    /// The format the level counts in.
    pub format: Format,
    /// The **text** of the level: the counters are written where a
    /// `%` and a digit are.
    pub text: String,
}

/// The kind a name says, if it says one this tree has.
///
/// A **single character** is a bullet of its own, and that is the
/// tree's own rule.
pub fn kind_of(name: &str) -> Option<String> {
    let name = name.trim();
    if let Some(kind) = KINDS.iter().find(|kind| kind.eq_ignore_ascii_case(name)) {
        return Some((*kind).to_string());
    }
    let mut characters = name.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    Some(character.to_string())
}

/// The **levels** of a list of a kind: nine of them.
///
/// The tree's own rule: the kinds that cycle cycle every three
/// levels, the outline takes its format from the level, and a legal
/// list shows every level in its text.
pub fn levels_for(kind: &str) -> Vec<Level> {
    let bullet = |character: char| Level { format: Format::Bullet, text: character.to_string() };
    let plain = |format: Format, text: String| Level { format, text };
    let cycle = |level: usize, a: Format, b: Format, c: Format| match level % 3 {
        0 => a,
        1 => b,
        _ => c,
    };
    (0..MAX_LEVELS)
        .map(|level| {
            let n = level + 1;
            match kind {
                "bullet" => bullet(default_bullet(level)),
                "numbered" => plain(
                    cycle(level, Format::Decimal, Format::LowerLetter, Format::LowerRoman),
                    format!("%{n}."),
                ),
                "numberedParen" => plain(
                    cycle(level, Format::Decimal, Format::LowerLetter, Format::LowerRoman),
                    format!("%{n})"),
                ),
                "upperLetter" => plain(
                    cycle(level, Format::UpperLetter, Format::LowerLetter, Format::LowerRoman),
                    format!("%{n}."),
                ),
                "lowerLetter" => plain(
                    cycle(level, Format::LowerLetter, Format::LowerRoman, Format::Decimal),
                    format!("%{n})"),
                ),
                "lowerLetterDot" => plain(
                    cycle(level, Format::LowerLetter, Format::LowerRoman, Format::Decimal),
                    format!("%{n}."),
                ),
                "lowerRoman" => plain(
                    cycle(level, Format::LowerRoman, Format::LowerLetter, Format::Decimal),
                    format!("%{n}."),
                ),
                "outline" => {
                    let format = match level {
                        0 => Format::UpperRoman,
                        1 => Format::UpperLetter,
                        2 => Format::Decimal,
                        3 => Format::LowerLetter,
                        _ => Format::LowerRoman,
                    };
                    plain(format, format!("%{n}."))
                }
                "legal" => {
                    let text: String = (1..=n).map(|level| format!("%{level}.")).collect();
                    plain(Format::Decimal, text)
                }
                // A **bullet character** of its own: the first level is
                // the character and the rest are the tree's bullets.
                character => {
                    if level == 0 {
                        bullet(character.chars().next().unwrap_or('•'))
                    } else {
                        bullet(default_bullet(level))
                    }
                }
            }
        })
        .collect()
}

/// The **bullet** a level of a plain bulleted list shows: `•`, `○`,
/// `▪`, repeating.
fn default_bullet(level: usize) -> char {
    ['•', '○', '▪'][level % 3]
}

/// A number written in **letters**: `a` to `z`, then `aa`.
///
/// The tree's own rule: the letters repeat, and they do not count in
/// twenty-sixes.
fn letters(number: u32) -> String {
    if number == 0 {
        return String::new();
    }
    let number = number.min(780);
    let times = (number - 1) / 26 + 1;
    let character = (b'a' + ((number - 1) % 26) as u8) as char;
    std::iter::repeat_n(character, times as usize).collect()
}

/// A number written in **roman** numerals.
fn roman(mut number: u32) -> String {
    const TABLE: [(u32, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    number = number.min(50_000);
    for (value, numeral) in TABLE {
        while number >= value {
            out.push_str(numeral);
            number -= value;
        }
    }
    out
}

/// A number written in a **format**.
fn written(format: Format, number: u32) -> String {
    match format {
        Format::Decimal => number.to_string(),
        Format::UpperRoman => roman(number).to_uppercase(),
        Format::LowerRoman => roman(number),
        Format::UpperLetter => letters(number).to_uppercase(),
        Format::LowerLetter => letters(number),
        Format::Bullet => String::new(),
    }
}

/// The **counters** of the lists of a document: how many items each
/// level has seen.
///
/// They are **derived**: a block that is moved is counted where it is
/// now, and a level **restarts** when a shallower level is used again.
///
/// The counting stops **at** a block when one is named, because the
/// number of a block is the count up to that block and not the count
/// of the whole document.
fn counters(document: &Document, upto: Option<u64>) -> BTreeMap<u64, [u32; MAX_LEVELS]> {
    let mut counters: BTreeMap<u64, [u32; MAX_LEVELS]> = BTreeMap::new();
    'counting: for node in document.walk().filter(|node| node.kind == "prose.document") {
        for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
            if let Some((list, level)) = numbering_of(document, block.id) {
                let entry = counters.entry(list).or_insert([0; MAX_LEVELS]);
                if let Some(counter) = entry.get_mut(level) {
                    *counter = counter.saturating_add(1);
                }
                // A deeper level **restarts** when a shallower level is
                // used again.
                for deeper in entry.iter_mut().skip(level + 1) {
                    *deeper = 0;
                }
            }
            if Some(block.id) == upto {
                break 'counting;
            }
        }
    }
    counters
}

/// The **list** and the **level** a block is in, if it is in one.
fn numbering_of(document: &Document, block: u64) -> Option<(u64, usize)> {
    let relation =
        document.relations.iter().find(|relation| relation.kind == NUMBERED_BY && relation.from == block)?;
    let list = relation.payload.get("list").and_then(|list| list.as_u64())?;
    let level = relation.payload.get("level").and_then(|level| level.as_u64()).unwrap_or(0);
    let level = (level as usize).min(MAX_LEVELS - 1);
    Some((list, level))
}

/// The **label** of a block: the number shown beside it.
///
/// The label is **derived**: it is the counters of the list and the
/// text of the level.
/// The **number** a block shows, when it is in a list that counts.
///
/// The tree's own rule: a list that writes a bullet has no number in
/// a table of contents, and neither has a block that is in no list.
pub fn number_of(document: &Document, block: u64) -> Option<String> {
    let (list, level) = numbering_of(document, block)?;
    let kind = document.node(list)?.data.get("kind").and_then(|kind| kind.as_str())?.to_string();
    let levels = levels_for(&kind);
    let level = levels.get(level)?;
    if level.format == Format::Bullet {
        return None;
    }
    label_of(document, block)
}

pub fn label_of(document: &Document, block: u64) -> Option<String> {
    let (list, level) = numbering_of(document, block)?;
    let kind = document.node(list)?.data.get("kind").and_then(|kind| kind.as_str())?.to_string();
    let levels = levels_for(&kind);
    let level = levels.get(level)?.clone();
    if level.format == Format::Bullet {
        return Some(level.text);
    }
    let counters = counters(document, Some(block));
    let counters = counters.get(&list).copied().unwrap_or([0; MAX_LEVELS]);
    let mut label = String::new();
    let mut characters = level.text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '%'
            && let Some(digit) = characters.peek().and_then(|digit| digit.to_digit(10))
        {
            characters.next();
            let index = (digit as usize).saturating_sub(1).min(MAX_LEVELS - 1);
            let counted = counters.get(index).copied().unwrap_or(0);
            // A level that has **not started** shows the number it
            // starts at.
            let value = if counted == 0 { 1 } else { counted };
            let format = levels.get(index).map(|level| level.format).unwrap_or(Format::Decimal);
            label.push_str(&written(format, value));
        } else {
            label.push(character);
        }
    }
    Some(label)
}

/// Put a **list** in a document.
///
/// A list is few — a paragraph may be in a list and a list may have a
/// hundred paragraphs — so a list is a **node**, and a block names it
/// with a **relation**, the way a block names a style.
#[derive(Command)]
#[command(id = "prose.list.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddList {
    /// The document it goes in.
    pub document: NodeRef,
    /// The kind of list: `bullet`, `numbered`, `numberedParen`,
    /// `outline`, `upperLetter`, `lowerLetter`, `lowerLetterDot`,
    /// `lowerRoman` or `legal` — or a single character, which is a
    /// bullet of its own.
    pub kind: Text,
}

impl AddList {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.list.add".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let wanted = self.kind.0.trim().to_string();
        let Some(kind) = kind_of(&wanted) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no list kind `{wanted}`: the kinds are {}, or a single character",
                    KINDS.join(", ")
                ),
            });
        };
        let list = session.document_mut()?.add_node_with_under(
            Some(document),
            kind.clone(),
            "prose.list",
            None,
            json!({ "kind": kind.clone() }),
        );
        Ok(Outcome::changed(format!("the document `{document}` has a list of {kind}"))
            .with_data(json!({ "id": list, "document": document, "kind": kind })))
    }
}

/// Put a **block** in a list, at a level.
///
/// The block's own list is replaced: a block is in one list.
#[derive(Command)]
#[command(id = "prose.block.numbering", area = "prose", layer = "prose", icon = "list", mutates, undoable)]
pub struct SetNumbering {
    /// The block.
    pub block: NodeRef,
    /// The list.
    pub list: NodeRef,
    /// The level, counting from zero. Without it the block is at the
    /// first level.
    pub level: Option<Number>,
}

impl SetNumbering {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.numbering".to_string();
        let block = self.block.0;
        let list = self.list.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if !node.kind.starts_with("prose.block.") {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        let Some(node) = session.document().node(list) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no list `{list}`"),
            });
        };
        if node.kind != "prose.list" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{list}` is a `{}` and not a list", node.kind),
            });
        }
        let level = match self.level {
            Some(level) => {
                let level = level.0;
                if !level.is_finite() || level < 0.0 || level.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a level is counted from zero: {level} is not one"),
                    });
                }
                level as usize
            }
            None => 0,
        };
        if level >= MAX_LEVELS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a list has {MAX_LEVELS} levels, and {level} is not one of them"),
            });
        }
        // A block is in **one** list: the old one goes.
        session
            .document_mut()?
            .remove_relations(|relation| relation.kind == NUMBERED_BY && relation.from == block);
        let relation =
            Relation::with_payload(block, list, NUMBERED_BY, json!({ "list": list, "level": level }));
        session.document_mut()?.add_relation(relation);
        Ok(Outcome::changed(format!("the block `{block}` is in the list `{list}` at the level {level}"))
            .with_data(json!({ "block": block, "list": list, "level": level })))
    }
}

/// **Read** the number of a block.
///
/// The number is **derived**: it is the count of the blocks of the list
/// before it and the format of the level. A block that is in no list
/// has no number, and that is said as nothing.
#[derive(Command)]
#[command(id = "prose.block.number", area = "prose", layer = "prose", icon = "list")]
pub struct ReadNumber {
    /// The block.
    pub block: NodeRef,
}

impl ReadNumber {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.number".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed { id, message: format!("there is no block `{block}`") });
        };
        if !node.kind.starts_with("prose.block.") {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        let document = session.document();
        let numbering = numbering_of(document, block);
        let label = label_of(document, block).map(Json::String).unwrap_or(Json::Null);
        let list = numbering.map(|(list, _)| list).map(Json::from).unwrap_or(Json::Null);
        let level = numbering.map(|(_, level)| level).map(Json::from).unwrap_or(Json::Null);
        Ok(Outcome::new(format!("the block `{block}`")).with_data(json!({
            "block": block,
            "list": list,
            "level": level,
            "label": label,
        })))
    }
}
