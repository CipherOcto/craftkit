//! The **encryption** of a PDF file.
//!
//! A file whose trailer names an **encryption** is a file whose strings
//! and streams are scrambled: a person needs a password to open it,
//! and the file says what the password lets them do.
//!
//! The **encryption dictionary** says:
//!
//! - the **filter** — the standard one — and the **version** and the
//!   **revision** of the scheme;
//! - the **key length**, in bits, and forty when the file does not
//!   say;
//! - the **crypt filters**: the names of the ways the strings and the
//!   streams are scrambled, and the method each one names;
//! - the **permissions**: the bits that say what a person may do with
//!   the file;
//! - whether the **metadata** is encrypted as well, which it is
//!   unless the file says otherwise.
//!
//! The **rules**:
//!
//! - a file that names no encryption is **not encrypted**, and the
//!   answer says so;
//! - the **permissions** are read from the bits the file holds, and
//!   the bits are numbered from the right, as the format numbers
//!   them;
//! - a file whose version is below four is scrambled with **RC4**,
//!   whatever the crypt filters say.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session, Text};

/// The **encryption** of a file: what its dictionary says.
#[derive(Debug, Clone, PartialEq)]
pub struct Encryption {
    /// The name of the filter: the standard one.
    pub filter: String,
    /// The version of the scheme.
    pub version: i64,
    /// The revision of the scheme.
    pub revision: i64,
    /// The length of the key, in bits. Forty when the file does not
    /// say.
    pub length_bits: i64,
    /// The bits that say what a person may do with the file.
    pub permissions: i64,
    /// Whether the metadata is encrypted as well.
    pub encrypt_metadata: bool,
    /// The way the **strings** are scrambled.
    pub strings: String,
    /// The way the **streams** are scrambled.
    pub streams: String,
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **name** an entry holds: the value of a name entry is a text.
fn name_of(entries: &[(String, Json)], key: &str) -> Option<String> {
    entry(entries, key)?.as_str().map(|text| text.to_string())
}

/// The **number** an entry holds.
fn number_of(entries: &[(String, Json)], key: &str) -> Option<i64> {
    crate::whole_number_of(entry(entries, key)?)
}

/// The **object number** a reference names.
fn reference_of(value: &Json) -> Option<u64> {
    crate::number_of(value)
}

/// The **crypt filters** of the encryption: the name of each and the
/// method it names.
fn crypt_filters(entries: &[(String, Json)]) -> Vec<(String, String)> {
    let Some(filters) = entry(entries, "CF") else { return Vec::new() };
    let Some(list) = filters.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|pair| {
            let pair = pair.as_array()?;
            let name = pair.first()?.as_str()?.to_string();
            let body = pair.get(1)?;
            let body = body.get("entries").and_then(|entries| entries.as_array())?;
            let method = body.iter().find_map(|entry| {
                let entry = entry.as_array()?;
                if entry.first()?.as_str()? != "CFM" {
                    return None;
                }
                Some(entry.get(1)?.as_str()?.to_string())
            });
            Some((name, method.unwrap_or_default()))
        })
        .collect()
}

/// The **method** a name says, in this tree's own words.
fn method_of(name: &str) -> String {
    match name {
        "V2" => "rc4".to_string(),
        "AESV2" => "aes-128".to_string(),
        "AESV3" => "aes-256".to_string(),
        _ => "none".to_string(),
    }
}

/// The **encryption** of a dictionary: what it says.
pub fn encryption_from(entries: &[(String, Json)]) -> Encryption {
    let filters = crypt_filters(entries);
    let named = |name: &str| -> String {
        if name.is_empty() || name == "Identity" {
            return "none".to_string();
        }
        filters
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, method)| method_of(method))
            .unwrap_or_else(|| "none".to_string())
    };
    let version = number_of(entries, "V").unwrap_or(0);
    // A file whose version is below four is scrambled with **RC4**,
    // whatever the crypt filters say.
    let (strings, streams) = if version < 4 {
        ("rc4".to_string(), "rc4".to_string())
    } else {
        (
            named(&name_of(entries, "StrF").unwrap_or_default()),
            named(&name_of(entries, "StmF").unwrap_or_default()),
        )
    };
    Encryption {
        filter: name_of(entries, "Filter").unwrap_or_default(),
        version,
        revision: number_of(entries, "R").unwrap_or(0),
        length_bits: number_of(entries, "Length").unwrap_or(40),
        permissions: number_of(entries, "P").unwrap_or(0),
        encrypt_metadata: entry(entries, "EncryptMetadata").and_then(|value| value.as_bool()).unwrap_or(true),
        strings,
        streams,
    }
}

