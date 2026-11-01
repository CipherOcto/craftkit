//! The **group** a clip is in: a set of clips that belong together.
//!
//! The tree's sound editor has a **group** on a clip: a shared number
//! that says which clips belong together. Two clips on two different
//! tracks can be in one group, and a group is not a folder — the clips
//! stay where they are, on their own tracks, and the group is a name
//! for the set.
//!
//! # What the tree's group is
//!
//! A number. `clip.group` gives the named clips a fresh number,
//! `clip.ungroup` takes it off them, and `clip.regroup` gives them the
//! number that most of them already had — because a person who groups
//! three clips that were already in a group means to put them back
//! together, and the tree has no other way of saying "the group they
//! were in".
//!
//! The number is not drawn and not named. The one place it is seen is
//! the tree's **clip group file**: the clips of a group are written out
//! together into a `.scgrp` file, and the group is named by the file.
//!
//! # What it is here
//!
//! A **group is a node**, and the clips in it **point at it** — an
//! edge of kind `sound.grouped`, the way a node points at the layer it
//! is on.
//!
//! Two reasons, and both of them are the kit's:
//!
//! - **A group has a name.** The tree's group is a number because the
//!   tree has nowhere to put a name; here the group is a node, and a
//!   node has a name. "The group called `Drums`" is a handle a person
//!   can say, and it is what the tree's `clip.regroup` is groping for
//!   when it counts ids.
//! - **A group is an edge and not a field.** The tree keeps the group
//!   number in the clip's own record, so a clip's group is a fact
//!   about the clip and the clip has to be rewritten to change it. Here
//!   the membership is an edge, and the clip's record is not touched.
//!
//! The kit's own **group** is a different thing: a group in the kit is
//! *structural* — a node with children, which the core walks into, and
//! the studio's `node.group` puts the selected nodes **into** it. A
//! sound group holds **nothing**: its members point at it, and the
//! clips stay on their own tracks, where they were. A clip that went
//! into a kit group would leave its playlist, and a playlist is where a
//! clip's place in the timeline is kept.
//!
//! # A group of tracks
//!
//! The tree's sound editor has a second kind of group: a group of
//! **tracks**, which is not a set of things that belong together but a
//! **link**. The tracks in it are mixed together: a change to one
//! track's level is a change to every track in the group.
//!
//! Here that is the same kind of node — a group — with two flags on
//! it: **mix** says the group links its members' levels, and
//! **active** says the group is on. A group that is off links nothing.

use craftkit_derive::{Command, NodeKind};

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Bool, Canvas, Document, EngineError, Node, NodeRef, Outcome, Point, Session, Text};

/// The edge that says a clip is in a group.
///
/// It goes **from** the clip **to** the group, the way `core.on_layer`
/// goes from the node to the layer it is on.
pub const GROUPED: &str = "sound.grouped";

/// A **group** of clips.
///
/// It holds nothing: the clips in a group point at it, and they stay on
/// their own tracks. It has no place — a group is not anywhere, it is
/// a set.
#[derive(NodeKind)]
#[node_kind(kind = "sound.group", label = "Group", layer = "sound", evaluates)]
pub struct GroupKind;

impl GroupKind {
    /// How a group is drawn.
    ///
    /// Nothing: a group is not a thing that is drawn, it is the name of
    /// a set of things.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a group.
    ///
    /// Nothing: a group is not anywhere.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a group **means**, in the plan: the clips in it.
    ///
    /// The members are **derived**: they are the edges that point at the
    /// group, and they are worked out every time rather than kept.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let members = members_of(document, node.id);
        let payload = payload_of(document, node.id);
        Some(serde_json::json!({
            "name": node.name,
            "members": members,
            "size": members.len(),
            "mix": payload.mix,
            "active": payload.active,
        }))
    }
}

/// What a group says about itself.
///
/// The two flags are about **tracks**: a group of clips has them and
/// nothing asks them, because there is no mix to link.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GroupPayload {
    /// Whether the group's members are **mixed together**: a change to
    /// one member's level is a change to every member's.
    pub(crate) mix: bool,
    /// Whether the group is **on**.
    ///
    /// A group that is off links nothing.
    pub(crate) active: bool,
}

