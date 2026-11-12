//! The tree's **index**: what commands exist, and where they are.
//!
//! # Why an index and not the registry
//!
//! The registry is the truth about the commands of a **program**: it is
//! filled by the linker and holds every `CommandSpec`. The xtask cannot read
//! it — the xtask links nothing of the tree's own, because a checker that
//! cannot be built when the tree does not compile is a checker that is
//! silent exactly when it is needed.
//!
//! So the xtask reads the tree as **text**, and the tree's own test compares
//! that reading with the registry and fails when they disagree (§848). One
//! truth, two readings, and a test that says they are the same.
//!
//! # What the index holds
//!
//! - the **commands**: their id, their label, their first line of
//!   documentation, the file and the line they are written on, and whether
//!   the battery marked one as its example;
//! - the **modules** of each battery: the files of its source, and the files
//!   of its tests;
//! - the **bounds** the batteries declare, with the file they are in.
//!
//! Every one of these is a question an arc asks before it writes a line, and
//! every one of them was a search.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One command of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The command's id: `<area>.<thing>`.
    pub id: String,
    /// The label, when the command gives one.
    pub label: Option<String>,
    /// The first line of the command's documentation.
    pub doc: Option<String>,
    /// The file the command is written in, relative to the root.
    pub file: PathBuf,
    /// The line the command's attribute is on.
    pub line: usize,
    /// The battery the command belongs to: the crate it is in.
    pub battery: String,
    /// Whether the battery marked this command as its example.
    pub example: bool,
}

impl Command {
    /// Where the command is, for a report: `crates/x/src/y.rs:42`.
    pub fn where_(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

/// The tree's index.
#[derive(Debug, Default, Clone)]
pub struct Index {
    /// Every command, sorted by id.
    pub commands: Vec<Command>,
    /// The files of each battery's source, by battery.
    pub modules: BTreeMap<String, Vec<PathBuf>>,
    /// The files of each battery's tests, by battery.
    pub tests: BTreeMap<String, Vec<PathBuf>>,
    /// The public bounds the batteries declare: the name, the value and the
    /// file.
    pub bounds: Vec<Bound>,
}

/// One bound a battery declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bound {
    /// The name of the constant.
    pub name: String,
    /// The value, as it is written.
    pub value: String,
    /// The file it is written in, relative to the root.
    pub file: PathBuf,
    /// The line it is written on.
    pub line: usize,
    /// The battery it belongs to.
    pub battery: String,
}

/// The batteries of the tree, and the crate of each.
///
/// The crates whose name begins with `craftkit-plugin-` and that have a
/// `src` directory are batteries; the rest of the plugins are not.
pub fn battery_names(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(root.join("crates")) else {
        return found;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(battery) = name.strip_prefix("craftkit-plugin-") else { continue };
        if !entry.path().join("src").is_dir() {
            continue;
        }
        found.push(battery.to_string());
    }
    found.sort();
    found
}

/// Every Rust file under a directory, sorted.
pub fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(directory, &mut found);
    found.sort();
    found
}

fn walk(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            found.push(path);
        }
    }
}

/// Build the index of the tree.
pub fn build(root: &Path) -> Result<Index, String> {
    let mut index = Index::default();
    for battery in battery_names(root) {
        let crate_directory = root.join("crates").join(format!("craftkit-plugin-{battery}"));
        let source = crate_directory.join("src");
        let files = rust_files(&source);
        index.modules.insert(battery.clone(), files.clone());
        let tests = rust_files(&crate_directory.join("tests"));
        index.tests.insert(battery.clone(), tests);
        for file in files {
            let text = crate::tree::read_rust(&file, crate::tree::Subject::Everything)
                .map_err(|e| format!("could not read {}: {e}", file.display()))?;
            let relative = relative_to(root, &file);
            index.commands.extend(commands_in(&text, &relative, &battery));
            index.commands.extend(table_commands_in(&text, &relative, &battery));
            index.bounds.extend(bounds_in(&text, &relative, &battery));
        }
    }
    index.commands.sort_by(|a, b| a.id.cmp(&b.id));
    index.bounds.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(index)
}

/// A path relative to the root.
pub fn relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).map(Path::to_path_buf).unwrap_or_else(|_| path.to_path_buf())
}

/// The commands in a file of source.
///
/// **The attribute is read as text.** A command's declaration may be written
/// across several lines, so the reader takes the whole `#[command( … )]`
/// block and reads the fields out of it.
pub fn commands_in(text: &str, file: &Path, battery: &str) -> Vec<Command> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[command(") {
        let start = offset + at;
        let body_start = start + "#[command(".len();
        let Some((block, end)) = block_of(text, body_start) else { break };
        let line = text[..start].lines().count().max(1);
        if let Some(id) = field(&block, "id") {
            found.push(Command {
                id,
                label: field(&block, "label"),
                doc: doc_above(text, start),
                file: file.to_path_buf(),
                line,
                battery: battery.to_string(),
                example: has_flag(&block, "example"),
            });
        }
        offset = end;
    }
    found
}

