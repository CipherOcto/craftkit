//! **Finding** and **replacing** in a block.
//!
//! A find is a **reading**: it says where the text is that matches a
//! pattern, and it changes nothing. A replace is a **change**: the
//! text is rewritten and the runs are rewritten with it, because the
//! runs describe slices of the text and the two must agree.
//!
//! The pattern is a text, and it may hold **wildcards**: a `*` stands
//! for any text at all and a `?` for one character, and a `~` in front
//! of one of them says that it is the character itself.
//!
//! # What the port found
//!
//! - **The tree's find has a language of its own** — `^p` for a
//!   paragraph mark, `^&` for the text that was found, the groups
//!   `\1` to `\9` — and it turns it into a **regular expression** so
//!   that a pattern cannot match for ever. Here the language is the
//!   wildcards and nothing else, and the matcher is **its own**: a
//!   pattern is matched by walking the text and the pattern together,
//!   and a `*` that matched too little takes one more character.
//! - **The tree's replace refuses to touch a tracked change.** Here the
//!   same: a match that lies inside a run with a **revision** is
//!   refused, and the block is left as it was.
//! - **The tree's replace keeps the formatting of the text it
//!   replaces.** Here the same: the replaced words carry the
//!   formatting of the run the match **begins** in.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

/// One piece of a **pattern**: a character, a wildcard, or a star.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    /// The character itself.
    Literal(char),
    /// One character, whatever it is.
    Any,
    /// Any text at all.
    Star,
}

/// The **tokens** of a pattern.
///
/// A `~` in front of a wildcard says that the wildcard is the
/// character itself, and a `~` in front of anything else is that
/// character.
fn tokens_of(pattern: &str, wildcards: bool) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character == '~' {
            if let Some(next) = characters.next() {
                tokens.push(Token::Literal(next));
            }
            continue;
        }
        if wildcards {
            match character {
                '*' => tokens.push(Token::Star),
                '?' => tokens.push(Token::Any),
                _ => tokens.push(Token::Literal(character)),
            }
        } else {
            tokens.push(Token::Literal(character));
        }
    }
    tokens
}

/// The pattern, folded when the case does not matter.
fn folded(tokens: &[Token], case: bool) -> Vec<Token> {
    if case {
        return tokens.to_vec();
    }
    tokens
        .iter()
        .map(|token| match token {
            Token::Literal(character) => {
                Token::Literal(character.to_lowercase().next().unwrap_or(*character))
            }
            other => *other,
        })
        .collect()
}

/// The text, folded when the case does not matter.
fn folded_text(text: &[char], case: bool) -> Vec<char> {
    if case {
        return text.to_vec();
    }
    text.iter().map(|character| character.to_lowercase().next().unwrap_or(*character)).collect()
}

/// Whether the pattern matches the text **at** a place, and where it
/// ends.
///
/// The `*` is the hard part: when the pattern does not fit, the star
/// takes one more character and the pattern is tried again.
fn match_at(tokens: &[Token], text: &[char], at: usize) -> Option<usize> {
    let mut in_text = at;
    let mut in_pattern = 0usize;
    // Where the last `*` was, and where the text was when it was
    // found.
    let mut star: Option<usize> = None;
    let mut mark = at;
    while in_text < text.len() {
        if in_pattern == tokens.len() {
            // The pattern is done and the text goes on: the pattern
            // matches the **beginning** of it. That is the tree's rule
            // for a search: the pattern is found somewhere in the text
            // and not only when it is the whole of it.
            return Some(in_text);
        }
        if tokens[in_pattern] == Token::Any || tokens[in_pattern] == Token::Literal(text[in_text]) {
            in_text += 1;
            in_pattern += 1;
        } else if tokens[in_pattern] == Token::Star {
            star = Some(in_pattern);
            in_pattern += 1;
            mark = in_text;
        } else {
            // The pattern does not fit and there is no star to give
            // it more room: it does not match here.
            let where_star = star?;
            in_pattern = where_star + 1;
            mark += 1;
            in_text = mark;
        }
    }
    // A pattern that ends with stars still matches.
    while in_pattern < tokens.len() && tokens[in_pattern] == Token::Star {
        in_pattern += 1;
    }
    if in_pattern == tokens.len() { Some(in_text) } else { None }
}

