//! The **auto rename** of a clip: the names that editing gives the
//! clips it separates.
//!
//! The tree's sound editor has an **Auto Rename** scheme: a name, a
//! number and a suffix, and the clips that are separated from a clip
//! with a scheme are named from it. "Kick" cut in four becomes
//! "Kick", "Kick1", "Kick2" and "Kick3" — and the numbers keep
//! counting, so the next clip separated from any of them is "Kick4".
//!
//! # What the tree's scheme is
//!
//! Four things: the **name** before the number, the **number** the
//! next new clip gets, the **zeros** that go in front of the number
//! and the **suffix** after it. The name of a new clip is the name,
//! then the zeros, then the number, then the suffix — "SFX", one
//! zero, twenty-four and ".Reel1" is "SFX024.Reel1".
//!
//! The tree keeps the schemes in the **session**, keyed by a number,
//! and each clip holds the key of the scheme it is in. The key is
//! what makes the scheme **shared**: the clips separated from one
//! clip share its scheme, so they count up one number sequence, and a
//! clip separated from one of *those* counts on in the same sequence.
//!
//! # What it is here
//!
//! A **scheme is a node**, and the clips in it **point at it** — the
//! same shape as a group (§199). Two reasons, and both of them are
//! the kit's:
//!
//! - **A scheme is a thing with a life of its own.** The tree's scheme
//!   is a number in a map beside the session's other settings, and it
//!   is **pruned** when no clip uses it; here a scheme is a node, and
//!   a node that says nothing is litter — so a scheme no clip points
//!   at is removed as well.
//! - **The membership is an edge and not a field.** The tree keeps the
//!   key in the clip's own record, so a clip has to be rewritten to
//!   change its scheme. Here the scheme is an edge, and the clip's
//!   record is not touched.
//!
//! The scheme is **counted up** by the pieces that editing makes: the
//! piece after a cut takes the scheme's number and the scheme counts
//! up. The piece **before** the cut keeps the name it had, because the
//! tree's does too.
//!
//! # What the tree removes and this tree keeps
//!
//! The tree's scheme has an **id** of its own, "to tell this scheme
//! from another one under the same key after an undo or in another
//! session, for pastes from the clipboard". The clipboard is in this
//! battery now (§247), and the schemes are told apart by
//! their **nodes** — a node is the thing, and the tree's id is a key
//! under a key — so there is no id here.

use craftkit_derive::{NodeKind, commands};

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Point, Session, Text};

/// The edge that says a clip is in an auto rename scheme.
///
/// It goes **from** the clip **to** the scheme, the way `sound.grouped`
/// goes from the clip to its group.
pub const AUTONAMED: &str = "sound.autonamed";

/// The most a scheme's first number may be.
///
/// The tree's own bound is nine hundred and ninety-nine million nine
/// hundred and ninety-nine thousand: a scheme that counts to a
/// thousand billion is a scheme that counts to infinity in practice,
/// and the tree refuses a number above its bound rather than taking it.
const MAX_NUMBER: u64 = 999_999_999;

/// The most zeros that may go in front of the number.
///
/// The tree's own bound is nine, and a name with ten zeros in it is a
/// name with a mistake in it.
const MAX_ZEROS: usize = 9;

/// The most characters the name and the suffix may have **together**.
///
/// The tree's bound is its own: twenty-seven characters are left free
/// for the nine zeros and the twenty digits of a big number, so that
/// a clip's name never loses its number to the limit on names. The
/// limit is the same here, because the reason is the same — the name
/// is a name and the number has to fit inside it.
const MAX_TEXT_CHARS: usize = 227;

/// A **scheme**: the names that editing gives the clips it separates.
///
/// It has no place — a scheme is not anywhere, it is a set of names.
#[derive(NodeKind)]
#[node_kind(kind = "sound.autoname", label = "Auto Rename", layer = "sound", evaluates)]
pub struct AutoNameKind;