impl GroupPayload {
    /// The payload of a group that links nothing.
    pub(crate) fn none() -> GroupPayload {
        GroupPayload { mix: false, active: true }
    }

    /// The payload of a group that **links** its members.
    pub(crate) fn linking() -> GroupPayload {
        GroupPayload { mix: true, active: true }
    }

    fn to_json(&self) -> Json {
        serde_json::json!({ "mix": self.mix, "active": self.active })
    }

    fn from_json(value: &Json) -> GroupPayload {
        GroupPayload {
            mix: value.get("mix").and_then(|mix| mix.as_bool()).unwrap_or(false),
            active: value.get("active").and_then(|active| active.as_bool()).unwrap_or(true),
        }
    }
}

/// The payload of a group, as the document holds it.
pub(crate) fn payload_of(document: &Document, group: u64) -> GroupPayload {
    document.node(group).map(|node| GroupPayload::from_json(&node.data)).unwrap_or_else(GroupPayload::none)
}

/// The tracks a change to `track`'s level applies to.
///
/// It is the track itself and the members of every **active** group
/// that **mixes** and has the track in it: the tree applies the same
/// change to the group, and a group that is off links nothing.
pub(crate) fn mix_siblings(document: &Document, track: u64) -> Vec<u64> {
    let mut tracks = vec![track];
    for group in document.walk().filter(|node| node.kind == "sound.group") {
        let payload = payload_of(document, group.id);
        if !payload.active || !payload.mix {
            continue;
        }
        for member in members_of(document, group.id) {
            if member == track {
                for other in members_of(document, group.id) {
                    if !tracks.contains(&other) {
                        tracks.push(other);
                    }
                }
            }
        }
    }
    tracks
}

/// The clips in a group.
fn members_of(document: &Document, group: u64) -> Vec<u64> {
    let mut members: Vec<u64> =
        document.relations_to(group, Some(GROUPED)).into_iter().map(|relation| relation.from).collect();
    members.sort_unstable();
    members
}

/// The group a clip is in, if it is in one.
pub(crate) fn group_of(document: &Document, clip: u64) -> Option<u64> {
    document.relations_from(clip, Some(GROUPED)).first().map(|relation| relation.to)
}

/// **Put clips in a group.**
///
/// The group is named: the clips go into the group with that name, and
/// the group is made if it is not there yet. So the same command puts
/// clips in a group and puts them back into the group they were in —
/// which is what the tree's `clip.regroup` does by counting ids, and
/// here it is done by saying the name.
///
/// A clip is in **one** group: a clip that is put in a group leaves the
/// group it was in. And a group of **one** clip is refused, because a
/// group is a set of clips that belong together and one clip belongs to
/// itself.
#[derive(Command)]
#[command(
    id = "sound.clip.group",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Group"),
    mutates,
    undoable
)]
pub struct Group {
    /// The name of the group.
    pub name: Text,
    /// The clips to put in it: their ids.
    pub clips: serde_json::Value,
}

impl Group {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.group".to_string();
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id, message: "a group has a name".to_string() });
        }
        let clips = crate::clip_ids_of(&self.clips, &id)?;
        if clips.len() < 2 {
            return Err(EngineError::Failed {
                id,
                message: "a group is two clips or more, and this is one".to_string(),
            });
        }
        for clip in &clips {
            let node = session.document().node(*clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
        }
        // The group: the one with that name, or a new one.
        let group = match session
            .document()
            .walk()
            .find(|node| node.kind == "sound.group" && node.name == name)
            .map(|node| node.id)
        {
            Some(group) => group,
            None => {
                let group = session.document_mut()?.add_node(name.clone(), "sound.group", None);
                // A group of **clips** links nothing: there is no mix
                // to link.
                if let Some(node) = session.document_mut()?.node_mut(group) {
                    node.data = GroupPayload::none().to_json();
                }
                group
            }
        };
        for clip in &clips {
            // A clip is in one group: the edge it had goes, and the new
            // one comes.
            session
                .document_mut()?
                .remove_relations(|relation| relation.from == *clip && relation.kind == GROUPED);
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(*clip, group, GROUPED));
        }
        // A group that is left with no clips is not a group.
        prune_empty_groups(session.document_mut()?);
        Ok(Outcome::changed(format!("the clips are in the group `{name}`"))
            .with_data(serde_json::json!({ "group": group, "name": name, "clips": clips.len() })))
    }
}

