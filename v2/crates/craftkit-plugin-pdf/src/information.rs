//! The **information** of a document.
//!
//! A PDF file has a **trailer**, and the trailer may name an
//! **information dictionary**: the object that says who wrote the file,
//! what it is called and when it was made. It is the `/Info` of the
//! format.
//!
//! Here the information dictionary is an **ordinary object** — a node
//! of the kind `pdf.object` — and the document says which object it is
//! with a node of its own. That is the honest port: the format says
//! the information is an object, and a thing that is an object is an
//! object here too.
//!
//! # What the port found
//!
//! - **The tree's information is a table of its own**: the properties
//!   of a document — its title, its author, its subject, its keywords,
//!   its creator — are fields with names, and a document that has one
//!   of them says so. Here the information is a **dictionary of
//!   entries**, and the entries are the object's own: the tree's
//!   names are the PDF's names, and the PDF's names are open.
//! - **The tree's trailer writes the information under a name of its
//!   own** (`DOCPROPS` in its own file format). Here the trailer
//!   writes `/Info`, which is what a PDF file says.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

/// The **information** a document has: the node of the object that is
/// the information dictionary.
fn the_information(document: &Document) -> Option<&Node> {
    document.walk().find(|node| node.kind == "pdf.info")
}

/// Say which object is the file's **information**.
#[derive(Command)]
#[command(id = "pdf.info.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetInformation {
    /// The object that is the information dictionary.
    pub object: NodeRef,
}

impl SetInformation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.info.set".to_string();
        let object = self.object.0;
        let Some(node) = session.document().node(object) else {
            return Err(EngineError::Failed { id, message: format!("there is no node `{object}`") });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{object}` is a `{}` and not an object", node.kind),
            });
        }
        let number = node.data.get("object").and_then(|number| number.as_u64()).ok_or_else(|| {
            EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{object}` has no object number"),
            }
        })?;
        // There is **one** information dictionary in a file, and a
        // second one would be a second answer to one question.
        if let Some(already) = the_information(session.document()) {
            let already_object = already.data.get("object").and_then(|object| object.as_u64()).unwrap_or(0);
            if already_object != object {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the file already has the information of the node `{already_object}`"),
                });
            }
            return Ok(Outcome::changed(format!("the object `{number}` is the information of the file"))
                .with_data(json!({ "object": object, "number": number })));
        }
        let info = session.document_mut()?.add_node_with_under(
            None,
            "the information".to_string(),
            "pdf.info",
            None,
            json!({ "object": object }),
        );
        Ok(Outcome::changed(format!("the object `{number}` is the information of the file"))
            .with_data(json!({ "id": info, "object": object, "number": number })))
    }
}
