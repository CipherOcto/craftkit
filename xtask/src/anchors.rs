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
//! - **The list is a list.** Every path on it is a file in the tree, no path
//!   is on it twice, and it is not empty. §187.
//! - **The script calls its check before its first swap**, and that check runs
//!   this gate. §187.
//! - **Every label is a literal the shell cannot change.** A label is the
//!   sentence that says what a case is, and it is the only place the log says
//!   what was tested; the shell sits between the author and the reader. §190.
//! - **The header names every way to run the script**, and no way that does
//!   not exist; the ways are read from the script's own code. §191.
//! - **The documents name the list of files a mutation may write to.** It is
//!   a file, and the script reads it on every run. §191.
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
//!
//! # The list
//!
//! The files a mutation may write to live in [`FILES_FILE`], and the script
//! reads that file rather than carrying the list. §187: a list inside a
//! script cannot be broken, because bash reads a script as it goes and a
//! case cannot edit the script it is running — so the rule that says a
//! mutation to a file the script cannot put back is refused had never been
//! shown to fail against the real list. A file that is read is a file that
//! can be mutated.
//!
//! # Checking a script that is not the tree's own
//!
//! `--script PATH` checks another script against the same tree. Two of the
//! rules are about the script's own text, and the only way to break one is
//! to break a **copy** of it: the script cannot be changed while it runs.
//! §187.

use std::path::{Path, PathBuf};

use crate::Outcome;

/// The script that holds the anchors.
pub const SCRIPT: &str = "scripts/mutation-test-gates.sh";

/// The file that holds the list of files a mutation may write to.
///
/// §187. The list used to be an array inside the script, and the gate read
/// it by parsing the script's text. That made the list a second copy of a
/// fact the script owned, and it made the list impossible to break.
pub const FILES_FILE: &str = "scripts/mutation-files.txt";

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

/// How a piece of a word was quoted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quoting {
    /// `'text'` — the shell changes nothing inside it.
    Single,
    /// `"text"` — the shell expands the variables and the backticks in it.
    Double,
    /// `$'text'` — the shell changes the escapes and nothing else.
    Ansi,
    /// A bare word: the shell expands the variables in it.
    Bare,
}

/// One quoted piece of a word.
///
/// The **value** is what the shell produces; the **body** is what was
/// written between the quotes. A reader that had only the value could not
/// tell an escaped `\$` — a literal dollar — from an unescaped one, and
/// the difference is exactly whether the shell expands it. §190.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// How the piece was quoted.
    pub quoting: Quoting,
    /// The text the shell produces, with the escapes resolved.
    pub value: String,
    /// The text between the quotes, exactly as it was written.
    pub body: String,
}

/// Read one word: a run of quoted pieces with nothing between them.
///
/// `'it'"'"'s'` is one word, and it is `it's`. The shell closes the first
/// quote, opens a double quote round the apostrophe, and opens the single
/// quote again; a reader that only knew about single quotes would see three
/// words and a string that ends in the wrong place.
fn read_word(script: &str, at: usize) -> Result<Option<(String, usize)>, String> {
    let Some((pieces, next)) = read_pieces(script, at)? else { return Ok(None) };
    let value = pieces.iter().map(|piece| piece.value.as_str()).collect();
    Ok(Some((value, next)))
}

/// Read one word as the pieces it is made of.
fn read_pieces(script: &str, at: usize) -> Result<Option<(Vec<Piece>, usize)>, String> {
    let mut pieces = Vec::new();
    let mut pos = at;
    while let Some((piece, next)) = read_piece(script, pos)? {
        pieces.push(piece);
        pos = next;
    }
    if pieces.is_empty() { Ok(None) } else { Ok(Some((pieces, pos))) }
}

