//! The tree's one home for seeing the tree: the walk, and the readers.
//!
//! Every gate that has to look at the tree's files needs the same walk, and
//! two walks are two rules about what a gate sees. Every gate that has to
//! open a file needs the same reader, and two readers are two rules about
//! what a file looks like when a gate reads it.
//!
//! # Where this came from
//!
//! This file is a **copy that was folded back together**, and it says so
//! because a copy that forgets where it came from is a copy that drifts.
//!
//! The assets gate had the walk **twice** — `walk` and `collect`, near
//! identical — and the lists gate had it **three** times: `collect_manifests`,
//! `collect_markdown` and `collect_rust`. §168 folded the assets gate's two
//! into this one and wrote that *"there is now one, in `xtask/src/tree.rs`,
//! and both gates use it"*. **That was false when it was written**: the same
//! commit left the lists gate's three behind.
//!
//! And they had already drifted, which is the whole argument for one walk:
//!
//! | The walk | Ignores |
//! |---|---|
//! | this one | `target`, `.git`, `node_modules`, `dist`, `fuzz-out`, and every dot-directory |
//! | `collect_manifests` | `target`, `node_modules`, and every dot-directory — **not `dist`** |
//! | `collect_markdown`, `collect_rust` | `target`, `node_modules`, `dist`, and every dot-directory |
//!
//! A gate that walks the tree and a gate that walks the tree differently do
//! not see the same tree, and the difference is a file nobody looked at. §170
//! folded the three in and added the gate that keeps it that way.
//!
//! `fuzz-out` is in the list for a reason of its own: it holds the fuzzer's
//! broken inputs, which are files of bytes nobody wrote. A gate that read one
//! of them would be reading a thing that is *meant* to be nonsense — and the
//! cleanroom gate, which looks for another product's name in the tree, would
//! find one in a mutation sooner or later. §180.
//!
//! # The readers, and where they came from
//!
//! §205. The readers were in the gates module — `read_text` and `read_rust`
//! beside `RUST_READING_GATES` — and the **rule** they were written for said
//! the tree had two ways to read a file and no third. The rule was about one
//! module: the gate that held it up counted `std::fs::read_to_string` in
//! `xtask/src/lists.rs` and expected two, and the xtask had **thirteen** reads
//! spelled that way and **four** spelled `std::fs::read`, in **eight** of its
//! **fourteen** modules. The clean-room gate read **every file in the tree**,
//! every Rust file among them, as bytes.
//!
//! So the readers came here, beside the walk: a reader is a way of seeing the
//! tree and not a gate, and the walk was already here for the same reason.
//! The gate now walks every module of the xtask instead of naming one, and
//! checks **both** spellings of a raw read, because a claim about the tree
//! that counts one spelling is a claim about a spelling.
//!
//! A reader is also where the tree's **subject** lives. `read_text` and
//! `read_bytes` refuse a Rust file outright, and `read_rust` asks which view
//! of it the caller wants: the code the tree ships, or the file as it is. The
//! subject is an argument, so it cannot be forgotten, and a reader that
//! refuses is a reader that cannot be misused by accident.

use std::path::{Path, PathBuf};

/// Paths never inspected: build output and version control.
pub const IGNORED_DIRS: &[&str] = &["target", ".git", "node_modules", "dist", "fuzz-out"];

/// Every file under `root` that `want` accepts, as paths relative to `root`.
///
/// Hand-written rather than done with a glob crate: it is twenty lines, and it
/// lets the tests run on a temporary directory with no filesystem assumptions
/// beyond "a directory is a directory".
///
/// A directory that cannot be read is skipped, not fatal: a permission error on
/// one path should not fail a gate about the whole repository. So this cannot
/// fail, and it does not pretend that it can.
pub fn walk(root: &Path, dir: &Path, found: &mut Vec<PathBuf>, want: &dyn Fn(&Path) -> bool) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if IGNORED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            walk(root, &path, found, want);
        } else if want(&path)
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_path_buf());
        }
    }
}

