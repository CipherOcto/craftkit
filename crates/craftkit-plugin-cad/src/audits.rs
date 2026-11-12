//! The **audit** of a drawing: what is wrong with it.
//!
//! A drawing is a document, and a document can be wrong: two layers
//! with the same name, an entity on a layer that is locked, an insert
//! that refers to a block of another drawing, an insert that refers to
//! the block it is in.
//!
//! The audit **reads** the drawing and **reports** what it finds. It
//! changes nothing: a report is a fact about the drawing, and a fact
//! that is stored is a fact that can go stale.
//!
//! # What the port found
//!
//! - **The tree's audit repairs as well as reports**, and it repairs
//!   by changing the drawing — a missing table row is recreated, a
//!   bad handle is given a new one, a broken reference is removed.
//!   Here the audit **reports and does not repair**: repairing is a
//!   change, and a change is a command that changes, and there is one
//!   command per thing.
//! - **The tree's audit is about a file**: it reads a drawing from
//!   disk and checks the tables it holds. Here a drawing is in the
//!   document, and the audit looks at the document — which is the
//!   same idea with the file taken out of it.
//! - **The tree's audit finds a block that refers to itself** and
//!   removes the reference. Here it is **reported**: the drawing is
//!   left as it is, and a person says what should happen to it.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

/// One **thing that is wrong** with a drawing.
struct Issue {
    /// What kind of thing it is.
    kind: &'static str,
    /// What is wrong, in words.
    message: String,
}

impl Issue {
    /// The issue, as it is reported.
    fn to_json(&self) -> Json {
        json!({ "kind": self.kind, "message": self.message })
    }
}

/// The **layer** a node is, if it is one.
fn is_layer(node: &Node) -> bool {
    node.kind == "cad.layer"
}

/// The **block** a node is, if it is one.
fn is_block(node: &Node) -> bool {
    node.kind == "cad.block"
}

/// The **entities** of a block.
fn entities_of(block: &Node) -> Vec<&Node> {
    block
        .children
        .iter()
        .map(|child| &**child)
        .filter(|child| child.kind.starts_with("cad.entity."))
        .collect()
}

/// The **issues** of one drawing.
fn issues_of(document: &Document, drawing: &Node) -> Vec<Issue> {
    let mut issues = Vec::new();
    let layers: Vec<&Node> =
        drawing.children.iter().map(|child| &**child).filter(|child| is_layer(child)).collect();
    // No two layers in a drawing share a name: a layer is found by its
    // name, and two layers with one name are two answers to one
    // question.
    for (index, layer) in layers.iter().enumerate() {
        for other in layers.iter().skip(index + 1) {
            if layer.name == other.name {
                issues.push(Issue {
                    kind: "duplicateLayerName",
                    message: format!(
                        "the layers `{}` and `{}` are both called `{}`",
                        layer.id, other.id, layer.name
                    ),
                });
            }
        }
    }
    let blocks: Vec<&Node> =
        drawing.children.iter().map(|child| &**child).filter(|child| is_block(child)).collect();
    for block in blocks {
        for entity in entities_of(block) {
            // The layer an entity is on is an **edge**, and the edge
            // says what it is: it points at a layer.
            for relation in document.relations_from(entity.id, Some(craftkit_sdk::layer::ON_LAYER)) {
                let Some(layer) = document.node(relation.to) else { continue };
                if !is_layer(layer) {
                    continue;
                }
                if crate::editable::is_locked(layer) {
                    issues.push(Issue {
                        kind: "onALockedLayer",
                        message: format!(
                            "the entity `{}` is on the locked layer `{}`",
                            entity.id, layer.name
                        ),
                    });
                }
                if crate::editable::is_frozen(layer) {
                    issues.push(Issue {
                        kind: "onAFrozenLayer",
                        message: format!(
                            "the entity `{}` is on the frozen layer `{}`",
                            entity.id, layer.name
                        ),
                    });
                }
            }
            // The block an insert refers to is an **edge** too, and the
            // block it refers to has a drawing of its own.
            for relation in document.relations_from(entity.id, Some("cad.inserts")) {
                let Some(block_node) = document.node(relation.to) else { continue };
                if !is_block(block_node) {
                    continue;
                }
                if block_node.id == block.id {
                    issues.push(Issue {
                        kind: "insertRefersToItsOwnBlock",
                        message: format!(
                            "the insert `{}` refers to the block `{}` it is in",
                            entity.id, block.id
                        ),
                    });
                }
                if !belongs_to(block_node, drawing) {
                    issues.push(Issue {
                        kind: "insertFromAnotherDrawing",
                        message: format!(
                            "the insert `{}` refers to the block `{}`, which is in another drawing",
                            entity.id, block_node.id
                        ),
                    });
                }
            }
        }
    }
    issues
}

/// Whether a block is **in** a drawing.
fn belongs_to(block: &Node, drawing: &Node) -> bool {
    drawing.children.iter().any(|child| child.id == block.id)
}

commands! {
    /// **Audit** a drawing.
    cad.audit {
        area: "cad",
        layer: "cad",
        icon: "list",
        params: {
            /// The drawing.

            drawing: NodeRef,
        },
        run: {

            let id = "cad.audit".to_string();
            let drawing = self.drawing.0;
            let Some(node) = session.document().node(drawing) else {
                return Err(EngineError::Failed { id, message: format!("there is no drawing `{drawing}`") });
            };
            if node.kind != "cad.drawing" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{drawing}` is a `{}` and not a drawing", node.kind),
                });
            }
            let issues = issues_of(session.document(), node);
            let found = issues.len();
            let message = if found == 0 {
                format!("the drawing `{drawing}` has nothing wrong with it")
            } else {
                format!("the drawing `{drawing}` has {found} thing(s) wrong with it")
            };
            Ok(Outcome::new(message).with_data(json!({
                "drawing": drawing,
                "count": found,
                "issues": issues.iter().map(Issue::to_json).collect::<Vec<Json>>(),
            })))

        }
    }
}