/// Read one quoted piece.
fn read_piece(script: &str, at: usize) -> Result<Option<(Piece, usize)>, String> {
    let bytes = script.as_bytes();
    if at >= bytes.len() {
        return Ok(None);
    }
    if script[at..].starts_with("$'") {
        let mut value = String::new();
        let mut body = String::new();
        let mut pos = at + 2;
        loop {
            if pos >= bytes.len() {
                return Err(format!("{}: an ANSI-C quote that is never closed", SCRIPT));
            }
            match bytes[pos] {
                b'\\' => {
                    let (ch, next) = ansi_escape(script, pos)?;
                    body.push_str(&script[pos..next]);
                    value.push(ch);
                    pos = next;
                }
                b'\'' => {
                    return Ok(Some((Piece { quoting: Quoting::Ansi, value, body }, pos + 1)));
                }
                _ => {
                    let ch = script[pos..].chars().next().unwrap_or('\u{fffd}');
                    body.push(ch);
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
        let body = script[at + 1..end].to_string();
        return Ok(Some((Piece { quoting: Quoting::Single, value: body.clone(), body }, end + 1)));
    }
    if bytes[at] == b'"' {
        let mut value = String::new();
        let mut body = String::new();
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
                            body.push('\\');
                            body.push(bytes[pos + 1] as char);
                            value.push(bytes[pos + 1] as char);
                            pos += 2;
                        }
                        b'\n' => {
                            body.push('\\');
                            body.push('\n');
                            pos += 2;
                        }
                        _ => {
                            body.push('\\');
                            value.push('\\');
                            pos += 1;
                        }
                    }
                }
                b'"' => {
                    return Ok(Some((Piece { quoting: Quoting::Double, value, body }, pos + 1)));
                }
                _ => {
                    let ch = script[pos..].chars().next().unwrap_or('\u{fffd}');
                    body.push(ch);
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

/// One function in the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    /// The name it is called by.
    pub name: String,
    /// The line it is written on.
    pub line: usize,
    /// Its body, as it was written.
    pub body: String,
}

/// Every function the script defines.
///
/// A function is written the way the script writes its functions: a line
/// that opens it, the lines of its body, and a line that is `}` and nothing
/// else. The shape is the one `the_body_of_the_check` already reads, and it
/// reads **all** of them: a rule about one function is a rule about the
/// script, and the script has thirteen. §190.
pub fn functions(script: &str) -> Result<Vec<Function>, String> {
    let view = code_view(script)?;
    let original: Vec<&str> = script.lines().collect();
    let mut out = Vec::new();
    let mut open: Option<(String, usize)> = None;
    for (index, line) in view.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_suffix("() {") {
            if let Some((name, line)) = open.take() {
                out.push(a_function(&original, name, line, index));
            }
            open = Some((name.to_string(), index + 1));
            continue;
        }
        if trimmed == "}" {
            if let Some((name, line)) = open.take() {
                out.push(a_function(&original, name, line, index));
            }
            continue;
        }
    }
    if let Some((name, line)) = open {
        out.push(a_function(&original, name, line, original.len()));
    }
    Ok(out)
}

/// One function, with its body as the script wrote it.
fn a_function(original: &[&str], name: String, line: usize, end: usize) -> Function {
    // `line` is the line the function is opened on, counting from one; the
    // body is what follows it, up to the closing brace.
    let start = line.min(original.len());
    let stop = end.min(original.len());
    let body = if start < stop { original[start..stop].join("\n") } else { String::new() };
    Function { name, line, body }
}

/// The script with its comments and its quoted strings blanked out, the
/// length unchanged.
///
/// A quoted string can hold a line that looks like a function definition —
/// the tree's own anchors hold Rust snippets — and a reader that counted
/// those would find thirty-seven functions in a script that has thirteen.
/// §190.
fn code_view(script: &str) -> Result<String, String> {
    let bytes = script.as_bytes();
    let mut out = String::with_capacity(script.len());
    let mut at = 0usize;
    while at < bytes.len() {
        // A comment runs to the end of its line, and the quotes in it are
        // not quotes: the script says "the plan's rate" in a comment, and
        // an apostrophe there closes a string that was never opened.
        if bytes[at] == b'#' && (at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'\'') {
            while at < bytes.len() && bytes[at] != b'\n' {
                out.push(' ');
                at += 1;
            }
            continue;
        }
        // A quoted word is blanked out, so that nothing inside it is read
        // as code.
        if let Some((_, next)) = read_piece(script, at)? {
            for byte in &bytes[at..next] {
                out.push(if *byte == b'\n' { '\n' } else { ' ' });
            }
            at = next;
            continue;
        }
        let ch = script[at..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        at += ch.len_utf8();
    }
    Ok(out)
}

/// The functions that take a label.
///
/// **Derived, not written down**: a function that takes a label declares one
/// (`local label="$1"`) and uses it, and every call to it has a label the
/// shell could rewrite. A helper that takes a label therefore cannot be
/// added without the gate noticing — there is no list to forget to update.
/// §190.
fn label_commands(script: &str) -> Result<Vec<String>, String> {
    Ok(functions(script)?
        .into_iter()
        .filter(|function| function.body.contains("label=") || function.body.contains("$label"))
        .map(|function| function.name)
        .collect())
}

/// One label: the sentence that says what a case is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    /// The line the label is written on.
    pub line: usize,
    /// The pieces the label is made of.
    pub pieces: Vec<Piece>,
    /// The label as the author wrote it.
    pub text: String,
}

/// Every label in the script, with the line it is written on.
///
/// A label is the first argument of a call to one of the script's
/// label-taking functions. §190.
pub fn labels(script: &str) -> Result<Vec<Label>, String> {
    let commands = label_commands(script)?;
    if commands.is_empty() {
        return Err(format!("{SCRIPT}: no function takes a label; the reader is wrong, not the script"));
    }
    let mut out = Vec::new();
    let bytes = script.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        // A comment runs to the end of its line, and the quotes in it are
        // not quotes: the script says "the plan's rate" in a comment, and
        // an apostrophe there closes a string that was never opened.
        if bytes[at] == b'#' && (at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'\'') {
            at = match script[at..].find('\n') {
                Some(n) => at + n + 1,
                None => bytes.len(),
            };
            continue;
        }
        // A command is at the start of a line.
        if at == 0 || bytes[at - 1] == b'\n' {
            let line = line_of(script, at);
            let mut pos = at;
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            let rest = &script[pos..];
            let name_len = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let name = &rest[..name_len];
            if commands.iter().any(|command| command == name) {
                let after = skip_space(script, pos + name_len);
                if let Some((pieces, next)) = read_word_pieces(script, after)? {
                    let text = pieces.iter().map(|piece| piece.value.as_str()).collect();
                    out.push(Label { line, pieces, text });
                    at = next;
                    continue;
                }
            }
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

/// Read one word as the pieces it is made of, bare chunks included.
///
/// A bare chunk is a piece of a word that is not quoted at all, and the
/// shell expands the variables in it — so `$label` as a label is a label
/// the shell would change, and the file a `swap` names is a bare chunk.
/// §190, §192.
fn read_word_pieces(script: &str, at: usize) -> Result<Option<(Vec<Piece>, usize)>, String> {
    let bytes = script.as_bytes();
    let mut pieces = Vec::new();
    let mut pos = at;
    loop {
        match read_piece(script, pos)? {
            Some((piece, next)) => {
                pieces.push(piece);
                pos = next;
            }
            None => {
                if pos >= bytes.len() || bytes[pos].is_ascii_whitespace() {
                    break;
                }
                let start = pos;
                while pos < bytes.len()
                    && !bytes[pos].is_ascii_whitespace()
                    && bytes[pos] != b'\''
                    && bytes[pos] != b'"'
                {
                    pos += 1;
                }
                let body = script[start..pos].to_string();
                pieces.push(Piece { quoting: Quoting::Bare, value: body.clone(), body });
            }
        }
    }
    if pieces.is_empty() { Ok(None) } else { Ok(Some((pieces, pos))) }
}

/// The first place at or after `at` that is not whitespace.
fn skip_space(script: &str, at: usize) -> usize {
    let bytes = script.as_bytes();
    let mut pos = at;
    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

/// Does the shell expand anything in this text?
///
/// A backslash before a character means the character is literal, and a
/// backslash before a backslash is one backslash. §190.
fn holds_an_expansion(body: &str) -> bool {
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '$' | '`' => return true,
            _ => {}
        }
    }
    false
}

/// What is wrong with the script's labels.
///
/// A label is the sentence that says what a case is, and it is the only
/// place the log says what was tested. The shell sits between the author
/// and the reader, and inside double quotes it expands a dollar and runs
/// a backtick — so the label a reader sees is not always the label the
/// author wrote. §190.
pub fn label_problems(script_path: &Path, script: &str) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for label in labels(script)? {
        for piece in &label.pieces {
            let expands = match piece.quoting {
                Quoting::Single | Quoting::Ansi => false,
                Quoting::Double | Quoting::Bare => holds_an_expansion(&piece.body),
            };
            if expands {
                let how = match piece.quoting {
                    Quoting::Double => "double quotes",
                    _ => "a bare word",
                };
                problems.push(format!(
                    "{}:{}: the label is written in {how} and the shell would change it: a dollar \
                     is a variable and a backtick is a command, so the label a reader sees is not \
                     the label the author wrote.\n     the label: {}",
                    script_path.display(),
                    label.line,
                    label.text
                ));
            }
        }
    }
    Ok(problems)
}

/// One text argument of a mutation call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextArgument {
    /// The line the call is written on.
    pub line: usize,
    /// The call it belongs to: `swap`, `append` or `broken_copy`.
    pub call: String,
    /// The pieces the text is made of.
    pub pieces: Vec<Piece>,
}

