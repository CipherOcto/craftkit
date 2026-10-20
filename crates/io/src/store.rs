//! Reading and writing files.
//!
//! Small on purpose. Everything interesting is in [`crate::format`]; this is
//! the part that touches the filesystem, which is the part that can fail in
//! ways a parser cannot.

use std::path::{Path, PathBuf};

use craftkit_model::Document;

use crate::format::{File, FormatError};

/// Why a file could not be read or written.
///
/// Deliberately **not** `Clone` or `Eq`: it carries an [`std::io::Error`], which
/// is neither. A caller that needs to compare errors should match on the
/// variants, which is also the only comparison that stays right when a message
/// changes.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("{0}")]
    Format(#[from] FormatError),

    #[error("could not read {}: {source}", .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not write {}: {source}", .path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{} is not a file", .0.display())]
    NotAFile(PathBuf),
}

impl SaveError {
    /// Whether a newer build, or a person, could plausibly fix this.
    pub fn is_recoverable(&self) -> bool {
        matches!(self, SaveError::Format(f) if f.is_version()) || matches!(self, SaveError::NotAFile(_))
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            SaveError::Read { path, .. } | SaveError::Write { path, .. } => Some(path),
            SaveError::NotAFile(p) => Some(p),
            SaveError::Format(_) => None,
        }
    }
}

/// Serialize a document to the text that goes on disk.
///
/// This is the shape `docs/design/file-format.md` describes, and the examples
/// here are the reason that document can be trusted: they are assertions, not
/// prose, and they fail if any of it drifts.
///
/// # Examples
///
/// The three shapes that surprise people writing a file by hand, pinned against
/// what the serialiser actually produces:
///
/// ```
/// use craftkit_core::Value;
/// use craftkit_io::save_text;
/// use craftkit_model::{Document, NodeKind, PropGroup, PropHint, PropNode, Property};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut doc = Document::empty("demo");
/// let id = doc.add_node("In", NodeKind::Source)?;
///
/// let prop = Property::new(1, "gain", Value::num("gain", 2.0)?)
///     .with_hint(PropHint::Slider { min: 0.0, max: 1.0 });
/// let mut group = PropGroup::default();
/// group.nodes.push(PropNode::Prop(prop));
/// doc.get_mut(id).expect("just added").props = group;
///
/// let file: serde_json::Value = serde_json::from_str(&save_text(&doc)?)?;
/// let prop = &file["document"]["nodes"][0]["props"]["nodes"][0]["Prop"];
///
/// // `format` is a tag and a version, not a number.
/// assert_eq!(file["format"], "craftkit/1");
///
/// // A value is adjacently tagged: `{"t": …, "v": …}`.
/// assert_eq!(prop["value"]["t"], "num");
/// assert_eq!(prop["value"]["v"], 2.0);
///
/// // The property node is externally tagged, so the variant name is the key.
/// assert!(prop.is_object(), "the leaf is under `Prop`, not inlined");
///
/// // And the hint is externally tagged too, with a capitalised name — serde's
/// // default, and the single most common hand-writing mistake.
/// assert_eq!(prop["hint"]["Slider"]["max"], 1.0);
/// # Ok(())
/// # }
/// ```
pub fn save_text(doc: &Document) -> Result<String, SaveError> {
    // The writer must not write what the reader refuses.
    //
    // A file that cannot be loaded back is worse than no file at all: it
    // looks like a save, it takes up the name the user's plan should have,
    // and the refusal comes later, when the work is already out of sight.
    // The reader validates the document (`File::from_text`); this is the
    // same check on the way out, so the two cannot drift apart. §50.
    doc.validate().map_err(|e| SaveError::Format(FormatError::InvalidDocument(e.to_string())))?;
    Ok(File::new(doc.clone()).to_text()?)
}

