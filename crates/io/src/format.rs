//! The file format, versioned.
//!
//! # The tag
//!
//! Every file starts with `"format": "craftkit/N"`, where `N` is the version.
//! A reader refuses a version it does not know rather than guessing, because a
//! guessed read of a newer file silently corrupts a design.
//!
//! # The pending list, and the other direction
//!
//! A format change that alters what is saved does **not** bump the version —
//! that is the whole point of the pending scheme. So the version cannot be the
//! only thing that says whether this build understands a file: the file's
//! `meta.pending` list says which changes had been applied to it, and a change
//! this build has never heard of means the document is in a shape this build
//! does not know.
//!
//! The reader applies the changes this build has that the file has not had.
//! It also refuses a file that names a change this build does not have: the
//! parts of the document that change had put there are dropped without a word,
//! and the next save writes the file back without them. §95.
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

    /// The file names a format change this build has never heard of.
    ///
    /// A file's `meta.pending` list says which format changes had been applied
    /// to it. A change this build does not have means the file was written by a
    /// build whose shape this build does not know — so the parts of the
    /// document that change had put there are dropped without a word, and the
    /// next save writes them back without them.
    ///
    /// §95: the reader looked at the changes a file still needed and never at
    /// the ones it had, so a file like this was read like any other.
    #[error(
        "this file has had the format change `{0}`, and this build has never heard of it. \
         It was written by a build with a format change this build does not have; \
         open it with that build."
    )]
    UnknownChange(String),

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
        version_of(&self.format)
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

/// The version a tag names.
///
/// One rule, in one place: the tag is either ours and carries a version, or
/// it is something else. The reader and the change check both ask this
/// question, and two spellings of it is how they would come to disagree.
fn version_of(tag: &str) -> Result<u32, FormatError> {
    if tag.trim().is_empty() {
        return Err(FormatError::MissingTag);
    }
    let Some(rest) = tag.strip_prefix(&format!("{TAG_PREFIX}/")) else {
        return Err(FormatError::NotOurs(tag.to_string()));
    };
    // The version is **digits and nothing else**, written the way the tree
    // writes it.
    //
    // `u32::from_str` accepts a leading `+`, so `"craftkit/+1"` parsed as
    // version 1 — and a version the tree has never written, read as a
    // version this build recognises. The document says the opposite: *"a
    // reader refuses a version it does not know rather than guessing,
    // because a guessed read of a newer file silently corrupts a design."*
    // §261.
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return Err(FormatError::TooOld(tag.to_string()));
    }
    // And no leading zeros: `"craftkit/01"` is a spelling of one, and not the
    // one. §261.
    if rest.len() > 1 && rest.starts_with('0') {
        return Err(FormatError::TooOld(tag.to_string()));
    }
    rest.parse::<u32>()
        .map_err(|_| FormatError::TooOld(tag.to_string()))
        .and_then(|v| if v == 0 { Err(FormatError::TooOld(tag.to_string())) } else { Ok(v) })
}

/// The format changes a file names that a build may not read.
///
/// `version` is the version the file claims, `current` the version this build
/// writes, `already` the file's own list and `known` the changes this build
/// has. §95.
///
/// A file's list names the changes that had been applied to it, on top of the
/// shape its version's base defines. A build at the **same** version knows
/// every change that can have been applied, so a name it has never heard of
/// means the file was written by a different build at the same version — and
/// the document is in a shape this build does not know.
///
/// A build at a **later** version may know a change it has in no list any
/// more: the change was folded into a version between the file's and this
/// build's. This build does not keep the folded names, so it cannot tell that
/// from a change it has never heard of — and refusing every such file would
/// refuse every design written before the last release. So an earlier file is
/// read, and what a folded change is stays the open question it is.
fn unknown_changes(version: u32, current: u32, already: &[String], known: &[&str]) -> Vec<String> {
    if version != current {
        return Vec::new();
    }
    pending::unknown_names(already, known)
}

impl Default for Format {
    fn default() -> Self {
        Self::new()
    }
}