/// The text arguments of the script's mutation calls.
///
/// A **text** is something the script writes into the tree: the text a
/// `swap` looks for, the text an `append` writes, the text a
/// `broken_copy` puts into its copy. A **command** is something the script
/// runs, and the shell may expand that on purpose — the gate command
/// names the snapshot directory, and it has to. §192.
pub fn text_arguments(script: &str) -> Result<Vec<TextArgument>, String> {
    let mut out = Vec::new();
    let bytes = script.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        // A comment runs to the end of its line, and the quotes in it are
        // not quotes.
        if bytes[at] == b'#' && (at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'\'') {
            at = match script[at..].find('\n') {
                Some(n) => at + n + 1,
                None => bytes.len(),
            };
            continue;
        }
        // A command is at the start of a line.
        if at == 0 || bytes[at - 1] == b'\n' {
            for (call, texts) in [("swap", 1usize), ("append", 1), ("broken_copy", 2)] {
                if !script[at..].starts_with(&format!("{call} ")) {
                    continue;
                }
                let line = line_of(script, at);
                let mut pos = at + call.len() + 1;
                let mut word = 0usize;
                while word <= texts {
                    // Between the words there is whitespace, and sometimes a
                    // line continuation.
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
                    let Some((pieces, next)) = read_word_pieces(script, pos)? else { break };
                    // The first word is the file the call writes to, or the
                    // path the copy is written to; the rest are texts.
                    if word >= 1 {
                        out.push(TextArgument { line, call: call.to_string(), pieces });
                    }
                    word += 1;
                    pos = next;
                }
                break;
            }
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

/// What is wrong with the script's text arguments.
///
/// A text the shell would change is a text that is not the text the
/// author wrote: the gate reads the anchor and finds it, and the shell
/// runs a command and does not. §192.
pub fn text_problems(script_path: &Path, script: &str) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for argument in text_arguments(script)? {
        for piece in &argument.pieces {
            let expands = match piece.quoting {
                Quoting::Single | Quoting::Ansi => false,
                Quoting::Double | Quoting::Bare => holds_an_expansion(&piece.body),
            };
            if expands {
                let how = match piece.quoting {
                    Quoting::Double => "double quotes",
                    _ => "a bare word",
                };
                problems.push(format!(
                    "{}:{}: the text of the `{}` call is written in {how} and the shell would change \
                     it: a dollar is a variable and a backtick is a command, so the text the gate \
                     reads is not the text the shell writes.",
                    script_path.display(),
                    argument.line,
                    argument.call
                ));
            }
        }
    }
    Ok(problems)
}

/// One `broken_copy` block in the script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokenCopy {
    /// The path the copy is written to.
    pub out: String,
    /// The text it looks for, as the shell makes it.
    ///
    /// The copy looks the text up in the script's own text, so this is the
    /// word the shell hands it and not the text between the quotes. §193.
    pub text: String,
    /// The line the call is written on.
    pub line: usize,
    /// Where the call is in the script: from its name to the end of its last
    /// argument.
    pub span: (usize, usize),
}

/// Every `broken_copy` block in the script.
///
/// A `broken_copy` names an anchor the way a `swap` does — a text it looks
/// for in a file — and §184 checks every `swap` anchor before the run
/// starts. These are not checked at all, and they are found when the case
/// that uses them is reached, which is at the end of the run. §193.
pub fn broken_copies(script: &str) -> Result<Vec<BrokenCopy>, String> {
    let mut out = Vec::new();
    let bytes = script.as_bytes();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == b'#' && (at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'\'') {
            at = match script[at..].find('\n') {
                Some(n) => at + n + 1,
                None => bytes.len(),
            };
            continue;
        }
        if (at == 0 || bytes[at - 1] == b'\n') && script[at..].starts_with("broken_copy ") {
            let (copy, next) = read_broken_copy(script, at)?;
            out.push(copy);
            at = next;
            continue;
        }
        // A word is skipped whole, so that nothing inside it is read as a
        // command.
        if let Some((_, next)) = read_word_pieces(script, at)? {
            at = next;
            continue;
        }
        at += 1;
    }
    Ok(out)
}

/// Read one `broken_copy` block: where the copy is written, the text it
/// looks for, and where the call is in the script.
fn read_broken_copy(script: &str, at: usize) -> Result<(BrokenCopy, usize), String> {
    let bytes = script.as_bytes();
    let mut pos = at + "broken_copy ".len();
    let mut words: Vec<(String, usize, usize)> = Vec::new();
    while words.len() < 3 {
        // Between the words there is whitespace, and sometimes a line
        // continuation.
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
        let start = pos;
        let Some((pieces, next)) = read_word_pieces(script, pos)? else { break };
        let value = pieces.iter().map(|piece| piece.value.as_str()).collect();
        words.push((value, start, next));
        pos = next;
    }
    if words.len() < 3 {
        return Err(format!(
            "{}:{}: a `broken_copy` with fewer than three arguments",
            SCRIPT,
            line_of(script, at)
        ));
    }
    let copy = BrokenCopy {
        out: words[0].0.clone(),
        text: words[1].0.clone(),
        line: line_of(script, at),
        span: (at, words[2].2),
    };
    Ok((copy, pos))
}

/// What is wrong with the script's `broken_copy` blocks.
///
/// A `broken_copy` names an anchor the way a `swap` does, and §184's rule
/// for a `swap` is that the anchor names **one** place. The copy replaces
/// the **first** place its text is in, and the call's own argument is a
/// place its text is in — so the rule for a `broken_copy` is *one place
/// apart from the call's own*.
///
/// Three ways to break it, and each is a copy that tests nothing: a text
/// the script does not hold makes a copy that changes nothing; a text whose
/// first place is the call itself makes a copy that breaks the call and not
/// the script; and a text in two places makes a copy that breaks whichever
/// comes first, which may not be the place the case meant. §193.
pub fn broken_copy_problems(script_path: &Path, script: &str) -> Result<Vec<String>, String> {
    let mut problems = Vec::new();
    for copy in broken_copies(script)? {
        let places = occurrences(script, &copy.text);
        let Some(first) = places.first().copied() else {
            problems.push(format!(
                "{}:{}: the `broken_copy` looks for text that is not in the script, so the copy it \
                 writes is the script unchanged and the case tests nothing",
                script_path.display(),
                copy.line
            ));
            continue;
        };
        let inside = |offset: usize| copy.span.0 <= offset && offset < copy.span.1;
        if inside(first) {
            problems.push(format!(
                "{}:{}: the first place the `broken_copy` looks for is inside the call itself \
                 (line {}), so the copy it writes breaks the call and not the script, and the \
                 case tests nothing",
                script_path.display(),
                copy.line,
                line_of(script, first)
            ));
            continue;
        }
        let outside: Vec<usize> = places.iter().copied().filter(|offset| !inside(*offset)).collect();
        if outside.len() > 1 {
            let lines: Vec<String> =
                outside.iter().map(|offset| line_of(script, *offset).to_string()).collect();
            problems.push(format!(
                "{}:{}: the `broken_copy` looks for text that is in {} places apart from the call \
                 (lines {}), and the copy replaces the first, so the copy may break a place the \
                 case did not mean",
                script_path.display(),
                copy.line,
                outside.len(),
                lines.join(", ")
            ));
        }
    }
    Ok(problems)
}

