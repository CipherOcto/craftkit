//! The file format, versioned.
//!
//! # The tag
//!
//! Every file starts with `"format": "craftkit/N"`, where `N` is the version.
//! A reader refuses a version it does not know rather than guessing, because a
//! guessed read of a newer file silently corrupts a design.
//!
//! # How a version is born
//!
//! **A change that alters what is saved adds ONE file**, `pending/<name>.rs`.
//! It is never an edit to `FORMAT`, never to `FORMAT_STEPS`, and never to a
//! `v<N>/` folder. The build works the number out — it is the next one while
//! anything is pending — and the release folds the pending files into `v<N>/`.
//!
//! That rule exists because the alternative is a diff that touches the version
//! *and* the reader *and* the tests every single time, and because the moment
//! two people edit one file is the moment the scheme breaks.
//!
//! # What this crate deliberately does not do
//!
//! It does not invent fields. Every field in [`Format`] is here because some
//! command reads it. A field nobody reads is a field that will drift.

pub mod pending;

use serde::{Deserialize, Serialize};

/// The tag written into every file.
pub const TAG_PREFIX: &str = "craftkit";

/// The version this build writes.
pub const CURRENT: u32 = 1;

/// The tag for a version.
pub fn tag(version: u32) -> String {
    format!("{TAG_PREFIX}/{version}")
}

/// Why a file could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FormatError {
    #[error("this is not a craftkit file: the `format` field is missing")]
    MissingTag,

    #[error("`{0}` is not a craftkit file")]
    NotOurs(String),

    #[error(
        "this file is format version {found}, and this build reads up to {CURRENT}. \
         Open it with a newer craftkit, or add a pending change for it."
    )]
    TooNew { found: u32 },

    #[error("this file is format version 0 or less (`{0}`)")]
    TooOld(String),

    #[error("the file is not valid JSON: {0}")]
    NotJson(String),

    #[error("the file is {bytes} bytes, and the limit is {limit}")]
    TooLarge { bytes: usize, limit: usize },

    /// The writer's side of the same limit.
    ///
    /// A separate variant because the message has to be true: there is no
    /// file yet, and saying "the file is N bytes" about a file that was never
    /// written is the kind of small lie this tree does not tell. The count is
    /// a **lower bound** — the writer stopped there rather than finishing the
    /// text to find out how big it really was.
    #[error("the file would be more than {bytes} bytes, and the limit is {limit}")]
    TooLargeToWrite { bytes: usize, limit: usize },

    #[error("the file's `meta` block is malformed: {0}")]
    Meta(String),

    #[error("the document in this file is not valid: {0}")]
    InvalidDocument(String),
}

impl FormatError {
    /// Whether a newer craftkit would probably be able to read this.
    pub fn is_version(&self) -> bool {
        matches!(self, FormatError::TooNew { .. } | FormatError::TooOld(_) | FormatError::NotOurs(_))
    }
}

/// The header of a `.craftkit` file.
///
/// Everything else in the file is the document itself, unchanged, so a future
/// reader does not have to know how the header nests it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Format {
    pub format: String,
    pub meta: Meta,
}

/// What this build knows about the file it is writing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    /// This build's version.
    pub written_by: String,
    /// **Always zero.** Reserved.
    ///
    /// The field is called `written_at` and the format is a tick count, but it
    /// does not carry a write time, and it must not: a file's bytes have to
    /// depend on the document and on nothing else, so that saving the same
    /// plan twice gives the same file and a saved plan diffs cleanly.
    /// `the_text_form_is_stable_so_saves_diff_cleanly` is the test that
    /// protects that, and a clock reading in here would break it on every
    /// save.
    ///
    /// Ticks, never seconds — see `craftkit-core`. §61.
    #[serde(default = "zero")]
    pub written_at: i128,
    /// The format changes that had already been applied to this file.
    ///
    /// A reader applies the changes **this build** has that the file has not
    /// had yet, and this list is what says which ones the file has. A file
    /// this build wrote lists every change this build has, so it is not
    /// rewritten when it is read back.
    ///
    /// Defaulted: a file written before this field existed must still load, and
    /// it has had none of the changes, so the empty list is right for it.
    ///
    /// §81: this field was written and never read — the reader applied every
    /// pending change to every file, including the files it had just written.
    #[serde(default)]
    pub pending: Vec<String>,
}

/// The default for [`Meta::written_at`], so a file written before the field
/// existed still loads.
fn zero() -> i128 {
    craftkit_core::Tick::ZERO.raw()
}

