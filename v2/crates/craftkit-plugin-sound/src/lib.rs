//! `craftkit-plugin-sound` — the **sound battery**.
//!
//! # What this is
//!
//! The port of a sound application: a document of tracks, of clips on
//! them, of a tempo map, of markers and of the media the clips point at.
//! It is a **battery** — an optional plugin — because a program that
//! edits pictures has no use for a tempo map, and a battery is how v2
//! says that: the crate is not there at all unless a feature asks for it.
//!
//! It contributes:
//!
//! - a **layer**, `sound`, which the rest of it is in;
//! - **sixteen kinds**: `sound.track`, `sound.playlist`, `sound.clip`,
//!   `sound.bus`, `sound.send`, `sound.insert`, `sound.lane`,
//!   `sound.tempo`, `sound.meter`, `sound.key`, `sound.marker`,
//!   `sound.ruler`, `sound.playhead`, `sound.source`, `sound.group` and
//!   `sound.autoname`;
//! - **a hundred and twenty-seven commands**, one for each thing a person does to a
//!   sound document: make a track, set a track, make a clip, mute a
//!   clip, space the clips against each other, snap a clip to its
//!   neighbour, loop a clip and take the
//!   loop away, put a clip in front of the others, make a take, choose
//!   the
//!   take that is played, set the tempo,
//!   set the meter, make a marker, make a source, set the session, set
//!   the **beat** of a clip and take it away, lock a clip's **content**
//!   and its **place**, rate a clip, group clips and take them out of
//!   the group, set the **grid** and put a clip on it, set a clip's
//!   **gain** and bypass it, **conform** a clip to the tempo, group
//!   **tracks** so that they are mixed together, make a track
//!   **elastic** so that its clips are stretched and the pitch stays,
//!   set a track's **fader** and its **pan**, move a clip's **pitch**,
//!   do a **process** to a clip's sound and take it off, render the
//!   **stems**, **separate** a clip into two or on the **grid**,
//!   **move** and **rename** a marker, **scale** the tempos and make a
//!   range of the tempo map **constant**,
//!   **mute** a track, make a track **solo safe**, **name** the clips
//!   that editing separates, **import** and **export** the **MIDI** of
//!   a clip, **quantise**, **transpose** and change the **velocity** and
//!   the **length** of the notes of a clip, **make** a MIDI clip and
//!   **add**, **edit**,
//!   **remove** and **move** the notes of one, **add**, **edit** and
//!   **remove** the controllers of one and take the duplicate notes
//!   out of it, **heal** the clips that
//!   were separated, **take away** a route, a reading, a send or a
//!   source, **set**, **take away**, **clear**, **write** and **thin**
//!   the breakpoints of a lane, **move** a clip and **trim** it to the
//!   end of its material,
//!   separate a clip at its **transients**, **strip** the silence out
//!   of a clip in any of its three **modes**, **copy**, **cut** and
//!   **paste** the material of a range, turn a track's **voice** off
//!   and on again, **commit** a track to a file, **take away** an
//!   **insert** and **move** it in the chain, **rename** a track and a
//!   clip, change the **settings** of a send, **duplicate** a track,
//!   **render** a clip to a file, take a **tempo**, a **meter** and a
//!   **key** away, and
//!   the rest that came with the mixer — the bus, the send, the insert,
//!   the lane, the route, the input, the peaks and the render;
//! - a **tool**, `tool.sound.clip`;
//! - a **panel**, `panel.sound`;
//! - **two evaluators**, `sound.timeline`, which says what the document
//!   means: the tempo map, and every clip in the place it sits, and
//!   `sound.routing`, which says what the mixer is made of.
//!
//! # What the port found
//!
//! Three things, and two of them are the core's business.
//!
//! **The core's node had a place.** A clip in a sound document is at a
//! **time**, and a time is not a place. The core changed: a node may have
//! no place, and the sound kinds have none.
//!
//! **The core's document had nowhere to put the session.** A sound
//! session's sample rate and timecode belong to the session and not to any
//! one node, so the document grew a payload of its own, and the sound
//! battery keeps its settings under one name in it.
//!
//! **The kinds of a sound document are open.** In the tree this ports, the
//! kind of a track is a closed enum of eight variants and the kind of a
//! clip is a closed enum of three, and every one of them is matched in
//! the mixer. Here a track's kind is a **string in the track's payload**
//! and a clip's content is a **field in the clip's payload**, so a third
//! party can add one without touching the core — which is the whole
//! point of the seam, arrived at by a port rather than by an argument.
//!
//! **A clip's beat is an option and not a zero.** The tree keeps the
//! place inside a clip where the beat falls as a number, and zero means
//! *none* — so a clip whose beat is at its very first sample cannot say
//! so. Here it is an option, and a beat at the head is a beat.
//!
//! **The grid is a setting of the session and not of the editor.** The
//! tree keeps the grid in its **edit state** — beside the tool, the
//! edit mode and the selection — and saves the session with it, so the
//! tree's file does say what grid it was made on, in a place where a
//! reader would not think to look. Here the grid is with the sample
//! rate and the timecode: a file says what it is without saying what
//! was selected when it was written.
//!
//! **A clip's colour is the kit's appearance.** The tree's sound
//! editor has a command of its own for the colour of a clip, a tint
//! for its bar on the timeline. The kit already says how every node is
//! drawn — an **appearance** is the colour a node is filled with, and
//! one command sets it on any node — so the battery has no command for
//! it, and the clip's colour is the appearance the clip carries.
//!
//! **The mix is worked out once.** The renderer builds the whole mix
//! and the stems are that same mix with one track in it: the tree
//! clones the session, solos the track and renders the mix again, and
//! here the mix is a function of the document alone, so a stem and the
//! mix are the same sound and not two ways of making one.
//!
//! **A pan is a balance and not a placement.** The tree's mixer keeps
//! a pan on every track, and a stereo track has one panner for each of
//! its channels; the tree's law for a panner is **constant power**,
//! which is a law for placing a mono sound in a stereo field, and it
//! takes three decibels off a stereo sound in the middle. Here a track
//! has one pan and it **balances** the sound: the middle is the
//! middle, and the sound is not quieter there than it is at the side.
//! The tree has the balance law too — it is the one the tree calls
//! `Zero` — and it is the law that fits a sound that is already in two
//! channels.
//!
//! **A process is a field and not a rendering.** The tree's audio
//! suite reverses a clip's sound, turns it upside down, brings it to a
//! level and takes the direct current away — and it does each of them
//! by rendering the material and replacing the clip's content with
//! the rendered file. Here a process is a name on the clip, and the
//! sound is worked out when it is played. The name is a string and
//! not a variant, for the same reason the kinds are strings here —
//! but a name this battery does not know is **refused**, because a
//! process that does nothing is worse than no process at all.
//!
//! **A shifted pitch is a field and not a rendering.** The tree's
//! pitch shift is one of its **audio-suite processes**: it renders the
//! material, replaces the clip's content with the rendered file, and
//! keeps the original in the edit state so that `clip.remove_pitch_shift`
//! can put it back. Here the pitch is a number of semitones on the
//! clip, and the sound is worked out when it is played — so there is
//! no rendered file, no edit state and nothing to remove. The two
//! steps the tree's shift is made of are the two this crate has: the
//! sound is resampled as if it were played faster, and stretched back
//! to the length it was.
//!
//! **A stretched clip is stretched one way.** The tree has a clip's
//! **speed** and a clip's **warp** as two things, and a warp bakes the
//! material into a rendered file and makes the clip longer, and the
//! original is kept in the edit state so that a warp can be taken off.
//! Here a clip has **one** stretch, and how it is played is the
//! **track's** business: an elastic track's clips are stretched and the
//! pitch stays, and every other track's are played at another speed and
//! the pitch falls with it. The tree's own words for the difference are
//! the same ones this uses, and the tree's rendering of the warp — a
//! file in the session, and a length in the edit state — has no place
//! here: the stretch is a fact and the material is derived from it.
//!
//! **A naming scheme is a node and not a number in a map.** The tree
//! keeps its auto rename schemes in the session, keyed by a number,
//! and each clip holds the key of the scheme it is in — and the key is
//! what makes the scheme **shared**, because the clips separated from
//! one clip share it and count up one sequence. Here the scheme is a
//! node and the clips **point at it**, the same shape as a group
//! (§199): a scheme is a thing with a life of its own, it is pruned
//! when no clip is in it, and a clip's record is not rewritten to
//! change its scheme.
//!
//! **A clip's content is a field and not a variant.** The tree keeps
//! the content of a clip in a closed enum of three — audio, MIDI and
//! video — and every one of them is matched in the mixer. Here the
//! clip says what its content is with a **string**, and the notes of a
//! MIDI clip are in the clip's payload: a note is a pitch and a place
//! and a length, and it has no life of its own.
//!
//! **The session's events are edited and not only put there.** A
//! marker could be added and not moved, and the tempo map could be
//! given a tempo and not scaled: so the tree's "Scale..." and
//! "Constant..." had no place here, and the tree's markers could be
//! edited and these could not.
//!
//! **The whole of a MIDI clip can be edited.** The tree's MIDI editor
//! has a pencil for the **notes** and none for the **controllers**;
//! here both have one, and the duplicate notes can be taken out — the
//! tree has a command for that too, and it is the library's own
//! `remove_duplicates`.
//!
//! **A clip could not be moved.** The tree's editor nudges the clips
//! and shifts the ones on a track; here a clip was placed and trimmed
//! and never **moved**, because the trim changes what is heard and the
//! move changes when it is heard. So the move is a command of its own,
//! and a locked clip does not move — the tree's own rule, and the
//! tree's own words: a locked clip "doesn't move".
//!
//! **A lane's breakpoints are edited and not only written.** The tree
//! has five commands for a lane — write a value over a range, clear
//! the breakpoints, thin them, and write the current value over the
//! range — and v2 had **one**: a lane could be made with its points
//! and not changed afterwards. So a curve could be drawn and not
//! drawn again, and a mistake in it could not be undone.
//!
//! **A link that cannot be taken away is a mistake that cannot be
//! undone.** The route between a track and a bus, the reading of a bus
//! by a track and the link of a send are **relations** and not nodes,
//! and a relation is not a node that `node.delete` can take away: so
//! the sound battery has commands of its own for taking them away. A
//! source is a node, and taking it away takes the **asset** it brought
//! with it — a document does not need a file that nothing plays.
//!
//! **A note is refused where the tree clamps it.** The tree's pencil
//! clamps a pitch, a velocity and a channel into the range a MIDI note
//! has, and a note asked for as a hundred and forty is a note the
//! caller does not know about. Here every one of them is refused, and
//! the note is not made — the same rule the transpose follows
//! (§219).
//!
//! **A limit clamps, and everything else refuses.** The tree changes a
//! note's velocity four ways and clamps all of them into the range.
//! Here the **set**, the **change** and the **scale** are refused when
//! the result would leave the range (§219), and only the **limit** moves
//! a note onto the edge of the limit — because a limit says *no softer
//! than this and no harder than that*, and moving a note onto its edge
//! is what was asked for and not a quiet correction. The limit's own
//! bounds are refused all the same (§229).
//!
//! **The room of a MIDI clip does not shrink.** The notes of a clip are
//! in ticks and the length of the clip is in samples, and the room the
//! notes need is worked out against the tempo map. A note that reaches
//! beyond the clip makes the clip **longer**, and a note that moves
//! away does not make it shorter: the room is the room. The tree's
//! clip has a length of its own and the notes do not touch it at all.
//!
//! **The quiet is thrown away and the loud parts are named.** The
//! tree's strip silence finds the parts of a sound that are not
//! silent, cuts the clip at the edges of them and keeps the loud
//! parts; the kept parts take the names of the auto rename scheme, in
//! time order. Here the parts are found in the **material** and not in
//! the clip's own sound, and the pieces that are thrown away take no
//! number: the scheme's number is put back before the kept pieces are
//! named, so that nothing is skipped.
//!
//! **A transient is found by the spectrum, as it is in the tree.** The
//! detector takes a fast Fourier transform of each frame and watches
//! the bands for a jump; the transform is in the audio library now,
//! written there because this kit's crates carry no dependencies at
//! all.
//!
//! **A separation is joined again by what it is, not by where it is.**
//! The tree's editor heals a track: it walks the clips in order and
//! joins each one with the one before it when they are **the same
//! sound continued** — the same source, the second starting where the
//! first's sound carries on, and the same way of playing it. Here the
//! rule is the same, and the tree's is checked in full: the mute, the
//! stretch, the gain **and** the pitch, because a clip played at
//! another pitch is not the same sound continued.
//!
//! **A note's place is a fact and the tree's quantise is not.** The
//! tree's quantise has a **random** component: a note is moved by a
//! random fraction of a step, and the same quantise twice does not
//! give the same notes. Here it does not: a note's place in a
//! document is a fact, and a fact that changes each time it is asked
//! for is not a fact.
//!
//! **A cut divides the notes of a MIDI clip.** The tree's cut keeps
//! the notes before the cut in the first piece and the notes at or
//! after it in the second, and moves the second's notes back by the
//! cut — a note is not in both pieces. Here the cut does the same,
//! and the boundary is the distance the cut made in the **ticks** of
//! the document's tempo, so the division follows the tempo (§232).
//!
//! **A trim rebases the notes of a MIDI clip.** The tree's trim of a
//! MIDI clip takes the notes off the head and counts the notes that
//! are left from the clip's new head — the model cannot do it, because
//! the notes are in ticks and the model does not know the tempo map,
//! so the **engine** does it after. Here the trim does the same
//! (§233).
//!
//! **The tree's automation is done over every track and this tree's
//! over one.** The tree's "Thin All" thins every lane of every track
//! and its "Write to Current" writes the static value of every
//! parameter over a range. Here both are **one track's** lanes,
//! because a track is a node and a node is something a command can be
//! given (§234).
//!
//! **The tree's fade is to the playhead and this tree's is to a
//! place.** The tree's "Fade to Start" and "Fade to End" fade the clip
//! under the playhead up to it or down from it. Here the place is
//! **given** — a place is a fact and the playhead is a node (§235).
//!
//! **The tree's time is a selection and this tree's is a place.** The
//! tree can insert time at a place in the selection or cut time out of
//! it, and every track of the session moves. Here the place or the
//! range is **given** and the whole of the document is the thing that
//! moves (§236).
//!
//! **The tree's strip of silence has three modes and a preview, and so
//! does this one.** The tree's editor strips the silence, or extracts
//! the sound and keeps the silence, or separates the clip at the edges
//! of the loud parts and keeps everything — and it has a preview that
//! finds the silence and says where the parts are without changing
//! anything. Here the mode is a **name** and a name this battery does
//! not know is **refused**, and the preview is a **read**: it says
//! the same thing and it does not change the document.
//!
//! **The tree clamps a pitch and this tree refuses.** The tree's
//! transpose pushes a note past the top of the range onto a hundred
//! and twenty-seven, and a note asked for as a hundred and thirty is
//! a note the caller does not know about. Here the transpose is
//! **refused** and no note is moved — the same rule the gain follows
//! (§201).
//!
//! **The quantise is onto the session's grid.** The tree's quantise
//! takes a grid of its own, in ticks; here the grid is the session's
//! (§200) and it is musical, so the lines move with the tempo and
//! follow the meter.
//!
//! **The tree's track has a voice and this tree's has one too.** The
//! tree's track header says "dyn" or "off", and "off" is an
//! **inactive** track: it is not heard, and it is still in the
//! document. Here the voice is a **field of the track** and a command
//! turns it off and on again, and the tree's **hidden** is not here:
//! this tree draws no track headers, and a thing that nothing draws is
//! not a fact.
//!
//! **The tree has a command for the tempo and not for the meter and
//! not for the key.** The rule is the same for all three — the event
//! at the first tick is the document's own and it stays — and a map
//! that can only grow is not a map, so all three are here. The tree's
//! removal says how many it removed and answers with zero when there
//! was nothing there; here a tick with no event is refused.
//!
//! **The tree's export renders the track and this one's renders the
//! track too.** The tree's `export_clips` renders the track over the
//! clip's range — the clips of the track, with their gains and their
//! fades, and without the track's strip — and writes that. It is the
//! same sound as the consolidate (§242) and the same sound as the
//! commit's file (§249): the sound of the clips and not the sound of
//! the track.
//!
//! **The tree's duplicate is a copy of a value and this one's is a
//! copy of what the track holds.** The tree's track is a value in a
//! list, and the duplicate clones it and gives the clips new ids; here
//! a track is a node with children — takes, clips, lanes, inserts,
//! sends — and the relations that go with them, and the copy is a copy
//! of the children, with the relation from a clip to its source made
//! again for the new clip.
//!
//! **The tree's send has a command for its level and this one had
//! none.** The tree's `mix.send_level` sets the level of a send; here
//! the level could only be given when the send was made, so a send's
//! level was a fact that could never change — and a fact that can
//! never change is not a fact about a send, it is a fact about the
//! moment the send was made. The tree clamps a level outside its range
//! to the edge, and here it is refused.
//!
//! **The tree's names are unique and this tree's are not.** The tree's
//! rename refuses a name that another track already has, because the
//! tree's tracks are named and a caller names them. Here the id is the
//! identity and the name is a label, and two tracks may carry the same
//! name. The tree also throws the end of a long name away — sixty-four
//! characters for a track, in its own command — and here a name that is
//! longer than the bound is **refused**.
//!
//! **The tree's clipboard carries the automation and this one's did
//! not.** The tree's copy gathers the breakpoints of every lane of the
//! track that fall inside the range and puts them in the clipboard
//! with the clips; the port of the clipboard left them out, and the
//! reason it gave — that a clipboard carrying them would carry a
//! second truth about the same timeline — was wrong. The lanes *are*
//! the automation, and a copy of a lane's breakpoints is a copy of the
//! curve and not a second curve. The truth is that the tree's
//! clipboard carries the **markers** as well, and those are nodes in
//! the document, and a copy of those would be a second truth.
//!
//! **The tree's insert lives in a slot and this tree's is a node.**
//! The tree's track carries a fixed ladder of slots and an insert is
//! at an index, and taking an insert away empties its slot; here an
//! insert is a node under the thing it is on, the chain is the order
//! of the children, and a taken-away insert leaves nothing behind.
//!
//! **The tree's commit renders two seconds more than the material.**
//! The tree's commit renders a track over a range that ends two
//! seconds after its last clip, so that the tail of an insert is in
//! the file. Here the mix is as long as the material, so the tail past
//! the last clip is not in the committed file. The tree's commit also
//! **hides** the old track: the SDK has a hidden flag, and the flag is
//! about drawing, and this battery draws nothing.
//!
//! **The tick of a note is the kit's tick.** The MIDI library counts
//! in nine hundred and sixty ticks a quarter note, and so does the
//! sound battery's tempo map (§19) — so a note's place in a clip and
//! a clip's place on the timeline are two readings of the same scale,
//! and the length of a MIDI clip is worked out from its notes at the
//! tempo of the document.
//!
//! # What this does not do
//!
//! - **It draws nothing.** A sound document is not a picture. v2 has no
//!   timeline and no camera, and a plugin that drew one would be the
//!   second set of camera controls in the tree. The kinds say what they
//!   are; the drawing is a question for the tree.
//! - **It makes no sound.** There is no audio thread and no real-time
//!   door: a command in v2 runs on the caller's thread, which is the wrong
//!   thread for a block of audio.
//! - **It reads no media.** A source is a **reference**: a path, a channel
//!   count and a length. The audio itself is not in the document, and
//!   nothing here decodes it.
//! - **The grid is musical and the tree's is not only that.** The tree's
//!   grid can be a number of seconds, a number of frames or a number of
//!   samples as well as a division of the bar. Here a grid is a
//!   division of the bar, because a sound document is a bar of beats and
//!   a grid in it is a musical thing.
//! - **The mix has the shape of the media.** The tree's session has a
//!   channel format of its own and its mixer builds a matrix from the
//!   source's channels to the destination's: a mono sound is *placed*
//!   in a stereo field and a stereo sound is folded down into a mono
//!   one. Here the mix has the shape of the media — a document of mono
//!   sources is a mono mix — so there is no field for a mono sound to
//!   be placed in, and the pan of a mono track does nothing. The
//!   matrix is a redesign of the mixer and not an addition to it.
//! - **A MIDI clip is not sound.** There is no synthesiser here: a
//!   MIDI clip has no source, and the render leaves it alone — so it
//!   is silent, and it is silent **honestly**: what it holds is the
//!   notes, and the notes are not a sound until something makes them
//!   one.
//! - **A file's tempo is not the document's tempo.** A MIDI file may
//!   carry a tempo of its own, and the import reads the notes at the
//!   tempo the **document** says: a second tempo map would be a second
//!   truth about the same timeline.
//! - **A scheme has no id of its own.** The tree's scheme carries an id
//!   "to tell this scheme from another one under the same key after an
//!   undo or in another session, for pastes from the clipboard". The
//!   clipboard is here now (§247), and the schemes it holds
//!   are told apart by their **nodes** and not by an id of their own:
//!   a node is the thing, and the tree's id is a key under a key.
//! - **A big shift loses the top of the sound.** The pitch shift is
//!   made by resampling the sound as if it were played faster, and a
//!   resample takes away what was above the new half of the rate: an
//!   octave up is made from a sound whose top is gone. The tree's
//!   shift has the same loss, for the same reason, and a shift that
//!   keeps every harmonic is a different piece of work.
//! - **Every elastic algorithm is one method.** The tree's algorithms —
//!   `polyphonic`, `rhythmic`, `monophonic`, `x-form` — are four ways of
//!   keeping the pitch, and each of them is a different piece of work. Here
//!   the algorithm is **named and kept**, and every one of them is
//!   rendered with the one method this has: waveform-similarity
//!   overlap-add. A third party may name an algorithm, and the name is
//!   the fact; what plays is the method, and it is the same for all of
//!   them.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel, Tool};

// The mixer: buses, routes, inserts, sends and automation lanes. It is
// the part of a sound document that is a **graph** and not a tree, so it
// is written in relations.
mod automate;
mod mixer;

// The **beat** of a clip: where the beat falls inside it.
mod beat;

// The **locks** on a clip: what may not change, and what may not move.
mod locks;

// The **marks** on a clip: what a person has said about it.
mod marks;

// The **auto rename** of a clip: the names editing gives the pieces it
// separates.
mod autoname;

// The **group** a clip is in: a set of clips that belong together.
mod group;

// The **grid**: the lines a clip is put on.
mod grid;

// The **gain** of a clip: how loud it is.
mod gain;

// The **conform** of a clip: making the material fit the tempo.
mod conform;

// The **MIDI** of a clip: the notes it holds.
mod midi;

// The **healing** of a track: the clips that were separated are put
// back together.
mod clipboard;
mod commit;
mod duplicate;
mod heal;
mod inactive;
mod keys;
mod ranges;
mod remove;
mod rename;
mod render;
mod repeat;
mod split;
mod time;

use std::collections::HashMap;

pub use autoname::{ReadAutoName, SetAutoName};
pub use beat::{ReadBeat, RemoveBeat, SetBeat};
pub use clipboard::{CopyRange, CutRange, PasteRange, ReadClipboard};
pub use commit::CommitTrack;
pub use conform::Conform;
use craftkit_sdk::document::{Path, Rect, Segment, SubPath};
use craftkit_sdk::{
    Bool, Canvas, Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Point, Session, Text,
};
pub use duplicate::DuplicateTrack;
pub use gain::{BypassGain, SetGain};
pub use grid::{Quantize, ReadGrid, SetGrid};
pub use group::{Group, GroupKind, ReadGroup, SetGroupFlags, SetTrackGroup, Ungroup, UngroupTracks};
pub use heal::HealTrack;
pub use inactive::SetTrackInactive;
pub use keys::{KeyKind, ReadKeys, SetKey};
pub use locks::{EditLock, TimeLock};
pub use marks::SetRating;
pub use midi::{
    DurationMidi, ExportMidi, ImportMidi, ImportMidiTracks, QuantizeMidi, ReadMidi, ReadMidiNames,
    TransposeMidi, VelocityMidi,
};
pub use mixer::{
    AddBus, AddInsert, AddLane, AddRoute, AddSend, BusKind, BusPayload, InsertKind, InsertPayload, LaneKind,
    LanePayload, MoveInsert, RemoveInsert, SendKind, SendPayload, SetSend,
};
pub use remove::{RemoveClip, RemoveMarker, RemoveTrack};
pub use rename::{RenameClip, RenameTrack};
pub use render::RenderClip;
pub use repeat::RepeatRange;
pub use split::SplitMono;
pub use time::{CutTime, InsertTime};

use craftkit_sdk::serde_json;
use craftkit_sdk::serde_json::Value as Json;

// The time model: a library, not a plugin.
use craftkit_sound_time::{Grid, MAX_POSITION, NoteValue, TempoMap};

mod timeline;

/// The sound layer.
///
/// It needs the core, so it says so, and the ecosystem checks the graph
/// when the program starts.
#[derive(Layer)]
#[layer(id = "sound", name = "Sound", requires("core"))]
pub struct SoundLayer;

// ------------------------------------------------------------------ the session --

/// The sound session's own settings.
///
/// They belong to the **session**, not to any one node: the sample rate is
/// the rate of every clip in the document, and the timecode is where the
/// document starts on a clock. The document's payload is where they live,
/// under one name, so that another plugin cannot collide with them by
/// accident.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundSession {
    /// The sample rate, in samples a second.
    pub sample_rate: u32,
    /// Where the document starts on a timecode, in samples.
    pub timecode_start: i64,
    /// Whether the **metronome** is heard.
    ///
    /// The click is mixed in after everything else, so it reaches the
    /// output without passing through a track, a send or an insert.
    pub metronome: bool,
    /// How loud the click is, in decibels.
    pub click_volume_db: f64,
    /// The click's pitch, as a note number.
    pub click_note: f64,
    /// The **accent's** pitch: the click on the first beat of a bar.
    pub click_accent_note: f64,
    /// The **grid**: the lines a clip is put on.
    ///
    /// The grid is a setting of the session and not of the editor: it
    /// is in the document, so a file says what grid it was made on.
    pub grid: Grid,
}

impl Default for SoundSession {
    fn default() -> Self {
        SoundSession {
            sample_rate: 48000,
            timecode_start: 0,
            metronome: false,
            click_volume_db: -6.0,
            click_note: 72.0,
            click_accent_note: 84.0,
            grid: Grid::default(),
        }
    }
}

/// The name the sound battery keeps its session settings under.
const SESSION_KEY: &str = "sound";

/// The session's settings, as the document holds them.
pub(crate) fn session_of(document: &Document) -> SoundSession {
    let settings = document.data.get(SESSION_KEY);
    let sample_rate = settings
        .and_then(|settings| settings.get("sample_rate"))
        .and_then(|rate| rate.as_u64())
        .and_then(|rate| u32::try_from(rate).ok())
        .filter(|rate| *rate > 0)
        .unwrap_or(SoundSession::default().sample_rate);
    let timecode_start = settings
        .and_then(|settings| settings.get("timecode_start"))
        .and_then(|start| start.as_i64())
        .unwrap_or(SoundSession::default().timecode_start);
    let metronome = settings
        .and_then(|settings| settings.get("metronome"))
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false);
    let click_volume_db = settings
        .and_then(|settings| settings.get("click_volume_db"))
        .and_then(|volume| volume.as_f64())
        .filter(|volume| volume.is_finite())
        .unwrap_or(SoundSession::default().click_volume_db);
    let click_note = settings
        .and_then(|settings| settings.get("click_note"))
        .and_then(|note| note.as_f64())
        .filter(|note| note.is_finite())
        .unwrap_or(SoundSession::default().click_note);
    let click_accent_note = settings
        .and_then(|settings| settings.get("click_accent_note"))
        .and_then(|note| note.as_f64())
        .filter(|note| note.is_finite())
        .unwrap_or(SoundSession::default().click_accent_note);
    // The **grid**: a note value and two flags. A grid that is not one
    // this tree knows is the default one, the way a sample rate that
    // makes no sense is the default one.
    let grid = match settings.and_then(|settings| settings.get("grid")) {
        Some(grid) => Grid {
            value: grid
                .get("value")
                .and_then(|value| value.as_str())
                .and_then(NoteValue::parse)
                .unwrap_or(SoundSession::default().grid.value),
            dotted: grid.get("dotted").and_then(|dotted| dotted.as_bool()).unwrap_or(false),
            triplet: grid.get("triplet").and_then(|triplet| triplet.as_bool()).unwrap_or(false),
        },
        None => SoundSession::default().grid,
    };
    SoundSession {
        sample_rate,
        timecode_start,
        metronome,
        click_volume_db,
        click_note,
        click_accent_note,
        grid,
    }
}

/// The clip ids a command was given.
///
/// The list is a list of **numbers**: an id that is not a number is not
/// an id, and a command that took it and looked it up would be a
/// command looking for a node that cannot be named.
pub(crate) fn clip_ids_of(value: &serde_json::Value, id: &str) -> Result<Vec<u64>, EngineError> {
    let list = value.as_array().ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: "the clips are a list of their ids".to_string(),
    })?;
    let mut ids = Vec::with_capacity(list.len());
    for item in list {
        let Some(number) = item.as_u64() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a clip's id is a number".to_string(),
            });
        };
        ids.push(number);
    }
    Ok(ids)
}

/// The **automation** of a range: the breakpoints that fall inside it.
pub(crate) use ranges::automation_of_range;
/// Take the material of a range away.
pub(crate) use ranges::clear_range_of;
/// The **copies** of the material of a range, trimmed to it.
pub(crate) use ranges::copies_of_range;
/// The **range** of a track: where it starts and where it ends.
pub(crate) use ranges::range_of;

/// Put the session's settings back into the document.
pub(crate) fn set_session(document: &mut Document, session: &SoundSession) {
    let mut data = match document.data.as_object() {
        Some(object) => object.clone(),
        None => serde_json::Map::new(),
    };
    data.insert(
        SESSION_KEY.to_string(),
        serde_json::json!({
            "sample_rate": session.sample_rate,
            "timecode_start": session.timecode_start,
            "metronome": session.metronome,
            "click_volume_db": session.click_volume_db,
            "click_note": session.click_note,
            "click_accent_note": session.click_accent_note,
            "grid": serde_json::json!({
                "value": session.grid.value.name(),
                "dotted": session.grid.dotted,
                "triplet": session.grid.triplet,
            }),
        }),
    );
    document.data = Json::Object(data);
}

// -------------------------------------------------------------------- the kinds --

/// The kind of a track.
///
/// The kind of a track — audio, instrument, auxiliary, master — is a
/// **string in the track's payload**, not a variant in the core and not a
/// variant in this plugin. The tree this ports has eight variants in its
/// model and a match on them in the mixer; here a track says what it is,
/// and a third party can say something the core has never heard of.
#[derive(NodeKind)]
#[node_kind(kind = "sound.track", label = "Track", layer = "sound", evaluates, place)]
pub struct TrackKind;

impl TrackKind {
    /// How a track is drawn.
    ///
    /// The **row**: a line along the top of the track's band, so that
    /// the timeline shows where one track ends and the next begins.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::track_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x + rect.w, rect.y),
        ]);
        let stroke = craftkit_sdk::document::Stroke {
            colour: craftkit_sdk::document::Colour::BLACK,
            width: 0.01,
            ..Default::default()
        };
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(stroke),
        );
        // A **soloed** track is a double rule: the row says which
        // track is heard alone.
        if track_is_soloed(document, node.id) {
            let under = craftkit_sdk::document::Path::polygon(&[
                Point::new(rect.x, rect.y + 0.05),
                Point::new(rect.x + rect.w, rect.y + 0.05),
            ]);
            canvas.path(
                &under,
                craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
                craftkit_sdk::document::BlendMode::Normal,
                Some(stroke),
            );
        }
    }

    /// Whether a point is inside a track.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::track_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a track **is**: its row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::track_rect(document, node)
    }

    /// What a track **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let track = TrackPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "track_kind": track.track_kind,
            "channels": track.channels,
            "playlists": node.children.len(),
        }))
    }
}

/// The kind of a playlist.
///
/// A track holds **playlists**: the alternative takes of the same track.
/// The tree this ports holds them as a list on the track with an index
/// saying which one is playing; here a playlist is a node, because a node
/// can hold other nodes, and the alternative takes are the clips under
/// it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.playlist", label = "Playlist", layer = "sound", evaluates, place)]
pub struct PlaylistKind;

impl PlaylistKind {
    /// How a playlist is drawn.
    ///
    /// **Nothing.** A playlist is the track's row with a different set of
    /// clips on it, and the row is already drawn by the track.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a playlist.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::playlist_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a playlist **is**: the row of the track it is under.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::playlist_rect(document, node)
    }

    /// What a playlist **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(serde_json::json!({ "clips": node.children.len() }))
    }
}

/// The kind of a clip.
///
/// A clip is a **span of time** with **content** in it. The content is a
/// field of the payload: an audio clip points at a source and an offset
/// into it, and a MIDI clip would hold notes — which the tree this ports
/// keeps in a closed enum of three kinds, matched in the mixer.
#[derive(NodeKind)]
#[node_kind(kind = "sound.clip", label = "Clip", layer = "sound", evaluates, place)]
pub struct ClipKind;