/// The **permissions** of a file: what a person may do with it.
///
/// The bits are numbered from the right, as the format numbers them,
/// and two of the permissions are the same as another one: filling in
/// the forms of a file is the same right as modifying its annotations,
/// and assembling a file is the same right as modifying its contents.
pub fn permissions_of(bits: i64) -> Vec<&'static str> {
    let bit = |number: u32| bits >> (number - 1) & 1 == 1;
    let mut names: Vec<&'static str> = Vec::new();
    if bit(3) {
        names.push("print");
    }
    if bit(4) {
        names.push("modify");
    }
    if bit(5) {
        names.push("copy");
    }
    if bit(6) {
        names.push("annotate");
    }
    if bit(9) || bit(6) {
        names.push("fill-forms");
    }
    if bit(10) {
        names.push("extract-for-accessibility");
    }
    if bit(11) || bit(4) {
        names.push("assemble");
    }
    if bit(12) {
        names.push("print-high-quality");
    }
    names
}

/// **Read** the **encryption** of a PDF file.
///
/// A query: it does not change the document. A file that names no
/// encryption is not encrypted, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.encrypt.read", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadEncryption {
    /// The entry to read.
    ///
    /// Leave it out and everything the encryption says is read.
    pub key: Option<Text>,
}

impl ReadEncryption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.encrypt.read".to_string();
        let document = session.document();
        let Some(value) = crate::trailer::entry_of(document, "Encrypt") else {
            return Ok(Outcome::new("the file is not encrypted").with_data(json!({
                "encrypted": false,
            })));
        };
        let Some(number) = reference_of(&value) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the file names its encryption, and it names it in no way".to_string(),
            });
        };
        let Some(node) = document
            .walk()
            .find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
        else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the file names the encryption of the object `{number}`, and that object is not in it"
                ),
            });
        };
        let entries = crate::entries_of(&node.data);
        let encryption = encryption_from(&entries);
        let permissions = permissions_of(encryption.permissions);
        // The **entries**: the dictionary of the encryption, or the
        // one that was asked for.
        let listed: Vec<Json> = match &self.key {
            None => entries.iter().map(|(key, value)| json!([key.clone(), value.clone()])).collect(),
            Some(key) => {
                let key = key.0.trim();
                let one: Vec<Json> = entries
                    .iter()
                    .filter(|(name, _)| name == key)
                    .map(|(key, value)| json!([key.clone(), value.clone()]))
                    .collect();
                if one.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the encryption has no entry `{key}`"),
                    });
                }
                one
            }
        };
        Ok(Outcome::new(format!("the file is encrypted with {}", encryption.strings)).with_data(json!({
            "encrypted": true,
            "filter": encryption.filter,
            "version": encryption.version,
            "revision": encryption.revision,
            "length_bits": encryption.length_bits,
            "permissions": encryption.permissions,
            "permission_names": permissions,
            "encrypt_metadata": encryption.encrypt_metadata,
            "strings": encryption.strings,
            "streams": encryption.streams,
            "entries": listed,
        })))
    }
}

/// **Encrypt** a file with a password, and **write** it out.
///
/// A query of the document, and a file is written: the document itself
/// is not changed. The file that is written is scrambled with the
/// **standard security handler** of revision two, and the trailer names
/// the object that holds the scramble.
#[derive(Command)]
#[command(id = "pdf.file.encrypt", area = "pdf", layer = "pdf", icon = "file")]
pub struct EncryptFile {
    /// The password that opens the file.
    pub password: Text,
    /// Where to write the encrypted file.
    pub path: Text,
    /// The password of the **owner**.
    ///
    /// Leave it out and the owner is the user: the one password opens
    /// the file and owns it.
    pub owner: Option<Text>,
}