/// The flags the script accepts, read from the script's own code.
///
/// The script tests its first argument against a literal, and those
/// literals are the flags. There is **no list to keep**: a flag the script
/// does not test is not a flag it accepts, and a flag it does test is
/// accepted whether or not anybody wrote it down. §191.
pub fn script_flags(script: &str) -> Vec<String> {
    let needle = "\"${1:-}\"";
    let mut out: Vec<String> = Vec::new();
    let mut at = 0usize;
    while let Some(found) = script[at..].find(needle) {
        let mut pos = at + found + needle.len();
        let bytes = script.as_bytes();
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let compares = script[pos..].starts_with("==") || script[pos..].starts_with("!=");
        if compares {
            pos += 2;
            while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if script[pos..].starts_with('"') {
                pos += 1;
                if let Some(end) = script[pos..].find('"') {
                    let flag = script[pos..pos + end].to_string();
                    if !flag.is_empty() && !out.contains(&flag) {
                        out.push(flag);
                    }
                }
            }
        }
        at = at + found + needle.len();
    }
    out
}

/// The script's own header: the comment block it opens with.
///
/// The header is where a reader learns how to run the script, and it is the
/// script's own copy of its interface — a copy nobody reads until something
/// has gone wrong. §191.
fn header_of(script: &str) -> String {
    let mut out = String::new();
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if out.is_empty() {
                continue;
            }
            break;
        }
        if !trimmed.starts_with('#') {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// The invocations the header names.
///
/// A header line that names the script is an invocation, and the word after
/// the name is its flag. §191.
pub fn header_flags(script: &str) -> Vec<String> {
    let header = header_of(script);
    let mut out: Vec<String> = Vec::new();
    for line in header.lines() {
        let Some(at) = line.find(SCRIPT) else { continue };
        let rest = &line[at + SCRIPT.len()..];
        let word = rest.split_whitespace().next().unwrap_or("");
        if word.starts_with("--") && !out.contains(&word.to_string()) {
            out.push(word.to_string());
        }
    }
    out
}

/// What is wrong with the script's header.
///
/// The header names the ways to run the script, and the script's own code
/// says what those ways are. A header that names a flag the script does
/// not accept sends a reader to an error; a header that misses a flag the
/// script accepts sends a reader nowhere. §191.
pub fn header_problems(script_path: &Path, script: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let flags = script_flags(script);
    let named = header_flags(script);
    if !header_of(script).contains(SCRIPT) {
        problems.push(format!(
            "{}: the header never names the script, so it says nothing about how to run it",
            script_path.display()
        ));
    }
    for flag in &flags {
        if !named.contains(flag) {
            problems.push(format!(
                "{}: the script accepts {flag} and the header does not name it: a reader who reads \
                 the header does not know the flag exists",
                script_path.display()
            ));
        }
    }
    for flag in &named {
        if !flags.contains(flag) {
            problems.push(format!(
                "{}: the header names {flag} and the script does not accept it",
                script_path.display()
            ));
        }
    }
    problems
}

/// Does this text name the script's array of files?
///
/// The whole word, so that `FILES_FILE` and the format change's own
/// `FILES` constant are not it. §191.
fn names_the_array(text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = b"FILES";
    let mut at = 0usize;
    while let Some(found) = text[at..].find("FILES") {
        let start = at + found;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        at = start + 1;
    }
    false
}

/// Is this byte part of a name?
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// What is wrong with the tree's documents about the mutation script.
///
/// The list of files a mutation may write to is `scripts/mutation-files.txt`,
/// and the script reads it on **every run**: the array it fills is rebuilt
/// each time, so a path added to the array is lost before the first case
/// runs. A document that tells a contributor to add a file to the array is
/// telling them to do something that does not survive the next run. §191.
pub fn prose_problems(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for doc in crate::lists::prose_files(root) {
        let Ok(text) = crate::tree::read_text(&doc) else { continue };
        let relative = doc.strip_prefix(root).unwrap_or(&doc).display().to_string();
        for (line, block) in crate::lists::blocks(&text) {
            if !block.to_lowercase().contains("mutation") {
                continue;
            }
            if names_the_array(&block) {
                problems.push(format!(
                    "{relative}:{line}: it names the script's `FILES` array as the list of files a \
                     mutation may write to, and the array is rebuilt from \
                     `scripts/mutation-files.txt` on every run: a path added to the array is lost \
                     before the first case runs."
                ));
            }
        }
    }
    problems
}

/// The files a mutation may write to, and what is wrong with the list.
///
/// §187. The list is a file, so it is read rather than parsed out of the
/// script, and so it can be broken on purpose. The problems are the list's
/// own: a blank line is not a path, the same path twice is not two files,
/// and a path that is not in the tree is a file the script cannot snapshot
/// and cannot put back.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct List {
    /// The paths, in the order the file lists them.
    pub paths: Vec<String>,
    /// What is wrong with the list.
    pub problems: Vec<String>,
}

/// Read the list of files a mutation may write to.
///
/// The script reads the same file, so the two never disagree about what the
/// list says: the script refuses a blank line and a path that is not in the
/// tree, and this refuses them too.
///
/// The format is one path per line. A line that starts with `#` is a
/// comment, and nothing else is: a path is not trimmed, so a path with a
/// space in it is a path that is not in the tree.
pub fn read_list(root: &Path) -> Result<List, String> {
    let path = root.join(FILES_FILE);
    let text = crate::tree::read_text(&path).map_err(|e| format!("could not read {FILES_FILE}: {e}"))?;
    let mut list = List::default();
    let mut first_seen: std::collections::BTreeMap<&str, usize> = Default::default();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            // A refinement of the rule below, not a second rule: a blank
            // line is a path that is not in the tree, and the message says
            // which of the two it is, because "lists ``" is not something a
            // reader can act on.
            list.problems
                .push(format!("{FILES_FILE}: line {number} is blank, and a blank line is not a path"));
            continue;
        }
        if let Some(earlier) = first_seen.get(line) {
            list.problems
                .push(format!("{FILES_FILE}: line {number} lists `{line}`, and line {earlier} lists it too"));
            continue;
        }
        first_seen.insert(line, number);
        if !root.join(line).is_file() {
            list.problems.push(format!(
                "{FILES_FILE}: line {number} lists `{line}`, and that file is not in the tree"
            ));
        }
        list.paths.push(line.to_string());
    }
    if list.paths.is_empty() {
        list.problems.push(format!("{FILES_FILE} lists no files at all"));
    }
    Ok(list)
}

/// The script's own anchor check, by name.
///
/// The definition is `check_anchors() {` and the call is `check_anchors` on a
/// line of its own.
const CHECK: &str = "check_anchors";

/// The command the script's check runs.
const GATE: &str = "cargo xtask anchors";

/// The line the script calls its anchor check on.
///
/// The gate looks for the **call**, and not for the command the check runs.
/// §187: the string `cargo xtask anchors` is in the function's body, so a
/// gate that looked for that string passed a script that never called the
/// function at all — and passed a script that called it after the first
/// mutation, which is the defect the call is there to prevent.
fn the_call_site(script: &str) -> Option<usize> {
    script.lines().enumerate().find_map(|(index, line)| (line.trim() == CHECK).then_some(index + 1))
}