/// Parse text into a document.
///
/// # Examples
///
/// A file written by hand, with nothing in it the serialiser did not put there.
///
/// ```
/// use craftkit_io::load_text;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let hand_written = r#"{
///   "format": "craftkit/1",
///   "meta": { "written_by": "a person", "written_at": 0, "pending": [] },
///   "document": {
///     "meta": { "name": "by hand", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" },
///     "nodes": [
///       { "id": 1, "name": "In", "kind": "source",
///         "props": { "id": "props", "label": "Properties", "nodes": [] },
///         "muted": false }
///     ]
///   }
/// }"#;
///
/// let doc = load_text(hand_written)?;
/// assert_eq!(doc.meta.name, "by hand");
/// assert_eq!(doc.nodes.len(), 1);
/// # Ok(())
/// # }
/// ```
///
/// The single most common mistake is a bare number where a tagged value belongs.
/// The message is written for somebody looking at a file they did not write, so
/// this asserts the words as well as the failure:
///
/// ```
/// # use craftkit_io::load_text;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let bare_value = r#"{
///   "format": "craftkit/1",
///   "meta": { "written_by": "a person", "written_at": 0, "pending": [] },
///   "document": {
///     "meta": { "name": "by hand", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" },
///     "nodes": [
///       { "id": 1, "name": "In", "kind": "source",
///         "props": { "id": "props", "label": "Properties",
///                    "nodes": [ { "Prop": { "id": 1, "label": "gain", "value": 2.0 } } ] },
///         "muted": false }
///     ]
///   }
/// }"#;
///
/// let err = load_text(bare_value).expect_err("a bare 2.0 is not a Value");
/// let message = err.to_string();
/// assert!(message.contains("expected adjacently tagged enum Value"), "{message}");
/// # Ok(())
/// # }
/// ```
pub fn load_text(text: &str) -> Result<Document, SaveError> {
    Ok(File::from_text(text)?.document)
}

/// Write a document to `path`, atomically.
///
/// **Atomically** means: write to a neighbouring temporary name, then rename.
/// A rename within a directory is the one filesystem operation that is
/// effectively all-or-nothing on every platform craftkit targets, so a crash
/// mid-save leaves the *previous* file intact rather than a half-written one.
///
/// A missing parent directory is created. A parent that exists as a *file* is
/// reported, not worked around.
pub fn save(path: &Path, doc: &Document) -> Result<(), SaveError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && parent.exists()
        && !parent.is_dir()
    {
        return Err(SaveError::NotAFile(parent.to_path_buf()));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|source| SaveError::Write { path: parent.to_path_buf(), source })?;
    }

    let text = save_text(doc)?;
    let tmp = with_suffix(path, "tmp");
    std::fs::write(&tmp, text).map_err(|source| SaveError::Write { path: tmp.clone(), source })?;
    match std::fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(source) => {
            // Do not leave the temporary behind for the next save to trip on.
            let _ = std::fs::remove_file(&tmp);
            Err(SaveError::Write { path: path.to_path_buf(), source })
        }
    }
}