impl ClipKind {
    /// How a clip is drawn: a **bar** on the track's row.
    ///
    /// The bar is the clip's **span**: where it starts and how long it
    /// is, in the world's seconds. A muted clip is drawn as nothing,
    /// which is what a muted clip is.
    ///
    /// The **fades** are notches in the bar: the quiet part of a clip
    /// is the paper showing through the ink, and the notch is the
    /// fade's own curve.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::clip_rect(document, node) else { return };
        let fill = if node.muted {
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE)
        } else {
            document
                .appearance_of(node)
                .map(|appearance| appearance.fill)
                .unwrap_or(craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::BLACK))
        };
        canvas.rect(rect, fill, craftkit_sdk::document::BlendMode::Normal);
        if node.muted {
            return;
        }
        let Ok(payload) = ClipPayload::from_json(&node.data) else { return };
        let rate = session_of(document).sample_rate as f64;
        if rate <= 0.0 {
            return;
        }
        let paper = craftkit_sdk::document::Colour::WHITE;
        // The **waveform**: the shape of the sound the clip plays, drawn
        // on the bar in the paper's colour, the way the fade's notch is.
        for path in waveform_of(document, node, &payload, &rect) {
            canvas.path(
                &path,
                craftkit_sdk::document::Fill::Solid(paper),
                craftkit_sdk::document::BlendMode::Normal,
                None,
            );
        }
        let width = payload.fade_in.len as f64 / rate;
        if width > 0.0
            && width < rect.w
            && let Some(path) = fade_in_path(&rect, width, payload.fade_in.fade())
        {
            canvas.path(
                &path,
                craftkit_sdk::document::Fill::Solid(paper),
                craftkit_sdk::document::BlendMode::Normal,
                None,
            );
        }
        let width = payload.fade_out.len as f64 / rate;
        if width > 0.0
            && width < rect.w
            && let Some(path) = fade_out_path(&rect, width, payload.fade_out.fade())
        {
            canvas.path(
                &path,
                craftkit_sdk::document::Fill::Solid(paper),
                craftkit_sdk::document::BlendMode::Normal,
                None,
            );
        }
    }

    /// Whether a point is inside a clip.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::clip_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a clip **is**: a bar on the track's row.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::clip_rect(document, node)
    }

    /// What a clip **means**, in the plan.
    ///
    /// The **group** is derived: it is the edge the clip carries, and
    /// not a field of the clip's own.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let clip = ClipPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "start": clip.start,
            "length": clip.length,
            "end": clip.start.saturating_add(clip.length),
            "source": clip.source,
            "gain_db": clip.gain_db,
            "fade_in": clip.fade_in.to_json(),
            "fade_out": clip.fade_out.to_json(),
            "sync": clip.sync,
            "rating": clip.rating,
            "gain_bypassed": clip.gain_bypassed,
            "source_bpm": clip.source_bpm,
            "group": group::group_of(document, node.id),
        }))
    }
}

/// The kind of a tempo event.
#[derive(NodeKind)]
#[node_kind(kind = "sound.tempo", label = "Tempo", layer = "sound", evaluates, place)]
pub struct TempoKind;

impl TempoKind {
    /// How a tempo event is drawn: a **mark in the ruler**.
    ///
    /// The ruler is the row **above** the tracks, and it is where the
    /// things that are not on a track live: the tempo and the meter.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::event_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a tempo event.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::event_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a tempo event **is**: a mark in the ruler.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::event_rect(document, node)
    }

    /// What a tempo event **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let tempo = TempoPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({ "tick": tempo.tick, "bpm": tempo.bpm }))
    }
}

/// The kind of a meter event.
#[derive(NodeKind)]
#[node_kind(kind = "sound.meter", label = "Meter", layer = "sound", evaluates, place)]
pub struct MeterKind;

impl MeterKind {
    /// How a meter event is drawn: a **mark in the ruler**.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::event_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a meter event.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::event_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a meter event **is**: a mark in the ruler.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::event_rect(document, node)
    }

    /// What a meter event **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let meter = MeterPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "tick": meter.tick,
            "numerator": meter.numerator,
            "denominator": meter.denominator,
        }))
    }
}

/// The kind of a marker.
#[derive(NodeKind)]
#[node_kind(kind = "sound.marker", label = "Marker", layer = "sound", evaluates, place)]
pub struct MarkerKind;

impl MarkerKind {
    /// How a marker is drawn: a **line** down the timeline.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::marker_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x, rect.y),
            Point::new(rect.x, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is inside a marker.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::marker_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where a marker **is**: a line at the time it is at.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::marker_rect(document, node)
    }

    /// What a marker **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let marker = MarkerPayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({ "start": marker.start, "end": marker.end }))
    }
}

/// The kind of a **ruler**: the row above the tracks, and its scale.
///
/// The ruler is a **node** because the ruler has a **setting**: the
/// scale it counts by. A setting is a thing in a document, and a node is
/// where settings live — the same rule as the measure of a paragraph
/// (§58) and the appearance of a node (§95).
///
/// The ruler's **marks** are not the ruler's: the tempo and the meter
/// are drawn in the ruler's row by their own kinds (§126), and the ruler
/// draws the **numbers**.
#[derive(NodeKind)]
#[node_kind(kind = "sound.ruler", label = "Ruler", layer = "sound", evaluates, place)]
pub struct RulerKind;

impl RulerKind {
    /// How a ruler is drawn: its **numbers**.
    ///
    /// The numbers are the kit's own letters (§128): the tree has no
    /// font file and no asset of any kind, and a number is a word of one
    /// or two letters.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let marks = timeline::scale_marks(document, node);
        let font = craftkit_font::Font::builtin();
        let Some(metrics) = craftkit_font::Metrics::new(timeline::SCALE_EM) else { return };
        let unit = metrics.unit();
        for mark in marks {
            let mut pen = mark.x;
            for character in mark.label.chars() {
                let Some(glyph) = font.glyph(character) else { continue };
                for stroke in &glyph.strokes {
                    if stroke.len() < 2 {
                        continue;
                    }
                    let mut segments = Vec::with_capacity(stroke.len() - 1);
                    for point in stroke.iter().skip(1) {
                        segments.push(craftkit_sdk::document::Segment::Line {
                            to: Point::new(pen + point[0] * unit, baseline() + point[1] * unit),
                        });
                    }
                    let start = stroke[0];
                    let path = craftkit_sdk::document::Path::single(craftkit_sdk::document::SubPath {
                        start: Point::new(pen + start[0] * unit, baseline() + start[1] * unit),
                        segments,
                        closed: false,
                    });
                    canvas.path(
                        &path,
                        craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
                        craftkit_sdk::document::BlendMode::Normal,
                        Some(craftkit_sdk::document::Stroke {
                            colour: craftkit_sdk::document::Colour::BLACK,
                            width: 0.01,
                            ..Default::default()
                        }),
                    );
                }
                pen += glyph.advance * unit;
            }
        }
    }

    /// Whether a point is on a **number**.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        let marks = timeline::scale_marks(document, node);
        let font = craftkit_font::Font::builtin();
        let Some(metrics) = craftkit_font::Metrics::new(timeline::SCALE_EM) else { return false };
        let unit = metrics.unit();
        marks.iter().any(|mark| {
            let width: f64 = mark
                .label
                .chars()
                .filter_map(|character| font.glyph(character))
                .map(|glyph| glyph.advance * unit)
                .sum();
            let box_of = craftkit_sdk::document::Rect::new(mark.x, baseline(), width, metrics.em);
            box_of.contains(point)
        })
    }

    /// Where a ruler **is**: the row above the tracks.
    fn place(_node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        Some(craftkit_sdk::document::Rect::new(
            0.0,
            -timeline::RULER_HEIGHT,
            timeline::length_of(document),
            timeline::RULER_HEIGHT,
        ))
    }

    /// What a ruler **means**: its scale and how many numbers it has.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let marks = timeline::scale_marks(document, node);
        Some(serde_json::json!({
            "scale": node.data.get("scale").and_then(|value| value.as_str()).unwrap_or("bars"),
            "numbers": marks.len(),
        }))
    }
}

/// The **baseline** of the ruler's numbers: the bottom of the ruler's
/// row.
///
/// The numbers stand on the bottom of the row and rise into it, so that
/// the marks of the tempo and the meter are above them.
fn baseline() -> f64 {
    -timeline::RULER_HEIGHT
}

/// Put a **ruler** in the document: the row above the tracks, and the
/// scale it counts by.
#[derive(Command)]
#[command(
    id = "sound.ruler.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Ruler"),
    mutates,
    undoable
)]
pub struct AddRuler {
    /// What to call it.
    pub name: Text,
    /// The scale the ruler counts by: `bars` or `seconds`.
    pub scale: Option<Text>,
}

impl AddRuler {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let scale = match &self.scale {
            Some(scale) => {
                let scale = scale.0.as_str();
                if scale != "bars" && scale != "seconds" {
                    return Err(EngineError::Failed {
                        id: "sound.ruler.add".to_string(),
                        message: format!("a ruler counts by bars or by seconds, and not by `{scale}`"),
                    });
                }
                scale.to_string()
            }
            None => "bars".to_string(),
        };
        let name = self.name.0.clone();
        let id = session.document_mut()?.add_node_with(
            name.clone(),
            "sound.ruler",
            None,
            serde_json::json!({ "scale": scale }),
        );
        Ok(Outcome::changed(format!("added a ruler `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "scale": scale })))
    }
}

/// The kind of a source.
///
/// A source is a **reference** to media: where it is, how many channels it
/// has, how long it is. The audio itself is not in the document, and the
/// tree this ports keeps it in a pool of shared buffers beside the
/// document rather than in it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.source", label = "Source", layer = "sound", evaluates)]
pub struct SourceKind;

impl SourceKind {
    /// How a source is drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn Canvas) {}

    /// Whether a point is inside a source.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a source **means**, in the plan.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let source = SourcePayload::from_json(&node.data).ok()?;
        Some(serde_json::json!({
            "path": source.path,
            "channels": source.channels,
            "frames": source.frames,
            "sample_rate": source.sample_rate,
        }))
    }
}

// ------------------------------------------------------------------- the payloads --

/// What a track says about itself.
#[derive(Debug, Clone, PartialEq)]
struct TrackPayload {
    /// What kind of track it is: `audio`, `instrument`, `aux`, `master`, …
    ///
    /// A **string**, and not a variant: the kinds of a track are open.
    track_kind: String,
    /// How many channels it has.
    channels: u32,
    /// Whether the track is **soloed**.
    ///
    /// A soloed track silences the others: when one track is soloed,
    /// the rest are not heard.
    solo: bool,
    /// The track's **trim**, in decibels: the gain of the track's
    /// input, before its inserts.
    ///
    /// The tree's trim runs from minus one hundred and forty-four
    /// decibels to twenty-four; it is the level of the input and not
    /// the level of the output.
    trim_db: f64,
    /// Whether the track is **solo safe**.
    ///
    /// A solo silences the other tracks, and a solo-safe track is not
    /// silenced by it: it is a track that stays when somebody listens
    /// to one thing on purpose.
    solo_safe: bool,
    /// Whether the track is **muted**.
    ///
    /// The tree's mixer keeps a mute of its own beside the mute lane,
    /// and here the lane is the mute's default: a lane with
    /// breakpoints overrides it, and a lane without one is the mute.
    mute: bool,
    /// The track's **pan**, between minus one and one: the balance of
    /// the track's sound in the mix.
    ///
    /// The tree's mixer keeps a pan per channel — a stereo track has
    /// two — and here a track has one: it balances the sound, and the
    /// middle is the middle.
    pan: f64,
    /// The track's **fader**, in decibels: the level of the track's
    /// output, after its inserts.
    ///
    /// The tree's fader runs from minus one hundred and forty-four
    /// decibels to twelve, and it is the **default** of the volume
    /// lane: a lane with breakpoints overrides it, and a lane without
    /// one is the fader.
    fader_db: f64,
    /// Whether the track's **phase is inverted**.
    ///
    /// The phase is a sign and not a level: the sound is the same and
    /// upside down.
    phase_invert: bool,
    /// The name of the **take** that is heard.
    ///
    /// A track may have several takes — the tree's playlists — and one
    /// of them is the one that is played. A track with no name here
    /// plays its first take.
    active_playlist: Option<String>,
    /// Whether the track's **voice is off**: whether the track is
    /// inactive and is not heard.
    ///
    /// The tree's track header says "dyn" or "off", and "off" is an
    /// inactive track: it is not in the mix, not on a bus and not a
    /// stem, and it is still in the document with its clips and its
    /// settings.
    inactive: bool,
    /// The **elastic algorithm** of the track: how its clips are
    /// stretched.
    ///
    /// The tree's track carries an elastic field with an algorithm in
    /// it — `polyphonic`, `rhythmic`, `monophonic`, `varispeed`,
    /// `x-form` — and a track with no algorithm is not an elastic
    /// track. Here the algorithm is a **string** and not a variant, so
    /// a third party may name one, and the names are the tree's.
    ///
    /// The algorithm `varispeed` says that the track's clips are played
    /// at another speed and the **pitch falls with it** — which is
    /// what every track does when it has no algorithm at all.
    elastic: Option<String>,
}

impl TrackPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "track_kind": self.track_kind,
            "channels": self.channels,
            "solo": self.solo,
            "trim_db": self.trim_db,
            "fader_db": self.fader_db,
            "pan": self.pan,
            "mute": self.mute,
            "solo_safe": self.solo_safe,
            "phase_invert": self.phase_invert,
            "active_playlist": self.active_playlist,
            "inactive": self.inactive,
            "elastic": self.elastic,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(TrackPayload {
            track_kind: value.get("track_kind").and_then(|kind| kind.as_str()).unwrap_or("audio").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
            solo: value.get("solo").and_then(|solo| solo.as_bool()).unwrap_or(false),
            trim_db: value.get("trim_db").and_then(|trim| trim.as_f64()).unwrap_or(0.0),
            fader_db: value.get("fader_db").and_then(|fader| fader.as_f64()).unwrap_or(0.0),
            pan: value.get("pan").and_then(|pan| pan.as_f64()).unwrap_or(0.0),
            mute: value.get("mute").and_then(|mute| mute.as_bool()).unwrap_or(false),
            solo_safe: value.get("solo_safe").and_then(|safe| safe.as_bool()).unwrap_or(false),
            phase_invert: value.get("phase_invert").and_then(|phase| phase.as_bool()).unwrap_or(false),
            active_playlist: value
                .get("active_playlist")
                .and_then(|playlist| playlist.as_str())
                .map(|playlist| playlist.to_string()),
            inactive: value.get("inactive").and_then(|inactive| inactive.as_bool()).unwrap_or(false),
            elastic: value
                .get("elastic")
                .and_then(|elastic| elastic.as_str())
                .map(|elastic| elastic.to_string()),
        })
    }
}

/// The **take** that is heard: the track's active playlist, or the
/// first one it has.
///
/// A track may have several takes and one of them is played; a track
/// whose active take is not there plays its first one, so that a track
/// is never silent by accident.
fn active_playlist(_document: &Document, track: &Node) -> Option<u64> {
    let playlists: Vec<&Node> =
        track.children.iter().filter(|child| child.kind == "sound.playlist").map(|child| &**child).collect();
    let wanted = TrackPayload::from_json(&track.data).ok().and_then(|payload| payload.active_playlist);
    match wanted {
        Some(name) => playlists
            .iter()
            .find(|playlist| playlist.name == name)
            .map(|playlist| playlist.id)
            .or_else(|| playlists.first().map(|playlist| playlist.id)),
        None => playlists.first().map(|playlist| playlist.id),
    }
}

/// Whether a track is a **master**: a track whose fader is over the
/// whole mix and not over a part of it.
///
/// The kinds of a track are open — they are a string in the track's
/// payload — and `master` is one of them: the tree's mixer runs the
/// master faders over the main mix, and a master holds no clips.
fn is_master(node: &Node) -> bool {
    TrackPayload::from_json(&node.data).map(|payload| payload.track_kind == "master").unwrap_or(false)
}

/// Whether a track is **inactive**: whether its voice is off.
///
/// An inactive track is not heard — not in the mix, not on a bus and
/// not as a stem — and it is still in the document.
fn is_inactive(node: &Node) -> bool {
    TrackPayload::from_json(&node.data).map(|payload| payload.inactive).unwrap_or(false)
}

/// Whether a track's clips are **stretched with the pitch kept**.
///
/// The tree's track carries an elastic algorithm, and every algorithm
/// but `varispeed` says that the clip's stretch is rendered
/// pitch-preserving. A track with no algorithm is not an elastic track
/// at all, and its clips are played at another speed and the pitch
/// falls with it.
fn is_elastic(node: &Node) -> bool {
    TrackPayload::from_json(&node.data)
        .ok()
        .and_then(|payload| payload.elastic)
        .map(|algorithm| algorithm != "varispeed")
        .unwrap_or(false)
}

/// The elastic algorithm of a track, if it has one.
fn elastic_algorithm(node: &Node) -> Option<String> {
    TrackPayload::from_json(&node.data).ok().and_then(|payload| payload.elastic)
}

/// The **least** and the **greatest** stretch a pitch-preserving
/// stretch may have.
///
/// The tree's own stretch refuses a ratio outside `0.1..10`: a stretch
/// of a hundredth would ask for a frame every hundredth of a window,
/// and the sound would be a noise. The bound is here and not in the
/// method, because a caller that means such a stretch should be told
/// that it cannot have one — and not be handed a different one.
const MIN_ELASTIC_STRETCH: f64 = 0.1;
const MAX_ELASTIC_STRETCH: f64 = 10.0;

/// What a **fade** says about itself.
///
/// A fade is a length, a shape and a bend, and it is the same thing at
/// both ends of a clip: the head fades in and the tail fades out.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FadePayload {
    /// How long it is, in samples. Zero is no fade at all.
    len: i64,
    /// The curve it follows.
    shape: craftkit_audio::FadeShape,
    /// How much the curve is bent, from -1 to 1.
    curvature: f32,
}

impl FadePayload {
    /// No fade at all.
    fn none() -> FadePayload {
        FadePayload { len: 0, shape: craftkit_audio::FadeShape::default(), curvature: 0.0 }
    }

    fn to_json(self) -> Json {
        serde_json::json!({
            "len": self.len,
            "shape": self.shape.label(),
            "curvature": self.curvature,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let len = value.get("len").and_then(|len| len.as_i64()).unwrap_or(0);
        if len < 0 {
            return Err("a fade does not run backwards".to_string());
        }
        let shape = match value.get("shape").and_then(|shape| shape.as_str()) {
            Some(label) => craftkit_audio::FadeShape::from_label(label)
                .ok_or_else(|| format!("there is no fade shape called `{label}`"))?,
            None => craftkit_audio::FadeShape::default(),
        };
        let curvature = value.get("curvature").and_then(|curvature| curvature.as_f64()).unwrap_or(0.0);
        if !curvature.is_finite() {
            return Err("a bend is a number".to_string());
        }
        Ok(FadePayload { len, shape, curvature: curvature as f32 })
    }

    /// The fade itself, as the audio crate has it.
    fn fade(&self) -> craftkit_audio::Fade {
        craftkit_audio::Fade { len: self.len.max(0) as u64, shape: self.shape, curvature: self.curvature }
    }
}

/// What a clip says about itself.
///
/// Every number here is in the **session's samples**: the clip's place
/// on the timeline, how long it is there, and how far into its source
/// it starts. The source has its own rate, and the render works out
/// the difference; the document speaks the timeline's language.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClipPayload {
    /// Where it starts, in samples.
    pub(crate) start: i64,
    /// How long it is, in samples.
    pub(crate) length: i64,
    /// The source it plays, if it plays one.
    source: Option<u64>,
    /// How far into the source it starts, in samples.
    offset: i64,
    /// Its gain, in decibels.
    gain_db: f64,
    /// The fade at its head.
    fade_in: FadePayload,
    /// The fade at its tail.
    fade_out: FadePayload,
    /// How much **longer** it is played than the source it plays.
    ///
    /// One is none. Two is twice as long and an octave lower, because
    /// the pitch follows the speed.
    stretch: f64,
    /// The **pitch** of the clip, in **semitones**.
    ///
    /// Zero is the pitch of the source, twelve semitones are an octave
    /// up, and the clip keeps its length whatever the pitch does.
    pitch_semitones: f64,
    /// The **processes** on the clip's sound, in the order they are
    /// done.
    processes: Vec<ProcessPayload>,
    /// Whether the clip is **muted**.
    ///
    /// A muted clip is not played and is still there: the tree's
    /// renderer skips it and the tree's editor keeps it.
    muted: bool,
    /// The clip's **sync point**, in samples from the clip's start.
    ///
    /// The sync point is the place inside the clip where the beat
    /// falls: it is what the clip is lined up by, and it is why a clip
    /// that was recorded against another one still lines up.
    ///
    /// It is an **option**, and that is the difference from the tree:
    /// the tree's sync point is a number where zero means *none*, and a
    /// clip whose sync point is at its very first sample cannot say so.
    /// Here the two are different: `None` is no sync point, and a
    /// sync point of zero is a sync point at the start.
    pub(crate) sync: Option<i64>,
    /// Whether the clip's **content** is locked.
    ///
    /// A locked clip's sound is finished: it is not trimmed, not faded
    /// and not stretched.
    pub(crate) edit_locked: bool,
    /// Whether the clip's **place** is locked.
    ///
    /// A locked clip stays where it is: it is not spaced and not
    /// snapped.
    pub(crate) time_locked: bool,
    /// How many **stars** the clip is rated.
    ///
    /// From none to five. Zero is a rating — the clip has none — and
    /// not the absence of one, so every clip carries a rating.
    pub(crate) rating: u8,
    /// Whether the clip's **gain** is bypassed.
    ///
    /// A bypassed clip plays as it is: the gain is not in the path.
    /// The gain is **kept**, so taking the bypass off brings it back.
    pub(crate) gain_bypassed: bool,
    /// The tempo the clip's **material** is at, in beats a minute.
    ///
    /// It is an option: a clip that has never been conformed does not
    /// know what tempo its material is at, and the first conform takes
    /// the tempo of the document where the clip starts.
    pub(crate) source_bpm: Option<f64>,
    /// What the clip's **content** is.
    ///
    /// It is a **string** and not a variant, for the same reason the
    /// kinds of a sound document are strings: the kinds are open here.
    /// `midi` is a clip of notes, and nothing is a clip of sound.
    pub(crate) content: Option<String>,
    /// The **notes** of a MIDI clip.
    ///
    /// It is an option, and it is there only when the content is
    /// `midi`: a clip of sound holds no notes.
    pub(crate) midi: Option<crate::midi::MidiPayload>,
}

/// A **process** on a clip's sound: something done to it on the way
/// out.
///
/// The name is a **string** and not a variant — the kinds of a sound
/// are open here, and so are its processes — and the parameters are
/// whatever the process takes. The ones this battery knows are
/// `reverse`, `invert`, `normalize` and `dc_offset`; a name it does
/// not know is **refused**, because a process that does nothing is
/// worse than no process at all.
#[derive(Debug, Clone, PartialEq)]
struct ProcessPayload {
    /// What the process is called.
    process: String,
    /// What it is given.
    params: Json,
}

impl ProcessPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "process": self.process, "params": self.params })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let process = value.get("process").and_then(|process| process.as_str()).unwrap_or("").to_string();
        if process.is_empty() {
            return Err("a process is a name".to_string());
        }
        Ok(ProcessPayload { process, params: value.get("params").cloned().unwrap_or(Json::Null) })
    }
}

/// The most a clip's pitch may be moved, in **semitones**.
///
/// The tree's pitch shift is a process and its own range is twenty-four
/// semitones either way; a shift of more than that is refused here
/// rather than clamped, because a caller that asked for a hundred
/// semitones would be given twenty-four and would not know.
const MAX_PITCH_SEMITONES: f64 = 24.0;

/// The most a clip may be stretched.
///
/// A stretch is a length ratio, and a clip that is stretched a
/// thousandfold would read a thousandth of a sample of its source and
/// hold it for a thousand times as long. The bound is here so that a
/// number from outside cannot ask for an absurd thing.
const MAX_STRETCH: f64 = 64.0;

impl ClipPayload {
    /// A clip with nothing set on it.
    ///
    /// The defaults are a clip that plays a source at unity, with no
    /// fade, no pitch, no process and no beat.
    pub(crate) fn new() -> ClipPayload {
        ClipPayload {
            start: 0,
            length: 0,
            source: None,
            offset: 0,
            gain_db: 0.0,
            fade_in: FadePayload::none(),
            fade_out: FadePayload::none(),
            stretch: 1.0,
            pitch_semitones: 0.0,
            processes: Vec::new(),
            muted: false,
            sync: None,
            edit_locked: false,
            time_locked: false,
            rating: 0,
            gain_bypassed: false,
            source_bpm: None,
            content: None,
            midi: None,
        }
    }

    fn to_json(&self) -> Json {
        serde_json::json!({
            "start": self.start,
            "length": self.length,
            "source": self.source,
            "offset": self.offset,
            "gain_db": self.gain_db,
            "fade_in": self.fade_in.to_json(),
            "fade_out": self.fade_out.to_json(),
            "stretch": self.stretch,
            "pitch_semitones": self.pitch_semitones,
            "processes": self.processes.iter().map(ProcessPayload::to_json).collect::<Vec<Json>>(),
            "muted": self.muted,
            "sync": self.sync,
            "edit_locked": self.edit_locked,
            "time_locked": self.time_locked,
            "rating": self.rating,
            "gain_bypassed": self.gain_bypassed,
            "source_bpm": self.source_bpm,
            "content": self.content,
            "midi": self.midi.as_ref().map(crate::midi::MidiPayload::to_json),
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        let start = value.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let length = value.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
        if start < 0 || length < 0 {
            return Err("a clip does not start before the beginning and does not run backwards".to_string());
        }
        let stretch = value.get("stretch").and_then(|stretch| stretch.as_f64()).unwrap_or(1.0);
        if !stretch.is_finite() || stretch <= 0.0 || stretch > MAX_STRETCH {
            return Err(format!(
                "a stretch is greater than zero and no more than {MAX_STRETCH}, and this one is {stretch}"
            ));
        }
        let pitch_semitones = value.get("pitch_semitones").and_then(|pitch| pitch.as_f64()).unwrap_or(0.0);
        if !pitch_semitones.is_finite()
            || !(-MAX_PITCH_SEMITONES..=MAX_PITCH_SEMITONES).contains(&pitch_semitones)
        {
            return Err(format!(
                "a pitch is from minus {MAX_PITCH_SEMITONES} semitones to {MAX_PITCH_SEMITONES}, \
                 and this one is {pitch_semitones}"
            ));
        }
        Ok(ClipPayload {
            start,
            length,
            source: value.get("source").and_then(|source| source.as_u64()),
            offset: value.get("offset").and_then(|offset| offset.as_i64()).unwrap_or(0),
            gain_db: value.get("gain_db").and_then(|gain| gain.as_f64()).unwrap_or(0.0),
            fade_in: match value.get("fade_in") {
                Some(fade) => FadePayload::from_json(fade)?,
                None => FadePayload::none(),
            },
            fade_out: match value.get("fade_out") {
                Some(fade) => FadePayload::from_json(fade)?,
                None => FadePayload::none(),
            },
            stretch,
            pitch_semitones,
            processes: match value.get("processes") {
                Some(processes) => {
                    let list = processes.as_array().ok_or("the processes of a clip are a list")?;
                    let mut out = Vec::with_capacity(list.len());
                    for process in list {
                        out.push(ProcessPayload::from_json(process)?);
                    }
                    out
                }
                None => Vec::new(),
            },
            muted: value.get("muted").and_then(|muted| muted.as_bool()).unwrap_or(false),
            sync: value.get("sync").and_then(|sync| sync.as_i64()),
            edit_locked: value.get("edit_locked").and_then(|locked| locked.as_bool()).unwrap_or(false),
            time_locked: value.get("time_locked").and_then(|locked| locked.as_bool()).unwrap_or(false),
            rating: match value.get("rating").and_then(|rating| rating.as_f64()) {
                None => 0,
                Some(rating) => {
                    // A rating read from a file is a number like any
                    // other, and a rating of nine stars is not a
                    // rating: the file is refused rather than
                    // believed.
                    if !rating.is_finite()
                        || rating.fract() != 0.0
                        || rating < 0.0
                        || rating > crate::marks::MAX_RATING as f64
                    {
                        return Err(format!(
                            "a rating is from none to {} stars, and {rating} is not a rating",
                            crate::marks::MAX_RATING
                        ));
                    }
                    rating as u8
                }
            },
            gain_bypassed: value
                .get("gain_bypassed")
                .and_then(|bypassed| bypassed.as_bool())
                .unwrap_or(false),
            // The tempo of the material: a number from outside, and a
            // tempo outside the range is not a tempo.
            source_bpm: match value.get("source_bpm").and_then(|bpm| bpm.as_f64()) {
                None => None,
                Some(bpm) => {
                    if !bpm.is_finite()
                        || bpm < crate::conform::MIN_SOURCE_BPM
                        || bpm > crate::conform::MAX_SOURCE_BPM
                    {
                        return Err(format!(
                            "the tempo of the material is from {} to {} beats a minute, and {bpm} is not",
                            crate::conform::MIN_SOURCE_BPM,
                            crate::conform::MAX_SOURCE_BPM
                        ));
                    }
                    Some(bpm)
                }
            },
            content: value
                .get("content")
                .and_then(|content| content.as_str())
                .map(|content| content.to_string()),
            midi: match value.get("midi") {
                Some(midi) if !midi.is_null() => Some(crate::midi::MidiPayload::from_json(midi)?),
                _ => None,
            },
        })
    }
}

/// What a tempo event says.
#[derive(Debug, Clone, PartialEq)]
struct TempoPayload {
    /// Where it happens, in ticks.
    tick: i64,
    /// How fast, in beats a minute.
    bpm: f64,
}

impl TempoPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "tick": self.tick, "bpm": self.bpm })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(TempoPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            bpm: value.get("bpm").and_then(|bpm| bpm.as_f64()).unwrap_or(120.0),
        })
    }
}

/// What a meter event says.
#[derive(Debug, Clone, PartialEq)]
struct MeterPayload {
    /// Where it happens, in ticks.
    tick: i64,
    /// How many beats a bar has.
    numerator: u32,
    /// Which beat a bar has.
    denominator: u32,
}

impl MeterPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "tick": self.tick,
            "numerator": self.numerator,
            "denominator": self.denominator,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(MeterPayload {
            tick: value.get("tick").and_then(|tick| tick.as_i64()).unwrap_or(0),
            numerator: value.get("numerator").and_then(|value| value.as_u64()).unwrap_or(4) as u32,
            denominator: value.get("denominator").and_then(|value| value.as_u64()).unwrap_or(4) as u32,
        })
    }
}

/// What a marker says.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MarkerPayload {
    /// Where it is, in samples.
    pub(crate) start: i64,
    /// Where it ends, in samples.
    end: i64,
}

impl MarkerPayload {
    fn to_json(&self) -> Json {
        serde_json::json!({ "start": self.start, "end": self.end })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        Ok(MarkerPayload {
            start: value.get("start").and_then(|start| start.as_i64()).unwrap_or(0),
            end: value.get("end").and_then(|end| end.as_i64()).unwrap_or(0),
        })
    }
}

/// What a source says.
#[derive(Debug, Clone, PartialEq)]
struct SourcePayload {
    /// Where the media is.
    path: String,
    /// How many channels it has.
    channels: u32,
    /// How long it is, in frames.
    frames: i64,
    /// The rate it was recorded at.
    sample_rate: u32,
    /// The **peaks** of the sound: the extent of the samples in each
    /// bucket, as a flat table of a least and a most.
    ///
    /// The sound is a great many numbers and a drawing needs few: the
    /// peaks are the shape of the sound, and the shape is what a reader
    /// sees. They are **measured** — a command reads the file and puts
    /// them here — because a file that is read at every frame is a file
    /// that is read a hundred times a second.
    peaks: Vec<f32>,
}

impl SourcePayload {
    fn to_json(&self) -> Json {
        serde_json::json!({
            "path": self.path,
            "channels": self.channels,
            "frames": self.frames,
            "sample_rate": self.sample_rate,
            "peaks": self.peaks,
        })
    }

    fn from_json(value: &Json) -> Result<Self, String> {
        // The peaks are **bounded**: the table comes from outside, and
        // a table of a million numbers is not a shape of a sound.
        let peaks = value
            .get("peaks")
            .and_then(|peaks| peaks.as_array())
            .map(|peaks| {
                peaks
                    .iter()
                    .filter_map(|peak| peak.as_f64())
                    .take(PEAK_NUMBERS)
                    .map(|peak| peak as f32)
                    .collect()
            })
            .unwrap_or_default();
        Ok(SourcePayload {
            path: value.get("path").and_then(|path| path.as_str()).unwrap_or("").to_string(),
            channels: value.get("channels").and_then(|channels| channels.as_u64()).unwrap_or(2) as u32,
            frames: value.get("frames").and_then(|frames| frames.as_i64()).unwrap_or(0),
            sample_rate: value.get("sample_rate").and_then(|rate| rate.as_u64()).unwrap_or(48000) as u32,
            peaks,
        })
    }
}

// -------------------------------------------------------------------- the commands --

/// Make a track.
///
/// A track comes with one playlist, because a track with no takes is not a
/// track yet: it is a row with nothing in it.
#[derive(Command)]
#[command(
    id = "sound.track.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "New"),
    mutates,
    undoable
)]
pub struct AddTrack {
    /// What to call it.
    pub name: Text,
    /// What kind of track it is: `audio`, `instrument`, `aux`, `master`.
    pub track_kind: Option<Text>,
    /// How many channels it has.
    pub channels: Option<Number>,
}

impl AddTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        let track_kind = match &self.track_kind {
            Some(kind) => kind.0.clone(),
            None => "audio".to_string(),
        };
        let channels = match &self.channels {
            Some(channels) => channels.0 as u32,
            None => 2,
        };
        let payload = TrackPayload {
            track_kind,
            channels,
            solo: false,
            trim_db: 0.0,
            fader_db: 0.0,
            pan: 0.0,
            mute: false,
            solo_safe: false,
            phase_invert: false,
            active_playlist: None,
            inactive: false,
            elastic: None,
        };
        let document = session.document_mut()?;
        let track = document.add_node_with(name.clone(), "sound.track", None, payload.to_json());
        document.add_node_with_under(Some(track), "A".to_string(), "sound.playlist", None, Json::Null);
        Ok(Outcome::changed(format!("added a track `{name}`"))
            .with_data(serde_json::json!({ "id": track, "name": name })))
    }
}

