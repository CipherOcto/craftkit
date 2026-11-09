//! The **links** of the stories of a layout document.
//!
//! A story may be **linked** to another: it takes the content of its
//! parent and stays linked to it, so that the content may be brought
//! up to date when the parent changes.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A story has a **revision**, and it moves when the text of the
//!   story is changed.
//! - A **link** is a pair: the story and its parent, and the revision
//!   of the parent at the time of the link was made.
//! - A link is **out of date** when the revision of the parent has
//!   moved on since the link was made.
//! - **Updating** a link takes the content of the parent and replaces
//!   the content of the child: the child's own edits are gone.
//! - **Unlinking** a story leaves its content as it is.
//! - A story may not be linked to **itself**, and a story that is not
//!   there is refused by name.
//!
//! # What it does not do
//!
//! - **The tree's link is made by placing**: a new frame is placed and
//!   its story copies the parent. Here a story is linked to another
//!   story, and the items that carry them are put on them as they are
//!   put on any story.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **revision** a story has when it is new.
const FIRST_REVISION: u64 = 1;

/// The **text** of a story.
fn text_of(story: &Node) -> String {
    story.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **revision** of a story.
fn revision_of(story: &Node) -> u64 {
    story.data.get("rev").and_then(|rev| rev.as_u64()).unwrap_or(FIRST_REVISION)
}

/// The **link** of a story: its parent and the revision of the parent
/// when the link was made.
fn link_of(story: &Node) -> Option<(u64, u64)> {
    let link = story.data.get("link")?;
    let parent = link.get("parent")?.as_u64()?;
    let rev = link.get("rev").and_then(|rev| rev.as_u64()).unwrap_or(FIRST_REVISION);
    Some((parent, rev))
}

/// The **story** a command is about.
fn the_story<'a>(session: &'a Session, command: &str, story: u64) -> Result<&'a Node, EngineError> {
    let Some(node) = session.document().node(story) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no story `{story}`"),
        });
    };
    if node.kind != "layout.story" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{story}` is a `{}` and not a story", node.kind),
        });
    }
    Ok(node)
}

/// **Set** the text of a story.
///
/// The revision of the story moves, and a link that copies this story
/// is out of date because of it.
#[derive(Command)]
#[command(id = "layout.story.set", area = "layout", layer = "layout", icon = "pencil", mutates, undoable)]
pub struct SetStoryText {
    /// The story.
    pub story: NodeRef,
    /// Its text.
    pub text: Text,
}

impl SetStoryText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.story.set";
        let story = self.story.0;
        the_story(session, command, story)?;
        let text = self.text.0.clone();
        let revision = revision_of(the_story(session, command, story)?).saturating_add(1);
        if let Some(node) = session.document_mut()?.node_mut(story) {
            node.data["text"] = json!(text);
            node.data["rev"] = json!(revision);
        }
        Ok(Outcome::changed(format!("the story `{story}` has new text")).with_data(json!({
            "story": story,
            "text": text,
            "rev": revision,
        })))
    }
}

/// **Link** a story to another story.
///
/// The story takes the content of its parent and stays linked to it.
#[derive(Command)]
#[command(id = "layout.story.link", area = "layout", layer = "layout", icon = "link", mutates, undoable)]
pub struct LinkStory {
    /// The story.
    pub story: NodeRef,
    /// The story it takes its content from.
    pub parent: NodeRef,
}

impl LinkStory {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.story.link";
        let story = self.story.0;
        let parent = self.parent.0;
        the_story(session, command, story)?;
        the_story(session, command, parent)?;
        if story == parent {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a story may not be linked to itself".to_string(),
            });
        }
        // The **content** of the parent, and the revision it is at.
        let text = text_of(the_story(session, command, parent)?);
        let revision = revision_of(the_story(session, command, parent)?);
        if let Some(node) = session.document_mut()?.node_mut(story) {
            node.data["text"] = json!(text);
            node.data["link"] = json!({ "parent": parent, "rev": revision });
        }
        Ok(Outcome::changed(format!("the story `{story}` is linked to the story `{parent}`")).with_data(
            json!({
                "story": story,
                "parent": parent,
                "rev": revision,
            }),
        ))
    }
}

/// **Unlink** a story.
///
/// The content of the story stays as it is.
#[derive(Command)]
#[command(id = "layout.story.unlink", area = "layout", layer = "layout", icon = "minus", mutates, undoable)]
pub struct UnlinkStory {
    /// The story.
    pub story: NodeRef,
}

impl UnlinkStory {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.story.unlink";
        let story = self.story.0;
        the_story(session, command, story)?;
        if let Some(node) = session.document_mut()?.node_mut(story) {
            node.data["link"] = Json::Null;
        }
        Ok(Outcome::changed(format!("the story `{story}` is not linked"))
            .with_data(json!({ "story": story })))
    }
}

/// **Update** the links of the stories: a story takes the content of
/// its parent again, and its own edits are replaced.
///
/// Every link that is out of date is updated, and only the one that is
/// named when a story is named.
#[derive(Command)]
#[command(id = "layout.story.update", area = "layout", layer = "layout", icon = "arrow", mutates, undoable)]
pub struct UpdateLinks {
    /// The story, when it is not every link that is out of date.
    pub story: Option<NodeRef>,
}

impl UpdateLinks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.story.update";
        // The **stories** of the document that are linked.
        let document =
            session.document().walk().find(|node| node.kind == "layout.document").map(|node| node.id);
        let Some(document) = document else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "there is no document".to_string(),
            });
        };
        let stories: Vec<u64> = session
            .document()
            .node(document)
            .map(|node| {
                node.children
                    .iter()
                    .filter(|child| child.kind == "layout.story")
                    .map(|child| child.id)
                    .collect()
            })
            .unwrap_or_default();
        let mut updated = 0usize;
        for story in stories {
            if let Some(only) = self.story
                && only.0 != story
            {
                continue;
            }
            let Some(node) = session.document().node(story).cloned() else { continue };
            let Some((parent, rev)) = link_of(&node) else { continue };
            let Some(parent_node) = session.document().node(parent).cloned() else { continue };
            // A link that is **named** is updated whatever its state,
            // and a link that is not named is updated only when it is
            // out of date.
            let out_of_date = revision_of(&parent_node) != rev;
            if self.story.is_none() && !out_of_date {
                continue;
            }
            let text = text_of(&parent_node);
            let revision = revision_of(&parent_node);
            if let Some(node) = session.document_mut()?.node_mut(story) {
                node.data["text"] = json!(text);
                node.data["link"] = json!({ "parent": parent, "rev": revision });
            }
            updated += 1;
        }
        Ok(Outcome::changed(format!("updated {updated} link(s)")).with_data(json!({ "updated": updated })))
    }
}

/// The **links** of the stories of a document, and which of them are
/// out of date.
#[derive(Command)]
#[command(id = "layout.document.links", area = "layout", layer = "layout", icon = "list")]
pub struct ReadLinks {
    /// The document.
    pub document: NodeRef,
}

impl ReadLinks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.document.links";
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("there is no document `{document}`"),
            });
        };
        let mut links = Vec::new();
        for child in node.children.iter().filter(|child| child.kind == "layout.story") {
            let Some((parent, rev)) = link_of(child) else { continue };
            // The link is **out of date** when the parent has moved on.
            let out_of_date =
                session.document().node(parent).map(|parent| revision_of(parent) != rev).unwrap_or(true);
            links.push(json!({
                "story": child.id,
                "parent": parent,
                "rev": rev,
                "out_of_date": out_of_date,
            }));
        }
        Ok(Outcome::new(format!("the document `{document}` holds {} link(s)", links.len())).with_data(
            json!({
                "document": document,
                "links": links,
                "count": links.len(),
            }),
        ))
    }
}
