//! `cargo xtask anchors` — the mutation script's anchors, checked before it runs.
//!
//! §184. The tree has an instrument: `scripts/mutation-test-gates.sh` breaks
//! each rule on purpose and checks that the gate notices. The instrument's
//! own anchors are text in the tree, and a moved anchor was found when the
//! case that used it was reached — up to twenty-five minutes into a run — and
//! the report said *what* the anchor was, never *where* it had been.
//!
//! # What is checked
//!
//! - **Every anchor is in the tree.** The file it names is there, and it holds
//!   the text the script looks for.
//! - **Every anchor names one place.** `swap` replaces the *first* match, so
//!   an anchor that matches five places points at none of them.
//! - **Every file an anchor names can be put back.** A mutation to a file the
//!   script does not snapshot is a mutation the script cannot undo.
//!
//! # Why it is a command of its own
//!
//! It is not part of the `lists` gate, and that is deliberate. `lists` is the
//! gate the mutation script runs for eleven of its cases, and a gate that
//! fails whenever the tree is mutated would make every one of those cases
//! pass for the wrong reason: a mutation removes the anchor it names — that
//! is what a mutation *is* — so an anchor gate inside `lists` would report
//! "caught" for every case, whatever the case was for.
//!
//! So the anchors gate is a command of its own, it is a phase of `ci`, and
//! the mutation script calls it **before** it starts. A moved anchor is then
//! found in a second, with the place it used to be, instead of twenty-five
//! minutes in.
//!
//! # The reader
//!
//! The script quotes its arguments three ways: `'single'`, `"double"` and
//! `$'ansi'`. A reader that understands one of them finds a third of the
//! anchors and says nothing about the rest, so this understands all three —
//! and an escape it does not know is an **error**, not a guess. A reader that
//! guesses is a reader that lies.
//!
//! The reader and the shell agree: the script's own `swap` refuses an anchor
//! that is not there, so a run that reaches a case proves the shell found
//! what this reader says is there.

use std::path::Path;

use crate::Outcome;

/// The script that holds the anchors.
pub const SCRIPT: &str = "scripts/mutation-test-gates.sh";

/// One `swap` block in the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Swap {
    /// The file the swap writes to.
    pub file: String,
    /// The text it looks for.
    pub anchor: String,
    /// The line of the script the block is written on.
    pub line: usize,
}

/// Every `swap` block in the script.
///
/// The reader walks the script the way the shell reads it: a `#` at the start
/// of a word is a comment and runs to the end of the line, a quoted word is
/// one word however many quoted pieces it is made of, and a word may span
/// lines.
pub fn swaps(script: &str) -> Result<Vec<Swap>, String> {
    let mut out = Vec::new();
    let bytes = script.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        // A comment runs to the end of its line, and the quotes in it are not
        // quotes: the script says "the plan's rate" in a comment, and an
        // apostrophe there closes a string that was never opened.
        if bytes[at] == b'#' && (at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'\'') {
            at = match script[at..].find('\n') {
                Some(n) => at + n + 1,
                None => bytes.len(),
            };
            continue;
        }
        // A command is at the start of a line.
        if (at == 0 || bytes[at - 1] == b'\n') && script[at..].starts_with("swap ") {
            let (swap, next) = read_swap(script, at)?;
            out.push(swap);
            at = next;
            continue;
        }
        // A quoted word is skipped whole, so that nothing inside it is read
        // as a command.
        if let Some((_, next)) = read_word(script, at)? {
            at = next;
            continue;
        }
        at += 1;
    }
    Ok(out)
}

