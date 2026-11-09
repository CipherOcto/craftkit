//! The **compare** of two documents: the differences between their
//! text.
//!
//! The tree's own compare: the two documents' words are diffed with
//! Myers' algorithm — after the common prefix and suffix are set
//! aside — and the edit script is grouped into **changes**: inserted,
//! deleted or replaced runs of words.
//!
//! # The rules
//!
//! - The **unit** of the comparison is the **piece** of text that the
//!   file shows: the text of one text-showing operation, with the box
//!   it takes. The tree's unit is the **word**, and the box of a word
//!   inside a piece would need the widths of the font — and a width
//!   that is guessed is a width that is wrong.
//! - A change is **replaced** when both sides have text, **deleted**
//!   when only the old side has it, and **inserted** when only the new
//!   side has it.
//! - The **boxes** of the words of a change are merged into one box
//!   for each line, the tree's own rule.
//! - A document whose text is the same as the other's has **no
//!   changes**, and that is said.
//!
//! # What it does not settle
//!
//! - The tree's **visual** compare is not here: it needs the two
//!   documents **rendered**, and this tree does not render a PDF.
//! - The tree's compare works on two **open** documents; here it works
//!   on two **files**, and the document of the session is not touched.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.compare";

/// A **piece** of the text of a document: what an operator showed, and
/// where it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// The text itself.
    pub text: String,
    /// The page it is on, counted from zero.
    pub page: usize,
    /// The box it takes: the left, the bottom, the right and the top.
    pub rect: [f64; 4],
}

/// The **kind** of a change: the tree's own names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The new side has text the old side does not have.
    Inserted,
    /// The old side has text the new side does not have.
    Deleted,
    /// Both sides have text, and it is not the same text.
    Replaced,
}

impl Kind {
    /// The **name** of the kind: the tree's own.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Inserted => "Inserted",
            Kind::Deleted => "Deleted",
            Kind::Replaced => "Replaced",
        }
    }

    /// The **name** of the kind, in lower case: what a file says.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Inserted => "inserted",
            Kind::Deleted => "deleted",
            Kind::Replaced => "replaced",
        }
    }
}

/// One **side** of a change: its text, its page and its boxes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Side {
    /// The text of the side.
    pub text: String,
    /// The page the change is on.
    pub page: usize,
    /// The boxes of the text: one for each line.
    pub rects: Vec<[f64; 4]>,
    /// The box of the word **next to** where the text would be, for a
    /// side with no text of its own.
    pub near: Option<[f64; 4]>,
}

/// One **change** between the two documents.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    /// What kind of change it is.
    pub kind: Kind,
    /// The old side of it.
    pub old: Side,
    /// The new side of it.
    pub new: Side,
}

/// What the comparison found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    /// The changes, in the order they were found.
    pub changes: Vec<Change>,
    /// The pieces of the old document.
    pub old_pieces: usize,
    /// The pieces of the new document.
    pub new_pieces: usize,
}

impl Comparison {
    /// How many changes of a kind there are.
    pub fn count(&self, kind: Kind) -> usize {
        self.changes.iter().filter(|change| change.kind == kind).count()
    }

    /// Whether the two documents' text is the same.
    pub fn identical(&self) -> bool {
        self.changes.is_empty()
    }
}

/// One **step** of an edit script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Op {
    /// The text is the same on both sides.
    Equal,
    /// The text is only on the old side.
    Delete,
    /// The text is only on the new side.
    Insert,
}

/// Segments longer than this (old + new pieces) are first split at
/// anchors, so Myers' saved frontiers stay small.
const DIRECT: usize = 3000;

