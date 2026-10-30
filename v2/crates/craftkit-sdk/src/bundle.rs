//! The **bundle**: a document and the files it needs, in one file.
//!
//! # The problem
//!
//! A document that points at a file — a sound document points at audio,
//! a vector document points at a font — is not finished. It is a **list
//! of instructions** that only work on the machine that has the files.
//!
//! Send it to somebody else and it opens with holes in it: the picture
//! that is not there, the sound that will not play. The sound application
//! this port comes from solves this with a **bundle**: one file with the
//! document and an `Audio Files` folder inside it.
//!
//! # What this is
//!
//! The container, and nothing else:
//!
//! - the document says which files it needs — its **assets** — and the
//!   bytes are not in it;
//! - a bundle is the document and those bytes, in one file, with a
//!   **checksum** so that a bundle that was damaged on the way is
//!   refused rather than half-read;
//! - reading a bundle gives back the document *and* the files, and a
//!   document can be asked what it is **missing**.
//!
//! The format is small and written out here rather than pulled in: a
//! bundle is a container, and a container that a document format depends
//! on is worth owning.
//!
//! # The shape of the file
//!
//! ```text
//! "craftkit-bundle"   14 bytes, the magic
//! version             u32, little endian, 1
//! document length     u64, little endian
//! document            the bytes of the document, in the newest format
//! asset count         u32, little endian
//! for each asset:
//!   path length       u32, little endian
//!   path              the bytes of the path
//!   data length       u64, little endian
//!   data              the bytes of the file
//! checksum            u64, little endian: the whole of the above
//! ```
//!
//! The checksum is a **FNV-1a** over everything before it. It is not a
//! security measure and it does not pretend to be one: it is there to
//! say "this file was damaged", which is a different thing.

use crate::contribution::Registry;
use crate::document::Document;
use crate::format::{FormatSpec, MigrationSpec};

/// The magic at the start of a bundle.
const MAGIC: &[u8; 15] = b"craftkit-bundle";

/// The version of the bundle format.
pub const VERSION: u32 = 1;

/// One file in a bundle: a path and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Where the file goes, relative to the document.
    pub path: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

impl File {
    /// A file.
    pub fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        File { path: path.into(), bytes }
    }
}

/// A bundle: a document and the files it needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Bundle {
    /// The document.
    pub document: Document,
    /// The files.
    pub files: Vec<File>,
}

impl Bundle {
    /// The paths of the files in it.
    pub fn paths(&self) -> Vec<String> {
        self.files.iter().map(|file| file.path.clone()).collect()
    }

    /// The files the document needs that are not in the bundle.
    ///
    /// The assets and the nodes' streams: both are files the document
    /// names, and both have to be there for the document to be finished.
    pub fn missing(&self) -> Vec<String> {
        let paths = self.paths();
        let mut missing = self.document.missing_assets(&paths);
        for path in self.document.missing_streams(&paths) {
            if !missing.contains(&path) {
                missing.push(path);
            }
        }
        missing
    }
}

