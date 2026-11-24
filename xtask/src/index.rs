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

/// One **kind** of the tree: a node kind, or an evaluator.
///
/// A kind is what the document may hold, and an evaluator is what works
/// out what a document means. Both are **named**: the id is the name the
/// doors and the plans use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    /// The kind's id: `<area>.<thing>`.
    pub id: String,
    /// The file the kind is written in, relative to the root.
    pub file: PathBuf,
    /// The line the kind's attribute is on.
    pub line: usize,
    /// The battery the kind belongs to: the crate it is in.
    pub battery: String,
}

impl Kind {
    /// Where the kind is, for a report: `crates/x/src/y.rs:42`.
    pub fn where_(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

/// The tree's index.
#[derive(Debug, Default, Clone)]
pub struct Index {
    /// Every command, sorted by id.
    pub commands: Vec<Command>,
    /// Every kind, sorted by id.
    pub kinds: Vec<Kind>,
    /// Every migration, sorted by id.
    pub migrations: Vec<Migration>,
    /// Every panel, sorted by id.
    pub panels: Vec<Panel>,
    /// Every tool, sorted by id.
    pub tools: Vec<Tool>,
    /// Every evaluator, sorted by id.
    pub evaluators: Vec<Evaluator>,
    /// Every layer, sorted by id.
    pub layers: Vec<Layer>,
    /// Every format, sorted by id.
    pub formats: Vec<Format>,
    /// The files of each battery's source, by battery.
    pub modules: BTreeMap<String, Vec<PathBuf>>,
    /// The files of each battery's tests, by battery.
    pub tests: BTreeMap<String, Vec<PathBuf>>,
    /// The public bounds the batteries declare: the name, the value and the
    /// file.
    pub bounds: Vec<Bound>,
}

/// A field of a **struct literal**: `name: "value"`.
///
/// A static that carries its own specification writes its fields with a
/// colon, and the tree's readers have to read both spellings: an
/// attribute's `name = "value"` and a literal's `name: "value"`.
pub fn field_colon(block: &str, name: &str) -> Option<String> {
    let needle = format!("{name}: \"");
    let at = block.find(&needle)?;
    let rest = &block[at + needle.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// The **evaluators** a file declares.
///
/// An evaluator is a gesture about the **whole document**: it may look at
/// every node, count them, and say something about the whole. It is
/// written two ways — an attribute on a type, and a static that carries
/// the specification itself — and the reader reads both.
pub fn evaluators_in(text: &str, file: &Path, crate_name: &str) -> Vec<Evaluator> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[evaluator(") {
        let start = offset + at;
        let Some((block, end)) = block_of(text, start + "#[evaluator(".len()) else { break };
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = field(&block, "id")
        {
            found.push(Evaluator {
                id,
                file: file.to_path_buf(),
                line: text[..start].lines().count().max(1),
                crate_name: crate_name.to_string(),
            });
        }
        offset = end;
    }
    found.extend(specs_in(text, "EvaluatorSpec", "id").into_iter().map(|(id, line)| Evaluator {
        id,
        file: file.to_path_buf(),
        line,
        crate_name: crate_name.to_string(),
    }));
    found
}

/// The **layers** a file declares.
///
/// A layer is a group of contributions that depend on each other, and it
/// is written two ways — an attribute on a type, and a static that carries
/// the specification itself — and the reader reads both.
pub fn layers_in(text: &str, file: &Path, crate_name: &str) -> Vec<Layer> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[layer(") {
        let start = offset + at;
        let Some((block, end)) = block_of(text, start + "#[layer(".len()) else { break };
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = field(&block, "id")
        {
            found.push(Layer {
                id,
                file: file.to_path_buf(),
                line: text[..start].lines().count().max(1),
                crate_name: crate_name.to_string(),
            });
        }
        offset = end;
    }
    found.extend(specs_in(text, "LayerSpec", "id").into_iter().map(|(id, line)| Layer {
        id,
        file: file.to_path_buf(),
        line,
        crate_name: crate_name.to_string(),
    }));
    found
}

/// The **formats** a file declares.
///
/// A format is a version of the document format: it has a tag, a version
/// and a layer, and it is written as a static that carries the
/// specification itself.
pub fn formats_in(text: &str, file: &Path, crate_name: &str) -> Vec<Format> {
    specs_in(text, "FormatSpec", "tag")
        .into_iter()
        .map(|(id, line)| Format { id, file: file.to_path_buf(), line, crate_name: crate_name.to_string() })
        .collect()
}

/// The **specifications** a file writes as statics: `pub static X: Spec { … }`.
///
/// The **name** of the specification is the field the reader is told to
/// read — an evaluator's id, a layer's id, a format's tag — and a static
/// that does not carry that field is not one of them. The line is the
/// line the static is on.
fn specs_in(text: &str, spec: &str, field_name: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find(spec) {
        let start = offset + at;
        let after = start + spec.len();
        // **The specification, and not a mention of it.** A type
        // annotation — `Registry<EvaluatorSpec>` — names the
        // specification without being one, and a reader that read the
        // block after the mention would read a field that belongs to
        // something else entirely.
        let rest = &text[after..];
        let trimmed = rest.trim_start();
        if !trimmed.starts_with('{') {
            offset = after;
            continue;
        }
        let brace = after + (rest.len() - trimmed.len());
        let Some((block, end)) = brace_block_of(text, brace + 1) else { break };
        if let Some(id) = field_colon(&block, field_name) {
            found.push((id, text[..start].lines().count().max(1)));
        }
        offset = end;
    }
    found
}

/// The **block** a brace opens, and where it ends.
///
/// The same walk as `block_of`, for the braces of a struct literal: the
/// depth is counted, a string is not a brace, and a character literal is
/// skipped whole.
fn brace_block_of(text: &str, body_start: usize) -> Option<(String, usize)> {
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

/// One **evaluator** the tree declares: a gesture about the whole
/// document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluator {
    /// The evaluator's id: `<area>.<thing>`.
    pub id: String,
    /// The file the evaluator is written in, relative to the root.
    pub file: PathBuf,
    /// The line the evaluator is on.
    pub line: usize,
    /// The crate the evaluator belongs to.
    pub crate_name: String,
}

/// One **layer** the tree declares: a group of contributions that depend
/// on each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// The layer's id: a word.
    pub id: String,
    /// The file the layer is written in, relative to the root.
    pub file: PathBuf,
    /// The line the layer is on.
    pub line: usize,
    /// The crate the layer belongs to.
    pub crate_name: String,
}

/// One **format** the tree declares: a version of the document format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// The format's tag: `<family>/<version>`.
    pub id: String,
    /// The file the format is written in, relative to the root.
    pub file: PathBuf,
    /// The line the format is on.
    pub line: usize,
    /// The crate the format belongs to.
    pub crate_name: String,
}

/// One **tool** the tree declares: a gesture that runs a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The tool's id: `tool.<thing>`.
    pub id: String,
    /// The file the tool is written in, relative to the root.
    pub file: PathBuf,
    /// The line the tool's attribute is on.
    pub line: usize,
    /// The crate the tool belongs to.
    pub crate_name: String,
}

impl Tool {
    /// Where the tool is, for a report: `crates/x/src/y.rs:42`.
    pub fn where_(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

/// One **panel** the tree declares: a battery's own window of commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// The panel's id: `panel.<thing>`.
    pub id: String,
    /// The file the panel is written in, relative to the root.
    pub file: PathBuf,
    /// The line the panel's attribute is on.
    pub line: usize,
    /// The crate the panel belongs to.
    pub crate_name: String,
}

impl Panel {
    /// Where the panel is, for a report: `crates/x/src/y.rs:42`.
    pub fn where_(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
}

/// One **migration** the tree declares: a step from one version of a
/// file format to the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    /// The step's id: `<family>.<from>to<to>`.
    pub id: String,
    /// The file the step is written in, relative to the root.
    pub file: PathBuf,
    /// The line the step's attribute is on.
    pub line: usize,
    /// The crate the step belongs to.
    pub crate_name: String,
}

impl Migration {
    /// Where the step is, for a report: `crates/x/src/y.rs:42`.
    pub fn where_(&self) -> String {
        format!("{}:{}", self.file.display(), self.line)
    }
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

/// The **limits** a file declares: the public constants whose names say
/// they are a limit.
///
/// A limit is `MAX_…` or `…_LIMIT`: the first says it is the most of
/// something, and the second says the same thing the other way round. A
/// private constant is the program's own business and is not read: only
/// a **public** limit is a promise or a cap somebody else may rely on.
pub fn limits_in(text: &str, file: &Path, battery: &str) -> Vec<Bound> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub const ") else { continue };
        let Some((name, rest)) = rest.split_once(':') else { continue };
        let name = name.trim();
        if !(name.starts_with("MAX_") || name.ends_with("_LIMIT")) {
            continue;
        }
        let value =
            rest.split_once('=').map(|(_, value)| value.trim().trim_end_matches(';').trim().to_string());
        found.push(Bound {
            name: name.to_string(),
            value: value.unwrap_or_default(),
            file: file.to_path_buf(),
            line: number + 1,
            battery: battery.to_string(),
        });
    }
    found
}

/// The **promise bounds** a file declares: `MAX_…_BYTES`, `MAX_…_MEMORY`
/// and `MAX_…_STEPS`.
///
/// A bound whose name says **what it bounds** is a promise: a reader may
/// rely on it, and the tree's own gate holds it to the prose. The value
/// is read as it is written — a promise may be written as a product of
/// other numbers — and the name is what the rule is about.
pub fn promise_bounds_in(text: &str, file: &Path, battery: &str) -> Vec<Bound> {
    let mut found = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub const ") else { continue };
        let Some((name, rest)) = rest.split_once(':') else { continue };
        let name = name.trim();
        if !is_a_promise_bound(name) {
            continue;
        }
        let value =
            rest.split_once('=').map(|(_, value)| value.trim().trim_end_matches(';').trim().to_string());
        found.push(Bound {
            name: name.to_string(),
            value: value.unwrap_or_default(),
            file: file.to_path_buf(),
            line: number + 1,
            battery: battery.to_string(),
        });
    }
    found
}