/// Add a **take** to a track.
///
/// A take is the tree's playlist: an alternative set of clips on the
/// same track, and one of the takes is the one that is played.
#[derive(Command)]
#[command(
    id = "sound.playlist.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "Takes"),
    mutates,
    undoable
)]
pub struct AddPlaylist {
    /// The track the take is on.
    pub track: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl AddPlaylist {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: "a take says what it is called".to_string(),
            });
        }
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if node.children.iter().any(|child| child.kind == "sound.playlist" && child.name == name) {
            return Err(EngineError::Failed {
                id: "sound.playlist.add".to_string(),
                message: format!("the track `{track}` already has a take called `{name}`"),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(track),
            name.clone(),
            "sound.playlist",
            None,
            Json::Null,
        );
        Ok(Outcome::changed(format!("the track `{track}` has a take called `{name}`"))
            .with_data(serde_json::json!({ "id": id, "track": track, "name": name })))
    }
}

/// Say which **take** of a track is played.
///
/// The tree's track has several playlists and an index of the one that
/// is played; here the take is named, because a name is what a person
/// says and an index is what a program says.
#[derive(Command)]
#[command(
    id = "sound.playlist.active",
    area = "sound",
    layer = "sound",
    icon = "check",
    menu("Track", "Takes"),
    mutates,
    undoable
)]
pub struct SetActivePlaylist {
    /// The track the take is on.
    pub track: NodeRef,
    /// The take that is played.
    pub playlist: Text,
}

impl SetActivePlaylist {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let track = self.track.0;
        let name = self.playlist.0.clone();
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if !node.children.iter().any(|child| child.kind == "sound.playlist" && child.name == name) {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("the track `{track}` has no take called `{name}`"),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.playlist.active".to_string(),
            message: error.to_string(),
        })?;
        payload.active_playlist = Some(name.clone());
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: "sound.playlist.active".to_string(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{track}` plays the take `{name}`"))
            .with_data(serde_json::json!({ "track": track, "playlist": name })))
    }
}

/// Set a track's **elastic algorithm**.
///
/// An elastic track is a track whose clips are **stretched and the
/// pitch stays**: the sound is longer and it is the same note. The
/// tree's track carries an algorithm for it — `polyphonic`,
/// `rhythmic`, `monophonic`, `varispeed`, `x-form` — and the
/// algorithm `varispeed` says that the pitch falls with the speed,
/// which is what an ordinary track does anyway.
///
/// The algorithm is a **string** and not a variant: the kinds of a
/// track are open here, and so are its algorithms. The names the tree
/// uses are the names this uses, and a third party may name one.
///
/// The algorithm `none` takes the elastic away, and so does an empty
/// name: a track with no algorithm is an ordinary track, and its
/// clips are played at another speed.
#[derive(Command)]
#[command(
    id = "sound.track.elastic",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Elastic"),
    mutates,
    undoable
)]
pub struct SetTrackElastic {
    /// The track to set.
    pub track: NodeRef,
    /// The algorithm: `polyphonic`, `rhythmic`, `monophonic`,
    /// `varispeed`, `x-form`, or `none` to take the elastic away.
    pub algorithm: Text,
}

impl SetTrackElastic {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.elastic".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.elastic".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let asked = self.algorithm.0.trim().to_string();
        if asked.is_empty() {
            return Err(EngineError::Failed {
                id: "sound.track.elastic".to_string(),
                message: "an algorithm is a name, and the name `none` takes the elastic away".to_string(),
            });
        }
        let algorithm = if asked == "none" { None } else { Some(asked.clone()) };
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.elastic".to_string(),
            message: error.to_string(),
        })?;
        payload.elastic = algorithm.clone();
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.elastic".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        let said = algorithm.clone().unwrap_or_else(|| "none".to_string());
        Ok(Outcome::changed(format!("the track `{id}` is `{said}`"))
            .with_data(serde_json::json!({ "track": id, "algorithm": algorithm })))
    }
}

/// Make a track **solo safe**, or not.
///
/// A solo silences the other tracks, and a **solo-safe** track is not
/// silenced by it: it is a track that stays when somebody listens to
/// one thing on purpose — a reference, a click, a talk track.
///
/// The tree's mixer keeps the flag beside the solo, and the group
/// shares the solo and not this: the flag is about one track and not
/// about the others.
#[derive(Command)]
#[command(
    id = "sound.track.solo.safe",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Solo Safe"),
    mutates,
    undoable
)]
pub struct SetTrackSoloSafe {
    /// The track.
    pub track: NodeRef,
    /// Whether it is solo safe.
    pub solo_safe: Bool,
}

impl SetTrackSoloSafe {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo.safe".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.solo.safe".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let solo_safe = self.solo_safe.0;
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.solo.safe".to_string(),
            message: error.to_string(),
        })?;
        payload.solo_safe = solo_safe;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo.safe".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{id}` is solo safe: {solo_safe}"))
            .with_data(serde_json::json!({ "id": id, "solo_safe": solo_safe })))
    }
}

/// Mute a track, or let it be heard.
///
/// The track's own **mute** is the level of the whole track: a muted
/// track is not heard, and neither is the copy that a send takes of
/// it. The tree's mixer keeps a mute beside the mute lane, and the
/// lane is the mute's default: a **mute lane** with breakpoints
/// overrides it, and a lane without one is the mute.
///
/// The mute is one of the things a **group** shares: the tree's group
/// shares the volume, the mute, the solo and the record, and the mute
/// is one of them — so a change to the mute of a track in a group
/// that mixes is a change to the mute of every member.
#[derive(Command)]
#[command(
    id = "sound.track.mute",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Mute"),
    mutates,
    undoable
)]
pub struct MuteTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether it is muted.
    pub muted: Bool,
}

impl MuteTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.mute".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.mute".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let muted = self.muted.0;
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.mute".to_string(),
            message: error.to_string(),
        })?;
        payload.mute = muted;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.mute".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        // The **mix link**: a group of tracks is mixed together, so a
        // change to the mute of one of them is a change to the mute of
        // all of them.
        let mut linked = Vec::new();
        for sibling in group::mix_siblings(session.document(), id) {
            if sibling == id {
                continue;
            }
            let Some(node) = session.document().node(sibling) else { continue };
            let Ok(mut other) = TrackPayload::from_json(&node.data) else { continue };
            other.mute = muted;
            let data = other.to_json();
            if let Some(node) = session.document_mut()?.node_mut(sibling) {
                node.data = data;
            }
            linked.push(sibling);
        }
        Ok(Outcome::changed(format!("the track `{id}` is muted: {muted}"))
            .with_data(serde_json::json!({ "id": id, "mute": muted, "linked": linked })))
    }
}

/// Set a track's **pan**.
///
/// The pan is the **balance** of the track's sound: minus one is all
/// to the left and one is all to the right, and the middle is the
/// middle. It is applied when the track is routed — a track that
/// feeds a bus is balanced before it reaches the bus — and a copy
/// that a send takes is balanced by the **send's** own pan and not by
/// this one.
///
/// The pan runs from minus one to one, and a pan outside that is
/// **refused** and not pulled to the edge.
#[derive(Command)]
#[command(
    id = "sound.track.pan",
    area = "sound",
    layer = "sound",
    icon = "sliders",
    menu("Track", "Pan"),
    mutates,
    undoable
)]
pub struct SetTrackPan {
    /// The track to set.
    pub track: NodeRef,
    /// Its balance: from minus one to one.
    pub pan: Number,
}

impl SetTrackPan {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.pan".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.pan".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let pan = self.pan.0;
        if !pan.is_finite() || !(-1.0..=1.0).contains(&pan) {
            return Err(EngineError::Failed {
                id: "sound.track.pan".to_string(),
                message: "a pan is a number between minus one and one".to_string(),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.pan".to_string(),
            message: error.to_string(),
        })?;
        payload.pan = pan;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.pan".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{id}` is at {pan}"))
            .with_data(serde_json::json!({ "id": id, "pan": pan })))
    }
}

/// Set a track's **trim** and **phase**.
///
/// The trim is the level of the track's **input**, before its inserts,
/// and the phase is a sign and not a level: the sound is the same and
/// upside down.
#[derive(Command)]
#[command(
    id = "sound.track.set",
    area = "sound",
    layer = "sound",
    icon = "sliders",
    menu("Track", "Trim"),
    mutates,
    undoable
)]
pub struct SetTrack {
    /// The track to set.
    pub track: NodeRef,
    /// Its trim, in decibels: from minus one hundred and forty-four to
    /// twenty-four.
    pub trim_db: Option<Number>,
    /// Whether its phase is inverted.
    pub phase: Option<Bool>,
}

impl SetTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.set".to_string(),
            message: error.to_string(),
        })?;
        if let Some(asked) = &self.trim_db {
            let decibels = asked.0;
            if !decibels.is_finite() || !(-144.0..=24.0).contains(&decibels) {
                return Err(EngineError::Failed {
                    id: "sound.track.set".to_string(),
                    message:
                        "a trim is a number of decibels from minus one hundred and forty-four to twenty-four"
                            .to_string(),
                });
            }
            payload.trim_db = decibels;
        }
        if let Some(phase) = &self.phase {
            payload.phase_invert = phase.0;
        }
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.set".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        // The **trim** is not linked by the group: the tree's group
        // links the **volume** and not the input gain, and the fader
        // is the volume.
        Ok(Outcome::changed(format!("the track `{id}` is set")).with_data(serde_json::json!({
            "id": id,
            "trim_db": payload.trim_db,
            "phase": payload.phase_invert,
        })))
    }
}

/// Set a track's **fader**.
///
/// The fader is the level of the track's **output**: it comes after
/// the track's inserts, and it is the level a track is heard at when
/// nothing automates it. The tree's fader runs from minus one hundred
/// and forty-four decibels to twelve, and a fader outside that is
/// **refused** and not pulled to the edge.
///
/// A **volume lane** overrides the fader: the tree's mixer runs the
/// lane when there is one and the fader when there is not, and the
/// fader is the lane's default.
///
/// The fader is the one thing a **group** links: the tree's group
/// carries a list of the attributes it shares — the volume, the mute,
/// the solo and the record — and the volume is the fader, so a change
/// to the fader of a track in a group that mixes is a change to the
/// fader of every member.
#[derive(Command)]
#[command(
    id = "sound.track.fader",
    area = "sound",
    layer = "sound",
    icon = "sliders",
    menu("Track", "Fader"),
    mutates,
    undoable
)]
pub struct SetTrackFader {
    /// The track to set.
    pub track: NodeRef,
    /// Its level, in decibels: from minus one hundred and forty-four
    /// to twelve.
    pub db: Number,
}

impl SetTrackFader {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.fader".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.fader".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let decibels = self.db.0;
        if !decibels.is_finite() || !(-144.0..=12.0).contains(&decibels) {
            return Err(EngineError::Failed {
                id: "sound.track.fader".to_string(),
                message: "a fader is a number of decibels from minus one hundred and forty-four to twelve"
                    .to_string(),
            });
        }
        let mut payload = TrackPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.track.fader".to_string(),
            message: error.to_string(),
        })?;
        payload.fader_db = decibels;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.fader".to_string(),
                message: format!("there is no track `{id}`"),
            });
        };
        node.data = data;
        // The **mix link**: a group of tracks is mixed together, so a
        // change to the fader of one of them is a change to the fader
        // of all of them.
        let mut linked = Vec::new();
        for sibling in group::mix_siblings(session.document(), id) {
            if sibling == id {
                continue;
            }
            let Some(node) = session.document().node(sibling) else { continue };
            let Ok(mut other) = TrackPayload::from_json(&node.data) else { continue };
            other.fader_db = decibels;
            let data = other.to_json();
            if let Some(node) = session.document_mut()?.node_mut(sibling) {
                node.data = data;
            }
            linked.push(sibling);
        }
        Ok(Outcome::changed(format!("the track `{id}` is at {decibels} decibels"))
            .with_data(serde_json::json!({ "id": id, "db": decibels, "linked": linked })))
    }
}

/// **Solo** a track.
///
/// A soloed track silences the others: when one track is soloed, the
/// rest are not heard — and their sends carry the silence, because a
/// send is a copy of the track's signal.
///
/// Soloing nothing is not the same as soloing everything: a document
/// with no soloed track is heard whole, and a document with one is
/// heard alone.
#[derive(Command)]
#[command(
    id = "sound.track.solo",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Track", "Solo"),
    mutates,
    undoable
)]
pub struct SoloTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether the track is soloed.
    pub solo: Bool,
}

impl SoloTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.track.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("the node `{id}` is a `{}` and not a track", node.kind),
            });
        }
        let solo = self.solo.0;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.track.solo".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        let mut payload = TrackPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.track.solo".to_string(), message: error })?;
        payload.solo = solo;
        node.data = payload.to_json();
        Ok(Outcome::changed(format!("the track `{id}` is soloed: {solo}"))
            .with_data(serde_json::json!({ "id": id, "solo": solo })))
    }
}

/// Put a clip on a track.
///
/// The track may be given as an id, and the playlist as a name: a track
/// holds playlists, and a clip goes in one of them.
#[derive(Command)]
#[command(
    id = "sound.clip.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Track", "New"),
    mutates,
    undoable
)]
pub struct AddClip {
    /// The track the clip goes on.
    pub track: NodeRef,
    /// The playlist it goes in, if the track has more than one.
    pub playlist: Option<Text>,
    /// Where it starts, in samples.
    pub start: Number,
    /// How long it is, in samples.
    pub length: Number,
    /// How much **longer** it is played than its source: one is none.
    pub stretch: Option<Number>,
    /// The source it plays.
    pub source: Option<NodeRef>,
    /// What to call it.
    pub name: Option<Text>,
}

impl AddClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let start = samples(self.start.0, "where the clip starts")?;
        let length = samples(self.length.0, "how long the clip is")?;
        if length == 0 {
            return Err(EngineError::Failed {
                id: "sound.clip.add".to_string(),
                message: "a clip with no length is not a clip".to_string(),
            });
        }
        // A **master** holds a fader and not a playlist: a clip on a
        // master would be a clip that is never heard.
        match session.document().node(self.track.0) {
            Some(node) if node.kind == "sound.track" && is_master(node) => {
                return Err(EngineError::Failed {
                    id: "sound.clip.add".to_string(),
                    message: "a master holds no clips: it has a fader and not a playlist".to_string(),
                });
            }
            _ => {}
        }
        // A **reader** takes a bus into the mix: it holds a bus and not
        // a playlist.
        if mixer::reads_bus(session.document(), self.track.0) {
            return Err(EngineError::Failed {
                id: "sound.clip.add".to_string(),
                message: "a track that reads a bus holds no clips: it has a bus and not a playlist"
                    .to_string(),
            });
        }
        // A clip on an **elastic** track is stretched with the pitch
        // kept, and that stretch is bounded the way the tree's is: a
        // stretch outside the bound is refused and not pulled to the
        // edge.
        let stretch = match self.stretch {
            Some(asked) => match session
                .document()
                .node(self.track.0)
                .and_then(elastic_algorithm)
                .filter(|algorithm| algorithm != "varispeed")
            {
                Some(algorithm) => elastic_stretch_of(asked.0, "sound.clip.add", &algorithm)?,
                None => stretch_of(asked.0, "sound.clip.add")?,
            },
            None => 1.0,
        };
        let playlist = playlist_of(session, self.track.0, self.playlist.as_ref().map(|name| name.0.clone()))?;
        let mut payload = ClipPayload::new();
        payload.start = start;
        payload.length = length;
        payload.source = self.source.map(|source| source.0);
        payload.stretch = stretch;
        let name = match &self.name {
            Some(name) => name.0.clone(),
            None => format!("clip at {start}"),
        };
        let id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            name.clone(),
            "sound.clip",
            None,
            payload.to_json(),
        );
        // The clip plays the source: that is a **relation**, not
        // containment. The clip is not in the source.
        if let Some(source) = self.source {
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(id, source.0, "source"));
        }
        Ok(Outcome::changed(format!("added a clip `{name}`")).with_data(
            serde_json::json!({ "id": id, "playlist": playlist, "start": start, "length": length }),
        ))
    }
}

/// **Move** clips along the timeline.
///
/// The clips move by so many samples, later or earlier, and their
/// **content** goes with them: a clip that moves keeps the sound it
/// plays and plays it at another time.
///
/// A **locked** clip does not move — the edit lock and the time lock
/// both hold it — and the answer says how many moved. The tree's
/// editor does the same, and the tree's own word for it is that a
/// locked clip "doesn't move".
///
/// A clip that would move **before the beginning** of the timeline is
/// **refused**: the move is not made for one clip and not for the
/// others, because a move that is half made is a move nobody asked
/// for.
#[derive(Command)]
#[command(
    id = "sound.clip.move",
    area = "sound",
    layer = "sound",
    icon = "move",
    menu("Clip", "Move"),
    mutates,
    undoable
)]
pub struct MoveClips {
    /// The clips to move: their ids.
    pub clips: serde_json::Value,
    /// How much later or earlier, in samples.
    pub delta: Number,
}

impl MoveClips {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.move".to_string();
        let clips = crate::clip_ids_of(&self.clips, &id)?;
        if clips.is_empty() {
            return Err(EngineError::Failed { id, message: "there are no clips to move".to_string() });
        }
        if !self.delta.0.is_finite() {
            return Err(EngineError::Failed { id, message: "a distance is a number of samples".to_string() });
        }
        let delta = self.delta.0.round() as i64;
        // The clips are read first, and the whole move is refused if
        // one of them would go before the beginning.
        let mut payloads: Vec<(u64, ClipPayload)> = Vec::with_capacity(clips.len());
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
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if locks::edit_is_locked(&payload) || locks::time_is_locked(&payload) {
                continue;
            }
            if payload.start + delta < 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` would move before the beginning of the timeline"),
                });
            }
            payloads.push((*clip, payload));
        }
        let mut moved = 0;
        for (clip, payload) in &payloads {
            let mut moved_payload = payload.clone();
            moved_payload.start += delta;
            if let Some(node) = session.document_mut()?.node_mut(*clip) {
                node.data = moved_payload.to_json();
            }
            moved += 1;
        }
        Ok(Outcome::changed(format!("moved {moved} clips"))
            .with_data(serde_json::json!({ "moved": moved, "delta": delta })))
    }
}

/// **Trim** clips to the **material** they play.
///
/// A clip plays a part of a source, and a clip that was moved may
/// reach past the end of it: the part of the clip that is past the end
/// is **silence**, and a clip that is half silence is a clip that says
/// something false about its own sound.
///
/// The clip is cut at the end of its material — and at the beginning
/// of it, when the clip starts past the end — and the part of the
/// material the clip plays is what is left.
///
/// The tree's editor has the same command, and the same name for it:
/// "To File Boundaries".
#[derive(Command)]
#[command(
    id = "sound.clip.trim.to.file",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Trim to Material"),
    mutates,
    undoable
)]
pub struct TrimClipsToFile {
    /// The clips to trim: their ids.
    pub clips: serde_json::Value,
}

impl TrimClipsToFile {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.trim.to.file".to_string();
        let clips = crate::clip_ids_of(&self.clips, &id)?;
        if clips.is_empty() {
            return Err(EngineError::Failed { id, message: "there are no clips to trim".to_string() });
        }
        let settings = session_of(session.document());
        let mut trimmed = 0;
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
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if locks::edit_is_locked(&payload) {
                continue;
            }
            let Some(source_id) = payload.source else { continue };
            let Some(source_node) = session.document().node(source_id) else { continue };
            let Ok(source) = SourcePayload::from_json(&source_node.data) else { continue };
            // The **material** in the session's samples: the source's
            // own frames, read at the session's rate.
            let ratio = settings.sample_rate.max(1) as f64 / source.sample_rate.max(1) as f64;
            let material = (source.frames.max(0) as f64 * ratio).round() as i64;
            let offset = payload.offset.max(0);
            if offset >= material {
                // The clip starts past the end of the material: there
                // is nothing of it to play.
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` starts past the end of its material"),
                });
            }
            // The clip's content runs for as long as the material
            // that is left of it, read at the clip's own stretch.
            let room = ((material - offset) as f64 * payload.stretch).round() as i64;
            if room >= payload.length {
                continue;
            }
            let mut shorter = payload.clone();
            shorter.length = room.max(1);
            if let Some(sync) = payload.sync {
                shorter.sync = (0..=shorter.length).contains(&sync).then_some(sync);
            }
            if let Some(node) = session.document_mut()?.node_mut(*clip) {
                node.data = shorter.to_json();
            }
            trimmed += 1;
        }
        Ok(Outcome::changed(format!("trimmed {trimmed} clips to their material"))
            .with_data(serde_json::json!({ "trimmed": trimmed })))
    }
}

/// Stretch a clip.
///
/// A stretched clip is **longer and lower**: the source is read more
/// slowly, so the sound lasts longer and the pitch falls with it. This
/// is a **varispeed**, and it is not a time stretch — a time stretch
/// keeps the pitch and changes the length, and that is a much harder
/// thing to do well.
///
/// The clip keeps its place and its length: the stretch says **how the
/// source is played**, and not how long the clip is. One is no stretch.
///
/// The clip's place in the source moves with it: trimming a stretched
/// clip moves the content by the delta **divided by the stretch**,
/// because the source is read that much more slowly.
///
/// On an **elastic** track the stretch is **pitch-preserving**: the
/// sound is longer and it is the same note, and that is a harder thing
/// to do — so it is bounded the way the tree's is, from a tenth to ten
/// times, and a stretch outside that is refused.
#[derive(Command)]
#[command(
    id = "sound.clip.stretch",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Stretch"),
    mutates,
    undoable
)]
pub struct SetClipStretch {
    /// The clip to stretch.
    pub clip: NodeRef,
    /// How much longer it is played than its source: one is none.
    pub stretch: Number,
}

impl SetClipStretch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.stretch".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.stretch".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        // A clip on an **elastic** track is stretched with the pitch
        // kept, and that is a harder thing to do: the stretch is
        // bounded the way the tree's is, and a stretch outside the
        // bound is refused.
        let elastic = timeline::track_of(session.document(), node).and_then(elastic_algorithm);
        let stretch = match elastic.as_deref() {
            Some(algorithm) if algorithm != "varispeed" => {
                elastic_stretch_of(self.stretch.0, "sound.clip.stretch", algorithm)?
            }
            _ => stretch_of(self.stretch.0, "sound.clip.stretch")?,
        };
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.stretch".to_string(), message: error })?;
        // A clip whose **content** is locked is finished: it is not
        // stretched.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: "sound.clip.stretch".to_string(),
                message: format!("the clip `{}` is locked: its sound is finished", self.clip.0),
            });
        }
        payload.stretch = stretch;
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("stretched the clip `{}` to {stretch}", self.clip.0))
            .with_data(serde_json::json!({ "id": self.clip.0, "stretch": stretch })))
    }
}

/// Do something to a clip's **sound**.
///
/// A process is one of the tree's **audio-suite processes**: the sound
/// is reversed, upside down, brought to a level, or has its direct
/// current taken away. The tree does each of them by rendering the
/// material and replacing the clip's content with the rendered file;
/// here the process is on the clip and the sound is worked out when it
/// is played.
///
/// The processes are done in the order they were added, so a clip may
/// have more than one — and a clip may have the same one twice, which
/// is what the tree does when the same process is asked for twice.
///
/// A process this battery does not know is **refused** and not
/// ignored: a process that does nothing is worse than no process at
/// all, because the document says the sound was changed and it was
/// not.
#[derive(Command)]
#[command(
    id = "sound.clip.process",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Process"),
    mutates,
    undoable
)]
pub struct AddClipProcess {
    /// The clip to process.
    pub clip: NodeRef,
    /// What to do: `reverse`, `invert`, `normalize` or `dc_offset`.
    pub process: Text,
    /// What it is given: `normalize` takes a `target_db`.
    pub params: Option<Json>,
}

impl AddClipProcess {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.process.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "sound.clip.process".to_string(),
                message: "a process is a name".to_string(),
            });
        }
        // The **parameters** are checked here, so that a process that
        // would be refused is refused before the clip is changed.
        let params = process_params(&name, self.params.as_ref(), "sound.clip.process")?;
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.process".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.process".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.process".to_string(), message: error })?;
        // A clip whose **content** is locked is finished: it is not
        // processed.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: "sound.clip.process".to_string(),
                message: format!("the clip `{}` is locked: its sound is finished", self.clip.0),
            });
        }
        payload.processes.push(ProcessPayload { process: name.clone(), params });
        let data = payload.to_json();
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the clip `{}` is processed with `{name}`", self.clip.0))
            .with_data(serde_json::json!({ "id": self.clip.0, "process": name })))
    }
}

/// Take a process off a clip.
///
/// The **last** process of that name is the one that goes, because the
/// processes are done in order and the last one is the one that was
/// asked for last. A clip with no such process has nothing to take
/// away, and that is refused.
#[derive(Command)]
#[command(
    id = "sound.clip.process.remove",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Unprocess"),
    mutates,
    undoable
)]
pub struct RemoveClipProcess {
    /// The clip to take the process off.
    pub clip: NodeRef,
    /// The name of the process.
    pub process: Text,
}

impl RemoveClipProcess {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.process.0.trim().to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.process.remove".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.process.remove".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.clip.process.remove".to_string(),
            message: error,
        })?;
        let Some(at) = payload.processes.iter().rposition(|process| process.process == name) else {
            return Err(EngineError::Failed {
                id: "sound.clip.process.remove".to_string(),
                message: format!("the clip `{}` has no `{name}`", self.clip.0),
            });
        };
        payload.processes.remove(at);
        let data = payload.to_json();
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = data;
        }
        Ok(Outcome::changed(format!("the clip `{}` is not processed with `{name}`", self.clip.0))
            .with_data(serde_json::json!({ "id": self.clip.0, "process": name })))
    }
}

/// The parameters of a process, and the check that it is one this
/// battery knows.
///
/// The name is a string and not a variant, because the kinds here are
/// open — but a name nothing knows is **refused**, because a process
/// that is quietly ignored is a process the document lies about.
fn process_params(name: &str, params: Option<&Json>, id: &str) -> Result<Json, EngineError> {
    let given = params.cloned().unwrap_or(Json::Null);
    match name {
        "reverse" | "invert" | "dc_offset" => Ok(Json::Null),
        "normalize" => {
            let target = given.get("target_db").and_then(|target| target.as_f64()).unwrap_or(0.0);
            if !target.is_finite() || !(-144.0..=24.0).contains(&target) {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: format!(
                        "a level is a number of decibels from minus one hundred and forty-four to \
                         twenty-four, and this one is {target}"
                    ),
                });
            }
            Ok(serde_json::json!({ "target_db": target }))
        }
        other => Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("not supported yet: the process `{other}`"),
        }),
    }
}

/// Move the **pitch** of a clip.
///
/// A pitch shift is the other half of a stretch: a stretch makes the
/// sound longer and the pitch stays, and a pitch shift moves the pitch
/// and the sound stays as long as it was. Twelve semitones are an
/// octave up and minus twelve an octave down.
///
/// The shift runs from minus twenty-four semitones to twenty-four, and
/// a shift outside that is **refused** and not pulled to the edge: a
/// caller that asked for a hundred semitones would be given
/// twenty-four and would not know.
///
/// A clip whose **content** is locked is finished, and it is not
/// shifted.
#[derive(Command)]
#[command(
    id = "sound.clip.pitch",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Pitch"),
    mutates,
    undoable
)]
pub struct SetClipPitch {
    /// The clip to shift.
    pub clip: NodeRef,
    /// How many semitones: twelve is an octave up, zero is none.
    pub semitones: Number,
}

impl SetClipPitch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let semitones = self.semitones.0;
        if !semitones.is_finite() || !(-MAX_PITCH_SEMITONES..=MAX_PITCH_SEMITONES).contains(&semitones) {
            return Err(EngineError::Failed {
                id: "sound.clip.pitch".to_string(),
                message: format!(
                    "a pitch is from minus {MAX_PITCH_SEMITONES} semitones to {MAX_PITCH_SEMITONES}, \
                     and this one is {semitones}"
                ),
            });
        }
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.pitch".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.pitch".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.pitch".to_string(), message: error })?;
        // A clip whose **content** is locked is finished: it is not
        // shifted.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: "sound.clip.pitch".to_string(),
                message: format!("the clip `{}` is locked: its sound is finished", self.clip.0),
            });
        }
        payload.pitch_semitones = semitones;
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("the clip `{}` is at {semitones} semitones", self.clip.0))
            .with_data(serde_json::json!({ "id": self.clip.0, "semitones": semitones })))
    }
}

/// **Fade** a clip **to a place**: up to it, or down to it.
///
/// The tree's editor fades a clip to the **playhead**: "Fade to
/// Start" fades the clip from its head up to the playhead, and "Fade
/// to End" fades it from the playhead down to its tail. The shape of
/// the fade is the tree's own — the equal-power curve — and the
/// curvature is not bent.
///
/// Here the place is **given**, because the playhead is a node of the
/// document and a node is not a place every command may read: the
/// place is where the caller says it is.
///
/// A place that is **not inside the clip** is refused: a fade that
/// ends outside the clip is a fade of nothing.
#[derive(Command)]
#[command(
    id = "sound.clip.fade.to",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Fade"),
    mutates,
    undoable
)]
pub struct FadeClipTo {
    /// The clip to fade.
    pub clip: NodeRef,
    /// Where the fade ends, in the clip's own samples.
    pub at: Number,
    /// Which way the fade goes: `in` fades the clip from its head up
    /// to the place, and `out` fades it from the place down to its
    /// tail.
    pub to: Option<Text>,
}

