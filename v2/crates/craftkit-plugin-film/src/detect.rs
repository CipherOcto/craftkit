//! The **format** of a caption file.
//!
//! The tree's own rule: the extension of the file and the content of
//! it say together what the file is, and a file they do not name is
//! not a caption file.
//!
//! ```text
//! the-captions.srt   the head has an arrow   → subrip
//! the-captions.scc   the head says Scenarist → scc
//! the-captions.stl   the bytes say 850       → stl
//! ```
//!
//! # The rules
//!
//! - The **extension** of the file must be one of the extensions of a
//!   caption file, and a file whose extension is not one is not a
//!   caption file at all.
//! - The **EBU STL file** says what it is in its bytes, and the head of
//!   a text file says it in its first words: `WEBVTT`, `Scenarist_SCC`
//!   or `File Format=MacCaption_MCC`.
//! - A **SubRip file** is a file whose head holds a timing line — two
//!   clocks and an arrow between them — and a file of an extension of a
//!   caption file that holds no timing line is not a caption file of
//!   that format.

/// The **extensions** of a caption file: the tree's own list.
const EXTENSIONS: [&str; 9] = ["srt", "vtt", "scc", "webvtt", "mcc", "stl", "ttml", "dfxp", "xml"];

/// The most **bytes** of the head of a file that are looked at.
const HEAD: usize = 4096;

/// The **format** of a caption file, by the tree's own rule.
///
/// `None` when the file is not a caption file this tree reads.
pub fn detect(path: &str, bytes: &[u8]) -> Option<&'static str> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.trim_start_matches('.').to_ascii_lowercase());
    let extension = extension?;
    if !EXTENSIONS.contains(&extension.as_str()) {
        return None;
    }
    // The **EBU STL file**: its bytes say what it is.
    if crate::stl::sniff(bytes) {
        return Some("stl");
    }
    // The **head** of the file: the first four kilobytes of it.
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(HEAD)])
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_string();
    if head.starts_with("WEBVTT") {
        return Some("webvtt");
    }
    if head.starts_with("Scenarist_SCC") {
        return Some("scc");
    }
    if head.starts_with("File Format=MacCaption_MCC") {
        return Some("mcc");
    }
    // The **TTML file**: the head of it says so.
    if head.starts_with('<')
        && (head.contains("<tt ") || head.contains("<tt>") || head.contains(":tt "))
        && (head.contains("ns/ttml") || head.contains("ttaf1"))
    {
        return Some("ttml");
    }
    // The **timing line**: the arrow of the clocks.
    let timing = head.lines().take(40).any(crate::captions::timing_line);
    match extension.as_str() {
        "srt" if timing || head.is_empty() => Some("subrip"),
        "vtt" if timing => Some("webvtt"),
        // The extension of a caption file, and no timing line in it: it
        // is not a caption file of that format.
        "srt" | "vtt" | "scc" | "webvtt" | "mcc" | "stl" => None,
        // The **TTML** and the **DFXP** files are read, and their head
        // says so: a file of one of those extensions that is not one
        // is not a caption file of that format.
        "ttml" | "dfxp" | "xml" => None,
        _ if timing => Some("subrip"),
        _ => None,
    }
}