impl Meta {
    pub fn now() -> Self {
        Self {
            written_by: concat!("craftkit ", env!("CARGO_PKG_VERSION")).to_string(),
            written_at: zero(),
            pending: pending::names().into_iter().map(String::from).collect(),
        }
    }
}

impl Format {
    /// A header for this build.
    pub fn new() -> Self {
        Self { format: tag(CURRENT), meta: Meta::now() }
    }

    /// The version this file claims.
    pub fn version(&self) -> Result<u32, FormatError> {
        if self.format.trim().is_empty() {
            return Err(FormatError::MissingTag);
        }
        let Some(rest) = self.format.strip_prefix(&format!("{TAG_PREFIX}/")) else {
            return Err(FormatError::NotOurs(self.format.clone()));
        };
        rest.parse::<u32>()
            .map_err(|_| FormatError::TooOld(self.format.clone()))
            .and_then(|v| if v == 0 { Err(FormatError::TooOld(self.format.clone())) } else { Ok(v) })
    }

    /// Refuse a file this build cannot read correctly.
    pub fn check(&self) -> Result<(), FormatError> {
        let v = self.version()?;
        if v > CURRENT {
            return Err(FormatError::TooNew { found: v });
        }
        Ok(())
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::new()
    }
}

/// How many bytes a design file may be.
///
/// The cap is generous for a document and small enough to bound the parse.
/// It is applied **while reading**, in [`crate::store::load`], so that a file
/// of any size costs at most this much memory — a cap checked after the read
/// has already paid for what it was meant to prevent. §49.
///
/// That bounds the **text**. The parse then turns the text into a document,
/// and a document is measured against its own budget
/// ([`craftkit_model::limits::MAX_DOCUMENT_BYTES`]), so the file's bytes and
/// the document's bytes are two different numbers and the reader's cap is the
/// looser one. A raw JSON input has no such second measurement, which is why
/// its limits are derived from a memory budget instead — §64.
pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

/// The shape of a whole file on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    /// Defaults to empty so a file with no `format` field produces
    /// [`FormatError::MissingTag`] — a message a person can act on — rather
    /// than a serde "missing field" error naming a struct path.
    #[serde(default)]
    pub format: String,
    /// Defaulted for the same reason as `format`: a hand-written or foreign
    /// JSON object should be told it is not a craftkit file, not that it is
    /// missing a struct field.
    #[serde(default)]
    pub meta: Meta,
    /// The document, stored as the model stores it.
    pub document: craftkit_model::Document,
}

impl File {
    pub fn new(document: craftkit_model::Document) -> Self {
        let head = Format::new();
        Self { format: head.format, meta: head.meta, document }
    }

    /// Refuse a file this build cannot read correctly.
    pub fn check(&self) -> Result<(), FormatError> {
        Format { format: self.format.clone(), meta: self.meta.clone() }.check()
    }

    /// The JSON this build writes, pretty-printed with a trailing newline.
    ///
    /// The trailing newline is not cosmetic: it makes every saved file end in
    /// one, so a diff of two saves shows only what changed.
    ///
    /// The text is written **through a limit**, so this build never produces a
    /// file its own reader would refuse. §51: the reader stops at
    /// [`MAX_FILE_BYTES`], and the writer used to have no limit at all, so a
    /// document big enough to serialise past it was written to disk as a file
    /// that could not be opened again. The limit is applied while the text is
    /// being written, not after — §49's lesson, in a new place: a cap checked
    /// once the bytes are in memory has already paid for what it prevents.
    pub fn to_text(&self) -> Result<String, FormatError> {
        self.to_text_bounded(MAX_FILE_BYTES)
    }

    /// The JSON this build writes, refusing to write more than `max` bytes.
    ///
    /// The limit is the point: it is what makes the writer and the reader
    /// agree about what a file is.
    pub fn to_text_bounded(&self, max: usize) -> Result<String, FormatError> {
        let mut out = LimitedWriter::new(max);
        match serde_json::to_writer_pretty(&mut out, self) {
            Ok(()) => {}
            Err(e) if e.is_io() => {
                return Err(FormatError::TooLargeToWrite { bytes: out.seen, limit: max });
            }
            Err(e) => return Err(FormatError::Meta(e.to_string())),
        }
        // The newline is part of the file, so it counts towards the limit.
        std::io::Write::write_all(&mut out, b"\n")
            .map_err(|_| FormatError::TooLargeToWrite { bytes: out.seen, limit: max })?;
        String::from_utf8(out.out).map_err(|e| FormatError::Meta(e.to_string()))
    }

    /// Read a file, refusing anything this build cannot interpret.
    pub fn from_text(text: &str) -> Result<Self, FormatError> {
        Self::from_text_with(text, &pending::steps())
    }