impl FadeClipTo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.fade.to".to_string();
        let clip_id = self.clip.0;
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{clip_id}`"),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        // The **way** the fade goes: `in` or `out`, and a name this
        // battery does not know is refused.
        let to = match &self.to {
            Some(to) => match to.0.trim() {
                "in" => "in",
                "out" => "out",
                other => {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("a fade goes `in` or `out`, and not `{other}`"),
                    });
                }
            },
            None => "in",
        };
        // The **place** is in the clip's own time: how far into the
        // clip it is.
        let at = samples(self.at.0, "where the fade ends")?;
        let into = at - payload.start;
        if into <= 0 || into >= payload.length {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "the clip `{clip_id}` runs from {} to {} and cannot fade at {at}",
                    payload.start,
                    payload.start + payload.length
                ),
            });
        }
        // The tree's own shape: the equal-power curve, and a bend of
        // nothing.
        //
        // The **length** of the fade is how far it runs: a fade in
        // runs from the clip's head to the place, and a fade out runs
        // from the place to the clip's tail.
        let len = if to == "in" { into } else { payload.length - into };
        let fade = FadePayload { len, shape: craftkit_audio::FadeShape::EqualPower, curvature: 0.0 };
        if to == "in" {
            payload.fade_in = fade;
        } else {
            payload.fade_out = fade;
        }
        if let Some(node) = session.document_mut()?.node_mut(clip_id) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("faded the clip `{clip_id}` {to} to {at}")).with_data(
            serde_json::json!({
                "id": clip_id,
                "at": at,
                "to": to,
                "fade_in": payload.fade_in.to_json(),
                "fade_out": payload.fade_out.to_json(),
            }),
        ))
    }
}

/// Fade a clip.
///
/// A fade is a **curve from silence to full**, and it is at the two
/// ends of a clip: the head fades in and the tail fades out. A fade
/// that is longer than the clip it fades is **refused**: a fade that
/// does not finish is a fade that is not there, and a fade that is
/// quietly shortened is a fade the caller did not ask for.
///
/// The **shape** is the curve: linear, equal_power, s_curve,
/// exponential or logarithmic. The **bend** pushes the curve away from
/// itself, and a bend of zero is the shape exactly.
///
/// A fade does not change the clip's length and does not change its
/// place: it changes how the clip is heard.
#[derive(Command)]
#[command(
    id = "sound.clip.fade",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Fade"),
    mutates,
    undoable
)]
pub struct SetClipFade {
    /// The clip to fade.
    pub clip: NodeRef,
    /// How long the fade in is, in samples.
    pub fade_in: Option<Number>,
    /// How long the fade out is, in samples.
    pub fade_out: Option<Number>,
    /// The shape of the curve: linear, equal_power, s_curve, exponential or logarithmic.
    pub shape: Option<Text>,
    /// How much the curve is bent, from -1 to 1.
    pub curvature: Option<Number>,
}

impl SetClipFade {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.fade".to_string(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.fade".to_string(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.fade".to_string(), message: error })?;
        // A clip whose **content** is locked is finished: it is not
        // faded.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: "sound.clip.fade".to_string(),
                message: format!("the clip `{}` is locked: its sound is finished", self.clip.0),
            });
        }
        if self.fade_in.is_none() && self.fade_out.is_none() {
            return Err(EngineError::Failed {
                id: "sound.clip.fade".to_string(),
                message: "a fade needs a length: how long is the fade in, and how long is the fade out?"
                    .to_string(),
            });
        }
        // The **shape** and the **bend** are the curve, and they are
        // read once: they apply to the fades that are being set.
        let shape = match &self.shape {
            Some(shape) => {
                craftkit_audio::FadeShape::from_label(&shape.0).ok_or_else(|| EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("there is no fade shape called `{}`", shape.0),
                })?
            }
            None => craftkit_audio::FadeShape::default(),
        };
        let curvature = match self.curvature {
            Some(curvature) => {
                if !curvature.0.is_finite() {
                    return Err(EngineError::Failed {
                        id: "sound.clip.fade".to_string(),
                        message: "a bend is a number".to_string(),
                    });
                }
                curvature.0.clamp(-1.0, 1.0) as f32
            }
            None => 0.0,
        };
        if let Some(fade_in) = self.fade_in {
            let len = samples(fade_in.0, "how long the fade in is")?;
            if len > payload.length {
                return Err(EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("the fade in is {len} samples long and the clip is {}", payload.length),
                });
            }
            payload.fade_in = FadePayload { len, shape, curvature };
        }
        if let Some(fade_out) = self.fade_out {
            let len = samples(fade_out.0, "how long the fade out is")?;
            if len > payload.length {
                return Err(EngineError::Failed {
                    id: "sound.clip.fade".to_string(),
                    message: format!("the fade out is {len} samples long and the clip is {}", payload.length),
                });
            }
            payload.fade_out = FadePayload { len, shape, curvature };
        }
        if let Some(node) = session.document_mut()?.node_mut(self.clip.0) {
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("faded the clip `{}`", self.clip.0)).with_data(serde_json::json!({
            "id": self.clip.0,
            "fade_in": payload.fade_in.to_json(),
            "fade_out": payload.fade_out.to_json(),
        })))
    }
}

/// Trim a clip.
///
/// A trim is about a **range**, and not about a node: the clip keeps its
/// place on the track and its place in the source, and what changes is
/// how much of it is heard. The range may be given, or it may come from
/// the selection — the selection holds a **part** of a node as well as
/// whole nodes, and a part of a clip is exactly what a trim is about.
///
/// The range is in **samples on the track's timeline**, and it is half
/// open: `from` is in the clip and `to` is not.
#[derive(Command)]
#[command(
    id = "sound.clip.trim",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Trim"),
    mutates,
    undoable
)]
pub struct TrimClip {
    /// The clip to trim, if the caller names one.
    pub clip: Option<NodeRef>,
    /// Where the clip starts, in samples.
    pub from: Option<Number>,
    /// Where the clip ends, in samples.
    pub to: Option<Number>,
}

impl TrimClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        // The clip: named, or the one that is selected.
        let (clip_id, span) = match (self.clip, selection_span(session)) {
            (Some(clip), Some((_, span))) => (clip.0, Some(span)),
            (Some(clip), None) => (clip.0, None),
            (None, Some((node, span))) => (node, Some(span)),
            (None, None) => {
                return Err(EngineError::Failed {
                    id: "sound.clip.trim".to_string(),
                    message: "which clip, and which part of it?".to_string(),
                });
            }
        };
        let (from, to) = match (self.from, self.to) {
            (Some(from), Some(to)) => (from.0, to.0),
            (None, None) => match span {
                Some((from, to)) => (from, to),
                None => {
                    return Err(EngineError::Failed {
                        id: "sound.clip.trim".to_string(),
                        message: "which part of the clip?".to_string(),
                    });
                }
            },
            _ => {
                return Err(EngineError::Failed {
                    id: "sound.clip.trim".to_string(),
                    message: "a range has a start and an end".to_string(),
                });
            }
        };
        if !from.is_finite() || !to.is_finite() {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: "a range is made of numbers".to_string(),
            });
        }
        if to <= from {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the range ends at {to} and starts at {from}"),
            });
        }
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.trim".to_string(),
            message: format!("there is no node `{clip_id}`"),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
            });
        }
        let payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: "sound.clip.trim".to_string(), message: error })?;
        // A clip whose **content** is locked is finished: it is not
        // trimmed.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        if from < payload.start as f64 {
            return Err(EngineError::Failed {
                id: "sound.clip.trim".to_string(),
                message: format!("the clip starts at {} and cannot be trimmed before it", payload.start),
            });
        }
        let mut trimmed = payload.clone();
        trimmed.start = from as i64;
        trimmed.length = (to - from) as i64;
        // The content moves with the start: a clip trimmed at its head
        // starts later in the source. A stretched clip's source is read
        // more slowly, so the content moves by the delta **divided by
        // the stretch**: the same stretch that makes the clip longer
        // makes it slower, and the two are the same fact.
        let delta = from as i64 - payload.start;
        trimmed.offset = payload.offset + ((delta as f64 / payload.stretch).round() as i64);
        // The **beat** moves with the audio it is in: the trim took
        // `delta` samples off the head, so the beat is `delta` nearer
        // the clip's head. A beat that the trim left outside the clip
        // is not a beat of that clip any more, and it goes.
        if let Some(sync) = payload.sync {
            let moved = sync - delta;
            trimmed.sync = (0..=trimmed.length).contains(&moved).then_some(moved);
        }
        // The **notes** of a MIDI clip move with the audio they are in:
        // the notes the trim took off the head are gone, and the notes
        // that are left are counted from the clip's new head. The
        // tree's trim does the same — the notes are re-anchored by the
        // engine, which knows the tempo map.
        //
        // The boundary is the distance the trim made, in the **ticks**
        // of the document's tempo at the clip's own start.
        if let Some(midi) = payload.midi.as_ref() {
            let settings = session_of(session.document());
            let map = tempo_map(session.document());
            let start_tick = map.samples_to_ticks(payload.start, settings.sample_rate);
            let cut_tick = map.samples_to_ticks(from as i64, settings.sample_rate);
            let boundary = cut_tick.saturating_sub(start_tick);
            trimmed.midi = Some(midi.divided(boundary, true));
        }
        if let Some(node) = session.document_mut()?.node_mut(clip_id) {
            node.data = trimmed.to_json();
        }
        Ok(Outcome::changed(format!("trimmed the clip `{clip_id}` to {from}..{to}")).with_data(
            serde_json::json!({
                "id": clip_id,
                "from": from,
                "to": to,
                "offset": trimmed.offset,
                "sync": trimmed.sync,
            }),
        ))
    }
}

/// Separate a clip into two.
///
/// The tree's editor separates a take at a place on the track: every
/// clip that crosses that place becomes two clips, and the two are
/// the same sound with a line between them. Here one clip is
/// separated at one place, and the place is **inside** the clip — a
/// place at its edge or outside it is refused, because there is
/// nothing to separate.
///
/// The two pieces are the same sound: the first keeps the head of the
/// clip and the second keeps the tail, and the second starts later in
/// the source by the distance the cut made — **divided by the
/// stretch**, because a stretched clip's source is read more slowly.
///
/// The **fade** at the cut is dropped: the tree drops it too, because
/// a fade that ended at the cut would be a fade of nothing.
#[derive(Command)]
#[command(
    id = "sound.clip.separate",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Separate"),
    mutates,
    undoable
)]
pub struct SeparateClip {
    /// The clip to separate.
    pub clip: NodeRef,
    /// Where to cut it, in samples on the timeline.
    pub at: Number,
}

impl SeparateClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.separate".to_string();
        let clip_id = self.clip.0;
        let at = self.at.0;
        if !at.is_finite() {
            return Err(EngineError::Failed { id: id.clone(), message: "a place is a number".to_string() });
        }
        let (name, payload) = {
            let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            (node.name.clone(), payload)
        };
        // A clip whose **content** is locked is finished: it is not
        // separated.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        let at = at as i64;
        let end = payload.start + payload.length;
        if at <= payload.start || at >= end {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip runs from {} to {end} and cannot be separated at {at}",
                    payload.start
                ),
            });
        }
        let second = separate_at(session, clip_id, &payload, &name, at)?;
        Ok(Outcome::changed(format!("separated the clip `{clip_id}` at {at}")).with_data(serde_json::json!({
            "id": clip_id,
            "at": at,
            "second": second,
        })))
    }
}

/// The **cut** of a clip at a place.
///
/// The clip becomes two clips under the same take: the first keeps
/// the head of the clip and the second keeps the tail, and the second
/// starts later in the source by the distance the cut made — **divided
/// by the stretch**, because a stretched clip's source is read more
/// slowly.
///
/// The **beat** moves with the audio it is in: a beat that the cut
/// left in the other piece is not a beat of this one.
///
/// The **fade** at the cut is dropped: the tree drops it too, because
/// a fade that ended at the cut would be a fade of nothing.
///
/// The **notes** of a MIDI clip are divided at the cut: the notes
/// before it stay in the first piece and the notes at or after it move
/// to the second, counted from the second piece's own head. The tree
/// divides them the same way — a note is not in both pieces.
///
/// The **second** piece's id is the answer: it is the piece that runs
/// from the cut to the end, and it is the piece a further cut goes
/// into.
fn separate_at(
    session: &mut Session,
    clip_id: u64,
    payload: &ClipPayload,
    name: &str,
    at: i64,
) -> Result<u64, EngineError> {
    let end = payload.start + payload.length;
    // The **first** piece: the head of the clip, with its head fade
    // and without a fade at the cut.
    let mut first = payload.clone();
    first.length = at - payload.start;
    first.fade_out = FadePayload::none();
    // The **beat** of the first piece is the beat of the clip if it
    // is inside it; a beat that the cut left in the second piece is
    // not a beat of this one.
    first.sync = payload.sync.filter(|sync| *sync <= first.length);
    // The **second** piece: the tail, with the tail's fade and
    // without one at the cut. Its content starts later in the source
    // by the distance the cut made, **divided by the stretch**: a
    // stretched clip's source is read more slowly, and the two are the
    // same fact.
    let mut second = payload.clone();
    second.start = at;
    second.length = end - at;
    second.fade_in = FadePayload::none();
    let delta = at - payload.start;
    second.offset = payload.offset + ((delta as f64 / payload.stretch).round() as i64);
    // The **beat** moves with the audio it is in: the beat of the
    // second piece is nearer its head by the distance the cut made,
    // and a beat that the cut left in the other piece is not a beat
    // of this one.
    second.sync = payload.sync.map(|sync| sync - delta).filter(|sync| *sync >= 0 && *sync <= second.length);
    // The **notes** of a MIDI clip are divided at the cut: the notes
    // before it stay in the first piece and the notes at or after it
    // move to the second, counted from the second piece's own head.
    //
    // The notes are in **ticks** and the cut is in **samples**, so the
    // boundary is the distance the cut made, in the ticks of the
    // document's tempo map at the clip's own start.
    if let Some(midi) = payload.midi.as_ref() {
        let settings = session_of(session.document());
        let map = tempo_map(session.document());
        let start_tick = map.samples_to_ticks(payload.start, settings.sample_rate);
        let cut_tick = map.samples_to_ticks(at, settings.sample_rate);
        let boundary = cut_tick.saturating_sub(start_tick);
        first.midi = Some(midi.divided(boundary, false));
        second.midi = Some(midi.divided(boundary, true));
    }
    let playlist = timeline::parent_of(session.document(), clip_id).map(|parent| parent.id);
    let source = payload.source;
    let first_data = first.to_json();
    let second_data = second.to_json();
    let document = session.document_mut()?;
    if let Some(node) = document.node_mut(clip_id) {
        node.data = first_data;
    }
    // The **second** piece is a clip of its own, under the same take,
    // and it plays the same source.
    let second_id = document.add_node_with_under(playlist, name.to_string(), "sound.clip", None, second_data);
    if let Some(source) = source {
        document.add_relation(craftkit_sdk::Relation::new(second_id, source, "source"));
    }
    // The **new piece** takes the next name of the scheme the clip is
    // in: the scheme counts up, and the piece before the cut keeps the
    // name it had.
    crate::autoname::name_the_new_piece(session, clip_id, second_id)?;
    Ok(second_id)
}

/// Separate a clip on the **grid**.
///
/// The tree's editor separates a take on the grid: the clip is cut at
/// every line of the grid it crosses, and the pieces are the bars or
/// the beats the grid says they are. The grid is the session's (§200)
/// and the lines are **musical**, so they move with the tempo.
///
/// The number of lines is **bounded**: a clip that crosses more lines
/// than the bound is **refused** and not cut at the first thousand of
/// them, because a clip cut at a thousand lines is a clip nobody
/// meant to make.
#[derive(Command)]
#[command(
    id = "sound.clip.separate.grid",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Separate on Grid"),
    mutates,
    undoable
)]
pub struct SeparateClipOnGrid {
    /// The clip to separate.
    pub clip: NodeRef,
}

impl SeparateClipOnGrid {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.separate.grid".to_string();
        let clip_id = self.clip.0;
        let (name, payload) = {
            let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            (node.name.clone(), payload)
        };
        // A clip whose **content** is locked is finished: it is not
        // separated.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        let settings = session_of(session.document());
        let map = tempo_map(session.document());
        let end = payload.start + payload.length;
        // The **lines** the clip crosses, and the bound: a clip that
        // crosses more lines than the bound is refused.
        let lines = settings.grid.lines(payload.start, end, &map, settings.sample_rate, MAX_GRID_LINES + 1);
        if lines.len() > MAX_GRID_LINES {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip crosses more than {MAX_GRID_LINES} lines of the grid"),
            });
        }
        // The **cut**: each line cuts the piece that is left, and the
        // piece that is left is the piece the next line cuts.
        let mut current = clip_id;
        let mut pieces = 1_usize;
        for line in &lines {
            let piece = ClipPayload::from_json(
                &session
                    .document()
                    .node(current)
                    .ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{current}` is gone"),
                    })?
                    .data,
            )
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if *line <= piece.start || *line >= piece.start + piece.length {
                continue;
            }
            current = separate_at(session, current, &piece, &name, *line)?;
            pieces += 1;
        }
        Ok(Outcome::changed(format!("separated the clip `{clip_id}` on the grid"))
            .with_data(serde_json::json!({ "id": clip_id, "pieces": pieces, "lines": lines.len() })))
    }
}

/// Separate a clip at its **transients**.
///
/// The tree's editor separates a take at the transients of its sound:
/// the places where the sound starts again — a drum, a plucked string,
/// the start of a phrase. Here the transients are found in the
/// **material** — the source the clip plays — and not in the clip's own
/// sound: the clip's sound is the material read the clip's way, and
/// the material's own attacks are where the pieces are.
///
/// The **sensitivity** is how small a rise may be and still be a
/// transient, from zero to one: the higher it is, the smaller a rise
/// counts.
///
/// The detector is the tree's own: it watches the **spectrum** — it
/// takes a fast Fourier transform of each frame and looks for a jump in
/// the bands of it — and the transform is in the audio library, written
/// there because this kit's crates carry no dependencies at all.
///
/// The number of cuts is **bounded**: a clip with more transients than
/// the bound is **refused** and not cut at the first thousand of them,
/// because a clip cut at a thousand places is a clip nobody meant to
/// make.
#[derive(Command)]
#[command(
    id = "sound.clip.separate.transients",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Separate on Transients"),
    mutates,
    undoable
)]
pub struct SeparateClipOnTransients {
    /// The clip to separate.
    pub clip: NodeRef,
    /// How small a rise may be and still be a transient, from zero to one.
    pub sensitivity: Option<Number>,
}

impl SeparateClipOnTransients {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.separate.transients".to_string();
        let clip_id = self.clip.0;
        let (name, payload) = {
            let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            (node.name.clone(), payload)
        };
        // A clip whose **content** is locked is finished: it is not
        // separated.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        let Some(source_id) = payload.source else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` plays no sound, and there is nothing to find"),
            });
        };
        let source_node = session.document().node(source_id).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no source `{source_id}`"),
        })?;
        let source = SourcePayload::from_json(&source_node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        let sound = read_sound(&id, &source.path)?;
        let sensitivity = match self.sensitivity {
            Some(sensitivity) if sensitivity.0.is_finite() => sensitivity.0.clamp(0.0, 1.0),
            Some(_) => {
                return Err(EngineError::Failed {
                    id,
                    message: "the sensitivity is a number from zero to one".to_string(),
                });
            }
            None => 0.5,
        };
        // The **transients** of the material, in the material's own
        // samples.
        let onsets = craftkit_audio::onsets(&sound.samples, sound.channels as usize, sound.rate, sensitivity);
        // The material's samples are the **session's** samples turned
        // into the source's rate, and the clip's offset says where in
        // the material the clip starts.
        let settings = session_of(session.document());
        let rate = settings.sample_rate.max(1) as f64;
        let source_rate = sound.rate.max(1) as f64;
        let ratio = source_rate / rate;
        let end = payload.start + payload.length;
        let mut cuts: Vec<i64> = Vec::new();
        for onset in onsets {
            // The onset in the material's samples, and then in the
            // clip's own time: a stretched clip reads its material more
            // slowly, so the stretch is in the way back.
            let at = at_of_frame(onset as i64, payload.offset, payload.stretch, ratio);
            let at = at.max(payload.start + 1);
            if at >= end {
                continue;
            }
            if cuts.last().is_some_and(|last| *last == at) {
                continue;
            }
            cuts.push(at);
        }
        if cuts.len() > MAX_TRANSIENT_CUTS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip has more than {MAX_TRANSIENT_CUTS} transients"),
            });
        }
        // The **cut**: each transient cuts the piece that is left, and
        // the piece that is left is the piece the next transient cuts.
        let mut current = clip_id;
        let mut pieces = 1_usize;
        for at in &cuts {
            let piece = ClipPayload::from_json(
                &session
                    .document()
                    .node(current)
                    .ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{current}` is gone"),
                    })?
                    .data,
            )
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if *at <= piece.start || *at >= piece.start + piece.length {
                continue;
            }
            current = separate_at(session, current, &piece, &name, *at)?;
            pieces += 1;
        }
        Ok(Outcome::changed(format!("separated the clip `{clip_id}` at its transients")).with_data(
            serde_json::json!({
                "id": clip_id,
                "pieces": pieces,
                "transients": cuts.len(),
                "sensitivity": sensitivity,
            }),
        ))
    }
}

/// **Strip the silence** out of a clip.
///
/// The tree's editor strips silence: it finds the parts of a sound that
/// are not silent, cuts the clip at the edges of them, and keeps the
/// loud parts and throws the quiet ones away. The kept parts are
/// **named** from the auto rename scheme, in time order.
///
/// Here the parts are found in the **material** — the source the clip
/// plays — and not in the clip's own sound: the clip's sound is the
/// material read the clip's way, and the material's own silence is
/// where the silence is.
///
/// The **threshold** is how far below the loudest part of the material
/// a part may be and still be kept, in decibels: forty-eight is the
/// tree's own default, and it is a part that is at least a fortieth of
/// the peak.
///
/// The **minimum length** is how long a part must be to be kept: a
/// click is not a part, and a click that is kept is a clip of its own
/// that says nothing.
///
/// The **pad** is how much of the silence is kept at each end of a
/// part, so that a part does not begin or end with a click.
///
/// The tree's editor does this in three **modes**: it **strips** the
/// silence and keeps the loud parts, it **extracts** the loud parts and
/// keeps the silence, and it **separates** the clip at the edges of the
/// loud parts and keeps everything. The mode is given by name, and a
/// name this battery does not know is **refused**.
#[derive(Command)]
#[command(
    id = "sound.clip.strip.silence",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Strip Silence"),
    mutates,
    undoable
)]
pub struct StripSilence {
    /// The clip to strip.
    pub clip: NodeRef,
    /// How far below the loudest part a part may be and still be kept,
    /// in decibels.
    pub threshold_db: Option<Number>,
    /// How long a part must be to be kept, in milliseconds.
    pub min_length_ms: Option<Number>,
    /// How much silence is kept before each part, in milliseconds.
    pub pad_before_ms: Option<Number>,
    /// How much silence is kept after each part, in milliseconds.
    pub pad_after_ms: Option<Number>,
    /// What to do with the parts: `strip` keeps the loud parts and
    /// throws the quiet ones away, `extract` keeps the quiet parts and
    /// throws the loud ones away, and `separate` cuts the clip at the
    /// edges of the loud parts and keeps everything.
    pub mode: Option<Text>,
}

impl StripSilence {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.strip.silence".to_string();
        let clip_id = self.clip.0;
        // The **mode**: what is kept and what is thrown away. A mode
        // this battery does not know is refused, and nothing is
        // changed.
        let mode = match &self.mode {
            Some(mode) => StripMode::parse(mode.0.trim()).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no mode called `{}`: the modes are `strip`, `extract` and `separate`",
                    mode.0.trim()
                ),
            })?,
            None => StripMode::Strip,
        };
        let (name, payload) = {
            let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            (node.name.clone(), payload)
        };
        // A clip whose **content** is locked is finished: it is not
        // stripped.
        if locks::edit_is_locked(&payload) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` is locked: its sound is finished"),
            });
        }
        let Some(source_id) = payload.source else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip_id}` plays no sound, and there is no silence in it"),
            });
        };
        if session.document().node(source_id).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no source `{source_id}`"),
            });
        }
        // The **loud parts** of the clip, in the clip's own time.
        let ranges = loud_parts_of(
            &id,
            session,
            &payload,
            self.threshold_db,
            self.min_length_ms,
            self.pad_before_ms,
            self.pad_after_ms,
        )?;
        if ranges.is_empty() {
            // A clip with no loud part at all is **silent**. The strip
            // and the separate have nothing to keep, and the extract
            // keeps the whole clip — which is what a silent clip is:
            // all silence.
            if mode != StripMode::Extract {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the clip `{clip_id}` is silent: there is nothing to keep"),
                });
            }
        }
        // The scheme and its **number** before the cut: the pieces
        // that are kept are named from it, and the pieces that are
        // thrown away must not take numbers.
        let scheme = crate::autoname::scheme_of(session.document(), clip_id);
        let number = scheme.map(|scheme| crate::autoname::number_of(session, scheme));
        // The **cut**: at the edges of the loud parts, so that the
        // pieces are the parts and the gaps.
        let end = payload.start + payload.length;
        let mut points: Vec<i64> = Vec::new();
        for (start, stop) in &ranges {
            for point in [start, stop] {
                if *point > payload.start && *point < end && !points.contains(point) {
                    points.push(*point);
                }
            }
        }
        points.sort_unstable();
        let mut current = clip_id;
        let mut pieces = 1_usize;
        for point in &points {
            let piece = ClipPayload::from_json(
                &session
                    .document()
                    .node(current)
                    .ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: format!("the clip `{current}` is gone"),
                    })?
                    .data,
            )
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if *point <= piece.start || *point >= piece.start + piece.length {
                continue;
            }
            current = separate_at(session, current, &piece, &name, *point)?;
            pieces += 1;
        }
        // The pieces are **kept** or **thrown away**, and which is
        // which depends on the **mode**: the strip keeps the loud
        // pieces, the extract keeps the quiet ones, and the separate
        // keeps everything and only cuts.
        let playlist = timeline::parent_of(session.document(), clip_id).map(|parent| parent.id);
        let mut kept: Vec<u64> = Vec::new();
        let mut removed: Vec<u64> = Vec::new();
        for node in session.document().walk() {
            if node.kind != "sound.clip" {
                continue;
            }
            if timeline::parent_of(session.document(), node.id).map(|parent| parent.id) != playlist {
                continue;
            }
            let Ok(payload) = ClipPayload::from_json(&node.data) else { continue };
            let start = payload.start;
            let stop = payload.start + payload.length;
            let loud = ranges.iter().any(|(from, to)| start >= *from && stop <= *to);
            let keep = match mode {
                StripMode::Strip => loud,
                StripMode::Extract => !loud,
                StripMode::Separate => true,
            };
            if keep {
                kept.push(node.id);
            } else {
                removed.push(node.id);
            }
        }
        for piece in &removed {
            session.document_mut()?.remove_node(*piece);
        }
        // The kept pieces are **named** from the scheme, in time order,
        // and the number the thrown-away pieces took is given back.
        if let (Some(scheme), Some(number)) = (scheme, number) {
            crate::autoname::set_number(session, scheme, number);
        }
        kept.sort_by_key(|piece| {
            session
                .document()
                .node(*piece)
                .and_then(|node| ClipPayload::from_json(&node.data).ok())
                .map(|payload| payload.start)
                .unwrap_or(0)
        });
        let named = match scheme {
            Some(scheme) => crate::autoname::name_the_pieces(session, scheme, &kept),
            None => 0,
        };
        let what = match mode {
            StripMode::Strip => "stripped the silence out of",
            StripMode::Extract => "extracted the sound of",
            StripMode::Separate => "separated the clip",
        };
        Ok(Outcome::changed(format!("{what} the clip `{clip_id}`")).with_data(serde_json::json!({
            "id": clip_id,
            "mode": match mode {
                StripMode::Strip => "strip",
                StripMode::Extract => "extract",
                StripMode::Separate => "separate",
            },
            "kept": kept.len(),
            "removed": removed.len(),
            "pieces": pieces,
            "named": named,
            "ranges": ranges.len(),
        })))
    }
}

/// The **loud parts** of a clip, in the clip's own time.
///
/// The parts are found in the **material** — the source the clip plays
/// — and not in the clip's own sound: the clip's sound is the
/// material read the clip's way, and the material's own silence is
/// where the silence is.
///
/// The **threshold** is how far below the loudest part of the material
/// a part may be and still be kept, in decibels. The **minimum
/// length** is how long a part must be to be kept, and the **pad** is
/// how much of the silence is kept at each end of a part.
fn loud_parts_of(
    id: &str,
    session: &Session,
    payload: &ClipPayload,
    threshold_db: Option<Number>,
    min_length_ms: Option<Number>,
    pad_before_ms: Option<Number>,
    pad_after_ms: Option<Number>,
) -> Result<Vec<(i64, i64)>, EngineError> {
    let Some(source_id) = payload.source else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "the clip plays no sound, and there is no silence in it".to_string(),
        });
    };
    let source_node = session.document().node(source_id).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no source `{source_id}`"),
    })?;
    let source = SourcePayload::from_json(&source_node.data)
        .map_err(|error| EngineError::Failed { id: id.to_string(), message: error })?;
    let sound = read_sound(id, &source.path)?;
    let settings = session_of(session.document());
    let rate = settings.sample_rate.max(1) as f64;
    let ratio = sound.rate.max(1) as f64 / rate;
    let threshold_db = match threshold_db {
        Some(value) if value.0.is_finite() => value.0.clamp(-96.0, 0.0),
        Some(_) => {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "the threshold is a number of decibels".to_string(),
            });
        }
        None => -48.0,
    };
    let ms = |value: Option<Number>, default: f64| -> f64 {
        match value {
            Some(value) if value.0.is_finite() => value.0.clamp(0.0, 10_000.0),
            _ => default,
        }
    };
    // The lengths are in **milliseconds**, and a millisecond is a
    // thousandth of a second.
    let min_length = (ms(min_length_ms, 50.0) / 1000.0 * rate) as i64;
    let pad_before = (ms(pad_before_ms, 5.0) / 1000.0 * rate) as i64;
    let pad_after = (ms(pad_after_ms, 20.0) / 1000.0 * rate) as i64;
    // The **loud parts** of the material, in the material's samples.
    let loud = craftkit_audio::loud_ranges(&sound.samples, sound.channels as usize, threshold_db, min_length);
    // And the loud parts in the **clip's own time**.
    let end = payload.start + payload.length;
    let mut ranges: Vec<(i64, i64)> = Vec::new();
    for (from, to) in loud {
        let start = at_of_frame(from as i64, payload.offset, payload.stretch, ratio) - pad_before;
        let stop = at_of_frame(to as i64, payload.offset, payload.stretch, ratio) + pad_after;
        let start = start.max(payload.start);
        let stop = stop.min(end);
        if stop <= start {
            continue;
        }
        if let Some(last) = ranges.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(stop);
            continue;
        }
        ranges.push((start, stop));
    }
    Ok(ranges)
}

/// **Read** the **loud parts** of a clip: where the silence is.
///
/// The tree's editor has a **preview** of its Strip Silence: it finds
/// the silence and says where the parts are, without changing
/// anything — so that a person can see what the strip would do before
/// it does it.
///
/// Here the preview is a **read**: it says the same thing and it does
/// not change the document. The clip is not cut, nothing is thrown
/// away and nothing is named.
#[derive(Command)]
#[command(id = "sound.clip.strip.silence.read", area = "sound", layer = "sound", icon = "list")]
pub struct ReadStripSilence {
    /// The clip to look at.
    pub clip: NodeRef,
    /// How far below the loudest part a part may be and still be kept,
    /// in decibels.
    pub threshold_db: Option<Number>,
    /// How long a part must be to be kept, in milliseconds.
    pub min_length_ms: Option<Number>,
    /// How much silence is kept before each part, in milliseconds.
    pub pad_before_ms: Option<Number>,
    /// How much silence is kept after each part, in milliseconds.
    pub pad_after_ms: Option<Number>,
}

impl ReadStripSilence {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.strip.silence.read".to_string();
        let clip_id = self.clip.0;
        let node = session.document().node(clip_id).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{clip_id}`"),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
            });
        }
        let payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        let ranges = loud_parts_of(
            &id,
            session,
            &payload,
            self.threshold_db,
            self.min_length_ms,
            self.pad_before_ms,
            self.pad_after_ms,
        )?;
        let parts: Vec<Json> = ranges.iter().map(|(from, to)| serde_json::json!([from, to])).collect();
        Ok(Outcome::new(format!("the clip `{clip_id}` has {} loud parts", ranges.len()))
            .with_data(serde_json::json!({ "id": clip_id, "ranges": parts })))
    }
}

/// What the strip of silence does with the pieces of a clip once it
/// has found the silence.
///
/// The tree's editor does this in three modes and names them: `strip`,
/// `extract` and `separate`. The name is a **string** and not a
/// variant, because the kinds of this kit are open — and a name this
/// battery does not know is **refused**, because a mode that does
/// nothing is worse than no mode at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StripMode {
    /// The quiet pieces are thrown away and the loud ones are kept.
    Strip,
    /// The loud pieces are thrown away and the quiet ones are kept.
    Extract,
    /// The clip is cut at the edges of the loud parts and everything
    /// is kept.
    Separate,
}

impl StripMode {
    /// The mode of that name, or **none** when the name is not one.
    fn parse(name: &str) -> Option<StripMode> {
        match name {
            "strip" => Some(StripMode::Strip),
            "extract" => Some(StripMode::Extract),
            "separate" => Some(StripMode::Separate),
            _ => None,
        }
    }
}

/// The **length** of a MIDI clip: the room its notes need, and never
/// less than the room it has.
///
/// The notes are in **ticks** and the length is in **samples**, and the
/// tempo map is what turns one into the other: the room the notes need
/// is measured from the clip's own start, at the tempo of the document
/// where the clip is.
///
/// A clip's room does **not** shrink because its notes moved: the room
/// is the room, and a note that reaches beyond it makes the clip
/// longer. The tree's clip has a length of its own and the notes do
/// not touch it — and here the notes may make the clip longer, but
/// they do not make it shorter.
pub(crate) fn midi_length(
    current: i64,
    midi: &crate::midi::MidiPayload,
    start: i64,
    map: &TempoMap,
    rate: u32,
) -> i64 {
    if midi.notes.is_empty() {
        return current.max(1);
    }
    let start_tick = map.samples_to_ticks(start, rate);
    let end_tick = start_tick.saturating_add(midi.end_tick());
    let room = map.ticks_to_samples(end_tick, rate).saturating_sub(map.ticks_to_samples(start_tick, rate));
    current.max(room).max(1)
}

/// The place on the **timeline** of a frame of the material.
///
/// The material speaks in its own samples and the timeline speaks in
/// the session's, and the clip's **offset** says where in the material
/// the clip starts. A stretched clip reads its material more slowly,
/// so the stretch is in the way back.
///
/// The ratio is the material's rate over the session's: one of the
/// session's samples is this many of the material's.
fn at_of_frame(frame: i64, offset: i64, stretch: f64, ratio: f64) -> i64 {
    (((frame as f64 / ratio.max(f64::EPSILON)) - offset as f64) * stretch).round() as i64
}

/// The most transients a clip may be separated at.
const MAX_TRANSIENT_CUTS: usize = 1024;

/// The most grid lines a clip may be separated on.
///
/// A clip that crosses a thousand lines is a clip that crosses more
/// than the loop is willing to, and the tree's own bound is ten
/// thousand — a bound and not a policy, the same way the tree's is.
const MAX_GRID_LINES: usize = 1024;

/// Space the **clips** against each other.
///
/// The clips are butted together: each one starts where the last one
/// ended, with a **gap** between them. The tree's editor does this when
/// the pieces of a take are laid out by hand.
#[derive(Command)]
#[command(
    id = "sound.clip.space",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip"),
    mutates,
    undoable
)]
pub struct SpaceClips {
    /// The clips to space: their ids.
    pub clips: serde_json::Value,
    /// The gap between them, in samples.
    pub gap: Number,
    /// `end-to-start` — each clip starts where the last one ended — or
    /// `start-to-start`, which keeps their starts the same distance
    /// apart. The first one if it is left out.
    pub mode: Option<Text>,
}

