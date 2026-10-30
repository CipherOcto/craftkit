//! Reading a **line** from a stream, within limits.
//!
//! # The problem
//!
//! Every door in this kit that speaks **newline-delimited JSON** reads
//! its lines with `BufRead::lines()` or `read_line()`, and both read
//! until they find a newline or the end of the stream — with **no
//! limit** on how much they will hold.
//!
//! That is fine for a file (§62 bounded the file) and not fine for a
//! **stream**: a peer that never sends a newline makes the reader
//! allocate until the machine has no memory left, and the reader has
//! no way to say "that is enough" because there is no "enough".
//!
//! So a line has a limit, the limit is checked **as the line is read**,
//! and a line that is over it is refused — and the stream is left
//! broken, because the reader cannot know where the next line starts.
//!
//! # The limit is the caller's
//!
//! A protocol's messages are small, and a protocol knows how small. The
//! kit does not: so the limit is an **argument**, and this module is
//! the mechanism every door uses.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::io::BufRead;

/// The most bytes a line may hold.
pub type LineLimit = usize;

/// Read one line, if it is not longer than the limit.
///
/// The line is read **byte by byte**, so that the limit is checked as
/// it goes and not after the whole line has been in memory. A line that
/// is over the limit is refused, and the bytes that were read are
/// gone: the stream cannot be resynchronised, and a door that sees
/// this closes the connection.
///
/// `Ok(None)` is the end of the stream.
pub fn read_line_bounded(reader: &mut impl BufRead, limit: LineLimit) -> Result<Option<String>, String> {
    let mut line: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = reader.read(&mut byte).map_err(|error| format!("the line could not be read: {error}"))?;
        if read == 0 {
            break;
        }
        if byte[0] == b'\n' {
            break;
        }
        if line.len() >= limit {
            return Err(format!("the line is longer than {limit} bytes"));
        }
        line.push(byte[0]);
    }
    if line.is_empty() {
        // The stream ended without a newline: there is no line.
        return Ok(None);
    }
    let text = String::from_utf8(line).map_err(|_| "the line is not text".to_string())?;
    Ok(Some(text.trim_end_matches('\r').to_string()))
}

/// Every line of a stream, within the limit.
///
/// A line that is over the limit ends the iteration: the stream cannot
/// be resynchronised, so there is nothing after it to read.
pub fn lines_bounded(reader: impl BufRead, limit: LineLimit) -> impl Iterator<Item = Result<String, String>> {
    let mut reader = reader;
    let mut refused = false;
    std::iter::from_fn(move || {
        if refused {
            return None;
        }
        match read_line_bounded(&mut reader, limit) {
            Ok(Some(line)) => Some(Ok(line)),
            Ok(None) => None,
            Err(error) => {
                // The stream cannot be resynchronised: there is nothing
                // after this line to read.
                refused = true;
                Some(Err(error))
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    /// A reader over some text.
    fn reader(text: &str) -> BufReader<std::io::Cursor<Vec<u8>>> {
        BufReader::new(std::io::Cursor::new(text.as_bytes().to_vec()))
    }

    #[test]
    fn a_line_is_read() {
        let mut reader = reader("hello\nworld\n");
        assert_eq!(
            read_line_bounded(&mut reader, 1024).expect("the line is read"),
            Some("hello".to_string())
        );
        assert_eq!(
            read_line_bounded(&mut reader, 1024).expect("the line is read"),
            Some("world".to_string())
        );
        assert_eq!(read_line_bounded(&mut reader, 1024).expect("the end"), None);
    }

    #[test]
    fn a_line_that_is_too_long_is_refused() {
        let mut reader = reader("hello, this line is much too long\n");
        let error = read_line_bounded(&mut reader, 8).expect_err("that line is too long");
        assert!(error.contains("longer"), "{error}");
    }

    #[test]
    fn a_line_with_no_newline_at_all_is_refused() {
        let mut reader = reader("a line that never ends");
        let error = read_line_bounded(&mut reader, 8).expect_err("that line is too long");
        assert!(error.contains("longer"), "{error}");
    }

    #[test]
    fn the_end_of_the_stream_is_not_a_line() {
        let mut reader = reader("");
        assert_eq!(read_line_bounded(&mut reader, 1024).expect("the end"), None);
    }

    #[test]
    fn every_line_is_read() {
        let mut reader = reader("one\ntwo\nthree\n");
        let lines: Vec<String> =
            lines_bounded(&mut reader, 1024).map(|line| line.expect("the line is read")).collect();
        assert_eq!(lines, vec!["one".to_string(), "two".to_string(), "three".to_string()]);
    }

    #[test]
    fn a_line_that_is_too_long_ends_the_iteration() {
        let mut reader = reader("ok\nthis line is much too long\nafter\n");
        let lines: Vec<Result<String, String>> = lines_bounded(&mut reader, 8).collect();
        assert_eq!(lines.len(), 2, "the good line, and then the refusal");
        assert!(lines[0].is_ok());
        assert!(lines[1].is_err());
    }
}
