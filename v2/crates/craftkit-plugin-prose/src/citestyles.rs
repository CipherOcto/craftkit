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
//! - The **entry** of a source is **derived** in the same way, and the
//!   **order** of the bibliography is the order the style asks for.
//!
//! # What it does not do
//!
//! - **The bibliography is not put into the document**: the entries are
//!   said, and writing them into the text is another thing.
//! - **The names of the authors are not written**: the tree's styles
//!   each write an author's name in their own way, and that is the
//!   writing of an entry. A citation names the **surnames**.

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

/// How a style writes **one author's name**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Name {
    /// `Rivera, Alex`
    LastFirst,
    /// `Alex Rivera`
    FirstLast,
    /// `Rivera, A.`
    LastInitials,
    /// `A. Rivera`
    InitialsLast,
    /// `Rivera A.`
    LastSpaceInitials,
    /// `RIVERA, Alex`
    UpperLastFirst,
    /// `RIVERA A`
    UpperLastBareInitials,
}

/// The **author list** of an entry: how the names are written and how
/// they are joined.
#[derive(Clone, Copy, Debug)]
pub struct Names {
    /// The form of the **first** author's name.
    pub first: Name,
    /// The form of **every later** author's name.
    pub rest: Name,
    /// What is written between two names.
    pub sep: &'static str,
    /// What is written between **exactly two** names.
    pub two: &'static str,
    /// What is written before the **last** of three or more.
    pub last: &'static str,
    /// How many authors are written at most; **zero** writes them all.
    pub max: usize,
    /// What is written after the names when some of them are left out.
    pub et_al: &'static str,
}