impl SpaceClips {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.space".to_string();
        let list = self.clips.as_array().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the clips to space are a list of their ids".to_string(),
        })?;
        let mut ids: Vec<u64> = Vec::with_capacity(list.len());
        for item in list {
            let Some(number) = item.as_u64() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip's id is a number".to_string(),
                });
            };
            ids.push(number);
        }
        if ids.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "two or more clips are spaced".to_string(),
            });
        }
        let gap = self.gap.0;
        if !gap.is_finite() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a gap is a number of samples".to_string(),
            });
        }
        let gap = gap.round() as i64;
        if gap < 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a gap is not a negative number of samples".to_string(),
            });
        }
        let start_to_start = match &self.mode {
            Some(mode) => match mode.0.trim().to_ascii_lowercase().as_str() {
                "end-to-start" | "end_to_start" => false,
                "start-to-start" | "start_to_start" => true,
                other => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the mode `{other}` is not one of the two"),
                    });
                }
            },
            None => false,
        };
        // The **clips** and where they are now.
        let mut clips: Vec<(u64, i64, i64)> = Vec::with_capacity(ids.len());
        for clip_id in &ids {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            // A clip whose **place** is locked stays where it is — and
            // the whole spacing is refused, because a spacing that
            // left one clip behind would be a spacing the caller did
            // not ask for.
            if locks::time_is_locked(&payload) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` is locked: it stays where it is"),
                });
            }
            clips.push((*clip_id, payload.start, payload.length));
        }
        clips.sort_by_key(|(clip_id, start, _)| (*start, *clip_id));
        // The **new** places: each clip butts against the last one.
        let mut moved: Vec<(u64, i64)> = Vec::with_capacity(clips.len());
        let mut current = clips[0].1;
        for (clip_id, start, length) in &clips {
            let at = if moved.is_empty() { *start } else { current };
            moved.push((*clip_id, at.max(0)));
            current = if start_to_start { at + gap } else { at + length + gap };
        }
        for (clip_id, at) in &moved {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            let mut payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            payload.start = *at;
            if let Some(node) = session.document_mut()?.node_mut(*clip_id) {
                node.data = payload.to_json();
            }
        }
        let count = moved.len();
        Ok(Outcome::changed(format!("spaced {count} clip(s)")).with_data(serde_json::json!({
            "clips": moved.iter().map(|(clip_id, at)| serde_json::json!({ "id": clip_id, "start": at })).collect::<Vec<_>>(),
            "gap": gap,
        })))
    }
}

/// The **relation** that says a clip is followed by its loop.
///
/// The tree's editor remembers a loop in a field on the clip, and has to
/// find the copies again — to take them away, or to draw them as one
/// thing — by looking at what is next to the clip on the track.
///
/// Here the loop is an **edge**: the copy is named by the clip it
/// follows, and the chain is a walk. Nothing has to be guessed.
const LOOP_RELATION: &str = "loop";

/// The most **loops** one clip may be given.
///
/// A clip that loops a thousand times is a thousand bars on the
/// timeline and a thousand copies of the clip in the document. The
/// bound is here so that a number from outside cannot ask for an
/// absurd thing — and the answer is a **refusal**, and not a shorter
/// loop: a loop that is not the loop that was asked for is a loop that
/// is wrong in a way nobody can hear.
const MAX_LOOPS: usize = 1000;

/// How many times a clip is played when nothing else is said.
///
/// The tree's dialog asks; the tree's default is the clip and three
/// loops.
const DEFAULT_LOOPS: i64 = 4;

/// Put a clip **in front** of the others on its track, or **behind**
/// them.
///
/// The order the clips are drawn in is the order they are in — and the
/// clips of a track are mixed and not laid over one another, so the
/// order is what a person sees and not what a person hears.
#[derive(Command)]
#[command(
    id = "sound.clip.order",
    area = "sound",
    layer = "sound",
    icon = "list",
    menu("Clip", "Order"),
    mutates,
    undoable
)]
pub struct OrderClip {
    /// The clip to move.
    pub clip: NodeRef,
    /// `front` or `back`.
    pub place: Text,
}

impl OrderClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.order".to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let place = self.place.0.trim().to_ascii_lowercase();
        let moved = match place.as_str() {
            "front" => session.document_mut()?.bring_to_front(self.clip.0),
            "back" => session.document_mut()?.send_to_back(self.clip.0),
            other => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a clip goes to the front or to the back, and not to `{other}`"),
                });
            }
        };
        if !moved {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is not in the document", self.clip.0),
            });
        }
        Ok(Outcome::changed(format!("the clip is {place}"))
            .with_data(serde_json::json!({ "id": self.clip.0, "place": place })))
    }
}

/// Loop a clip: the copies of it that follow it.
///
/// The tree's editor pastes a copy of the clip onto the track, and the
/// copies are ordinary clips — they may be trimmed, muted and faded on
/// their own. This tree does the same, and says what the tree can only
/// infer: the copy is **named by the clip it follows**, in a relation,
/// so that the loop can be found again without guessing.
///
/// Three ways to say how long the loop is:
///
/// - **count** — the clip and its loops: four is the clip and three
///   copies;
/// - **length** — how long the whole looped clip is, in samples, and
///   the last copy is cut short to fit;
/// - **fill** — up to the next clip on the track, or else to the end of
///   the timeline.
///
/// They are taken in that order: a fill beats a length, and a length
/// beats a count.
#[derive(Command)]
#[command(
    id = "sound.clip.loop",
    area = "sound",
    layer = "sound",
    icon = "link",
    menu("Clip", "Loop"),
    mutates,
    undoable
)]
pub struct LoopClip {
    /// The clip to loop.
    pub clip: NodeRef,
    /// How many times the clip is played: the clip and its loops.
    pub count: Option<Number>,
    /// How long the whole looped clip is, in samples. The last loop is
    /// cut short to fit.
    pub length: Option<Number>,
    /// Loop up to the next clip on the track, or else to the end of the
    /// timeline.
    pub fill: Option<Bool>,
}

impl LoopClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.loop".to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let payload = ClipPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        if payload.length <= 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a clip with no length is not a clip".to_string(),
            });
        }
        // The **playlist** the clip is in: the copies go beside it.
        let playlist =
            timeline::parent_of(session.document(), self.clip.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is on no playlist", self.clip.0),
            })?;
        // A clip that is **already followed** by a copy is refused: the
        // chain is a line and not a tree, and a branch would have two
        // answers to the same question.
        if !session.document().relations_from(self.clip.0, Some(LOOP_RELATION)).is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is already looped: unloop it first", self.clip.0),
            });
        }
        // How many copies, and how long the last one is.
        let (reps, last) = self.the_plan(session, node, &payload)?;
        if reps == 0 {
            return Ok(Outcome::new("there is no room for a loop").with_data(serde_json::json!({
                "loops": 0,
                "clips": Vec::<u64>::new(),
            })));
        }
        // The **copies**: each one starts where the last one ended, and
        // the last one is cut short to fit.
        let name = node.name.clone();
        let playlist_id = playlist.id;
        let mut made: Vec<u64> = Vec::with_capacity(reps);
        let mut previous = self.clip.0;
        for step in 1..=reps {
            let mut copy = payload.clone();
            copy.start = payload.start.saturating_add(payload.length.saturating_mul(step as i64));
            if step == reps && last < payload.length {
                copy.length = last;
                // The **fades** are shortened to fit: a fade that is
                // longer than the clip is a fade past the end of it.
                copy.fade_in.len = copy.fade_in.len.min(copy.length);
                copy.fade_out.len = copy.fade_out.len.min(copy.length);
                // And a **beat** past the cut is not a beat of the
                // copy: the copy is shorter, and the beat is gone.
                copy.sync = copy.sync.filter(|beat| (0..=copy.length).contains(beat));
            }
            let made_id = session.document_mut()?.add_node_with_under(
                Some(playlist_id),
                name.clone(),
                "sound.clip",
                None,
                copy.to_json(),
            );
            // The copy plays the same source, and it is said so.
            if let Some(source) = payload.source {
                session.document_mut()?.add_relation(craftkit_sdk::Relation::new(made_id, source, "source"));
            }
            // And the copy **follows** the clip before it: that is the
            // loop, and it is an edge and not a guess.
            session.document_mut()?.add_relation(craftkit_sdk::Relation::new(
                previous,
                made_id,
                LOOP_RELATION,
            ));
            previous = made_id;
            made.push(made_id);
        }
        let count = made.len();
        Ok(Outcome::changed(format!("looped the clip {count} time(s)")).with_data(serde_json::json!({
            "loops": count,
            "clips": made,
        })))
    }

    /// How many **copies** the loop has, and how long the last one is.
    ///
    /// The three ways to say it are taken in order: a fill beats a
    /// length, and a length beats a count.
    fn the_plan(
        &self,
        session: &Session,
        node: &Node,
        payload: &ClipPayload,
    ) -> Result<(usize, i64), EngineError> {
        let id = "sound.clip.loop".to_string();
        // **Fill**: up to the next clip on the track, or else to the end
        // of the timeline.
        if self.fill.map(|fill| fill.0) == Some(true) {
            let end = the_next_start(session.document(), node, payload)
                .unwrap_or_else(|| timeline::end_in_samples(session.document()));
            let span = end.saturating_sub(payload.start.saturating_add(payload.length));
            return Self::the_reps(span, payload.length, &id);
        }
        // **Length**: the whole looped clip is this long, and the last
        // copy is cut short to fit.
        if let Some(length) = self.length {
            let total = samples(length.0, "how long the looped clip is")?;
            let span = total.saturating_sub(payload.length);
            return Self::the_reps(span, payload.length, &id);
        }
        // **Count**: the clip and its loops.
        let count = match self.count {
            Some(count) => samples(count.0, "how many times the clip is played")?,
            None => DEFAULT_LOOPS,
        };
        if count < 1 {
            return Err(EngineError::Failed {
                id,
                message: "a clip that is played no times is not a clip".to_string(),
            });
        }
        let reps = count - 1;
        if reps > MAX_LOOPS as i64 {
            return Err(EngineError::Failed {
                id,
                message: format!("a clip loops at most {MAX_LOOPS} times, and this one was asked for {reps}"),
            });
        }
        Ok((reps as usize, payload.length))
    }

    /// How many copies fit in a **span**, and how long the last one is.
    ///
    /// A span that is not a whole number of clips has a last copy that
    /// is cut short — which is the tree's own rule, and the reason a
    /// loop of a whole number of bars ends exactly where it began.
    fn the_reps(span: i64, length: i64, id: &str) -> Result<(usize, i64), EngineError> {
        if span <= 0 || length <= 0 {
            return Ok((0, length));
        }
        let whole = span / length;
        let rest = span % length;
        let reps = whole + i64::from(rest > 0);
        if reps > MAX_LOOPS as i64 {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("a clip loops at most {MAX_LOOPS} times, and this one would need {reps}"),
            });
        }
        let last = if rest > 0 { rest } else { length };
        Ok((reps as usize, last))
    }
}

/// Unloop a clip: take the copies away.
///
/// The tree's editor has to **find** the copies before it can take them
/// away, and it finds them by looking at the track: a clip that starts
/// where this one ends, of the same material. This tree knows them —
/// the copy is named by the clip it follows — so the copies are found
/// by walking the chain.
///
/// Two ways to be rid of a loop:
///
/// - **remove** — the copies are taken away, and the clip is a clip
///   again;
/// - **flatten** — the copies stay, and they are not a loop any more:
///   they are clips in their own right.
#[derive(Command)]
#[command(
    id = "sound.clip.unloop",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Clip", "Unloop"),
    mutates,
    undoable
)]
pub struct UnloopClip {
    /// The clip whose loop is to be taken away.
    pub clip: NodeRef,
    /// `remove` — the copies are taken away. `flatten` — the copies
    /// stay, and are not a loop any more. The first one if it is left
    /// out.
    pub mode: Option<Text>,
}

impl UnloopClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.unloop".to_string();
        let node = session.document().node(self.clip.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{}`", self.clip.0),
        })?;
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{}` is a `{}` and not a clip", self.clip.0, node.kind),
            });
        }
        let remove = match &self.mode {
            None => true,
            Some(mode) => match mode.0.trim().to_ascii_lowercase().as_str() {
                "remove" => true,
                "flatten" => false,
                other => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the mode `{other}` is not one of the two"),
                    });
                }
            },
        };
        // The **chain**: the clip, and the copies that follow it.
        let chain = the_chain(session.document(), self.clip.0);
        if chain.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{}` is not a looped clip", self.clip.0),
            });
        }
        let copies = chain.len() - 1;
        if remove {
            // The copies go, and their edges go with them: a document
            // that keeps the edge to a clip that is not there is a
            // document that says a clip plays a source that is gone.
            for copy in chain.iter().skip(1) {
                session.document_mut()?.remove_node(*copy);
            }
        } else {
            // The copies stay, and the **edges** go: they are clips in
            // their own right, and nothing says they are a loop.
            let members = chain.clone();
            session.document_mut()?.remove_relations(|relation| {
                relation.kind == LOOP_RELATION
                    && members.contains(&relation.from)
                    && members.contains(&relation.to)
            });
        }
        let message = if remove {
            format!("took away {copies} cop(y|ies) of the loop")
        } else {
            format!("the {copies} cop(y|ies) are not a loop any more")
        };
        Ok(Outcome::changed(message).with_data(serde_json::json!({
            "copies": copies,
            "removed": remove,
            "clips": chain.iter().skip(1).collect::<Vec<_>>(),
        })))
    }
}

/// The **chain** of a loop: the clip and the copies that follow it.
///
/// The chain is a walk: back along the edges to the head, and then
/// forward along the copies. A clip that is in no loop is a chain of
/// one.
fn the_chain(document: &craftkit_sdk::document::Document, clip: u64) -> Vec<u64> {
    // The **head**: the clip that nothing follows into.
    let mut head = clip;
    loop {
        let incoming = document.relations_to(head, Some(LOOP_RELATION));
        let Some(relation) = incoming.first() else { break };
        let from = relation.from;
        if from == head {
            break;
        }
        head = from;
    }
    // The **copies**: the head, and then each one after it.
    let mut chain = vec![head];
    let mut current = head;
    loop {
        let outgoing = document.relations_from(current, Some(LOOP_RELATION));
        let Some(relation) = outgoing.first() else { break };
        let to = relation.to;
        if chain.contains(&to) {
            break;
        }
        chain.push(to);
        current = to;
    }
    chain
}

/// The start of the **next clip** on the clip's track, if there is one.
///
/// The tree's editor looks for the next clip that starts at or after
/// this one ends: a clip that overlaps is not a neighbour, because the
/// loop would run into the middle of it.
fn the_next_start(
    document: &craftkit_sdk::document::Document,
    clip: &Node,
    payload: &ClipPayload,
) -> Option<i64> {
    let track = timeline::track_of(document, clip)?;
    let end = payload.start.saturating_add(payload.length);
    let mut next: Option<i64> = None;
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        if node.id == clip.id {
            continue;
        }
        if timeline::track_of(document, node).map(|owner| owner.id) != Some(track.id) {
            continue;
        }
        let Ok(other) = ClipPayload::from_json(&node.data) else { continue };
        if other.start < end {
            continue;
        }
        next = Some(next.map_or(other.start, |best: i64| best.min(other.start)));
    }
    next
}

/// Snap a clip to its **neighbour**.
///
/// The tree's editor has a menu — *Edit ▸ Snap to* — with two items in
/// it: **Next** moves a clip so that its end meets the start of the
/// clip that follows it, and **Previous** moves a clip so that its
/// start meets the end of the clip that comes before it.
///
/// The tree's editor snaps the clip **nearest the destination first**,
/// so that the neighbours stay where they are: a row of clips that are
/// snapped together does not pile itself up on one clip.
///
/// The clips that are named are the destinations too. The tree's
/// editor found that out the hard way — ignoring the selected
/// neighbours made *Snap to Next* pile every selected clip onto the
/// same later clip, and made *Snap to Previous* leave the gap between
/// the selected clips in place.
#[derive(Command)]
#[command(
    id = "sound.clip.snap",
    area = "sound",
    layer = "sound",
    icon = "trim",
    menu("Clip", "Snap to"),
    mutates,
    undoable
)]
pub struct SnapClip {
    /// The clips to snap: their ids.
    pub clips: serde_json::Value,
    /// `next` — each clip's end meets the start of the clip that
    /// follows it. `previous` — each clip's start meets the end of the
    /// clip that comes before it.
    pub direction: Text,
}

impl SnapClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.clip.snap".to_string();
        let list = self.clips.as_array().ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: "the clips to snap are a list of their ids".to_string(),
        })?;
        let mut ids: Vec<u64> = Vec::with_capacity(list.len());
        for item in list {
            let Some(number) = item.as_u64() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a clip's id is a number".to_string(),
                });
            };
            ids.push(number);
        }
        if ids.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "there is no clip to snap".to_string(),
            });
        }
        let next = match self.direction.0.trim().to_ascii_lowercase().as_str() {
            "next" => true,
            "previous" => false,
            other => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the direction `{other}` is not one of the two"),
                });
            }
        };
        // The **clips** and where they are now.
        let mut order: Vec<(u64, i64, i64, u64)> = Vec::with_capacity(ids.len());
        for clip_id in &ids {
            let node = session.document().node(*clip_id).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip_id}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip_id}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            // A clip whose **place** is locked stays where it is — and
            // the whole snap is refused, because a snap that left one
            // clip behind would be a snap the caller did not ask for.
            if locks::time_is_locked(&payload) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` is locked: it stays where it is"),
                });
            }
            // The **list** the clip is in: the playlist under the
            // track, or the track itself. A clip's neighbours are the
            // clips of its own list — a take is a list of clips of its
            // own, and a clip in another take is not on this timeline.
            let list =
                timeline::parent_of(session.document(), *clip_id).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip_id}` is on no playlist"),
                })?;
            order.push((*clip_id, payload.start, payload.length, list.id));
        }
        // The clip **nearest the destination** is snapped first, so that
        // the neighbours stay where they are.
        order.sort_by_key(|(_, start, _, _)| if next { -*start } else { *start });
        let mut moved: Vec<(u64, i64)> = Vec::with_capacity(order.len());
        for (clip_id, start, length, list) in order {
            let end = start.saturating_add(length);
            // The **neighbour**: the clip the destination is. The clips
            // that are named are neighbours too.
            let delta = if next {
                clips_in(session.document(), list)
                    .into_iter()
                    .filter(|(other, _)| *other != clip_id)
                    .map(|(_, payload)| payload.start)
                    .filter(|other_start| *other_start >= end)
                    .min()
                    .map(|other_start| other_start - end)
            } else {
                clips_in(session.document(), list)
                    .into_iter()
                    .filter(|(other, _)| *other != clip_id)
                    .map(|(_, payload)| (payload.start, payload.start.saturating_add(payload.length)))
                    .filter(|(_, other_end)| *other_end <= start)
                    .map(|(_, other_end)| other_end)
                    .max()
                    .map(|other_end| other_end - start)
            };
            // A clip that is already against its neighbour is not
            // moved: there is nothing to snap.
            let Some(delta) = delta.filter(|delta| *delta != 0) else { continue };
            let at = start.saturating_add(delta).max(0);
            if let Some(node) = session.document_mut()?.node_mut(clip_id) {
                let mut payload = ClipPayload::from_json(&node.data)
                    .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
                payload.start = at;
                node.data = payload.to_json();
            }
            moved.push((clip_id, at));
        }
        let count = moved.len();
        Ok(Outcome::changed(format!("snapped {count} clip(s)")).with_data(serde_json::json!({
            "moved": count,
            "clips": moved.iter().map(|(clip_id, at)| serde_json::json!({ "id": clip_id, "start": at })).collect::<Vec<_>>(),
        })))
    }
}

/// The **clips** in a list, with what they say.
///
/// A list is a playlist under a track, or a track that holds its clips
/// itself. A clip's neighbours are the clips of the list it is in.
fn clips_in(document: &craftkit_sdk::document::Document, list: u64) -> Vec<(u64, ClipPayload)> {
    let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        if timeline::parent_of(document, node.id).map(|parent| parent.id) != Some(list) {
            continue;
        }
        let Ok(payload) = ClipPayload::from_json(&node.data) else { continue };
        clips.push((node.id, payload));
    }
    clips
}

/// Set the tempo.
///
/// The tempo map is a **list of events** and not a single number, so this
/// command sets the tempo at a tick: a second call at the same tick
/// changes the one that is there.
#[derive(Command)]
#[command(
    id = "sound.tempo.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Tempo"),
    mutates,
    undoable
)]
pub struct SetTempo {
    /// How fast, in beats a minute.
    pub bpm: Number,
    /// Where, in ticks.
    pub tick: Option<Number>,
}

impl SetTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let bpm = self.bpm.0;
        if !bpm.is_finite() || !(5.0..=1000.0).contains(&bpm) {
            return Err(EngineError::Failed {
                id: "sound.tempo.set".to_string(),
                message: format!("{bpm} is not a tempo between 5 and 1000 beats a minute"),
            });
        }
        let tick = match &self.tick {
            Some(tick) => ticks(tick.0)?,
            None => 0,
        };
        let payload = TempoPayload { tick, bpm };
        let existing = find_event(session, "sound.tempo", tick);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = payload.to_json();
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                format!("tempo {bpm}"),
                "sound.tempo",
                None,
                payload.to_json(),
            ),
        };
        Ok(Outcome::changed(format!("the tempo at tick {tick} is {bpm} beats a minute"))
            .with_data(serde_json::json!({ "id": id, "tick": tick, "bpm": bpm })))
    }
}

/// **Take away** the tempo at a tick.
///
/// The tempo map is a list of events, and a map that can only grow is
/// not a map a person can work with: the tempo at a tick goes away as
/// well as the one that is put there.
///
/// The tempo at the **first tick** is refused: it is the document's
/// tempo, and a document without one has no beat. A tick with **no
/// tempo** is refused too: a tempo that was never there is not a tempo
/// that was taken away.
#[derive(Command)]
#[command(
    id = "sound.tempo.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Event", "Tempo"),
    mutates,
    undoable
)]
pub struct RemoveTempo {
    /// Where the tempo is, in ticks.
    pub tick: Number,
}

impl RemoveTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.tempo.remove".to_string();
        let tick = ticks(self.tick.0)?;
        if tick == 0 {
            return Err(EngineError::Failed {
                id,
                message: "the tempo at the first tick is the document's tempo and it stays".to_string(),
            });
        }
        let Some(node) = find_event(session, "sound.tempo", tick) else {
            return Err(EngineError::Failed { id, message: format!("there is no tempo at tick {tick}") });
        };
        session.document_mut()?.remove_node(node);
        Ok(Outcome::changed(format!("the tempo at tick {tick} is gone"))
            .with_data(serde_json::json!({ "id": node, "tick": tick })))
    }
}

/// **Take away** the meter at a tick.
///
/// The meter is a list of events, and the same rule the tempo follows
/// applies to it: the event at the **first tick** is the document's
/// meter and it stays, and a tick with **no meter** is refused.
#[derive(Command)]
#[command(
    id = "sound.meter.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Event", "Meter"),
    mutates,
    undoable
)]
pub struct RemoveMeter {
    /// Where the meter is, in ticks.
    pub tick: Number,
}

impl RemoveMeter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.meter.remove".to_string();
        let tick = ticks(self.tick.0)?;
        if tick == 0 {
            return Err(EngineError::Failed {
                id,
                message: "the meter at the first tick is the document's meter and it stays".to_string(),
            });
        }
        let Some(node) = find_event(session, "sound.meter", tick) else {
            return Err(EngineError::Failed { id, message: format!("there is no meter at tick {tick}") });
        };
        session.document_mut()?.remove_node(node);
        Ok(Outcome::changed(format!("the meter at tick {tick} is gone"))
            .with_data(serde_json::json!({ "id": node, "tick": tick })))
    }
}

/// **Take away** the key at a tick.
///
/// The key is a list of events too, and the same rule the tempo
/// follows applies to it: the event at the **first tick** is the
/// document's key and it stays, and a tick with **no key** is refused.
#[derive(Command)]
#[command(
    id = "sound.key.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("Event", "Key"),
    mutates,
    undoable
)]
pub struct RemoveKey {
    /// Where the key is, in ticks.
    pub tick: Number,
}

impl RemoveKey {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.key.remove".to_string();
        let tick = ticks(self.tick.0)?;
        if tick == 0 {
            return Err(EngineError::Failed {
                id,
                message: "the key at the first tick is the document's key and it stays".to_string(),
            });
        }
        let Some(node) = find_event(session, "sound.key", tick) else {
            return Err(EngineError::Failed { id, message: format!("there is no key at tick {tick}") });
        };
        session.document_mut()?.remove_node(node);
        Ok(Outcome::changed(format!("the key at tick {tick} is gone"))
            .with_data(serde_json::json!({ "id": node, "tick": tick })))
    }
}

/// Set the meter.
///
/// The meter is a list of events too: a bar may be three beats long from
/// here on.
#[derive(Command)]
#[command(
    id = "sound.meter.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Meter"),
    mutates,
    undoable
)]
pub struct SetMeter {
    /// How many beats a bar has.
    pub numerator: Number,
    /// Which beat a bar has.
    pub denominator: Number,
    /// Where, in ticks.
    pub tick: Option<Number>,
}

impl SetMeter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let numerator = self.numerator.0 as u32;
        let denominator = self.denominator.0 as u32;
        if !(1..=64).contains(&numerator) || ![1, 2, 4, 8, 16, 32, 64].contains(&denominator) {
            return Err(EngineError::Failed {
                id: "sound.meter.set".to_string(),
                message: format!("{numerator}/{denominator} is not a meter"),
            });
        }
        let tick = match &self.tick {
            Some(tick) => ticks(tick.0)?,
            None => 0,
        };
        let payload = MeterPayload { tick, numerator, denominator };
        let existing = find_event(session, "sound.meter", tick);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = payload.to_json();
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                format!("meter {numerator}/{denominator}"),
                "sound.meter",
                None,
                payload.to_json(),
            ),
        };
        Ok(Outcome::changed(format!("the meter at tick {tick} is {numerator}/{denominator}"))
            .with_data(serde_json::json!({ "id": id, "tick": tick })))
    }
}

/// **Move** a marker, or **rename** it.
///
/// The marker's **name** is what a person calls it, and its **place**
/// is where it is on the timeline, and both of them may be changed: a
/// marker is a thing in the document and not a thing that is put there
/// once and for ever.
#[derive(Command)]
#[command(
    id = "sound.marker.set",
    area = "sound",
    layer = "sound",
    icon = "pencil",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct SetMarker {
    /// The marker to change.
    pub marker: NodeRef,
    /// The new name.
    pub name: Option<Text>,
    /// The new place, in samples.
    pub start: Option<Number>,
    /// The new end, in samples.
    pub end: Option<Number>,
}

impl SetMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.marker.set".to_string();
        let marker = self.marker.0;
        let node = session.document().node(marker).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{marker}`"),
        })?;
        if node.kind != "sound.marker" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{marker}` is a `{}` and not a marker", node.kind),
            });
        }
        let mut payload = MarkerPayload::from_json(&node.data)
            .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
        // The marker moves as a **whole**: its span is kept, so that
        // moving it does not make it longer or shorter than it was.
        if let Some(start) = self.start {
            let start = samples(start.0, "where the marker is")?;
            let delta = start - payload.start;
            payload.start = start;
            payload.end += delta;
        }
        if let Some(end) = self.end {
            payload.end = samples(end.0, "where the marker ends")?;
        }

        if payload.end < payload.start {
            return Err(EngineError::Failed {
                id,
                message: format!("the marker ends at {} and starts at {}", payload.end, payload.start),
            });
        }
        let name = self.name.as_ref().map(|name| name.0.clone()).unwrap_or(node.name.clone());
        if let Some(node) = session.document_mut()?.node_mut(marker) {
            node.name = name.clone();
            node.data = payload.to_json();
        }
        Ok(Outcome::changed(format!("changed the marker `{name}`"))
            .with_data(serde_json::json!({ "marker": marker, "name": name, "start": payload.start })))
    }
}

/// **Scale** the tempos of the document.
///
/// Every tempo is multiplied by the **factor**: a factor of two is
/// twice as fast everywhere, and the times the tempos happen at do not
/// move — the music is faster, and it starts at the same moment.
///
/// A factor that would take a tempo outside the range a tempo may have
/// — five to a thousand beats a minute — is **refused**: a tempo of
/// zero is not a tempo, and a tempo of two thousand is not one either.
#[derive(Command)]
#[command(
    id = "sound.tempo.scale",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Tempo"),
    mutates,
    undoable
)]
pub struct ScaleTempo {
    /// The factor every tempo is multiplied by.
    pub factor: Number,
}

impl ScaleTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.tempo.scale".to_string();
        let factor = self.factor.0;
        if !factor.is_finite() || factor <= 0.0 {
            return Err(EngineError::Failed {
                id,
                message: format!("the factor is a number above zero, and {factor} is not one"),
            });
        }
        // Every tempo is read first, and the whole scale is refused if
        // one of them would leave the range: a scale that is half made
        // is a map that says something false about half of itself.
        let mut tempos: Vec<(u64, i64, f64)> = Vec::new();
        for node in session.document().walk() {
            if node.kind != "sound.tempo" {
                continue;
            }
            let Ok(payload) = TempoPayload::from_json(&node.data) else { continue };
            let scaled = payload.bpm * factor;
            if !scaled.is_finite() || !(5.0..=1000.0).contains(&scaled) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the tempo of {} beats a minute would become {scaled}, which is not a tempo",
                        payload.bpm
                    ),
                });
            }
            tempos.push((node.id, payload.tick, scaled));
        }
        for (node, tick, bpm) in &tempos {
            if let Some(node) = session.document_mut()?.node_mut(*node) {
                node.data = TempoPayload { tick: *tick, bpm: *bpm }.to_json();
                node.name = format!("tempo {bpm}");
            }
        }
        Ok(Outcome::changed(format!("scaled {} tempos by {factor}", tempos.len()))
            .with_data(serde_json::json!({ "tempos": tempos.len(), "factor": factor })))
    }
}

/// Make a **range** of the tempo map constant.
///
/// The tempo at the start of the range is the one given, the tempo
/// changes inside the range are **taken away**, and the tempo at the
/// end of the range comes back — so that the music after the range is
/// as fast as it was.
///
/// The tree's command is the same one, and the tree's own name for it
/// is "Constant...".
#[derive(Command)]
#[command(
    id = "sound.tempo.constant",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Event", "Tempo"),
    mutates,
    undoable
)]
pub struct ConstantTempo {
    /// How fast, in beats a minute.
    pub bpm: Number,
    /// Where the range starts, in ticks.
    pub start: Option<Number>,
    /// Where the range ends, in ticks.
    pub end: Option<Number>,
}

impl ConstantTempo {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.tempo.constant".to_string();
        let bpm = self.bpm.0;
        if !bpm.is_finite() || !(5.0..=1000.0).contains(&bpm) {
            return Err(EngineError::Failed {
                id,
                message: format!("{bpm} is not a tempo between 5 and 1000 beats a minute"),
            });
        }
        let start = match self.start {
            Some(start) => ticks(start.0)?,
            None => 0,
        };
        let end = match self.end {
            Some(end) => ticks(end.0)?,
            None => start,
        };
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range from {start} to {end} is empty"),
            });
        }
        // The tempo at the end of the range is read **before** the
        // edit: the end is a musical place, and the new tempo moves it.
        let map = tempo_map(session.document());
        let after = map.tempo_at_tick(end);
        let had_end = find_event(session, "sound.tempo", end).is_some();
        // The tempo at the start of the range.
        match find_event(session, "sound.tempo", start) {
            Some(node) => {
                if let Some(node) = session.document_mut()?.node_mut(node) {
                    node.data = TempoPayload { tick: start, bpm }.to_json();
                    node.name = format!("tempo {bpm}");
                }
            }
            None => {
                session.document_mut()?.add_node_with(
                    format!("tempo {bpm}"),
                    "sound.tempo",
                    None,
                    TempoPayload { tick: start, bpm }.to_json(),
                );
            }
        }
        // The tempo changes **inside** the range are taken away.
        let inside: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.tempo")
            .filter(|node| {
                TempoPayload::from_json(&node.data)
                    .map(|payload| payload.tick > start && payload.tick < end)
                    .unwrap_or(false)
            })
            .map(|node| node.id)
            .collect();
        for node in inside {
            session.document_mut()?.remove_node(node);
        }
        // And the tempo at the end of the range comes back.
        if !had_end {
            session.document_mut()?.add_node_with(
                format!("tempo {after}"),
                "sound.tempo",
                None,
                TempoPayload { tick: end, bpm: after }.to_json(),
            );
        }
        Ok(Outcome::changed(format!("the tempo from tick {start} to {end} is {bpm}"))
            .with_data(serde_json::json!({ "start": start, "end": end, "bpm": bpm })))
    }
}

/// Put a marker in the document.
#[derive(Command)]
#[command(
    id = "sound.marker.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("Marker"),
    mutates,
    undoable
)]
pub struct AddMarker {
    /// What to call it.
    pub name: Text,
    /// Where it is, in samples.
    pub start: Number,
    /// Where it ends, in samples.
    pub end: Option<Number>,
}

impl AddMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let start = samples(self.start.0, "where the marker is")?;
        let end = match &self.end {
            Some(end) => samples(end.0, "where the marker ends")?,
            None => start,
        };
        let payload = MarkerPayload { start, end };
        let name = self.name.0.clone();
        let id = session.document_mut()?.add_node_with(name.clone(), "sound.marker", None, payload.to_json());
        Ok(Outcome::changed(format!("added a marker `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name, "start": start, "end": end })))
    }
}

/// Add a source: a reference to media.
///
/// The path is **relative to the document**, because a document that
/// travels carries its media with it. Nothing here reads the file.
#[derive(Command)]
#[command(
    id = "sound.source.add",
    area = "sound",
    layer = "sound",
    icon = "plus",
    menu("File", "Import"),
    mutates,
    undoable
)]
pub struct AddSource {
    /// Where the media is.
    pub path: Text,
    /// How many channels it has.
    pub channels: Option<Number>,
    /// How long it is, in frames.
    pub frames: Option<Number>,
}

impl AddSource {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let channels = match &self.channels {
            Some(channels) => channels.0 as u32,
            None => 2,
        };
        let frames = match &self.frames {
            Some(frames) => samples(frames.0, "how long the source is")?,
            None => 0,
        };
        let session_settings = session_of(session.document());
        let payload = SourcePayload {
            path: path.clone(),
            channels,
            frames,
            sample_rate: session_settings.sample_rate,
            peaks: Vec::new(),
        };
        let id = session.document_mut()?.add_node_with(path.clone(), "sound.source", None, payload.to_json());
        // The document now says that it needs this file. The bytes are
        // not in the document: they travel beside it, in a bundle.
        session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path.clone(), "audio", None));
        Ok(Outcome::changed(format!("added a source `{path}`"))
            .with_data(serde_json::json!({ "id": id, "path": path })))
    }
}

/// **Take away** a source.
///
/// The source goes and so does the **asset** it brought: the document
/// says it needs the file, and a document that needs a file that
/// nothing plays is carrying a file for nothing. The clips that played
/// it keep their own word about the source — the clip is silent until
/// it is told what to play — and the source itself is gone.
///
/// The tree's mixer has no such command: the tree's source is a file
/// that the session names and the session does not lose.
#[derive(Command)]
#[command(
    id = "sound.source.remove",
    area = "sound",
    layer = "sound",
    icon = "minus",
    menu("File", "Remove Source"),
    mutates,
    undoable
)]
pub struct RemoveSource {
    /// The source to take away.
    pub source: NodeRef,
}

