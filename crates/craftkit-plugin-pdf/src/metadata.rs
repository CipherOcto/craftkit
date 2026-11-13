//! The **XMP metadata** of a file.
//!
//! A PDF file may say what it is in two places: the **information
//! dictionary** — who wrote it, what it is called — and the **XMP
//! metadata**, a packet of XML that says what the file is and what it
//! follows.
//!
//! The packet is built from the information the file already has, and
//! it says the **level** the file is: `PDF/A-2b` or `PDF/A-3b`.
//!
//! The rules:
//!
//! - the packet is **uncompressed**, so that another tool may find it
//!   in the file without decoding anything;
//! - the **dates** are written the way XMP writes them —
//!   `2024-01-05T09:30:00Z` — and a date that is not a date is left
//!   out;
//! - the **words** are escaped, so that a title with an ampersand in
//!   it does not break the packet.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, EngineError, Node, Outcome, Text};

use crate::preflight::Level;

/// The **information** of the file: the object that says who wrote it.
///
/// The information node names the **node** of the object, the way the
/// file's own writer reads it.
fn the_information(document: &Document) -> Option<&Node> {
    crate::information::the_information(document)
}

/// The **text** an entry of the information holds.
fn information_of(document: &Document, key: &str) -> Option<String> {
    let entries = crate::entries_of(&the_information(document)?.data);
    let (_, value) = entries.into_iter().find(|(name, _)| name == key)?;
    let text = crate::fields::text_of(&value)?;
    (!text.trim().is_empty()).then_some(text)
}

/// The **text** of a value, escaped for the packet.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A PDF date (`D:YYYYMMDDHHmmSSOHH'mm'`) as an XMP date.
///
/// The date is ISO 8601 — `2024-01-05T09:30:00Z` — and a text that is
/// not a date has no date to write.
pub fn iso_date(date: &str) -> Option<String> {
    let trimmed = date.trim();
    let trimmed = trimmed.strip_prefix("D:").unwrap_or(trimmed);
    let digits: String = trimmed.chars().take_while(|character| character.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return None;
    }
    let part =
        |at: usize, to: usize, default: &str| -> String { digits.get(at..to).unwrap_or(default).to_string() };
    let year = part(0, 4, "0000");
    let month = part(4, 6, "01");
    let day = part(6, 8, "01");
    let hour = part(8, 10, "00");
    let minute = part(10, 12, "00");
    let second = part(12, 14, "00");
    // The **zone**: `Z`, `+HH'mm'` or `-HH'mm'`.
    let rest = &trimmed[digits.len()..];
    let zone = match rest.chars().next() {
        Some('Z') | None => "Z".to_string(),
        Some(sign @ ('+' | '-')) => {
            let zone: String = rest[1..].chars().filter(|character| character.is_ascii_digit()).collect();
            format!("{sign}{}:{}", zone.get(0..2).unwrap_or("00"), zone.get(2..4).unwrap_or("00"))
        }
        _ => "Z".to_string(),
    };
    Some(format!("{year}-{month}-{day}T{hour}:{minute}:{second}{zone}"))
}

/// The **XMP packet** of the file: what the file says about itself.
///
/// The packet carries the **identification** — the level the file is —
/// and the **information** the file has: its title, its author, its
/// subject, its keywords, the tool that made it and when.
pub fn packet(document: &Document, level: Level) -> String {
    let mut properties = String::new();
    if let Some(title) = information_of(document, "Title") {
        properties.push_str(&format!(
            "   <dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>\n",
            escape(&title)
        ));
    }
    if let Some(author) = information_of(document, "Author") {
        properties.push_str(&format!(
            "   <dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>\n",
            escape(&author)
        ));
    }
    if let Some(subject) = information_of(document, "Subject") {
        properties.push_str(&format!(
            "   <dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>\n",
            escape(&subject)
        ));
    }
    if let Some(keywords) = information_of(document, "Keywords") {
        properties.push_str(&format!("   <pdf:Keywords>{}</pdf:Keywords>\n", escape(&keywords)));
    }
    if let Some(producer) = information_of(document, "Producer") {
        properties.push_str(&format!("   <pdf:Producer>{}</pdf:Producer>\n", escape(&producer)));
    }
    if let Some(creator) = information_of(document, "Creator") {
        properties.push_str(&format!("   <xmp:CreatorTool>{}</xmp:CreatorTool>\n", escape(&creator)));
    }
    if let Some(date) = information_of(document, "CreationDate").and_then(|date| iso_date(&date)) {
        properties.push_str(&format!("   <xmp:CreateDate>{date}</xmp:CreateDate>\n"));
    }
    if let Some(date) = information_of(document, "ModDate").and_then(|date| iso_date(&date)) {
        properties.push_str(&format!("   <xmp:ModifyDate>{date}</xmp:ModifyDate>\n"));
    }
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\"\n\
    xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\
    xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\"\n\
    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n\
    xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n\
   <pdfaid:part>{}</pdfaid:part>\n\
   <pdfaid:conformance>B</pdfaid:conformance>\n\
{properties}  </rdf:Description>\n\
 </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>",
        level.part()
    )
}