/// The commands in a **table** of commands.
///
/// A table is the tree's second spelling of a command: `commands! { … }`
/// declares a family of them, and a row says the id, the label, the doc
/// and the flag. A tool that reads the source and does not know the table
/// is a tool that is **wrong about a battery that is a table** — and the
/// fx battery is one.
///
/// So the reader reads both, and the counts of the tree are the counts of
/// the commands, however they are written.
pub fn table_commands_in(text: &str, file: &Path, battery: &str) -> Vec<Command> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("commands!") {
        let start = offset + at;
        let Some(open) = text[start..].find('{') else { break };
        let body_start = start + open + 1;
        let Some((block, end)) = braces_of(text, body_start) else { break };
        found.extend(rows_of(&block, text, body_start, file, battery));
        offset = end;
    }
    found
}

/// The rows of a table: the commands it declares.
///
/// A row is an id and a block: `fx.layer.switch { … }`.
fn rows_of(block: &str, text: &str, body_start: usize, file: &Path, battery: &str) -> Vec<Command> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while offset < block.len() {
        // The id is the text before the row's opening brace.
        let Some(open) = block[offset..].find('{') else { break };
        let head = block[offset..offset + open].trim();
        let id: String = head.split_whitespace().last().unwrap_or("").to_string();
        // **A row's id is dotted and lower case**, and nothing else is: a
        // comment that ends in a full stop is not a row.
        if !is_an_id(&id) {
            // Not a row: the text is a comment, or a word.
            offset += open + 1;
            continue;
        }
        let row_start = offset + open + 1;
        let Some((row, end)) = braces_of(block, row_start) else { break };
        // The id's own place: the head may hold a comment, and the
        // documentation is above the **id**.
        let head_text = &block[offset..offset + open];
        let id_at = offset + head_text.trim_end().len() - id.len();
        let at = body_start + id_at;
        let line = text[..at].lines().count().max(1);
        found.push(Command {
            id: id.clone(),
            label: entry_of(&row, "label"),
            // The documentation of a row is **inside** the table, above
            // the row: the table is the file's own text and the row is
            // part of it.
            doc: doc_above(block, id_at),
            file: file.to_path_buf(),
            line,
            battery: battery.to_string(),
            example: has_row_flag(&row, "example"),
        });
        offset = end;
    }
    found
}

/// Whether a word is a command's **id**: dotted, lower case.
///
/// A row's id is `<area>.<thing>`, and a sentence that ends in a full
/// stop is not an id. The reader that counts the rows has to tell them
/// apart, or it counts the prose.
pub fn is_an_id(word: &str) -> bool {
    let mut parts = word.split('.');
    let Some(first) = parts.next() else { return false };
    if first.is_empty() || !first.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
        return false;
    }
    let mut more = false;
    for part in parts {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return false;
        }
        more = true;
    }
    more
}

/// The body of a block of braces, and where it ends.
///
/// **The braces are counted, and the strings are skipped.** A run body may
/// hold a string with a brace in it, and a reader that stopped at the
/// first `}` would read half a row and call it the whole.
///
/// **And the characters are skipped too.** A body may hold the character
/// `'"'` — a quote, as a character — and a reader that took it for the
/// start of a string would read the rest of the file as text.
fn braces_of(text: &str, body_start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut at = body_start;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '\'' => {
                    if let Some(step) = the_character(bytes, at) {
                        at += step;
                        continue;
                    }
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((text[body_start..at].to_string(), at + 1));
                    }
                }
                _ => {}
            }
        }
        at += 1;
    }
    None
}

/// The length of the character literal at `at`, if there is one.
///
/// A character is `'x'` or `'\\x'`, and it is **text**: the braces and
/// the quotes in it are not the file's braces and quotes. A lifetime — `'a`
/// — is not a character, and this says so.
pub fn the_character(bytes: &[u8], at: usize) -> Option<usize> {
    // `'x'` — three bytes, and the middle one may be a quote.
    if bytes.get(at + 2) == Some(&b'\'') {
        return Some(3);
    }
    // `'\\x'` — four bytes.
    if bytes.get(at + 1) == Some(&b'\\') && bytes.get(at + 3) == Some(&b'\'') {
        return Some(4);
    }
    None
}