/// Read one `swap` block: the file it names and the text it looks for.
fn read_swap(script: &str, at: usize) -> Result<(Swap, usize), String> {
    let bytes = script.as_bytes();
    let mut pos = at + "swap ".len();
    let file_start = pos;
    while pos < bytes.len() && !bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    let file = script[file_start..pos].to_string();
    if file.is_empty() {
        return Err(format!("{}:{}: a `swap` with no file", SCRIPT, line_of(script, at)));
    }

    let mut words: Vec<String> = Vec::new();
    loop {
        // Between the words there is whitespace, and sometimes a line
        // continuation: `swap FILE \` then the words on the next line.
        while pos < bytes.len() {
            if bytes[pos].is_ascii_whitespace() {
                pos += 1;
                continue;
            }
            if bytes[pos] == b'\\' && pos + 1 < bytes.len() && bytes[pos + 1] == b'\n' {
                pos += 2;
                continue;
            }
            break;
        }
        match read_word(script, pos)? {
            Some((word, next)) => {
                words.push(word);
                pos = next;
            }
            None => break,
        }
    }

    let Some(anchor) = words.first().cloned() else {
        return Err(format!("{}:{}: a `swap` on {} with no anchor", SCRIPT, line_of(script, at), file));
    };
    let swap = Swap { file, anchor, line: line_of(script, at) };
    Ok((swap, pos))
}

/// Read one word: a run of quoted pieces with nothing between them.
///
/// `'it'"'"'s'` is one word, and it is `it's`. The shell closes the first
/// quote, opens a double quote round the apostrophe, and opens the single
/// quote again; a reader that only knew about single quotes would see three
/// words and a string that ends in the wrong place.
fn read_word(script: &str, at: usize) -> Result<Option<(String, usize)>, String> {
    let mut value = String::new();
    let mut pos = at;
    let mut any = false;
    while let Some((piece, next)) = read_piece(script, pos)? {
        value.push_str(&piece);
        pos = next;
        any = true;
    }
    if any { Ok(Some((value, pos))) } else { Ok(None) }
}

/// Read one quoted piece.
fn read_piece(script: &str, at: usize) -> Result<Option<(String, usize)>, String> {
    let bytes = script.as_bytes();
    if at >= bytes.len() {
        return Ok(None);
    }
    if script[at..].starts_with("$'") {
        let mut value = String::new();
        let mut pos = at + 2;
        loop {
            if pos >= bytes.len() {
                return Err(format!("{}: an ANSI-C quote that is never closed", SCRIPT));
            }
            match bytes[pos] {
                b'\\' => {
                    let (ch, next) = ansi_escape(script, pos)?;
                    value.push(ch);
                    pos = next;
                }
                b'\'' => return Ok(Some((value, pos + 1))),
                _ => {
                    let ch = script[pos..].chars().next().unwrap_or('\u{fffd}');
                    value.push(ch);
                    pos += ch.len_utf8();
                }
            }
        }
    }
    if bytes[at] == b'\'' {
        // Nothing is an escape inside single quotes: a backslash is a
        // backslash, and the string ends at the next apostrophe.
        let end = match script[at + 1..].find('\'') {
            Some(n) => at + 1 + n,
            None => return Err(format!("{}: a single quote that is never closed", SCRIPT)),
        };
        return Ok(Some((script[at + 1..end].to_string(), end + 1)));
    }
    if bytes[at] == b'"' {
        let mut value = String::new();
        let mut pos = at + 1;
        loop {
            if pos >= bytes.len() {
                return Err(format!("{}: a double quote that is never closed", SCRIPT));
            }
            match bytes[pos] {
                b'\\' if pos + 1 < bytes.len() => {
                    match bytes[pos + 1] {
                        // These four are the escapes a double quote has. A
                        // backslash before anything else is a backslash.
                        b'$' | b'`' | b'"' | b'\\' => {
                            value.push(bytes[pos + 1] as char);
                            pos += 2;
                        }
                        b'\n' => pos += 2,
                        _ => {
                            value.push('\\');
                            pos += 1;
                        }
                    }
                }
                b'"' => return Ok(Some((value, pos + 1))),
                _ => {
                    let ch = script[pos..].chars().next().unwrap_or('\u{fffd}');
                    value.push(ch);
                    pos += ch.len_utf8();
                }
            }
        }
    }
    Ok(None)
}