/// **Take clips out of their groups.**
///
/// A clip that is in no group is left alone: there is nothing to take
/// it out of, and a command that changes nothing leaves no undo step
/// behind.
///
/// A group that is left with no clips is **removed**: an empty group is
/// not a group, and a node that says nothing is litter.
#[derive(Command)]
#[command(
    id = "sound.clip.ungroup",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Clip", "Ungroup"),
    mutates,
    undoable
)]
pub struct Ungroup {
    /// The clips to take out of their groups: their ids.
    pub clips: serde_json::Value,
}

impl Ungroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.ungroup".to_string();
        let clips = crate::clip_ids_of(&self.clips, &id)?;
        let mut left = 0;
        for clip in &clips {
            if session.document().node(*clip).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            }
            left += session
                .document_mut()?
                .remove_relations(|relation| relation.from == *clip && relation.kind == GROUPED);
        }
        if left == 0 {
            return Ok(Outcome::new("the clips are in no group"));
        }
        prune_empty_groups(session.document_mut()?);
        Ok(Outcome::changed(format!("{left} clips left their group")).with_data(serde_json::json!({
            "clips": left,
        })))
    }
}

/// **Read the group** a clip is in.
///
/// The group is an edge and not a field, so the group of a clip is
/// **derived**: it is the edge the clip carries, and the members of
/// that group are the edges that point at it.
#[derive(Command)]
#[command(id = "sound.clip.group.read", area = "sound", layer = "sound", icon = "list")]
pub struct ReadGroup {
    /// The clip to read the group of.
    pub clip: NodeRef,
}

impl ReadGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.group.read".to_string();
        let clip = self.clip.0;
        let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{clip}`"),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
            });
        }
        let Some(group) = group_of(session.document(), clip) else {
            return Ok(Outcome::new(format!("the clip `{clip}` is in no group"))
                .with_data(serde_json::json!({ "id": clip, "group": Json::Null })));
        };
        let name = session.document().node(group).map(|node| node.name.clone()).unwrap_or_default();
        let members = members_of(session.document(), group);
        Ok(Outcome::new(format!("the clip `{clip}` is in the group `{name}`")).with_data(serde_json::json!({
            "id": clip,
            "group": group,
            "name": name,
            "members": members,
        })))
    }
}

/// Take away the groups that have no clips in them.
///
/// A group with no members is a node that says nothing: it is left in
/// the document and the document would carry a thing that means
/// nothing.
fn prune_empty_groups(document: &mut Document) {
    let empty: Vec<u64> = document
        .walk()
        .filter(|node| node.kind == "sound.group")
        .filter(|node| members_of(document, node.id).is_empty())
        .map(|node| node.id)
        .collect();
    for group in empty {
        document.remove_relations(|relation| relation.to == group && relation.kind == GROUPED);
        document.remove_node(group);
    }
}

/// **Put tracks in a group**.
///
/// The group is named: the tracks go into the group with that name, and
/// the group is made if it is not there yet — the same handle the group
/// of clips uses.
///
/// A group of tracks is a **link**: the members are mixed together, so
/// a change to one member's level is a change to every member's. That
/// can be taken off, and the group can be taken off altogether; both
/// are commands of their own.
///
/// A group of **one** track is refused, the way a group of one clip is:
/// a group is a set of things that belong together, and one thing
/// belongs to itself.
#[derive(Command)]
#[command(
    id = "sound.track.group",
    area = "sound",
    layer = "sound",
    icon = "group",
    menu("Track", "Group"),
    mutates,
    undoable
)]
pub struct SetTrackGroup {
    /// The name of the group.
    pub name: Text,
    /// The tracks to put in it: their ids.
    pub tracks: serde_json::Value,
}

impl SetTrackGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.group".to_string();
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id, message: "a group has a name".to_string() });
        }
        let tracks = crate::clip_ids_of(&self.tracks, &id)?;
        if tracks.len() < 2 {
            return Err(EngineError::Failed {
                id,
                message: "a group is two tracks or more, and this is one".to_string(),
            });
        }
        for track in &tracks {
            let node = session.document().node(*track).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{track}`"),
            })?;
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
        }
        // The group: the one with that name, or a new one. A group of
        // tracks **links** its members.
        let group = match session
            .document()
            .walk()
            .find(|node| node.kind == "sound.group" && node.name == name)
            .map(|node| node.id)
        {
            Some(group) => group,
            None => {
                let group = session.document_mut()?.add_node(name.clone(), "sound.group", None);
                if let Some(node) = session.document_mut()?.node_mut(group) {
                    node.data = GroupPayload::linking().to_json();
                }
                group
            }
        };
        for track in &tracks {
            // A track is in one group: the edge it had goes, and the
            // new one comes.
            session
                .document_mut()?
                .remove_relations(|relation| relation.from == *track && relation.kind == GROUPED);
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(*track, group, GROUPED));
        }
        // A group that is left with no members is not a group.
        prune_empty_groups(session.document_mut()?);
        Ok(Outcome::changed(format!("the tracks are in the group `{name}`"))
            .with_data(serde_json::json!({ "group": group, "name": name, "tracks": tracks.len() })))
    }
}