/// Whether a bound's name is a **promise**: it says what it bounds.
///
/// The shape is the tree's own: `MAX_`, and then what it bounds —
/// bytes, memory or steps. A cap on a count of entries is the program's
/// own business and is not a promise to a reader.
pub fn is_a_promise_bound(name: &str) -> bool {
    name.starts_with("MAX_")
        && (name.ends_with("BYTES") || name.ends_with("MEMORY") || name.ends_with("STEPS"))
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
        // **The macros of the battery are read first.** A macro may be
        // called in a file other than the one it is written in, so the
        // definitions are gathered before the calls are read.
        //
        // **The kinds are read from the production code.** A kind is a
        // rule the tree has, and a rule written in a test is an example
        // of one: the derive crate's own test holds a declaration in a
        // string, and a reader that counted it would hold the tree to
        // account for a kind that does not ship.
        let mut macros = Vec::new();
        let mut texts = Vec::new();
        let mut kinds_texts = Vec::new();
        for file in &files {
            let text = crate::tree::read_rust(file, crate::tree::Subject::Everything)
                .map_err(|e| format!("could not read {}: {e}", file.display()))?;
            let production = crate::tree::read_rust(file, crate::tree::Subject::Production)
                .map_err(|e| format!("could not read {}: {e}", file.display()))?;
            let relative = relative_to(root, file);
            macros.extend(macro_definitions_in(&production, &relative));
            texts.push((text, relative.clone()));
            kinds_texts.push((production, relative));
        }
        for (text, relative) in texts {
            index.commands.extend(commands_in(&text, &relative, &battery));
            index.commands.extend(table_commands_in(&text, &relative, &battery));
            index.commands.extend(macro_commands_in(&text, &relative, &battery, &macros));
            index.bounds.extend(bounds_in(&text, &relative, &battery));
        }
        for (text, relative) in kinds_texts {
            index.kinds.extend(kinds_in(&text, &relative, &battery));
            index.kinds.extend(macro_kinds_in(&text, &relative, &battery, &macros));
            index.panels.extend(panels_in(&text, &relative, &battery));
            index.tools.extend(tools_in(&text, &relative, &battery));
            index.evaluators.extend(evaluators_in(&text, &relative, &battery));
            index.layers.extend(layers_in(&text, &relative, &battery));
            index.formats.extend(formats_in(&text, &relative, &battery));
        }
    }
    index.commands.sort_by(|a, b| a.id.cmp(&b.id));
    index.kinds.sort_by(|a, b| a.id.cmp(&b.id));
    index.panels.sort_by(|a, b| a.id.cmp(&b.id));
    index.tools.sort_by(|a, b| a.id.cmp(&b.id));
    index.evaluators.sort_by(|a, b| a.id.cmp(&b.id));
    index.layers.sort_by(|a, b| a.id.cmp(&b.id));
    index.formats.sort_by(|a, b| a.id.cmp(&b.id));
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

