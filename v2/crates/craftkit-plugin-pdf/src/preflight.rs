//! The **preflight** of a PDF file.
//!
//! A preflight says what a file **declares** about itself and what it
//! **breaks**: the tree's PDF/A check, in the shape the tree keeps it.
//!
//! - what the file declares: the **PDF/A part** and its conformance,
//!   the **PDF/UA part**, and the **output intents** it names;
//! - what the file breaks: the rules of the tree's own table, each with
//!   the **clause** it breaks, the **message** that says what is
//!   wrong, and whether the tree can **fix** it.
//!
//! The rules that are here are the ones this tree can see in the
//! **document**: the metadata and what it says, the fonts and whether
//! they are embedded, the colour the file paints in and the output
//! intent it needs for it, the actions a PDF/A file may not have, the
//! annotations it may not have, and the additional actions it may not
//! have. The rule about encryption is not here — the tree checks it
//! against the file, and this tree has a document and not a file.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome, Session, Text};

/// The actions a PDF/A file may **not** have: the tree's own list.
const FORBIDDEN_ACTIONS: [&str; 11] = [
    "JavaScript",
    "Launch",
    "Sound",
    "Movie",
    "ResetForm",
    "ImportData",
    "Hide",
    "SetOCGState",
    "Rendition",
    "Trans",
    "GoTo3DView",
];

/// The annotations a PDF/A file may **not** have: the tree's own list.
const FORBIDDEN_ANNOTS: [&str; 5] = ["Sound", "Movie", "Screen", "3D", "RichMedia"];

/// The PDF/A part and conformance level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// PDF/A-2b.
    A2b,
    /// PDF/A-3b.
    A3b,
}

impl Level {
    /// The **part** of the level: 2 or 3.
    pub fn part(self) -> u8 {
        match self {
            Level::A2b => 2,
            Level::A3b => 3,
        }
    }

    /// The **label** of the level: `PDF/A-2b`.
    pub fn label(self) -> &'static str {
        match self {
            Level::A2b => "PDF/A-2b",
            Level::A3b => "PDF/A-3b",
        }
    }

    /// The level a name says, if it says one: `2b`, `a2b` or `pdfa2b`.
    pub fn from_id(id: &str) -> Option<Level> {
        match id.trim().to_ascii_lowercase().replace(['/', '-', ' '], "").as_str() {
            "pdfa2b" | "a2b" | "2b" => Some(Level::A2b),
            "pdfa3b" | "a3b" | "3b" => Some(Level::A3b),
            _ => None,
        }
    }
}

/// A rule a document **breaks**.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    /// The clause of the standard.
    pub clause: &'static str,
    /// What is wrong, in words.
    pub message: String,
    /// Whether the tree can **fix** it.
    pub fixable: bool,
}

/// What a document **declares** about the standards it follows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Declared {
    /// The PDF/A part and its conformance: `(2, "B")`.
    pub pdfa: Option<(u8, String)>,
    /// The PDF/UA part.
    pub pdfua: Option<u8>,
    /// The identifiers of the output intents.
    pub output_intents: Vec<String>,
}

/// The **value** of a simple XMP property, written either as an
/// attribute (`pdfaid:part="2"`) or as an element
/// (`<pdfaid:part>2</pdfaid:part>`).
pub fn xmp_value(xmp: &str, key: &str) -> Option<String> {
    let attribute = format!("{key}=\"");
    if let Some(at) = xmp.find(&attribute) {
        let rest = &xmp[at + attribute.len()..];
        return rest.find('"').map(|end| unescape(&rest[..end]));
    }
    let attribute = format!("{key}='");
    if let Some(at) = xmp.find(&attribute) {
        let rest = &xmp[at + attribute.len()..];
        return rest.find('\'').map(|end| unescape(&rest[..end]));
    }
    let open = format!("<{key}>");
    let close = format!("</{key}>");
    let start = xmp.find(&open)? + open.len();
    let end = start + xmp[start..].find(&close)?;
    let inner = &xmp[start..end];
    // `rdf:Alt` and `rdf:Seq`: the first `rdf:li`.
    if let Some(at) = inner.find("<rdf:li") {
        let start = at + inner[at..].find('>')? + 1;
        let end = start + inner[start..].find("</rdf:li>")?;
        return Some(unescape(inner[start..end].trim()));
    }
    Some(unescape(inner.trim()))
}

