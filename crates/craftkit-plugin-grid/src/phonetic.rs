//! The **reading** of a text: the phonetic guide over it.
//!
//! A file may hold a **reading** of the text of a cell — the furigana
//! of a Japanese text, the pinyin of a Chinese one — beside the text
//! itself: a **run** of the reading over a part of the text.
//!
//! A run says where it lies with two **offsets** into the text, counted
//! in UTF-16 code units, and holds the reading itself.
//!
//! The reading is a fact of the **text** and not of the cell, but the
//! cell is what carries the text, so the cell carries the reading too.

use craftkit_sdk::serde_json::{Value as Json, json};

/// A **run** of a reading: the reading of a part of a text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhoneticRun {
    /// Where the run starts, counted in UTF-16 code units.
    pub start: u32,
    /// Where the run ends, counted in UTF-16 code units.
    pub end: u32,
    /// The reading itself.
    pub text: String,
}

/// The **properties** of a reading.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhoneticProps {
    /// The **font** of the reading: the number of the font in the
    /// styles of the workbook.
    pub font_id: Option<u32>,
    /// The **kind** of the reading: how it is written.
    pub kind: Option<String>,
    /// The **alignment** of the reading over the text.
    pub alignment: Option<String>,
}

/// The **reading** of a text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Phonetic {
    /// The runs, in the order they come.
    pub runs: Vec<PhoneticRun>,
    /// The properties of the reading.
    pub props: Option<PhoneticProps>,
}

impl Phonetic {
    /// The reading, as the answer holds it.
    pub fn to_json(&self) -> Json {
        let runs: Vec<Json> = self
            .runs
            .iter()
            .map(|run| json!({ "start": run.start, "end": run.end, "text": run.text }))
            .collect();
        let props = match &self.props {
            Some(props) => json!({
                "font_id": props.font_id,
                "kind": props.kind,
                "alignment": props.alignment,
            }),
            None => Json::Null,
        };
        json!({ "runs": runs, "props": props })
    }

    /// The reading, from the answer that holds it.
    ///
    /// A reading that is not one this tree can read is nothing: the
    /// text is the text, and a reading that cannot be read is not a
    /// reading.
    pub fn from_json(value: &Json) -> Option<Phonetic> {
        let runs = value.get("runs")?.as_array()?;
        let mut read = Vec::new();
        for run in runs {
            let start = run.get("start")?.as_u64()?;
            let end = run.get("end")?.as_u64()?;
            let text = run.get("text")?.as_str()?;
            read.push(PhoneticRun {
                start: u32::try_from(start).ok()?,
                end: u32::try_from(end).ok()?,
                text: text.to_string(),
            });
        }
        let props = match value.get("props") {
            Some(Json::Object(_)) => {
                let font_id = value
                    .get("props")
                    .and_then(|props| props.get("font_id"))
                    .and_then(|font| font.as_u64())
                    .and_then(|font| u32::try_from(font).ok());
                let kind = value
                    .get("props")
                    .and_then(|props| props.get("kind"))
                    .and_then(|kind| kind.as_str())
                    .map(|kind| kind.to_string());
                let alignment = value
                    .get("props")
                    .and_then(|props| props.get("alignment"))
                    .and_then(|alignment| alignment.as_str())
                    .map(|alignment| alignment.to_string());
                Some(PhoneticProps { font_id, kind, alignment })
            }
            _ => None,
        };
        Some(Phonetic { runs: read, props })
    }
}