/// The offset of the delimiter that closes the one at `open`.
///
/// Strings are stepped over, so a parenthesis inside one does not count, and
/// the depth never goes below one, so a stray closer cannot underflow.
pub fn matching(source: &str, open: usize, left: u8, right: u8) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut at = open;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at];
        if in_string {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else if c == b'"' {
            in_string = true;
        } else if c == left {
            depth += 1;
        } else if c == right {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(at);
            }
        }
        at += 1;
    }
    None
}

/// The source with its comments, its string contents and its char literals
/// blanked out, and every line kept.
///
/// This is the tree's **one** way to read code out of a file, and there is
/// one because §149 measured what happens when there are three: the rule that
/// a mention in a string is not code was written three times — as
/// `code_only`, as `without_comments` and as `strip_code` — and the gates
/// that read a *symbol* or a *module* used none of them, so a test that
/// stood only inside a string satisfied a document's claim about it. A rule
/// written three times is three chances to forget it.
///
/// Every line is kept, because a check that names a line has to name the
/// **file's** line: dropping a block comment's newlines moved the line of
/// every constant after it, and the gate that reported the line was wrong by
/// the size of the comment.
///
/// A **char literal** is blanked too, because a brace inside one is not a
/// brace: the scanner that reads a type's members counts braces, and a
/// `'{'` in the source would open a body that is not there. A **lifetime**
/// is not a char literal: `fn f<'a>(x: &'a str)` has one, and treating it as
/// a literal would swallow the rest of the file.
///
/// The view is the **same length** as the file, byte for byte, so a position
/// in one is a position in the other. That is what lets a gate read the
/// *shape* of a thing from the code and its *contents* from the text: a
/// command's id is a string, and a string is the subject there, but a `pub
/// static` in a comment is not a command. §155.
///
/// It is **public** because it is the tree's one reader and another gate
/// needs it: a rule written twice is two chances to forget it, and the
/// assets gate had a copy of the idea in it — a shape read out of the text.
/// §166.
pub fn code_lines_kept(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_block = false;
    let mut in_line = false;
    while let Some(c) = chars.next() {
        if in_line {
            if c == '\n' {
                in_line = false;
                out.push('\n');
            } else {
                blank(&mut out, c);
            }
            continue;
        }
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
                // Two bytes for two bytes: the view keeps the file's length.
                out.push(' ');
                out.push(' ');
            } else if c == '\n' {
                out.push('\n');
            } else {
                blank(&mut out, c);
            }
            continue;
        }
        if in_string {
            if escaped {
                // A **continuation**: a backslash at the end of a line inside
                // a string. The line it ends is still a line of the file, and
                // a check that names a line has to name the file's line —
                // the tree's own `crates/io/src/format.rs` has three of them.
                blank(&mut out, c);
                escaped = false;
                continue;
            }
            match c {
                '\\' => {
                    escaped = true;
                    out.push(' ');
                }
                '"' => {
                    in_string = false;
                    out.push(' ');
                }
                '\n' => out.push('\n'),
                _ => blank(&mut out, c),
            }
            continue;
        }
        match c {
            'r' if !ends_with_word_byte(&out) => {
                // A raw string: `r"…"`, `r#"…"#`. The scanner that blanks the
                // contents of an ordinary string would end a raw one at the
                // first quote inside it and read the rest of the file as
                // code. §158.
                let mut ahead = chars.clone();
                let mut hashes = 0usize;
                while ahead.peek() == Some(&'#') {
                    ahead.next();
                    hashes += 1;
                }
                if ahead.peek() != Some(&'"') {
                    // Not a raw string: an identifier that starts with `r`.
                    out.push(c);
                    continue;
                }
                out.push(c);
                blank_a_raw_string(&mut out, &mut chars, hashes);
            }
            'b' if !ends_with_word_byte(&out) => {
                // A byte string. `b"…"` is an ordinary string to the
                // scanner; `br"…"` is a raw one.
                let mut ahead = chars.clone();
                let mut hashes = 0usize;
                let raw = ahead.peek() == Some(&'r');
                if raw {
                    ahead.next();
                    while ahead.peek() == Some(&'#') {
                        ahead.next();
                        hashes += 1;
                    }
                }
                if raw && ahead.peek() == Some(&'"') {
                    out.push(c);
                    // The `r` of `br` is code, and it keeps its place.
                    chars.next();
                    out.push('r');
                    blank_a_raw_string(&mut out, &mut chars, hashes);
                    continue;
                }
                out.push(c);
            }
            '"' => {
                in_string = true;
                out.push(' ');
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line = true;
                out.push(' ');
                out.push(' ');
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block = true;
                out.push(' ');
                out.push(' ');
            }
            '\'' => {
                // A char literal, or a lifetime.
                if !opens_a_char_literal(&chars) {
                    out.push('\'');
                    continue;
                }
                out.push(' ');
                while let Some(c) = chars.next() {
                    if c == '\\' {
                        out.push(' ');
                        if let Some(next) = chars.next() {
                            blank(&mut out, next);
                        }
                        continue;
                    }
                    if c == '\'' {
                        out.push(' ');
                        break;
                    }
                    blank(&mut out, c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Whether the output so far ends with a word byte.
///
/// A raw string starts with an `r` that is not part of an identifier:
/// `r#"…"#` is a raw string and `result` is not.
fn ends_with_word_byte(out: &str) -> bool {
    out.as_bytes().last().is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
}

/// Blank the contents of a raw string.
///
/// A raw string — `r"…"`, `r#"…"#` — has a quote in it, and the scanner that
/// blanks an ordinary string's contents would end it at the first quote
/// inside and read the rest of the file as code. The hashes and the quotes
/// are code; the contents are not. The closing delimiter is a quote followed
/// by as many hashes as the opening one had. §158.
fn blank_a_raw_string(out: &mut String, chars: &mut std::iter::Peekable<std::str::Chars<'_>>, hashes: usize) {
    // The **opening** delimiter: as many hashes as the closing one has, and a
    // quote. They are code, and they are *consumed* here — the loop below must
    // see the contents and not the delimiter again, or every position after it
    // is off by one byte.
    for _ in 0..hashes {
        chars.next();
        out.push('#');
    }
    chars.next();
    out.push('"');
    while let Some(next) = chars.next() {
        if next == '"' {
            let mut probe = chars.clone();
            let mut seen = 0usize;
            while seen < hashes && probe.peek() == Some(&'#') {
                probe.next();
                seen += 1;
            }
            if seen == hashes {
                out.push('"');
                for _ in 0..hashes {
                    out.push('#');
                    chars.next();
                }
                break;
            }
        }
        blank(out, next);
    }
}

/// Blank a character, keeping the file's byte count.
///
/// A character can be more than one byte, and the code view has to be the
/// same length as the file: a position in one is a position in the other.
/// §155.
fn blank(out: &mut String, c: char) {
    if c == '\n' {
        out.push('\n');
        return;
    }
    for _ in 0..c.len_utf8() {
        out.push(' ');
    }
}

/// Whether the quote just read opens a char literal rather than a lifetime.
///
/// `'a'` is a literal and `'a` is a lifetime. The difference is the quote that
/// closes it: a literal is one character — or an escape — between two quotes,
/// and a lifetime is a name with no closing quote. The escape counts because
/// `'\''` is three characters and `'a'` is one, and the two have to be told
/// apart the same way.
fn opens_a_char_literal(chars: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut ahead = chars.clone();
    match ahead.next() {
        Some('\\') => true,
        Some(_) => ahead.next() == Some('\''),
        None => false,
    }
}

/// Whether a byte is part of an identifier.
pub fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Which view of a Rust file a gate's subject is about.
///
/// §204. A gate that reads the tree's Rust source has to say whether it is
/// asking about **the code the tree ships** or about **the file as it is**.
/// The difference is a test: a constant that only a test declares is not a
/// bound of the tree, a struct that only a test declares is not a type of it,
/// and a line that only a test owns is not a line of the tree.
///
/// The subject is an **argument**, so it cannot be forgotten: the rule is not
/// "remember to cut the tests", it is "say which view you want".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// The code the tree ships: the file without its tests.
    Production,
    /// The file as it is, tests included.
    Everything,
}

/// A file's **bytes**.
///
/// §205. The xtask's one way to read a file that is not text — the fuzzer's
/// seeds, and the clean-room gate's look at every byte of every file.
///
/// Like `read_text`, it **refuses** a Rust file: a Rust file is read with a
/// subject, and a gate that reads one without saying which view it wants has
/// forgotten something.
pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, std::io::Error> {
    let path = path.as_ref();
    if path.extension().and_then(|e| e.to_str()) == Some("rs") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "a Rust file is read with a subject",
        ));
    }
    std::fs::read(path)
}