impl RemoveSource {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.source.remove".to_string();
        let source = self.source.0;
        let node = session.document().node(source).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no source `{source}`"),
        })?;
        if node.kind != "sound.source" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{source}` is a `{}` and not a source", node.kind),
            });
        }
        let path = node.name.clone();
        session.document_mut()?.remove_node(source);
        // The **asset** goes with it: the document does not need a file
        // that nothing plays.
        let removed = session.document_mut()?.remove_assets(|asset| asset.path == path);
        Ok(Outcome::changed(format!("took the source `{path}` away"))
            .with_data(serde_json::json!({ "source": source, "path": path, "assets": removed })))
    }
}

/// **Measure** the peaks of a source.
///
/// The peaks are the shape of the sound in a small table, and the shape
/// is what a clip's bar is drawn from.
///
/// Measuring them reads the file, so it is a **command** and not a
/// thing that happens at every frame: a file that is read a hundred
/// times a second is a file that is read a hundred times too often.
/// The measurement is in the document, so it is undone with everything
/// else and it is saved with the document.
#[derive(Command)]
#[command(id = "sound.source.peaks", area = "sound", layer = "sound", icon = "list", mutates, undoable)]
pub struct MeasurePeaks {
    /// The source to measure.
    pub source: NodeRef,
}

impl MeasurePeaks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.source.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("there is no node `{id}`"),
            });
        };
        if node.kind != "sound.source" {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("the node `{id}` is a `{}` and not a source", node.kind),
            });
        }
        let payload = SourcePayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.source.peaks".to_string(),
            message: format!("the source `{id}` cannot be read: {error}"),
        })?;
        let sound = read_sound("sound.source.peaks", &payload.path)?;
        let peaks = peaks_of(&sound);
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.source.peaks".to_string(),
                message: format!("there is no source `{id}`"),
            });
        };
        // The file says what it is: how long it is, how many channels
        // it has and the rate it was recorded at. A source that has
        // not been measured does not know.
        node.data["frames"] = serde_json::json!(sound.frames as i64);
        node.data["channels"] = serde_json::json!(sound.channels as u32);
        node.data["sample_rate"] = serde_json::json!(sound.rate);
        node.data["peaks"] = serde_json::json!(peaks);
        Ok(Outcome::changed(format!("measured the peaks of the source {id}")).with_data(serde_json::json!({
            "id": id,
            "numbers": PEAK_NUMBERS,
            "frames": sound.frames as i64,
            "channels": sound.channels as u32,
            "sample_rate": sound.rate,
        })))
    }
}

/// The kind of a **playhead**: where the sound is.
///
/// The playhead is a **node** because the playhead has a **setting** —
/// the position — and a setting is a thing in a document, and a node is
/// where settings live: the same rule as the ruler's scale (§131).
///
/// The position is a **number of samples** and not an event: the
/// transport is not a stream of "and then" messages, it is a place,
/// and everything else is worked out from it.
#[derive(NodeKind)]
#[node_kind(kind = "sound.playhead", label = "Playhead", layer = "sound", evaluates, place)]
pub struct PlayheadKind;

impl PlayheadKind {
    /// How a playhead is drawn: a **line** down the timeline.
    ///
    /// The line runs from the ruler's row through the tracks, because a
    /// playhead is where the sound is and the sound is under the ruler.
    fn paint(node: &Node, document: &Document, canvas: &mut dyn Canvas) {
        let Some(rect) = timeline::playhead_rect(document, node) else { return };
        let line = craftkit_sdk::document::Path::polygon(&[
            Point::new(rect.x + rect.w / 2.0, rect.y),
            Point::new(rect.x + rect.w / 2.0, rect.y + rect.h),
        ]);
        canvas.path(
            &line,
            craftkit_sdk::document::Fill::Solid(craftkit_sdk::document::Colour::NONE),
            craftkit_sdk::document::BlendMode::Normal,
            Some(craftkit_sdk::document::Stroke {
                colour: craftkit_sdk::document::Colour::BLACK,
                width: 0.01,
                ..Default::default()
            }),
        );
    }

    /// Whether a point is on the playhead.
    fn hit(node: &Node, document: &Document, point: Point) -> bool {
        timeline::playhead_rect(document, node).is_some_and(|rect| rect.contains(point))
    }

    /// Where the playhead **is**: the line at the position.
    fn place(node: &Node, document: &Document) -> Option<craftkit_sdk::document::Rect> {
        timeline::playhead_rect(document, node)
    }

    /// What a playhead **means**: where it is, in samples and in
    /// seconds.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let position = timeline::playhead_position(node)?;
        let rate = session_of(document).sample_rate;
        Some(serde_json::json!({
            "position": position,
            "seconds": position as f64 / rate as f64,
        }))
    }
}

/// Put the **playhead** where it is, or move it.
///
/// The playhead is a **node**, so there is one of it and it is moved.
#[derive(Command)]
#[command(
    id = "sound.playhead.set",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Playhead"),
    mutates,
    undoable
)]
pub struct SetPlayhead {
    /// Where it is, in samples.
    pub position: Number,
}

impl SetPlayhead {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let position = samples(self.position.0, "where the playhead is")?;
        if position < 0 {
            return Err(EngineError::Failed {
                id: "sound.playhead.set".to_string(),
                message: "a playhead cannot be before the beginning".to_string(),
            });
        }
        let existing =
            session.document().walk().find(|node| node.kind == "sound.playhead").map(|node| node.id);
        let id = match existing {
            Some(id) => {
                if let Some(node) = session.document_mut()?.node_mut(id) {
                    node.data = serde_json::json!({ "position": position });
                }
                id
            }
            None => session.document_mut()?.add_node_with(
                "playhead".to_string(),
                "sound.playhead",
                None,
                serde_json::json!({ "position": position }),
            ),
        };
        Ok(Outcome::changed(format!("the playhead is at {position} samples"))
            .with_data(serde_json::json!({ "id": id, "position": position })))
    }
}

/// **Mute** a clip, or unmute it.
///
/// A muted clip is not played and is still there: the tree's renderer
/// skips it and the tree's editor keeps it, so that a clip that is
/// quiet is a clip that may be loud again.
#[derive(Command)]
#[command(
    id = "sound.clip.mute",
    area = "sound",
    layer = "sound",
    icon = "sound",
    menu("Track", "Mute"),
    mutates,
    undoable
)]
pub struct MuteClip {
    /// The clip to mute.
    pub clip: NodeRef,
    /// Whether it is muted. Without it, the clip is muted.
    pub muted: Option<Bool>,
}

impl MuteClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.clip.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("there is no clip `{id}`"),
            });
        };
        if node.kind != "sound.clip" {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("the node `{id}` is a `{}` and not a clip", node.kind),
            });
        }
        let mut payload = ClipPayload::from_json(&node.data).map_err(|error| EngineError::Failed {
            id: "sound.clip.mute".to_string(),
            message: error.to_string(),
        })?;
        let muted = self.muted.map(|muted| muted.0).unwrap_or(true);
        payload.muted = muted;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "sound.clip.mute".to_string(),
                message: format!("there is no clip `{id}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(if muted {
            format!("the clip `{id}` is muted")
        } else {
            format!("the clip `{id}` is not muted")
        })
        .with_data(serde_json::json!({ "id": id, "muted": muted })))
    }
}

/// Render the document into a **sound file**.
///
/// The renderer is the first thing in this tree that makes a sound: it
/// reads the sources the clips play, mixes them and writes the file.
/// It is **offline** — it writes a file and it stops — because the kit
/// has no audio device and no audio thread (§ the kit's own notes), and
/// a file is a thing a program can make without a device.
///
/// What it does:
/// - the clips are the **tracks**: each clip plays a source from an
///   offset, for a length, at a gain;
/// - the sources are read from the **files** the document names, and
///   the samples are the ones the WAV reader gives back (§133);
/// - the tracks are mixed (§134) and the mix is written as a WAV.
#[derive(Command)]
#[command(id = "sound.render", area = "sound", layer = "sound", icon = "play", menu("Render"))]
pub struct RenderSound {
    /// Where to write the sound.
    pub path: Text,
}

impl RenderSound {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        let mix = mix_of(session.document(), "sound.render", None)?;
        let channels = mix.channels;
        let bytes = craftkit_wav::write(channels, mix.rate, &mix.master);
        write_file(&path, &bytes)?;
        Ok(Outcome::new(format!("rendered {} frames to `{path}`", mix.frames)).with_data(serde_json::json!({
            "path": path,
            "frames": mix.frames,
            "rate": mix.rate,
            "channels": channels,
            "clips": mix.tracks.len(),
        })))
    }
}

/// The **mix** of a document: the sound of the whole thing, and the
/// sound of each of its tracks.
///
/// The mixer is worked out **once** and the file is written from it,
/// so that the render and the stems are the same sound: a stem is
/// the mix with one track in it, and not a second way of mixing.
struct Mix {
    /// The whole mix: every track and every bus, after the master
    /// faders and the metronome.
    master: Vec<f32>,
    /// The tracks that are heard, with their signal.
    tracks: Vec<(u64, Vec<f32>)>,
    /// The signal of one track **before** its trim and its inserts,
    /// when a caller asked for it.
    ///
    /// This is the sound of the track's own clips and nothing else: the
    /// track's trim and the track's inserts are not in it, because they
    /// belong to the track and the track is still there.
    raw: Option<(u64, Vec<f32>)>,
    /// The signal of one track **before** its fader, when a caller
    /// asked for it.
    ///
    /// The sound of the track's clips through its **trim** and its
    /// **inserts**, and not through its **mute** and not through its
    /// **fader**. The tree's commit renders a track before its fader,
    /// so that the new track's fader is the old track's fader and the
    /// level is not counted twice.
    pre_fader: Option<(u64, Vec<f32>)>,
    /// How many channels the mix has.
    channels: u16,
    /// The rate the mix was made at.
    rate: u32,
    /// How many frames the mix is.
    frames: usize,
}

/// The **mix** of a document.
///
/// The `id` is the command that asked for it, so that a refusal says
/// which command refused and not which one it was borrowed from.
///
/// The `raw_track` is the track whose **raw** signal is kept: the sum
/// of its clips, before its trim and before its inserts.
fn mix_of(document: &Document, id: &str, capture: Option<u64>) -> Result<Mix, EngineError> {
    let settings = session_of(document);
    let rate = settings.sample_rate;
    // The clips: each one plays a source from an offset, for a
    // length, at a gain.
    let mut clips: Vec<(u64, u64, ClipPayload)> = Vec::new();
    for clip in document.walk().filter(|node| node.kind == "sound.clip") {
        // A clip that cannot be read is **refused**, and not left
        // out: a sound that is missing a clip without saying so is
        // a sound that is wrong in a way nobody can hear.
        let payload = ClipPayload::from_json(&clip.data).map_err(|error| EngineError::Failed {
            id: id.to_string(),
            message: format!("the clip `{}` cannot be read: {error}", clip.id),
        })?;
        let Some(source_id) = payload.source else { continue };
        // A **muted** clip is not played and is still there.
        if payload.muted {
            continue;
        }
        // The **take**: a clip that is not in the take that is
        // played is not heard, and the tree's tracks have takes for
        // exactly that reason.
        if let Some(track) = timeline::track_of(document, clip) {
            let active = active_playlist(document, track);
            let take = timeline::parent_of(document, clip.id).map(|playlist| playlist.id);
            if let (Some(active), Some(take)) = (active, take)
                && active != take
            {
                continue;
            }
        }
        clips.push((source_id, clip.id, payload));
    }
    // The sources are read once each, and the clips that play one
    // share it.
    let mut sources: Vec<(u64, craftkit_wav::Sound)> = Vec::new();
    let mut channels = 0_u16;
    for (source_id, _, _) in &clips {
        if sources.iter().any(|(id, _)| id == source_id) {
            continue;
        }
        let Some(node) = document.node(*source_id) else { continue };
        let Ok(source) = SourcePayload::from_json(&node.data) else { continue };
        let sound = read_sound("sound.render", &source.path)?;
        // The mix is as **wide** as the widest source: a mono source in
        // a stereo session is heard in both channels, the way the tree's
        // renderer hears it.
        channels = channels.max(sound.channels);
        sources.push((*source_id, sound));
    }
    // Each clip's samples are **prepared**: a source at another rate
    // is resampled, a **stretched** clip is played at another
    // speed, and the track's **automation** is applied to them,
    // sample by sample.
    let mut prepared: Vec<Vec<f32>> = Vec::with_capacity(clips.len());
    for (source_id, clip_id, payload) in &clips {
        let Some((_, sound)) = sources.iter().find(|(id, _)| id == source_id) else {
            prepared.push(Vec::new());
            continue;
        };
        let (from, to) = the_span(sound, payload, rate);
        let slice = &sound.samples[from..to];
        let samples = if sound.rate == rate {
            slice.to_vec()
        } else {
            // A source that cannot be played at the session's rate
            // is **refused**, and not truncated: a sound that is
            // missing its tail is a sound that is wrong in a way
            // nobody can hear.
            craftkit_audio::resample(
                slice,
                sound.channels as usize,
                sound.rate,
                rate,
                &craftkit_audio::Limits::default(),
            )
            .map_err(|error| EngineError::Failed {
                id: id.to_string(),
                message: format!("the source cannot be played at {rate} Hz: {error}"),
            })?
        };
        // Whether the clip's track is **elastic**: the track's
        // algorithm says so, and the algorithms are open strings.
        let elastic = document
            .node(*clip_id)
            .and_then(|clip| timeline::track_of(document, clip))
            .map(is_elastic)
            .unwrap_or(false);
        // The **stretch**: the source is read more slowly, so the
        // sound is longer and the pitch is lower. The clip's own
        // length does not change: the stretch says how the source
        // is played.
        //
        // An **elastic** track is the exception: its clips are
        // stretched and the pitch stays, because that is what an
        // elastic track is for. The track says so by its
        // **algorithm** — the algorithms are open strings, and
        // every one of them but `varispeed` says so.
        let samples = if (payload.stretch - 1.0).abs() > f64::EPSILON {
            if elastic {
                craftkit_audio::time_stretch(&samples, sound.channels as usize, payload.stretch, rate)
            } else {
                craftkit_audio::varispeed(&samples, sound.channels as usize, payload.stretch)
            }
        } else {
            samples
        };
        // The **pitch**: the clip's own note, moved. The shift
        // keeps the length — it is the other half of a stretch,
        // and the two are the same sound read two ways.
        let samples = if payload.pitch_semitones.abs() > f64::EPSILON {
            craftkit_audio::pitch_shift(&samples, sound.channels as usize, payload.pitch_semitones, rate)
        } else {
            samples
        };
        // The **processes**: what is done to the sound on the way
        // out. They are done in the order they were added, and
        // each of them is the tree's own.
        let samples =
            apply_processes(&payload.processes, &samples, sound.channels as usize).map_err(|error| {
                EngineError::Failed {
                    id: id.to_string(),
                    message: format!("the clip `{}` cannot be played: {error}", clip_id),
                }
            })?;
        // The **fades**: the head of the clip fades in and the tail
        // fades out, and a fade is a gain and not a thing that
        // belongs to a channel.
        let samples = craftkit_audio::faded(
            &samples,
            sound.channels as usize,
            payload.fade_in.fade(),
            payload.fade_out.fade(),
        );
        // The **track's** automation is **not** applied here: the
        // fader comes after the track's inserts, the way the
        // tree's mixer runs it, and a send may be taken before the
        // fader. The lanes under the track move the track's sound
        // and not one clip's.
        //
        // The **clip's own** automation: the lanes under the clip,
        // and their times are the clip's own — a clip carries a
        // curve of its own, and the family's does.
        let mut own =
            document.node(*clip_id).map(|clip| automate::automation_of(document, clip)).unwrap_or_default();
        // A **bypassed** gain is not in the path: the clip is as
        // loud as it is, and the curve that automates its level is
        // not in the path either. The curve is **kept**, and it
        // comes back when the bypass is taken off.
        if payload.gain_bypassed {
            own.volume = None;
        }
        let mut samples = automate::apply(&own, &samples, sound.channels as usize, 0);
        // The clip's **gain**: the level of the clip itself. The
        // tree's mixer multiplies it into the clip before the
        // track's inserts, and so does this one.
        let clip_gain =
            if payload.gain_bypassed { 1.0 } else { craftkit_audio::db_to_linear(payload.gain_db) as f32 };
        if clip_gain != 1.0 {
            samples.iter_mut().for_each(|sample| *sample *= clip_gain);
        }
        // The clip's **own effects**: the tree's clip effects are an
        // equaliser and a compressor on one clip and not on the
        // track, and they run over the clip's own sound before it
        // is mixed into the track.
        if let Some(clip) = document.node(*clip_id) {
            for insert in inserts_of(document, clip) {
                if insert.bypass {
                    continue;
                }
                let mut plugin = craftkit_plugins::make(
                    &insert.plugin,
                    &craftkit_plugins::Params::from_map(insert.params),
                )
                .map_err(|error| EngineError::Failed { id: id.to_string(), message: error.to_string() })?;
                plugin.process(&mut samples, sound.channels as usize, rate);
            }
        }
        prepared.push(samples);
    }
    // How long the sound is: the end of the longest clip.
    let mut frames = 0_i64;
    for (index, (_, _, payload)) in clips.iter().enumerate() {
        let end = payload.start.max(0) + prepared[index].len() as i64 / channels.max(1) as i64;
        if end > frames {
            frames = end;
        }
    }
    let frames = frames.max(0) as usize;
    // The **tracks**: each track's signal is the sum of its clips,
    // its **inserts** are run over that sum in the order of the
    // tree, and its **fader** comes after them — the fader is the
    // track's automation, and the tree's mixer runs the fader
    // last.
    //
    // The signal before the fader is kept for the tracks whose
    // sends ask for it: a **pre-fader** send is a copy of the
    // signal before the fader, and the copy is the whole point of
    // it.
    let mut pre_fader: Vec<(u64, Vec<f32>)> = Vec::new();
    let mut raw: Option<(u64, Vec<f32>)> = None;
    let mut before_fader: Option<(u64, Vec<f32>)> = None;
    let mut tracks: Vec<(u64, Vec<f32>, automate::Automation)> = Vec::new();
    // The **routed tracks**: the tracks that feed a bus, and the
    // signal they feed it with.
    let mut routed: Vec<(u64, Vec<f32>)> = Vec::new();
    // The **readers**: the tracks that read a bus. A reader is
    // finished after the buses are, because its sound is the bus's
    // sound.
    let readers = mixer::readers_of(document);
    for track in document.walk().filter(|node| {
        node.kind == "sound.track"
            && !is_master(node)
            && !is_inactive(node)
            && !mixer::reads_bus(document, node.id)
    }) {
        // The signal is **interleaved**: it holds a frame of every
        // channel, and it is as long as the mix in frames.
        let mut signal = vec![0.0_f32; frames * channels.max(1) as usize];
        for (index, (source_id, clip_id, payload)) in clips.iter().enumerate() {
            let Some(clip) = document.node(*clip_id) else { continue };
            if timeline::track_of(document, clip).map(|owner| owner.id) != Some(track.id) {
                continue;
            }
            // The clip's own gain is already in its samples: the
            // tree's mixer multiplies it into the clip.
            // The clip's place is a place in **frames**, and the
            // signal is interleaved: the clip's samples go at that
            // many frames, and not at that many samples.
            let at = payload.start.max(0) as usize * channels.max(1) as usize;
            // A source that is **narrower** than the mix is heard in
            // every channel of it: the tree's renderer takes the
            // source's channel `c % n` for the mix's channel `c`, and
            // a mono source is heard in both channels of a stereo mix.
            let source_channels = sources
                .iter()
                .find(|(id, _)| *id == *source_id)
                .map(|(_, sound)| sound.channels.max(1) as usize)
                .unwrap_or(channels.max(1) as usize);
            add_channels(&mut signal, &prepared[index], at, 1.0, source_channels, channels.max(1) as usize);
        }
        // The **raw** signal of the track: its clips and nothing else.
        // The trim and the inserts belong to the track, and a caller
        // that wants the sound of the clips alone asks for this.
        if capture == Some(track.id) {
            raw = Some((track.id, signal.clone()));
        }
        // The **trim**: the level of the track's input, and its
        // phase. The tree's mixer applies the trim to the strip
        // before the inserts, so an insert sees the trimmed sound.
        let trim = TrackPayload::from_json(&track.data)
            .map(|payload| {
                let gain = craftkit_audio::db_to_linear(payload.trim_db) as f32;
                let phase = if payload.phase_invert { -1.0 } else { 1.0 };
                gain * phase
            })
            .unwrap_or(1.0);
        if trim != 1.0 {
            signal.iter_mut().for_each(|sample| *sample *= trim);
        }
        for insert in inserts_of(document, track) {
            if insert.bypass {
                continue;
            }
            let mut plugin =
                craftkit_plugins::make(&insert.plugin, &craftkit_plugins::Params::from_map(insert.params))
                    .map_err(|error| EngineError::Failed {
                        id: id.to_string(),
                        message: error.to_string(),
                    })?;
            plugin.process(&mut signal, channels as usize, rate);
        }
        // The **pre-fader** signal of the track that was asked for:
        // the sound of its clips through its trim and its inserts, and
        // not through its mute and not through its fader. The tree's
        // commit renders the track before its fader, and this is that
        // sound.
        if capture == Some(track.id) {
            before_fader = Some((track.id, signal.clone()));
        }
        // The **mute** gates the audio and the track's pre-fader
        // tap: a muted track is not heard anywhere.
        let automation = automate::automation_of(document, track);
        let gated = automate::apply_mute(&automation, &signal, channels as usize, 0);
        // The **fader**: the track's volume, over the whole signal
        // and not over one clip.
        let faded = automate::apply_fader(&automation, &gated, channels as usize, 0);
        if sends_of(document).iter().any(|send| send.track == track.id && send.pre_fader) {
            pre_fader.push((track.id, gated));
        }
        // The **route**: a track that feeds a bus is heard on that
        // bus and not in the main mix. The tree's strip goes to
        // its output and to nowhere else.
        match mixer::route_of(document, track.id) {
            // The **balance** is applied when the track is routed:
            // a track that feeds a bus is balanced before it
            // reaches the bus, and it is not balanced again.
            Some(bus) => {
                let panned = automate::apply_pan(&automation, &faded, channels.max(1) as usize, 0);
                routed.push((bus, panned));
            }
            None => tracks.push((track.id, faded, automation)),
        }
    }
    // The **solo**: a soloed track silences the others, and their
    // sends carry the silence, because a send is a copy of the
    // track's signal.
    if tracks.iter().any(|(id, _, _)| track_is_soloed(document, *id)) {
        for (id, signal, _) in &mut tracks {
            // A **solo-safe** track is not silenced by the solo: it is
            // a track that stays when somebody listens to one thing
            // on purpose.
            if !track_is_soloed(document, *id) && !track_is_solo_safe(document, *id) {
                signal.iter_mut().for_each(|sample| *sample = 0.0);
            }
        }
        // A soloed-out track's **pre-fader** copy is silent too: the
        // tree's mixer skips the send of a track that is muted, and
        // the copy is not heard.
        for (id, signal) in &mut pre_fader {
            if !track_is_soloed(document, *id) && !track_is_solo_safe(document, *id) {
                signal.iter_mut().for_each(|sample| *sample = 0.0);
            }
        }
    }
    // The **buses**: each bus is fed by the sends that name it, and
    // a send carries a copy of its track's signal at its own level.
    let mut buses: Vec<(u64, Vec<f32>)> = Vec::new();
    for bus in document.walk().filter(|node| node.kind == "sound.bus") {
        let mut signal = vec![0.0_f32; frames * channels.max(1) as usize];
        for send in sends_of(document) {
            if send.bus != bus.id {
                continue;
            }
            // A **pre-fader** send takes the signal from before the
            // fader, and a post-fader send takes it from after.
            let source: Option<&[f32]> = if send.pre_fader {
                pre_fader.iter().find(|(id, _)| *id == send.track).map(|(_, signal)| signal.as_slice())
            } else {
                tracks.iter().find(|(id, _, _)| *id == send.track).map(|(_, signal, _)| signal.as_slice())
            };
            let Some(track_signal) = source else {
                continue;
            };
            if send.mute {
                // A muted send is a copy of **nothing**: the track
                // is still heard on its own and the bus hears
                // nothing.
                continue;
            }
            let level = craftkit_audio::db_to_linear(send.level_db) as f32;
            let panned = panned(track_signal, send.pan, channels as usize);
            add_into(&mut signal, &panned, 0, level);
        }
        buses.push((bus.id, signal));
    }
    // The **routed tracks**: a track that feeds a bus is heard on
    // that bus.
    for (bus, signal) in &routed {
        if let Some((_, bus_signal)) = buses.iter_mut().find(|(id, _)| id == bus) {
            add_into(bus_signal, signal, 0, 1.0);
        }
    }
    // The **mix order**: the buses and the readers, in the order
    // they are mixed. A bus is mixed before the readers that read
    // it, and a reader is mixed before the bus it feeds — so a
    // reader that feeds a bus is heard there and not by itself.
    let stages = mixer::stage_order(document)
        .map_err(|message| EngineError::Failed { id: id.to_string(), message })?;
    for stage in stages {
        match stage {
            mixer::Stage::Bus(id) => {
                // A bus that feeds another bus is heard there and
                // not by itself.
                let Some(destination) = mixer::route_of(document, id) else { continue };
                let Some(index) = buses.iter().position(|(bus, _)| *bus == id) else { continue };
                let signal = buses[index].1.clone();
                if let Some((_, target)) = buses.iter_mut().find(|(bus, _)| *bus == destination) {
                    add_into(target, &signal, 0, 1.0);
                }
            }
            mixer::Stage::Reader(reader, bus) => {
                // A track that reads a bus takes the bus's sound,
                // with its own inserts and its own fader. A bus
                // that is read is not heard by itself: the tree's
                // bus is read by an aux and is silent on its own.
                let Some((_, bus_signal)) = buses.iter().find(|(id, _)| *id == bus) else { continue };
                let Some(track) = document.node(reader) else { continue };
                // An **inactive** track is not heard, and a reader is
                // a track: its voice is off too.
                if is_inactive(track) {
                    continue;
                }
                let mut signal = bus_signal.clone();
                for insert in inserts_of(document, track) {
                    if insert.bypass {
                        continue;
                    }
                    let mut plugin = craftkit_plugins::make(
                        &insert.plugin,
                        &craftkit_plugins::Params::from_map(insert.params),
                    )
                    .map_err(|error| EngineError::Failed {
                        id: id.to_string(),
                        message: error.to_string(),
                    })?;
                    plugin.process(&mut signal, channels.max(1) as usize, rate);
                }
                let automation = automate::automation_of(document, track);
                let gated = automate::apply_mute(&automation, &signal, channels.max(1) as usize, 0);
                let faded = automate::apply_fader(&automation, &gated, channels.max(1) as usize, 0);
                // A reader that **feeds a bus** is heard there and
                // not in the main mix.
                match mixer::route_of(document, reader) {
                    Some(destination) => {
                        if let Some((_, target)) = buses.iter_mut().find(|(bus, _)| *bus == destination) {
                            add_into(target, &faded, 0, 1.0);
                        }
                    }
                    None => tracks.push((reader, faded, automation)),
                }
            }
        }
    }
    // The **master** is the file: every track and every bus.
    let mut master = vec![0.0_f32; frames * channels.max(1) as usize];
    for (_, signal, automation) in &tracks {
        // The **balance**: the track's own pan is the track's, and
        // it is applied when the track is routed. A copy that a
        // send takes is balanced by the **send's** own pan.
        let own = automate::apply_pan(automation, signal, channels.max(1) as usize, 0);
        add_into(&mut master, &own, 0, 1.0);
    }
    for (bus, signal) in &buses {
        // A bus that is **read** is heard through the reader, and a
        // bus that **feeds another bus** is heard there: neither is
        // heard by itself.
        if readers.iter().any(|(_, read)| read == bus) {
            continue;
        }
        if mixer::route_of(document, *bus).is_some() {
            continue;
        }
        add_into(&mut master, signal, 0, 1.0);
    }
    // The **master faders**: a track of the kind `master` holds a
    // fader for the whole mix and not for a part of it. The tree's
    // mixer runs the master faders over the main mix, in the order
    // of the tree, and the metronome is summed after them.
    for master_track in
        document.walk().filter(|node| node.kind == "sound.track" && is_master(node) && !is_inactive(node))
    {
        for insert in inserts_of(document, master_track) {
            if insert.bypass {
                continue;
            }
            let mut plugin =
                craftkit_plugins::make(&insert.plugin, &craftkit_plugins::Params::from_map(insert.params))
                    .map_err(|error| EngineError::Failed {
                        id: id.to_string(),
                        message: error.to_string(),
                    })?;
            plugin.process(&mut master, channels.max(1) as usize, rate);
        }
        // The **fader** of the master: its own lanes, over the whole
        // mix.
        let automation = automate::automation_of(document, master_track);
        master = automate::apply_mute(&automation, &master, channels.max(1) as usize, 0);
        master = automate::apply_fader(&automation, &master, channels.max(1) as usize, 0);
    }
    let channels = channels.max(1);
    // The **metronome**: the click is mixed in after the master, so
    // it reaches the output without passing through a track, a send
    // or an insert. The beats come from the tempo map, and the
    // first beat of a bar is the accent.
    if settings.metronome {
        click_into(&mut master, channels as usize, rate, &tempo_map(document), &settings);
    }
    // The **limit**: the mix may not be longer than the audio library
    // allows, and the limit is in frames.
    let limits = craftkit_audio::Limits::default();
    if frames as i64 > limits.max_frames {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the mix is {frames} frames long and the limit is {}", limits.max_frames),
        });
    }
    Ok(Mix {
        master,
        tracks: tracks.into_iter().map(|(id, signal, _)| (id, signal)).collect(),
        raw,
        pre_fader: before_fader,
        channels,
        rate,
        frames,
    })
}

/// Render the **stems**: one file for each track.
///
/// A stem is the mix with **one track in it**: the tree's bounce solos
/// the track and renders the whole mix, so a stem carries the master
/// faders and the metronome with it, and a stem of a track that feeds
/// a bus carries the bus.
///
/// The files are written into a **directory**, and the directory is
/// made if it is not there. Each file is named after its track, and a
/// name that is taken gets a number after it — two tracks with the
/// same name are two files and not one file written twice.
///
/// A **muted** track is not bounced: the tree does not bounce it, and
/// a stem of silence is a file nobody asked for. The mute is a **lane**
/// here and not a field of the track, so a muted track is one whose
/// mute lane says so at the start.
#[derive(Command)]
#[command(id = "sound.render.stems", area = "sound", layer = "sound", icon = "play", menu("Render"))]
pub struct RenderStems {
    /// The directory to write the stems into.
    pub directory: Text,
}

impl RenderStems {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let directory = self.directory.0.clone();
        if directory.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "sound.render.stems".to_string(),
                message: "a directory is a path".to_string(),
            });
        }
        std::fs::create_dir_all(&directory).map_err(|error| EngineError::Failed {
            id: "sound.render.stems".to_string(),
            message: format!("the directory `{directory}` cannot be made: {error}"),
        })?;
        let document = session.document().clone();
        // The **tracks** that are bounced: a track that is not a track
        // is not bounced, a **master** is not bounced (it holds the
        // fader of the mix and not a sound of its own), and a muted
        // track is not bounced.
        let mut files = Vec::new();
        for track in document
            .walk()
            .filter(|node| node.kind == "sound.track" && !is_master(node) && !is_inactive(node))
        {
            // The **mute** is a field of the track and a lane over it:
            // a muted track is one whose own mute is on, or one whose
            // mute lane says so.
            let automation = automate::automation_of(&document, track);
            let muted = automation.muted
                || automation
                    .mute
                    .as_ref()
                    .map(|points| automate::value_at(points, 0) >= 0.5)
                    .unwrap_or(false);
            if muted {
                continue;
            }
            // The **stem** is the mix with this track in it and the
            // others out: the track is soloed and the mix is worked
            // out again, so a stem is the same sound as the mix and
            // not a second way of mixing.
            let mut soloed = document.clone();
            let tracks: Vec<u64> =
                soloed.walk().filter(|node| node.kind == "sound.track").map(|node| node.id).collect();
            for other in tracks {
                let Some(node) = soloed.node_mut(other) else { continue };
                let Ok(mut payload) = TrackPayload::from_json(&node.data) else { continue };
                payload.solo = other == track.id;
                node.data = payload.to_json();
            }
            let mix = mix_of(&soloed, "sound.render.stems", None)?;
            let bytes = craftkit_wav::write(mix.channels, mix.rate, &mix.master);
            let path = a_stem_path(&directory, &track.name);
            write_file(&path, &bytes)?;
            files.push(path);
        }
        Ok(Outcome::new(format!("bounced {} stems into `{directory}`", files.len()))
            .with_data(serde_json::json!({ "directory": directory, "files": files })))
    }
}

/// **Consolidate** a track: the clips of a track become one clip of a
/// file that holds their sound.
///
/// The tree's consolidate renders the sound of a track's clips over a
/// range, writes it to a file, and puts one clip of that file where the
/// clips were. The sound is the same and the editing is not: the
/// clips are gone and there is one clip with a file behind it.
///
/// The sound that is rendered is the sound of the track's **clips**:
/// the track's trim and the track's inserts are not in it, because
/// they belong to the track and the track is still there. A clip that
/// crosses the edge of the range keeps the part outside it.
///
/// The **MIDI** clips of the track are left alone: they hold no sound
/// to render, and a MIDI clip is not a file.
#[derive(Command)]
#[command(
    id = "sound.track.consolidate",
    area = "sound",
    layer = "sound",
    icon = "play",
    menu("Track", "Consolidate"),
    mutates,
    undoable
)]
pub struct ConsolidateTrack {
    /// The track to consolidate.
    pub track: NodeRef,
    /// Where the range starts, in samples.
    ///
    /// Without it, the range starts at the beginning of the session.
    pub start: Option<Number>,
    /// Where the range ends, in samples.
    ///
    /// Without it, the range ends at the end of the track's clips.
    pub end: Option<Number>,
    /// The directory the file is written into.
    pub directory: Text,
}

