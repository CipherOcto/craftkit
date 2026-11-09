//! The **marks** of a page: the watermark, the header, the footer and
//! the background — and the taking of them away.
//!
//! The tree's own: every mark is a piece of page content marked with
//! its **kind**, and the tree takes a mark away by finding it.
//!
//! # The rules
//!
//! - The **kinds** are the tree's own three: the `headerfooter`, the
//!   `watermark` and the `background`; a kind that is not one of them
//!   is **refused**.
//! - A mark is the content between the `q` that opens it and the `EMC`
//!   that closes it, and taking it away **cuts that content out** of
//!   the page.
//! - Every mark of that kind on the page is taken away, and the page
//!   with no mark of that kind is said.
//!
//! # What it does not settle
//!
//! - The marks of a page are not **listed**: the command says how many
//!   it took away and not where they were.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.marks.remove";

/// The **kinds** of mark: the tree's own three.
pub const KINDS: [&str; 3] = ["headerfooter", "watermark", "background"];

/// The **mark** of a kind: the name the tree writes in it.
fn mark_of(kind: &str) -> Option<&'static str> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "headerfooter" | "header" | "footer" => Some("HeaderFooter"),
        "watermark" => Some("Watermark"),
        "background" => Some("Background"),
        _ => None,
    }
}

/// The **streams** of the page's content: the nodes of them, in the
/// order the content is in.
fn content_streams(document: &craftkit_sdk::Document, page: &craftkit_sdk::Node) -> Vec<u64> {
    let entries = crate::entries_of(&page.data);
    let Some((_, value)) = entries.into_iter().find(|(key, _)| key == "Contents") else {
        return Vec::new();
    };
    let mut out: Vec<u64> = Vec::new();
    let mut push = |value: Json| {
        let Some(number) = value
            .get("ref")
            .and_then(|reference| reference.get(0))
            .and_then(Json::as_f64)
            .filter(|number| number.is_finite() && number.fract() == 0.0 && *number >= 0.0)
            .map(|number| number as u64)
        else {
            return;
        };
        if let Some(node) = document
            .walk()
            .find(|node| crate::the_number_of(node) == Some(number) && node.kind == "pdf.stream")
        {
            out.push(node.id);
        }
    };
    match value {
        Json::Array(list) => {
            for item in list {
                push(item.clone());
            }
        }
        other => push(other.clone()),
    }
    out
}

/// The **marks** of a kind in a content, and the content without them.
///
/// The tree's own rule: a mark is the content between the `q` that
/// opens it and the `EMC` that closes it.
fn without_marks(content: &str, mark: &str) -> (String, usize) {
    let mut out = String::new();
    let mut rest = content;
    let mut taken = 0usize;
    loop {
        let Some(at) = rest.find(mark) else {
            out.push_str(rest);
            break;
        };
        let Some(start) = rest[..at].rfind("q\n/Artifact") else {
            out.push_str(&rest[..at + mark.len()]);
            rest = &rest[at + mark.len()..];
            continue;
        };
        let after = &rest[at + mark.len()..];
        let Some(end) = after.find("EMC\nQ\n") else {
            out.push_str(&rest[..at + mark.len()]);
            rest = after;
            continue;
        };
        out.push_str(&rest[..start]);
        rest = &after[end + "EMC\nQ\n".len()..];
        taken += 1;
    }
    (out, taken)
}

/// **Take away** the marks of a page: the watermark, the header, the
/// footer or the background.
///
/// The tree's own: the mark is found by the name the tree gives it and
/// the content of it is cut out of the page.
#[derive(Command)]
#[command(id = "pdf.page.marks.remove", area = "pdf", layer = "pdf", icon = "trash", mutates, undoable)]
pub struct RemoveMarks {
    /// The page the marks are taken away from.
    pub page: NodeRef,
    /// The kind of mark: `headerfooter`, `watermark` or `background`.
    pub kind: Text,
}

impl RemoveMarks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let Some(mark) = mark_of(&self.kind.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a kind of mark: the kinds are {}",
                    self.kind.0.trim(),
                    KINDS.join(", ")
                ),
            });
        };
        let Some(node) = session.document().node(self.page.0) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{}`", self.page.0),
            });
        };
        let streams = content_streams(session.document(), node);
        let mut taken = 0usize;
        for stream in streams {
            let stream_node = session.document().node(stream).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{stream}`"),
            })?;
            let Some(path) = stream_node.stream.clone() else {
                continue;
            };
            let limits = crate::filters::Limits::default();
            let bytes =
                craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
                    .map_err(|message| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: format!("the page's content cannot be read: {message}"),
                    })?;
            let content = String::from_utf8_lossy(&bytes).to_string();
            let (without, count) = without_marks(&content, &format!("/PCMark /{mark}"));
            if count == 0 {
                continue;
            }
            taken += count;
            std::fs::write(&path, without.as_bytes()).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page's content cannot be written: {error}"),
            })?;
        }
        Ok(Outcome::changed(format!("{taken} mark(s) taken away")).with_data(json!({
            "page": self.page.0,
            "kind": self.kind.0.trim().to_ascii_lowercase(),
            "removed": taken,
        })))
    }
}