/// The edit script for `a` to `b`.
fn diff(a: &[u64], b: &[u64], out: &mut Vec<Op>, depth: usize) {
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    out.extend(std::iter::repeat_n(Op::Equal, prefix));
    let (ma, mb) = (&a[prefix..a.len() - suffix], &b[prefix..b.len() - suffix]);
    let replace = |out: &mut Vec<Op>| {
        out.extend(std::iter::repeat_n(Op::Delete, ma.len()));
        out.extend(std::iter::repeat_n(Op::Insert, mb.len()));
    };
    if ma.len() + mb.len() <= DIRECT {
        out.extend(myers(ma, mb));
    } else {
        let anchors = if depth > 8 { Vec::new() } else { anchors(ma, mb) };
        if anchors.is_empty() {
            // A long stretch with nothing in common: one replacement.
            replace(out);
        } else {
            let (mut i, mut j) = (0, 0);
            for (x, y) in anchors {
                diff(&ma[i..x], &mb[j..y], out, depth + 1);
                out.push(Op::Equal);
                i = x + 1;
                j = y + 1;
            }
            diff(&ma[i..], &mb[j..], out, depth + 1);
        }
    }
    out.extend(std::iter::repeat_n(Op::Equal, suffix));
}

/// **Patience anchors**: the pieces that occur once in each side, in an
/// order both sides agree on — the longest increasing subsequence of
/// their positions.
fn anchors(a: &[u64], b: &[u64]) -> Vec<(usize, usize)> {
    let mut count: HashMap<u64, (usize, usize, usize, usize)> = HashMap::new();
    for (index, hash) in a.iter().enumerate() {
        let entry = count.entry(*hash).or_insert((0, 0, 0, 0));
        entry.0 += 1;
        entry.2 = index;
    }
    for (index, hash) in b.iter().enumerate() {
        if let Some(entry) = count.get_mut(hash) {
            entry.1 += 1;
            entry.3 = index;
        }
    }
    let mut pairs: Vec<(usize, usize)> =
        count.values().filter(|entry| entry.0 == 1 && entry.1 == 1).map(|entry| (entry.2, entry.3)).collect();
    pairs.sort();
    // The longest increasing subsequence of the positions, with back
    // links.
    let mut tails: Vec<usize> = Vec::new();
    let mut previous = vec![usize::MAX; pairs.len()];
    for (index, (_, second)) in pairs.iter().enumerate() {
        let place = tails.partition_point(|tail| pairs[*tail].1 < *second);
        if place > 0 {
            previous[index] = tails[place - 1];
        }
        if place == tails.len() {
            tails.push(index);
        } else {
            tails[place] = index;
        }
    }
    let mut out = Vec::new();
    let mut index = tails.last().copied().unwrap_or(usize::MAX);
    while index != usize::MAX {
        out.push(pairs[index]);
        index = previous[index];
    }
    out.reverse();
    out
}

/// Myers' diff of `a` and `b`: the edit script.
fn myers(a: &[u64], b: &[u64]) -> Vec<Op> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        let mut ops: Vec<Op> = std::iter::repeat_n(Op::Delete, n).collect();
        ops.extend(std::iter::repeat_n(Op::Insert, m));
        return ops;
    }
    let max = n + m;
    let offset = max as isize + 1;
    let mut v = vec![0usize; 2 * max + 3];
    // The frontier d spans the diagonals -d..=d: only that slice is
    // kept.
    let mut trace: Vec<Vec<usize>> = Vec::new();
    let mut found = false;
    'outer: for d in 0..=max as isize {
        trace.push(v[(offset - d - 1).max(0) as usize..=(offset + d + 1) as usize].to_vec());
        let mut k = -d;
        while k <= d {
            let i = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) { v[i + 1] } else { v[i - 1] + 1 };
            let mut y = (x as isize - k) as usize;
            while x < n && y < m && a[x] == b[y] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                found = true;
                break 'outer;
            }
            k += 2;
        }
    }
    if !found {
        // Too different: everything is replaced.
        let mut ops: Vec<Op> = std::iter::repeat_n(Op::Delete, n).collect();
        ops.extend(std::iter::repeat_n(Op::Insert, m));
        return ops;
    }
    // Walk back through the saved frontiers.
    let mut ops: Vec<Op> = Vec::new();
    let (mut x, mut y) = (n as isize, m as isize);
    for d in (0..trace.len() as isize).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let base = (offset - d - 1).max(0);
        let at = |k: isize| (k + offset - base) as usize;
        let previous_k = if k == -d || (k != d && v[at(k - 1)] < v[at(k + 1)]) { k + 1 } else { k - 1 };
        let previous_x = v[at(previous_k)] as isize;
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            ops.push(Op::Equal);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            ops.push(if x == previous_x { Op::Insert } else { Op::Delete });
        }
        x = previous_x;
        y = previous_y;
    }
    ops.reverse();
    ops
}

