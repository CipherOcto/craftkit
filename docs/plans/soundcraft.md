# The soundcraft plan: what the upstream has and the sound battery does not

This plan is a survey of one upstream tree against the sound battery, written
before any of it is built. It names ten arcs of future work, in order, and says
plainly what the upstream has that the battery does not.

## 0. The rule

Every arc serves one rule: **a capability the upstream has is a command in the
sound battery, and a capability the upstream does not have is not.**

## 1. What the survey found

The survey read the upstream's command registry and compared it with
`craftkit-plugin-sound` as it stands. The battery has 144 commands. The
upstream's commands that the battery already has are parity, not gaps, and
they are noted below rather than given an arc. Twenty-four rows are gaps:
either the command is absent, or it exists only as a field inside another
command's payload.

| capability | upstream path:line | craftkit's state | verdict |
|---|---|---|---|
| Invert the phase of a track | `crates/engine/src/cmd/mix.rs:66` | partial: `sound.track.set` carries `phase`, but no command of its own | PORT |
| Trim a track's input | `crates/engine/src/cmd/mix.rs:77` | partial: `sound.track.set` carries `trim_db` | PORT |
| Bypass a plugin | `crates/engine/src/cmd/mix.rs:100` | partial: `sound.insert.set` carries `bypass` | PORT |
| Surround-pan a send | `crates/engine/src/cmd/surround.rs:19` | absent | PORT |
| Copy automation to a send | `crates/engine/src/cmd/mix.rs:176` | absent | PORT |
| Track comments | `crates/engine/src/cmd/track.rs:65` | absent | PORT |
| Track colour | `crates/engine/src/cmd/track.rs:66` | absent | PORT |
| Track folders | `crates/engine/src/cmd/track.rs:43` | partial: the battery groups tracks but has no folder | PORT |
| Track playlists: select and promote | `crates/engine/src/cmd/track.rs:123` | partial: the battery has playlists, but no per-track select or promote | PORT |
| Extract the MIDI of a track | `crates/engine/src/cmd/track_more.rs:52` | partial: `sound.track.midi.import` imports; there is no extract | PORT |
| Clear the material of a range | `crates/engine/src/cmd/edit.rs:22` | absent | PORT |
| Insert silence | `crates/engine/src/cmd/edit.rs:95` | absent (there is `sound.time.insert`, which inserts time on the timeline) | PORT |
| Shift a range later or earlier | `crates/engine/src/cmd/edit.rs:94` | absent | PORT |
| Nudge a range | `crates/engine/src/cmd/edit.rs:140` | absent | PORT |
| Paste special | `crates/engine/src/cmd/edit_more.rs:103` | absent | PORT |
| Copy a selection to another playlist | `crates/engine/src/cmd/edit_more.rs:179` | absent | PORT |
| Set a clip's colour | `crates/engine/src/cmd/clip.rs:117` | absent | PORT |
| Capture a range as a clip | `crates/engine/src/cmd/clip.rs:177` | absent | PORT |
| Regroup clips | `crates/engine/src/cmd/clip_more.rs:13` | absent | PORT |
| Split a note into two | `crates/engine/src/cmd/event_more.rs:81` | absent | PORT |
| Extract the chords of a clip | `crates/engine/src/cmd/event_more.rs:120` | absent | PORT |
| Import audio | `crates/engine/src/cmd/file.rs:62` | partial: `sound.source.add` makes a source of a file; there is no import that reads and registers one | PORT |
| Export the session as text | `crates/engine/src/cmd/file.rs:121` | absent | PORT |
| Session audio health check | `crates/engine/src/cmd/health.rs:11` | absent | PORT |
| Inspect the session, a track or a clip | `crates/engine/src/cmd/query.rs:9` | partial: the battery's kinds are readable, but there is no one inspect command | PORT |
| Place an audio file on a track | `crates/engine/src/cmd/clip.rs:141` | partial: the parts exist, but there is no one command that places a source at a place | PORT |