/// A file that is not Rust: a document, a manifest, a workflow.
///
/// §204. The tree's one way to read everything that is not a Rust file, and it
/// **refuses** a Rust file: a Rust file is read with a subject, and a gate that
/// reads one without saying which view it wants has forgotten something.
pub fn read_text(path: impl AsRef<Path>) -> Result<String, std::io::Error> {
    let path = path.as_ref();
    if path.extension().and_then(|e| e.to_str()) == Some("rs") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "a Rust file is read with a subject",
        ));
    }
    std::fs::read_to_string(path)
}

/// A Rust file, read for a subject.
///
/// §204. The tree's one way to open a Rust file. `Subject::Production` cuts
/// the file's tests — the cut §150 through §154 gave the tree, applied here
/// once — and `Subject::Everything` keeps them, for the gates whose question
/// is about the file as it is.
pub fn read_rust(path: impl AsRef<Path>, subject: Subject) -> Result<String, std::io::Error> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path)?;
    Ok(match subject {
        Subject::Production => production_of(path, &text),
        Subject::Everything => text,
    })
}

/// The production code of a file: everything except its test items and its
/// test-only methods.
///
/// The tests are not the production code, and a gate whose subject is a
/// *production* property has to say so: a test may name the thing to prove
/// that the production code does not do it. §150.
///
/// A **test item** is a `mod tests { … }` block, or a top-level item marked
/// `#[cfg(test)]`. Both are removed **whole**, line by line, so that
/// production code *after* a test item is still production code: a cut that
/// truncated the file at the first test item would hide a limit used further
/// down, and would hide a panic boundary caught further down. §153.
///
/// An **indented** `#[cfg(test)]` marks a **method** inside an impl block,
/// and the method is removed whole as well — the methods around it are
/// production code, and they stay visible, because the method is removed
/// line by line and not by truncating the file at it. §157.
///
/// A file under a **tests directory** is a test file from its first line to
/// its last: Rust's convention puts a crate's integration tests there, and
/// each file in it is a crate of its own. Everything in such a file is a
/// test, so the cut removes the whole of it — and the tree's fourteen
/// `#[test]` functions in `apps/craftkit-cli/tests/` stop being read as
/// production code. §167.
///
/// The line count is the file's, whatever is removed: a gate that names a
/// line has to name the file's line.
pub fn production_of(path: &Path, text: &str) -> String {
    let code = code_lines_kept(text);
    let lines: Vec<&str> = text.lines().collect();
    // The byte offset of each line's first byte.
    let mut starts = Vec::with_capacity(lines.len());
    let mut at = 0usize;
    for line in &lines {
        starts.push(at);
        at += line.len() + 1;
    }

    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index];
        if is_a_test_file(path) {
            // The whole file is a test: every line goes, and the lines stay.
            out.push('\n');
            index += 1;
            continue;
        }
        if line.starts_with("mod tests {") || line.starts_with("#[cfg(test)]") {
            // The item ends where its **braces** say, and the braces are
            // found in the code view: a `}` inside a string is not a brace.
            // §160.
            let last = top_level_item_last_line(&code, &starts, &lines, index);
            for _ in index..=last {
                out.push('\n');
            }
            index = last + 1;
            continue;
        }
        if is_indented_test_attribute(line)
            && let Some(last) = test_method_last_line(&code, &starts, &lines, index)
        {
            for _ in index..=last {
                out.push('\n');
            }
            index = last + 1;
            continue;
        }
        out.push_str(line);
        out.push('\n');
        index += 1;
    }
    out
}

