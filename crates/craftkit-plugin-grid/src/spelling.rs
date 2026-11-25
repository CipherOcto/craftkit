//! The **spelling** of a sheet.
//!
//! A sheet may hold words that are not words. The **check** finds them:
//! it reads the text of the cells, and every word of it that the word
//! list does not hold — and that is not an **inflection** of a word it
//! holds — is an **issue**, with the **suggestions** for it.
//!
//! # The word list
//!
//! The list is the **system's**: the word list of the machine, which is
//! not in this tree and is not in any file of it. A caller may name a
//! file of its own instead — one word a line — and then that is the
//! list.
//!
//! # The words the user taught
//!
//! A word the user says is a word is kept in the **workbook**: the
//! document carries the payload, and the payload of a workbook that
//! has been taught a word is that word. So the words travel with the
//! workbook and not with the machine.
//!
//! # What the port found
//!
//! - **The reference keeps the user's words in the preferences of the
//!   program**, and this tree has no preferences: it has a document. So
//!   the words are kept in the workbook, and they travel with it.
//! - **The reference's list is the system's and the reference refuses
//!   when there is none.** So does this — and a caller may name a file
//!   of its own, which is how the tree's own tests may be written on a
//!   machine that has no word list.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{CellRef, Value};

/// The **word lists** the system's word list may be in, in the order
/// they are tried.
const SYSTEM_LISTS: [&str; 3] =
    ["/usr/share/dict/words", "/usr/share/dict/american-english", "/usr/share/dict/british-english"];

/// The **most** lines of a word list that are read.
pub const MAX_SPELLING_LINES: usize = 2_000_000;

/// The **most** bytes of a word list that are read.
const MAX_SPELLING_BYTES: u64 = 64 * 1024 * 1024;

/// The **most** issues one check reports.
pub const MAX_ISSUES: usize = 500;

/// The **most** characters a word may have for a suggestion to be made.
///
/// The candidates of a word grow with the square of its length, so a
/// long word gets no suggestion rather than an unbounded work.
///
/// The suggestions of a word that is too long are **not made**, and the
/// issue says so: a word with no suggestions and a word with no
/// alternatives are not the same thing.
pub const MAX_SPELLING_WORD: usize = 64;

/// The **most** suggestions one word gets.
///
/// The suggestions beyond this one are not offered, and the issue says
/// so.
pub const MAX_SPELLING_SUGGESTIONS: usize = 6;

/// The most **related words** the thesaurus answers with.
pub const MAX_RELATED: usize = 20;

/// The **suffixes** a word may have and still be a word: the
/// inflections the list does not spell out.
const INFLECTIONS: [&str; 8] = ["s", "es", "ed", "ing", "ly", "er", "ers", "'s"];

/// The **words** of a word list: the file's lines, lowercased, and
/// whether the list was **cut** at the tree's bound.
///
/// A list that is cut is a list the reader does not have whole, and a
/// word the list holds beyond the cut would be called misspelled: the
/// answer says so, because the answer must not lie about a word.
fn words_of(path: &str) -> Option<(std::collections::HashSet<String>, bool)> {
    let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(path), MAX_SPELLING_BYTES).ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    let mut words = std::collections::HashSet::new();
    let mut truncated = false;
    for line in lines.by_ref().take(MAX_SPELLING_LINES) {
        let line = line.trim().to_lowercase();
        if !line.is_empty() {
            words.insert(line);
        }
    }
    if lines.next().is_some() {
        truncated = true;
    }
    Some((words, truncated))
}

/// The **word list** to check against: the file that was named, or the
/// system's.
fn a_dictionary(path: Option<&str>) -> Option<(std::collections::HashSet<String>, bool)> {
    match path {
        Some(path) => words_of(path),
        None => {
            for path in SYSTEM_LISTS {
                if let Some(words) = words_of(path) {
                    return Some(words);
                }
            }
            None
        }
    }
}

/// Whether a **word** is a word.
///
/// The list holds it, or the user taught it, or it is an **inflection**
/// of a word the list holds: a plural, a past, a gerund, an adverb, a
/// comparative — and the plural of a word that ends in `y`.
fn is_known(words: &std::collections::HashSet<String>, taught: &[String], word: &str) -> bool {
    let lower = word.to_lowercase();
    if words.contains(&lower) || taught.iter().any(|word| word.eq_ignore_ascii_case(word)) {
        return true;
    }
    for suffix in INFLECTIONS {
        if let Some(stem) = lower.strip_suffix(suffix)
            && stem.len() > 2
            && (words.contains(stem) || words.contains(&format!("{stem}e")))
        {
            return true;
        }
    }
    if let Some(stem) = lower.strip_suffix("ies")
        && words.contains(&format!("{stem}y"))
    {
        return true;
    }
    false
}