impl ConsolidateTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.consolidate".to_string();
        let track = self.track.0;
        let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no node `{track}`"),
        })?;
        if node.kind != "sound.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        if is_master(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the master holds the fader of the mix and not a sound of its own".to_string(),
            });
        }
        let directory = self.directory.0.clone();
        if directory.trim().is_empty() {
            return Err(EngineError::Failed { id, message: "a directory is a path".to_string() });
        }
        std::fs::create_dir_all(&directory).map_err(|error| EngineError::Failed {
            id: id.clone(),
            message: format!("the directory `{directory}` cannot be made: {error}"),
        })?;
        // The **range**: the track's own clips say where it ends, and
        // a range with no length is refused.
        let document = session.document().clone();
        let mut clips: Vec<(u64, ClipPayload)> = Vec::new();
        let mut last = 0_i64;
        for clip in document.walk().filter(|node| node.kind == "sound.clip") {
            if timeline::track_of(&document, clip).map(|owner| owner.id) != Some(track) {
                continue;
            }
            let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
            last = last.max(payload.start.max(0) + payload.length.max(0));
            clips.push((clip.id, payload));
        }
        let start = match &self.start {
            Some(start) => samples(start.0, "where the range starts")?,
            None => 0,
        };
        let end = match &self.end {
            Some(end) => samples(end.0, "where the range ends")?,
            None => last,
        };
        if end <= start {
            return Err(EngineError::Failed {
                id,
                message: format!("the range ends at {end} and starts at {start}"),
            });
        }
        // The **sound** of the track's clips: the raw signal, before
        // the track's trim and before the track's inserts.
        let mix = mix_of(&document, &id, Some(track))?;
        let channels = mix.channels.max(1) as usize;
        let from = start.max(0) as usize * channels;
        let to = end.max(0) as usize * channels;
        let Some((_, signal)) = mix.raw else {
            return Err(EngineError::Failed {
                id,
                message: format!("the track `{track}` has no sound to consolidate"),
            });
        };
        if signal.len() < to {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "the sound of the track is {} frames long and the range asks for {end}",
                    signal.len() / channels
                ),
            });
        }
        let slice = &signal[from..to];
        // The **file**: the sound of the range, written beside the
        // session.
        let name = format!("{}_consolidated", node.name);
        let path = a_stem_path(&directory, &name);
        let bytes = craftkit_wav::write(mix.channels, mix.rate, slice);
        write_file(&path, &bytes)?;
        // The **source** of the file, and the **asset** that carries it.
        let frames = (end - start).max(0);
        let payload = SourcePayload {
            path: path.clone(),
            channels: mix.channels as u32,
            frames,
            sample_rate: mix.rate,
            peaks: Vec::new(),
        };
        let source =
            session.document_mut()?.add_node_with(path.clone(), "sound.source", None, payload.to_json());
        session.document_mut()?.add_asset(craftkit_sdk::Asset::new(path.clone(), "audio", None));
        // The **clips** of the range are gone: the ones inside it are
        // thrown away, and the ones that cross its edge keep the part
        // outside it.
        let mut removed = 0_usize;
        let mut kept = 0_usize;
        for (clip_id, payload) in clips {
            let clip_start = payload.start.max(0);
            let clip_end = clip_start + payload.length.max(0);
            if clip_end <= start || clip_start >= end {
                continue;
            }
            if clip_start >= start && clip_end <= end {
                session.document_mut()?.remove_node(clip_id);
                removed += 1;
                continue;
            }
            // The part **outside** the range is kept.
            let mut payload = payload;
            if clip_start < start {
                payload.length = (start - clip_start).max(0);
            } else {
                let cut = end - clip_start;
                payload.offset += cut;
                payload.start = end;
                payload.length = (clip_end - end).max(0);
            }
            if let Some(node) = session.document_mut()?.node_mut(clip_id) {
                node.data = payload.to_json();
            }
            kept += 1;
        }
        // The **clip** of the new file, where the old ones were.
        let playlist = playlist_of(session, track, None)?;
        let mut clip = ClipPayload::new();
        clip.start = start;
        clip.length = frames;
        clip.source = Some(source);
        let clip_id = session.document_mut()?.add_node_with_under(
            Some(playlist),
            name.clone(),
            "sound.clip",
            None,
            clip.to_json(),
        );
        Ok(Outcome::changed(format!("consolidated the track `{track}` into `{path}`")).with_data(
            serde_json::json!({
                "track": track,
                "clip": clip_id,
                "source": source,
                "path": path,
                "start": start,
                "end": end,
                "removed": removed,
                "kept": kept,
            }),
        ))
    }
}

/// Put a node **after** another one, at the top level.
///
/// The document's own order is the order the mixer works in, and a
/// thing that stands in another's place — a **committed** track, a
/// **copy** of a track — stands there in the order as well.
pub(crate) fn place_after(session: &mut Session, node: u64, after: u64) {
    let Ok(document) = session.document_mut() else { return };
    let Some(index) = document.nodes.iter().position(|entry| entry.id == node) else { return };
    let Some(mut place) = document.nodes.iter().position(|entry| entry.id == after) else { return };
    let entry = document.nodes.remove(index);
    // The node that was after the one we moved is one place earlier
    // now.
    if place > index {
        place -= 1;
    }
    document.nodes.insert(place + 1, entry);
}

/// The **path** of a stem: the track's name in the directory, and a
/// name that is taken gets a number after it.
///
/// The name is made safe for a file system: the letters and the
/// numbers and a few marks are kept and the rest becomes an
/// underscore, and a name that is empty is `untitled`.
fn a_stem_path(directory: &str, name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '-' | '_' | ' ' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim().trim_start_matches('.').to_string();
    let safe = if safe.is_empty() { "untitled".to_string() } else { safe.chars().take(100).collect() };
    let first = format!("{directory}/{safe}.wav");
    if !std::path::Path::new(&first).exists() {
        return first;
    }
    let mut number = 2_u32;
    loop {
        let path = format!("{directory}/{safe}-{number}.wav");
        if !std::path::Path::new(&path).exists() {
            return path;
        }
        number = number.saturating_add(1);
        if number > 10_000 {
            return format!("{directory}/{safe}.wav");
        }
    }
}

/// Whether a track is **solo safe**: a track a solo does not silence.
fn track_is_solo_safe(document: &Document, track: u64) -> bool {
    let Some(node) = document.node(track) else { return false };
    TrackPayload::from_json(&node.data).map(|payload| payload.solo_safe).unwrap_or(false)
}

/// Whether a track is **soloed**.
fn track_is_soloed(document: &Document, track: u64) -> bool {
    let Some(node) = document.node(track) else { return false };
    TrackPayload::from_json(&node.data).map(|payload| payload.solo).unwrap_or(false)
}

/// The **mix** of a sound document, as a **lane job**.
///
/// This is the real-time work of a sound application: the tracks are
/// prepared on the **control thread** — the sources are read, the
/// clips are prepared and the tracks' signals are built — and the lane
/// does the rest: the block's samples are the sum of the tracks, and
/// nothing is allocated while it happens.
///
/// The tree has **no device**, so nothing plays the job: a caller runs
/// the blocks itself, and the rule that the work does not allocate is
/// **measured** on it rather than asserted in a comment.
pub struct MixJob {
    /// The tracks, prepared: their signals and where they start.
    tracks: Vec<craftkit_audio::Track>,
    /// The block's samples.
    block: Vec<f32>,
    /// The frames the job has mixed.
    played: i64,
}

impl MixJob {
    /// A job that mixes these tracks.
    pub fn new(tracks: Vec<craftkit_audio::Track>) -> MixJob {
        MixJob { tracks, block: Vec::new(), played: 0 }
    }

    /// The block's samples, after the lane has run it.
    pub fn samples(&self) -> &[f32] {
        &self.block
    }

    /// How many frames the job has mixed.
    pub fn played(&self) -> i64 {
        self.played
    }
}

impl craftkit_sdk::LaneJob for MixJob {
    /// Prepare: the **control thread**. The block's buffer is made
    /// here, because the lane may not allocate.
    fn prepare(&mut self, block: usize) -> Result<(), String> {
        self.block = vec![0.0_f32; block];
        Ok(())
    }

    /// Run: the **lane**. The tracks are summed into the block, and
    /// nothing is allocated.
    fn run(&mut self, frames: usize) -> craftkit_sdk::Block {
        let frames = frames.min(self.block.len());
        for sample in self.block.iter_mut().take(frames) {
            *sample = 0.0;
        }
        let block_start = self.played.max(0) as usize;
        let block_end = block_start + frames;
        for track in &self.tracks {
            let start = track.start.max(0) as usize;
            let end = start + track.samples.len();
            // The part of the track that is in this block.
            let from = start.max(block_start);
            let to = end.min(block_end);
            if from >= to {
                continue;
            }
            let gain = track.gain as f32;
            for (index, sample) in track.samples[from - start..to - start].iter().enumerate() {
                if let Some(slot) = self.block.get_mut(from - block_start + index) {
                    *slot += *sample * gain;
                }
            }
        }
        self.played += frames as i64;
        craftkit_sdk::Block::Played
    }
}

/// An **insert**: the plugin a track runs over its signal.
struct Insert {
    /// The plugin's name.
    plugin: String,
    /// Its parameters, by name.
    params: HashMap<String, f64>,
    /// Whether it is bypassed.
    bypass: bool,
}

/// The inserts of a track, in the order of the tree.
///
/// The order of the tree is the order of the drawing (§ the order), and
/// it is the order of the **processing** too: the first insert is the
/// first program the signal meets.
fn inserts_of(document: &Document, track: &Node) -> Vec<Insert> {
    let mut inserts = Vec::new();
    for child in &track.children {
        let Some(node) = document.node(child.id) else { continue };
        if node.kind != "sound.insert" {
            continue;
        }
        let Ok(payload) = InsertPayload::from_json(&node.data) else { continue };
        let mut params: HashMap<String, f64> = HashMap::new();
        if let Some(values) = payload.params.as_object() {
            for (name, value) in values {
                if let Some(number) = value.as_f64() {
                    params.insert(name.clone(), number);
                }
            }
        }
        inserts.push(Insert { plugin: payload.plugin, params, bypass: payload.bypass });
    }
    inserts
}

/// A **send**: the level and the pan with which a track feeds a bus.
struct Send {
    /// The track that feeds.
    track: u64,
    /// The bus that is fed.
    bus: u64,
    /// Its level, in decibels.
    level_db: f64,
    /// Its pan, from minus one to one.
    pan: f64,
    /// Whether it is silent.
    mute: bool,
    /// Whether it is taken before the track's fader.
    pre_fader: bool,
}

/// The sends of the document: the tracks that feed the buses.
fn sends_of(document: &Document) -> Vec<Send> {
    let mut sends = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.send") {
        let Ok(payload) = SendPayload::from_json(&node.data) else { continue };
        let Some(track) = timeline::parent_of(document, node.id) else { continue };
        let Some(relation) = document.relations_from(node.id, Some("send")).into_iter().next() else {
            continue;
        };
        sends.push(Send {
            track: track.id,
            bus: relation.to,
            level_db: payload.level_db,
            pan: payload.pan,
            mute: payload.mute,
            pre_fader: payload.pre_fader,
        });
    }
    sends
}

/// Add a signal into another, at a place, at a gain.
/// The samples of a clip, **added into** a signal that may be wider
/// than they are.
///
/// A clip's samples are as wide as the source it plays, and the signal
/// is as wide as the mix: a clip of a mono source in a stereo mix is
/// heard in both channels, and the tree's renderer does the same — it
/// takes the source's channel `c % n` for the mix's channel `c`.
fn add_channels(
    target: &mut [f32],
    source: &[f32],
    at: usize,
    gain: f32,
    source_channels: usize,
    mix_channels: usize,
) {
    let source_channels = source_channels.max(1);
    let mix_channels = mix_channels.max(1);
    let frames = source.len() / source_channels;
    for frame in 0..frames {
        for channel in 0..mix_channels {
            let target_at = at + frame * mix_channels + channel;
            if target_at >= target.len() {
                break;
            }
            let source_at = frame * source_channels + (channel % source_channels);
            if let Some(value) = source.get(source_at) {
                target[target_at] += *value * gain;
            }
        }
    }
}

fn add_into(target: &mut [f32], source: &[f32], at: usize, gain: f32) {
    for (index, sample) in source.iter().enumerate() {
        let position = at + index;
        if position >= target.len() {
            break;
        }
        target[position] += sample * gain;
    }
}

/// The **processes** on a clip's sound, done in order.
///
/// Each process is the tree's own: the sound is reversed, upside down,
/// brought to a level, or has its direct current taken away. A process
/// the battery does not know is an **error** and not a sound that is
/// quietly left alone — a document that says the sound was changed and
/// was not is a document that lies.
fn apply_processes(
    processes: &[ProcessPayload],
    samples: &[f32],
    channels: usize,
) -> Result<Vec<f32>, String> {
    let mut out = samples.to_vec();
    for process in processes {
        out = match process.process.as_str() {
            "reverse" => craftkit_audio::reverse(&out, channels),
            "invert" => craftkit_audio::invert(&out),
            "dc_offset" => craftkit_audio::remove_dc(&out, channels),
            "normalize" => {
                let target =
                    process.params.get("target_db").and_then(|target| target.as_f64()).unwrap_or(0.0);
                craftkit_audio::normalize(&out, target)
            }
            other => return Err(format!("not supported yet: the process `{other}`")),
        };
    }
    Ok(out)
}

/// The signal of a track, **panned** between its channels.
///
/// The pan is a balance: minus one is all to the left and one is all
/// to the right, and the middle is the middle. A **mono** signal is
/// not balanced and is left alone.
fn panned(signal: &[f32], pan: f64, channels: usize) -> Vec<f32> {
    if channels < 2 || pan == 0.0 {
        return signal.to_vec();
    }
    let (left, right) = automate::balance(pan);
    let mut out = signal.to_vec();
    for index in 0..signal.len() / channels {
        for channel in 0..channels {
            let at = index * channels + channel;
            out[at] *= if channel % 2 == 0 { left } else { right };
        }
    }
    out
}

/// The span of a source's samples a clip plays: from its offset, for
/// the part of the source its length covers.
///
/// The clip speaks in the **session's samples** and the source speaks in
/// **its own**, and the span is where the two meet: the offset is
/// turned into the source's frames, and the length is turned into the
/// frames the source is read in — which is **less** than the length
/// when the clip is stretched, because a stretched clip reads its
/// source more slowly.
///
/// The span is in **frames**, and the samples of the source are
/// interleaved, so the frames are multiplied by the number of channels.
fn the_span(sound: &craftkit_wav::Sound, payload: &ClipPayload, rate: u32) -> (usize, usize) {
    let width = sound.channels as usize;
    // How many frames of the source one sample of the session is.
    let ratio = sound.rate as f64 / rate.max(1) as f64;
    let from_frame = (payload.offset.max(0) as f64 * ratio).round() as usize;
    let span_frames = ((payload.length.max(0) as f64 / payload.stretch) * ratio).round() as usize;
    let from = (from_frame * width).min(sound.samples.len());
    let to = ((from_frame + span_frames) * width).min(sound.samples.len()).max(from);
    (from, to)
}

/// How many **segments** a drawn fade has.
///
/// A fade is a curve, and a curve drawn with two points is a line. The
/// number is small because a notch in a bar is small, and because the
/// notch is a hint and not the sound.
const FADE_SEGMENTS: usize = 8;

/// How many **numbers** a sound's peaks have.
///
/// The peaks are the shape of a sound in a small table, and the table
/// has to be small enough to live in a payload: a thousand numbers is
/// a shape that a reader can see and a document can carry. The
/// buckets and the channels share it — a sound with two channels has
/// half the buckets of a sound with one, so that the table is the same
/// size either way.
const PEAK_NUMBERS: usize = 1024;

/// The **peaks** of a sound: the extent of the samples in each bucket.
///
/// A sound is a great many numbers and a drawing needs few. The peaks
/// are the least and the most of each bucket of samples, and they are
/// what a clip's bar is drawn from.
fn peaks_of(sound: &craftkit_wav::Sound) -> Vec<f32> {
    let channels = sound.channels.max(1) as usize;
    let frames = sound.frames as usize;
    // The buckets and the channels **share** the table: a table of a
    // thousand numbers is a thousand numbers however many channels
    // there are.
    let buckets = (PEAK_NUMBERS / (channels * 2)).max(1);
    let per_bucket = (frames / buckets).max(1);
    let mut out = Vec::with_capacity(buckets * channels * 2);
    for bucket in 0..buckets {
        let from = bucket * per_bucket;
        let to = (from + per_bucket).min(frames);
        for channel in 0..channels {
            if from >= frames {
                // A sound shorter than the table: the rest of the table
                // is silence, and silence is a shape too.
                out.push(0.0);
                out.push(0.0);
                continue;
            }
            let mut least = f32::MAX;
            let mut most = f32::MIN;
            for frame in from..to {
                let Some(sample) = sound.samples.get(frame * channels + channel) else { continue };
                if *sample < least {
                    least = *sample;
                }
                if *sample > most {
                    most = *sample;
                }
            }
            out.push(least);
            out.push(most);
        }
    }
    out
}

/// The **peaks** of a source, as the payload holds them.
///
/// The peaks are a flat table — a least and a most for each bucket —
/// and the source says how many frames of the sound the table covers.
struct Peaks {
    /// The table: for each bucket, the least and the most of each
    /// channel.
    table: Vec<f32>,
    /// How many channels the sound has.
    channels: usize,
    /// How many frames of the sound the table covers.
    frames: u64,
}

/// The peaks of a source, if it has been measured.
fn peaks_of_source(document: &Document, source: u64) -> Option<Peaks> {
    let node = document.node(source)?;
    let payload = SourcePayload::from_json(&node.data).ok()?;
    let channels = payload.channels.max(1) as usize;
    if payload.peaks.is_empty() {
        return None;
    }
    Some(Peaks { table: payload.peaks, channels, frames: payload.frames.max(0) as u64 })
}

/// The **waveform** of a clip: the shape of the sound it plays.
///
/// The clip plays a **part** of a source, from its offset, for as long
/// as its length says — and a stretched clip reads its source more
/// slowly, so it plays less of it. The bar is the clip, and the shape
/// drawn on the bar is the part of the source the clip plays.
fn waveform_paths<F: Fn(usize, usize) -> f32>(
    rect: &Rect,
    peaks: &Peaks,
    from_frame: usize,
    span_frames: usize,
    gain: F,
) -> Vec<Path> {
    let channels = peaks.channels.max(1);
    let count = peaks.table.len() / (channels * 2);
    if count == 0 || peaks.frames == 0 {
        return Vec::new();
    }
    let from_bucket = from_frame.saturating_mul(count) / peaks.frames as usize;
    let to_bucket = ((from_frame + span_frames).saturating_mul(count) / peaks.frames as usize)
        .min(count)
        .max(from_bucket + 1);
    if from_bucket >= count {
        return Vec::new();
    }
    let buckets = to_bucket - from_bucket;
    let width = rect.w;
    let mut paths: Vec<Path> = Vec::with_capacity(channels);
    for channel in 0..channels {
        // The **band** of the channel: the bar is as tall as the
        // channels it holds, and each channel has its own.
        let band = Rect::new(
            rect.x,
            rect.y + rect.h * channel as f64 / channels as f64,
            rect.w,
            rect.h / channels as f64,
        );
        // The **upper** edge of the shape, left to right, and then the
        // lower edge, right to left: the shape is the sound between
        // them.
        let mut upper: Vec<Point> = Vec::with_capacity(buckets);
        let mut lower: Vec<Point> = Vec::with_capacity(buckets);
        for index in 0..buckets {
            let bucket = from_bucket + index;
            let at = (bucket * channels + channel) * 2;
            let least = peaks.table.get(at).copied().unwrap_or(0.0);
            let most = peaks.table.get(at + 1).copied().unwrap_or(least);
            let gain = gain(index, buckets);
            let x = rect.x + (index as f64 + 0.5) * width / buckets as f64;
            upper.push(Point::new(x, level_y(&band, most * gain)));
            lower.push(Point::new(x, level_y(&band, least * gain)));
        }
        if upper.is_empty() {
            continue;
        }
        let mut segments: Vec<Segment> = Vec::with_capacity(upper.len() * 2);
        for point in upper.iter().skip(1) {
            segments.push(Segment::Line { to: *point });
        }
        for point in lower.iter().rev().skip(1) {
            segments.push(Segment::Line { to: *point });
        }
        let Some(start) = upper.first() else { continue };
        paths.push(Path::single(SubPath { start: *start, segments, closed: true }));
    }
    paths
}
/// The **waveform** of a clip: the shape of the sound it plays.
///
/// The clip plays a part of a source, and the part is in the source's
/// own frames: the offset is turned into the source's frames, and the
/// length is turned into the frames the source is read in — which is
/// less than the length when the clip is stretched, because a stretched
/// clip reads its source more slowly.
fn waveform_of(document: &Document, clip: &Node, payload: &ClipPayload, rect: &Rect) -> Vec<Path> {
    let Some(source) = payload.source else { return Vec::new() };
    let Some(peaks) = peaks_of_source(document, source) else { return Vec::new() };
    let Some(source_node) = document.node(source) else { return Vec::new() };
    let Ok(source_payload) = SourcePayload::from_json(&source_node.data) else { return Vec::new() };
    let session_rate = session_of(document).sample_rate.max(1) as f64;
    let ratio = source_payload.sample_rate.max(1) as f64 / session_rate;
    let from_frame = (payload.offset.max(0) as f64 * ratio).round() as usize;
    let span_frames =
        ((payload.length.max(0) as f64 / payload.stretch.max(f64::EPSILON)) * ratio).round() as usize;
    // The **gain** of the sound at each place: the clip's own gain and
    // the track's automation. A clip that runs across a breakpoint is
    // loud at one end and quiet at the other, and the shape on the bar
    // says so — the shape is the sound as it is heard and not the
    // sound as it is on the disk.
    let length = payload.length.max(0);
    waveform_paths(rect, &peaks, from_frame, span_frames, |index, count| {
        let at = payload.start + (length * index as i64) / count.max(1) as i64;
        gain_at(document, clip, payload, at) as f32
    })
}

/// The **gain** of a clip's sound at that time.
///
/// The clip's own gain and the track's **automation**: the volume lane
/// is in decibels and the mute lane is one or less, and a muted track
/// is silent however loud its sound is.
fn gain_at(document: &Document, clip: &Node, payload: &ClipPayload, at: i64) -> f64 {
    // A **bypassed** gain is not in the path: the clip is as loud as it
    // is, and the gain that is kept comes back when the bypass is off.
    let mut decibels = if payload.gain_bypassed { 0.0 } else { payload.gain_db };
    if let Some(track) = timeline::track_of(document, clip) {
        let automation = automate::automation_of(document, track);
        if let Some(points) = &automation.volume {
            decibels += automate::value_at(points, at);
        }
        if let Some(points) = &automation.mute
            && automate::value_at(points, at) >= 1.0
        {
            return 0.0;
        }
    }
    craftkit_audio::db_to_linear(decibels)
}

/// Where a **level** of the sound stands in a bar.
///
/// The sound runs from minus one to one, and the bar is the height it
/// has: the top of the bar is the loudest sound and the bottom is the
/// quietest.
fn level_y(rect: &Rect, level: f32) -> f64 {
    let level = level.clamp(-1.0, 1.0) as f64;
    rect.y + (1.0 - level) * rect.h / 2.0
}

/// The **notch** of a fade at the head of a bar.
///
/// The notch is the part of the clip that is quiet: it is the region
/// under the fade's curve, and the curve is the fade's own shape.
fn fade_in_path(rect: &Rect, width: f64, fade: craftkit_audio::Fade) -> Option<Path> {
    let mut segments = Vec::with_capacity(FADE_SEGMENTS);
    for step in 1..=FADE_SEGMENTS {
        let t = step as f64 / FADE_SEGMENTS as f64;
        let x = rect.x + t * width;
        let y = rect.y + rect.h - fade.gain(t as f32) as f64 * rect.h;
        segments.push(Segment::Line { to: Point::new(x, y) });
    }
    Some(Path::single(SubPath { start: Point::new(rect.x, rect.y + rect.h), segments, closed: true }))
}

/// The **notch** of a fade at the tail of a bar.
///
/// The notch is the part of the clip that is quiet, and at the tail the
/// quiet part is **above** the curve: the curve falls from the full
/// height to nothing, and what is left of the bar above it is the
/// silence.
fn fade_out_path(rect: &Rect, width: f64, fade: craftkit_audio::Fade) -> Option<Path> {
    let start = rect.x + rect.w - width;
    let mut segments = Vec::with_capacity(FADE_SEGMENTS);
    for step in 1..=FADE_SEGMENTS {
        let t = step as f64 / FADE_SEGMENTS as f64;
        let x = start + t * width;
        let y = rect.y + rect.h - fade.gain((1.0 - t) as f32) as f64 * rect.h;
        segments.push(Segment::Line { to: Point::new(x, y) });
    }
    Some(Path::single(SubPath { start: Point::new(start, rect.y), segments, closed: true }))
}

/// The **metronome**: a click at every beat.
///
/// The beats come from the tempo map, and the meter says how long a
/// beat is: a click at every beat, and a different click on the first
/// beat of a bar, because that is the beat a musician waits for.
///
/// The click is mixed into the master **after** everything else, so it
/// does not pass through a track, a send or an insert: it is a voice
/// of its own.
fn click_into(
    master: &mut [f32],
    channels: usize,
    rate: u32,
    map: &craftkit_sound_time::TempoMap,
    settings: &SoundSession,
) {
    let level = craftkit_audio::db_to_linear(settings.click_volume_db) as f32;
    if level <= 0.0 {
        return;
    }
    let frames = master.len() / channels.max(1);
    // The walk is over the beats, and it stops when the beats are past
    // the end of the sound. The counter is the bound: a document with
    // a tempo of a million beats a minute would otherwise walk for a
    // very long time.
    let mut tick = 0_i64;
    let mut walked = 0_u32;
    while walked < 1_000_000 {
        let at = map.ticks_to_samples(tick, rate);
        if at < 0 || at as usize >= frames {
            break;
        }
        let accent = map.bar_beat_at_tick(tick).beat == 1;
        let note = if accent { settings.click_accent_note } else { settings.click_note };
        craftkit_audio::blip(master, channels, at as usize, craftkit_audio::midi_to_hz(note), level, rate);
        let (_, denominator) = map.meter_at_tick(tick);
        tick += craftkit_sound_time::ticks_per_beat(denominator).max(1);
        walked += 1;
    }
}

/// Read a sound file, within limits.
///
/// The file came from outside — it is named by a document that came
/// from outside — so it is read **within limits** and a file that is
/// too big is refused before a byte of it is read.
fn read_sound(id: &str, path: &str) -> Result<craftkit_wav::Sound, EngineError> {
    let limits = craftkit_wav::Limits::default();
    let metadata = std::fs::metadata(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` cannot be read: {error}"),
    })?;
    if metadata.len() as usize > limits.max_bytes {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the source `{path}` is larger than the limits allow"),
        });
    }
    let bytes = std::fs::read(path).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` cannot be read: {error}"),
    })?;
    craftkit_wav::read(&bytes, &limits).map_err(|error| EngineError::Failed {
        id: id.to_string(),
        message: format!("the source `{path}` is not a sound the kit reads: {error}"),
    })
}

/// Write the sound to a file.
fn write_file(path: &str, bytes: &[u8]) -> Result<(), EngineError> {
    std::fs::write(path, bytes).map_err(|error| EngineError::Failed {
        id: "sound.render".to_string(),
        message: format!("the sound cannot be written to `{path}`: {error}"),
    })
}

/// Set the session: the sample rate and the timecode.
///
/// These are the settings that belong to the session and not to any one
/// node, and they live in the document's own payload.
#[derive(Command)]
#[command(
    id = "sound.session.set",
    area = "sound",
    layer = "sound",
    icon = "music",
    menu("Options"),
    mutates,
    undoable
)]
pub struct SetSession {
    /// The sample rate, in samples a second.
    pub sample_rate: Option<Number>,
    /// Where the document starts on a timecode, in samples.
    pub timecode_start: Option<Number>,
    /// Whether the **metronome** is heard.
    pub metronome: Option<Bool>,
    /// How loud the click is, in decibels.
    pub click_volume_db: Option<Number>,
    /// The click's pitch, as a note number.
    pub click_note: Option<Number>,
    /// The **accent's** pitch: the click on the first beat of a bar.
    pub click_accent_note: Option<Number>,
}

impl SetSession {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let mut settings = session_of(session.document());
        if let Some(rate) = &self.sample_rate {
            if !rate.0.is_finite() || rate.0 < 1.0 || rate.0 > 768_000.0 {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: format!("{} is not a sample rate", rate.0),
                });
            }
            settings.sample_rate = rate.0 as u32;
        }
        if let Some(start) = &self.timecode_start {
            settings.timecode_start = samples(start.0, "where the timecode starts")?;
        }
        if let Some(metronome) = &self.metronome {
            settings.metronome = metronome.0;
        }
        if let Some(volume) = &self.click_volume_db {
            if !volume.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a level is a number of decibels".to_string(),
                });
            }
            settings.click_volume_db = volume.0;
        }
        if let Some(note) = &self.click_note {
            if !note.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a pitch is a note number".to_string(),
                });
            }
            settings.click_note = note.0;
        }
        if let Some(note) = &self.click_accent_note {
            if !note.0.is_finite() {
                return Err(EngineError::Failed {
                    id: "sound.session.set".to_string(),
                    message: "a pitch is a note number".to_string(),
                });
            }
            settings.click_accent_note = note.0;
        }
        set_session(session.document_mut()?, &settings);
        Ok(Outcome::changed(format!("the session runs at {} samples a second", settings.sample_rate))
            .with_data(serde_json::json!({
                "sample_rate": settings.sample_rate,
                "timecode_start": settings.timecode_start,
                "metronome": settings.metronome,
                "click_volume_db": settings.click_volume_db,
                "click_note": settings.click_note,
                "click_accent_note": settings.click_accent_note,
            })))
    }
}

// ------------------------------------------------------------------ the tool, the panel --

/// The clip tool.
///
/// A tool is a gesture that runs a command. It is not a command: it has no
/// parameters of its own, it has an icon, and it says which command it
/// runs.
#[derive(Tool)]
#[tool(id = "tool.sound.clip", label = "Clip", layer = "sound", icon = "plus", command = "sound.clip.add")]
pub struct ClipTool;

/// The panel that shows the sound commands.
#[derive(Panel)]
#[panel(
    id = "panel.sound",
    label = "Sound",
    layer = "sound",
    commands(
        "sound.track.add",
        "sound.clip.add",
        "sound.tempo.set",
        "sound.meter.set",
        "sound.marker.add",
        "sound.ruler.add",
        "sound.insert.set",
        "sound.playhead.set",
        "sound.source.add",
        "sound.render",
        "sound.session.set",
        "sound.clip.sync.set",
        "sound.clip.sync.remove",
        "sound.clip.sync.read",
        "sound.clip.lock.edit",
        "sound.clip.lock.time"
    )
)]
pub struct SoundPanel;

// ------------------------------------------------------------------- the evaluator --

/// The evaluator that says what a sound document means.
///
/// A kind may speak only about its own nodes; this speaks about the whole
/// document: the **tempo map**, which is derived from the tempo and meter
/// events rather than stored, and every **clip**, in samples and in bars
/// and beats.
#[derive(Evaluator)]
#[evaluator(id = "sound.timeline", label = "Timeline", layer = "sound")]
pub struct TimelineEvaluator;

impl TimelineEvaluator {
    /// The tempo map, derived from the document's events.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        let settings = session_of(document);
        let map = tempo_map(document);

        let mut tracks = Vec::new();
        for track in document.walk().filter(|node| node.kind == "sound.track") {
            let mut playlists = Vec::new();
            for playlist in &track.children {
                if playlist.kind != "sound.playlist" {
                    continue;
                }
                let mut clips = Vec::new();
                for clip in &playlist.children {
                    if clip.kind != "sound.clip" {
                        continue;
                    }
                    let Ok(payload) = ClipPayload::from_json(&clip.data) else { continue };
                    let position = map.samples_to_ticks(payload.start, settings.sample_rate).max(0);
                    let bar_beat = map.bar_beat_at_tick(position);
                    clips.push(serde_json::json!({
                        "id": clip.id,
                        "name": clip.name,
                        "start": payload.start,
                        "length": payload.length,
                        "end": payload.start.saturating_add(payload.length),
                        "source": payload.source,
                        "position": format!("{}.{}.{}", bar_beat.bar, bar_beat.beat, bar_beat.tick),
                        "sync": payload.sync,
                    }));
                }
                playlists.push(serde_json::json!({
                    "id": playlist.id,
                    "name": playlist.name,
                    "clips": clips,
                }));
            }
            tracks.push(serde_json::json!({
                "id": track.id,
                "name": track.name,
                "playlists": playlists,
            }));
        }

        plan.items.push(PlanItem {
            node: 0,
            kind: "sound.timeline".to_string(),
            value: serde_json::json!({
                "sample_rate": settings.sample_rate,
                "timecode_start": settings.timecode_start,
                "tempos": map
                    .tempos()
                    .iter()
                    .map(|tempo| serde_json::json!({ "tick": tempo.tick, "bpm": tempo.bpm }))
                    .collect::<Vec<_>>(),
                "meters": map
                    .meters()
                    .iter()
                    .map(|meter| serde_json::json!({
                        "tick": meter.tick,
                        "numerator": meter.numerator,
                        "denominator": meter.denominator,
                    }))
                    .collect::<Vec<_>>(),
                "tracks": tracks,
            }),
        });
        plan
    }
}