/// Whether a file is a test file: a file under a `tests` directory.
///
/// Rust's convention: a crate's `tests` directory holds its integration
/// tests, and each file in it is a crate of its own. Nothing in such a file
/// is production code, and the cut that removes a file's test items has to
/// know that. §167.
fn is_a_test_file(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))
}

/// The last line of the top-level item a test marker starts.
///
/// The item ends where its **braces** say: the `}` that matches its `{`, or
/// its `;` when it has no braces. The braces are found in the **code view**,
/// because it has no strings in it — a `}` inside a string is not a brace.
///
/// The tree's own `xtask/src/web.rs` has one: a raw string with a `}` at the
/// start of a line. The cut that ended at the first `}` in the first column
/// ended there, and the **eighteen test functions** after it were read as
/// production code. §160.
///
/// The search is bounded by the file, so a malformed file cannot make the
/// cut swallow the rest of the file.
fn top_level_item_last_line(code: &str, starts: &[usize], lines: &[&str], index: usize) -> usize {
    let from = starts[index];
    let Some(rest) = code.get(from..) else { return index };
    let brace = rest.find('{');
    let semi = rest.find(';');
    // The first of the two is the item's own: an item with braces ends at
    // the `}` that matches its `{`, and an item without any ends at its `;`.
    let open = match (brace, semi) {
        (Some(b), Some(s)) if s < b => return line_of(code, from + s, lines),
        (Some(b), _) => from + b,
        (None, Some(s)) => return line_of(code, from + s, lines),
        (None, None) => return index,
    };
    let Some(close) = matching(code, open, b'{', b'}') else {
        return lines.len().saturating_sub(1);
    };
    line_of(code, close, lines)
}

