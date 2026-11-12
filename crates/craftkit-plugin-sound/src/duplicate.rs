//! The **duplicate** of a track: a copy of the track, next to it.
//!
//! The tree's editor duplicates a track: the copy has the same clips,
//! the same automation, the same inserts and the same sends, and it
//! stands where the old one stands. The copy is a **new track** — a
//! new set of clips with their own ids — and the material it plays is
//! the same material: the sources are shared, and a sound is a file
//! and a file may be played twice.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome};

use crate::mixer::{InsertPayload, LanePayload, SendPayload};

commands! {
    /// **Duplicate** a track: a copy of it, next to it.
    ///
    /// The copy carries the track's **clips**, its **automation**, its
    /// **inserts** and its **sends**, and each of them may be left out.
    /// The **sources** are shared: a sound is a file, and a file may be
    /// played twice.
    ///
    /// A **master** is refused: it holds the fader of the mix and not a
    /// sound of its own, and two of them would be two faders over one
    /// mix.
    sound.track.duplicate {
        area: "sound",
        layer: "sound",
        icon: "copy",
        mutates,
        undoable,
        menu: ("Track", "Duplicate"),
        params: {
            /// The track to duplicate.

            track: NodeRef,
            /// Whether the copy carries the **playlists** and their clips.

            playlists: Option<Bool>,
            /// Whether the copy carries the **automation**.

            automation: Option<Bool>,
            /// Whether the copy carries the **inserts**.

            inserts: Option<Bool>,
            /// Whether the copy carries the **sends**.

            sends: Option<Bool>,
        },
        run: {

            let id = "sound.track.duplicate".to_string();
            let track = self.track.0;
            // The **document** is copied first: the copy is made out of
            // it, and the session is written to while it is read.
            let document = session.document().clone();
            let Some(node) = document.node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            if crate::is_master(node) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the master holds the fader of the mix and not a sound of its own".to_string(),
                });
            }
            let playlists = self.playlists.map(|playlists| playlists.0).unwrap_or(true);
            let automation = self.automation.map(|automation| automation.0).unwrap_or(true);
            let inserts = self.inserts.map(|inserts| inserts.0).unwrap_or(true);
            let sends = self.sends.map(|sends| sends.0).unwrap_or(true);
            let name = node.name.clone();
            // The **track** of the copy: the same track, with the same
            // settings, under a name that says it is a copy.
            let new_name = format!("{name}.dup");
            let payload = crate::TrackPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            let new_track =
                session.document_mut()?.add_node_with(new_name.clone(), "sound.track", None, payload.to_json());
            // The **playlists** and their clips: the copies are new clips
            // with their own ids, and they play the same sources.
            let mut clips = 0_usize;
            if playlists {
                for playlist in node.children.iter().filter(|child| child.kind == "sound.playlist") {
                    let new_playlist = session.document_mut()?.add_node_with_under(
                        Some(new_track),
                        playlist.name.clone(),
                        "sound.playlist",
                        None,
                        craftkit_sdk::serde_json::Value::Null,
                    );
                    for clip in playlist.children.iter().filter(|child| child.kind == "sound.clip") {
                        let clip_payload = match crate::ClipPayload::from_json(&clip.data) {
                            Ok(payload) => payload,
                            Err(_) => continue,
                        };
                        let source = clip_payload.source;
                        let new_clip = session.document_mut()?.add_node_with_under(
                            Some(new_playlist),
                            clip.name.clone(),
                            "sound.clip",
                            None,
                            clip_payload.to_json(),
                        );
                        if let Some(source) = source {
                            session
                                .document_mut()?
                                .add_relation(craftkit_sdk::Relation::new(new_clip, source, "source"));
                        }
                        clips += 1;
                    }
                }
            }
            // The **automation**: the lanes, with their breakpoints.
            let mut lanes = 0_usize;
            if automation {
                for lane in node.children.iter().filter(|child| child.kind == "sound.lane") {
                    let Ok(payload) = LanePayload::from_json(&lane.data) else { continue };
                    session.document_mut()?.add_node_with_under(
                        Some(new_track),
                        lane.name.clone(),
                        "sound.lane",
                        None,
                        payload.to_json(),
                    );
                    lanes += 1;
                }
            }
            // The **inserts**: the plugins, with their parameters.
            let mut inserted = 0_usize;
            if inserts {
                for insert in node.children.iter().filter(|child| child.kind == "sound.insert") {
                    let Ok(payload) = InsertPayload::from_json(&insert.data) else { continue };
                    session.document_mut()?.add_node_with_under(
                        Some(new_track),
                        insert.name.clone(),
                        "sound.insert",
                        None,
                        payload.to_json(),
                    );
                    inserted += 1;
                }
            }
            // The **sends**: the same buses, at the same levels.
            let mut sent = 0_usize;
            if sends {
                for send in node.children.iter().filter(|child| child.kind == "sound.send") {
                    let Ok(payload) = SendPayload::from_json(&send.data) else { continue };
                    let Some(bus) = document.relations_from(send.id, Some("send")).into_iter().next() else {
                        continue;
                    };
                    let new_send = session.document_mut()?.add_node_with_under(
                        Some(new_track),
                        send.name.clone(),
                        "sound.send",
                        None,
                        payload.to_json(),
                    );
                    session.document_mut()?.add_relation(craftkit_sdk::Relation::new(new_send, bus.to, "send"));
                    sent += 1;
                }
            }
            // The **route**: a track that feeds a bus feeds it after the
            // copy as well.
            let route = crate::mixer::route_of(&document, track);
            if let Some(bus) = route {
                session.document_mut()?.add_relation(craftkit_sdk::Relation::new(new_track, bus, "route"));
            }
            // The **place** of the copy: where the old one stands.
            crate::place_after(session, new_track, track);
            Ok(Outcome::changed(format!("duplicated the track `{track}` into `{new_name}`")).with_data(json!({
                "track": track,
                "duplicate": new_track,
                "name": new_name,
                "clips": clips,
                "lanes": lanes,
                "inserts": inserted,
                "sends": sent,
                "bus": route,
            })))

        }
    }
}