    /// Read a file, applying the given pending steps.
    ///
    /// The steps are the ones this build has. The seam is here so that the
    /// reader's rule — *only the changes a file has not had* — can be tested
    /// before a change exists: a test reads a file with a step of its own and
    /// sees which steps the file is given. §81.
    pub fn from_text_with(text: &str, steps: &[pending::Step]) -> Result<Self, FormatError> {
        // A hostile or truncated file must not reach `serde_json::from_str`
        // with a megabyte of input. The cap is generous for a document and
        // small enough to bound the parse.
        if text.len() > MAX_FILE_BYTES {
            return Err(FormatError::TooLarge { bytes: text.len(), limit: MAX_FILE_BYTES });
        }
        let mut file: Self = serde_json::from_str(text).map_err(|e| FormatError::NotJson(e.to_string()))?;
        file.check()?;
        // Pending rewrites run before validation, because a `step` is exactly
        // the thing that fixes an old file's shape into this version's.
        //
        // Only the changes the file has **not** had: a file this build wrote
        // already has them, and applying them again would rewrite it. §81.
        let already = file.meta.pending.clone();
        pending::apply_steps(&mut file.document, &already, steps);
        file.document.validate().map_err(|e| FormatError::InvalidDocument(e.to_string()))?;
        // The document's id counter is deliberately not stored: it is derived
        // from the ids that *are* stored, so a file cannot claim an id that
        // collides with its own next one. Forgetting this line is the single
        // easiest way to make a loaded design hand out an id that already
        // belongs to a node — so it is here, in the reader, and tested.
        file.document.reseat_id_counter();
        Ok(file)
    }
}

/// A writer that stops the serialisation at a limit.
///
/// It exists so that the limit is applied **while** the text is being built.
/// A document that serialises to 75 MiB must not be built in memory and then
/// refused — that is §49's mistake (the cap checked after the bytes are
/// already in memory) wearing the writer's clothes.
struct LimitedWriter {
    /// What has been written so far, never more than `max` bytes.
    out: Vec<u8>,
    /// The limit.
    max: usize,
    /// How many bytes the serialiser *asked* for, including the refused ones.
    /// This is what the error reports: the size the file would have had.
    seen: usize,
}

impl LimitedWriter {
    fn new(max: usize) -> Self {
        Self { out: Vec::new(), max, seen: 0 }
    }
}