/// **Set the flags** of a group of tracks.
///
/// **Mix** says the group's members are mixed together, and **active**
/// says the group is on. A group that is off links nothing, whatever
/// its other flags say.
///
/// The parts that are not named are **left alone**.
#[derive(Command)]
#[command(
    id = "sound.track.group.set",
    area = "sound",
    layer = "sound",
    icon = "group",
    menu("Track", "Group Settings"),
    mutates,
    undoable
)]
pub struct SetGroupFlags {
    /// The group.
    pub group: NodeRef,
    /// Whether the members are mixed together.
    pub mix: Option<Bool>,
    /// Whether the group is on.
    pub active: Option<Bool>,
}

impl SetGroupFlags {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.group.set".to_string();
        let group = self.group.0;
        let node = session.document().node(group).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{group}`"),
        })?;
        if node.kind != "sound.group" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{group}` is a `{}` and not a group", node.kind),
            });
        }
        let mut payload = payload_of(session.document(), group);
        if let Some(mix) = self.mix {
            payload.mix = mix.0;
        }
        if let Some(active) = self.active {
            payload.active = active.0;
        }
        if let Some(node) = session.document_mut()?.node_mut(group) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("the group `{group}` is set")).with_data(serde_json::json!({
            "group": group,
            "mix": payload.mix,
            "active": payload.active,
        })))
    }
}

/// **Take a group** of tracks away.
///
/// The group is removed and the tracks stay where they were: a group is
/// a link between them and not a box they are in.
#[derive(Command)]
#[command(
    id = "sound.track.ungroup",
    area = "sound",
    layer = "sound",
    icon = "group",
    menu("Track", "Ungroup"),
    mutates,
    undoable
)]
pub struct UngroupTracks {
    /// The group to take away.
    pub group: NodeRef,
}

impl UngroupTracks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.ungroup".to_string();
        let group = self.group.0;
        let node = session.document().node(group).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{group}`"),
        })?;
        if node.kind != "sound.group" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{group}` is a `{}` and not a group", node.kind),
            });
        }
        let members = members_of(session.document(), group);
        session.document_mut()?.remove_relations(|relation| relation.to == group && relation.kind == GROUPED);
        session.document_mut()?.remove_node(group);
        Ok(Outcome::changed(format!("the group `{group}` is taken away"))
            .with_data(serde_json::json!({ "group": group, "tracks": members.len() })))
    }
}
