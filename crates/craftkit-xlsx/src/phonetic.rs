//! The **reading** of a text: the phonetic guide a file may hold
//! beside it.
//!
//! A file may hold a **reading** of the text of a cell — the furigana
//! of a Japanese text, the pinyin of a Chinese one — beside the text
//! itself: a **run** of the reading over a part of the text.
//!
//! A run says where it lies with two **offsets** into the text, counted
//! in UTF-16 code units, and holds the reading itself:
//!
//! ```text
//! <rPh sb="0" eb="2"><t>トウキョウ</t></rPh>
//! ```
//!
//! A run that cannot refer to the text — one without both offsets, one
//! whose offsets are the wrong way round, one that reaches past the end
//! of the text, one with no reading — is **not read**: the tree says how
//! many it left out rather than repairing it. A repaired run is a run
//! that says something the file did not say.
//!
//! The **properties** of a reading — the font it is written in, the
//! kind of reading it is, and how it sits over the text — are read as
//! they are written, and a kind or an alignment that is not one the
//! format defines is not read.

use craftkit_xml::El;

/// A **run** of a reading: the reading of a part of a text.
#[derive(Debug, Clone, PartialEq)]
pub struct PhoneticRun {
    /// Where the run starts, counted in UTF-16 code units.
    pub start: u32,
    /// Where the run ends, counted in UTF-16 code units.
    pub end: u32,
    /// The reading itself.
    pub text: String,
}

/// The **properties** of a reading.
#[derive(Debug, Clone, PartialEq)]
pub struct PhoneticProps {
    /// The **font** of the reading: the number of the font in the
    /// styles of the workbook.
    pub font_id: Option<u32>,
    /// The **kind** of the reading: how the file says it is written.
    pub kind: Option<String>,
    /// The **alignment** of the reading over the text.
    pub alignment: Option<String>,
}

/// The **reading** of a text.
#[derive(Debug, Clone, PartialEq)]
pub struct Phonetic {
    /// The runs, in the order the file holds them.
    pub runs: Vec<PhoneticRun>,
    /// The properties of the reading.
    pub props: Option<PhoneticProps>,
}

/// The **kinds** of reading the format defines.
const KINDS: &[&str] = &["noConversion", "hiragana", "katakana", "alpha", "number"];

/// The **alignments** of a reading the format defines.
const ALIGNMENTS: &[&str] = &["left", "center", "right", "distributed", "justify"];

/// Read the **reading** of a text, and say how many runs were not read.
///
/// `base` is the text the reading is over: the offsets of a run are
/// counted in it, and a run that reaches past its end is not read.
pub fn read(element: &El, base: &str) -> (Option<Phonetic>, usize) {
    let length = u32::try_from(base.encode_utf16().count()).unwrap_or(u32::MAX);
    let mut runs = Vec::new();
    let mut dropped = 0usize;
    for run in element.kids("rPh") {
        let (Some(start), Some(end)) = (run.attr_u32("sb"), run.attr_u32("eb")) else {
            dropped += 1;
            continue;
        };
        // The run must lie within the text: an offset past the end of
        // the text is not an offset the text has.
        if start >= end || end > length {
            dropped += 1;
            continue;
        }
        let text = run.child("t").map(|t| t.text.clone()).unwrap_or_default();
        if text.is_empty() {
            dropped += 1;
            continue;
        }
        runs.push(PhoneticRun { start, end, text });
    }
    let props = element.child("phoneticPr").map(props_of);
    if runs.is_empty() && props.is_none() {
        return (None, dropped);
    }
    (Some(Phonetic { runs, props }), dropped)
}

/// The **properties** of a reading, as the file writes them.
fn props_of(element: &El) -> PhoneticProps {
    PhoneticProps {
        font_id: element.attr_u32("fontId"),
        kind: element.attr("type").filter(|kind| KINDS.contains(kind)).map(|kind| kind.to_string()),
        alignment: element
            .attr("alignment")
            .filter(|alignment| ALIGNMENTS.contains(alignment))
            .map(|alignment| alignment.to_string()),
    }
}
