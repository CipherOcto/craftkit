//! Readers that turn bytes into text, within a bound.
//!
//! # Why the bound is in the reader
//!
//! Every input this stack accepts has a limit, and the limit is nearly always
//! checked in the **parser**: the line is read, the file is read, and only
//! then does the code look at how long it is.
//!
//! That order spends the memory the limit was supposed to prevent. Measured
//! in this tree: a 200 MiB line on the MCP server's stdin was refused
//! correctly — *"the line is 209715208 bytes; the limit is 4194304"* (the
//! limit was 4 MiB then; it is derived now) — after
//! the process had already grown to 203 MiB of resident memory. A 300 MiB
//! design file was refused by the file reader after the process had already
//! read all 300 MiB. The cap was real, and the memory was spent anyway.
//!
//! A cap is only a cap if it is applied **before** the allocation. So the
//! bounded readers live here, in the layer whose job is *"the readers that
//! turn hostile input into typed values"*, and every server and file reader
//! in the stack uses them rather than re-implementing the same mistake.
//!
//! # The bound is on the text, and the text is not the memory
//!
//! A reader here bounds the **bytes it keeps**. The parser then turns those
//! bytes into a tree, and the tree is up to [`JSON_EXPANSION`] times bigger:
//! a JSON number is two bytes of text and thirty-two bytes of memory,
//! because a value has to carry its own type. So a limit of *n* bytes of
//! text is a limit of about *32n* bytes of memory, and the limits that use
//! these readers are derived from a memory budget rather than chosen on
//! their own. §64.

use std::io::{BufRead, Read};
use std::path::Path;

/// Why a bounded read could not produce a line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// The input was longer than the limit.
    ///
    /// `len` is how long the input is **when that is known**: the file reader
    /// knows the file's size before it reads a byte of it, and reports that.
    /// A stream reader stops at the limit, so it cannot know how much was
    /// left — it reports the bytes it read, which is the limit plus one.
    ///
    /// The message says what is true in both cases. It used to say *"the
    /// input is {len} bytes"*, which was the file's size for one reader and
    /// the limit plus one for the other, so a caller that printed it was
    /// told a length that was not the input's. §175.
    #[error("the input is longer than the limit of {max} bytes")]
    TooLong { len: usize, max: usize },
    /// The bytes could not be read.
    ///
    /// The kind is kept rather than the error itself, so a caller can tell a
    /// timeout from a real failure — and so this type can be `Clone` and
    /// `PartialEq`, like every other error in this crate.
    #[error("could not read the line: {message}")]
    Io { kind: std::io::ErrorKind, message: String },
    /// The bytes were not text.
    #[error("the line is not valid UTF-8")]
    NotUtf8,
}

impl ReadError {
    /// Whether the read failed because the line was too long.
    pub fn is_too_long(&self) -> bool {
        matches!(self, ReadError::TooLong { .. })
    }

    /// The line's length, when it was too long.
    pub fn too_long_len(&self) -> Option<usize> {
        match self {
            ReadError::TooLong { len, .. } => Some(*len),
            _ => None,
        }
    }
}

/// Read one line from `reader`, refusing one longer than `max` bytes.
///
/// The limit is applied **while reading**: the bytes past the limit are
/// counted and dropped rather than kept, so a line of any length costs at
/// most `max` bytes of memory — which is the whole point, because a cap
/// checked after the read has already paid for what it was meant to prevent.
///
/// That bounds the **text**. The parser then turns the text into a tree, and
/// the tree is up to [`crate::JSON_EXPANSION`] times bigger: a JSON number is
/// two bytes of text and thirty-two bytes of memory, because a value has to
/// carry its own type. So a line of `max` bytes can cost `max *
/// JSON_EXPANSION` bytes of memory, and the limits that use this reader are
/// chosen against the memory budget rather than against the text. §64.
///
/// The rest of an over-long line is still **read**, so the reader is left at
/// the start of the next line and a server that carries on after refusing one
/// stays in step with its client. The bytes are dropped, not the line.
///
/// `Ok(None)` means the input ended before a line did. The returned line has
/// no trailing newline.
pub fn read_line_bounded<R: BufRead>(reader: &mut R, max: usize) -> Result<Option<String>, ReadError> {
    let mut out: Vec<u8> = Vec::new();
    let mut content_len = 0usize;
    let mut ended = false;
    loop {
        let (found, used) = {
            let available = reader.fill_buf().map_err(io_error)?;
            if available.is_empty() {
                ended = true;
                break;
            }
            match available.iter().position(|byte| *byte == b'\n') {
                Some(index) => {
                    content_len = content_len.saturating_add(index);
                    keep(&mut out, &available[..index], max);
                    (true, index + 1)
                }
                None => {
                    content_len = content_len.saturating_add(available.len());
                    keep(&mut out, available, max);
                    (false, available.len())
                }
            }
        };
        reader.consume(used);
        if found {
            break;
        }
    }
    if content_len > max {
        return Err(ReadError::TooLong { len: content_len, max });
    }
    if out.is_empty() && ended {
        return Ok(None);
    }
    String::from_utf8(out).map(Some).map_err(|_| ReadError::NotUtf8)
}

/// Keep at most `max` bytes of `chunk`, and no more.
///
/// The buffer this fills is the memory a long line would cost, so its size
/// is the property — not the error that eventually comes of it. A line that is
/// too long is *counted* in full and *kept* in part.
fn keep(out: &mut Vec<u8>, chunk: &[u8], max: usize) {
    let room = max.saturating_sub(out.len());
    let take = room.min(chunk.len());
    out.extend_from_slice(&chunk[..take]);
}