/// The lines of the script's own anchor check.
///
/// The function is written the way the script writes its functions: a line
/// that opens it, the lines of its body, and a line that is `}` and nothing
/// else. The body is what says which command the script runs, and the gate
/// reads it because the call alone does not say it.
fn the_body_of_the_check(script: &str) -> Option<String> {
    let mut lines = script.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "check_anchors() {" {
            let mut body = String::new();
            for line in lines {
                if line.trim() == "}" {
                    return Some(body);
                }
                body.push_str(line);
                body.push('\n');
            }
            return None;
        }
    }
    None
}

/// Where an anchor is, in the file it names.
///
/// The line it starts on, and the function it is in when the file has one.
fn where_it_is(file: &str, text: &str, offset: usize) -> String {
    let line = line_of(text, offset);
    let code = crate::tree::code_lines_kept(text);
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
///
/// The line is the one the offset is **on**, so the count is of the newlines
/// before it: the partial line before the offset is the line the offset is
/// on, and counting the lines of the prefix would count it twice. §193.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
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

/// The mutation script's anchors are in the tree, and each names one place.
///
/// §184. Six rules, and each is a way the instrument could lie about what it
/// was testing.
///
/// The anchors are grouped: twenty-seven of them are used by more than one
/// case, and a report that said the same thing twice about the same text
/// would be a report nobody reads.
pub fn the_anchors_are_in_the_tree(root: &Path) -> Result<Vec<String>, String> {
    the_anchors_are_in_the_tree_of(root, &root.join(SCRIPT))
}

/// The same check, for a script the caller names.
///
/// §187. Two of the rules are about the script's own text, and the script
/// cannot be edited while it runs — bash reads a script as it goes — so the
/// only way to break one of them is to break a **copy**. The tree it is
/// checked against is still the tree.
pub fn the_anchors_are_in_the_tree_of(root: &Path, script_path: &Path) -> Result<Vec<String>, String> {
    let script = crate::tree::read_text(script_path)
        .map_err(|e| format!("could not read {}: {e}", script_path.display()))?;
    let swaps = swaps(&script)?;
    if swaps.is_empty() {
        return Err(format!(
            "{} has no swap blocks; the reader is wrong, not the script",
            script_path.display()
        ));
    }
    let list = read_list(root)?;
    let mut problems = list.problems.clone();

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
        let path = root.join(file);
        // The file an anchor names may be Rust, and the anchor has to be in
        // the file wherever it is: the file is read as it is. §205.
        let text = if file.ends_with(".rs") {
            crate::tree::read_rust(&path, crate::tree::Subject::Everything).ok()
        } else {
            crate::tree::read_text(&path).ok()
        };
        if text.is_none() {
            problems.push(format!(
                "{}: the anchor names `{}` (line{} {}), and that file is not in the tree",
                script_path.display(),
                file,
                if lines.len() == 1 { "" } else { "s" },
                lines.iter().map(|n| n.to_string()).collect::<Vec<String>>().join(", ")
            ));
        }
        if !list.paths.iter().any(|listed| listed == file) {
            problems.push(format!(
                "{}: the anchor names `{}` (line{} {}), and the list of files a mutation may \
                 write to does not have it, so a mutation to it could not be put back",
                script_path.display(),
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
            script_path.display(),
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
        match the_call_site(&script) {
            Some(line) if line <= first.line => {}
            Some(line) => problems.push(format!(
                "{}: the script calls `{CHECK}` on line {line}, and its first swap block is \
                 on line {}: a moved anchor is found when the case that uses it is reached, \
                 which is up to twenty-five minutes into a run",
                script_path.display(),
                first.line
            )),
            None => problems.push(format!(
                "{}: the script never calls `{CHECK}`: a moved anchor is found when the case \
                 that uses it is reached, which is up to twenty-five minutes into a run",
                script_path.display()
            )),
        }
        // And the call runs the gate. A script that calls a check which
        // checks nothing has not checked its anchors.
        match the_body_of_the_check(&script) {
            Some(body) if body.contains(GATE) => {}
            Some(_) => problems.push(format!(
                "{}: the script calls `{CHECK}`, and its body does not run `{GATE}`",
                script_path.display()
            )),
            None => problems.push(format!(
                "{}: the script has no `{CHECK}()` to call, or the function is never closed",
                script_path.display()
            )),
        }
    }

    // And the script's labels are the labels the author wrote. A label the
    // shell would change is a label a reader cannot trust, and the label is
    // the only place the log says what was tested. §190.
    problems.extend(label_problems(script_path, &script)?);

    // And the header names the ways to run the script, and the tree's
    // documents name the list of files a mutation may write to. §191.
    problems.extend(header_problems(script_path, &script));
    problems.extend(prose_problems(root));

    // And the texts the script writes are the texts the author wrote. §192.
    problems.extend(text_problems(script_path, &script)?);

    // And the copies the script makes break the script. §193.
    problems.extend(broken_copy_problems(script_path, &script)?);

    problems.sort();
    Ok(problems)
}

/// The gate.
pub fn run(root: &Path, args: &[String]) -> Result<Outcome, String> {
    crate::step("anchors");
    let script = the_script_to_check(root, args)?;
    let problems = the_anchors_are_in_the_tree_of(root, &script)?;
    if !problems.is_empty() {
        let mut message = format!("{} problem(s) in {}:\n", problems.len(), script.display());
        for problem in &problems {
            message.push_str(&format!("   • {problem}\n"));
        }
        return Err(message);
    }
    println!(
        "   ok: every anchor in {} is in the tree, each one names a single place, every label is \
         a literal the shell cannot change, the header names every way to run the script, the \
         documents name the list of files, and every copy the script makes breaks the script in one place",
        script.display()
    );
    Ok(Outcome::Ran)
}

/// The script the gate checks: the tree's own, unless another one is named.
///
/// §187. `--script PATH` is how a rule about the script's own text is broken
/// on purpose: the script cannot be edited while it runs, so a copy is
/// checked instead.
fn the_script_to_check(root: &Path, args: &[String]) -> Result<PathBuf, String> {
    let mut script = root.join(SCRIPT);
    let mut index = 0usize;
    while index < args.len() {
        let Some(flag) = args.get(index) else { break };
        match flag.as_str() {
            "--script" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("`--script` needs a path".to_string());
                };
                script = if Path::new(value).is_absolute() { PathBuf::from(value) } else { root.join(value) };
                index += 2;
            }
            other => {
                return Err(format!(
                    "`cargo xtask anchors` does not take `{other}`\n\n  usage: cargo xtask anchors \
                     [--script PATH]"
                ));
            }
        }
    }
    Ok(script)
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

    /// A tree with the script, the list and a second file in it.
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
        std::fs::write(dir.join("crates/a.rs"), "the anchor\n").expect("the fabricated file");
        std::fs::write(dir.join("crates/b.rs"), "the other file\n").expect("the fabricated file");
        dir
    }

    /// The lines the script's swap blocks are written on.
    ///
    /// Read from the script, never written down: the fabricated script
    /// grows, and a line number in a test is a number that goes stale.
    fn the_swap_lines(dir: &std::path::Path) -> Vec<usize> {
        let script = crate::tree::read_text(dir.join("scripts/mutation-test-gates.sh")).expect("the script");
        swaps(&script).expect("the script reads").iter().map(|swap| swap.line).collect()
    }

    /// The line the script's first swap block is written on.
    fn the_first_swap_line(dir: &std::path::Path) -> usize {
        the_swap_lines(dir).first().copied().unwrap_or(0)
    }

    /// The script's own anchor check, and a function that takes a label.
    ///
    /// §187: the gate looks for the **call**, not for the command the
    /// function runs, so a fabricated script with only the function is a
    /// script that never checks its anchors.
    ///
    /// §190: the reader finds the functions that take a label by looking for
    /// `$label`, and a script with none of them is a script it cannot read —
    /// it says so rather than reporting nothing. So every fabricated script
    /// carries one, as the real one does.
    fn a_check_that(body: &str) -> String {
        format!(
            "#   scripts/mutation-test-gates.sh\n\
             expect_catch() {{\n    local label=\"$1\"\n    printf '  %s\\n' \"$label\"\n}}\n\
             check_anchors() {{\n    {body}\n}}\n"
        )
    }

    /// A script and a file that agree.
    fn a_good_tree() -> std::path::PathBuf {
        let dir = a_tree("good");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/a.rs\n").expect("the fabricated list");
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            format!(
                "{}check_anchors\nswap crates/a.rs 'the anchor' 'the break'\n",
                a_check_that("cargo xtask anchors")
            ),
        )
        .expect("the fabricated script");
        std::fs::write(dir.join("crates/a.rs"), "the anchor\n").expect("the fabricated file");
        dir
    }

    #[test]
    fn the_list_is_read_from_the_file_the_script_reads() {
        let dir = a_tree("list");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/a.rs\ncrates/b.rs\n")
            .expect("the fabricated list");
        std::fs::write(dir.join("crates/a.rs"), "the anchor\n").expect("the fabricated file");
        let list = read_list(&dir).expect("the list reads");
        assert_eq!(list.paths, vec!["crates/a.rs".to_string(), "crates/b.rs".to_string()]);
        assert!(list.problems.is_empty(), "{:?}", list.problems);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A blank line is not a path, and the script refuses it too.
    #[test]
    fn a_blank_line_in_the_list_is_reported() {
        let dir = a_tree("blank");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/a.rs\n\ncrates/b.rs\n")
            .expect("the fabricated list");
        let list = read_list(&dir).expect("the list reads");
        assert_eq!(list.problems.len(), 1, "{:?}", list.problems);
        assert!(list.problems[0].contains("is blank"), "{}", list.problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The same file twice is not two files.
    #[test]
    fn the_same_file_twice_in_the_list_is_reported() {
        let dir = a_tree("twice");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/a.rs\ncrates/a.rs\n")
            .expect("the fabricated list");
        let list = read_list(&dir).expect("the list reads");
        assert_eq!(list.problems.len(), 1, "{:?}", list.problems);
        assert!(list.problems[0].contains("lists it too"), "{}", list.problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A path that is not in the tree is a file the script cannot snapshot
    /// and cannot put back.
    #[test]
    fn a_file_in_the_list_that_is_not_in_the_tree_is_reported() {
        let dir = a_tree("gone");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/a.rs\ncrates/gone.rs\n")
            .expect("the fabricated list");
        let list = read_list(&dir).expect("the list reads");
        assert_eq!(list.problems.len(), 1, "{:?}", list.problems);
        assert!(list.problems[0].contains("is not in the tree"), "{}", list.problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// And an empty list is not a list.
    #[test]
    fn an_empty_list_is_reported() {
        let dir = a_tree("empty");
        std::fs::write(dir.join("scripts/mutation-files.txt"), "").expect("the fabricated list");
        let list = read_list(&dir).expect("the list reads");
        assert_eq!(list.problems.len(), 1, "{:?}", list.problems);
        assert!(list.problems[0].contains("lists no files at all"), "{}", list.problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ------------------------------------------------------- the gate ----

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
        // The line the swap is on, read from the script rather than written
        // down: the fabricated script grows, and a line number in the test
        // is a number that goes stale.
        let line = the_first_swap_line(&dir);
        assert!(problems[0].contains(&format!("line {line}")), "{}", problems[0]);
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
        std::fs::write(dir.join("scripts/mutation-files.txt"), "crates/b.rs\n").expect("the fabricated list");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("does not have it"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file the anchor names that is not in the tree at all.
    #[test]
    fn a_file_that_is_not_in_the_tree_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            format!(
                "{}check_anchors\nswap crates/gone.rs 'the anchor' 'the break'\n",
                a_check_that("cargo xtask anchors")
            ),
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("is not in the tree")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("does not have it")), "{problems:?}");
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
            format!("{}swap crates/a.rs 'the anchor' 'the break'\n", a_check_that("cargo xtask anchors")),
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
            format!(
                "{}swap crates/a.rs 'the anchor' 'the break'\ncheck_anchors\n",
                a_check_that("cargo xtask anchors")
            ),
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("twenty-five minutes"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A script that calls a check which checks nothing has not checked its
    /// anchors. §187.
    #[test]
    fn a_script_whose_check_runs_nothing_is_reported() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            format!(
                "{}check_anchors\nswap crates/a.rs 'the anchor' 'the break'\n",
                a_check_that("echo nothing")
            ),
        )
        .expect("the fabricated script");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("does not run"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The gate checks the script it is given, so a **copy** can be broken on
    /// purpose: the script cannot be edited while it runs. §187.
    #[test]
    fn another_script_can_be_checked_against_the_same_tree() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/copy.sh"),
            format!("{}swap crates/a.rs 'the anchor' 'the break'\n", a_check_that("cargo xtask anchors")),
        )
        .expect("the copy");
        let problems =
            the_anchors_are_in_the_tree_of(&dir, &dir.join("scripts/copy.sh")).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("never calls"), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The tree's own script, unless another one is named.
    #[test]
    fn the_script_to_check_is_the_trees_own_unless_another_is_named() {
        let dir = a_tree("which");
        assert_eq!(the_script_to_check(&dir, &[]).expect("the default"), dir.join(SCRIPT));
        let named = the_script_to_check(&dir, &["--script".to_string(), "scripts/copy.sh".to_string()])
            .expect("the named script");
        assert_eq!(named, dir.join("scripts/copy.sh"));
        assert!(the_script_to_check(&dir, &["--script".to_string()]).is_err());
        assert!(the_script_to_check(&dir, &["--nope".to_string()]).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Twenty-seven anchors are used by more than one case. They are reported
    /// once, with every line that uses them.
    #[test]
    fn an_anchor_used_by_two_cases_is_reported_once_with_both_lines() {
        let dir = a_good_tree();
        std::fs::write(
            dir.join("scripts/mutation-test-gates.sh"),
            format!(
                "{}check_anchors\nswap crates/a.rs 'the anchor' 'the break'\n\
                 swap crates/a.rs 'the anchor' 'other'\n",
                a_check_that("cargo xtask anchors")
            ),
        )
        .expect("the fabricated script");
        std::fs::write(dir.join("crates/a.rs"), "the anchor\nthe anchor\n").expect("the file");
        let problems = the_anchors_are_in_the_tree(&dir).expect("the gate runs");
        assert_eq!(problems.len(), 1, "{problems:?}");
        let lines: Vec<usize> = the_swap_lines(&dir);
        assert!(problems[0].contains(&format!("lines {}, {}", lines[0], lines[1])), "{}", problems[0]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A script with no swaps at all means the reader is wrong, and a gate
    /// that cannot see the thing it checks must say so rather than pass.
    #[test]
    fn a_script_with_no_swaps_is_an_error_and_not_a_pass() {
        let dir = a_good_tree();
        std::fs::write(dir.join("scripts/mutation-test-gates.sh"), "check_anchors() {\n}\n")
            .expect("the file");
        let result = the_anchors_are_in_the_tree(&dir);
        assert!(result.is_err(), "a reader that found nothing must not report a pass");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // --------------------------------------------------------- the labels ----

    /// A script with one case, and the label it carries.
    fn a_script_with(label: &str) -> String {
        let mut script = String::new();
        script.push_str("expect_catch() {\n");
        script.push_str("    local label=\"$1\"\n");
        script.push_str("    printf '  %s\\n' \"$label\"\n");
        script.push_str("}\n");
        script.push_str("expect_catch ");
        script.push_str(label);
        script.push_str(" \"cargo xtask lists\"\n");
        script
    }

    /// A label in single quotes is literal, and the shell cannot change it.
    ///
    /// This is the defect the tree had: a backtick inside double quotes is a
    /// command substitution, and the label a reader saw was not the label the
    /// author wrote. §190.
    #[test]
    fn a_label_in_double_quotes_with_a_backtick_is_reported() {
        let script = a_script_with("\"a case with `ci` in it\"");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("backtick"), "{problems:#?}");
    }

    /// A dollar is a variable, and the shell expands it just the same.
    #[test]
    fn a_label_in_double_quotes_with_a_dollar_is_reported() {
        let script = a_script_with("\"the count is $n today\"");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
    }

    /// And the same text in single quotes is literal.
    #[test]
    fn a_label_in_single_quotes_is_a_literal() {
        let script = a_script_with("'a case with `ci` in it'");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// An escaped dollar is a dollar, and the shell does not expand it.
    ///
    /// The reader has to see the **body** of the word, not only what the
    /// shell would produce: `"a \$ b"` is literal and `"a $b"` is not.
    #[test]
    fn an_escaped_dollar_in_double_quotes_is_a_literal() {
        let script = a_script_with("\"a case with \\$ in it\"");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A label that is only a variable is not a literal, and the shell
    /// would change it.
    ///
    /// This is the shape a helper takes when it hands a label on, and the
    /// tree has none: every label is written where the case is. §190.
    #[test]
    fn a_label_that_is_only_a_variable_is_reported() {
        let script = a_script_with("$label");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("bare word"), "{problems:#?}");
    }

    /// The functions that take a label are **derived**, not written down.
    ///
    /// A helper that takes a label cannot be added without the gate
    /// noticing, because there is no list to forget to update. §190.
    #[test]
    fn a_new_helper_that_takes_a_label_is_checked_too() {
        let mut script = String::new();
        script.push_str("expect_catch() {\n    local label=\"$1\"\n}\n");
        script.push_str("expect_other() {\n    local label=\"$1\"\n}\n");
        script.push_str("expect_other \"a case with `ci` in it\" \"cargo xtask lists\"\n");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
    }

    /// And a function that does not take a label is not one.
    #[test]
    fn a_function_that_does_not_take_a_label_is_not_one() {
        let mut script = String::new();
        script.push_str("expect_catch() {\n    local label=\"$1\"\n}\n");
        script.push_str("expect_other() {\n    local gate=\"$1\"\n}\n");
        script.push_str("expect_other \"a case with `ci` in it\" \"cargo xtask lists\"\n");
        let found = labels(&script).expect("the script reads");
        assert!(found.is_empty(), "{found:#?}");
    }

    /// A label in a comment is not a label: a comment is not code.
    #[test]
    fn a_label_in_a_comment_is_not_a_label() {
        let mut script = String::new();
        script.push_str("expect_catch() {\n    local label=\"$1\"\n}\n");
        script.push_str("# expect_catch \"a case with `ci` in it\" \"cargo xtask lists\"\n");
        let problems = label_problems(Path::new("s.sh"), &script).expect("the script reads");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A script with no function that takes a label means the reader is
    /// wrong, and it must say so rather than report nothing.
    #[test]
    fn a_script_with_no_label_taking_function_is_an_error_and_not_a_pass() {
        let script = "expect_catch() {\n    local gate=\"$1\"\n}\nexpect_catch \"a\" \"b\"\n";
        let result = label_problems(Path::new("s.sh"), script);
        assert!(result.is_err(), "a reader that found nothing must not report a pass");
    }

    // ---------------------------------------------------------- the header ----

    /// A script whose header names these invocations, and whose code accepts
    /// these flags.
    fn a_script_with_a_header(header: &[&str], accepts: &[&str]) -> String {
        let mut script = String::new();
        script.push_str("#!/usr/bin/env bash\n");
        script.push_str("# The header of the script.\n");
        for flag in header {
            script.push_str(&format!("#   scripts/mutation-test-gates.sh {flag}\n"));
        }
        script.push_str("set -uo pipefail\n");
        script.push_str("expect_catch() {\n    local label=\"$1\"\n}\n");
        for flag in accepts {
            script.push_str(&format!("if [[ \"${{1:-}}\" == \"{flag}\" ]]; then\n    exit 0\nfi\n"));
        }
        script
    }

    /// The flags are read from the script's own code, so there is no list to
    /// keep. §191.
    #[test]
    fn the_flags_are_read_from_the_scripts_own_code() {
        let script = a_script_with_a_header(&[], &["--check", "--self-test"]);
        assert_eq!(script_flags(&script), vec!["--check", "--self-test"]);
    }

    /// A header that misses a flag the script accepts sends a reader
    /// nowhere. §191.
    #[test]
    fn a_header_that_misses_a_flag_is_reported() {
        let script = a_script_with_a_header(&["--self-test"], &["--check", "--self-test"]);
        let problems = header_problems(Path::new("s.sh"), &script);
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("--check"), "{problems:#?}");
    }

    /// And a header that names a flag the script does not accept sends a
    /// reader to an error.
    #[test]
    fn a_header_that_names_a_flag_the_script_does_not_accept_is_reported() {
        let script = a_script_with_a_header(&["--check", "--self-test"], &["--self-test"]);
        let problems = header_problems(Path::new("s.sh"), &script);
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("--check"), "{problems:#?}");
    }

    /// A header that names every way to run the script is right.
    #[test]
    fn a_header_that_names_every_flag_is_not_reported() {
        let script = a_script_with_a_header(&["--check", "--self-test"], &["--check", "--self-test"]);
        let problems = header_problems(Path::new("s.sh"), &script);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// The header is the script's own copy of its interface, and a header
    /// that never names the script says nothing about how to run it.
    #[test]
    fn a_header_that_never_names_the_script_is_reported() {
        let script = "#!/usr/bin/env bash\n# A header with no name in it.\nset -uo pipefail\n";
        let problems = header_problems(Path::new("s.sh"), script);
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("never names"), "{problems:#?}");
    }

    // ------------------------------------------------------- the documents ----

    /// A document that tells a contributor to add a file to the script's
    /// array is telling them to do something that does not survive the next
    /// run. §191.
    #[test]
    fn a_document_that_names_the_scripts_array_is_reported() {
        let dir = a_tree("array");
        std::fs::write(
            dir.join("CONTRIBUTING.md"),
            "**A mutation's target file must also be in that script's `FILES` array.**\n",
        )
        .expect("the fabricated document");
        let problems = prose_problems(&dir);
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("CONTRIBUTING.md"), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A document that names the file is right.
    #[test]
    fn a_document_that_names_the_file_is_not_reported() {
        let dir = a_tree("thefile");
        std::fs::write(
            dir.join("CONTRIBUTING.md"),
            "**A mutation's target file must also be in `scripts/mutation-files.txt`.**\n",
        )
        .expect("the fabricated document");
        let problems = prose_problems(&dir);
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The format change's own `FILES` constant is a different thing, and a
    /// document that is about it is not naming the script's array.
    #[test]
    fn the_format_changes_own_files_constant_is_not_the_scripts_array() {
        let dir = a_tree("format");
        std::fs::write(
            dir.join("CONTRIBUTING.md"),
            "The `FILES` table in `crates/io/src/format/pending.rs` is a different thing.\n",
        )
        .expect("the fabricated document");
        let problems = prose_problems(&dir);
        assert!(problems.is_empty(), "{problems:#?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------------------------------------------------------- the texts ----

    /// The texts a mutation call writes are found: the anchor of a `swap`,
    /// the text of an `append`, and the two of a `broken_copy`.
    #[test]
    fn the_text_arguments_are_found() {
        let script = "swap crates/a.rs 'the anchor' 'the break'\n\
                      append ATTRIBUTION.md 'a line'\n\
                      broken_copy \"$SNAPSHOT/x.sh\" 'from' 'to'\n";
        let found = text_arguments(script).expect("the script reads");
        assert_eq!(found.len(), 4, "{found:#?}");
    }

    /// A text the shell would change is not the text the author wrote: the
    /// gate reads the anchor and finds it, and the shell runs a command and
    /// does not. §192.
    #[test]
    fn a_double_quoted_text_with_a_backtick_is_reported() {
        let script = "swap crates/a.rs \"an anchor with `cmd` in it\" 'the break'\n";
        let problems = text_problems(Path::new("s.sh"), script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("`swap`"), "{problems:#?}");
    }

    /// And a single-quoted one is literal.
    #[test]
    fn a_single_quoted_text_is_a_literal() {
        let script = "swap crates/a.rs 'an anchor with `cmd` in it' 'the break'\n";
        let problems = text_problems(Path::new("s.sh"), script).expect("the script reads");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A **command** is meant to be run, and the shell may expand it: the
    /// gate command names the snapshot directory, and it has to. §192.
    #[test]
    fn a_command_may_be_expanded_but_a_text_may_not() {
        let script = "expect_catch 'a label' \"cargo xtask anchors --script $SNAPSHOT/s.sh\"\n\
                      swap crates/a.rs \"an anchor with `cmd` in it\" 'the break'\n";
        let problems = text_problems(Path::new("s.sh"), script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("`swap`"), "{problems:#?}");
    }

    // -------------------------------------------------- the copies' anchors ----

    /// A `broken_copy` names an anchor the way a `swap` does, and the gate
    /// checks it before the run starts. §193.
    #[test]
    fn the_broken_copies_are_found() {
        let script = "broken_copy \"$SNAPSHOT/x.sh\" 'from' 'to'\n";
        let copies = broken_copies(script).expect("the script reads");
        assert_eq!(copies.len(), 1, "{copies:#?}");
        assert_eq!(copies[0].text, "from");
        assert_eq!(copies[0].out, "$SNAPSHOT/x.sh");
        assert_eq!(copies[0].line, 1);
    }

    /// The copy looks its text up in the script's own text, so the text is
    /// the word the shell makes and not the text between the quotes. §193.
    #[test]
    fn the_text_of_a_copy_is_the_word_the_shell_makes() {
        let script = "broken_copy \"$SNAPSHOT/x.sh\" 'a''b' 'to'\n";
        let copies = broken_copies(script).expect("the script reads");
        assert_eq!(copies[0].text, "ab", "{copies:#?}");
    }

    /// A text the script does not hold makes a copy that changes nothing,
    /// and the run stops when the case is reached — at the end of it. §193.
    #[test]
    fn a_broken_copy_whose_text_is_not_in_the_script_is_reported() {
        let script = "broken_copy out.sh 'a''b' 'to'\n";
        let problems = broken_copy_problems(Path::new("s.sh"), script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("not in the script"), "{problems:#?}");
    }

    /// The copy replaces the **first** place its text is in, and the call's
    /// own argument is a place its text is in. So a call written before the
    /// text it names makes a copy that breaks the call and not the script.
    /// §193.
    #[test]
    fn a_broken_copy_whose_text_is_its_own_argument_is_reported() {
        let script = "broken_copy out.sh 'nowhere' 'to'\n";
        let problems = broken_copy_problems(Path::new("s.sh"), script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("inside the call itself"), "{problems:#?}");
    }

    /// And a text that comes first is a text the copy breaks.
    #[test]
    fn a_broken_copy_whose_text_comes_first_is_a_literal() {
        let script = "nowhere\nbroken_copy out.sh 'nowhere' 'to'\n";
        let problems = broken_copy_problems(Path::new("s.sh"), script).expect("the script reads");
        assert!(problems.is_empty(), "{problems:#?}");
    }

    /// A text in two places is a text whose copy breaks whichever comes
    /// first, and that may not be the place the case meant. §193.
    #[test]
    fn a_broken_copy_whose_text_is_in_two_places_is_reported() {
        let script = "nowhere\nand nowhere again\nbroken_copy out.sh 'nowhere' 'to'\n";
        let problems = broken_copy_problems(Path::new("s.sh"), script).expect("the script reads");
        assert_eq!(problems.len(), 1, "{problems:#?}");
        assert!(problems[0].contains("2 places"), "{problems:#?}");
    }

    /// The line an offset is on is the line the offset is on, even when the
    /// offset is in the middle of it. §193.
    #[test]
    fn a_line_is_the_line_the_offset_is_on() {
        let text = "one\ntwo three\nfour\n";
        assert_eq!(line_of(text, 0), 1);
        assert_eq!(line_of(text, 4), 2);
        // The start of "three", in the middle of the second line.
        assert_eq!(line_of(text, 8), 2);
        assert_eq!(line_of(text, 14), 3);
    }
}
