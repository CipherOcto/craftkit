//! The **citation styles** of a document.
//!
//! A citation style says how a document cites its sources in the text
//! and how it lists them at the end. The tree has **twelve** of them —
//! APA, Chicago, MLA and the rest — and a document names the one it
//! uses.
//!
//! # The rules
//!
//! - The styles are **the tree's own**, and they are **data**: what a
//!   style is does not change, and a document does not hold a copy of
//!   it.
//! - A style is found by its **name**, whatever the case, or by a name
//!   that **begins** with one of its aliases — the **longest** alias
//!   wins. The name of a style sheet file names it too: `\APA.XSL` is
//!   APA, and `\GostTitle.XSL` is the GOST that sorts by title.
//! - A style that is **not one of the tree's** is refused **by name**:
//!   the tree's own rule, and the refusal names the styles there are.
//! - A document that names **no** style cites by **APA**: the tree's
//!   own default.
//! - The **citation** of a source is **derived**: it is worked out from
//!   the source and the style of the document every time, and it is
//!   never written down, so it cannot be out of date.
//!
//! # What it does not do
//!
//! - **The bibliography is not written**: the entries of a style are
//!   another thing.
//! - **The names of the authors are not written**: the tree's styles
//!   each write an author's name in their own way, and that is the
//!   writing of an entry. A citation names the **surnames**.
//! - **The citation is not put into the text**: a citation is said, and
//!   putting it into a paragraph is another thing.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// How a style cites **in the text**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cite {
    /// The author and the year, and the pages: the year is written
    /// after the author, and the pages after the year.
    AuthorDate {
        /// What is written between the author and the year.
        year_sep: &'static str,
        /// What is written between the year and the pages.
        pages_sep: &'static str,
    },
    /// The author and the pages.
    AuthorPage,
    /// A number, in brackets or in parentheses.
    Number {
        /// What opens the number.
        open: &'static str,
        /// What closes the number.
        close: &'static str,
        /// What is written between the number and the pages, when the
        /// style writes pages at all.
        pages_sep: Option<&'static str>,
    },
}

/// How a style **orders** its bibliography.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// By the surname of the first author, and by the title when there
    /// is no author.
    Author,
    /// By the title.
    Title,
    /// In the order the sources are first cited.
    Cited,
}

/// A **citation style**, as the tree's data.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// The name the style is shown by.
    pub name: &'static str,
    /// The other names that mean the style: the beginnings of the
    /// names of the style sheets of the tree.
    pub aliases: &'static [&'static str],
    /// How the style cites in the text.
    pub cite: Cite,
    /// The word that joins two authors.
    pub and: &'static str,
    /// How the style orders its bibliography.
    pub order: Order,
    /// From how many authors the style says **et al.** instead of
    /// naming them all.
    pub et_al_from: usize,
    /// The title of the bibliography of the style.
    pub bib_title: &'static str,
}