commands! {
    /// **Write** the **XMP metadata** of a file.
    ///
    /// The packet is built from the information the file has, and it is
    /// written to a **file** of its own — the bytes of a stream are bytes
    /// in a file, and that is the truth here too. The catalog then names
    /// it, and the file says what it is.
    ///
    /// The rules:
    ///
    /// - a path that is **empty** is refused;
    /// - a level that is **not a level** is refused;
    /// - a file with **no catalog** is refused: there is nothing to name
    ///   the metadata.
    pdf.metadata.write {
        area: "pdf",
        layer: "pdf",
        icon: "file",
        mutates,
        undoable,
        params: {
            /// Where the packet's **bytes** are written.
            path: Text,
            /// The level the file is: `a2b` or `a3b`.
            level: Text,
        },
        run: {
            let id = "pdf.metadata.write".to_string();
            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the packet's bytes go somewhere".to_string(),
                });
            }
            let level = Level::from_id(&self.level.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a level: the levels are a2b, a3b", self.level.0.trim()),
            })?;
            let Some(catalog) = crate::preflight::the_catalog(session.document()).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the file has no catalog".to_string(),
                });
            };
            // The **packet**: what the file says about itself.
            let packet = packet(session.document(), level);
            // The **bytes**: they go in a file of their own.
            if let Some(parent) = std::path::Path::new(&path).parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(|error| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the packet's bytes have nowhere to go: {error}"),
                })?;
            }
            std::fs::write(&path, packet.as_bytes()).map_err(|error| EngineError::Failed {
                id: id.clone(),
                message: format!("the packet's bytes cannot be written: {error}"),
            })?;
            // The **object**: the metadata, uncompressed.
            let number = crate::the_next_number(session.document(), 0);
            let entries = json!([["Type", "Metadata"], ["Subtype", "XML"]]);
            let data = json!({ "object": number, "entries": entries });
            let metadata =
                session.document_mut()?.add_node_with(format!("metadata {number}"), "pdf.stream", None, data);
            if let Some(node) = session.document_mut()?.node_mut(metadata) {
                node.stream = Some(path.clone());
            }
            // The **catalog** names the metadata.
            let mut catalog_entries = crate::entries_of(&catalog.data);
            let value = json!({ "ref": [number, 0] });
            match catalog_entries.iter_mut().find(|(key, _)| key == "Metadata") {
                Some(entry) => entry.1 = value,
                None => catalog_entries.push(("Metadata".to_string(), value)),
            }
            let data = json!({
                "object": catalog.data.get("object").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null),
                "entries": catalog_entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
            });
            if let Some(node) = session.document_mut()?.node_mut(catalog.id) {
                node.data = data;
            }
            // And the **edge**: the catalog names the metadata.
            let payload = json!({ "key": "Metadata", "generation": 0 });
            let relation = craftkit_sdk::Relation::with_payload(catalog.id, metadata, "ref", payload);
            session.document_mut()?.add_relation(relation);
            Ok(Outcome::changed(format!("the file says it is {}", level.label())).with_data(json!({
                "id": metadata,
                "object": number,
                "path": path,
                "level": level.label(),
                "part": level.part(),
            })))

        }
    }
}