/// The **tools** a file declares, where the attribute is at the start of
/// its line.
///
/// A tool is a gesture that runs a command, and it is a rule the kit
/// publishes as much as a panel is. The attribute is at the start of its
/// line when it is a **declaration**: an attribute in a comment is an
/// example of one, and the example is not a tool.
pub fn tools_in(text: &str, file: &Path, crate_name: &str) -> Vec<Tool> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[tool(") {
        let start = offset + at;
        let Some((block, end)) = block_of(text, start + "#[tool(".len()) else { break };
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = field(&block, "id")
        {
            found.push(Tool {
                id,
                file: file.to_path_buf(),
                line: text[..start].lines().count().max(1),
                crate_name: crate_name.to_string(),
            });
        }
        offset = end;
    }
    found
}

/// The **panels** a file declares, where the attribute is at the start of
/// its line.
///
/// A panel is a battery's own window of commands, and the attribute is at
/// the start of its line when it is a **declaration**: an attribute in a
/// comment is an example of one, and the example is not a panel.
pub fn panels_in(text: &str, file: &Path, crate_name: &str) -> Vec<Panel> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[panel(") {
        let start = offset + at;
        let Some((block, end)) = block_of(text, start + "#[panel(".len()) else { break };
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = field(&block, "id")
        {
            found.push(Panel {
                id,
                file: file.to_path_buf(),
                line: text[..start].lines().count().max(1),
                crate_name: crate_name.to_string(),
            });
        }
        offset = end;
    }
    found
}