/// The **styles** of the tree, in the order the tree shows them.
pub const STYLES: [Style; 12] = [
    Style {
        name: "APA",
        aliases: &["apa"],
        cite: Cite::AuthorDate { year_sep: ", ", pages_sep: ", p. " },
        and: "&",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "Chicago",
        aliases: &["chicago"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "GB/T 7714",
        aliases: &["gb"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "GOST (name sort)",
        aliases: &["gost", "gostname"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "GOST (title sort)",
        aliases: &["gosttitle"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Title,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "Harvard (Anglia)",
        aliases: &["harvard"],
        cite: Cite::AuthorDate { year_sep: ", ", pages_sep: ", p. " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "References",
    },
    Style {
        name: "IEEE",
        aliases: &["ieee"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: None },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "ISO 690 (author-date)",
        aliases: &["iso690"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", p. " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "References",
    },
    Style {
        name: "ISO 690 (numerical)",
        aliases: &["iso690num", "iso690nmerical"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 4,
        bib_title: "References",
    },
    Style {
        name: "MLA",
        aliases: &["mla"],
        cite: Cite::AuthorPage,
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "Works Cited",
    },
    Style {
        name: "SIST02",
        aliases: &["sist"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
    },
    Style {
        name: "Turabian",
        aliases: &["turabian"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "Bibliography",
    },
];

/// The style a name says, if it is **one of the tree's**.
///
/// The tree's own rule: the name of the style, whatever the case, and
/// then a name that **begins** with one of its aliases — the longest
/// alias wins. The name of a style sheet file names it too, and the
/// `xsl` at the end of it is not part of the name.
pub fn find(name: &str) -> Option<&'static Style> {
    if let Some(style) = STYLES.iter().find(|style| style.name.eq_ignore_ascii_case(name.trim())) {
        return Some(style);
    }
    let key: String = name
        .chars()
        .filter(|letter| letter.is_ascii_alphanumeric())
        .map(|letter| letter.to_ascii_lowercase())
        .collect();
    let key = key.strip_suffix("xsl").unwrap_or(&key);
    STYLES
        .iter()
        .flat_map(|style| style.aliases.iter().map(move |alias| (style, *alias)))
        .filter(|(_, alias)| key.starts_with(alias))
        .max_by_key(|(_, alias)| alias.len())
        .map(|(style, _)| style)
}

/// The names of the **styles** of the tree, as the refusal names them.
pub fn names() -> Vec<&'static str> {
    STYLES.iter().map(|style| style.name).collect()
}

/// Whether a style **numbers** its citations: the tree's own rule.
pub fn numbered(style: &Style) -> bool {
    matches!(style.cite, Cite::Number { .. })
}

/// The style a **document** cites by.
///
/// A document that names no style cites by APA: the tree's own
/// default.
pub fn style_of(node: &Node) -> &'static Style {
    let name = node.data.get("citation_style").and_then(|style| style.as_str()).unwrap_or_default();
    find(name).unwrap_or(&STYLES[0])
}

/// The **authors** of a source: the surname and the first name of each.
///
/// The tree's own rule: the authors are separated by a semicolon, and
/// an author is written "surname, first name" — and when it is not, the
/// last word of it is the surname.
pub fn authors_of(author: &str) -> Vec<(String, String)> {
    author
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part.split_once(',') {
            Some((last, first)) => (last.trim().to_string(), first.trim().to_string()),
            None => match part.rsplit_once(' ') {
                Some((first, last)) => (last.trim().to_string(), first.trim().to_string()),
                None => (part.to_string(), String::new()),
            },
        })
        .collect()
}

/// The **citation** of a source, in a style.
///
/// The tree's own rules:
/// - the **who** is the surnames of the authors: one surname is said
///   alone, two are joined by the word of the style, and from as many
///   authors as the style says **et al.** the first surname and "et
///   al." are said instead;
/// - a source with **no author** is cited by its **title**;
/// - the citation is then written the way the style writes it: the
///   author and the year, the author and the pages, or the number of
///   the source.
pub fn cite_of(source: &Json, style: &Style, number: usize, pages: &str) -> String {
    let author = source.get("author").and_then(|author| author.as_str()).unwrap_or_default();
    let title = source.get("title").and_then(|title| title.as_str()).unwrap_or_default();
    let year = source.get("year").and_then(|year| year.as_str()).unwrap_or_default();
    let authors = authors_of(author);
    let surnames: Vec<&str> = authors.iter().map(|(last, _)| last.as_str()).collect();
    let who = match surnames.as_slice() {
        [] => title.to_string(),
        [one] => (*one).to_string(),
        [first, ..] if surnames.len() >= style.et_al_from.max(2) => format!("{first} et al."),
        [a, b] => format!("{a} {} {b}", style.and),
        [init @ .., last] => format!("{} {} {last}", init.join(", "), style.and),
    };
    match style.cite {
        Cite::AuthorPage if pages.is_empty() => format!("({who})"),
        Cite::AuthorPage => format!("({who} {pages})"),
        Cite::AuthorDate { year_sep, .. } if pages.is_empty() => format!("({who}{year_sep}{year})"),
        Cite::AuthorDate { year_sep, pages_sep } => format!("({who}{year_sep}{year}{pages_sep}{pages})"),
        Cite::Number { open, close, pages_sep: Some(sep) } if !pages.is_empty() => {
            format!("{open}{number}{sep}{pages}{close}")
        }
        Cite::Number { open, close, .. } => format!("{open}{number}{close}"),
    }
}

/// A style, as it is reported.
fn to_json(style: &Style) -> Json {
    json!({
        "name": style.name,
        "aliases": style.aliases.to_vec(),
        "cite": match style.cite {
            Cite::AuthorDate { .. } => json!("author-date"),
            Cite::AuthorPage => json!("author-page"),
            Cite::Number { .. } => json!("number"),
        },
        "cite_format": match style.cite {
            Cite::AuthorDate { year_sep, pages_sep } => json!({ "year": year_sep, "pages": pages_sep }),
            Cite::AuthorPage => json!({}),
            Cite::Number { open, close, pages_sep } => json!({
                "open": open,
                "close": close,
                "pages": match pages_sep {
                    Some(sep) => json!(sep),
                    None => Json::Null,
                },
            }),
        },
        "numbered": numbered(style),
        "and": style.and,
        "order": match style.order {
            Order::Author => json!("author"),
            Order::Title => json!("title"),
            Order::Cited => json!("cited"),
        },
        "et_al_from": style.et_al_from,
        "bib_title": style.bib_title,
    })
}

/// **Read** the **citation styles** of this tree.
///
/// A query: it does not change the document. The styles are the
/// tree's own, and what each one says is reported.
#[derive(Command)]
#[command(id = "prose.citation.styles", area = "prose", layer = "prose", icon = "list")]
pub struct ListCitationStyles {}

impl ListCitationStyles {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let styles: Vec<Json> = STYLES.iter().map(to_json).collect();
        Ok(Outcome::new(format!("this tree has {} citation style(s)", styles.len()))
            .with_data(json!({ "styles": styles })))
    }
}

/// **Say** which **citation style** a document cites by.
///
/// The style is a fact of the document, and a style that is not one of
/// the tree's is refused by name.
#[derive(Command)]
#[command(
    id = "prose.document.citestyle.set",
    area = "prose",
    layer = "prose",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct SetCitationStyle {
    /// The document.
    pub document: NodeRef,
    /// The name of the style.
    ///
    /// The name of the style, or a name that begins with one of its
    /// aliases: `apa`, `chicago`, `mla`, `gosttitle`…
    pub style: Text,
}

impl SetCitationStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.citestyle.set".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let name = self.style.0.trim();
        let Some(style) = find(name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the style `{name}` is not one of this tree's: it is one of {}",
                    names().join(", ")
                ),
            });
        };
        let name = style.name.to_string();
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        node.data["citation_style"] = json!(name);
        Ok(Outcome::changed(format!("the document `{document}` cites by `{name}`"))
            .with_data(json!({ "document": document, "style": name })))
    }
}

