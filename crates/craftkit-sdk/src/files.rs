//! Reading a **file**, within limits.
//!
//! # The problem
//!
//! A document arrives as a file, and a file arrives as **bytes**. Every
//! bound in this kit so far has been about the *values* inside a
//! document — a command's parameters (§61's payload reader), a node's
//! payload (§61), a metric's tables (§60) — and every one of them is
//! applied **after** the bytes are in memory.
//!
//! That is the wrong order for the biggest of them. A file of ten
//! gigabytes is read into a `String` before anything has had a chance
//! to look at it, and the refusal that follows is a refusal of the
//! *reading*, not of the *file*: the memory has already been spent.
//!
//! So the bound that matters most is on the **bytes**, and it is
//! checked **before** the read, on the file's own length.
//!
//! # The limit is the caller's
//!
//! A program that opens a document needs one limit, a program that
//! opens a bundle of media files needs another, and a program that
//! opens a video needs a third. The kit does not know which one a
//! program is: so the limit is an **argument**, and this module is the
//! mechanism every program uses.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::path::Path;

/// The most bytes a file may hold.
///
/// The limit is the **caller's**: this module does not have an opinion
/// about what a program opens.
pub type ByteLimit = u64;

/// Read a file, if it is not longer than the limit.
///
/// The file's own length is checked **first**, so a file that is too
/// big is refused without being read.
pub fn read_bounded(path: &Path, limit: ByteLimit) -> Result<Vec<u8>, String> {
    let length = length_of(path)?;
    if length > limit {
        return Err(format!("`{}` is {length} bytes, and the limit is {limit}", path.display()));
    }
    std::fs::read(path).map_err(|error| format!("`{}` could not be read: {error}", path.display()))
}

/// Read a file as text, if it is not longer than the limit.
///
/// The same bound, and the same check before the read.
pub fn read_text_bounded(path: &Path, limit: ByteLimit) -> Result<String, String> {
    let length = length_of(path)?;
    if length > limit {
        return Err(format!("`{}` is {length} bytes, and the limit is {limit}", path.display()));
    }
    std::fs::read_to_string(path).map_err(|error| format!("`{}` could not be read: {error}", path.display()))
}

/// Read the **head** of a file, at most the limit.
///
/// Some files are big and only their head is wanted: a picture's header
/// says how big the picture is, and the pixels are not needed for that.
/// The reader reads what it was asked for and stops.
///
/// The head is **not** a smaller file: a file whose header is not in the
/// bytes is a file this call cannot measure, and the caller is told so.
///
/// The **second** value says whether the reader **stopped**: the file is
/// longer than the limit, and there is more of it than was read. A file
/// that is exactly as long as the limit is not cut — there is nothing
/// beyond the bound to have missed.
pub fn read_prefix_bounded(path: &Path, limit: ByteLimit) -> Result<(Vec<u8>, bool), String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("`{}` could not be read: {error}", path.display()))?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; 8192];
    let mut read: u64 = 0;
    while read < limit {
        let want = ((limit - read) as usize).min(chunk.len());
        let count = file
            .read(&mut chunk[..want])
            .map_err(|error| format!("`{}` could not be read: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..count]);
        read += count as u64;
    }
    // Whether there is **more** of the file than the limit: one byte past
    // the limit says so, and a file that is exactly as long as the limit
    // has nothing beyond it.
    let mut beyond = [0u8; 1];
    let stopped = match file.read(&mut beyond) {
        Ok(count) => count > 0,
        Err(error) => return Err(format!("`{}` could not be read: {error}", path.display())),
    };
    Ok((buffer, stopped))
}

/// The length of a file, in bytes.
///
/// A file that is not there is a refusal, and so is one whose length
/// cannot be had: a pipe, say, or a file that is being written while
/// it is read. A file whose length cannot be known is not read through
/// this module — the caller may read it another way, and say so.
fn length_of(path: &Path) -> Result<u64, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("`{}` could not be read: {error}", path.display()))?;
    Ok(metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file with some bytes in it.
    fn temporary(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = craftkit_scratch::file(&format!("craftkit-files-{name}")).expect("the scratch is made");
        std::fs::write(&path, bytes).expect("the file is written");
        path
    }

    #[test]
    fn a_file_that_fits_is_read() {
        let path = temporary("fits", b"hello");
        let bytes = read_bounded(&path, 1024).expect("the file is read");
        assert_eq!(bytes, b"hello");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_is_too_big_is_refused() {
        let path = temporary("too-big", &[0; 2048]);
        let error = read_bounded(&path, 1024).expect_err("that file is too big");
        assert!(error.contains("limit"), "{error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_text_file_that_fits_is_read() {
        let path = temporary("text-fits", "hello".as_bytes());
        let text = read_text_bounded(&path, 1024).expect("the file is read");
        assert_eq!(text, "hello");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_text_file_that_is_too_big_is_refused() {
        let path = temporary("text-too-big", &[b'x'; 2048]);
        let error = read_text_bounded(&path, 1024).expect_err("that file is too big");
        assert!(error.contains("limit"), "{error}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_is_not_there_is_refused() {
        let path = craftkit_scratch::file("craftkit-files-not-there").expect("the scratch is made");
        let error = read_bounded(&path, 1024).expect_err("there is no such file");
        assert!(error.contains("could not be read"), "{error}");
    }

    #[test]
    fn a_head_that_fits_is_read_and_is_not_cut() {
        let path = temporary("head-fits", b"hello");
        let (bytes, stopped) = read_prefix_bounded(&path, 1024).expect("the head is read");
        assert_eq!(bytes, b"hello");
        assert!(!stopped, "a file inside the bound is not cut: {stopped}");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_head_that_is_too_big_is_read_and_says_it_stopped() {
        let path = temporary("head-too-big", &[0; 2048]);
        let (bytes, stopped) = read_prefix_bounded(&path, 1024).expect("the head is read");
        assert_eq!(bytes.len(), 1024, "the head is as long as the bound");
        assert!(stopped, "a file beyond the bound is cut");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_that_is_exactly_as_long_as_the_bound_is_not_cut() {
        let path = temporary("head-exact", &[0; 1024]);
        let (bytes, stopped) = read_prefix_bounded(&path, 1024).expect("the head is read");
        assert_eq!(bytes.len(), 1024, "the whole file is read");
        assert!(!stopped, "there is nothing beyond the bound to have missed");
        let _ = std::fs::remove_file(&path);
    }
}