/// The **migrations** a file declares, where the attribute is at the
/// start of its line.
///
/// A migration is a step from one version of a file format to the next,
/// and it is a rule the kit publishes: it says what a file of the older
/// version becomes. The attribute is at the start of its line when it is
/// a **declaration**: an attribute in a comment is an example of one, and
/// the example is not a step.
pub fn migrations_in(text: &str, file: &Path, crate_name: &str) -> Vec<Migration> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("#[migration(") {
        let start = offset + at;
        let Some((block, end)) = block_of(text, start + "#[migration(".len()) else { break };
        let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
        if text[line_start..start].trim().is_empty()
            && let Some(id) = field(&block, "id")
        {
            found.push(Migration {
                id,
                file: file.to_path_buf(),
                line: text[..start].lines().count().max(1),
                crate_name: crate_name.to_string(),
            });
        }
        offset = end;
    }
    found
}

/// The kinds a file declares, where the attribute is at the start of its
/// line.
///
/// A **node kind** says `kind =` and an **evaluator** says `id =`, and
/// both are read as text. The attribute is at the start of its line when
/// it is a **declaration**: an attribute in a comment is an example of
/// one, and the example is not a kind.
pub fn kinds_in(text: &str, file: &Path, battery: &str) -> Vec<Kind> {
    let mut found = Vec::new();
    for (attribute, name) in [("#[node_kind(", "kind"), ("#[evaluator(", "id")] {
        let mut offset = 0usize;
        while let Some(at) = text[offset..].find(attribute) {
            let start = offset + at;
            let Some((block, end)) = block_of(text, start + attribute.len()) else { break };
            let line_start = text[..start].rfind('\n').map(|at| at + 1).unwrap_or(0);
            if text[line_start..start].trim().is_empty()
                && let Some(id) = field(&block, name)
            {
                found.push(Kind {
                    id,
                    file: file.to_path_buf(),
                    line: text[..start].lines().count().max(1),
                    battery: battery.to_string(),
                });
            }
            offset = end;
        }
    }
    found
}