/// Write a bundle.
///
/// The document is written in the **oldest** format of the family that
/// can say everything the document says — so that a program which knows
/// only that version can read the file — and the files are written as
/// they are.
///
/// If no version round trips, which should not happen, the **newest** one
/// is used: a bundle that cannot be written at all is worse than one
/// that is written at a version that loses something.
pub fn write(
    document: &Document,
    files: Vec<File>,
    formats: &Registry<FormatSpec>,
    migrations: &Registry<MigrationSpec>,
) -> Result<Vec<u8>, String> {
    let newest = crate::format::newest_format(formats, "craftkit")
        .ok_or_else(|| "this program has no format to write the document in".to_string())?;
    let (document_json, _) = crate::format::write_document_oldest(document, formats, migrations, "craftkit")
        .unwrap_or_else(|| (crate::format::write_document(document, newest), newest.version));
    let document_bytes = document_json.to_string().into_bytes();
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(document_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&document_bytes);
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for file in &files {
        let path = file.path.as_bytes();
        out.extend_from_slice(&(path.len() as u32).to_le_bytes());
        out.extend_from_slice(path);
        out.extend_from_slice(&(file.bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&file.bytes);
    }
    let checksum = checksum(&out);
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

/// The most files a bundle may hold.
///
/// A bundle's file count comes from the file, and a loop count that
/// comes from outside is **capped** — the kit's rule, and the reason
/// this number is here. The cap **refuses** the bundle: a bundle with
/// more files than the limit allows is not truncated into one that fits.
pub const MAX_FILES: usize = 4096;

/// The most bytes the **document** in a bundle may hold.
///
/// The bundle's own bytes are the program's business (the program's own
/// bound on the file it reads), but the document is the part every
/// reader shares — so this one is the format's, and it is named in the
/// v2 README beside the other bounds.
const MAX_BUNDLE_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;

/// The most bytes a **path** in a bundle may hold.
///
/// A path is a name, and a name is not long. It is named in the v2
/// README beside the other bounds.
const MAX_BUNDLE_PATH_BYTES: usize = 4096;

/// Read a bundle.
///
/// A bundle that is not a bundle, or that was damaged on the way, is
/// **refused** — and the reason says which.
///
/// The counts that come out of the file are **capped**: the number of
/// files, the length of the document and the length of a path. A cap
/// that can be traded away is not a cap, so each of them refuses the
/// bundle rather than quietly reading less.
pub fn read(
    bytes: &[u8],
    formats: &Registry<FormatSpec>,
    migrations: &Registry<MigrationSpec>,
) -> Result<Bundle, String> {
    if bytes.len() < MAGIC.len() + 4 + 8 + 8 {
        return Err("this is too short to be a bundle".to_string());
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err("this is not a bundle".to_string());
    }
    let body = &bytes[..bytes.len() - 8];
    let written = u64::from_le_bytes(
        bytes[bytes.len() - 8..].try_into().map_err(|_| "the checksum is not there".to_string())?,
    );
    if checksum(body) != written {
        return Err("the bundle was damaged on the way".to_string());
    }
    let mut at = MAGIC.len();
    let version = read_u32(bytes, &mut at)?;
    if version != VERSION {
        return Err(format!("this is a bundle of version {version} and not of version {VERSION}"));
    }
    let document_length = read_u64(bytes, &mut at)? as usize;
    if document_length > MAX_BUNDLE_DOCUMENT_BYTES {
        return Err(format!(
            "the document in the bundle is {document_length} bytes, and the limit is {MAX_BUNDLE_DOCUMENT_BYTES}"
        ));
    }
    let document_bytes = read_slice(bytes, &mut at, document_length)?;
    let document_value: crate::serde_json::Value = crate::serde_json::from_slice(document_bytes)
        .map_err(|error| format!("the document in the bundle is not JSON: {error}"))?;
    let document = crate::format::read_document(&document_value, formats, migrations)
        .map_err(|message| format!("the document in the bundle is not one anybody can read: {message}"))?;
    let count = read_u32(bytes, &mut at)? as usize;
    if count > MAX_FILES {
        return Err(format!("the bundle holds {count} files, and the limit is {MAX_FILES}"));
    }
    let mut files = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let path_length = read_u32(bytes, &mut at)? as usize;
        if path_length > MAX_BUNDLE_PATH_BYTES {
            return Err(format!(
                "a path in the bundle is {path_length} bytes, and the limit is {MAX_BUNDLE_PATH_BYTES}"
            ));
        }
        let path = read_slice(bytes, &mut at, path_length)?;
        let path =
            String::from_utf8(path.to_vec()).map_err(|_| "a path in the bundle is not text".to_string())?;
        let data_length = read_u64(bytes, &mut at)? as usize;
        let data = read_slice(bytes, &mut at, data_length)?;
        files.push(File::new(path, data.to_vec()));
    }
    Ok(Bundle { document, files })
}

/// Read a u32 at a place, and move the place on.
fn read_u32(bytes: &[u8], at: &mut usize) -> Result<u32, String> {
    let slice = read_slice(bytes, at, 4)?;
    Ok(u32::from_le_bytes(slice.try_into().map_err(|_| "a number is not there".to_string())?))
}

/// Read a u64 at a place, and move the place on.
fn read_u64(bytes: &[u8], at: &mut usize) -> Result<u64, String> {
    let slice = read_slice(bytes, at, 8)?;
    Ok(u64::from_le_bytes(slice.try_into().map_err(|_| "a number is not there".to_string())?))
}

/// Read some bytes at a place, and move the place on.
fn read_slice<'a>(bytes: &'a [u8], at: &mut usize, length: usize) -> Result<&'a [u8], String> {
    let end = at.checked_add(length).ok_or_else(|| "a length is too big".to_string())?;
    if end > bytes.len() {
        return Err("the bundle ends before it should".to_string());
    }
    let slice = &bytes[*at..end];
    *at = end;
    Ok(slice)
}