/// The **text** of an XMP value, with the escapes undone.
fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// The **entries** of a node's dictionary: the key and the value, in
/// the order the file had them.
pub fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
pub fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **name** an entry says: the value of a name entry is a text.
pub fn name_of(entries: &[(String, Json)], key: &str) -> Option<String> {
    entry(entries, key)?.as_str().map(|text| text.to_string())
}

/// The **catalog** of the document: the object that says it is one.
pub fn the_catalog(document: &Document) -> Option<&Node> {
    document.walk().find(|node| {
        node.kind == "pdf.object"
            && entries_of(node).iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
    })
}

/// The **object number** a node holds.
fn number_of(node: &Node) -> Option<u64> {
    node.data.get("object").and_then(|object| object.as_u64())
}

/// The **node** a reference names: the node with that object number.
pub fn node_of<'a>(document: &'a Document, value: &Json) -> Option<&'a Node> {
    let number = value.get("ref").and_then(|pair| pair.as_array())?.first()?;
    let number = number.as_u64()?;
    document.walk().find(|node| number_of(node) == Some(number))
}

/// The **metadata** of the document: the XMP the catalog names.
fn metadata_of(document: &Document) -> Option<String> {
    let catalog = the_catalog(document)?;
    let entries = entries_of(catalog);
    let stream = node_of(document, entry(&entries, "Metadata")?)?;
    let decoded = crate::decoded_bytes(stream, None, None, false, "pdf.preflight").ok()?;
    Some(String::from_utf8_lossy(&decoded.bytes).into_owned())
}

/// What the document **declares** about the standards it follows.
pub fn declared(document: &Document) -> Declared {
    let mut declared = Declared::default();
    if let Some(xmp) = metadata_of(document) {
        let part = xmp_value(&xmp, "pdfaid:part").and_then(|part| part.trim().parse::<u8>().ok());
        let conformance =
            xmp_value(&xmp, "pdfaid:conformance").unwrap_or_default().trim().to_ascii_uppercase();
        declared.pdfa = part.map(|part| (part, conformance));
        declared.pdfua = xmp_value(&xmp, "pdfuaid:part").and_then(|part| part.trim().parse::<u8>().ok());
    }
    if let Some(catalog) = the_catalog(document) {
        let entries = entries_of(catalog);
        if let Some(intents) = entry(&entries, "OutputIntents").and_then(|intents| intents.as_array()) {
            for intent in intents {
                let Some(node) = node_of(document, intent) else { continue };
                let identifier = name_of(&entries_of(node), "OutputConditionIdentifier").unwrap_or_default();
                declared.output_intents.push(identifier);
            }
        }
    }
    declared
}

/// Whether an object is an **action**: the tree's own rule.
pub fn is_action(entries: &[(String, Json)]) -> bool {
    match name_of(entries, "Type") {
        Some(kind) => kind == "Action",
        None => !(entry(entries, "P").is_some() && entry(entries, "K").is_some()),
    }
}