/// The **matches** of a pattern in a text: where they begin and where
/// they end.
///
/// The matches do not overlap: a match that begins inside another one
/// is not a match of its own.
fn matches_of(tokens: &[Token], text: &[char], case: bool) -> Vec<(usize, usize)> {
    let pattern = folded(tokens, case);
    let haystack = folded_text(text, case);
    let mut found: Vec<(usize, usize)> = Vec::new();
    let mut at = 0usize;
    while at < text.len() {
        match match_at(&pattern, &haystack, at) {
            Some(end) if end > at => {
                found.push((at, end));
                at = end;
            }
            // A pattern that matches nothing here may match further
            // on, and a pattern that matches nothing at all is not a
            // list of matches.
            _ => at += 1,
        }
    }
    found
}

/// The **text** of a block.
fn text_of(node: &Node) -> String {
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **runs** of a block.
fn runs_of(node: &Node) -> Vec<Json> {
    node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default()
}

/// The **block** a node is, if it is one.
fn the_block<'a>(session: &'a Session, id: &str, node: u64) -> Result<&'a Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no block `{node}`"),
        });
    };
    if !found.kind.starts_with("prose.block.") {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{node}` is a `{}` and not a block", found.kind),
        });
    }
    Ok(found)
}

/// The **pattern** a command is about, and whether the wildcards and
/// the case are as it says.
fn pattern_of(
    id: &str,
    what: &str,
    wildcards: Option<Bool>,
    case: Option<Bool>,
) -> Result<(Vec<Token>, bool), EngineError> {
    if what.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "there is nothing to find".to_string(),
        });
    }
    let tokens = tokens_of(what, wildcards.map(|wildcards| wildcards.0).unwrap_or(false));
    Ok((tokens, case.map(|case| case.0).unwrap_or(false)))
}

/// **Find** a pattern in a block.
#[derive(Command)]
#[command(id = "prose.find", area = "prose", layer = "prose", icon = "list")]
pub struct Find {
    /// The block.
    pub block: NodeRef,
    /// The pattern to find.
    pub what: Text,
    /// Whether the pattern holds **wildcards**: a `*` for any text and
    /// a `?` for one character.
    pub wildcards: Option<Bool>,
    /// Whether the **case** matters.
    pub case: Option<Bool>,
}

impl Find {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.find".to_string();
        let block = self.block.0;
        let (tokens, case) = pattern_of(&id, &self.what.0, self.wildcards, self.case)?;
        let node = the_block(session, &id, block)?;
        let text = text_of(node);
        let characters: Vec<char> = text.chars().collect();
        let found = matches_of(&tokens, &characters, case);
        let matches: Vec<Json> = found
            .iter()
            .map(|(from, to)| {
                json!({
                    "start": from,
                    "end": to,
                    "text": characters[*from..*to].iter().collect::<String>(),
                })
            })
            .collect();
        Ok(Outcome::new(format!("the block `{block}` has {} match(es)", matches.len()))
            .with_data(json!({ "block": block, "matches": matches })))
    }
}

/// **Replace** a pattern in a block.
///
/// The text is rewritten and the **runs** are rewritten with it: the
/// replaced words carry the formatting of the run the match begins in.
#[derive(Command)]
#[command(id = "prose.replace", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct Replace {
    /// The block.
    pub block: NodeRef,
    /// The pattern to find.
    pub what: Text,
    /// What to put in its place.
    pub with: Text,
    /// Whether the pattern holds **wildcards**: a `*` for any text and
    /// a `?` for one character.
    pub wildcards: Option<Bool>,
    /// Whether the **case** matters.
    pub case: Option<Bool>,
}

impl Replace {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.replace".to_string();
        let block = self.block.0;
        let (tokens, case) = pattern_of(&id, &self.what.0, self.wildcards, self.case)?;
        let node = the_block(session, &id, block)?;
        // The text of an **equation** is derived from its parts, and a
        // replace would make the two disagree.
        if crate::equations::is_equation(node) {
            return Err(EngineError::Failed {
                id,
                message: "the text of an equation is derived from its parts, and a replace would make them disagree"
                    .to_string(),
            });
        }
        let text = text_of(node);
        let runs = runs_of(node);
        let characters: Vec<char> = text.chars().collect();
        let found = matches_of(&tokens, &characters, case);
        if found.is_empty() {
            return Ok(Outcome::new(format!("the block `{block}` has nothing to replace"))
                .with_data(json!({ "block": block, "count": 0 })));
        }
        // A **revision** is not replaced: the tree refuses to touch a
        // tracked change, and so does this tree.
        let mut at = 0usize;
        for run in &runs {
            let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            let end = at + length;
            if run.get("revision").is_some() && found.iter().any(|(from, to)| *from < end && at < *to) {
                return Err(EngineError::Failed {
                    id,
                    message: "a tracked change is not replaced".to_string(),
                });
            }
            at = end;
        }
        let (new_text, new_runs) = rewritten(&characters, &runs, &found, &self.with.0);
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("there is no block `{block}`"),
            });
        };
        node.data["text"] = json!(new_text);
        node.data["runs"] = Json::Array(new_runs);
        Ok(Outcome::changed(format!("replaced {} match(es) in the block `{block}`", found.len()))
            .with_data(json!({ "block": block, "count": found.len() })))
    }
}

/// The **text** and the **runs** of a block with the matches replaced.
///
/// The replaced words carry the formatting of the run the match
/// **begins** in, and the runs that follow it lose the words of the
/// match.
fn rewritten(
    characters: &[char],
    runs: &[Json],
    found: &[(usize, usize)],
    with: &str,
) -> (String, Vec<Json>) {
    // The **boundaries** of the runs: where each one begins and ends.
    let mut starts: Vec<usize> = Vec::with_capacity(runs.len());
    let mut at = 0usize;
    for run in runs {
        starts.push(at);
        at += run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
    }
    // The run a character is in.
    let run_of = |position: usize| -> usize {
        let mut index = 0usize;
        for (number, start) in starts.iter().enumerate() {
            if position >= *start {
                index = number;
            } else {
                break;
            }
        }
        index
    };
    // The **pieces**: the text and the run each piece belongs to.
    let mut pieces: Vec<(String, usize)> = Vec::new();
    let mut cursor = 0usize;
    for (from, to) in found {
        // The words between the last match and this one, split by the
        // runs they lie in.
        let mut at = cursor;
        while at < *from {
            let owner = run_of(at);
            let end =
                starts[owner] + runs[owner].get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            // A run that has ended already — the text is longer than
            // the runs cover — still holds the words that follow it.
            let stop = if end > at { end.min(*from) } else { (at + 1).min(*from) };
            if stop <= at {
                at += 1;
                continue;
            }
            pieces.push((characters[at..stop].iter().collect(), owner));
            at = stop;
        }
        // The words of the match are the ones of the run it begins in.
        pieces.push((with.to_string(), run_of(*from)));
        cursor = *to;
    }
    let mut at = cursor;
    while at < characters.len() {
        let owner = run_of(at);
        let end = starts[owner] + runs[owner].get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let stop = if end > at { end.min(characters.len()) } else { at + 1 };
        if stop <= at {
            at += 1;
            continue;
        }
        pieces.push((characters[at..stop].iter().collect(), owner));
        at = stop;
    }
    // The **text** and the **runs**: the pieces that belong to the same
    // run and follow each other make one run.
    let mut text = String::new();
    let mut new_runs: Vec<(Json, usize)> = Vec::new();
    for (piece, owner) in pieces {
        text.push_str(&piece);
        let length = piece.chars().count();
        let Some(run) = runs.get(owner) else { continue };
        match new_runs.last_mut() {
            Some((last, last_owner)) if *last_owner == owner => {
                let current = last.get("len").and_then(|len| len.as_u64()).unwrap_or(0);
                last["len"] = json!(current + length as u64);
            }
            _ => {
                let mut run = run.clone();
                run["len"] = json!(length as u64);
                new_runs.push((run, owner));
            }
        }
    }
    (text, new_runs.into_iter().map(|(run, _)| run).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The matcher finds a plain word.
    #[test]
    fn the_matcher_finds_a_plain_word() {
        let tokens = tokens_of("quick", false);
        let text: Vec<char> = "the quick brown fox".chars().collect();
        let found = matches_of(&tokens, &text, false);
        assert_eq!(found, vec![(4, 9)], "the word is at the fourth character");
    }
}
