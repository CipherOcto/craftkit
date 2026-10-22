//! How the design file is written: the JSON layout, and the bound on it.
//!
//! # Why the indentation stops
//!
//! The file is written pretty — one item per line, indented by its depth —
//! because a person reads it, and because a diff of two saves shows only what
//! changed.
//!
//! But the indentation **grows with the depth of the design**, and the depth
//! is the design's own: a property tree may nest sixteen deep, and a keyframe
//! inside it is written nine levels down. So the file's text was not a
//! constant multiple of the design's memory. Measured over the shapes a
//! document can take, the file was **2.5 times** the design for a flat one and
//! **9.9 times** it for a deep one — and the tree's file cap (64 MiB) was
//! smaller than the file that some designs *inside the model's own budget*
//! needed: 311 MiB of file for a 31 MiB design. The tree could hold designs it
//! could not save. §141.
//!
//! So the indentation stops growing past [`MAX_INDENT_DEPTH`]. The file is
//! still one item per line and still diffs the same way; the lines below that
//! depth are not indented further, and the file's text is at most
//! [`crate::format::FILE_EXPANSION`] times the design's memory — a number
//! that is measured rather than guessed.

use std::io::{self, Write};

use serde::Serialize;
use serde_json::ser::{Formatter, Serializer};

/// How many levels of indentation the file gets.
///
/// Past this, a line is not indented further. The number is a bound on the
/// file's size, not a taste: it is the depth at which the indentation stops
/// costing more than the design it describes. §141.
pub const MAX_INDENT_DEPTH: usize = 4;

/// What one level of indentation is worth.
const INDENT: &[u8] = b"  ";

/// The JSON layout the design file is written in.
///
/// It is [`serde_json`]'s pretty formatter with one change: the indentation
/// does not grow without bound. Everything else — where the newlines go, how
/// a string is escaped, how a number is written — is serde_json's, so the file
/// is the same JSON it always was, and only the spaces at the start of a line
/// differ. §141.
pub struct FileFormatter {
    /// How deep the writer is. The file's own nesting is eight levels deep
    /// before a design's own structure is reached.
    depth: usize,
    /// Whether the container being closed had a value in it.
    ///
    /// An empty array is `[]` and not `[\n]`, and this is how the writer
    /// knows the difference.
    has_value: bool,
}

impl FileFormatter {
    /// A formatter for the design file.
    pub fn new() -> Self {
        Self { depth: 0, has_value: false }
    }

    /// Write the indentation for the current depth, bounded.
    fn write_indent<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        for _ in 0..self.depth.min(MAX_INDENT_DEPTH) {
            w.write_all(INDENT)?;
        }
        Ok(())
    }
}

impl Default for FileFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for FileFormatter {
    fn begin_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.depth = self.depth.saturating_add(1);
        self.has_value = false;
        w.write_all(b"[")
    }

    fn end_array<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.depth = self.depth.saturating_sub(1);
        if self.has_value {
            w.write_all(b"\n")?;
            self.write_indent(w)?;
        }
        w.write_all(b"]")
    }

    fn begin_array_value<W: ?Sized + Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        w.write_all(if first { b"\n" } else { b",\n" })?;
        self.write_indent(w)
    }

    fn end_array_value<W: ?Sized + Write>(&mut self, _w: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }

    fn begin_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.depth = self.depth.saturating_add(1);
        self.has_value = false;
        w.write_all(b"{")
    }

    fn end_object<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        self.depth = self.depth.saturating_sub(1);
        if self.has_value {
            w.write_all(b"\n")?;
            self.write_indent(w)?;
        }
        w.write_all(b"}")
    }

    fn begin_object_key<W: ?Sized + Write>(&mut self, w: &mut W, first: bool) -> io::Result<()> {
        w.write_all(if first { b"\n" } else { b",\n" })?;
        self.write_indent(w)
    }

    fn begin_object_value<W: ?Sized + Write>(&mut self, w: &mut W) -> io::Result<()> {
        w.write_all(b": ")
    }

    fn end_object_value<W: ?Sized + Write>(&mut self, _w: &mut W) -> io::Result<()> {
        self.has_value = true;
        Ok(())
    }
}

/// Write `value` as the design file's JSON.
///
/// The bytes go through `w`, so the caller keeps control of where they land
/// and of how many of them are kept. §49: the limit is applied while the text
/// is being written, not after it is in memory.
pub fn write_json<W: ?Sized + Write, T: Serialize + ?Sized>(
    w: &mut W,
    value: &T,
) -> Result<(), serde_json::Error> {
    let mut ser = Serializer::with_formatter(w, FileFormatter::new());
    value.serialize(&mut ser)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file is one item per line, and the lines below the bound are not
    /// indented further.
    #[test]
    fn the_indentation_stops_growing_at_the_bound() {
        let value = serde_json::json!({ "a": { "b": { "c": { "d": { "e": 1 } } } } });
        let mut out = Vec::new();
        write_json(&mut out, &value).expect("json");
        let text = String::from_utf8(out).expect("utf8");
        let indents: Vec<usize> = text.lines().map(|l| l.len() - l.trim_start().len()).collect();
        let deepest = indents.iter().copied().max().expect("a line");
        assert_eq!(
            deepest,
            MAX_INDENT_DEPTH * INDENT.len(),
            "the deepest line is indented {deepest} spaces: {text}"
        );
    }

    /// The file this build writes is the JSON serde_json would write, with the
    /// indentation bounded.
    ///
    /// The escaping, the numbers and the newlines are serde_json's, so the
    /// only difference between the two writers is the spaces at the start of a
    /// line. A design that fits inside the bound is written exactly as
    /// serde_json writes it.
    #[test]
    fn a_design_inside_the_bound_is_written_as_serde_json_writes_it() {
        let value = serde_json::json!({
            "format": "craftkit/1",
            "meta": { "written_by": "craftkit 0.1.0", "pending": [] },
            "document": {
                "meta": { "name": "x", "fps": 24 },
                "nodes": [ { "id": 1, "name": "n", "kind": "source", "muted": false } ],
            },
        });
        let mut ours = Vec::new();
        write_json(&mut ours, &value).expect("json");
        let theirs = serde_json::to_vec_pretty(&value).expect("json");
        assert_eq!(String::from_utf8(ours).expect("utf8"), String::from_utf8(theirs).expect("utf8"));
    }

    /// A string with a quote in it is escaped the way serde_json escapes it.
    ///
    /// The formatter does not escape anything itself: serde_json hands it the
    /// pieces of a string and asks for them to be written, and the escapes are
    /// serde_json's. A formatter that escaped its own would have to know the
    /// whole table, and a table that is one rule short is a corrupt file.
    #[test]
    fn a_string_with_awkward_characters_is_escaped() {
        let value = serde_json::json!({ "text": "a \"quote\", a \\ and a \n newline" });
        let mut out = Vec::new();
        write_json(&mut out, &value).expect("json");
        let text = String::from_utf8(out).expect("utf8");
        let back: serde_json::Value = serde_json::from_str(&text).expect("the file is json");
        assert_eq!(back, value, "the text did not survive the round trip: {text}");
    }
}