The twenty-four rows above are the gaps. The ten arcs of §2 cover twenty of
them; the four rows left for later are named in §2 and in §5: copying
automation to a send, the folder of tracks, promoting a playlist onto a track,
and writing a track's MIDI out to a file.

The rest of the upstream's surface is parity: the battery already has a
command for it, and a command the tree has is not an arc. Those rows carry the
same PORT mark in the survey, because the survey's verdict column marks every
capability the upstream has, and it is the state column that says whether the
command is already there.

The survey's five sentences are the shape of the whole comparison. The upstream
has a transport and this battery has none. The upstream has a runtime layer —
automation modes, metering, a playing mixer — and this battery has the
document half and none of the runtime half. The upstream has clip- and
track-level colour, comments, height, hidden state, pin and folders, and this
battery has none of them. The upstream has a family of commands that move a
value between two owners, and this battery has the owners but not the moves.
The upstream has an interchange and an export surface this battery does not
reach.

The runtime half and the export surface are named in §5. The rest of the
comparison — the facts, the moves, the edits — is what the ten arcs are.

## 2. The arcs

Each arc is three edits: a row in the port manifest, a row in the README's claim
table, and a section in the README. The sections reserved for this plan are
**§1272 through §1281**, one per arc, in the order below.

| arc | rule | from | to | module | command | test | section |
|---|---|---|---|---|---|---|---|
| **1** | a track's own facts are facts on the track: the phase, the input trim, the comment and the colour | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/track.rs` | `sound.track.phase`, `sound.track.trim`, `sound.track.comment`, `sound.track.color` | `crates/craftkit-plugin-sound/tests/track.rs` | §1272 |
| **2** | bypassing is a flag on the insert, and not a removal of it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/mixer.rs` | `sound.insert.bypass` | `crates/craftkit-plugin-sound/tests/insert.rs` | §1273 |
| **3** | a send carries a surround pan the same way a track does | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/surround.rs` | `sound.send.surround` | `crates/craftkit-plugin-sound/tests/surround.rs` | §1274 |
| **4** | a range has its own operations: clearing it, shifting it, nudging it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/ranges.rs` | `sound.range.clear`, `sound.range.shift`, `sound.range.nudge` | `crates/craftkit-plugin-sound/tests/ranges.rs` | §1275 |
| **5** | silence is a range with no material in it | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/time.rs` | `sound.time.insert.silence` | `crates/craftkit-plugin-sound/tests/silence.rs` | §1276 |
| **6** | the material in the clipboard may be moved to another place in special ways | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clipboard.rs` | `sound.clipboard.paste.merge`, `sound.playlist.copy.to` | `crates/craftkit-plugin-sound/tests/clipboard.rs` | §1277 |
| **7** | a clip's own facts are facts on the clip: the colour, the capture, the group | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clip.rs` | `sound.clip.color`, `sound.clip.capture`, `sound.clip.regroup` | `crates/craftkit-plugin-sound/tests/clip.rs` | §1278 |
| **8** | the notes of a clip are rules over the notes | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/midi.rs` | `sound.clip.midi.note.split`, `sound.clip.midi.chords` | `crates/craftkit-plugin-sound/tests/midinotes.rs` | §1279 |
| **9** | the session is read from outside: a file is imported, the session is written out, the media is checked, the document is inspected | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/session.rs` | `sound.source.import`, `sound.session.text`, `sound.health.read`, `sound.session.inspect` | `crates/craftkit-plugin-sound/tests/session.rs` | §1280 |
| **10** | placing is a source, a track and a place, and the three parts are already commands | `docs/plans/soundcraft.md` | `craftkit-plugin-sound` | `src/clip.rs` | `sound.clip.place` | `crates/craftkit-plugin-sound/tests/place.rs` | §1281 |