impl AutoNameKind {
    /// How a scheme is drawn.
    ///
    /// Nothing: a scheme is not a thing that is drawn, it is the way
    /// the clips separated from a clip are named.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a scheme.
    ///
    /// Nothing: a scheme is not anywhere.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a scheme **means**, in the plan: the names it gives and the
    /// clips that are in it.
    ///
    /// The members are **derived**: they are the edges that point at
    /// the scheme, and they are worked out every time rather than kept.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let payload = payload_of(document, node.id);
        let members = members_of(document, node.id);
        Some(serde_json::json!({
            "name": payload.name,
            "number": payload.number,
            "zeros": payload.zeros,
            "suffix": payload.suffix,
            "next": payload.next_name(),
            "members": members,
            "size": members.len(),
        }))
    }
}

/// What a scheme says about itself.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AutoNamePayload {
    /// The text before the number.
    pub(crate) name: String,
    /// The number the next new clip gets.
    pub(crate) number: u64,
    /// How many zeros go in front of the number.
    pub(crate) zeros: usize,
    /// The text after the number.
    pub(crate) suffix: String,
}

impl AutoNamePayload {
    /// The name of the next clip the scheme names.
    ///
    /// The name, then the zeros, then the number, then the suffix —
    /// "SFX", one zero, twenty-four and ".Reel1" is "SFX024.Reel1".
    pub(crate) fn next_name(&self) -> String {
        format!("{}{}{}{}", self.name, "0".repeat(self.zeros), self.number, self.suffix)
    }

    fn to_json(&self) -> Json {
        serde_json::json!({
            "name": self.name,
            "number": self.number,
            "zeros": self.zeros,
            "suffix": self.suffix,
        })
    }

    fn from_json(value: &Json) -> AutoNamePayload {
        AutoNamePayload {
            name: value.get("name").and_then(|name| name.as_str()).unwrap_or("").to_string(),
            number: value.get("number").and_then(|number| number.as_u64()).unwrap_or(1),
            zeros: value.get("zeros").and_then(|zeros| zeros.as_u64()).unwrap_or(0) as usize,
            suffix: value.get("suffix").and_then(|suffix| suffix.as_str()).unwrap_or("").to_string(),
        }
    }
}

/// The payload of a scheme, as the document holds it.
pub(crate) fn payload_of(document: &Document, scheme: u64) -> AutoNamePayload {
    document.node(scheme).map(|node| AutoNamePayload::from_json(&node.data)).unwrap_or_else(|| {
        AutoNamePayload { name: String::new(), number: 1, zeros: 0, suffix: String::new() }
    })
}

/// The clips in a scheme.
fn members_of(document: &Document, scheme: u64) -> Vec<u64> {
    let mut members: Vec<u64> =
        document.relations_to(scheme, Some(AUTONAMED)).into_iter().map(|relation| relation.from).collect();
    members.sort_unstable();
    members
}

/// The scheme a clip is in, if it is in one.
pub(crate) fn scheme_of(document: &Document, clip: u64) -> Option<u64> {
    document.relations_from(clip, Some(AUTONAMED)).first().map(|relation| relation.to)
}

/// A scheme that no clip is in is not a scheme, and a node that says
/// nothing is litter.
fn prune_empty_schemes(document: &mut Document) {
    let empty: Vec<u64> = document
        .walk()
        .filter(|node| node.kind == "sound.autoname")
        .filter(|node| members_of(document, node.id).is_empty())
        .map(|node| node.id)
        .collect();
    for scheme in empty {
        document.remove_node(scheme);
    }
}