/// The character an ANSI-C escape stands for.
///
/// An escape this reader does not know is an error. The script uses three
/// today — `\'`, `\\` and `\0` — and a fourth one it does not know would
/// make the reader say the wrong thing, which is worse than saying nothing.
fn ansi_escape(script: &str, at: usize) -> Result<(char, usize), String> {
    let bytes = script.as_bytes();
    let next = match bytes.get(at + 1) {
        Some(b) => *b,
        None => return Err(format!("{}: a backslash at the end of the script", SCRIPT)),
    };
    let simple = match next {
        b'n' => Some('\n'),
        b't' => Some('\t'),
        b'r' => Some('\r'),
        b'a' => Some('\u{7}'),
        b'b' => Some('\u{8}'),
        b'e' => Some('\u{1b}'),
        b'f' => Some('\u{c}'),
        b'v' => Some('\u{b}'),
        b'\\' => Some('\\'),
        b'\'' => Some('\''),
        b'"' => Some('"'),
        b'0' => Some('\0'),
        _ => None,
    };
    if let Some(ch) = simple {
        return Ok((ch, at + 2));
    }
    if next == b'x' {
        let hex = script.get(at + 2..at + 4).unwrap_or("");
        if let Ok(value) = u32::from_str_radix(hex, 16)
            && let Some(ch) = char::from_u32(value)
        {
            return Ok((ch, at + 4));
        }
    }
    Err(format!(
        "{}: the escape `\\{}` is not one this reader knows; the anchor it is in cannot be read",
        SCRIPT, next as char
    ))
}
/// The files the script snapshots.
///
/// A mutation to a file that is not in this list cannot be put back, and the
/// script's own `guarded` refuses such a mutation when it reaches it.
pub fn files_list(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("readonly FILES=(") {
            inside = true;
            continue;
        }
        if inside {
            if trimmed == ")" {
                break;
            }
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

/// Where an anchor is, in the file it names.
///
/// The line it starts on, and the function it is in when the file has one.
fn where_it_is(file: &str, text: &str, offset: usize) -> String {
    let line = line_of(text, offset);
    let code = crate::lists::code_lines_kept(text);
    let functions = crate::lists::rust_functions(file, text, &code);
    match function_at(&functions, line) {
        Some(name) => format!("{file}:{line} (in `{name}`)"),
        None => format!("{file}:{line}"),
    }
}

/// The function a line is in.
///
/// The last function written before it whose body is still open: a function
/// inside a function is written after the one that holds it, so the inner one
/// wins — and a line after a function has ended is in no function at all,
/// which is not the same answer.
fn function_at(functions: &[crate::lists::Function], line: usize) -> Option<&str> {
    functions
        .iter()
        .filter(|function| function.fn_line <= line && line <= function.end_line)
        .max_by_key(|function| function.fn_line)
        .map(|function| function.name.as_str())
}

/// The line an offset is on.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].lines().count() + 1
}

/// Every place an anchor is, in the text it is looked for in.
fn occurrences(text: &str, needle: &str) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() {
        return out;
    }
    let mut at = 0usize;
    while let Some(found) = text[at..].find(needle) {
        let start = at + found;
        out.push(start);
        at = start + needle.len();
    }
    out
}

/// The command the script calls to check its anchors.
///
/// The gate reads the script, so the call is in the script: the gate can
/// check that the script makes it, and does so before the first mutation.
const CALL: &str = "cargo xtask anchors";