impl std::io::Write for LimitedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.seen = self.seen.saturating_add(data.len());
        if self.seen > self.max {
            // Refuse the whole chunk: nothing past the limit is kept, so the
            // buffer never grows past `max`.
            return Err(std::io::Error::other("the text is over the limit"));
        }
        self.out.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_model::Document;

    #[test]
    fn the_tag_names_this_build() {
        assert_eq!(tag(1), "craftkit/1");
        assert_eq!(Format::new().version().expect("valid"), CURRENT);
    }

    /// The writer refuses a text that is over the limit, and says which limit.
    ///
    /// §51: the reader stops at `MAX_FILE_BYTES`, and the writer used to have
    /// no limit at all — so it could write a file the reader would refuse.
    #[test]
    fn the_writer_refuses_a_text_over_the_limit() {
        let file = File::new(Document::empty("x"));
        let err = file.to_text_bounded(64).expect_err("refused");
        assert!(matches!(err, FormatError::TooLargeToWrite { limit: 64, .. }), "{err}");
        assert!(err.to_string().contains("the limit is 64"), "{err}");
    }

    /// The limit stops the text **while it is being written**.
    ///
    /// §49's lesson: a cap checked after the bytes are in memory has already
    /// paid for what it prevents. So the writer keeps no more than the limit,
    /// whatever it is asked for.
    #[test]
    fn the_writer_keeps_no_more_than_the_limit() {
        let mut w = LimitedWriter::new(10);
        let _ = std::io::Write::write(&mut w, b"0123456789abcdef");
        assert!(w.out.len() <= 10, "the writer kept {} bytes of a 10-byte limit", w.out.len());
        assert!(w.seen > 10, "the writer did not see the whole chunk it refused");
    }

    #[test]
    fn this_builds_own_file_round_trips() {
        let mut doc = Document::empty("Round trip");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        let file = File::new(doc.clone());
        let text = file.to_text().expect("serialize");
        assert!(text.ends_with('\n'), "saved files end in one newline");
        let back = File::from_text(&text).expect("parse");
        assert_eq!(back.document, doc);
        assert_eq!(back.format, file.format);
    }

    /// A file is given only the changes it has not had.
    ///
    /// §81: the reader used to apply every pending change to every file,
    /// including the files this build had just written — so a file did not
    /// read back as it was written, and the round-trip tests caught it the
    /// moment a real pending change existed.
    ///
    /// The step below is synthetic and the file's header names it, so the
    /// rule is tested **now**, while there is still nothing pending: a file
    /// that says it has had the change is left alone, and a file that does not
    /// is given it.
    #[test]
    fn a_file_is_given_only_the_changes_it_has_not_had() {
        fn rename(doc: &mut Document) {
            if let Some(node) = doc.nodes.first_mut() {
                node.name = "Renamed".to_string();
            }
        }
        let steps: &[pending::Step] = &[("a_change", rename)];

        let mut doc = Document::empty("x");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);

        // A file that has not had the change is given it.
        let mut file = File::new(doc.clone());
        file.meta.pending = Vec::new();
        let text = file.to_text().expect("json");
        let back = File::from_text_with(&text, steps).expect("parse");
        assert_eq!(
            back.document.nodes.first().map(|n| n.name.clone()).unwrap_or_default(),
            "Renamed",
            "a file that had not had the change was not given it"
        );

        // A file that has had it is left alone.
        let mut file = File::new(doc.clone());
        file.meta.pending = vec!["a_change".to_string()];
        let text = file.to_text().expect("json");
        let back = File::from_text_with(&text, steps).expect("parse");
        assert_eq!(
            back.document.nodes.first().map(|n| n.name.clone()).unwrap_or_default(),
            "A",
            "a change was applied to a file that had already had it"
        );
    }

    #[test]
    fn a_newer_file_is_refused_rather_than_guessed_at() {
        let mut file = File::new(Document::empty("x"));
        file.format = tag(CURRENT + 1);
        let err = File::from_text(&file.to_text().expect("json")).expect_err("refused");
        assert!(matches!(err, FormatError::TooNew { found } if found == CURRENT + 1), "{err}");
        assert!(err.is_version());
        assert!(err.to_string().contains("newer craftkit"), "{err}");
    }

    #[test]
    fn an_older_file_is_read() {
        let mut file = File::new(Document::empty("old"));
        file.format = tag(1);
        assert!(file.check().is_ok());
    }

    #[test]
    fn a_foreign_tag_is_refused() {
        let mut file = File::new(Document::empty("x"));
        file.format = "somethingelse/9".to_string();
        let err = file.check().expect_err("refused");
        assert!(matches!(err, FormatError::NotOurs(_)), "{err}");
    }

    #[test]
    fn a_zero_or_unparsable_version_is_refused() {
        let mut file = File::new(Document::empty("x"));
        file.format = "craftkit/0".to_string();
        assert!(matches!(file.check(), Err(FormatError::TooOld(_))));
        file.format = "craftkit/next".to_string();
        assert!(matches!(file.check(), Err(FormatError::TooOld(_))));
    }

    #[test]
    fn a_file_whose_tag_was_removed_is_told_so() {
        // Everything else about the file is valid, so the only thing wrong with
        // it is the tag — and the message must say *that*, not "missing field".
        let mut file = File::new(Document::empty("x"));
        file.format = String::new();
        let text = file.to_text().expect("json");
        let err = File::from_text(&text).expect_err("refused");
        assert!(matches!(err, FormatError::MissingTag), "{err}");
        assert!(err.to_string().contains("not a craftkit file"), "{err}");
    }

    #[test]
    fn something_that_is_not_a_file_at_all_says_so() {
        for text in ["{}", "[]", "null", "\"hello\"", "42"] {
            let err = File::from_text(text).expect_err("refused");
            assert!(err.to_string().contains("not valid JSON"), "for {text}: {err}");
        }
    }

    #[test]
    fn invalid_json_is_refused_with_the_parser_message() {
        let err = File::from_text("{ not json").expect_err("refused");
        assert!(matches!(err, FormatError::NotJson(_)));
    }

    #[test]
    fn a_file_whose_document_is_invalid_is_refused() {
        // Two nodes with the same id: the model refuses it, and the reader must
        // not hand a corrupt document to the editor.
        let mut doc = Document::empty("dupes");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        let id = doc.nodes[0].id;
        doc.nodes.push(doc.nodes[0].clone());
        doc.nodes[1].id = id;
        let mut file = File::new(doc);
        file.format = tag(CURRENT);
        let text = file.to_text().expect("json");
        let err = File::from_text(&text).expect_err("refused");
        assert!(matches!(err, FormatError::InvalidDocument(_)), "{err}");
    }

    #[test]
    fn an_oversized_file_is_refused_before_it_is_parsed() {
        let big = "x".repeat(65 * 1024 * 1024);
        let err = File::from_text(&big).expect_err("refused");
        assert!(err.to_string().contains("limit is"), "{err}");
    }

    /// A file carrying a name the grammar cannot write is refused.
    ///
    /// The document is deserialized straight into the model, so nothing
    /// checked the label on the way in — and the reader used to hand the
    /// editor a property that no path could name. Measured: a file with the
    /// label `a/b` loaded, published the path `a/b`, and
    /// `prop.set { path: "a/b" }` created a **second** property instead of
    /// changing the one on screen. §88.
    #[test]
    fn a_file_carrying_a_name_the_grammar_cannot_write_is_refused() {
        for label in ["a/b", "@5", "#2"] {
            let mut doc = Document::empty("by hand");
            let _ = doc.add_node("In", craftkit_model::NodeKind::Source);
            doc.nodes[0].props.push(craftkit_model::Property::new(1, label, craftkit_core::Value::Num(2.0)));
            let mut file = File::new(doc);
            file.format = tag(CURRENT);
            let text = file.to_text().expect("json");
            let err = File::from_text(&text).expect_err("refused");
            assert!(matches!(err, FormatError::InvalidDocument(_)), "for {label}: {err}");
            assert!(err.to_string().contains("cannot be written as a path"), "for {label}: {err}");
        }
    }

    /// A file carrying a keyframe list that breaks the model's rules is
    /// refused.
    ///
    /// The keyframes were the one part of a document the reader did **not**
    /// check: §73 checked their values, and nothing checked their times,
    /// their easings or the list itself. Measured, before this was refused:
    /// a file carrying an unsorted list loaded, validated `Ok` — and the
    /// animation sampled to the **wrong value** (keys at 500 and 0 gave the
    /// value of the later key, where the sorted list gives the interpolation
    /// between them). A file carrying a duplicated time, a tick outside the
    /// range a reply could carry, an easing the model refuses, or more keys
    /// than `MAX_KEYS` all loaded as well. §91.
    #[test]
    fn a_file_carrying_broken_keyframes_is_refused() {
        // The list is written by hand, because the model cannot build one: the
        // point of the check is that the *file* can.
        let file_with = |keys: &str| {
            format!(
                r#"{{
  "format": "{tag}",
  "meta": {{ "written_by": "probe", "written_at": 0, "pending": [] }},
  "document": {{
    "meta": {{ "name": "probe", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" }},
    "nodes": [
      {{
        "id": 1,
        "name": "In",
        "kind": "source",
        "props": {{
          "id": "props",
          "label": "Properties",
          "nodes": [
            {{
              "Prop": {{
                "id": 1,
                "label": "amount",
                "value": {{ "t": "num", "v": 0.0 }},
                "keys": [{keys}],
                "locked": false,
                "hint": "Number"
              }}
            }}
          ]
        }},
        "muted": false
      }}
    ]
  }}
}}"#,
                tag = tag(CURRENT)
            )
        };
        let key = |time: &str, value: f64, ease: &str| {
            format!(r#"{{ "time": {time}, "value": {{ "t": "num", "v": {value} }}, "ease": {ease} }}"#)
        };

        let cases: Vec<(String, String)> = vec![
            (
                "an unsorted list".to_string(),
                file_with(&format!("{},{}", key("500", 1.0, "\"linear\""), key("0", 2.0, "\"linear\""))),
            ),
            (
                "two keys at the same time".to_string(),
                file_with(&format!("{},{}", key("0", 1.0, "\"linear\""), key("0", 2.0, "\"linear\""))),
            ),
            (
                "a time outside the range a reply can carry".to_string(),
                file_with(&key("18446744073709551616", 1.0, "\"linear\"")),
            ),
            (
                "an easing the model refuses".to_string(),
                file_with(&key("0", 1.0, r#"{"bezier":{"out_x":5.0,"out_y":1.0}}"#)),
            ),
            (
                "a non-finite easing".to_string(),
                file_with(&key("0", 1.0, r#"{"bezier":{"out_x":1e40,"out_y":1.0}}"#)),
            ),
        ];

        for (what, text) in cases {
            let err = File::from_text(&text).expect_err("refused");
            assert!(matches!(err, FormatError::InvalidDocument(_)), "for {what}: {err}");
            assert!(err.to_string().contains("amount"), "for {what}: {err}");
        }
    }

    #[test]
    fn the_meta_block_lists_the_pending_changes() {
        let meta = Meta::now();
        assert!(meta.written_by.starts_with("craftkit "), "{:?}", meta.written_by);
        assert_eq!(meta.pending, pending::names().into_iter().map(String::from).collect::<Vec<_>>());
    }
}
