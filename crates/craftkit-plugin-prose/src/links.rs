//! The **links** of a block.
//!
//! A run may carry a **link**: the target it points at. The words the
//! run covers are **derived**, and so is whether the link **may be
//! opened**.
//!
//! The tree's rule is the tree's own: a link that the system may
//! **follow** is a **web link** or a **mail link** — `http`, `https`
//! and `mailto` — and nothing else. A script, a `data:` URL, a
//! `file:` path, a link to **another machine** — a protocol-relative
//! `//host` or a UNC `\\host` — and a relative link are all links that
//! the system does not follow, and each is refused for its own
//! reason.
//!
//! And the rule is **exact**: a leading space or a control character
//! makes the target not a link at all, so ` https://example.com` is
//! not a web link and neither is a URL with a NUL byte in it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The most characters a target may have.
pub const MAX_TARGET: usize = 2048;

/// The **schemes** a link that may be opened has.
pub const OPENABLE: [&str; 3] = ["http", "https", "mailto"];

/// Whether a target **may be opened**.
///
/// The tree's rule, exactly: the scheme is what is before the first
/// colon, and it is one of the three.
///
/// And a target that names a **place in this document** — `#the-name` —
/// may be opened too, if the document has a bookmark with that name:
/// the tree's rule is about a link that leaves the document, and a
/// link that stays inside it is not leaving.
pub fn may_be_opened(document: &craftkit_sdk::Document, target: &str) -> bool {
    if let Some(name) = target.strip_prefix('#') {
        return bookmark_exists(document, name);
    }
    let Some((scheme, _)) = target.split_once(':') else { return false };
    OPENABLE.iter().any(|openable| scheme.eq_ignore_ascii_case(openable))
}

/// Whether the document has a **bookmark** with that name.
fn bookmark_exists(document: &craftkit_sdk::Document, name: &str) -> bool {
    document
        .walk()
        .filter(|node| node.kind == "prose.document")
        .flat_map(|node| node.children.iter())
        .filter(|child| child.kind.starts_with("prose.block."))
        .any(|child| crate::bookmarks::bookmark_of(child).as_deref() == Some(name))
}

/// Why a target may **not** be opened, if it may not.
///
/// Each kind of target is refused for its own reason, because a
/// refusal that does not say why is a refusal the caller cannot act
/// on.
pub fn why_not(document: &craftkit_sdk::Document, target: &str) -> Option<String> {
    if may_be_opened(document, target) {
        return None;
    }
    if let Some(name) = target.strip_prefix('#') {
        return Some(format!("there is no bookmark `{name}` in the document"));
    }
    let scheme = target.split_once(':').map(|(scheme, _)| scheme.to_ascii_lowercase());
    match scheme.as_deref() {
        Some("javascript") | Some("vbscript") => {
            Some("a script is not a link that may be opened".to_string())
        }
        Some("data") => Some("a data link is not a link that may be opened".to_string()),
        Some("file") => Some("a file is not a link that may be opened".to_string()),
        _ => {
            // A link to **another machine**: a protocol-relative
            // reference or a UNC path. Following it would send the
            // user's own credentials.
            let cleaned: String = target.chars().filter(|c| !c.is_ascii_control() && *c != ' ').collect();
            if cleaned.replace('\\', "/").starts_with("//") {
                Some("a link to another machine is not a link that may be opened".to_string())
            } else if scheme.is_none() {
                Some("a relative link is not a link that may be opened".to_string())
            } else {
                Some("a link that is not a web link or a mail link may not be opened".to_string())
            }
        }
    }
}

commands! {
    /// The **links** of a block.
    ///
    /// The answer is every run that carries a link, with the words it
    /// covers, the target it points at, and whether the target **may be
    /// opened**.
    prose.block.links {
        area: "prose",
        layer: "prose",
        icon: "link",
        params: {
            /// The block.
            block: NodeRef,
        },
        run: {
            let id = "prose.block.links".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed { id, message: format!("there is no block `{block}`") });
            };
            if !node.kind.starts_with("prose.block.") {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{block}` is a `{}` and not a block", node.kind),
                });
            }
            let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
            let characters: Vec<char> = text.chars().collect();
            let document = session.document();
            let mut links: Vec<Json> = Vec::new();
            let mut at = 0usize;
            let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
            for (index, run) in runs.iter().enumerate() {
                let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
                let end = at.saturating_add(length).min(characters.len());
                let covered: String = characters[at.min(characters.len())..end].iter().collect();
                at = end;
                let Some(target) = run.get("link").and_then(|target| target.as_str()) else { continue };
                let reason = why_not(document, target);
                links.push(json!({
                    "run": index,
                    "len": length,
                    "text": covered,
                    "target": target,
                    "may_be_opened": reason.is_none(),
                    "reason": reason,
                }));
            }
            Ok(Outcome::new(format!("the block `{block}` has {} link(s)", links.len())).with_data(json!({
                "block": block,
                "links": links,
            })))

        }
    }
}