/// The **hash** of a text: what the diff compares.
fn hash(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// The **boxes** of the pieces of a change, merged into one box for
/// each line: the tree's own rule.
fn rects(pieces: &[&Piece]) -> Vec<[f64; 4]> {
    let mut out: Vec<[f64; 4]> = Vec::new();
    for piece in pieces {
        let rect = piece.rect;
        match out.last_mut() {
            Some(last)
                if (last[1] - rect[1]).abs() < (rect[3] - rect[1]).max(1.0) * 0.5
                    && rect[0] - last[2] < (rect[3] - rect[1]).max(4.0) * 2.0
                    && rect[0] >= last[0] - 1.0 =>
            {
                last[0] = last[0].min(rect[0]);
                last[1] = last[1].min(rect[1]);
                last[2] = last[2].max(rect[2]);
                last[3] = last[3].max(rect[3]);
            }
            _ => out.push(rect),
        }
    }
    out
}

/// One **side** of a change.
fn side(pieces: &[&Piece], neighbour: Option<&Piece>) -> Side {
    Side {
        text: pieces.iter().map(|piece| piece.text.as_str()).collect::<Vec<_>>().join(" "),
        page: pieces.first().or(neighbour.as_ref()).map_or(0, |piece| piece.page),
        rects: rects(pieces),
        near: if pieces.is_empty() { neighbour.map(|piece| piece.rect) } else { None },
    }
}

/// Compare the two documents' pieces of text.
pub fn compare(old: &[Piece], new: &[Piece]) -> Comparison {
    let a: Vec<u64> = old.iter().map(|piece| hash(&piece.text)).collect();
    let b: Vec<u64> = new.iter().map(|piece| hash(&piece.text)).collect();
    let mut ops: Vec<Op> = Vec::with_capacity(a.len().max(b.len()));
    diff(&a, &b, &mut ops, 0);

    let mut changes: Vec<Change> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let mut k = 0usize;
    while k < ops.len() {
        if ops[k] == Op::Equal {
            i += 1;
            j += 1;
            k += 1;
            continue;
        }
        let (mut deleted, mut inserted): (Vec<&Piece>, Vec<&Piece>) = (Vec::new(), Vec::new());
        while k < ops.len() && ops[k] != Op::Equal {
            match ops[k] {
                Op::Delete => {
                    if let Some(piece) = old.get(i) {
                        deleted.push(piece);
                    }
                    i += 1;
                }
                _ => {
                    if let Some(piece) = new.get(j) {
                        inserted.push(piece);
                    }
                    j += 1;
                }
            }
            k += 1;
        }
        let kind = match (deleted.is_empty(), inserted.is_empty()) {
            (false, false) => Kind::Replaced,
            (false, true) => Kind::Deleted,
            _ => Kind::Inserted,
        };
        // Where a change of one side sits in the other: next to the
        // neighbouring piece.
        let old_near = old.get(i).or_else(|| old.last());
        let new_near = new.get(j).or_else(|| new.last());
        changes.push(Change { kind, old: side(&deleted, old_near), new: side(&inserted, new_near) });
    }
    Comparison { changes, old_pieces: old.len(), new_pieces: new.len() }
}

/// The compare report's text: a summary, then each change with its
/// pages.
pub fn report(comparison: &Comparison, old_name: &str, new_name: &str) -> String {
    let mut out = format!(
        "Compare Report\n\nOld file: {old_name}\nNew file: {new_name}\n\n{} changes: {} replaced, {} inserted, {} deleted.\n",
        comparison.changes.len(),
        comparison.count(Kind::Replaced),
        comparison.count(Kind::Inserted),
        comparison.count(Kind::Deleted)
    );
    if comparison.identical() {
        out.push_str("\nThe documents' text is identical.\n");
    }
    for (number, change) in comparison.changes.iter().enumerate() {
        out.push_str(&format!(
            "\n{}. {} (old page {}, new page {})\n",
            number + 1,
            change.kind.label(),
            change.old.page + 1,
            change.new.page + 1
        ));
        if !change.old.text.is_empty() {
            out.push_str(&format!("   Old: {}\n", change.old.text));
        }
        if !change.new.text.is_empty() {
            out.push_str(&format!("   New: {}\n", change.new.text));
        }
    }
    out
}

/// The **pages** of the document.
fn pages_of(document: &Document) -> Vec<u64> {
    document
        .walk()
        .filter(|node| {
            (node.kind == "pdf.object" || node.kind == "pdf.stream")
                && crate::entries_of(&node.data)
                    .iter()
                    .any(|(key, value)| key == "Type" && value.as_str() == Some("Page"))
        })
        .map(|node| node.id)
        .collect()
}

/// The **pieces** of the text of a document, page by page, in the order
/// the pages come.
pub fn pieces_of(document: &Document) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for (index, page) in pages_of(document).into_iter().enumerate() {
        let Ok(text) = crate::pagetext::text_of_page(document, page) else { continue };
        for piece in text.pieces {
            if piece.text.trim().is_empty() {
                continue;
            }
            let rect = piece.bounds.unwrap_or([0.0, 0.0, 0.0, 0.0]);
            out.push(Piece { text: piece.text, page: index, rect });
        }
    }
    out
}