/// The kinds the macros of a battery declare, in one file.
///
/// **The call is read as the pattern says.** The macro's pattern says
/// which argument is the kind's id, and the call says what it is.
pub fn macro_kinds_in(text: &str, file: &Path, battery: &str, macros: &[MacroDef]) -> Vec<Kind> {
    let mut found = Vec::new();
    for definition in macros {
        for position in [definition.node_kind, definition.evaluator].into_iter().flatten() {
            let needle = format!("{}!(", definition.name);
            let mut offset = 0usize;
            while let Some(at) = text[offset..].find(&needle) {
                let start = offset + at;
                let args_start = start + needle.len();
                let Some((body, end)) = parens_of(text, args_start) else { break };
                let arguments = split_arguments(&body);
                if let Some(id) = literal_at(&arguments, Some(position)) {
                    let at = arguments.get(position).map(|(_, offset)| args_start + offset).unwrap_or(start);
                    found.push(Kind {
                        id,
                        file: file.to_path_buf(),
                        line: text[..at].lines().count().max(1),
                        battery: battery.to_string(),
                    });
                }
                offset = end;
            }
        }
    }
    found
}

/// A **macro** that declares a command.
///
/// A battery may declare a family of commands through a `macro_rules!`
/// macro: the macro holds the declaration and each call holds the
/// arguments. The attribute inside the macro says `id = $id` — and a
/// reader that reads only string literals sees no command at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroDef {
    /// The macro's name.
    pub name: String,
    /// The position of each argument of the pattern, by the variable's name.
    pub arguments: BTreeMap<String, usize>,
    /// The position of the argument the command's **id** comes from.
    pub id: Option<usize>,
    /// The position of the argument the command's **label** comes from.
    pub label: Option<usize>,
    /// The position of the argument the **node kind's** id comes from.
    ///
    /// A battery may declare a family of kinds through a macro, and the
    /// macro's body says `kind = $kind`: the call says what the kind is.
    pub node_kind: Option<usize>,
    /// The position of the argument the **evaluator's** id comes from.
    pub evaluator: Option<usize>,
    /// The documentation of the command the macro declares.
    pub doc: Option<String>,
    /// Whether the command the macro declares is the battery's example.
    pub example: bool,
    /// The file the macro is written in.
    pub file: PathBuf,
    /// The line the macro is written on.
    pub line: usize,
}

/// The macros of a file that declare a command.
///
/// **The pattern is read, and so is the body.** The pattern says which
/// argument is which — `$id:literal` is the second argument — and the
/// body says which argument the command's id comes from: `id = $id`.
pub fn macro_definitions_in(text: &str, file: &Path) -> Vec<MacroDef> {
    let mut found = Vec::new();
    let mut offset = 0usize;
    while let Some(at) = text[offset..].find("macro_rules!") {
        let start = offset + at;
        let after = start + "macro_rules!".len();
        let Some((name, after_name)) = name_after(text, after) else { break };
        let Some(open) = text[after_name..].find('{') else { break };
        let body_start = after_name + open + 1;
        let Some((body, end)) = braces_of(text, body_start) else { break };
        if let Some(definition) = macro_definition_of(&name, &body, text, body_start, file) {
            found.push(definition);
        }
        offset = end;
    }
    found
}