/// The evaluator that works out the routing.
///
/// The buses, in the order they are fed in. It is **derived state**: the
/// document holds the edges and this works out what they mean.
#[derive(Evaluator)]
#[evaluator(id = "sound.routing", label = "Routing", layer = "sound")]
pub struct RoutingEvaluator;

impl RoutingEvaluator {
    /// The routing, in order.
    fn evaluate(_document: &Document) -> Plan {
        let mut plan = Plan::new();
        plan.items.push(PlanItem {
            node: 0,
            kind: "sound.routing".to_string(),
            value: serde_json::json!({ "buses": mixer::routing(_document) }),
        });
        plan
    }
}

// ---------------------------------------------------------------------- the helpers --

/// The tempo map, read from the document's events.
///
/// The map is **derived**: the document holds the events, and the map is
/// what they mean. That is the evaluator's job and not the document's.
pub(crate) fn tempo_map(document: &Document) -> TempoMap {
    let mut map = TempoMap::new();
    for node in document.walk() {
        if node.kind == "sound.tempo"
            && let Ok(payload) = TempoPayload::from_json(&node.data)
        {
            let _ = map.set_tempo(payload.tick, payload.bpm);
        }
        if node.kind == "sound.meter"
            && let Ok(payload) = MeterPayload::from_json(&node.data)
        {
            let _ = map.set_meter(payload.tick, payload.numerator, payload.denominator);
        }
    }
    map
}

/// The event node at a tick, if the document has one.
fn find_event(session: &Session, kind: &str, tick: i64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| {
            if node.kind != kind {
                return false;
            }
            let at = match kind {
                "sound.tempo" => TempoPayload::from_json(&node.data).map(|payload| payload.tick),
                "sound.meter" => MeterPayload::from_json(&node.data).map(|payload| payload.tick),
                "sound.key" => crate::keys::KeyPayload::from_json(&node.data).map(|payload| payload.tick),
                _ => Ok(-1),
            };
            matches!(at, Ok(at) if at == tick)
        })
        .map(|node| node.id)
}

/// Put a **track** into the document, with its take under it.
///
/// A track is a node and its take is a node under it: the track is
/// the row of the timeline and the take is where the clips go. The
/// track made here is an **instrument** track, because a track of
/// notes is a track that is played.
pub(crate) fn add_track(session: &mut Session, name: &str) -> Result<u64, EngineError> {
    let payload = TrackPayload {
        track_kind: "instrument".to_string(),
        channels: 2,
        solo: false,
        trim_db: 0.0,
        fader_db: 0.0,
        pan: 0.0,
        mute: false,
        solo_safe: false,
        phase_invert: false,
        active_playlist: None,
        inactive: false,
        elastic: None,
    };
    let document = session.document_mut()?;
    let track = document.add_node_with(name.to_string(), "sound.track", None, payload.to_json());
    document.add_node_with_under(Some(track), "A".to_string(), "sound.playlist", None, Json::Null);
    Ok(track)
}

/// The playlist a clip goes in.
///
/// The playlist is named, or it is the first one the track has.
fn playlist_of(session: &Session, track: u64, name: Option<String>) -> Result<u64, EngineError> {
    let node = session.document().node(track).ok_or_else(|| EngineError::Failed {
        id: "sound.clip.add".to_string(),
        message: format!("there is no node {track}"),
    })?;
    if node.kind != "sound.track" {
        return Err(EngineError::Failed {
            id: "sound.clip.add".to_string(),
            message: format!("node {track} is a `{}` and not a track", node.kind),
        });
    }
    let playlists: Vec<&Node> =
        node.children.iter().filter(|child| child.kind == "sound.playlist").map(|child| &**child).collect();
    match name {
        Some(name) => {
            playlists.iter().find(|playlist| playlist.name == name).map(|playlist| playlist.id).ok_or_else(
                || EngineError::Failed {
                    id: "sound.clip.add".to_string(),
                    message: format!("the track has no playlist called `{name}`"),
                },
            )
        }
        None => playlists.first().map(|playlist| playlist.id).ok_or_else(|| EngineError::Failed {
            id: "sound.clip.add".to_string(),
            message: format!("the track {track} has no playlist"),
        }),
    }
}

/// The part of a node that is selected, if a part is selected.
///
/// The selection holds whole nodes and **parts** of nodes, and a part is
/// a range in the node's own units. A sound document's units are
/// samples, so a part of a clip is a range of samples.
pub(crate) fn selection_span(session: &Session) -> Option<(u64, (f64, f64))> {
    session.selection().iter().find_map(|item| {
        let from = item.from?;
        let to = item.to?;
        Some((item.node, (from, to)))
    })
}

/// A number of samples, from a number a caller sent.
///
/// A number from outside is hostile: it may not be a number at all, and it
/// may be a number so large that it is not a position.
fn samples(value: f64, what: &str) -> Result<i64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: format!("{what} is not a number"),
        });
    }
    let rounded = value.round();
    if rounded.abs() > MAX_POSITION as f64 {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: format!("{what} is past the end of time"),
        });
    }
    Ok(rounded as i64)
}

/// A stretch, from a number a caller sent.
///
/// The stretch is a **length ratio**, and it is bounded: it is greater
/// than zero, because a clip that plays nothing is not a clip, and it
/// is no more than [`MAX_STRETCH`], because a clip stretched a
/// thousandfold would hold a thousandth of a sample for a thousand
/// times as long.
pub(crate) fn stretch_of(value: f64, id: &str) -> Result<f64, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed { id: id.to_string(), message: "a stretch is a number".to_string() });
    }
    if value <= 0.0 || value > MAX_STRETCH {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "a stretch is greater than zero and no more than {MAX_STRETCH}, and this one is {value}"
            ),
        });
    }
    Ok(value)
}

/// A stretch, from a number a caller sent, for a track that is
/// **elastic**.
///
/// A pitch-preserving stretch is a harder thing to do than a
/// varispeed, and it is bounded the way the tree's is: the sound may
/// be stretched from a tenth to ten times, and not more. A stretch
/// outside that is **refused** — and not quietly pulled to the edge,
/// because a caller that asked for a hundredfold stretch would be
/// given a tenthfold one and would not know.
fn elastic_stretch_of(value: f64, id: &str, algorithm: &str) -> Result<f64, EngineError> {
    let stretch = stretch_of(value, id)?;
    if !(MIN_ELASTIC_STRETCH..=MAX_ELASTIC_STRETCH).contains(&stretch) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "a stretch on an `{algorithm}` track is from {MIN_ELASTIC_STRETCH} to {MAX_ELASTIC_STRETCH}, \
                 and this one is {stretch}"
            ),
        });
    }
    Ok(stretch)
}

/// A number of ticks, from a number a caller sent.
fn ticks(value: f64) -> Result<i64, EngineError> {
    let ticks = samples(value, "where the event happens")?;
    if ticks < 0 {
        return Err(EngineError::Failed {
            id: "sound".to_string(),
            message: "an event does not happen before the beginning".to_string(),
        });
    }
    Ok(ticks)
}

// The plugin's anchor.
//
// One name that keeps every item above in the program, on every target —
// including `wasm32-unknown-unknown`, whose linker drops a crate that
// nothing names. See the SDK's `anchor` module.
craftkit_sdk::plugin_anchor!(
    SoundLayer,
    TrackKind,
    PlaylistKind,
    ClipKind,
    TempoKind,
    MeterKind,
    KeyKind,
    MarkerKind,
    SourceKind,
    AddTrack,
    SoloTrack,
    AddClip,
    SetClipStretch,
    SetClipFade,
    SetTempo,
    SetMeter,
    SetKey,
    ReadKeys,
    ImportMidiTracks,
    ConsolidateTrack,
    CommitTrack,
    CopyRange,
    DuplicateTrack,
    MoveInsert,
    CutRange,
    PasteRange,
    ReadClipboard,
    RemoveClip,
    RemoveInsert,
    RemoveKey,
    RemoveMeter,
    RemoveTempo,
    RenderClip,
    RemoveMarker,
    RemoveTrack,
    RenameClip,
    RenameTrack,
    SetSend,
    SetTrackInactive,
    RepeatRange,
    SplitMono,
    AddMarker,
    AddSource,
    MeasurePeaks,
    SetSession,
    ClipTool,
    SoundPanel,
    TimelineEvaluator,
    RoutingEvaluator,
    BusKind,
    InsertKind,
    SendKind,
    LaneKind,
    AddBus,
    AddRoute,
    AddInsert,
    AddSend,
    AddLane,
    SetBeat,
    RemoveBeat,
    ReadBeat,
    EditLock,
    TimeLock,
    SetRating,
    GroupKind,
    Group,
    Ungroup,
    ReadGroup,
    SetGrid,
    ReadGrid,
    Quantize,
    SetGain,
    BypassGain,
    Conform,
    SetTrackGroup,
    SetGroupFlags,
    UngroupTracks
);

#[cfg(test)]
mod tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// The plugin's own kinds are in the registry.
    #[test]
    fn the_kinds_are_there() {
        let ecosystem = ecosystem();
        assert!(ecosystem.kinds.get("sound.track").is_some());
        assert!(ecosystem.kinds.get("sound.clip").is_some());
        assert!(ecosystem.commands.get("sound.track.add").is_some());
        assert!(ecosystem.evaluators.get("sound.timeline").is_some());
    }

    /// A track comes with a playlist.
    #[test]
    fn a_track_comes_with_a_playlist() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        assert_eq!(track.kind, "sound.track");
        assert_eq!(track.rect, None, "a track is at no place: it is at a time");
        assert_eq!(track.children.len(), 1, "the track has a playlist");
        assert_eq!(track.children[0].kind, "sound.playlist");
    }

    /// A clip is a **bar** on the track's row: the timeline is the sound
    /// document's own world.
    ///
    /// The place is **derived** — from the clip's span and from the row
    /// of the track it is under — and it is not in the document, because
    /// a rectangle in the document would go stale the moment a track was
    /// added above it.
    #[test]
    fn a_clip_is_a_bar_on_the_rows() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": track.id, "start": 48000, "length": 96000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let rect = ClipKind::place(clip, session.document()).expect("the clip has a place");
        assert_eq!(rect.x, 1.0, "the clip starts one second in");
        assert_eq!(rect.w, 2.0, "and it is two seconds long");
        assert_eq!(rect.y, 0.0, "on the first row");
        assert_eq!(rect.h, timeline::ROW_HEIGHT, "and the row is one unit tall");
        assert_eq!(clip.rect, None, "and the document holds no rectangle for it");
    }

    /// The **second** track is on the second row, and the clip on it
    /// moves with it.
    #[test]
    fn a_second_track_is_on_the_second_row() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the first track");
        session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the second track");
        let bass = session.document().node_by_name("bass").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": bass.id, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let rect = ClipKind::place(clip, session.document()).expect("the clip has a place");
        assert_eq!(rect.y, timeline::ROW_HEIGHT, "the clip is on the second row");
    }

    /// A clip is **drawn**, and a clip is **picked**.
    #[test]
    fn a_clip_is_drawn_and_picked() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there");
        session
            .execute("sound.clip.add", &json!({ "track": track.id, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there");
        let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
        ClipKind::paint(clip, session.document(), &mut canvas);
        assert_eq!(canvas.drawn.len(), 1, "the clip is one bar");
        assert!(canvas.drawn[0].starts_with("rect("), "and it is a rectangle: {}", canvas.drawn[0]);
        assert!(
            ClipKind::hit(clip, session.document(), Point::new(0.5, 0.5)),
            "a point inside the bar is in the clip"
        );
        assert!(
            !ClipKind::hit(clip, session.document(), Point::new(5.0, 0.5)),
            "and a point outside it is not"
        );
    }

    /// A tempo event is a **mark in the ruler**, and the ruler is above
    /// the tracks.
    ///
    /// The event's time is in **ticks** and the world is in **seconds**,
    /// so the tick is worked out against the tempo map: a tick is a
    /// musical position, and it is not a length until the tempo says how
    /// fast the music is.
    #[test]
    fn a_tempo_event_is_a_mark_in_the_ruler() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "tick": 960, "bpm": 120.0 })).expect("the tempo is set");
        let tempo =
            session.document().walk().find(|node| node.kind == "sound.tempo").expect("the tempo is there");
        let rect = TempoKind::place(tempo, session.document()).expect("the tempo has a place");
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "the ruler is above the tracks");
        // At a hundred and twenty beats a minute, a beat is half a
        // second, and tick 960 is one beat.
        assert!((rect.x - 0.5).abs() < 0.001, "and the beat is half a second in: {}", rect.x);
    }

    /// The ruler carries the **bar numbers**.
    ///
    /// The bars come from the tempo map: at a hundred and twenty beats
    /// a minute a bar of four beats is two seconds long.
    #[test]
    fn the_ruler_carries_the_bar_numbers() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "tick": 0, "bpm": 120.0 })).expect("the tempo is set");
        session
            .execute("sound.meter.set", &json!({ "tick": 0, "numerator": 4, "denominator": 4 }))
            .expect("the meter is set");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480000 }))
            .expect("the clip is added");
        session.execute("sound.ruler.add", &json!({ "name": "ruler" })).expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let marks = timeline::scale_marks(session.document(), ruler);
        assert!(!marks.is_empty(), "the ruler has no numbers");
        assert_eq!(marks[0].label, "1", "the first bar is bar one");
        assert!((marks[0].x - 0.0).abs() < 0.001, "and it is at the beginning");
        let second = marks.iter().find(|mark| mark.label == "2").expect("the second bar is numbered");
        assert!((second.x - 2.0).abs() < 0.001, "and a bar is two seconds long: {}", second.x);
    }

    /// The ruler can count by **seconds** instead.
    #[test]
    fn the_ruler_can_count_by_seconds() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480000 }))
            .expect("the clip is added");
        session
            .execute("sound.ruler.add", &json!({ "name": "ruler", "scale": "seconds" }))
            .expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let marks = timeline::scale_marks(session.document(), ruler);
        assert_eq!(marks[0].label, "1", "the first second is second one");
        assert!((marks[0].x - 1.0).abs() < 0.001, "and it is one second in");
    }

    /// The ruler is the **row above the tracks**.
    #[test]
    fn the_ruler_is_the_row_above_the_tracks() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.ruler.add", &json!({ "name": "ruler" })).expect("the ruler is added");
        let ruler =
            session.document().walk().find(|node| node.kind == "sound.ruler").expect("the ruler is there");
        let rect = RulerKind::place(ruler, session.document()).expect("the ruler has a place");
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "the ruler is above the tracks");
        assert_eq!(rect.h, timeline::RULER_HEIGHT, "and it is a row");
    }

    /// A scale that is not a scale is **refused**.
    #[test]
    fn a_scale_that_is_not_a_scale_is_refused() {
        let mut session = session();
        let error = session
            .execute("sound.ruler.add", &json!({ "name": "ruler", "scale": "eighths" }))
            .expect_err("that is not a scale");
        assert!(error.to_string().contains("bars"), "the refusal says what is wrong: {error}");
    }

    /// The document is **rendered**: the clips are played and the sound
    /// is written to a file.
    ///
    /// The renderer is offline — it makes a file and it stops — and the
    /// file it makes is a sound the kit can read back.
    #[test]
    fn the_document_is_rendered_into_a_file() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-render-source.wav");
        let output_path = directory.join("craftkit-v2-render-out.wav");
        // A source: four samples of half.
        let bytes = craftkit_wav::write(1, 48_000, &[0.5, 0.5, 0.5, 0.5]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 4, "the clip is four samples long");
        assert!((sound.samples[0] - 0.5).abs() < 0.001, "the sample is the source's: {}", sound.samples[0]);
    }

    /// A source at **another rate** is resampled: the sound is the same
    /// sound, at the rate the session plays at.
    ///
    /// The clip's length is in the **session's** samples, so a clip of
    /// four is four samples long whatever the source's rate is — and
    /// it is four samples long in the picture too, because the
    /// timeline draws the clip's length in the session's samples. The
    /// source is read at the rate it was written at and played at the
    /// rate the session plays at, and the clip takes the part of it
    /// that fits.
    #[test]
    fn a_source_at_another_rate_is_resampled() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-render-rate.wav");
        let output_path = directory.join("craftkit-v2-render-rate-out.wav");
        // A source of eight thousand samples a second, four frames.
        let bytes = craftkit_wav::write(1, 8_000, &[0.5, 0.5, 0.5, 0.5]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 16_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.rate, 16_000, "the sound is at the session's rate");
        assert_eq!(sound.frames, 4, "and it is as long as the clip says: {}", sound.frames);
        assert!((sound.samples[0] - 0.5).abs() < 0.001, "and it holds the source's samples");
    }

    /// The **automation** moves the volume: a lane of volume is heard.
    ///
    /// The lane is applied **per sample**, because that is what a curve
    /// is: the volume at the sample's own time.
    #[test]
    fn the_automation_moves_the_volume() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-auto-source.wav");
        let output_path = directory.join("craftkit-v2-auto-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.5; 4]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        // A lane of volume: half of it is six decibels down, and the
        // last of it is loud again.
        session
            .execute(
                "sound.lane.add",
                &json!({
                    "track": track,
                    "param": "volume",
                    "points": [[0, -6.0], [3, 0.0]],
                }),
            )
            .expect("the lane is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 4);
        // The first sample is at the lane's first point: six decibels
        // down is about half.
        assert!((sound.samples[0] - 0.25).abs() < 0.01, "the first sample is quieter: {}", sound.samples[0]);
        // The last is at the second point: nothing down at all.
        assert!((sound.samples[3] - 0.5).abs() < 0.01, "the last sample is loud again: {}", sound.samples[3]);
    }

    /// A **mute** lane mutes the track, and a lane of a parameter the
    /// kit does not know is drawn and not heard.
    #[test]
    fn a_mute_lane_mutes_the_track() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-mute-source.wav");
        let output_path = directory.join("craftkit-v2-mute-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.5; 2]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 1.0]] }))
            .expect("the lane is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert_eq!(sound.frames, 2);
        assert!((sound.samples[0] - 0.0).abs() < 0.001, "the track is muted: {}", sound.samples[0]);
    }

    /// The **routing** is heard: a track that sends to a bus is heard
    /// twice — once on its own and once through the bus.
    #[test]
    fn the_routing_is_heard() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-send-source.wav");
        let output_path = directory.join("craftkit-v2-send-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[0.25, 0.25]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        // The track sends to a bus, at six decibels down: about half.
        session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
        let bus = session.document().node_by_name("fx").expect("the bus is there").id;
        session
            .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": -6.0 }))
            .expect("the send is added");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        // A quarter on its own and a quarter at half: about three
        // eighths.
        assert!(
            (sound.samples[0] - 0.375).abs() < 0.01,
            "the track and the bus are both heard: {}",
            sound.samples[0]
        );
    }

    /// An **insert** is run over the track's signal: the insert names a
    /// plugin and the plugin is a program.
    #[test]
    fn an_insert_is_run_over_the_track() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-insert-source.wav");
        let output_path = directory.join("craftkit-v2-insert-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[1.0, 1.0]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
            .expect("the clip is added");
        // A gain insert, at six decibels down.
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
            .expect("the insert is added");
        let insert =
            session.document().walk().find(|node| node.kind == "sound.insert").expect("the insert is there");
        session
            .execute("sound.insert.set", &json!({ "insert": insert.id, "parameters": { "gain_db": -6.0 } }))
            .expect("the insert is changed");
        session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect("the sound is rendered");

        let written = std::fs::read(&output_path).expect("the sound is on the disk");
        let sound =
            craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
        assert!((sound.samples[0] - 0.5).abs() < 0.01, "the insert changed the sound: {}", sound.samples[0]);
    }

    /// A plugin that is **not in the catalogue** is refused: a plugin
    /// that cannot be made cannot be run.
    #[test]
    fn a_plugin_that_is_not_in_the_catalogue_is_refused() {
        let directory = std::env::temp_dir();
        let source_path = directory.join("craftkit-v2-unknown-source.wav");
        let output_path = directory.join("craftkit-v2-unknown-out.wav");
        let bytes = craftkit_wav::write(1, 48_000, &[1.0]);
        std::fs::write(&source_path, bytes).expect("the source is written");

        let mut session = session();
        session
            .execute("sound.source.add", &json!({ "path": source_path.to_string_lossy() }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
            .expect("the clip is added");
        session
            .execute("sound.insert.add", &json!({ "track": track, "plugin": "reverb" }))
            .expect("the insert is added");
        let error = session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect_err("there is no reverb in the catalogue");
        assert!(error.to_string().contains("reverb"), "the refusal says what is wrong: {error}");
    }

    /// A source that is **not there** is refused: the renderer does not
    /// make a sound out of nothing.
    #[test]
    fn a_source_that_is_not_there_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.source.add", &json!({ "path": "/nowhere/at/all.wav" }))
            .expect("the source is added");
        let source = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.source")
            .map(|node| node.id)
            .expect("the source is there");
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4, "source": source }))
            .expect("the clip is added");
        let directory = std::env::temp_dir();
        let output_path = directory.join("craftkit-v2-render-missing.wav");
        let error = session
            .execute("sound.render", &json!({ "path": output_path.to_string_lossy() }))
            .expect_err("the source is not there");
        assert!(error.to_string().contains("source"), "the refusal says what is wrong: {error}");
    }

    /// The playhead is **where the sound is**: a line at the position.
    #[test]
    fn the_playhead_is_a_line_at_the_position() {
        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        session.execute("sound.playhead.set", &json!({ "position": 24_000 })).expect("the playhead is set");
        let playhead = session
            .document()
            .walk()
            .find(|node| node.kind == "sound.playhead")
            .expect("the playhead is there");
        let rect = PlayheadKind::place(playhead, session.document()).expect("the playhead has a place");
        // Twenty four thousand samples at forty eight thousand a second
        // is half a second.
        assert!((rect.x + rect.w / 2.0 - 0.5).abs() < 0.001, "the playhead is half a second in: {}", rect.x);
        // And it runs from the ruler's row through the tracks.
        assert_eq!(rect.y, -timeline::RULER_HEIGHT, "it starts at the ruler");
        assert!(rect.h > timeline::RULER_HEIGHT, "and it reaches the tracks");
    }

    /// The playhead is **moved**, and there is still only one of it.
    #[test]
    fn the_playhead_is_moved_and_not_duplicated() {
        let mut session = session();
        session.execute("sound.playhead.set", &json!({ "position": 0 })).expect("the playhead is set");
        session.execute("sound.playhead.set", &json!({ "position": 960 })).expect("the playhead is moved");
        let playheads: Vec<_> =
            session.document().walk().filter(|node| node.kind == "sound.playhead").collect();
        assert_eq!(playheads.len(), 1, "there is one playhead");
        assert_eq!(playheads[0].data["position"], json!(960), "and it is where it was moved");
    }

    /// A position that is **before the beginning** is refused.
    #[test]
    fn a_position_before_the_beginning_is_refused() {
        let mut session = session();
        let error = session
            .execute("sound.playhead.set", &json!({ "position": -1 }))
            .expect_err("that is before the beginning");
        assert!(
            error.to_string().contains("before the beginning"),
            "the refusal says what is wrong: {error}"
        );
    }

    /// The kind of a track is a string, and it is open.
    #[test]
    fn the_kind_of_a_track_is_a_string() {
        let mut session = session();
        session
            .execute("sound.track.add", &json!({ "name": "bass", "track_kind": "instrument" }))
            .expect("the track is added");
        let track = session.document().node_by_name("bass").expect("the track is there");
        let payload = TrackPayload::from_json(&track.data).expect("the payload is read");
        assert_eq!(payload.track_kind, "instrument");
    }

    /// A clip goes on a track, in its playlist.
    #[test]
    fn a_clip_goes_on_a_track() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        let clip = session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip");
        let payload = ClipPayload::from_json(&clip.data).expect("the payload is read");
        assert_eq!(payload.start, 0);
        assert_eq!(payload.length, 48000);
        assert_eq!(clip.rect, None, "a clip is at a time and not at a place");
    }

    /// A clip on something that is not a track is refused.
    #[test]
    fn a_clip_on_something_else_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.clip.add", &json!({ "track": track + 100, "start": 0, "length": 1 }))
            .expect_err("the clip is refused");
        assert!(error.to_string().contains("no node"), "{error}");
    }

    /// A clip with no length is refused.
    #[test]
    fn a_clip_with_no_length_is_refused() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        let error = session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 0 }))
            .expect_err("the clip is refused");
        assert!(error.to_string().contains("no length"), "{error}");
    }

    /// The tempo map is a list of events, and the second one at a tick
    /// changes the first.
    #[test]
    fn the_tempo_is_a_list_of_events() {
        let mut session = session();
        session.execute("sound.tempo.set", &json!({ "bpm": 120.0 })).expect("the tempo is set");
        session.execute("sound.tempo.set", &json!({ "bpm": 140.0, "tick": 960 })).expect("the tempo is set");
        session.execute("sound.tempo.set", &json!({ "bpm": 100.0 })).expect("the tempo is set");
        let map = tempo_map(session.document());
        assert_eq!(map.tempos().len(), 2, "the second tempo at tick zero changed the first");
        assert_eq!(map.tempo_at_tick(0), 100.0);
        assert_eq!(map.tempo_at_tick(960), 140.0);
    }

    /// A tempo that is not a tempo is refused.
    #[test]
    fn a_tempo_that_is_not_a_tempo_is_refused() {
        let mut session = session();
        let error =
            session.execute("sound.tempo.set", &json!({ "bpm": 0.0 })).expect_err("the tempo is refused");
        assert!(error.to_string().contains("not a tempo"), "{error}");
    }

    /// The session's settings live in the document's payload.
    #[test]
    fn the_session_lives_in_the_document() {
        let mut session = session();
        session.execute("sound.session.set", &json!({ "sample_rate": 44100 })).expect("the session is set");
        let settings = session_of(session.document());
        assert_eq!(settings.sample_rate, 44100);
        assert_eq!(
            session.document().data.get("sound").and_then(|sound| sound.get("sample_rate")),
            Some(&json!(44100)),
            "the settings are in the document's payload"
        );
    }

    /// The timeline says where every clip is, in samples and in bars.
    #[test]
    fn the_timeline_says_where_the_clips_are() {
        let mut session = session();
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 48000, "length": 24000 }))
            .expect("the clip is added");
        let plan = TimelineEvaluator::evaluate(session.document());
        let item = &plan.items[0];
        let tracks = item.value.get("tracks").and_then(|tracks| tracks.as_array()).expect("the tracks");
        let clips = tracks[0]
            .get("playlists")
            .and_then(|playlists| playlists.as_array())
            .and_then(|playlists| playlists[0].get("clips").and_then(|clips| clips.as_array()))
            .expect("the clips");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].get("start"), Some(&json!(48000)));
        // At 48 kHz and 120 bpm a beat is 24000 samples, so the clip at
        // 48000 sits on the third beat of the first bar.
        assert_eq!(clips[0].get("position"), Some(&json!("1.3.0")));
    }

    /// A source is a file the document needs.
    ///
    /// The document says so in one place, so that a program can tell what
    /// is missing before it starts editing.
    #[test]
    fn a_source_is_an_asset() {
        let mut session = session();
        session.execute("sound.source.add", &json!({ "path": "kit/kick.wav" })).expect("the source is added");
        let assets = &session.document().assets;
        assert_eq!(assets.len(), 1, "the document needs the file: {assets:?}");
        assert_eq!(assets[0].path, "kit/kick.wav");
        assert_eq!(assets[0].kind, "audio");
        assert_eq!(assets[0].length, None, "nobody has read the file, so nobody knows its length");
    }

    /// The kinds of a sound document are open: a track whose kind the core
    /// has never heard of is still a track.
    #[test]
    fn the_kinds_are_open() {
        let mut document = Document::new("open");
        document.add_node_with("odd", "sound.track", None, json!({ "track_kind": "quantum" }));
        let track = document.nodes[0].clone();
        let payload = TrackPayload::from_json(&track.data).expect("the payload is read");
        assert_eq!(payload.track_kind, "quantum");
    }
}

#[cfg(test)]
mod waveform_tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// A track with a clip on it, and the clip's payload.
    fn a_clip(session: &mut Session) -> (u64, u64, ClipPayload) {
        session.execute("sound.track.add", &json!({ "name": "track" })).expect("the track is added");
        let track = session.document().node_by_name("track").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
            .expect("the clip is added");
        let clip =
            session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        (track, clip, payload)
    }

    /// The **gain** of a clip's sound at that time, with the track's
    /// volume lane saying what it says.
    #[test]
    fn the_gain_follows_the_automation() {
        let mut session = session();
        let (track, clip, payload) = a_clip(&mut session);
        session
            .execute(
                "sound.lane.add",
                &json!({ "track": track, "param": "volume", "points": [[0, 0.0], [48_000, -20.0]] }),
            )
            .expect("the track is automated");
        let node = session.document().node(clip).expect("the clip is there");
        let document = session.document();
        let loud = gain_at(document, node, &payload, 0);
        let quiet = gain_at(document, node, &payload, 48_000);
        assert!((loud - 1.0).abs() < 1e-6, "the sound starts as it is: {loud}");
        assert!((quiet - 0.1).abs() < 1e-3, "and is twenty decibels down at the end: {quiet}");
    }

    /// A **muted** track is silent however loud its sound is.
    #[test]
    fn a_muted_track_is_silent() {
        let mut session = session();
        let (track, clip, payload) = a_clip(&mut session);
        session
            .execute("sound.lane.add", &json!({ "track": track, "param": "mute", "points": [[0, 1.0]] }))
            .expect("the track is muted");
        let node = session.document().node(clip).expect("the clip is there");
        let document = session.document();
        assert_eq!(gain_at(document, node, &payload, 0), 0.0, "the sound is silent");
    }

    /// The **shape** on the bar is the shape of the sound as it is
    /// heard: a gain that falls makes the shape fall.
    #[test]
    fn the_shape_follows_the_gain() {
        let peaks = Peaks { table: vec![-0.5, 0.5, -0.5, 0.5], channels: 1, frames: 2 };
        let rect = Rect::new(0.0, 0.0, 2.0, 1.0);
        let loud = waveform_paths(&rect, &peaks, 0, 2, |_index, _count| 1.0);
        let quiet = waveform_paths(&rect, &peaks, 0, 2, |_index, _count| 0.25);
        assert_eq!(loud.len(), 1, "a mono sound has one band");
        let loud = loud.first().expect("the shape is drawn");
        let quiet = quiet.first().expect("the shape is drawn");
        // The shape is a path: the upper edge first, and the lower edge
        // after it. The upper edge of the loud shape is at the top of
        // the bar, and the quiet one is a quarter of the way down.
        let loud_top = loud.subpaths.first().map(|subpath| subpath.start.y).unwrap_or(0.0);
        let quiet_top = quiet.subpaths.first().map(|subpath| subpath.start.y).unwrap_or(0.0);
        assert!((loud_top - 0.25).abs() < 1e-6, "half a sound is at the top of the bar: {loud_top}");
        assert!(quiet_top > loud_top, "and a quarter of it is lower: {quiet_top}");
    }
}

#[cfg(test)]
mod trim_tests {
    use super::*;

    use craftkit_sdk::Ecosystem;
    use craftkit_sdk::serde_json::json;

    /// The ecosystem of this plugin alone.
    fn ecosystem() -> Ecosystem {
        Ecosystem::from_inventory().expect("the ecosystem is built")
    }

    /// A session over the plugin's own contributions.
    fn session() -> Session {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    }

    /// A track with one clip on it, and the clip's id.
    fn with_clip(session: &mut Session) -> u64 {
        session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
        let track = session.document().node_by_name("drums").expect("the track is there").id;
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48000 }))
            .expect("the clip is added");
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip").id
    }

    /// A trim with an explicit range shortens the clip.
    #[test]
    fn a_trim_shortens_the_clip() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 0, "to": 24000 }))
            .expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.start, 0);
        assert_eq!(payload.length, 24000);
    }

    /// A trim with a range that starts later moves the content with it.
    #[test]
    fn a_trim_moves_the_content_with_it() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 12000, "to": 24000 }))
            .expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.start, 12000);
        assert_eq!(payload.length, 12000);
        assert_eq!(payload.offset, 12000, "the content starts later in the source");
    }

    /// A trim takes its range from the **selection**.
    ///
    /// The selection holds a part of a node as well as whole nodes, and a
    /// part of a clip is what a trim is about.
    #[test]
    fn a_trim_takes_its_range_from_the_selection() {
        let mut session = session();
        let clip = with_clip(&mut session);
        session
            .execute("select.span", &json!({ "node": clip, "from": 0.0, "to": 24000.0 }))
            .expect("the part is selected");
        session.execute("sound.clip.trim", &json!({})).expect("the clip is trimmed");
        let node = session.document().node(clip).expect("the clip is there");
        let payload = ClipPayload::from_json(&node.data).expect("the payload is read");
        assert_eq!(payload.length, 24000, "the clip was trimmed to the selection");
    }

    /// A trim before the clip's own start is refused.
    #[test]
    fn a_trim_before_the_clip_is_refused() {
        let mut session = session();
        let clip = with_clip(&mut session);
        let error = session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": -100.0, "to": 24000.0 }))
            .expect_err("the trim is refused");
        assert!(error.to_string().contains("cannot be trimmed before"), "{error}");
    }

    /// A range that ends before it starts is refused.
    #[test]
    fn a_range_that_ends_first_is_refused() {
        let mut session = session();
        let clip = with_clip(&mut session);
        let error = session
            .execute("sound.clip.trim", &json!({ "clip": clip, "from": 100.0, "to": 100.0 }))
            .expect_err("the trim is refused");
        assert!(error.to_string().contains("starts at"), "{error}");
    }
}