/// **Read** the **citation style** of a document.
///
/// A query: it does not change the document. A document that names no
/// style cites by APA: the tree's own default.
#[derive(Command)]
#[command(id = "prose.document.citestyle.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadCitationStyle {
    /// The document.
    pub document: NodeRef,
}

impl ReadCitationStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.citestyle.read".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let style = style_of(&node);
        Ok(Outcome::new(format!("the document `{document}` cites by `{}`", style.name))
            .with_data(json!({ "document": document, "style": to_json(style) })))
    }
}

/// **Read** the **citation** of a source, in the style of its
/// document.
///
/// A query: it does not change the document. The citation is
/// **derived**: it is worked out from the source and the style of the
/// document every time, so it cannot be out of date.
#[derive(Command)]
#[command(id = "prose.document.citation", area = "prose", layer = "prose", icon = "list")]
pub struct ReadCitation {
    /// The document the source belongs to.
    pub document: NodeRef,
    /// The tag that names the source.
    pub tag: Text,
    /// The pages of the source that are cited.
    pub pages: Option<Text>,
}

impl ReadCitation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.citation".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let tag = self.tag.0.trim().to_string();
        let sources = crate::sources::sources_of(&node);
        let Some(source) = sources.iter().find(|source| {
            source.get("tag").and_then(|held| held.as_str()).map(|held| held == tag).unwrap_or(false)
        }) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no source with the tag `{tag}`"),
            });
        };
        let style = style_of(&node);
        // The **number** of the source: where it stands in the order
        // the sources are cited, and the tree's own rule for a source
        // that is not cited yet — it stands after the ones that are.
        let cited = crate::sources::cited_of(session, document);
        let number = match cited.iter().position(|held| *held == tag) {
            Some(at) => at + 1,
            None => cited.len() + 1,
        };
        let pages = self.pages.as_ref().map(|pages| pages.0.trim().to_string()).unwrap_or_default();
        let citation = cite_of(source, style, number, &pages);
        Ok(Outcome::new(format!("the source `{tag}` is cited as `{citation}`")).with_data(json!({
            "document": document,
            "tag": tag,
            "style": style.name,
            "number": match numbered(style) {
                true => json!(number),
                false => Json::Null,
            },
            "pages": match pages.is_empty() {
                true => Json::Null,
                false => json!(pages),
            },
            "citation": citation,
        })))
    }
}