/// The macro of a name, if it declares a command.
fn macro_definition_of(
    name: &str,
    body: &str,
    text: &str,
    body_start: usize,
    file: &Path,
) -> Option<MacroDef> {
    // The pattern is the first parenthesised run of the body.
    let open = body.find('(')?;
    let (pattern, _after_pattern) = parens_of(body, open + 1)?;
    // The arguments of the pattern, by the variable's name.
    let mut arguments = BTreeMap::new();
    for (position, (fragment, _)) in split_arguments(&pattern).iter().enumerate() {
        let fragment = fragment.trim();
        let Some(rest) = fragment.strip_prefix('$') else { continue };
        let variable: String = rest.chars().take_while(|c| *c != ':').collect();
        if variable.is_empty() {
            continue;
        }
        arguments.insert(variable, position);
    }
    // The command the macro declares: its attribute and its documentation.
    //
    // **A macro may declare a kind and no command.** A battery writes a
    // family of kinds through a macro — `art_kind!` says `kind = $kind` —
    // and such a macro has no `#[command(` in it at all. A reader that
    // stopped at the command would see no kinds there, and a kind the
    // reader cannot see is a kind the gate cannot hold to account.
    let command_at = body.find("#[command(");
    let block = match command_at {
        Some(at) => block_of(body, at + "#[command(".len()).map(|(block, _)| block),
        None => None,
    };
    let id = block.as_deref().and_then(|block| metavariable_of(block, "id"));
    let label = block.as_deref().and_then(|block| metavariable_of(block, "label"));
    let id = id.and_then(|variable| arguments.get(&variable).copied());
    let label = label.and_then(|variable| arguments.get(&variable).copied());
    // A macro may declare a **kind** as well: a node kind says `kind =`,
    // and an evaluator says `id =` in an attribute of its own.
    let node_kind = kind_position(body, "#[node_kind(", "kind", &arguments);
    let evaluator = kind_position(body, "#[evaluator(", "id", &arguments);
    // A macro that declares neither a command nor a kind is not one of
    // the tree's rules, and it is not read.
    if block.is_none() && node_kind.is_none() && evaluator.is_none() {
        return None;
    }
    // The documentation is above the derive, inside the macro's body.
    let derive_at = body
        .find("#[derive(Command)]")
        .or_else(|| body.find("#[derive(NodeKind)]"))
        .or(command_at)
        .unwrap_or(0);
    let doc = doc_above(body, derive_at);
    let line = text[..body_start].lines().count().max(1);
    Some(MacroDef {
        name: name.to_string(),
        arguments,
        id,
        label,
        node_kind,
        evaluator,
        doc,
        example: block.as_deref().is_some_and(|block| has_flag(block, "example")),
        file: file.to_path_buf(),
        line,
    })
}

/// The position of the argument a **kind's** id comes from.
///
/// The attribute is read as text, and the id is a metavariable of it:
/// `#[node_kind(kind = $kind, …)]` takes the kind from the argument the
/// pattern names `$kind`.
fn kind_position(
    body: &str,
    attribute: &str,
    field: &str,
    arguments: &BTreeMap<String, usize>,
) -> Option<usize> {
    let at = body.find(attribute)?;
    let (block, _) = block_of(body, at + attribute.len())?;
    let variable = metavariable_of(&block, field)?;
    arguments.get(&variable).copied()
}