commands! {
    /// **Name the clips that editing separates.**
    ///
    /// The scheme says what the new pieces are called: the **name**, then
    /// the **zeros**, then the **number** the next piece gets, then the
    /// **suffix**. The clips are named when they are **separated** — the
    /// piece before a cut keeps the name it had, and the piece after it
    /// takes the next number of the scheme.
    ///
    /// Without a name, the clips' scheme is **taken away**: the tree's
    /// command does the same, and a clip that is separated afterwards
    /// keeps the names it has.
    ///
    /// The number and the zeros and the suffix **need** a name: they say
    /// what the name is made of, and without a name there is nothing to
    /// make.
    sound.clip.autoname {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Auto Rename"),
        params: {
            /// The clips to name: their ids.

            clips: serde_json::Value,
            /// The name before the number. Without it, the scheme is taken away.

            name: Option<Text>,
            /// The number the next new piece gets. One without it.

            number: Option<Number>,
            /// How many zeros go in front of the number. None without it.

            zeros: Option<Number>,
            /// The text after the number. Nothing without it.

            suffix: Option<Text>,
        },
        run: {

            let id = "sound.clip.autoname".to_string();
            let clips = crate::clip_ids_of(&self.clips, &id)?;
            if clips.is_empty() {
                return Err(EngineError::Failed { id, message: "there are no clips to name".to_string() });
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
            // A **blank** name is no name: the tree takes the scheme away
            // for a name of spaces as well.
            let name = self.name.as_ref().map(|name| name.0.trim().to_string()).filter(|name| !name.is_empty());
            // The number, the zeros and the suffix say what the name is
            // made of: without a name there is nothing to make.
            if name.is_none() && (self.number.is_some() || self.zeros.is_some() || self.suffix.is_some()) {
                return Err(EngineError::Failed {
                    id,
                    message: "the number, the zeros and the suffix need a name".to_string(),
                });
            }
            let Some(name) = name else {
                // **Taking the scheme away**: the clips keep their names
                // and are named by nothing when they are separated.
                let mut left = 0;
                for clip in &clips {
                    left += session
                        .document_mut()?
                        .remove_relations(|relation| relation.from == *clip && relation.kind == AUTONAMED);
                }
                prune_empty_schemes(session.document_mut()?);
                if left == 0 {
                    return Ok(Outcome::new("the clips are named by nothing already"));
                }
                return Ok(Outcome::changed(format!("the scheme is taken away from {left} clips"))
                    .with_data(serde_json::json!({ "clips": left, "named": false })));
            };
            let payload = AutoNamePayload {
                name,
                number: whole(&id, self.number, 1, MAX_NUMBER, "number")?,
                zeros: whole(&id, self.zeros, 0, MAX_ZEROS as u64, "zeros")? as usize,
                suffix: self.suffix.as_ref().map(|suffix| suffix.0.clone()).unwrap_or_default(),
            };
            if payload.name.chars().count() + payload.suffix.chars().count() > MAX_TEXT_CHARS {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "the name and the suffix together must be at most {MAX_TEXT_CHARS} characters"
                    ),
                });
            }
            // The scheme is a node of its own, and it holds nothing.
            let scheme = session.document_mut()?.add_node(payload.name.clone(), "sound.autoname", None);
            if let Some(node) = session.document_mut()?.node_mut(scheme) {
                node.data = payload.to_json();
            }
            for clip in &clips {
                // A clip is in one scheme: the edge it had goes, and the
                // new one comes.
                session
                    .document_mut()?
                    .remove_relations(|relation| relation.from == *clip && relation.kind == AUTONAMED);
                session.document_mut()?.add_relation(craftkit_sdk::Relation::new(*clip, scheme, AUTONAMED));
            }
            prune_empty_schemes(session.document_mut()?);
            Ok(Outcome::changed(format!("the clips are named by `{}`", payload.name)).with_data(
                serde_json::json!({
                    "clips": clips.len(),
                    "named": true,
                    "scheme": scheme,
                    "next": payload.next_name(),
                }),
            ))

        }
    }
    /// **Read the scheme** a clip is in.
    ///
    /// The scheme is read and the **next name** is worked out: the name
    /// the next piece separated from the clip would get.
    sound.clip.autoname.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
            /// The clip to read the scheme of.

            clip: NodeRef,
        },
        run: {

            let id = "sound.clip.autoname.read".to_string();
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
            let Some(scheme) = scheme_of(session.document(), clip) else {
                return Ok(Outcome::new("the clip is named by nothing").with_data(serde_json::json!({
                    "clip": clip,
                    "named": false,
                })));
            };
            let payload = payload_of(session.document(), scheme);
            Ok(Outcome::new(format!("the clip is named by `{}`", payload.name)).with_data(serde_json::json!({
                "clip": clip,
                "named": true,
                "scheme": scheme,
                "name": payload.name,
                "number": payload.number,
                "zeros": payload.zeros,
                "suffix": payload.suffix,
                "next": payload.next_name(),
            })))

        }
    }
}