/// The line a byte of the file is on.
///
/// The file's lines are the file's lines, and the code view keeps every one
/// of them, so a position in one is a position in the other. §155.
pub fn line_of(code: &str, byte: usize, lines: &[&str]) -> usize {
    code[..byte].matches('\n').count().min(lines.len().saturating_sub(1))
}

/// Whether a line is an **indented** `#[cfg(test)]` attribute.
///
/// An indented attribute marks a method inside an impl block; an unindented
/// one marks a test item, and the cut already has those.
fn is_indented_test_attribute(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed == "#[cfg(test)]" && trimmed.len() != line.len()
}

/// The last line of the method an indented test attribute marks.
///
/// The method ends at the brace that closes it. The **code view** is used to
/// find that brace, because it has no strings in it: a brace inside a string
/// is not a brace, and the text's own strings would have to be stepped over
/// to find it. The two views are the same length, so a position in one is a
/// position in the other. §155.
///
/// The search stops at the end of the block the attribute is in, so a
/// malformed file cannot make the cut swallow the rest of the file.
fn test_method_last_line(code: &str, starts: &[usize], lines: &[&str], index: usize) -> Option<usize> {
    let indent = lines[index].len() - lines[index].trim_start().len();
    // The end of the enclosing block: the first following line that is not
    // blank and is indented **less** than the attribute. The method's own
    // signature and its closing brace are indented like the attribute, so
    // they are part of it.
    let mut end = lines.len();
    for (at, line) in lines.iter().enumerate().skip(index + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let here = line.len() - line.trim_start().len();
        if here < indent {
            end = at;
            break;
        }
    }
    let from = starts[index] + lines[index].len();
    let to = if end < lines.len() { starts[end] } else { code.len() };
    let open = code.get(from..to)?.find('{')? + from;
    let close = matching(code, open, b'{', b'}')?;
    // The line the closing brace is on.
    Some(code[..close].matches('\n').count())
}