/// The **changes** of a comparison, as the answer gives them.
fn changes_json(comparison: &Comparison) -> Json {
    let changes: Vec<Json> = comparison
        .changes
        .iter()
        .map(|change| {
            json!({
                "kind": change.kind.name(),
                "old": {
                    "text": change.old.text,
                    "page": change.old.page,
                    "rects": change.old.rects,
                    "near": change.old.near.map(|rect| json!(rect)).unwrap_or(Json::Null),
                },
                "new": {
                    "text": change.new.text,
                    "page": change.new.page,
                    "rects": change.new.rects,
                    "near": change.new.near.map(|rect| json!(rect)).unwrap_or(Json::Null),
                },
            })
        })
        .collect();
    Json::Array(changes)
}

/// The **number** of the work area: every comparison has one of its
/// own.
fn work_number() -> usize {
    static NUMBER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    NUMBER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The **document** of a file: the objects of the file, and the bytes of
/// its streams in a directory of their own.
///
/// The file is read into a document of its own, so that the document of
/// the session is not touched by a comparison.
fn document_of(bytes: &[u8], into: &std::path::Path) -> Result<Document, EngineError> {
    let file = crate::loader::load(bytes).map_err(|error| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("the file cannot be loaded: {error:?}"),
    })?;
    std::fs::create_dir_all(into).map_err(|error| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("the streams' bytes have nowhere to go: {error}"),
    })?;
    let mut document = Document::new("a file");
    // The **first pass**: the objects. A reference may point forward,
    // so every object is in the document before any reference is made.
    let mut nodes: Vec<(u64, u64)> = Vec::with_capacity(file.objects.len());
    for object in &file.objects {
        let entries: Vec<Json> = object.entries.iter().map(|(key, value)| json!([key, value])).collect();
        let data = json!({
            "object": object.number,
            "generation": object.generation,
            "entries": entries,
        });
        let has_stream = object.stream.is_some();
        let name = if has_stream {
            format!("stream {}", object.number)
        } else {
            format!("object {}", object.number)
        };
        let kind = if has_stream { "pdf.stream" } else { "pdf.object" };
        let id = document.add_node_with(name, kind, None, data);
        if let Some(stream) = &object.stream {
            let path = into.join(format!("{}.bin", object.number));
            std::fs::write(&path, stream).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the stream's bytes cannot be written: {error}"),
            })?;
            if let Some(node) = document.node_mut(id) {
                node.stream = Some(path.to_string_lossy().to_string());
            }
        }
        nodes.push((object.number, id));
    }
    // The **second pass**: the references.
    for object in &file.objects {
        let Some((_, from)) = nodes.iter().find(|(number, _)| *number == object.number) else {
            continue;
        };
        for reference in &object.references {
            let Some((_, to)) = nodes.iter().find(|(number, _)| *number == reference.number) else {
                continue;
            };
            let payload = json!({
                "key": reference.key,
                "generation": reference.generation,
            });
            let relation = craftkit_sdk::Relation::with_payload(*from, *to, "ref", payload);
            document.add_relation(relation);
        }
    }
    // The **trailer** of the file is the file's last word and not an
    // object of the file.
    if !file.trailer.is_empty() {
        let trailer: Vec<Json> = file.trailer.iter().map(|(key, value)| json!([key, value])).collect();
        document.data["pdf.trailer"] = json!({ "entries": trailer });
    }
    if !file.free.is_empty() {
        crate::free::set_the_free(&mut document, &file.free);
    }
    Ok(document)
}