/// The **device colour** a document paints in: whether it paints in
/// red, green and blue or in gray, and whether it paints in CMYK.
///
/// The colour is said in two ways: a dictionary that names a **device
/// colour space** says it outright, and a **content stream** says it
/// with the operators that set a colour — `rg` and `RG` for red,
/// green and blue, `g` and `G` for gray, `k` and `K` for CMYK.
fn device_colour(document: &Document) -> (bool, bool) {
    let mut rgb_or_gray = false;
    let mut cmyk = false;
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        let entries = entries_of(node);
        if let Some(space) = name_of(&entries, "ColorSpace") {
            match space.as_str() {
                "DeviceRGB" | "DeviceGray" => rgb_or_gray = true,
                "DeviceCMYK" => cmyk = true,
                _ => {}
            }
        }
        if node.kind != "pdf.stream" {
            continue;
        }
        // A **content stream** is a stream that is not an image and
        // not a form: it holds the instructions of a page.
        let kind = name_of(&entries, "Type");
        let subtype = name_of(&entries, "Subtype");
        let is_content = kind.is_none() && (subtype.is_none() || subtype.as_deref() == Some("Form"));
        if !is_content {
            continue;
        }
        let Ok(decoded) = crate::decoded_bytes(node, None, None, false, "pdf.preflight") else {
            continue;
        };
        let Ok(operations) = crate::content::operations(&decoded.bytes) else { continue };
        for operation in operations {
            match operation.operator.as_str() {
                "rg" | "RG" | "g" | "G" => rgb_or_gray = true,
                "k" | "K" => cmyk = true,
                _ => {}
            }
        }
    }
    (rgb_or_gray, cmyk)
}

/// Whether the document has a **PDF/A output intent**: an intent of
/// the subtype `GTS_PDFA1` that names a profile.
fn has_a_pdfa_intent(document: &Document) -> bool {
    let Some(catalog) = the_catalog(document) else { return false };
    let entries = entries_of(catalog);
    let Some(intents) = entry(&entries, "OutputIntents").and_then(|intents| intents.as_array()) else {
        return false;
    };
    intents.iter().any(|intent| {
        let Some(node) = node_of(document, intent) else { return false };
        let entries = entries_of(node);
        // The **subtype** of the intent is `/S`, the name the file
        // uses — not `/Subtype`, which no file has.
        name_of(&entries, "S").as_deref() == Some("GTS_PDFA1")
            && entry(&entries, "DestOutputProfile").is_some()
    })
}

/// The **action** a dictionary holds, when it is one that a PDF/A
/// file may not have.
///
/// The answer is the name of the action — `JavaScript`, `Launch` and
/// the rest of the tree's own list — and nothing when the dictionary
/// is not one of them.
pub fn forbidden_action(entries: &[(String, Json)]) -> Option<String> {
    let action = name_of(entries, "S")?;
    if FORBIDDEN_ACTIONS.contains(&action.as_str()) && is_action(entries) { Some(action) } else { None }
}