/// The **names** of a style that writes every author.
const fn names(first: Name, rest: Name, sep: &'static str, two: &'static str, last: &'static str) -> Names {
    Names { first, rest, sep, two, last, max: 0, et_al: "" }
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
    /// How the style writes the author list of an entry.
    pub names: Names,
    /// The template of the entry of a **book**.
    pub book: &'static str,
    /// The template of the entry of an **article**.
    pub article: &'static str,
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
        names: names(Name::LastInitials, Name::LastInitials, ", ", ", & ", ", & "),
        book: "⟦{names} ⟧({year}). {title}.⟦ {publisher}.⟧⟦ {url}⟧",
        article: "⟦{names} ⟧({year}). {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ {url}⟧",
    },
    Style {
        name: "Chicago",
        aliases: &["chicago"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "References",
        names: names(Name::LastFirst, Name::FirstLast, ", ", ", and ", ", and "),
        book: "⟦{names}. ⟧{year}. {title}.⟦ {pub}.⟧⟦ {url}⟧",
        article: "⟦{names}. ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ {url}⟧",
    },
    Style {
        name: "GB/T 7714",
        aliases: &["gb"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
        names: Names {
            max: 3,
            et_al: ", et al.",
            ..names(Name::UpperLastBareInitials, Name::UpperLastBareInitials, ", ", ", ", ", ")
        },
        book: "[{n}] ⟦{names}. ⟧{title}[{type}].⟦ {pub},⟧ {year}.⟦ {url}.⟧",
        article: "[{n}] ⟦{names}. ⟧{title}[{type}]. {journal}, {year}⟦, {volume}⟧⟦: {pages}⟧.⟦ {url}.⟧",
    },
    Style {
        name: "GOST (name sort)",
        aliases: &["gost", "gostname"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "References",
        names: names(Name::LastSpaceInitials, Name::LastSpaceInitials, ", ", ", ", ", "),
        book: GOST_BOOK,
        article: GOST_ARTICLE,
    },
    Style {
        name: "GOST (title sort)",
        aliases: &["gosttitle"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Title,
        et_al_from: 3,
        bib_title: "References",
        names: names(Name::LastSpaceInitials, Name::LastSpaceInitials, ", ", ", ", ", "),
        book: GOST_BOOK,
        article: GOST_ARTICLE,
    },
    Style {
        name: "Harvard (Anglia)",
        aliases: &["harvard"],
        cite: Cite::AuthorDate { year_sep: ", ", pages_sep: ", p. " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "References",
        names: names(Name::LastInitials, Name::LastInitials, ", ", " and ", " and "),
        book: "⟦{names}, ⟧{year}. {title}.⟦ {pub}.⟧⟦ Available at: <{url}>.⟧",
        article: "⟦{names}, ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, pp. {pages}⟧.⟦ Available at: <{url}>.⟧",
    },
    Style {
        name: "IEEE",
        aliases: &["ieee"],
        cite: Cite::Number { open: "[", close: "]", pages_sep: None },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
        names: names(Name::InitialsLast, Name::InitialsLast, ", ", ", ", ", "),
        book: "[{n}] ⟦{names}, ⟧“{title},”⟦ {pub},⟧ {year}.⟦ {url}⟧",
        article: "[{n}] ⟦{names}, ⟧“{title},” {journal}⟦, {volume}⟧⟦, {pages}⟧, {year}.⟦ {url}⟧",
    },
    Style {
        name: "ISO 690 (author-date)",
        aliases: &["iso690"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", p. " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "References",
        names: names(Name::UpperLastFirst, Name::UpperLastFirst, ", ", " and ", " and "),
        book: "⟦{names}, ⟧{year}. {title}.⟦ {pub}.⟧⟦ Available from: {url}⟧",
        article: "⟦{names}, ⟧{year}. {title}. {journal}⟦, {volume}⟧⟦, {pages}⟧.⟦ Available from: {url}⟧",
    },
    Style {
        name: "ISO 690 (numerical)",
        aliases: &["iso690num", "iso690nmerical"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 4,
        bib_title: "References",
        names: names(Name::UpperLastFirst, Name::UpperLastFirst, ", ", " and ", " and "),
        book: "{n}. ⟦{names}. ⟧{title}.⟦ {pub},⟧ {year}.⟦ Available from: {url}⟧",
        article: "{n}. ⟦{names}. ⟧{title}. {journal}. {year}⟦, {volume}⟧⟦, {pages}⟧.⟦ Available from: {url}⟧",
    },
    Style {
        name: "MLA",
        aliases: &["mla"],
        cite: Cite::AuthorPage,
        and: "and",
        order: Order::Author,
        et_al_from: 3,
        bib_title: "Works Cited",
        names: Names {
            max: 1,
            et_al: ", et al.",
            ..names(Name::LastFirst, Name::LastFirst, ", ", ", ", ", ")
        },
        book: "⟦{names}. ⟧“{title}.”⟦ {publisher},⟧ {year}.⟦ {url}⟧",
        article: "⟦{names}. ⟧“{title}.” {journal}⟦, {volume}⟧⟦, {pages}⟧, {year}.⟦ {url}⟧",
    },
    Style {
        name: "SIST02",
        aliases: &["sist"],
        cite: Cite::Number { open: "(", close: ")", pages_sep: Some(", p. ") },
        and: "and",
        order: Order::Cited,
        et_al_from: 3,
        bib_title: "References",
        names: names(Name::LastFirst, Name::LastFirst, "; ", "; ", "; "),
        book: "{n}) ⟦{names}. ⟧{title}.⟦ {city},⟧⟦ {publisher},⟧ {year}.⟦ {url}⟧",
        article: "{n}) ⟦{names}. ⟧{title}. {journal}. {year}⟦, vol. {volume}⟧⟦, p. {pages}⟧.⟦ {url}⟧",
    },
    Style {
        name: "Turabian",
        aliases: &["turabian"],
        cite: Cite::AuthorDate { year_sep: " ", pages_sep: ", " },
        and: "and",
        order: Order::Author,
        et_al_from: 4,
        bib_title: "Bibliography",
        names: names(Name::LastFirst, Name::FirstLast, ", ", ", and ", ", and "),
        book: "⟦{names}. ⟧{title}.⟦ {pub},⟧ {year}.⟦ {url}.⟧",
        article: "⟦{names}. ⟧“{title}.” {journal}⟦ {volume}⟧ ({year})⟦: {pages}⟧.⟦ {url}.⟧",
    },
];

/// The entry of a **book**, as the GOST writes it.
const GOST_BOOK: &str = "{n}. ⟦{names} ⟧{title}. –⟦ {city} :⟧⟦ {publisher},⟧ {year}.⟦ – URL: {url}.⟧";

/// The entry of an **article**, as the GOST writes it.
const GOST_ARTICLE: &str =
    "{n}. ⟦{names} ⟧{title} // {journal}. – {year}.⟦ – Vol. {volume}.⟧⟦ – P. {pages}.⟧⟦ – URL: {url}.⟧";

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
pub fn names_of_styles() -> Vec<&'static str> {
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

/// The **initials** of a first name: `Alex Mei` is `A. M.`, or `A M`
/// without the dots.
fn initials_of(first: &str, dots: bool) -> String {
    let separator = if dots { ". " } else { " " };
    let mut out = first
        .split_whitespace()
        .filter_map(|word| word.chars().next().map(String::from))
        .collect::<Vec<String>>()
        .join(separator);
    if dots && !out.is_empty() {
        out.push('.');
    }
    out
}

/// One author's name, in the form of a style.
fn name_of(form: Name, last: &str, first: &str) -> String {
    let join = |a: String, separator: &str, b: String| {
        if a.is_empty() || b.is_empty() { format!("{a}{b}") } else { format!("{a}{separator}{b}") }
    };
    match form {
        Name::LastFirst => join(last.to_string(), ", ", first.to_string()),
        Name::FirstLast => join(first.to_string(), " ", last.to_string()),
        Name::LastInitials => join(last.to_string(), ", ", initials_of(first, true)),
        Name::InitialsLast => join(initials_of(first, true), " ", last.to_string()),
        Name::LastSpaceInitials => join(last.to_string(), " ", initials_of(first, true)),
        Name::UpperLastFirst => join(last.to_uppercase(), ", ", first.to_string()),
        Name::UpperLastBareInitials => join(last.to_uppercase(), " ", initials_of(first, false)),
    }
}

/// The **author list** of an entry, as the style writes it.
///
/// The tree's own rules: at most so many authors are written, and the
/// rest are left out with the style's **et al.**; two authors are
/// joined by one word, and three or more by another.
fn name_list_of(authors: &[(String, String)], names: &Names) -> String {
    let shown = if names.max > 0 { authors.get(..names.max).unwrap_or(authors) } else { authors };
    let parts: Vec<String> = shown
        .iter()
        .enumerate()
        .map(|(at, (last, first))| name_of(if at == 0 { names.first } else { names.rest }, last, first))
        .collect();
    let mut out = match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a}{}{b}", names.two),
        [init @ .., last] => format!("{}{}{last}", init.join(names.sep), names.last),
    };
    if shown.len() < authors.len() {
        out.push_str(names.et_al);
    }
    out
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

/// The **type code** of a source, as the GB/T 7714 writes it: `M` a
/// book, `J` a journal article, `R` a report, `EB/OL` a web page and
/// `Z` anything else. `/OL` marks a source that was read online.
fn type_of(source: &Json) -> String {
    let kind = source.get("kind").and_then(|kind| kind.as_str()).unwrap_or_default().to_ascii_lowercase();
    let journal = source.get("journal").and_then(|journal| journal.as_str()).unwrap_or_default();
    let url = source.get("url").and_then(|url| url.as_str()).unwrap_or_default();
    let code = match kind.as_str() {
        "website" | "web" => return "EB/OL".to_string(),
        _ if !journal.is_empty() => "J",
        "book" | "" => "M",
        "article" => "J",
        "report" => "R",
        _ => "Z",
    };
    if url.is_empty() { code.to_string() } else { format!("{code}/OL") }
}

/// The **entry** of a source, in a style.
///
/// The tree's own rules: a source with a **journal** is written as an
/// article and every other source as a book; the **year** of a source
/// with no year is `n.d.`; and the text between `⟦` and `⟧` is left
/// out when a field in it is empty.
pub fn entry_of(source: &Json, style: &Style, number: usize) -> String {
    let part = |key: &str| source.get(key).and_then(|value| value.as_str()).unwrap_or_default().to_string();
    let author = part("author");
    let city = part("city");
    let publisher = part("publisher");
    let authors = authors_of(&author);
    let template = if part("journal").is_empty() { style.book } else { style.article };
    let field = |key: &str| -> String {
        match key {
            "n" => number.to_string(),
            "names" => name_list_of(&authors, &style.names),
            "year" => match source.get("year").and_then(|year| year.as_str()) {
                Some(year) if !year.is_empty() => year.to_string(),
                _ => "n.d.".to_string(),
            },
            "title" => part("title"),
            "city" => city.clone(),
            "publisher" => publisher.clone(),
            "pub" => [city.as_str(), publisher.as_str()]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<&str>>()
                .join(": "),
            "journal" => part("journal"),
            "volume" => part("volume"),
            "pages" => part("pages"),
            "url" => part("url"),
            "type" => type_of(source),
            _ => String::new(),
        }
    };
    render(template, &field)
}

/// Append text, dropping a leading full stop when the text already
/// ends with one: `M.` and `. ` make `M. ` and not `M.. `.
fn push_text(out: &mut String, text: &str) {
    match text.strip_prefix('.') {
        Some(rest) if out.ends_with('.') => out.push_str(rest),
        _ => out.push_str(text),
    }
}

/// Fill one segment of a template; `None` when it is optional and a
/// field in it is empty.
fn fill_segment(segment: &str, field: &dyn Fn(&str) -> String, optional: bool) -> Option<String> {
    let mut out = String::new();
    let mut rest = segment;
    while let Some((before, after)) = rest.split_once('{') {
        push_text(&mut out, before);
        let Some((key, after)) = after.split_once('}') else {
            push_text(&mut out, after);
            return Some(out);
        };
        let value = field(key);
        if value.is_empty() && optional {
            return None;
        }
        push_text(&mut out, &value);
        rest = after;
    }
    push_text(&mut out, rest);
    Some(out)
}

/// Fill a template: the text between `⟦` and `⟧` is left out when a
/// field in it is empty, and `{name}` is the field of that name.
fn render(template: &str, field: &dyn Fn(&str) -> String) -> String {
    let mut out = String::new();
    let mut rest = template;
    while !rest.is_empty() {
        let (segment, optional, after) = match rest.strip_prefix('⟦') {
            Some(group) => match group.split_once('⟧') {
                Some((group, after)) => (group, true, after),
                None => (group, true, ""),
            },
            None => match rest.split_once('⟦') {
                Some((plain, _)) => (plain, false, rest.get(plain.len()..).unwrap_or("")),
                None => (rest, false, ""),
            },
        };
        if let Some(text) = fill_segment(segment, field, optional) {
            push_text(&mut out, &text);
        }
        rest = after;
    }
    out
}

/// The **order** of a bibliography: the sources that are cited, or
/// every source while nothing is cited, sorted the way the style asks.
///
/// The tree's own rules: a style that orders by the author sorts by
/// the surname of the first author — and by the title when the source
/// has no author — and a style that orders by the title sorts by the
/// title.
pub fn order_of(sources: &[Json], cited: &[String], order: Order) -> Vec<Json> {
    let mut chosen: Vec<Json> = if cited.is_empty() {
        sources.to_vec()
    } else {
        cited
            .iter()
            .filter_map(|tag| {
                sources.iter().find(|source| {
                    source.get("tag").and_then(|held| held.as_str()).map(|held| held == tag).unwrap_or(false)
                })
            })
            .cloned()
            .collect()
    };
    if order != Order::Cited {
        chosen.sort_by_key(|source| match order {
            Order::Title => {
                source.get("title").and_then(|title| title.as_str()).unwrap_or_default().to_lowercase()
            }
            _ => {
                let authors =
                    authors_of(source.get("author").and_then(|author| author.as_str()).unwrap_or_default());
                match authors.first() {
                    Some((last, _)) => last.to_lowercase(),
                    None => source
                        .get("title")
                        .and_then(|title| title.as_str())
                        .unwrap_or_default()
                        .to_lowercase(),
                }
            }
        });
    }
    chosen
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
                    names_of_styles().join(", ")
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