/// An entry of a row: `name: "value"`.
///
/// The table writes a colon and the attribute writes an equals, and a
/// reader that knows only one of them reads half a row.
pub fn entry_of(block: &str, name: &str) -> Option<String> {
    for needle in [format!("{name}: \""), format!("{name} = \"")] {
        let Some(at) = block.find(&needle) else { continue };
        let rest = &block[at + needle.len()..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    None
}

/// The body of an attribute, and where it ends.
///
/// **The parentheses are counted, and the strings are skipped.** An
/// attribute may hold a path — `menu("Edit", "Trim")` — and the `)` of the
/// path is not the end of the attribute. A reader that stopped at the first
/// `)` would read half an attribute and call it the whole.
fn block_of(text: &str, body_start: usize) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut at = body_start;
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '\'' => {
                    if let Some(step) = the_character(bytes, at) {
                        at += step;
                        continue;
                    }
                }
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((text[body_start..at].to_string(), at + 1));
                    }
                }
                _ => {}
            }
        }
        at += 1;
    }
    None
}

/// Whether an attribute block carries a **flag**: a word of its own.
///
/// **A word, not a substring.** An id may hold the word — a command called
/// `pdf.example.set` is not a flag — so the block is split into words and
/// the word is looked for among them.
pub fn has_flag(block: &str, flag: &str) -> bool {
    block.split(|c: char| !(c.is_alphanumeric() || c == '_')).any(|word| word == flag)
}

/// Whether a **row** carries a flag: a word of its own, at the row's level.
///
/// A row's flags are its own words: `mutates`, `undoable`, `example`. The
/// row also holds the **parameters** and the **body**, and those hold
/// words of their own — a parameter's documentation may say
/// `https://example.com` — and a word in a parameter's documentation is
/// not a flag of the command. So the row is read **by its braces**: only
/// the words at the row's own level are its flags.
pub fn has_row_flag(row: &str, flag: &str) -> bool {
    let bytes = row.as_bytes();
    let mut depth = 0i32;
    let mut at = 0usize;
    let mut word = String::new();
    let mut in_string = false;
    let mut escaped = false;
    while at < bytes.len() {
        let c = bytes[at] as char;
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            at += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
        } else if c == '{' || c == '(' {
            depth += 1;
        } else if c == '}' || c == ')' {
            depth -= 1;
        } else if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            if depth == 0 && word == flag {
                return true;
            }
            word.clear();
        }
        at += 1;
    }
    depth == 0 && word == flag
}

/// A field of an attribute block: `name = "value"`.
fn field(block: &str, name: &str) -> Option<String> {
    let needle = format!("{name} = \"");
    let at = block.find(&needle)?;
    let rest = &block[at + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The first line of the documentation above a declaration.
///
/// **The doc comment, not the prose above it.** A command's documentation is
/// the `///` lines immediately before it; anything else is a note about the
/// code, and a note about the code is not the command's documentation.
fn doc_above(text: &str, at: usize) -> Option<String> {
    let before = &text[..at];
    let mut lines: Vec<&str> = Vec::new();
    let mut started = false;
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // A blank line between the documentation and the attribute is
            // still the same documentation; a blank line after the
            // documentation has started is the end of it.
            if started {
                break;
            }
            continue;
        }
        if trimmed.starts_with("#[") {
            // **The derive is between the documentation and the attribute.**
            // The documentation of a command is above its derive, not above
            // its attribute.
            if started {
                break;
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("///") {
            lines.push(rest.trim());
            started = true;
            continue;
        }
        break;
    }
    lines.reverse();
    lines.into_iter().find(|line| !line.is_empty()).map(|line| line.to_string())
}

/// The public bounds a file declares: `pub const NAME: … = …;`.
///
/// The name, the value and the file: the three things the tree's prose has
/// to agree with.
pub fn bounds_in(text: &str, file: &Path, battery: &str) -> Vec<Bound> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub const ") else { continue };
        let Some((name, rest)) = rest.split_once(':') else { continue };
        let Some((_kind, value)) = rest.split_once('=') else { continue };
        let name = name.trim();
        if !name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit()) {
            continue;
        }
        let value = value.trim().trim_end_matches(';').trim().to_string();
        // **A bound is a number.** The tree's bounds are the numbers a
        // document may not exceed, and a constant that holds a list is not a
        // bound — so the reader says no rather than guessing.
        if !value.starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        found.push(Bound {
            name: name.to_string(),
            value,
            file: file.to_path_buf(),
            line: number + 1,
            battery: battery.to_string(),
        });
    }
    found
}

/// The commands of one battery.
pub fn commands_of<'a>(index: &'a Index, battery: &str) -> Vec<&'a Command> {
    index.commands.iter().filter(|c| c.battery == battery).collect()
}

/// The commands of one area.
pub fn commands_in_area<'a>(index: &'a Index, area: &str) -> Vec<&'a Command> {
    index.commands.iter().filter(|c| c.id.starts_with(&format!("{area}."))).collect()
}