/// The **suggestions** for a word: the words of the list that are one
/// edit away from it — a letter added, a letter taken away, a letter
/// changed, two letters swapped — and what was left out.
///
/// The list is cut at `MAX_SPELLING_SUGGESTIONS`, and a word that is
/// longer than `MAX_SPELLING_WORD` gets none at all: both are said, and
/// an answer that says nothing about them is an answer that leaves the
/// reader to guess.
fn suggestions(words: &std::collections::HashSet<String>, word: &str) -> Suggestions {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
    let characters: Vec<char> = word.to_lowercase().chars().collect();
    if characters.len() > MAX_SPELLING_WORD {
        // The word is too long for the candidates to be made: the
        // suggestions are **omitted**, and the issue says so.
        return Suggestions { words: Vec::new(), truncated: false, omitted: true };
    }
    let mut candidates: Vec<String> = Vec::new();
    for index in 0..=characters.len() {
        if index < characters.len() {
            let mut taken = characters.clone();
            taken.remove(index);
            candidates.push(taken.iter().collect());
            if index + 1 < characters.len() {
                let mut swapped = characters.clone();
                swapped.swap(index, index + 1);
                candidates.push(swapped.iter().collect());
            }
            for letter in LETTERS.chars() {
                let mut changed = characters.clone();
                changed[index] = letter;
                candidates.push(changed.iter().collect());
            }
        }
        for letter in LETTERS.chars() {
            let mut added = characters.clone();
            added.insert(index, letter);
            candidates.push(added.iter().collect());
        }
    }
    let mut found: Vec<String> =
        candidates.into_iter().filter(|candidate| words.contains(candidate)).collect();
    found.sort();
    found.dedup();
    // The list is **cut** at the bound, and the cut is said.
    let truncated = found.len() > MAX_SPELLING_SUGGESTIONS;
    if truncated {
        found.truncate(MAX_SPELLING_SUGGESTIONS);
    }
    // The capitalisation of the word that was written.
    let capital = word.chars().next().is_some_and(char::is_uppercase);
    let words = found
        .into_iter()
        .map(|suggestion| {
            if capital {
                let mut letters = suggestion.chars();
                letters
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + letters.as_str())
                    .unwrap_or(suggestion)
            } else {
                suggestion
            }
        })
        .collect();
    Suggestions { words, truncated, omitted: false }
}

/// The **suggestions** of a word, and what was left out of them.
#[derive(Debug, Clone, Default, PartialEq)]
struct Suggestions {
    /// The words that are one edit away from the word.
    words: Vec<String>,
    /// Whether the list was cut at `MAX_SPELLING_SUGGESTIONS`.
    truncated: bool,
    /// Whether the suggestions were not made at all, because the word is
    /// longer than `MAX_SPELLING_WORD`.
    omitted: bool,
}

/// The **words** of a text: the runs of letters and apostrophes.
fn words_of_text(text: &str) -> Vec<&str> {
    text.split(|character: char| !(character.is_alphabetic() || character == '\''))
        .map(|word| word.trim_matches('\''))
        .filter(|word| word.chars().count() >= 2 && !word.chars().all(char::is_uppercase) && word.is_ascii())
        .collect()
}

/// The **words** the user taught the workbook.
fn taught_of(node: &craftkit_sdk::Node) -> Vec<String> {
    node.data
        .get("dictionary")
        .and_then(|words| words.as_array())
        .map(|words| words.iter().filter_map(|word| word.as_str()).map(|word| word.to_string()).collect())
        .unwrap_or_default()
}

/// The **workbook** a sheet is in, and the words it was taught.
fn taught_of_sheet(session: &Session, sheet: u64) -> Vec<String> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == sheet))
        .map(taught_of)
        .unwrap_or_default()
}

