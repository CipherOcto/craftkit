//! The **sources** of a document, and the **citations** of them.
//!
//! A **source** is a thing a document points at: a book, an article,
//! a report, a recording. A **citation** is a run that names one, and
//! the **bibliography** is the list of the sources that are cited,
//! worked out every time.
//!
//! # The rules
//!
//! - The sources live in the **payload** of the document: they are
//!   numerous and they hold nothing.
//! - The **kind** of a source is one of the tree's own: `book`,
//!   `bookSection`, `article`, `periodical`, `website`,
//!   `electronicsource`, `report`, `conference`, `case`, `art`,
//!   `film`, `interview`, `patent`, `performance`, `soundrecording`,
//!   `standard`, `thesis` and `misc`. A kind that is not one of them
//!   is refused by name.
//! - A source has a **title**, and a source with no title is refused.
//! - The **tag** names a source, and it is at most **64** characters
//!   long. A source whose tag is not given is **given one**: the
//!   first three letters of the first author and the last two digits
//!   of the year, with `1`, `2`… added while the tag is taken — the
//!   tree's own rule.
//! - The **author** is a text of at most **256** characters, and the
//!   **year** is a text of at most **32**.
//! - The **parts** of a source — the place it was made, the publisher,
//!   the journal, the volume, the pages and the address — are texts of
//!   at most **1024** characters. They are the parts the tree's own
//!   entries are written from, and a part that is not said keeps what
//!   it was.
//! - A **run** may carry a **citation**: the tag of a source. A tag
//!   that names no source is refused.
//! - The **bibliography** is **derived**: it is the sources that are
//!   cited, in the order they are first cited, and it cannot be out
//!   of date.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **kinds** a source may have: the tree's own.
pub const KINDS: [&str; 18] = [
    "book",
    "bookSection",
    "article",
    "periodical",
    "website",
    "electronicsource",
    "report",
    "conference",
    "case",
    "art",
    "film",
    "interview",
    "patent",
    "performance",
    "soundrecording",
    "standard",
    "thesis",
    "misc",
];

/// The longest **tag** a source may have.
pub const MAX_TAG: usize = 64;

/// The longest **author** a source may have.
pub const MAX_AUTHOR: usize = 256;

/// The longest **year** a source may have.
pub const MAX_YEAR: usize = 32;

/// The longest **part** of a source may have: the place, the
/// publisher, the journal, the volume, the pages and the address.
pub const MAX_PART: usize = 1024;

/// The **sources** of a document, as they are kept.
pub fn sources_of(node: &Node) -> Vec<Json> {
    node.data.get("sources").and_then(|sources| sources.as_array()).cloned().unwrap_or_default()
}

/// The **document** a block is in.
pub fn document_of(session: &Session, block: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == block))
        .map(|node| node.id)
}

/// The **kind** a name says, if it is one.
fn kind_of(name: &str, id: &str) -> Result<&'static str, EngineError> {
    KINDS.iter().find(|kind| kind.eq_ignore_ascii_case(name.trim())).copied().ok_or_else(|| {
        EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no kind `{name}`: the kinds are {}", KINDS.join(", ")),
        }
    })
}

/// The **tag** a name says.
fn tag_of(name: &str, id: &str) -> Result<String, EngineError> {
    let tag = name.trim();
    if tag.is_empty() {
        return Err(EngineError::Failed { id: id.to_string(), message: "a source has a tag".to_string() });
    }
    if tag.chars().count() > MAX_TAG {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a tag is at most {MAX_TAG} characters long"),
        });
    }
    Ok(tag.to_string())
}

/// The **tag** a source is given when it is not given one: the
/// tree's own rule — the first three letters of the first author and
/// the last two digits of the year, with `1`, `2`… added while the
/// tag is taken.
fn made_tag(sources: &[Json], author: &str, year: &str) -> String {
    let first = author.split(',').next().unwrap_or(author);
    let letters: String = first.chars().filter(|character| character.is_alphanumeric()).take(3).collect();
    let letters = if letters.is_empty() { "Src".to_string() } else { letters };
    let digits: Vec<char> = year.chars().filter(|character| character.is_ascii_digit()).collect();
    let year: String = digits.get(digits.len().saturating_sub(2)..).unwrap_or_default().iter().collect();
    let base = format!("{letters}{year}");
    let taken = |tag: &str| {
        sources.iter().any(|source| {
            source.get("tag").and_then(|tag| tag.as_str()).map(|held| held == tag).unwrap_or(false)
        })
    };
    if !taken(&base) {
        return base;
    }
    (1u32..).map(|number| format!("{base}{number}")).find(|tag| !taken(tag)).unwrap_or(base)
}