/// The mutation script's anchors are in the tree, and each names one place.
///
/// §184. Four rules, and each is a way the instrument could lie about what it
/// was testing.
///
/// The anchors are grouped: twenty-seven of them are used by more than one
/// case, and a report that said the same thing twice about the same text
/// would be a report nobody reads.
pub fn the_anchors_are_in_the_tree(root: &Path) -> Result<Vec<String>, String> {
    let script =
        std::fs::read_to_string(root.join(SCRIPT)).map_err(|e| format!("could not read {SCRIPT}: {e}"))?;
    let swaps = swaps(&script)?;
    if swaps.is_empty() {
        return Err(format!("{SCRIPT} has no swap blocks; the reader is wrong, not the script"));
    }
    let files = files_list(&script);
    let mut problems = Vec::new();

    // The files the anchors name, and the lines of the script that name them.
    let mut by_file: std::collections::BTreeMap<String, Vec<usize>> = Default::default();
    let mut by_anchor: std::collections::BTreeMap<(String, String), Vec<usize>> = Default::default();
    for swap in &swaps {
        by_file.entry(swap.file.clone()).or_default().push(swap.line);
        by_anchor.entry((swap.file.clone(), swap.anchor.clone())).or_default().push(swap.line);
    }

    // Every file an anchor names is in the tree, and can be put back.
    let mut texts: std::collections::BTreeMap<String, Option<String>> = Default::default();
    for (file, lines) in &by_file {
        let text = std::fs::read_to_string(root.join(file)).ok();
        if text.is_none() {
            problems.push(format!(
                "{}: the anchor names `{}` (line{} {}), and that file is not in the tree",
                SCRIPT,
                file,
                if lines.len() == 1 { "" } else { "s" },
                lines.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", ")
            ));
        }
        if !files.iter().any(|listed| listed == file) {
            problems.push(format!(
                "{}: the anchor names `{}` (line{} {}), and the script's FILES list does not \
                 have it, so a mutation to it could not be put back",
                SCRIPT,
                file,
                if lines.len() == 1 { "" } else { "s" },
                lines.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", ")
            ));
        }
        texts.insert(file.clone(), text);
    }

    // Every anchor is in the file it names, and names one place.
    for ((file, anchor), lines) in &by_anchor {
        let Some(text) = texts.get(file).and_then(|text| text.as_ref()) else { continue };
        let where_ = format!(
            "{}: the anchor for `{}` (line{} {})",
            SCRIPT,
            file,
            if lines.len() == 1 { "" } else { "s" },
            lines.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", ")
        );
        let hits = occurrences(text, anchor);
        match hits.len() {
            0 => problems.push(format!(
                "{where_} is not in the file any more:\n     {}\n     The swap would change \
                 nothing, and the run would stop with \"MUTATION DID NOT APPLY\".",
                anchor.replace('\n', "\n     ")
            )),
            1 => {}
            n => {
                let places: Vec<String> =
                    hits.iter().map(|offset| where_it_is(file, text, *offset)).collect();
                problems.push(format!(
                    "{where_} is in the file {n} times, and `swap` replaces the first:\n{}\n     \
                     An anchor that names {n} places names none of them.",
                    places.iter().map(|place| format!("     {place}")).collect::<Vec<String>>().join("\n")
                ));
            }
        }
    }

    // And the script checks the anchors before it starts. A gate that runs
    // twenty-five minutes after the thing it is about is a gate that is
    // twenty-five minutes late.
    if let Some(first) = swaps.first() {
        if let Some(at) = script.find(CALL) {
            let line = line_of(&script, at);
            if line > first.line {
                problems.push(format!(
                    "{}: the script calls `{CALL}` on line {line}, and its first swap block is \
                     on line {}: a moved anchor is found when the case that uses it is reached, \
                     which is up to twenty-five minutes into a run",
                    SCRIPT, first.line
                ));
            }
        } else {
            problems.push(format!(
                "{}: the script never calls `{CALL}`: a moved anchor is found when the case that \
                 uses it is reached, which is up to twenty-five minutes into a run",
                SCRIPT
            ));
        }
    }

    problems.sort();
    Ok(problems)
}