commands! {
    /// **Check** the spelling of the text of a sheet.
    ///
    /// The answer says the **issues**: the words that the word list does
    /// not hold, with the cells they are in and the suggestions for
    /// them.
    ///
    /// The word list is the **system's**, unless a file of one word a
    /// line is named.
    grid.spelling.check {
        area: "grid",
        layer: "grid",
        icon: "search",
        params: {
            /// The sheet to check.
            sheet: NodeRef,
            /// The range to check. Without it, the range the sheet uses.
            range: Option<Text>,
            /// The **word list**: a file of one word a line. Without it,
            /// the system's word list is used.
            dictionary: Option<Text>,
        },
        run: {
            let id = "grid.spelling.check".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            // The **word list**: the one that was named, or the system's.
            let path = self.dictionary.as_ref().map(|path| path.0.trim().to_string());
            let Some((words, dictionary_truncated)) = a_dictionary(path.as_deref()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: match path {
                        Some(path) => format!("the word list `{path}` could not be read"),
                        None => "there is no word list on this machine".to_string(),
                    },
                });
            };
            let taught = taught_of_sheet(session, sheet);
            // The **range** of the check.
            let area = match self.range.as_ref() {
                Some(range) => crate::fill::area_at(&range.0, &id)?,
                None => match crate::reading::used_range(&crate::cells_of(&node)) {
                    Some(area) => area,
                    None => {
                        return Ok(Outcome::new("the sheet holds no text to check").with_data(json!({
                            "sheet": sheet,
                            "issues": Vec::<craftkit_sdk::serde_json::Value>::new(),
                            "done": true,
                            "truncated": false,
                            "dictionary_truncated": dictionary_truncated,
                        })));
                    }
                },
            };
            let cells = crate::cells_of(&node);
            let mut issues: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
            // The check stops at the tree's bound: the sheet may hold
            // more issues than the answer lists, and the answer says so.
            // The flag is set when an issue is found that is **not**
            // listed, so a sheet with exactly the bound's number of
            // issues is not called cut.
            let mut truncated = false;
            // The suggestions of the issues are cut at their own bound,
            // and the answer says so: a reader must not take six
            // suggestions for the whole of the alternatives.
            let mut suggestions_truncated = false;
            let mut suggestions_omitted = false;
            'scan: for row in area.start.row..=area.end.row {
                for column in area.start.col..=area.end.col {
                    let cell = CellRef { col: column, row };
                    let Some(entry) = cells.get(cell) else { continue };
                    if entry.formula.is_some() {
                        continue;
                    }
                    let Some(Value::Text(text)) = entry.value.clone() else { continue };
                    for word in words_of_text(&text) {
                        if is_known(&words, &taught, word) {
                            continue;
                        }
                        if issues.len() >= MAX_ISSUES {
                            truncated = true;
                            break 'scan;
                        }
                        let found = suggestions(&words, word);
                        suggestions_truncated = suggestions_truncated || found.truncated;
                        suggestions_omitted = suggestions_omitted || found.omitted;
                        issues.push(json!({
                            "cell": cell.to_text(),
                            "word": word,
                            "suggestions": found.words,
                            "suggestions_truncated": found.truncated,
                            "suggestions_omitted": found.omitted,
                        }));
                    }
                }
            }
            let done = issues.is_empty();
            Ok(Outcome::new(format!("{} issue(s)", issues.len())).with_data(json!({
                "sheet": sheet,
                "issues": issues,
                "done": done,
                "truncated": truncated,
                "dictionary_truncated": dictionary_truncated,
                "suggestions_truncated": suggestions_truncated,
                "suggestions_omitted": suggestions_omitted,
            })))

        }
    }

    /// **Teach** a workbook a word.
    ///
    /// The word is kept in the workbook, and the check reads it: a word
    /// the user taught is a word.
    grid.spelling.add {
        area: "grid",
        layer: "grid",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The workbook to teach.
            workbook: NodeRef,
            /// The word.
            word: Text,
        },
        run: {
            let id = "grid.spelling.add".to_string();
            let workbook = self.workbook.0;
            let word = self.word.0.trim().to_string();
            if word.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a word is not empty".to_string(),
                });
            }
            let Some(node) = session.document().node(workbook).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no workbook `{workbook}`"),
                });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{workbook}` is not a workbook"),
                });
            }
            let mut taught = taught_of(&node);
            if taught.iter().any(|other| other.eq_ignore_ascii_case(&word)) {
                return Ok(Outcome::new(format!("the workbook already knows `{word}`")).with_data(json!({
                    "workbook": workbook,
                    "word": word,
                    "words": taught.len(),
                })));
            }
            taught.push(word.clone());
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                node.data["dictionary"] = json!(taught);
            }
            Ok(Outcome::changed(format!("the workbook learns `{word}`")).with_data(json!({
                "workbook": workbook,
                "word": word,
                "words": taught.len(),
            })))

        }
    }

    /// **Change** a misspelled word in the text of a cell.
    ///
    /// The word is the one the **check** found, and the correction is the
    /// one the caller gives: the **first** time the word is in the text
    /// of the cell is the time it is changed, and the cell keeps its
    /// **format**.
    ///
    /// A cell that holds a **formula** is not changed: the check does
    /// not look at formulas, and a formula is not a misspelling.
    grid.spelling.change {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet the cell is in.
            sheet: NodeRef,
            /// The cell whose text is changed.
            cell: Text,
            /// The word to change.
            word: Text,
            /// What to write in its place.
            to: Text,
        },
        run: {
            let id = "grid.spelling.change".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let word = self.word.0.clone();
            if word.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a word is not empty".to_string(),
                });
            }
            let to = self.to.0.clone();
            let Some(node) = session.document().node(sheet).cloned() else {
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
            let cells = crate::cells_of(&node);
            let Some(entry) = cells.get(cell) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the cell {} holds nothing", cell.to_text()),
                });
            };
            if entry.formula.is_some() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the cell {} holds a formula, and a formula is not a misspelling",
                        cell.to_text()
                    ),
                });
            }
            let Some(Value::Text(text)) = entry.value.clone() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the cell {} holds no text", cell.to_text()),
                });
            };
            // The **first** time the word is in the text is the time it
            // is changed: the check found the word there.
            let Some(at) = text.find(&word) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the word `{word}` is not in the text of the cell {}",
                        cell.to_text()
                    ),
                });
            };
            let mut new = String::with_capacity(text.len());
            new.push_str(&text[..at]);
            new.push_str(&to);
            new.push_str(&text[at + word.len()..]);
            // The cells that are **locked** or **covered** are not
            // written.
            crate::protection::refuse_locked(&node, &crate::style_locks(session, sheet), &[cell], &id)?;
            crate::merges::refuse_covered(&node, &[cell], &id)?;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                let mut cells: crate::formula::Cells = crate::sheet_cells_mut(node);
                cells.set_value(cell, Value::Text(new.clone()));
                node.data["cells"] = crate::cells_into(&cells);
            }
            Ok(Outcome::changed(format!("the word `{word}` of {} is changed", cell.to_text())).with_data(json!({
                "sheet": sheet,
                "cell": cell.to_text(),
                "word": word,
                "with": to,
                "text": new,
            })))

        }
    }

    /// The **thesaurus**: the words that share a **stem** with a word.
    ///
    /// The stem of a word is the word without its last two letters, and
    /// the related words are the words of the list that begin with it.
    grid.thesaurus {
        area: "grid",
        layer: "grid",
        icon: "search",
        params: {
            /// The word.
            word: Text,
            /// The **word list**: a file of one word a line. Without it,
            /// the system's word list is used.
            dictionary: Option<Text>,
        },
        run: {
            let id = "grid.thesaurus".to_string();
            let word = self.word.0.trim().to_lowercase();
            let path = self.dictionary.as_ref().map(|path| path.0.trim().to_string());
            let Some((words, dictionary_truncated)) = a_dictionary(path.as_deref()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: match path {
                        Some(path) => format!("the word list `{path}` could not be read"),
                        None => "there is no word list on this machine".to_string(),
                    },
                });
            };
            if word.chars().count() < 3 {
                return Ok(Outcome::new("the word is too short to have a stem").with_data(json!({
                    "word": word,
                    "related": Vec::<String>::new(),
                    "truncated": false,
                    "dictionary_truncated": dictionary_truncated,
                })));
            }
            let stem: String =
                word.chars().take(word.chars().count().saturating_sub(2).max(3)).collect();
            let mut related: Vec<String> = words
                .iter()
                .filter(|other| {
                    other.starts_with(&stem) && **other != word && other.len() <= word.len() + 4
                })
                .cloned()
                .collect();
            related.sort();
            let truncated = related.len() > MAX_RELATED;
            related.truncate(MAX_RELATED);
            Ok(Outcome::new(format!("{} related word(s)", related.len())).with_data(json!({
                "word": word,
                "stem": stem,
                "related": related,
                // The list of the related words is cut at the tree's
                // bound, and the word list may be cut too: the answer
                // says so.
                "truncated": truncated,
                "dictionary_truncated": dictionary_truncated,
            })))

        }
    }
}