/// Read a document from `path`.
pub fn load(path: &Path) -> Result<Document, SaveError> {
    // The limit is applied **while reading**, by the shared bounded reader:
    // a file of any size costs one buffer rather than its whole size in
    // memory. The check used to happen after the read — the file was read in
    // full, and only then refused — which meant the memory was already
    // spent. Measured: a 300 MiB file was read in 180 ms and refused
    // afterwards, with the peak resident size at 300 MiB. §49.
    let text = match craftkit_core::read_file_bounded(path, crate::format::MAX_FILE_BYTES) {
        Ok(text) => text,
        Err(craftkit_core::ReadError::TooLong { len, max }) => {
            return Err(SaveError::Format(FormatError::TooLarge { bytes: len, limit: max }));
        }
        Err(e) => {
            return Err(SaveError::Read {
                path: path.to_path_buf(),
                source: std::io::Error::other(e.to_string()),
            });
        }
    };
    load_text(&text)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    name.push('.');
    name.push_str(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_model::NodeKind;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join("craftkit-io-tests").join(format!("{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn sample() -> Document {
        let mut d = Document::empty("Stored");
        let _ = d.add_node("A", NodeKind::Source);
        let _ = d.add_node("B", NodeKind::Filter);
        d
    }

    /// The writer must not write what the reader refuses.
    ///
    /// A file that cannot be loaded back is worse than no file: it looks like
    /// a save, and the refusal comes later, when the work is already out of
    /// sight. §50.
    #[test]
    fn the_writer_refuses_a_document_the_reader_would_refuse() {
        let mut doc = sample();
        // Two nodes with the same id — the reader refuses this document.
        let id = doc.nodes[0].id;
        doc.nodes[1].id = id;

        let err = save_text(&doc).expect_err("the writer refuses it too");
        let message = err.to_string();
        assert!(message.contains("share an id"), "{message}");

        // And the file it would have written is not on disk.
        let path = scratch("refused").join("plan.craftkit");
        let err = save(&path, &doc).expect_err("the writer refuses it too");
        assert!(err.to_string().contains("share an id"), "{err}");
        assert!(!path.exists(), "a refused document left a file behind");
    }

    /// A file whose range no reply could carry is refused.
    ///
    /// The format is wider than the model — a bare integer is read straight
    /// into an `i128` — so a file can name an instant that no JSON number
    /// holds. Reading it would make every reply that mentions the playhead
    /// fail inside `json!`. §60.
    #[test]
    fn a_file_whose_range_is_too_far_out_is_refused() {
        let good = save_text(&sample()).expect("the sample writes");
        let patched = good
            .replace("\"end\": 1270080000000", "\"end\": 18446744073709551617")
            .replace("\"start\": 0", "\"start\": 18446744073709551616");
        assert_ne!(patched, good, "the patch did not apply");

        let err = load_text(&patched).expect_err("the reader refuses it");
        let message = err.to_string();
        assert!(message.contains("outside what a time can be"), "{message}");
    }

    /// A file whose value is out of range is refused.
    ///
    /// The value's invariants are kept by its constructors, and a file is
    /// deserialized straight into the model: no constructor runs, so nothing
    /// checked the number. A file could carry a number a hundred million
    /// times past the limit, and the evaluator's comment says every number
    /// the tree stores is in range. §73.
    #[test]
    fn a_file_whose_value_is_out_of_range_is_refused() {
        let file = |value: &str| {
            format!(
                r#"{{
  "format": "craftkit/1",
  "meta": {{ "written_by": "a person", "written_at": 0, "pending": [] }},
  "document": {{
    "meta": {{ "name": "by hand", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" }},
    "nodes": [
      {{ "id": 1, "name": "In", "kind": "source",
        "props": {{ "id": "props", "label": "Properties",
                   "nodes": [ {{ "Prop": {{ "id": 1, "label": "amount", "value": {{ "t": "num", "v": {value} }} }} }} ] }},
        "muted": false }}
    ]
  }}
}}"#
            )
        };

        // The same file with a number the tree can hold is read.
        let good = load_text(&file("0.5")).expect("a number in range is read");
        assert_eq!(good.nodes.len(), 1);

        // And one a hundred million times past the limit is not.
        let err = load_text(&file("1e300")).expect_err("the reader refuses it");
        let message = err.to_string();
        assert!(message.contains("out of range"), "{message}");
        assert!(message.contains("amount"), "{message}");
    }

    /// A document the reader would refuse must not be written.
    ///
    /// §49 gave the reader a limit of 64 MiB. The writer had none, so a
    /// document big enough to serialise past it was written to disk as a file
    /// that could not be opened again. Measured: a document of 32 properties
    /// with 10 000 keyframes on each — well inside the model's own limits —
    /// serialised to 74 820 332 bytes, was written, and was then refused by
    /// the reader with *"the file is 74820332 bytes, and the limit is
    /// 67108864"*. §51.
    #[test]
    fn a_document_too_large_to_read_back_is_not_written() {
        let doc = too_large_to_read();

        let err = save_text(&doc).expect_err("the writer refuses it too");
        let message = err.to_string();
        assert!(message.contains("the limit is"), "{message}");

        let path = scratch("too-large").join("plan.craftkit");
        let err = save(&path, &doc).expect_err("the writer refuses it too");
        assert!(err.to_string().contains("the limit is"), "{err}");
        assert!(!path.exists(), "a refused document left a file behind");
    }

    /// A document the reader *can* read back is written, and reads back.
    ///
    /// The other half of the test above: a limit that refuses everything is
    /// not a limit, it is a wall.
    #[test]
    fn a_document_the_reader_can_read_back_is_written() {
        let path = scratch("readable").join("plan.craftkit");
        let doc = sample();
        save(&path, &doc).expect("save");
        assert_eq!(load(&path).expect("load"), doc);
    }

    /// A document the model's own limits allow, and the reader's do not.
    ///
    /// One node, 32 properties, 10 000 keyframes on each: inside
    /// `MAX_PROPERTIES` and `MAX_KEYS`, and 74 MiB of text.
    fn too_large_to_read() -> Document {
        let mut doc = Document::empty("Big");
        let id = doc.add_node("n", NodeKind::Source).expect("node");
        let mut group = craftkit_model::PropGroup::new("props", "Properties");
        for p in 0..32u64 {
            let mut prop =
                craftkit_model::Property::new(p + 1, format!("p{p}"), craftkit_core::Value::Num(1.0));
            for k in 0..10_000i128 {
                let _ = prop.keys.set(
                    craftkit_model::Key::new(
                        craftkit_core::Tick::from_raw(k),
                        craftkit_core::Value::Num(k as f64),
                        craftkit_model::Easing::Linear,
                    )
                    .expect("key"),
                );
            }
            group.push(prop);
        }
        doc.get_mut(id).expect("just added").props = group;
        doc
    }

    #[test]
    fn a_document_round_trips_through_a_file() {
        let path = scratch("roundtrip").join("plan.craftkit");
        let doc = sample();
        save(&path, &doc).expect("save");
        assert_eq!(load(&path).expect("load"), doc);
    }

    #[test]
    fn saving_creates_a_missing_directory() {
        let path = scratch("mkdir").join("a").join("b").join("plan.craftkit");
        save(&path, &sample()).expect("save into a directory that did not exist");
        assert!(path.is_file());
    }

    #[test]
    fn saving_leaves_no_temporary_behind() {
        let dir = scratch("no-temp");
        let path = dir.join("plan.craftkit");
        save(&path, &sample()).expect("save");
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn saving_over_an_existing_file_replaces_it_completely() {
        let path = scratch("replace").join("plan.craftkit");
        save(&path, &sample()).expect("first save");
        let mut smaller = Document::empty("Stored");
        let _ = smaller.add_node("A", NodeKind::Source);
        save(&path, &smaller).expect("second save");
        let back = load(&path).expect("load");
        assert_eq!(back.nodes.len(), 1, "the longer file's tail survived the rewrite");
    }

    #[test]
    fn a_parent_that_is_a_file_is_reported_rather_than_ignored() {
        let dir = scratch("not-a-dir");
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, b"x").expect("write blocker");
        let err = save(&blocker.join("plan.craftkit"), &sample()).expect_err("refused");
        assert!(matches!(err, SaveError::NotAFile(_)), "{err}");
        assert!(err.is_recoverable());
        assert!(err.path().is_some());
    }

    #[test]
    fn reading_a_file_that_is_not_there_names_the_path() {
        let path = scratch("missing").join("nope.craftkit");
        let err = load(&path).expect_err("refused");
        assert!(err.to_string().contains("nope.craftkit"), "{err}");
    }

    #[test]
    fn reading_a_newer_file_says_so_instead_of_guessing() {
        let path = scratch("newer").join("plan.craftkit");
        let mut file = File::new(sample());
        file.format = crate::format::tag(crate::format::CURRENT + 1);
        std::fs::write(&path, file.to_text().expect("json")).expect("write");
        let err = load(&path).expect_err("refused");
        assert!(err.is_recoverable(), "{err}");
        assert!(err.to_string().contains("newer"), "{err}");
    }

    #[test]
    fn the_text_form_is_stable_so_saves_diff_cleanly() {
        let doc = sample();
        let a = save_text(&doc).expect("text");
        let b = save_text(&doc).expect("text");
        assert_eq!(a, b);
        assert!(a.ends_with('\n'));
        assert!(a.contains("\"format\": \"craftkit/1\""), "{a}");
    }
}