These arcs are ten because the survey's twenty-four gaps fall into ten groups
by owner. The track's own facts are one arc, because the phase, the trim, the
comment and the colour are all facts about one track and each of them is a
field the battery's track command already carries in one form. The insert's
bypass is one arc on its own, because it is a flag on an insert and the
insert is a different owner. The send's surround pan is one arc, because a
send is a third owner and the battery's surround commands are about tracks.

The range's operations are one arc — clear, shift, nudge — because all three
are edits over the same range and they share one notion of what a range is.
Silence is an arc of its own, because it is an insertion and not an edit of
what is already there, and because the battery already has an insertion of
time that it must not be confused with. The material moved to another place is
one arc, because a special paste and a copy into another playlist are the same
move with different rules over the same clipboard.

The clip's own facts are one arc: the colour, the capture and the regroup are
all about a clip, and a group is already a name over a set of clips in the
battery. The notes of a clip are one arc, because a split and a chord
extraction are both rules over the notes of one MIDI clip.

The session read from outside is one arc of four commands, because all four
read or register the session rather than edit it: importing a file, writing the
session out as text, checking that the media is still there, and inspecting
what the document holds. The placement of a source is an arc of its own,
because placing composes three existing commands into one and is therefore
the only arc here that is a composition rather than a new fact.

Four rows are left for later and are named in §5: copying automation to a send
(`crates/engine/src/cmd/mix.rs:176`), the folder of tracks
(`crates/engine/src/cmd/track.rs:43`), promoting a playlist onto a track
(`crates/engine/src/cmd/track.rs:123`), and writing a track's MIDI out
(`crates/engine/src/cmd/track_more.rs:52`). Each of them needs a model the
battery does not yet have — a lane that can be copied onto a send, a folder
beside a group, a per-track playlist selection, and a file written out of a
document that holds notes — and each is a model before it is a command. Ten arcs
that land are worth more than fourteen that do not.

## 3. The order, and what each arc ships

| arc | ships | cost |
|---|---|---|
| 1 | a track's own facts, each as a command of its own | small — four fields the track command already carries |
| 2 | the insert's bypass flag | small — a flag and a reader |
| 3 | a send's surround pan | small — the same pan a track has |
| 4 | the range's own operations | medium — three edits that each move or remove material |
| 5 | silence in a range | medium — an insertion that leaves a hole and a length |
| 6 | the special pastes and the copy into another playlist | medium — a clipboard that is read by rule, not pasted whole |
| 7 | the clip's own facts | medium — a colour, a capture and a move between groups |
| 8 | the notes of a clip | medium — a split and a read of the chords |
| 9 | the session read from outside | medium — a file reader, a text writer, a check and an inspect |
| 10 | the placement of a source | small — a composition of three commands that already exist |

The order follows the owners, from the top of the session downwards. The
track's facts come first, because a track is what the other owners hang from,
and because two of the four commands of arc 1 are the fields the battery's
existing track command already carries. The insert's flag and the send's pan
follow, because they are the two other owners that carry a setting of their
own.

The range's operations come next, because they need a range to be a range, and
the silence follows them, because it is the same range with nothing in it. The
clipboard arc comes after the ranges, because a special paste is a rule over
material that has already been copied out of one. The clip's own facts come
after that, because a capture is a range turned into a clip and a regroup is a
move between groups the battery already has. The notes of a clip come after the
clip, because a note lives in a MIDI clip.

The session read from outside comes near the end, because it is a read of the
whole document and it is worth reading once the document holds everything the
arcs above put in it. The placement of a source comes last, because it is a
composition of commands that already exist and nothing else depends on it.

## 4. The tests

One test file per arc, named in the arc's row above, and each one lands with
the arc it belongs to.