/// A **citation** names a source: the tag must be one of them.
pub fn check_citation(session: &Session, block: u64, tag: &str, id: &str) -> Result<(), EngineError> {
    let Some(document) = document_of(session, block) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the block `{block}` is in no document"),
        });
    };
    let sources = session.document().node(document).map(sources_of).unwrap_or_default();
    let found = sources.iter().any(|source| {
        source.get("tag").and_then(|held| held.as_str()).map(|held| held == tag).unwrap_or(false)
    });
    if !found {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no source with the tag `{tag}`"),
        });
    }
    Ok(())
}

/// The **citations** of a document: the tags of the sources that are
/// cited, in the order they are first cited.
pub fn cited_of(session: &Session, document: u64) -> Vec<String> {
    let Some(node) = session.document().node(document) else { return Vec::new() };
    let mut cited: Vec<String> = Vec::new();
    for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        let Some(runs) = block.data.get("runs").and_then(|runs| runs.as_array()) else { continue };
        for run in runs {
            let Some(tag) = run.get("citation").and_then(|tag| tag.as_str()) else { continue };
            if !cited.iter().any(|held| held == tag) {
                cited.push(tag.to_string());
            }
        }
    }
    cited
}

/// The **document** the sources belong to.
fn the_document(session: &Session, command: &str, document: u64) -> Result<(), EngineError> {
    let Some(node) = session.document().node(document) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no document `{document}`"),
        });
    };
    if node.kind != "prose.document" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{document}` is a `{}` and not a document", node.kind),
        });
    }
    Ok(())
}

/// **Say** what a **source** is.
///
/// A source whose tag is not there yet is **added**, and a source
/// whose tag is there is **changed**: the parts that are named change
/// and the parts that are not named keep what they were.
#[derive(Command)]
#[command(id = "prose.document.source.set", area = "prose", layer = "prose", icon = "tag", mutates, undoable)]
pub struct SetSource {
    /// The document.
    pub document: NodeRef,
    /// The tag that names the source.
    ///
    /// A source that is not named by a tag is given one.
    pub tag: Option<Text>,
    /// The kind of the source.
    pub kind: Option<Text>,
    /// Who wrote it.
    pub author: Option<Text>,
    /// What it is called.
    pub title: Option<Text>,
    /// When it was made.
    pub year: Option<Text>,
    /// Who published it.
    pub publisher: Option<Text>,
    /// Where it was made.
    pub city: Option<Text>,
    /// The journal it is in.
    pub journal: Option<Text>,
    /// The volume of the journal it is in.
    pub volume: Option<Text>,
    /// The pages of it.
    pub pages: Option<Text>,
    /// Where it may be found.
    pub url: Option<Text>,
}

impl SetSource {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.source.set".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        let mut sources = sources_of(node);
        let kind = match &self.kind {
            Some(kind) => Some(kind_of(&kind.0, &id)?),
            None => None,
        };
        let author = match &self.author {
            Some(author) => {
                let author = author.0.trim();
                if author.chars().count() > MAX_AUTHOR {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("an author is at most {MAX_AUTHOR} characters long"),
                    });
                }
                Some(author.to_string())
            }
            None => None,
        };
        let year = match &self.year {
            Some(year) => {
                let year = year.0.trim();
                if year.chars().count() > MAX_YEAR {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a year is at most {MAX_YEAR} characters long"),
                    });
                }
                Some(year.to_string())
            }
            None => None,
        };
        // The **source** that is there, if it is there.
        let tag = match &self.tag {
            Some(tag) => Some(tag_of(&tag.0, &id)?),
            None => None,
        };
        let held = tag.as_ref().and_then(|tag| {
            sources
                .iter()
                .position(|source| source.get("tag").and_then(|held| held.as_str()) == Some(tag.as_str()))
        });
        let mut source = match held {
            Some(at) => sources[at].clone(),
            None => {
                // A **new** source needs a title, and a tag.
                let Some(title) = &self.title else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a source has a title".to_string(),
                    });
                };
                let title = title.0.trim();
                if title.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a source has a title".to_string(),
                    });
                }
                let tag = match tag {
                    Some(tag) => tag,
                    None => {
                        made_tag(&sources, author.as_deref().unwrap_or(""), year.as_deref().unwrap_or(""))
                    }
                };
                json!({
                    "tag": tag,
                    "kind": "book",
                    "author": "",
                    "title": title,
                    "year": "",
                })
            }
        };
        if let Some(kind) = kind {
            source["kind"] = json!(kind);
        }
        if let Some(author) = author {
            source["author"] = json!(author);
        }
        if let Some(title) = &self.title {
            let title = title.0.trim();
            if title.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a source has a title".to_string(),
                });
            }
            source["title"] = json!(title);
        }
        if let Some(year) = year {
            source["year"] = json!(year);
        }
        // The **parts** the tree's entries are written from. A part
        // that is not said keeps what it was, and a part that is said
        // empty is not there.
        for (key, part) in [
            ("publisher", &self.publisher),
            ("city", &self.city),
            ("journal", &self.journal),
            ("volume", &self.volume),
            ("pages", &self.pages),
            ("url", &self.url),
        ] {
            let Some(part) = part else { continue };
            let part = part.0.trim();
            if part.chars().count() > MAX_PART {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the {key} of a source is at most {MAX_PART} characters long"),
                });
            }
            if part.is_empty() {
                if let Json::Object(map) = &mut source {
                    map.remove(key);
                }
                continue;
            }
            source[key] = json!(part);
        }
        let tag = source.get("tag").and_then(|tag| tag.as_str()).unwrap_or_default().to_string();
        match held {
            Some(at) => sources[at] = source.clone(),
            None => sources.push(source.clone()),
        }
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        node.data["sources"] = Json::Array(sources);
        Ok(Outcome::changed(format!("the source `{tag}` of the document `{document}` is said"))
            .with_data(json!({ "document": document, "source": source })))
    }
}

/// **Take away** a **source** of a document.
#[derive(Command)]
#[command(
    id = "prose.document.source.remove",
    area = "prose",
    layer = "prose",
    icon = "trash",
    mutates,
    undoable
)]
pub struct RemoveSource {
    /// The document.
    pub document: NodeRef,
    /// The tag that names the source.
    pub tag: Text,
}

impl RemoveSource {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.source.remove".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let tag = tag_of(&self.tag.0, &id)?;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        let mut sources = sources_of(node);
        let before = sources.len();
        sources.retain(|source| {
            source.get("tag").and_then(|held| held.as_str()).map(|held| held != tag).unwrap_or(true)
        });
        if sources.len() == before {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no source with the tag `{tag}`"),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        node.data["sources"] = Json::Array(sources);
        Ok(Outcome::changed(format!("the source `{tag}` of the document `{document}` is taken away"))
            .with_data(json!({ "document": document, "tag": tag })))
    }
}

/// **Read** the **sources** of a document.
#[derive(Command)]
#[command(id = "prose.document.source.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadSources {
    /// The document.
    pub document: NodeRef,
}

impl ReadSources {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.source.read".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let sources = session.document().node(document).map(sources_of).unwrap_or_default();
        Ok(Outcome::new(format!("the document `{document}` has {} sources", sources.len()))
            .with_data(json!({ "document": document, "sources": sources })))
    }
}

/// **Read** the **bibliography** of a document: the sources that are
/// cited, in the order they are first cited.
///
/// The bibliography is **derived**: it is worked out from the sources
/// and the citations every time it is asked for.
#[derive(Command)]
#[command(id = "prose.document.bibliography", area = "prose", layer = "prose", icon = "list")]
pub struct ReadBibliography {
    /// The document.
    pub document: NodeRef,
}

impl ReadBibliography {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.bibliography".to_string();
        let document = self.document.0;
        the_document(session, &id, document)?;
        let sources = session.document().node(document).map(sources_of).unwrap_or_default();
        let cited = cited_of(session, document);
        let bibliography: Vec<Json> = cited
            .iter()
            .filter_map(|tag| {
                sources
                    .iter()
                    .find(|source| source.get("tag").and_then(|held| held.as_str()) == Some(tag.as_str()))
                    .cloned()
            })
            .collect();
        Ok(Outcome::new(format!("the document `{document}` cites {} sources", bibliography.len()))
            .with_data(json!({ "document": document, "bibliography": bibliography })))
    }
}