impl EncryptFile {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.file.encrypt".to_string();
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "the file has a path".to_string() });
        }
        let document = session.document();
        // The **objects** of the document, and the bytes of the streams.
        let mut objects: Vec<crate::writer::Object> = Vec::new();
        for node in document.walk() {
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                continue;
            }
            let Some(number) = node.data.get("object").and_then(|number| number.as_u64()) else {
                continue;
            };
            let entries = crate::entries_of(&node.data);
            let stream = match &node.stream {
                Some(path) => {
                    let limits = crate::filters::Limits::default();
                    let bytes = craftkit_sdk::files::read_bounded(
                        std::path::Path::new(path),
                        limits.max_input as u64,
                    )
                    .map_err(|message| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the stream's bytes cannot be read: {message}"),
                    })?;
                    Some(bytes)
                }
                None => None,
            };
            objects.push(crate::writer::Object { number, entries, stream });
        }
        if objects.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the document holds no objects to encrypt".to_string(),
            });
        }
        // The **root**: the catalog of the file.
        let root = objects
            .iter()
            .find(|object| {
                object.entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
            })
            .map(|object| object.number)
            .ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: "the document holds no catalog to be the root of the file".to_string(),
            })?;
        let info = crate::crypt::number_of(document, "Info");
        let password = self.password.0.as_bytes().to_vec();
        let owner = match &self.owner {
            Some(owner) => owner.0.as_bytes().to_vec(),
            None => password.clone(),
        };
        // The **identifier** of the file: sixteen bytes, and they are
        // worked out from the passwords and from the numbers of the
        // objects, so that the same file and the same password always
        // make the same file.
        let identifier = identifier_of(&password, &owner, &objects, root);
        // The **owner's entry**: the user's password under the owner's
        // own key.
        let owner_entry = crate::crypt::owner_entry(&owner, &password, REVISION, LENGTH_BITS);
        // The **key of the file**, and the **user's entry** it makes.
        let scheme = crate::crypt::Scheme {
            number: 0,
            revision: REVISION,
            length_bits: LENGTH_BITS,
            permissions: PERMISSIONS,
            encrypt_metadata: true,
            strings: "rc4".to_string(),
            streams: "rc4".to_string(),
            owner: owner_entry.clone(),
            user: Vec::new(),
            identifier: identifier.clone(),
        };
        let key = crate::crypt::file_key(&password, &scheme);
        let user_entry = crate::crypt::user_entry(&key, &identifier, REVISION);
        // The **encryption** is an object of the file of its own, and
        // its own words are not scrambled.
        let number = crate::free::the_next_number(document, 0);
        let mut entries: Vec<(String, Json)> = vec![
            ("Filter".to_string(), Json::String("Standard".to_string())),
            ("V".to_string(), Json::Number((1).into())),
            ("R".to_string(), Json::Number((REVISION).into())),
            ("Length".to_string(), Json::Number((LENGTH_BITS).into())),
            ("P".to_string(), Json::Number((PERMISSIONS).into())),
        ];
        entries.push(("O".to_string(), text_of(&owner_entry)));
        entries.push(("U".to_string(), text_of(&user_entry)));
        objects.push(crate::writer::Object { number, entries, stream: None });
        // The **objects** are scrambled under the key of each one.
        let mut streams = 0usize;
        for object in objects.iter_mut() {
            if object.number == number {
                continue;
            }
            let object_key = crate::crypt::object_key(&key, object.number, 0, false);
            object.entries = object
                .entries
                .iter()
                .map(|(name, value)| (name.clone(), crate::crypt::scrambled(value, &object_key)))
                .collect();
            if let Some(bytes) = &object.stream {
                streams += 1;
                object.stream = Some(crate::crypt::rc4(&object_key, bytes));
            }
        }
        let free = crate::free::the_free(document);
        let trailer = crate::writer::Trailer {
            root,
            info,
            encrypt: Some(number),
            identifier: Some(hex_of(&identifier)),
        };
        let file = crate::writer::write(&objects, &free, &trailer).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be written: {error:?}"),
        })?;
        std::fs::write(&path, &file).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the file cannot be written to `{path}`: {error}"),
        })?;
        let count = objects.len();
        Ok(Outcome::new(format!("wrote {count} object(s) with their encryption")).with_data(json!({
            "path": path,
            "objects": count,
            "streams": streams,
            "bytes": file.len(),
            "root": root,
            "encrypt": number,
        })))
    }
}

/// The **revision** of the scheme the file is scrambled with.
const REVISION: i64 = 2;

/// The **length** of the key of the file, in bits.
const LENGTH_BITS: i64 = 40;

/// The **permissions** of the file: every right there is.
const PERMISSIONS: i64 = -1;

/// The **identifier** of a file that is made: sixteen bytes.
///
/// The bytes are worked out from the passwords and from the numbers of
/// the objects, so that the same file and the same password always make
/// the same file.
fn identifier_of(password: &[u8], owner: &[u8], objects: &[crate::writer::Object], root: u64) -> Vec<u8> {
    let mut message: Vec<u8> = Vec::new();
    message.extend_from_slice(password);
    message.extend_from_slice(owner);
    message.extend_from_slice(&(root as u32).to_le_bytes());
    for object in objects {
        message.extend_from_slice(&(object.number as u32).to_le_bytes());
    }
    crate::crypt::md5(&message).to_vec()
}

/// The **text** of bytes: the payload's way of holding a string.
fn text_of(bytes: &[u8]) -> Json {
    let text: String = bytes.iter().map(|byte| *byte as char).collect();
    json!({ "string": text })
}

/// The **hex** of bytes: the way a file writes an identifier.
fn hex_of(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02X}"));
    }
    out
}