/// How many bytes of file text one byte of document memory costs, measured.
///
/// A design file and the design it holds are the same thing in two
/// currencies: the file's **text** and the document's **memory**. The number
/// that converts one into the other is not a constant of the language but a
/// property of this format — and it used to be a property of the *design*,
/// because the file was indented by the design's depth. Measured over the
/// shapes a document can take, the text was 2.5 times the memory for a flat
/// design and 9.9 times it for a deep one, and the tree's file cap was
/// smaller than the file some designs inside the model's own budget needed:
/// 311 MiB of file for a 31 MiB design. §141.
///
/// The writer's indentation now stops growing
/// ([`craftkit_foundation::writer::MAX_INDENT_DEPTH`]), so the factor is a constant of the
/// format rather than a function of the design. Measured over the same
/// shapes, the worst case is **1.51** bytes of text per byte of memory — on a
/// 64-bit machine.
///
/// **And the estimate is a fact about the machine**, so the factor is the worst
/// case over the **machines**, not over one of them. On a 32-bit target the same
/// design is estimated a third smaller — the estimate counts the tree's own
/// structs, and a pointer is four bytes there — while its file is the same file,
/// and the ratio there measured **2.247**. §231.
///
/// So the number is **3**: a constant, not a measurement, and the worst case over
/// every platform the tree runs on. A cap that is the worst case over one
/// machine is a cap that is wrong on another, and the tree holds a design it
/// cannot save when that happens.
pub const FILE_EXPANSION: usize = 3;

