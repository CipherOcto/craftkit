//! The **statistics** of a document.
//!
//! A document may be measured: how many words it holds, how many
//! characters, how many paragraphs.
//!
//! The rules:
//!
//! - a **word** is a run of text between spaces that holds at least
//!   one letter or digit — the tree's counting rule, and the same
//!   rule here: a mark that is not a letter or a digit is not a word
//!   of its own;
//! - the **characters** are counted twice: with the spaces and
//!   without them;
//! - the counting is of the **body** of the document: the footnotes
//!   and the comments are other stories, and they are not counted
//!   here — a number that counts more than it counts is a number a
//!   person does not believe.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// The **words** a text holds: the runs of text between spaces that
/// hold a letter or a digit.
pub fn words_of(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter(|word| word.chars().any(|character| character.is_alphanumeric()))
        .map(|word| word.to_string())
        .collect()
}

/// The **text** a block holds, as a string.
fn text_of(block: &craftkit_sdk::Node) -> String {
    match block.data.get("text") {
        Some(craftkit_sdk::serde_json::Value::String(text)) => text.clone(),
        _ => String::new(),
    }
}

/// **Read** the **statistics** of a document.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "prose.document.stats", area = "prose", layer = "prose", icon = "list")]
pub struct ReadStatistics {
    /// The document to measure.
    pub document: NodeRef,
}

impl ReadStatistics {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.stats".to_string();
        let document = session.document();
        let Some(node) = document.node(self.document.0) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", self.document.0),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a document", self.document.0, node.kind),
            });
        }
        let mut blocks = 0usize;
        let mut paragraphs = 0usize;
        let mut words = 0usize;
        let mut characters = 0usize;
        let mut characters_without_spaces = 0usize;
        for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
            blocks += 1;
            if block.kind == "prose.block.paragraph" {
                paragraphs += 1;
            }
            let text = text_of(block);
            words += words_of(&text).len();
            characters += text.chars().count();
            characters_without_spaces += text.chars().filter(|character| !character.is_whitespace()).count();
        }
        Ok(Outcome::new(format!("the document holds {words} word(s)")).with_data(json!({
            "document": self.document.0,
            "blocks": blocks,
            "paragraphs": paragraphs,
            "words": words,
            "characters": characters,
            "characters_without_spaces": characters_without_spaces,
        })))
    }
}
