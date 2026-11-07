//! The **groups** of a drawing.
//!
//! A group is a **set** of the entities of a drawing: it says that
//! these entities belong together, and the drawing may be asked about
//! them as one. The group stands in the block of the drawing, and its
//! members are the **relations** it holds.
//!
//! # The rules
//!
//! - **A group has a name.** The tree's group may be unnamed and is
//!   then called `*A1`, `*A2` and so on; here a group is named, and a
//!   name that is not said is not a name.
//! - **A group has at least one member.** A group of nothing is not a
//!   group.
//! - **The same entity twice is one member.** A group is a set.
//! - **The members move together.** The group is a fact of the
//!   drawing and the moving of it is derived: every member of the
//!   group goes by the vector the group goes by.
//!
//! # What it does not do
//!
//! - **The group is not a block.** The tree's group may be edited as
//!   one — a member may be added or taken away — and here the members
//!   are the relations the group holds.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Relation, Session, Text};

/// The most members a group may hold.
const MOST_MEMBERS: usize = 1000;

/// The **relation** a member is: from the group to the entity.
const MEMBER: &str = "cad.member";

/// The **group** a node is, if it is one.
fn the_group<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no group `{node}`"),
        });
    };
    if found.kind != "cad.group" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a group", found.kind),
        });
    }
    Ok(found)
}

/// The **members** of a group: the entities it holds.
fn members_of(document: &Document, group: u64) -> Vec<u64> {
    document.relations_from(group, Some(MEMBER)).iter().map(|relation| relation.to).collect()
}

/// The **groups** of a drawing: the nodes of the kind `cad.group` that
/// stand in it.
fn groups_of(document: &Document, drawing: u64) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    gather(document, drawing, &mut out);
    out
}

/// The **groups** under a node.
fn gather(document: &Document, node: u64, out: &mut Vec<u64>) {
    let Some(node) = document.node(node) else { return };
    for child in node.children.iter() {
        if child.kind == "cad.group" {
            out.push(child.id);
        }
        gather(document, child.id, out);
    }
}

/// **Make** a **group** of entities.
///
/// The group stands in a block, and its members are the entities it
/// holds.
#[derive(Command)]
#[command(id = "cad.group.add", area = "cad", layer = "cad", icon = "group", mutates, undoable)]
pub struct AddGroup {
    /// What to call the group.
    pub name: Text,
    /// The block the group stands in.
    pub block: NodeRef,
    /// The entities that are members of the group.
    pub members: Json,
}

impl AddGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.group.add".to_string();
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a group has a name".to_string() });
        }
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if node.kind != "cad.block" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        // The **members**: the entities the group holds.
        let listed: Vec<u64> = match &self.members {
            Json::Array(list) => list.iter().filter_map(|member| member.as_u64()).collect(),
            one => one.as_u64().map(|member| vec![member]).unwrap_or_default(),
        };
        if listed.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a group has at least one member".to_string(),
            });
        }
        if listed.len() > MOST_MEMBERS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a group holds at most {MOST_MEMBERS} members"),
            });
        }
        let mut members: Vec<u64> = Vec::new();
        for member in listed {
            let Some(node) = session.document().node(member) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no entity `{member}`"),
                });
            };
            if !node.kind.starts_with("cad.entity.") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{member}` is a `{}` and not an entity", node.kind),
                });
            }
            // The same entity twice is one member.
            if !members.contains(&member) {
                members.push(member);
            }
        }
        let group = session.document_mut()?.add_node_with_under(
            Some(block),
            name.clone(),
            "cad.group",
            None,
            json!({}),
        );
        for member in &members {
            session.document_mut()?.add_relation(Relation::with_payload(group, *member, MEMBER, json!({})));
        }
        let count = members.len();
        Ok(Outcome::changed(format!("made the group `{name}`")).with_data(json!({
            "group": group,
            "name": name,
            "members": members,
            "count": count,
        })))
    }
}

/// **Read** the **groups** of a drawing.
///
/// The members of a group are the relations it holds, and they are
/// read from the drawing.
#[derive(Command)]
#[command(id = "cad.group.list", area = "cad", layer = "cad", icon = "list")]
pub struct ListGroups {
    /// The drawing.
    pub drawing: NodeRef,
}

impl ListGroups {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.group.list".to_string();
        let drawing = self.drawing.0;
        let Some(node) = session.document().node(drawing) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no drawing `{drawing}`"),
            });
        };
        if node.kind != "cad.drawing" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{drawing}` is a `{}` and not a drawing", node.kind),
            });
        }
        let mut listed: Vec<Json> = Vec::new();
        for group in groups_of(session.document(), drawing) {
            let Some(node) = session.document().node(group) else { continue };
            let members = members_of(session.document(), group);
            listed.push(json!({
                "group": group,
                "name": node.name,
                "members": members,
                "count": members.len(),
            }));
        }
        let count = listed.len();
        Ok(Outcome::new(format!("the drawing `{drawing}` holds {count} group(s)")).with_data(json!({
            "drawing": drawing,
            "groups": listed,
            "count": count,
        })))
    }
}

/// **Move** the **members** of a group.
///
/// The group is a fact of the drawing and the moving of it is derived:
/// every member of the group goes by the vector.
#[derive(Command)]
#[command(id = "cad.group.move", area = "cad", layer = "cad", icon = "move", mutates, undoable)]
pub struct MoveGroup {
    /// The group.
    pub group: NodeRef,
    /// The vector the members go by.
    pub by: Json,
}

impl MoveGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "cad.group.move".to_string();
        let group = self.group.0;
        the_group(session, &id, group)?;
        crate::editable::may_edit(session.document(), group, &id)?;
        let x = self.by.get("x").and_then(|x| x.as_f64()).unwrap_or(f64::NAN);
        let y = self.by.get("y").and_then(|y| y.as_f64()).unwrap_or(f64::NAN);
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the vector is a vector in the world's units".to_string(),
            });
        }
        let members = members_of(session.document(), group);
        let mut moved: Vec<u64> = Vec::new();
        for member in members {
            let Some(node) = session.document().node(member) else { continue };
            let kind = node.kind.clone();
            let Some(geometry) = node.data.get("geometry").cloned() else { continue };
            let Some(new) = crate::modify::moved(&kind, &geometry, x, y) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("an entity of the kind `{kind}` may not be moved"),
                });
            };
            crate::editable::may_edit(session.document(), member, &id)?;
            let Some(node) = session.document_mut()?.node_mut(member) else { continue };
            node.data["geometry"] = new;
            moved.push(member);
        }
        let count = moved.len();
        Ok(Outcome::changed(format!("moved the members of the group `{group}`")).with_data(json!({
            "group": group,
            "members": moved,
            "count": count,
            "by": { "x": x, "y": y },
        })))
    }
}