/// How many bytes a design file may be.
///
/// Derived from the model's own budget, the way every other limit in the tree
/// is derived: a design may hold
/// [`craftkit_model::limits::MAX_DOCUMENT_BYTES`] of memory, and its file may
/// be at most [`FILE_EXPANSION`] times that.
///
/// **96 MiB** — 32 MiB of design and three times that in text. The number this
/// cap has always been was 64 MiB, and it was a consequence rather than a
/// choice; it is a consequence now too, and it cannot drift away from the budget
/// it protects. The other half of the derivation is what makes the two agree:
/// every document the model accepts has a file inside this cap, so the tree
/// can hold a design and save it. §141.
///
/// **The cost of the wider factor is the reader's bound**, because the same
/// number is applied while reading: a file of up to 96 MiB may cost up to
/// 96 MiB to read, where it was 64 MiB before. §231.
///
/// It is applied **while reading**, in [`crate::store::load`], so that a file
/// of any size costs at most this much memory — a cap checked after the read
/// has already paid for what it was meant to prevent. §49.
pub const MAX_FILE_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * FILE_EXPANSION;

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
    ///
    /// The version, and the format changes the file names that this build has
    /// never heard of. §95.
    pub fn check(&self) -> Result<(), FormatError> {
        self.check_with(&pending::names())
    }

    /// The same check, against a known set of format changes.
    ///
    /// The seam is [`pending::Step`]s: a test that drives the reader with a
    /// step of its own has to be able to say that the file's header names a
    /// change this reader does have — otherwise the refusal of §95 would
    /// refuse the test that §81 needs. §81, §95.
    pub fn check_with(&self, known: &[&str]) -> Result<(), FormatError> {
        Format { format: self.format.clone(), meta: self.meta.clone() }.check()?;
        self.check_changes(known)
    }

    /// The format changes this build has, plus the ones the seam hands in.
    fn known_changes(steps: &[pending::Step]) -> Vec<&'static str> {
        let mut known = pending::names();
        for (name, _) in steps {
            if !known.contains(name) {
                known.push(name);
            }
        }
        known
    }

    /// Refuse a file that names a format change this build has never heard of.
    ///
    /// A file at the current version may only name the changes this build has.
    /// The file's document is in the "after" shape of the changes it names, and
    /// a change this build does not have means it does not know that shape:
    /// the parts of the document it does not recognise are dropped without a
    /// word, and the next save writes them back without them. §95.
    fn check_changes(&self, known: &[&str]) -> Result<(), FormatError> {
        let version = version_of(&self.format)?;
        match unknown_changes(version, CURRENT, &self.meta.pending, known).first() {
            Some(name) => Err(FormatError::UnknownChange(name.clone())),
            None => Ok(()),
        }
    }

    /// The JSON this build writes: one item per line, with the indentation
    /// bounded, and a trailing newline.
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
        match craftkit_foundation::writer::write_json(&mut out, self) {
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
        file.check_with(&File::known_changes(steps))?;
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
    use craftkit_model::{Document, NodeKind, PropGroup, Property};

    #[test]
    fn the_tag_names_this_build() {
        assert_eq!(tag(1), "craftkit/1");
        assert_eq!(Format::new().version().expect("valid"), CURRENT);
    }

    /// The file's cap is the design's own budget, not a number somebody chose.
    ///
    /// §141. The cap used to be 64 MiB written out as a literal, and the two
    /// numbers it has to agree with — the model's budget for a document, and
    /// the format's expansion from a document to its text — were nowhere in
    /// the same place. A literal that happens to be the right number today is
    /// a number that can be wrong tomorrow.
    #[test]
    fn the_file_cap_is_the_model_budget_times_the_formats_expansion() {
        assert_eq!(
            MAX_FILE_BYTES,
            craftkit_model::limits::MAX_DOCUMENT_BYTES * FILE_EXPANSION,
            "the file's cap is not the design's own budget times the format's expansion"
        );
    }

    /// A design's file is at most [`FILE_EXPANSION`] times the design's memory.
    ///
    /// This is the property that makes the file's cap and the model's budget
    /// agree: if a design's file could be bigger than the cap, the tree would
    /// hold designs it could not save. §141.
    ///
    /// The shapes are the ones a design can take — many small nodes, long text
    /// values, keyframes, and keyframes inside the deepest property tree the
    /// model allows — and the worst of them measured 1.51 before the writer's
    /// indentation was bounded. It was 9.9.
    ///
    /// **And it is a property of the machine it runs on.** The estimate counts
    /// the tree's own structs, so on a 32-bit target it is a third smaller while
    /// the file is the same file, and this test is what says so there: it failed
    /// on the device with the cap at 2, and the ratio there is 2.247. §231.
    #[test]
    fn a_designs_file_is_bounded_by_the_design_it_holds() {
        for (name, doc) in shapes() {
            let text = File::new(doc.clone()).to_text().expect("the design is written");
            let memory = doc.estimated_bytes();
            assert!(
                text.len() <= FILE_EXPANSION * memory,
                "{name}: the file is {} bytes and the design holds {memory} bytes",
                text.len()
            );
        }
    }

    /// The expansion covers the worst case over the platforms the tree runs on.
    ///
    /// The property above is checked **per platform**, and the platforms do not
    /// agree: the same design is estimated 1.498 times its file on a 64-bit
    /// host and 2.247 times its file on a 32-bit ARM target, measured. A cap
    /// that is the worst case over one machine is wrong on another, and the
    /// tree holds a design it cannot save when that happens. §231.
    ///
    /// **The number is written down**, because a constant cannot be a
    /// measurement: the ratio is measured, and the constant has to be a number
    /// the tree can hold still.
    #[test]
    fn the_expansion_covers_the_worst_case_over_the_platforms() {
        // The measured worst case, in ten-thousandths: 2.2466 bytes of file
        // for every byte of estimate, on a 32-bit ARM target.
        const MEASURED_WORST_X_10_000: usize = 22_467;
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same way `crates/model/src/lib.rs` asserts its own.
        let (expansion, worst) = (FILE_EXPANSION, MEASURED_WORST_X_10_000);
        assert!(
            expansion * 10_000 >= worst,
            "the expansion is {expansion}, and the worst case measured over the platforms the \
             tree runs on is 2.2466 — a 64-bit host measures 1.498 and a 32-bit ARM target 2.247"
        );
    }

    /// The shapes a design can take, each big enough that the file's own head
    /// does not decide the ratio.
    fn shapes() -> Vec<(&'static str, Document)> {
        let mut out: Vec<(&'static str, Document)> = Vec::new();

        // Many small nodes: the file is verbose per node.
        {
            let mut doc = Document::empty("Nodes");
            for n in 0..2_000u64 {
                let id = doc.add_node(format!("node{n}"), NodeKind::Filter).expect("node");
                let mut g = PropGroup::new("props", "Properties");
                for p in 0..3u64 {
                    g.push(Property::new(p + 1, format!("p{p}"), craftkit_foundation::Value::Num(1.0)));
                }
                doc.get_mut(id).expect("just added").props = g;
            }
            out.push(("many small nodes", doc));
        }

        // Long text values: the file is the same size as the design.
        {
            let text = "x".repeat(65_000);
            let mut doc = Document::empty("Text");
            let id = doc.add_node("wide", NodeKind::Source).expect("node");
            let mut g = PropGroup::new("props", "Properties");
            for p in 0..64u64 {
                g.push(Property::new(p + 1, format!("t{p}"), craftkit_foundation::Value::Text(text.clone())));
            }
            doc.get_mut(id).expect("just added").props = g;
            out.push(("text values", doc));
        }

        // Keyframes, and the same keyframes inside the deepest property tree
        // the model allows: the shape the unbounded indentation hurt most.
        for (name, depth) in [("keyframes", 0usize), ("keyframes in the deepest tree", 15)] {
            let mut doc = Document::empty("Keys");
            let id = doc.add_node("animated", NodeKind::Cue).expect("node");
            let mut g = PropGroup::new("props", "Properties");
            for p in 0..8u64 {
                let mut prop = Property::new(p + 1, format!("p{p}"), craftkit_foundation::Value::Num(1.0));
                for t in 0..2_000i128 {
                    let Ok(key) = craftkit_model::Key::new(
                        craftkit_core::Tick::from_raw(t * 10_584_000),
                        craftkit_foundation::Value::Num(t as f64),
                        craftkit_model::Easing::Linear,
                    ) else {
                        break;
                    };
                    if prop.keys.set(key).is_err() {
                        break;
                    }
                }
                g.push(prop);
            }
            let mut inner = g;
            for d in 0..depth {
                let mut group = PropGroup::new(format!("g{d}"), format!("Group {d}"));
                group.push_group(inner);
                inner = group;
            }
            doc.get_mut(id).expect("just added").props = inner;
            out.push((name, doc));
        }

        out
    }

    /// The writer's own limit still stops a design whose file would be too big.
    ///
    /// §51. The writer and the reader share the cap, so a file the writer
    /// writes is a file the reader can read back. §141 bounded the
    /// indentation, so a design *the model accepts* now always fits — but the
    /// writer does not know what the model accepts, and the cap is what it
    /// has.
    #[test]
    fn a_design_whose_file_would_be_too_big_is_not_written() {
        let doc = a_design_bigger_than_a_file();
        let err = File::new(doc).to_text().expect_err("the writer refuses it");
        assert!(matches!(err, FormatError::TooLargeToWrite { limit: MAX_FILE_BYTES, .. }), "{err}");
    }

    /// A design whose file is bigger than the cap.
    ///
    /// Eighty properties with 10 000 keyframes on each: 73.3 MiB of memory and
    /// 105.1 MiB of text, past the 96 MiB cap. The model's own budget would
    /// refuse it too, but the writer is asked on its own here — `File::to_text`
    /// does not validate the design, and the cap is what the writer has.
    fn a_design_bigger_than_a_file() -> Document {
        let mut doc = Document::empty("Big");
        let id = doc.add_node("n", NodeKind::Source).expect("node");
        let mut group = PropGroup::new("props", "Properties");
        for p in 0..80u64 {
            let mut prop = Property::new(p + 1, format!("p{p}"), craftkit_foundation::Value::Num(1.0));
            for t in 0..10_000i128 {
                let Ok(key) = craftkit_model::Key::new(
                    craftkit_core::Tick::from_raw(t),
                    craftkit_foundation::Value::Num(t as f64),
                    craftkit_model::Easing::Linear,
                ) else {
                    break;
                };
                if prop.keys.set(key).is_err() {
                    break;
                }
            }
            group.push(prop);
        }
        doc.get_mut(id).expect("just added").props = group;
        doc
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

    /// A file that names a format change this build has never heard of is
    /// refused.
    ///
    /// §95: the reader looked at the changes a file still **needed** and never
    /// at the ones it had **already had**. A file naming a change this build
    /// does not have was written by a build whose shape this build does not
    /// know, and the reader took it anyway.
    ///
    /// Measured, before this was refused: the file below loaded without a word
    /// of complaint, its node's `nickname` — a field this build has never heard
    /// of — was dropped on the way in, and the file written back had no trace
    /// of it. The version tag could not catch it either, because a pending
    /// change does not bump the version: the file said `craftkit/1`, exactly
    /// like every file this build writes.
    #[test]
    fn a_file_that_names_a_change_this_build_has_never_heard_of_is_refused() {
        let mut file = File::new(Document::empty("x"));
        file.meta.pending = vec!["a_change_this_build_has_never_heard_of".to_string()];
        let text = file.to_text().expect("json");

        let err = File::from_text(&text).expect_err("refused");
        assert!(
            matches!(err, FormatError::UnknownChange(ref name) if name == "a_change_this_build_has_never_heard_of"),
            "{err}"
        );
        assert!(err.to_string().contains("a_change_this_build_has_never_heard_of"), "{err}");
        assert!(err.to_string().contains("never heard of it"), "{err}");
    }

    /// A file that names a change this build **has** is read.
    ///
    /// The other half of the rule above, and the reason the check is written
    /// against a known set rather than against the build's own list: a test
    /// that drives the reader with a step of its own has to be able to say
    /// that the file's header names a change this reader does have. §81.
    #[test]
    fn a_file_that_names_a_change_this_build_has_is_read() {
        fn rename(doc: &mut Document) {
            if let Some(node) = doc.nodes.first_mut() {
                node.name = "Renamed".to_string();
            }
        }
        let steps: &[pending::Step] = &[("a_change", rename)];

        let mut doc = Document::empty("x");
        let _ = doc.add_node("A", craftkit_model::NodeKind::Source);
        let mut file = File::new(doc);
        file.meta.pending = vec!["a_change".to_string()];
        let text = file.to_text().expect("json");

        let back = File::from_text_with(&text, steps).expect("read");
        assert_eq!(back.document.nodes.first().map(|n| n.name.clone()).unwrap_or_default(), "A");
    }

    /// A file from an earlier version may name a change that has been folded.
    ///
    /// The half of §95 that is still open: a change folded into a later
    /// version is not in this build's pending list any more, so a file from
    /// before the fold names a change this build cannot find — and refusing
    /// every such file would refuse every design written before the last
    /// release.
    ///
    /// So an earlier file is read, and what a folded change *is* — and whether
    /// the reader has to know the older shapes by hand — is the open question
    /// about the folded format change. It is written down rather than guessed
    /// at, and this test is where it will change.
    ///
    /// The rule is asked with the two versions it is about, because the tree
    /// has no version earlier than the current one: version 1 is the first, so
    /// there is no earlier file to write. §95 measured that too — the test
    /// that claimed to read an older file was tagging one with the *current*
    /// version, and so was reading a file of its own age.
    #[test]
    fn an_earlier_file_may_name_a_change_that_was_folded() {
        assert!(
            unknown_changes(1, 2, &["a_change_folded".to_string()], &[]).is_empty(),
            "a file from before the fold may name the change the fold carried"
        );
        assert_eq!(
            unknown_changes(2, 2, &["a_change_folded".to_string()], &[]),
            vec!["a_change_folded".to_string()],
            "a file at the current version may not name a change this build has never heard of"
        );
    }

    /// A file whose group id is longer than a name is refused.
    ///
    /// §97: a group's id is a path segment, and the model had no length rule
    /// for it — so a file could carry a group whose id was thirty thousand
    /// bytes. The document was read, it validated, and the listing published a
    /// path of thirty thousand bytes that no command would carry. The property
    /// label beside it has had the rule all along.
    #[test]
    fn a_file_with_a_group_id_longer_than_a_name_is_refused() {
        let id = "a".repeat(craftkit_model::limits::MAX_NAME + 1);
        let text = format!(
            r#"{{
  "format": "craftkit/1",
  "meta": {{ "written_by": "probe", "written_at": 0, "pending": [] }},
  "document": {{
    "meta": {{ "name": "probe", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" }},
    "nodes": [
      {{ "id": 1, "name": "In", "kind": "source",
        "props": {{ "id": "props", "label": "Properties", "nodes": [
          {{ "Group": {{ "id": "{id}", "label": "{id}", "nodes": [
            {{ "Prop": {{ "id": 5, "label": "amount", "value": {{ "t": "num", "v": 1.0 }} }} }}
          ]}} }}
        ]}},
        "muted": false }}
    ]
  }}
}}"#
        );
        let err = File::from_text(&text).expect_err("refused");
        assert!(err.to_string().contains("group id"), "{err}");
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

    /// A file at the current version is read.
    ///
    /// This test used to be called `an_older_file_is_read` and to tag the file
    /// with version 1 — which is the **current** version, because version 1 is
    /// the first version there is. So it was reading a file of its own age and
    /// claiming to read an older one. There is no older version to write, and
    /// that is worth saying out loud: the tree has never read a file written
    /// by an earlier craftkit, and cannot until version 2 exists. The rule
    /// about an earlier version is `unknown_changes`, and it is tested there
    /// with the two versions the rule is about. §95.
    #[test]
    fn a_file_at_the_current_version_is_read() {
        let mut file = File::new(Document::empty("old"));
        file.format = tag(CURRENT);
        assert!(file.check().is_ok());
    }

    #[test]
    fn a_foreign_tag_is_refused() {
        let mut file = File::new(Document::empty("x"));
        file.format = "somethingelse/9".to_string();
        let err = file.check().expect_err("refused");
        assert!(matches!(err, FormatError::NotOurs(_)), "{err}");
    }

    /// The version is digits and nothing else. §261.
    ///
    /// The defect this test exists for: `u32::from_str` accepts a leading
    /// `+`, so `"craftkit/+1"` parsed as version 1 — a tag no build of the
    /// tree has ever written, read as a version this build recognises. The
    /// document says a reader refuses a version it does not know rather than
    /// guessing.
    #[test]
    fn a_version_that_is_not_a_plain_number_is_refused() {
        for tag in ["craftkit/+1", "craftkit/01", "craftkit/ 1", "craftkit/1 ", "craftkit/"] {
            let mut file = File::new(Document::empty("x"));
            file.format = tag.to_string();
            assert!(matches!(file.check(), Err(FormatError::TooOld(_))), "{tag} was not refused");
        }
        // And the one the tree writes is read.
        let mut file = File::new(Document::empty("x"));
        file.format = tag(CURRENT);
        assert!(file.check().is_ok());
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
        // One byte past the cap, which is the cap and not a number somebody
        // chose: the cap moved in §231 and this test moved with it.
        let big = "x".repeat(MAX_FILE_BYTES + 1);
        let err = File::from_text(&big).expect_err("refused");
        assert!(err.to_string().contains("limit"), "{err}");
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
            doc.nodes[0].props.push(craftkit_model::Property::new(
                1,
                label,
                craftkit_foundation::Value::Num(2.0),
            ));
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

    /// A file carrying a node with an unusable name is refused.
    ///
    /// A node's name is a name: not empty, not blank, not longer than the
    /// limit. The command and the validator used to disagree about that —
    /// `node.add` accepted an empty name and the validator refused it
    /// afterwards, so the user's empty name came back as the tree's own
    /// unexpected failure — and a file could carry the same name. §92.
    #[test]
    fn a_file_carrying_an_unusable_node_name_is_refused() {
        for name in ["", "   "] {
            let mut doc = Document::empty("by hand");
            let _ = doc.add_node("In", craftkit_model::NodeKind::Source);
            doc.nodes[0].name = name.to_string();
            let mut file = File::new(doc);
            file.format = tag(CURRENT);
            let text = file.to_text().expect("json");
            let err = File::from_text(&text).expect_err("refused");
            assert!(matches!(err, FormatError::InvalidDocument(_)), "for {name:?}: {err}");
            assert!(err.to_string().contains("unusable name"), "for {name:?}: {err}");
        }
    }

    #[test]
    fn the_meta_block_lists_the_pending_changes() {
        let meta = Meta::now();
        assert!(meta.written_by.starts_with("craftkit "), "{:?}", meta.written_by);
        assert_eq!(meta.pending, pending::names().into_iter().map(String::from).collect::<Vec<_>>());
    }
}