/// The **issues** the document has: the rules it breaks.
pub fn issues(document: &Document, level: Option<Level>) -> Vec<Issue> {
    let mut issues: Vec<Issue> = Vec::new();
    let mut issue = |clause: &'static str, message: String, fixable: bool| {
        issues.push(Issue { clause, message, fixable });
    };
    // The **metadata**: the file must have it, and it must say what
    // the file is.
    match metadata_of(document) {
        None => issue("6.6.2.1", "The document has no XMP metadata".to_string(), true),
        Some(xmp) => {
            if let Some(level) = level {
                let part = xmp_value(&xmp, "pdfaid:part");
                let conformance =
                    xmp_value(&xmp, "pdfaid:conformance").map(|value| value.trim().to_ascii_uppercase());
                if part.as_deref() != Some(level.part().to_string().as_str())
                    || conformance.as_deref() != Some("B")
                {
                    issue(
                        "6.6.4",
                        format!("The metadata doesn't identify the file as {}", level.label()),
                        true,
                    );
                }
            }
        }
    }
    // The **fonts**: a PDF/A file must have every font embedded.
    for font in crate::fonts::fonts(document) {
        if !font.embedded {
            issue("6.2.11.4", format!("The font {} isn't embedded", font.base), false);
        }
    }
    // The **encryption**: a PDF/A file may not be encrypted. The tree
    // can take the encryption away, because it decrypts the file when
    // it opens it; this tree cannot, so the rule is not one it can
    // fix.
    if crate::trailer::is_encrypted(document) {
        issue("6.1.3", "The document is encrypted".to_string(), false);
    }
    // The **form**: it may not ask the viewer to make the
    // appearances.
    if let Some(catalog) = the_catalog(document) {
        let entries = entries_of(catalog);
        if let Some(form) = entry(&entries, "AcroForm") {
            // The form is a dictionary of its own: either an object of
            // the file, or a dictionary written inside the catalog.
            let form = match node_of(document, form) {
                Some(node) => entries_of(node),
                None => crate::entries_of(form),
            };
            if form.iter().any(|(key, value)| key == "NeedAppearances" && value == &Json::Bool(true)) {
                issue(
                    "6.4.1",
                    "The form asks viewers to make appearances (NeedAppearances)".to_string(),
                    true,
                );
            }
        }
    }
    // The **colour**: device colour needs a PDF/A output intent.
    let (rgb, cmyk) = device_colour(document);
    if (rgb || cmyk) && !has_a_pdfa_intent(document) {
        issue("6.2.3", "Device colour is used, but there is no PDF/A output intent".to_string(), !cmyk);
        if cmyk {
            issue(
                "6.2.4.3",
                "DeviceCMYK is used: it needs a CMYK output intent (not added automatically)".to_string(),
                false,
            );
        }
    }
    // The **objects**: the actions and the annotations a PDF/A file
    // may not have, and the additional actions it may not have.
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        let entries = entries_of(node);
        // An **action** that is not allowed.
        if let Some(action) = name_of(&entries, "S")
            && FORBIDDEN_ACTIONS.contains(&action.as_str())
            && is_action(&entries)
        {
            issue("6.5.1", format!("A {action} action isn't allowed"), true);
        }
        // An **annotation** that is not allowed.
        let kind = name_of(&entries, "Type");
        let subtype = name_of(&entries, "Subtype");
        let is_annotation = kind.as_deref() == Some("Annot")
            || (subtype.is_some() && entry(&entries, "Rect").is_some() && entry(&entries, "P").is_some());
        if is_annotation
            && let Some(subtype) = subtype.as_deref()
            && FORBIDDEN_ANNOTS.contains(&subtype)
        {
            issue("6.3.1", format!("{subtype} annotations aren't allowed"), true);
            continue;
        }
        // **Additional actions**: the document, a page or an
        // annotation may not have them.
        if entry(&entries, "AA").is_some() {
            let what = if is_annotation {
                format!("A {subtype:?} annotation has additional actions")
            } else {
                "The document has additional actions (/AA)".to_string()
            };
            issue("6.5.2", what, true);
        }
    }
    issues
}

/// **Check** a PDF file: what it declares and what it breaks.
#[derive(Command)]
#[command(id = "pdf.preflight", area = "pdf", layer = "pdf", icon = "check")]
pub struct Preflight {
    /// The level to check against: `2b` or `3b`.
    ///
    /// Without it, the rules that need a level are not checked.
    pub level: Option<Text>,
}

impl Preflight {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.preflight".to_string();
        let level = match &self.level {
            None => None,
            Some(level) => Some(Level::from_id(&level.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a level: a level is `2b` or `3b`", level.0.trim()),
            })?),
        };
        let document = session.document();
        let declared = declared(document);
        let issues = issues(document, level);
        let (rgb, cmyk) = device_colour(document);
        let count = issues.len();
        Ok(Outcome::new(format!("the file breaks {count} rule(s)")).with_data(json!({
            "pdfa": match &declared.pdfa {
                Some((part, conformance)) => json!({ "part": part, "conformance": conformance }),
                None => Json::Null,
            },
            "pdfua": match declared.pdfua {
                Some(part) => json!(part),
                None => Json::Null,
            },
            "output_intents": declared.output_intents.clone(),
            "fonts": crate::fonts::fonts(document).iter().map(crate::fonts::Font::to_json).collect::<Vec<_>>(),
            "colour": {
                "rgb": rgb,
                "cmyk": cmyk,
            },
            "issues": issues
                .iter()
                .map(|issue| json!({
                    "clause": issue.clause,
                    "message": issue.message.clone(),
                    "fixable": issue.fixable,
                }))
                .collect::<Vec<_>>(),
        })))
    }
}