- **1** — a track's phase, trim, comment and colour are what was set; a phase that is not a sign is refused; a track that is not there is refused.
- **2** — a bypassed insert says so and stays on the chain; un-bypassing it says so otherwise; bypassing an insert that is not there is refused.
- **3** — a send's surround pan is what was set; a send that is not there is refused.
- **4** — a cleared range holds no material; a shifted range's material moved and the rest did not; a nudged range moved by one step of the current unit; a range that is not there is refused.
- **5** — the silence is the length asked for and holds no material; the material around it stays where it was.
- **6** — a merged paste keeps what was on the timeline and adds what was copied; a copy into another playlist puts the material in that playlist and not in the one it came from.
- **7** — a clip's colour is what was set; a capture makes a clip of the range's material and leaves the range as it was; a regroup moves the clips from one group into another and empties the first.
- **8** — a split note makes two notes that meet at the split point; the chords of a clip are the notes that fall together; a note that is not there is refused.
- **9** — an imported file becomes a source that is registered; the text written out holds the session's facts; a health check reports a missing file and changes nothing; an inspect answers with what the document holds and writes nothing.
- **10** — a placed source sits at the place asked for, on the track asked for; a place that is not a place is refused.

## 5. What it does not do

The four rows left out of the arcs are named here with the reason each is not
now.

- **Copy automation to a send** (`crates/engine/src/cmd/mix.rs:176`) — it needs a lane that can be read on one owner and written on another, and the battery's lanes are per owner today.
- **Track folders** (`crates/engine/src/cmd/track.rs:43`) — the battery groups tracks, and a folder is a group that is drawn folded; the drawing is the front end's business, so the folder waits for a group that carries more than a name.
- **Track playlists: select and promote** (`crates/engine/src/cmd/track.rs:123`) — a per-track selection of an alternate set of clips is a second set of clips per track, and the battery has playlists without that indirection.
- **Extract the MIDI of a track** (`crates/engine/src/cmd/track_more.rs:52`) — it writes a file out of a document that holds notes, and the battery's import reads a file rather than writing one.

The later work is named here too, and none of it is in this plan: automation
modes, plugin state kept in the session, an instrument on a track, a sampler,
pre-roll, converting volume automation to clip gain, a track's timebase, a
track preset, rendering a clip's gain into the audio, removing a warp, removing
a pitch shift, an edit mode, tempo curves, step input, restoring a
performance, audio as MIDI, an interchange file analysis, setup and devices, and
metering. Each of those is a runtime model or a whole subsystem before it is a
command: the automation modes are a state machine over the lanes, the
instrument and the sampler are devices, the metering is a running mixer, and the
interchange analysis is a reader of somebody else's file format.

The out-of-scope rows are named for the same reason, and they are not gaps at
all: the transport and playback, because running a document is somebody else's
business; undo, redo and selection, which the core owns; the cursor tools and
the zoom, which the front end and the camera own; plugin discovery, which the
plugin batteries own; pictures and video, which the film battery owns; saving
and opening the session, which the kit owns; and the score printing, the
keyboard shortcuts and the view preferences.

## 6. The open questions

The survey read the upstream's command registry and not its handler bodies, so
for every upstream row it knows what the command asks for and not the
arithmetic behind it. The rules in §2 are therefore the rules the survey read,
and the arithmetic behind them is still to be read.

A few upstream ids are cited at the line where the macro opens, which is one
line above the id literal. The row names the command; the line is one off.

The survey could not decide whether some upstream capabilities duplicate this
tree's other batteries, because it read only their ids and not their rules. The
batteries it compared with are the film battery and the plugin batteries, and
whether a row is a gap or somebody else's command is still open.

The view and options families were classified out of scope on the tree's own
rules, and if the owner wants saved view preferences in the document, several
of those become ports.

The edit-mode family was judged later rather than out of scope, and the
survey is not certain of that judgement.

The survey did not check whether one command is the intended home for a
track's trim and phase, or whether each should be a command of its own. Arc 1
takes them as commands of their own, on the reading that a command is what an
agent calls.
