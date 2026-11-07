//! The **encryption** of a PDF file, undone with a password.
//!
//! A file whose trailer names an **encryption** is a file whose strings
//! and streams are scrambled. The file says how, and the **standard
//! security handler** (the module `crypt`) says how that is undone.
//!
//! # The rules
//!
//! - **A password that does not open the file is refused.** The file
//!   keeps the user's entry, and the entry is worked out from the
//!   password: a password whose entry is not the file's entry does not
//!   open the file.
//! - **The owner's password opens the file too.** The owner's entry
//!   holds the user's password, and a password that is the owner's is
//!   read as the owner's and the user's password is taken from it.
//! - **A file scrambled with AES is not read.** The standard handler
//!   of revision six and above uses AES, and this tree does not have
//!   it: the command says so and does nothing.
//! - **The strings of the encryption are not scrambled.** The owner's
//!   entry and the user's entry are the file's own words, and they are
//!   left as they are.
//!
//! # What it does not do
//!
//! - **The permissions are not enforced.** The bits the file holds say
//!   what a person may do with it, and this module reads them and does
//!   not act on them.
//! - **The metadata is not treated apart.** A file whose encryption
//!   says its metadata is not scrambled is not handled differently here.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.file.decrypt";

/// **Decrypt** a file and **write** it out without its encryption.
///
/// A query of the document, and a file is written: the document itself
/// is not changed. A file that is not encrypted is refused, and so is
/// a password that does not open it.
#[derive(Command)]
#[command(id = "pdf.file.decrypt", area = "pdf", layer = "pdf", icon = "file")]
pub struct DecryptFile {
    /// The password that opens the file.
    pub password: Text,
    /// Where to write the file without its encryption.
    pub path: Text,
}

impl DecryptFile {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the file has a path".to_string(),
            });
        }
        let document = session.document();
        let Some(scheme) = crate::crypt::scheme_of(document, COMMAND)? else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the file is not encrypted".to_string(),
            });
        };
        if scheme.strings != "rc4" {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "not supported yet: the file is encrypted with {}, and only RC4 is read here",
                    scheme.strings
                ),
            });
        }
        let password = self.password.0.as_bytes().to_vec();
        let Some(key) = crate::crypt::opens(&password, &scheme) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the password does not open the file".to_string(),
            });
        };
        // The **objects** of the file, unscrambled.
        let mut objects: Vec<crate::writer::Object> = Vec::new();
        let mut streams = 0usize;
        for node in document.walk() {
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                continue;
            }
            let Some(number) = node.data.get("object").and_then(|number| number.as_u64()) else {
                continue;
            };
            if number == scheme.number {
                // The **encryption** is not carried over: the file that
                // is written is not encrypted.
                continue;
            }
            let generation = node.data.get("generation").and_then(|number| number.as_u64()).unwrap_or(0);
            let object_key = crate::crypt::object_key(&key, number, generation);
            let entries: Vec<(String, Json)> = crate::entries_of(&node.data)
                .into_iter()
                .map(|(name, value)| (name, crate::crypt::scrambled(&value, &object_key)))
                .collect();
            let stream = match &node.stream {
                Some(path) => {
                    let limits = crate::filters::Limits::default();
                    let bytes = craftkit_sdk::files::read_bounded(
                        std::path::Path::new(path),
                        limits.max_input as u64,
                    )
                    .map_err(|message| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the stream's bytes cannot be read: {message}"),
                    })?;
                    streams += 1;
                    Some(crate::crypt::rc4(&object_key, &bytes))
                }
                None => None,
            };
            objects.push(crate::writer::Object { number, entries, stream });
        }
        let Some(root) = crate::crypt::number_of(document, "Root") else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the file names no root".to_string(),
            });
        };
        let info = crate::crypt::number_of(document, "Info");
        let free = crate::free::the_free(document);
        let trailer = crate::writer::Trailer { root, info, encrypt: None, identifier: None };
        let file = crate::writer::write(&objects, &free, &trailer).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file cannot be written: {error:?}"),
        })?;
        std::fs::write(&path, &file).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file cannot be written to `{path}`: {error}"),
        })?;
        let count = objects.len();
        Ok(Outcome::new(format!("wrote {count} object(s) without their encryption")).with_data(json!({
            "path": path,
            "objects": count,
            "streams": streams,
            "bytes": file.len(),
            "root": root,
        })))
    }
}