/// The **number** a scheme is at.
///
/// It is the number the next piece would take, and a scheme that is
/// counting is a scheme that has already named something.
pub(crate) fn number_of(session: &Session, scheme: u64) -> u64 {
    payload_of(session.document(), scheme).number
}

/// Put a scheme's **number** back where it was.
///
/// The strip of a silence cuts a clip into pieces and every one of
/// them takes a name as it is cut — and then the quiet pieces are
/// thrown away, and their names are names of pieces that are not
/// there. So the number is put back before the pieces that are kept
/// are named: the kept pieces take the numbers of the scheme, in time
/// order, and nothing is skipped.
pub(crate) fn set_number(session: &mut Session, scheme: u64, number: u64) -> bool {
    let mut payload = payload_of(session.document(), scheme);
    payload.number = number;
    let Ok(document) = session.document_mut() else { return false };
    if let Some(node) = document.node_mut(scheme) {
        node.data = payload.to_json();
    }
    true
}

/// Name the **pieces** of a cut in order, from the scheme.
///
/// The pieces are named in the order they are given, and the scheme
/// counts up once for each of them: the first piece takes the
/// scheme's number and the scheme counts up, and the next piece takes
/// the next number.
///
/// The pieces are already **in** the scheme — the cut put them there —
/// and this is about their names: the scheme is one sequence and the
/// pieces take it in order.
pub(crate) fn name_the_pieces(session: &mut Session, scheme: u64, pieces: &[u64]) -> usize {
    let mut payload = payload_of(session.document(), scheme);
    let mut named = 0;
    for piece in pieces {
        let name = payload.next_name();
        payload.number = payload.number.saturating_add(1);
        let Ok(document) = session.document_mut() else { break };
        if let Some(node) = document.node_mut(*piece) {
            node.name = name;
        }
        named += 1;
    }
    if let Ok(document) = session.document_mut()
        && let Some(node) = document.node_mut(scheme)
    {
        node.data = payload.to_json();
    }
    named
}

/// A whole number from a parameter, refused when it is not one.
///
/// A number that is not whole — a half a number, or a billion — is
/// not a count of clips, and it is **refused** and not rounded.
fn whole(id: &str, value: Option<Number>, default: u64, max: u64, what: &str) -> Result<u64, EngineError> {
    let Some(value) = value else { return Ok(default) };
    let number = value.0;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > max as f64 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the {what} is a whole number from 0 to {max}, and {number} is not one"),
        });
    }
    Ok(number as u64)
}

/// **Name the piece** that a cut made.
///
/// The piece after the cut is a new clip, and it takes the next name of
/// the scheme the clip was in: the scheme's number, and then the
/// scheme counts up. The piece **before** the cut keeps the name it
/// had — the tree's does too, because the clip it was is still the
/// clip it was.
///
/// A clip with no scheme is not named by anything, and the piece it
/// makes is not named either.
pub(crate) fn name_the_new_piece(session: &mut Session, clip: u64, piece: u64) -> Result<(), EngineError> {
    let Some(scheme) = scheme_of(session.document(), clip) else { return Ok(()) };
    let mut payload = payload_of(session.document(), scheme);
    let name = payload.next_name();
    payload.number = payload.number.saturating_add(1);
    // The piece is in the scheme too, and the scheme has counted up.
    session.document_mut()?.add_relation(craftkit_sdk::Relation::new(piece, scheme, AUTONAMED));
    if let Some(node) = session.document_mut()?.node_mut(scheme) {
        node.data = payload.to_json();
    }
    if let Some(node) = session.document_mut()?.node_mut(piece) {
        node.name = name;
    }
    Ok(())
}