/// The commands the macros of a battery declare, in one file.
///
/// **The call is read as the pattern says.** The macro's pattern says
/// which argument is the id, and the call says what the id is.
pub fn macro_commands_in(text: &str, file: &Path, battery: &str, macros: &[MacroDef]) -> Vec<Command> {
    let mut found = Vec::new();
    for definition in macros {
        let needle = format!("{}!(", definition.name);
        let mut offset = 0usize;
        while let Some(at) = text[offset..].find(&needle) {
            let start = offset + at;
            let args_start = start + needle.len();
            let Some((body, end)) = parens_of(text, args_start) else { break };
            let arguments = split_arguments(&body);
            if let Some(id) = literal_at(&arguments, definition.id) {
                let label = literal_at(&arguments, definition.label);
                // The id's own place: the call may write it on a line of
                // its own, and the line is where the command is named.
                let at = definition
                    .id
                    .and_then(|position| arguments.get(position))
                    .map(|(_, offset)| args_start + offset)
                    .unwrap_or(start);
                let line = text[..at].lines().count().max(1);
                found.push(Command {
                    id,
                    label,
                    doc: definition.doc.clone(),
                    file: file.to_path_buf(),
                    line,
                    battery: battery.to_string(),
                    example: definition.example,
                });
            }
            offset = end;
        }
    }
    found
}

/// A name after a place in the text: the characters up to the first that
/// is not a name.
///
/// **The space is skipped.** `macro_rules! name { … }` writes a space
/// between the word and the name, and a reader that stopped at the space
/// would read no name at all.
fn name_after(text: &str, at: usize) -> Option<(String, usize)> {
    let rest = &text[at..];
    let trimmed = rest.trim_start();
    let skipped = rest.len() - trimmed.len();
    let end = trimmed.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(trimmed.len());
    if end == 0 {
        return None;
    }
    Some((trimmed[..end].to_string(), at + skipped + end))
}

/// A metavariable of a field: `id = $id` is the variable `id`.
fn metavariable_of(block: &str, name: &str) -> Option<String> {
    let needle = format!("{name} = $");
    let at = block.find(&needle)?;
    let rest = &block[at + needle.len()..];
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    Some(rest[..end].to_string())
}

/// The arguments of a call or a pattern, and where each one is.
///
/// **The commas that are inside a bracket do not count.** A pattern may
/// hold a type with a comma in it, and a reader that split on every
/// comma would count one argument as two.
pub fn split_arguments(text: &str) -> Vec<(String, usize)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut at = 0usize;
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
        } else if c == '\'' {
            if let Some(step) = the_character(bytes, at) {
                at += step;
                continue;
            }
        } else if c == '(' || c == '[' || c == '{' {
            depth += 1;
        } else if c == ')' || c == ']' || c == '}' {
            depth -= 1;
        } else if c == ',' && depth == 0 {
            found.push((text[start..at].trim().to_string(), start));
            start = at + 1;
        }
        at += 1;
    }
    found.push((text[start..].trim().to_string(), start));
    found
}

/// The literal at a position of a list of arguments.
fn literal_at(arguments: &[(String, usize)], position: Option<usize>) -> Option<String> {
    let position = position?;
    let (argument, _) = arguments.get(position)?;
    let trimmed = argument.trim();
    if !trimmed.starts_with('"') || !trimmed.ends_with('"') || trimmed.len() < 2 {
        return None;
    }
    Some(trimmed[1..trimmed.len() - 1].to_string())
}

/// The body of a run of parentheses, and where it ends.
fn parens_of(text: &str, body_start: usize) -> Option<(String, usize)> {
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

/// The **block** of an attribute, and where it ends.
///
/// **The parentheses are counted, and the strings are skipped.** An
/// attribute may hold a path — `menu("Edit", "Trim")` — and the `)` of the
/// path is not the end of the attribute. A reader that stopped at the first
/// `)` would read half an attribute and call it the whole.
pub fn block_of(text: &str, body_start: usize) -> Option<(String, usize)> {
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
pub fn field(block: &str, name: &str) -> Option<String> {
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