/// The **pieces** of the text of a file.
fn pieces_of_file(path: &str, into: &std::path::Path) -> Result<Vec<Piece>, EngineError> {
    let limits = crate::filters::Limits::default();
    let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(path), limits.max_input as u64)
        .map_err(|message| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the file cannot be read: {message}"),
        })?;
    let document = document_of(&bytes, into)?;
    Ok(pieces_of(&document))
}

/// **Compare** two files of PDF: the differences between their text.
///
/// The tree's own compare: the two documents' text is diffed, and the
/// changes are said.
#[derive(Command)]
#[command(id = "pdf.compare", area = "pdf", layer = "pdf", icon = "git-compare")]
pub struct CompareFiles {
    /// The file that was there before.
    pub old: Text,
    /// The file that is there now.
    pub new: Text,
}

impl CompareFiles {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let old_path = self.old.0.trim().to_string();
        let new_path = self.new.0.trim().to_string();
        if old_path.is_empty() || new_path.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "two files are compared, and both of them have a path".to_string(),
            });
        }
        // The **work area**: the bytes of the streams of the two files
        // go into a directory of their own, and it is taken away when
        // the comparison is done. The directory is **of this
        // comparison**: two comparisons that run at the same time do
        // not share one.
        let work = std::env::temp_dir().join(format!(
            "craftkit-v2-compare-{}-{}",
            std::process::id(),
            work_number()
        ));
        let old_pieces = pieces_of_file(&old_path, &work.join("old"));
        let new_pieces = pieces_of_file(&new_path, &work.join("new"));
        let _ = std::fs::remove_dir_all(&work);
        let old_pieces = old_pieces?;
        let new_pieces = new_pieces?;
        let comparison = compare(&old_pieces, &new_pieces);
        let text = report(&comparison, &old_path, &new_path);
        Ok(Outcome::new(format!("{} change(s) between the two files", comparison.changes.len())).with_data(
            json!({
                "old": old_path,
                "new": new_path,
                "changes": changes_json(&comparison),
                "count": comparison.changes.len(),
                "replaced": comparison.count(Kind::Replaced),
                "inserted": comparison.count(Kind::Inserted),
                "deleted": comparison.count(Kind::Deleted),
                "old_pieces": comparison.old_pieces,
                "new_pieces": comparison.new_pieces,
                "identical": comparison.identical(),
                "report": text,
            }),
        ))
    }
}