/// An [`std::io::Error`] as a [`ReadError`].
fn io_error(source: std::io::Error) -> ReadError {
    ReadError::Io { kind: source.kind(), message: source.to_string() }
}

/// Read a whole input into a string, refusing one larger than `max` bytes.
///
/// The reader is cut off at the limit, so an input of any size costs at most
/// `max` bytes of memory. This is the difference between a cap and a cap
/// *after the allocation*: a file that is too big is refused without ever
/// being read into memory.
pub fn read_to_string_bounded<R: Read>(reader: R, max: usize) -> Result<String, ReadError> {
    let mut out = String::new();
    reader.take(max as u64 + 1).read_to_string(&mut out).map_err(io_error)?;
    if out.len() > max {
        return Err(ReadError::TooLong { len: out.len(), max });
    }
    Ok(out)
}

/// Read a file into a string, refusing one larger than `max` bytes.
///
/// The file's own size is reported in the error, because a file that is too
/// big is a fact about the file and not about how much of it was read.
pub fn read_file_bounded(path: &Path, max: usize) -> Result<String, ReadError> {
    let file = std::fs::File::open(path).map_err(io_error)?;
    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
    read_to_string_bounded(file, max).map_err(|e| match e {
        ReadError::TooLong { max, .. } => {
            ReadError::TooLong { len: usize::try_from(size).unwrap_or(usize::MAX), max }
        }
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    fn reader(text: &str) -> BufReader<&[u8]> {
        BufReader::new(text.as_bytes())
    }

    #[test]
    fn a_line_is_read_without_its_newline() {
        let mut r = reader("hello\nworld\n");
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), Some("hello".to_string()));
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), Some("world".to_string()));
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), None);
    }

    #[test]
    fn a_last_line_without_a_newline_is_still_a_line() {
        let mut r = reader("hello");
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), Some("hello".to_string()));
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), None);
    }

    #[test]
    fn an_empty_line_is_a_line() {
        let mut r = reader("\n");
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), Some(String::new()));
    }

    #[test]
    fn a_line_at_the_limit_is_read_and_one_over_is_refused() {
        let ten = "x".repeat(10);
        let mut r = reader(&ten);
        assert_eq!(read_line_bounded(&mut r, 10).expect("read"), Some(ten));

        let eleven = "x".repeat(11);
        let mut r = reader(&eleven);
        let err = read_line_bounded(&mut r, 10).expect_err("too long");
        assert!(err.is_too_long(), "{err}");
        assert_eq!(err, ReadError::TooLong { len: 11, max: 10 });
    }

    /// The point of the whole module: the memory is not spent.
    #[test]
    fn a_long_line_is_refused_without_being_kept() {
        // Ten megabytes with no newline. The reader must stop keeping the
        // bytes at the limit.
        let huge = "x".repeat(10 * 1024 * 1024);
        let mut r = reader(&huge);
        let err = read_line_bounded(&mut r, 1024).expect_err("too long");
        assert!(err.is_too_long(), "{err}");
        assert_eq!(err.too_long_len(), Some(10 * 1024 * 1024));
    }

    /// A server that carries on after refusing a line must stay in step.
    #[test]
    fn an_over_long_line_does_not_swallow_the_next_one() {
        let mut r = reader("aaaaaaaaaaaa\nshort\n");
        let err = read_line_bounded(&mut r, 4).expect_err("too long");
        assert!(err.is_too_long(), "{err}");
        assert_eq!(read_line_bounded(&mut r, 100).expect("read"), Some("short".to_string()));
    }

    #[test]
    fn bytes_that_are_not_text_are_an_error_and_not_a_panic() {
        let mut r = BufReader::new(&[0xffu8, 0xfe, 0x0a][..]);
        let err = read_line_bounded(&mut r, 100).expect_err("not text");
        assert_eq!(err, ReadError::NotUtf8);
    }

    /// The buffer a long line fills is the memory it costs, so its size is
    /// the property — and it is one a test can see exactly.
    #[test]
    fn the_bytes_kept_never_exceed_the_limit() {
        let mut out: Vec<u8> = Vec::new();
        keep(&mut out, &[b'x'; 100], 10);
        assert_eq!(out.len(), 10);
        keep(&mut out, &[b'y'; 100], 10);
        assert_eq!(out.len(), 10, "the second chunk must not grow the buffer");
    }

    /// A reader that counts the bytes it hands out.
    struct Counting {
        data: Vec<u8>,
        position: usize,
    }

    impl Read for Counting {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let left = self.data.len() - self.position;
            let n = left.min(buf.len());
            buf[..n].copy_from_slice(&self.data[self.position..self.position + n]);
            self.position += n;
            Ok(n)
        }
    }

    /// The bound is applied **to the read**, not after it.
    ///
    /// This is the property the whole module exists for, and it is the one a
    /// test can see exactly: the reader is never asked for more than the
    /// limit. A cap checked after the read would leave `position` at the
    /// whole megabyte.
    #[test]
    fn a_bounded_read_never_asks_for_more_than_the_limit() {
        let mut counting = Counting { data: vec![b'x'; 1024 * 1024], position: 0 };
        let err = read_to_string_bounded(&mut counting, 10).expect_err("too long");
        assert!(err.is_too_long(), "{err}");
        assert!(counting.position <= 11, "the reader took {} bytes of a megabyte", counting.position);
    }

    #[test]
    fn a_bounded_read_returns_a_short_input_whole() {
        let mut counting = Counting { data: b"hello".to_vec(), position: 0 };
        assert_eq!(read_to_string_bounded(&mut counting, 100).expect("read"), "hello");
        assert_eq!(counting.position, 5);
    }
}