/// The gate.
pub fn run(root: &Path, _args: &[String]) -> Result<Outcome, String> {
    crate::step("anchors");
    let problems = the_anchors_are_in_the_tree(root)?;
    if !problems.is_empty() {
        let mut message = format!("{} problem(s) in {SCRIPT}:\n", problems.len());
        for problem in &problems {
            message.push_str(&format!("   • {problem}\n"));
        }
        return Err(message);
    }
    println!("   ok: every anchor in {SCRIPT} is in the tree, and each one names a single place");
    Ok(Outcome::Ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------- the reader ----

    /// The three ways the script quotes a word, and the one that is a word
    /// made of three pieces.
    #[test]
    fn the_reader_understands_the_three_quoting_conventions() {
        let script = "swap crates/a.rs \\\n    'one' \\\n    'two'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].file, "crates/a.rs");
        assert_eq!(blocks[0].anchor, "one");
        assert_eq!(blocks[0].line, 1);
    }

    /// A double-quoted word, and the escape it holds.
    #[test]
    fn a_double_quoted_word_is_read_and_its_escapes_are_the_shells() {
        let script = "swap f.rs \"a\\\"b\" 'x'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks[0].anchor, "a\"b");
    }

    /// A backslash that is not one of the four a double quote has is a
    /// backslash, and stays.
    #[test]
    fn a_backslash_a_double_quote_does_not_know_stays_a_backslash() {
        let script = "swap f.rs \"a\\nb\" 'x'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks[0].anchor, "a\\nb");
    }

    /// An ANSI-C word, and the escapes it holds.
    #[test]
    fn an_ansi_c_word_is_read_and_its_escapes_are_the_shells() {
        let script = "swap f.rs $'a\\'b\\\\c' 'x'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks[0].anchor, "a'b\\c");
    }

    /// An escape this reader does not know is an error, not a guess.
    #[test]
    fn an_unknown_escape_is_an_error_and_not_a_guess() {
        let script = "swap f.rs $'a\\qb' 'x'\n";
        let result = swaps(script);
        assert!(result.is_err(), "an unknown escape must not be read as a letter");
    }

    /// A word made of three pieces is one word: `'it'"'"'s'` is `it's`.
    #[test]
    fn an_apostrophe_inside_a_word_is_one_word() {
        let script = "swap f.rs 'it'\"'\"'s' 'x'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks[0].anchor, "it's");
    }

    /// A comment's quotes are not quotes.
    #[test]
    fn a_comment_is_not_a_quote() {
        let script = "# it says \"the plan's rate\" here\nswap f.rs 'anchor' 'x'\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].anchor, "anchor");
    }

    /// A word may span lines, and the block may put the file and the word on
    /// the same line.
    #[test]
    fn a_block_may_be_written_on_one_line_or_on_three() {
        let one = swaps("swap f.rs 'anchor' 'x'\n").expect("the script reads");
        let three = swaps("swap f.rs \\\n    'anchor' \\\n    'x'\n").expect("the script reads");
        assert_eq!(one[0].anchor, three[0].anchor);
        assert_eq!(one[0].file, three[0].file);
    }

    /// A word that is empty is a word: one case replaces a line with nothing.
    #[test]
    fn an_empty_word_is_still_a_word() {
        let script = "swap f.rs 'the line to remove' ''\n";
        let blocks = swaps(script).expect("the script reads");
        assert_eq!(blocks[0].anchor, "the line to remove");
    }

    // ------------------------------------------------------- the list ----

    #[test]
    fn the_files_list_is_the_one_the_script_snapshots() {
        let script = "readonly FILES=(\n    Cargo.toml\n    crates/a.rs\n)\n";
        assert_eq!(files_list(script), vec!["Cargo.toml".to_string(), "crates/a.rs".to_string()]);
    }

    // ------------------------------------------------------- the gate ----

    /// A tree with the script and one file in it.
    ///
    /// The name is unique to the call: the tests run at the same time, and
    /// two of them sharing one directory would delete each other's tree
    /// half way through. §184.
    fn a_tree(name: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let count = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("craftkit-anchors-{name}-{}-{count}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).expect("the fabricated tree");
        std::fs::create_dir_all(dir.join("crates")).expect("the fabricated tree");
        dir
    }

    /// A script and a file that agree.
    fn a_good_tree() -> std::path::PathBuf {
        let dir = a_tree("good");
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/a.rs\n)\ncargo xtask anchors\nswap crates/a.rs 'the anchor' 'the break'\n",
        )
        .expect("the fabricated script");
        std::fs::write(dir.join("crates/a.rs"), "the anchor\n").expect("the fabricated file");
        dir
    }

    #[test]
    fn a_tree_where_every_anchor_is_in_the_file_has_no_problems() {
        let dir = a_good_tree();
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert!(problems.is_empty(), "{problems:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The failure this catches: an anchor that moved. The swap would change
    /// nothing, and the run would stop twenty-five minutes in.
    #[test]
    fn an_anchor_that_is_not_in_the_file_is_reported_with_the_place_to_look() {
        let dir = a_good_tree();
        std::fs::write(dir.join("crates/a.rs"), "the line was renamed\n").expect("the file");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("is not in the file any more"), "{}", problems[0]);
        assert!(problems[0].contains("line 5"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The failure this catches: an anchor that matches more than one place.
    /// `swap` replaces the first, which may be the wrong one.
    #[test]
    fn an_anchor_that_is_in_the_file_twice_is_reported_with_both_places() {
        let dir = a_good_tree();
        std::fs::write(dir.join("crates/a.rs"), "the anchor\nthe anchor\n").expect("the file");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("is in the file 2 times"), "{}", problems[0]);
        assert!(problems[0].contains("crates/a.rs:1"), "{}", problems[0]);
        assert!(problems[0].contains("crates/a.rs:2"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A mutation to a file the script does not snapshot cannot be put back.
    #[test]
    fn a_file_the_script_cannot_put_back_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/other.rs\n)\ncargo xtask anchors\nswap crates/a.rs 'the anchor' 'the break'\n",
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("FILES list does not have it"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file the anchor names that is not in the tree at all.
    #[test]
    fn a_file_that_is_not_in_the_tree_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/a.rs\n)\ncargo xtask anchors\nswap crates/gone.rs 'the anchor' 'the break'\n",
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("is not in the tree")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("FILES list does not have it")), "{problems:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The failure this catches: a script that checks its anchors only after
    /// it has started changing the tree. A moved anchor would then be found
    /// when the case that uses it is reached — up to twenty-five minutes in.
    #[test]
    fn a_script_that_never_checks_its_anchors_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/a.rs\n)\nswap crates/a.rs 'the anchor' 'the break'\n",
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("never calls"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// And the other half: a script that checks its anchors *after* the first
    /// mutation is the same defect, twenty-five minutes later.
    #[test]
    fn a_script_that_checks_its_anchors_too_late_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/a.rs\n)\nswap crates/a.rs 'the anchor' 'the break'\ncargo xtask anchors\n",
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("twenty-five minutes"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Twenty-seven anchors are used by more than one case. They are reported
    /// once, with every line that uses them.
    #[test]
    fn an_anchor_used_by_two_cases_is_reported_once_with_both_lines() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            "readonly FILES=(\n    crates/a.rs\n)\ncargo xtask anchors\nswap crates/a.rs 'the anchor' 'the break'\nswap crates/a.rs 'the anchor' 'other'\n",
        )
        .expect("the fabricated script");
        std::fs::write(dir.join("crates/a.rs"), "the anchor\nthe anchor\n").expect("the file");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("lines 5, 6"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A script with no swaps at all means the reader is wrong, and a gate
    /// that cannot see the thing it checks must say so rather than pass.
    #[test]
    fn a_script_with_no_swaps_is_an_error_and_not_a_pass() {
        let dir = a_good_tree();
        std::fs::write(dir.join("scripts/mutation-test-gates.sh"), "readonly FILES=(\n)\n")
            .expect("the file");
        let result = the_anchors_are_in_the_tree(&dir);
        assert!(result.is_err(), "a reader that found nothing must not report a pass");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