/// The FNV-1a checksum of some bytes.
///
/// It is not a security measure: it says "this was damaged", and it
/// says it in six lines.
fn checksum(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contribution::Registry;
    use crate::document::Asset;

    /// The formats this program knows.
    fn formats() -> Registry<FormatSpec> {
        Registry::from_inventory().expect("the formats are registered")
    }

    /// The steps between the versions.
    fn migrations() -> Registry<MigrationSpec> {
        Registry::from_inventory().expect("the migrations are registered")
    }

    /// A document with a file in it.
    fn document() -> Document {
        let mut document = Document::new("bundled");
        document.add_node("rect", "geometry.rect", None);
        document.add_asset(Asset::new("sounds/kick.wav", "audio", Some(4)));
        document
    }

    /// A bundle round trips.
    #[test]
    fn a_bundle_round_trips() {
        let document = document();
        let bytes =
            write(&document, vec![File::new("sounds/kick.wav", vec![1, 2, 3, 4])], &formats(), &migrations())
                .expect("the bundle is written");
        let bundle = read(&bytes, &formats(), &migrations()).expect("the bundle is read");
        assert_eq!(bundle.document, document, "the document came back");
        assert_eq!(bundle.files.len(), 1);
        assert_eq!(bundle.files[0].path, "sounds/kick.wav");
        assert_eq!(bundle.files[0].bytes, vec![1, 2, 3, 4]);
    }

    /// A bundle that is not a bundle is refused.
    #[test]
    fn a_bundle_that_is_not_a_bundle_is_refused() {
        let error = read(b"this is long enough to be something else", &formats(), &migrations())
            .expect_err("it is refused");
        assert!(error.contains("not a bundle"), "{error}");
    }

    /// A bundle that was damaged is refused.
    #[test]
    fn a_damaged_bundle_is_refused() {
        let document = document();
        let mut bytes =
            write(&document, vec![File::new("sounds/kick.wav", vec![1, 2, 3, 4])], &formats(), &migrations())
                .expect("the bundle is written");
        let last = bytes.len() - 1;
        bytes[last] ^= 0xff;
        let error = read(&bytes, &formats(), &migrations()).expect_err("it is refused");
        assert!(error.contains("damaged"), "{error}");
    }

    /// Something too short to be a bundle is refused.
    #[test]
    fn something_too_short_is_refused() {
        let error = read(b"short", &formats(), &migrations()).expect_err("it is refused");
        assert!(error.contains("too short"), "{error}");
    }

    /// A bundle that ends early is refused.
    #[test]
    fn a_bundle_that_ends_early_is_refused() {
        let document = document();
        let bytes =
            write(&document, vec![File::new("sounds/kick.wav", vec![1, 2, 3, 4])], &formats(), &migrations())
                .expect("the bundle is written");
        let error = read(&bytes[..bytes.len() - 20], &formats(), &migrations()).expect_err("it is refused");
        assert!(!error.is_empty(), "{error}");
    }

    /// A document says what it is missing.
    #[test]
    fn a_document_says_what_it_is_missing() {
        let document = document();
        assert_eq!(document.missing_assets(&[]), vec!["sounds/kick.wav".to_string()]);
        assert!(document.missing_assets(&["sounds/kick.wav".to_string()]).is_empty());
    }

    /// A bundle that claims **more files** than the limit allows is
    /// refused: the count comes from the file, and a count that comes
    /// from outside is capped.
    #[test]
    fn a_bundle_with_too_many_files_is_refused() {
        let formats = formats();
        let migrations = Registry::<MigrationSpec>::new();
        let mut bytes =
            write(&Document::new("bundled"), vec![], &formats, &migrations).expect("the bundle is written");
        // The file count is the u32 after the version, the document's
        // length and the document itself.
        let document_length = u64::from_le_bytes(
            bytes[MAGIC.len() + 4..MAGIC.len() + 12].try_into().expect("the length is there"),
        ) as usize;
        let at = MAGIC.len() + 4 + 8 + document_length;
        let count = (MAX_FILES as u32) + 1;
        bytes[at..at + 4].copy_from_slice(&count.to_le_bytes());
        // The checksum covers the body, so it is put back.
        let body = bytes[..bytes.len() - 8].to_vec();
        let written = checksum(&body);
        let last = bytes.len() - 8;
        bytes[last..].copy_from_slice(&written.to_le_bytes());
        let error = read(&bytes, &formats, &migrations).expect_err("it is refused");
        assert!(error.contains("files"), "{error}");
    }

    /// A bundle whose **document** is bigger than the limit allows is
    /// refused, and so is one with a path that is too long.
    #[test]
    fn a_bundle_with_a_too_big_document_is_refused() {
        let formats = formats();
        let migrations = Registry::<MigrationSpec>::new();
        let mut bytes =
            write(&Document::new("bundled"), vec![], &formats, &migrations).expect("the bundle is written");
        // The document's length is the u64 after the version.
        let at = MAGIC.len() + 4;
        let length = (MAX_BUNDLE_DOCUMENT_BYTES as u64) + 1;
        bytes[at..at + 8].copy_from_slice(&length.to_le_bytes());
        let body = bytes[..bytes.len() - 8].to_vec();
        let written = checksum(&body);
        let last = bytes.len() - 8;
        bytes[last..].copy_from_slice(&written.to_le_bytes());
        let error = read(&bytes, &formats, &migrations).expect_err("it is refused");
        assert!(error.contains("document"), "{error}");
    }

    /// A bundle knows what the document is missing.
    #[test]
    fn a_bundle_knows_what_is_missing() {
        let document = document();
        let bundle =
            Bundle { document: document.clone(), files: vec![File::new("sounds/other.wav", vec![9])] };
        assert_eq!(bundle.missing(), vec!["sounds/kick.wav".to_string()]);
        let full = Bundle { document, files: vec![File::new("sounds/kick.wav", vec![9])] };
        assert!(full.missing().is_empty());
    }
}
