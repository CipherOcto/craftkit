//! The **film battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §866 converted this battery from one declaration per command to
//! **one table per file** — the `commands!` table of the derive. A
//! conversion that changed a single id, a label, a layer, an icon, a menu
//! path, a parameter or a flag would be a conversion that changed the
//! tree.
//!
//! So the tree's own registry is **pinned**: every command of this battery
//! is written down here — its id, its label, its area, its capability, its
//! layer, its icon, its documentation, its chord, its menu, its parameters
//! and its flags — and this test says whether the registry still says it.
//!
//! The snapshot was taken **before** the conversion, from the registry
//! itself. So the test is not a description of what the table says; it is
//! a description of **what the tree was**, and the table has to agree with
//! it.
//!
//! # Why it is here and not in the battery
//!
//! A battery's own test binary sees **no commands**: the plugin's items
//! are behind its anchor, and the anchor keeps them for a program that
//! links the plugin. The studio links the batteries, so the studio's
//! ecosystem is where the registry of a battery can be read.

#![cfg(feature = "film")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the film battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r##"film.bin.add | Add | film | command | film | plus | Put a **bin** in the project. | - | film | name(text)! help="What to call it.",parent(node-ref)? help="The bin it goes in, if it goes in one." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"film.bin.get | Get | film | command | film | list | Read a **bin**. | - | film | bin(node-ref)! help="The bin to read." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.bin.remove | Remove | film | command | film | minus | **Take away** a bin. | - | film | bin(node-ref)! help="The bin." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.bin.rename | Rename | film | command | film | pencil | **Name** a bin. | - | film | bin(node-ref)! help="The bin.",name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.add | Add | film | command | film | plus | Add a **cue** to a caption track. | - | film | track(node-ref)! help="The caption track.",start(number)! help="The frame the cue starts at.",end(number)! help="The frame the cue ends at.",text(text)! help="The text of the cue." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.burn | Burn | film | command | film | image | **Burn** a caption into a frame: the caption is drawn, and the picture of it is written. | - | film | text(text)! help="The text of the caption.",track(node-ref)? help="The caption track that holds the style; the tree's own style when it is not said.",width(number)! help="The width of the frame.",height(number)! help="The height of the frame.",settings(text)? help="The settings of the cue: `line:50% align:center`.",to(text)! help="Where the picture is written." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.caption.duplicate | Duplicate | film | command | film | copy | **Duplicate** a cue: the copy stands by the delta that is given. | - | film | caption(node-ref)! help="The cue.",delta(number)! help="The frames to stand it by." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.import | Import | film | command | film | plus | **Import** a caption file as a new caption track of a sequence. | - | film | sequence(node-ref)! help="The sequence the track goes on.",path(text)! help="The file to read.",format(text)? help="The format of the file: `subrip` when it is not given.",name(text)? help="The name of the track: the name of the file when it is not given." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.layout | Layout | film | command | film | list | **Lay** a **caption** out on a frame of a given size. | - | film | text(text)! help="The text of the caption.",track(node-ref)? help="The caption track that holds the style; the tree's own style when it is not said.",width(number)! help="The width of the frame.",height(number)! help="The height of the frame.",settings(text)? help="The settings of the cue: `line:50% align:center`." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.caption.merge | Merge | film | command | film | merge | **Merge** the cues that are next to each other into one. | - | film | captions(json)? help="The cues, in the order they stand in." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.move | Move | film | command | film | arrow | **Move** a cue: it stays at or after the start of the sequence and it does not run into its neighbours. | - | film | caption(node-ref)! help="The cue.",delta(number)! help="The frames to move it by." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.read | Read | film | command | film | list | The **cues** of a caption track, in the order they start in. | - | film | track(node-ref)! help="The caption track." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.caption.remove | Remove | film | command | film | minus | Take a **cue** away from a caption track. | - | film | caption(node-ref)! help="The cue." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.set | Set | film | command | film | pencil | **Set** the text of a cue, and the **speaker** of it. | - | film | caption(node-ref)! help="The cue.",text(text)? help="Its new text.",speaker(text)? help="Its new speaker. An empty speaker takes it away." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.split | Split | film | command | film | split | **Split** a cue in two at a frame. | - | film | caption(node-ref)! help="The cue.",at(number)! help="The frame to split it at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.times | Times | film | command | film | clock | **Set** the times of a cue. | - | film | caption(node-ref)! help="The cue.",start(number)? help="The frame it starts at.",end(number)? help="The frame it ends at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.track.add | Add | film | command | film | plus | Add a **caption track** to a sequence. | - | film | name(text)! help="What to call it.",sequence(node-ref)! help="The sequence it is in.",format(text)? help="The format of the track: `subtitle` when it is not given." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.track.delete | Delete | film | command | film | minus | **Delete** a caption track, and its cues go with it. | - | film | track(node-ref)! help="The caption track." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.track.set | Set | film | command | film | pencil | **Set** the track of the captions: its name, its format, its language and its flags. | - | film | track(node-ref)! help="The caption track.",name(text)? help="Its new name.",format(text)? help="Its format: `subtitle`, `cea608`, `cea708` or `teletext`.",language(text)? help="Its language: a tag such as `en` or `fr-CA`.",enabled(bool)? help="Whether it is shown in the monitor.",locked(bool)? help="Whether it may be changed.",sync_lock(bool)? help="Whether it follows the inserts and the extracts of the other tracks." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.track.style | Style | film | command | film | text | **Set** the style of a caption track. | - | film | track(node-ref)! help="The caption track.",font(text)? help="The font the cues are written in.",size(number)? help="The size of the cues, in the tree's pixels.",colour(text)? help="The colour of the cues.",background(bool)? help="Whether a box is drawn behind the cues.",background_colour(text)? help="The colour of the box.",align(text)? help="`left`, `center` or `right`.",anchor(text)? help="`top`, `middle` or `bottom`.",margin(number)? help="The room from the edge, as a fraction of the height of the frame.",line_spacing(number)? help="The line spacing, as a multiple of the size.",outline(number)? help="The width of the outline, in the tree's pixels.",outline_colour(text)? help="The colour of the outline.",reset(bool)? help="Whether the style is put back as it was." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.caption.trim | Trim | film | command | film | trim | **Trim** a cue at its **in** edge or at its **out** edge. | - | film | caption(node-ref)! help="The cue.",edge(text)! help="The edge: `in` or `out`.",delta(number)! help="The frames to take away, or to add." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.captions.detect | Detect | film | command | film | search | **Detect** the format of a caption file. | - | film | path(text)! help="The file to look at." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.captions.export | Export | film | command | film | plus | **Export** the cues of a caption track as a caption file. | - | film | track(node-ref)! help="The caption track.",path(text)! help="The file to write.",format(text)? help="The format of the file: `subrip` when it is not given.",cea_608(bool)? help="Whether an MCC file carries the **CEA-608** pairs of the field one: it does when this is not given.",cea_708(bool)? help="Whether an MCC file carries the **CEA-708** service data: it does when this is not given." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.captions.read | Read | film | command | film | list | **Read** the **cues** of a caption file. | - | film | path(text)! help="The file to read.",format(text)? help="The format of the file: `subrip` when it is not given." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.captions.write | Write | film | command | film | plus | **Write** a caption file. | - | film | path(text)! help="The file to write.",format(text)? help="The format of the file: `subrip` when it is not given.",cues(json)! help="The cues: each one a `start`, an `end` and a `text`.",blocks(json)? help="The blocks of a WebVTT file: the notes, the styles and the regions.",drop_frame(bool)? help="Whether an SCC file carries the **drop-frame** timecode: it does not when this is not given.",cea_608(bool)? help="Whether an MCC file carries the **CEA-608** pairs of the field one: it does when this is not given.",cea_708(bool)? help="Whether an MCC file carries the **CEA-708** service data: it does when this is not given." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.add | Add | film | command | film | plus | Put a **clip** on a track. | - | film | track(node-ref)! help="The track it goes on.",item(node-ref)! help="The item it plays.",start(number)! help="Where it starts on the timeline, in frames.",duration(number)! help="How long it is, in frames.",source_in(number)? help="Where in the media it begins, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.enable | Enable | film | command | film | power | **Enable** a clip, or disable it. | - | film | clip(node-ref)! help="The clip.",on(bool)? help="Whether the clip plays. Without it the flag is toggled." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.extract | Extract | film | command | film | minus | **Extract** a range of a track: take the clips that are in it away and close the gap. | - | film | track(node-ref)! help="The track the range is on.",from(number)! help="Where the range starts, in frames.",to(number)! help="Where the range ends, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.get | Get | film | command | film | list | Read a **clip**. | - | film | clip(node-ref)! help="The clip to read." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.clip.hold | Hold | film | command | film | pin | **Hold** a frame of a clip. | - | film | clip(node-ref)! help="The clip.",at(number)! help="The **source** frame the clip holds: the frame at that point of the media, held to the end of the clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.hold.delete | Delete | film | command | film | unpin | **Take away** the frame hold of a clip. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.insert | Insert | film | command | film | arrow | **Insert** a clip at a place on a track. | - | film | clip(node-ref)! help="The clip.",track(node-ref)! help="The track it goes on.",start(number)! help="Where it goes, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.label | Label | film | command | film | paint | Put a **label** on a clip. | - | film | clip(node-ref)! help="The clip.",label(text)! help="The label: `Violet`, `Iris`, …" | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.label.delete | Delete | film | command | film | minus | Take the **label** off a clip. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.label.group | Group | film | command | film | list | The **group** of a clip: every clip that carries its label. | - | film | clip(node-ref)! help="The clip whose label makes the group." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.clip.lift | Lift | film | command | film | minus | **Lift** a range of a track: take the clips that are in it away. | - | film | track(node-ref)! help="The track the range is on.",from(number)! help="Where the range starts, in frames.",to(number)! help="Where the range ends, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.link | Link | film | command | film | link | **Link** two clips together. | - | film | clip(node-ref)! help="The clip.",with(node-ref)! help="The clip it is linked to." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.move | Move | film | command | film | arrow | **Move** a clip to a place on a track. | - | film | clip(node-ref)! help="The clip.",track(node-ref)! help="The track it goes on. The track it is on, if it stays there.",start(number)! help="Where it starts on that track, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.nest | Nest | film | command | film | group | **Nest** the clips of a sequence into a clip of their own. | - | film | clips(json)! help="The clips that are nested.",name(text)! help="What to call the clip that holds them." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.remove | Remove | film | command | film | minus | **Take away** a clip. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.rename | Rename | film | command | film | pencil | **Name** a clip. | - | film | clip(node-ref)! help="The clip.",name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.replace | Replace | film | command | film | swap | **Replace** the media a clip plays. | - | film | clip(node-ref)! help="The clip.",item(node-ref)! help="The media it plays now." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.reverse | Reverse | film | command | film | repeat | **Reverse** a clip: play its media backwards. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.ripple | Ripple | film | command | film | minus | **Take away** a clip and close the gap. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.roll | Roll | film | command | film | split | **Roll** the cut between two clips. | - | film | left(node-ref)! help="The clip before the cut.",right(node-ref)! help="The clip after the cut.",delta(number)! help="How far the cut moves, in frames. A negative number moves it the other way." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.slide | Slide | film | command | film | slide | **Slide** a clip between its neighbours. | - | film | clip(node-ref)! help="The clip.",delta(number)! help="How far it slides, in frames. A negative number slides it the other way." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.slip | Slip | film | command | film | move | **Slip** a clip: show a different part of its media. | - | film | clip(node-ref)! help="The clip.",delta(number)! help="How far the clip slips, in frames of the timeline. A negative number slips it the other way." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.speed | Speed | film | command | film | repeat | **Set** the speed of a clip. | - | film | clip(node-ref)! help="The clip.",speed(number)! help="The speed: one is the media as it is, two is twice as fast.",ripple(bool)? help="Whether the clips that were after it move up by the change in its length." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.split | Split | film | command | film | scissors | **Split** a clip at a place. | - | film | clip(node-ref)! help="The clip.",at(number)! help="Where it is divided, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.stretch | Stretch | film | command | film | expand | **Stretch** one edge of a clip. | - | film | clip(node-ref)! help="The clip.",edge(text)! help="The edge that is stretched: `in` or `out`.",delta(number)! help="How much it is stretched, in frames. A negative number stretches it the other way." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.synchronise | Synchronise | film | command | film | link | **Synchronise** a clip with another one. | - | film | clip(node-ref)! help="The clip.",reference(node-ref)! help="The clip it lines up with.",method(text)? help="Which point of the reference lines up: `in` or `out`. The default is `in`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.trim | Trim | film | command | film | scissors | **Trim** one edge of a clip. | - | film | clip(node-ref)! help="The clip.",edge(text)! help="The edge that is trimmed: `in` or `out`.",delta(number)! help="How much it is trimmed, in frames. A negative number trims the other way." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.ungroup | Ungroup | film | command | film | ungroup | **Take a nested clip apart** and put its clips back where they were. | - | film | clip(node-ref)! help="The nested clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.clip.unlink | Unlink | film | command | film | unlink | **Unlink** a clip from its group. | - | film | clip(node-ref)! help="The clip." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.export.ale | Ale | film | command | film | list | **Write** the Avid log exchange of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.export.edl | Edl | film | command | film | list | **Write** the edit decision list of a sequence. | - | film | sequence(node-ref)! help="The sequence.",title(text)? help="The title of the list. Without it, the name of the sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.export.fcpxml | Fcpxml | film | command | film | list | The **text** of the FCPXML of a sequence. | - | film | sequence(node-ref)! help="The sequence.",width(number)! help="The **width** of the sequence, in pixels. The document does not say it, so the caller does.",height(number)! help="The **height** of the sequence, in pixels." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.export.otio | Otio | film | command | film | list | The **text** of the OTIO of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.import.ale | Ale | film | command | film | plus | **Read** an Avid log exchange and make the **sequence** it names. | - | film | path(text)! help="The file to read.",name(text)? help="What to call the sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.import.edl | Edl | film | command | film | plus | **Read** an edit decision list and make the **sequence** it names. | - | film | path(text)! help="The file to read.",name(text)? help="What to call the sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.import.fcp7 | Fcp7 | film | command | film | plus | **Read** an FCP7 document and make the **sequence** it names. | - | film | path(text)! help="The file to read.",name(text)? help="What to call the sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.import.fcpxml | Fcpxml | film | command | film | plus | **Read** an FCPXML file and make the **sequence** it names. | - | film | path(text)! help="The file to read.",name(text)? help="What to call the sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.import.otio | Otio | film | command | film | plus | **Read** an OTIO file and make the **sequence** it names. | - | film | path(text)! help="The file to read.",name(text)? help="What to call the sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.add | Add | film | command | film | plus | Put an **item** in a bin. | - | film | name(text)! help="What to call it.",bin(node-ref)! help="The bin it goes in.",item(number)! help="The item's own id, in the editor.",path(text)? help="Where the media is, if it is a media file.",duration(number)? help="How long it is, in frames.",offline(bool)? help="Whether it was made offline on purpose." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.generator | Generator | film | command | film | plus | Put a **generated** item in a bin. | - | film | name(text)! help="What to call it.",bin(node-ref)! help="The bin it goes in.",item(number)! help="The item's own id, in the editor.",kind(text)! help="What kind of generator it is: `Black Video`, `Transparent Video`, `Bars and Tone`, `Universal Counting Leader` or `Color Matte`.",seconds(number)? help="How long it is, in seconds. The default is five, and the bars and the tone are ten.",rate(text)? help="The rate it is made at: `25`, `23.976`, … The default is the tree's own `23.976`.",colour(text)? help="The colour of the matte: `#rrggbb`. The default is the tree's own dark grey, and a colour on another generator is refused." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.get | Get | film | command | film | list | Read a **media item**. | - | film | item(node-ref)! help="The item to read." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.item.label | Label | film | command | film | paint | Put a **label** on a media item. | - | film | item(node-ref)! help="The item.",label(text)! help="The label: `Violet`, `Iris`, …" | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.label.delete | Delete | film | command | film | minus | Take the **label** off a media item. | - | film | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.label.group | Group | film | command | film | list | The **group** of an item: every item that carries its label. | - | film | item(node-ref)! help="The item whose label makes the group." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.item.mark | Mark | film | command | film | flag | Put the **marks** on a media. | - | film | item(node-ref)! help="The media.",mark_in(number)? help="The mark in: the first frame of the media that may be played.",mark_out(number)? help="The mark out: the frame after the last one that may be played." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.matte.colour | Colour | film | command | film | paint | The **colour** of a colour matte is changed. | - | film | item(node-ref)! help="The item: the colour matte.",colour(text)! help="The colour of the matte: `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.remove | Remove | film | command | film | minus | **Take away** a media item. | - | film | item(node-ref)! help="The item." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.item.rename | Rename | film | command | film | pencil | **Name** a media item. | - | film | item(node-ref)! help="The item.",name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.add | Add | film | command | film | flag | **Add** a marker to a sequence. | - | film | sequence(node-ref)! help="The sequence the marker goes on.",start(number)! help="Where it starts, in frames.",duration(number)? help="How long it is, in frames.",name(text)? help="What to call it.",comment(text)? help="What to say about it.",color(text)? help="Its colour: one of the tree's own sixteen labels.",kind(text)? help="What kind of marker it is: `Comment`, `Chapter`, `Segmentation`, `WebLink` or `FlashCue`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.clear | Clear | film | command | film | trash | **Take away** every marker of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.next | Next | film | command | film | flag | **Move the playhead to the next marker** of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.previous | Previous | film | command | film | flag | **Move the playhead to the marker before** the one it is at. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.remove | Remove | film | command | film | trash | **Take** a marker away. | - | film | marker(node-ref)! help="The marker." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.marker.set | Set | film | command | film | pencil | **Change** a marker. | - | film | marker(node-ref)! help="The marker.",name(text)? help="What to call it.",comment(text)? help="What to say about it.",color(text)? help="Its colour: one of the tree's own sixteen labels.",kind(text)? help="What kind of marker it is.",duration(number)? help="How long it is, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.end | End | film | command | film | forward | **Go to** the end of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.get | Get | film | command | film | list | **Read** the playhead of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.playhead.goto.mark | Mark | film | command | film | playhead | **Go to** the **mark** of a sequence: the mark in or the mark out. | - | film | sequence(node-ref)! help="The sequence.",which(text)! help="Which mark the playhead goes to: `in` or `out`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.nextedit | Nextedit | film | command | film | next | **Go to** the next edit point of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.prevedit | Prevedit | film | command | film | previous | **Go to** the previous edit point of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.set | Set | film | command | film | playhead | **Set** the playhead of a sequence. | - | film | sequence(node-ref)! help="The sequence.",frame(number)! help="The frame." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.start | Start | film | command | film | rewind | **Go to** the start of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.playhead.step | Step | film | command | film | step | **Step** the playhead of a sequence. | - | film | sequence(node-ref)! help="The sequence.",frames(number)! help="The frames: a negative number steps back." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.project | Project | film | command | film | list | The **project**: the bins, the items, and the clips that play them. | - | film |  | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.sequence.mark.clear | Clear | film | command | film | minus | **Take** the **marks** of a sequence away. | - | film | sequence(node-ref)! help="The sequence.",which(text)! help="Which mark is taken away: `in`, `out` or `both`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.sequence.mark.in | In | film | command | film | flag | **Set** the **mark in** of a sequence. | - | film | sequence(node-ref)! help="The sequence.",frame(number)! help="The frame the mark in is at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.sequence.mark.out | Out | film | command | film | flag | **Set** the **mark out** of a sequence. | - | film | sequence(node-ref)! help="The sequence.",frame(number)! help="The frame the mark out is at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.sequence.rate.get | Get | film | command | film | list | **Read** the rate of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.sequence.rate.set | Set | film | command | film | clock | **Set** the rate of a sequence. | - | film | sequence(node-ref)! help="The sequence.",rate(text)! help="The rate: `25`, `23.976` or `30000/1001`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.timeline | Timeline | film | command | film | list | The timeline of the document. | - | film |  | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.timeline.closegap | Closegap | film | command | film | collapse | **Close** the gap that holds a place on a track. | - | film | track(node-ref)! help="The track the gap is on.",time(number)! help="The place in the gap, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.timeline.razor | Razor | film | command | film | razor | **Cut** the clip that is under the playhead with the **razor**. | - | film | sequence(node-ref)! help="The sequence.",track(node-ref)! help="The track the clip is on.",at(number)? help="The frame to cut at. The playhead of the sequence when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.add | Add | film | command | film | plus | Put a **track** in a sequence. | - | film | name(text)! help="What to call it.",sequence(node-ref)! help="The sequence it is in.",kind(text)! help="What kind of track it is: `video`, `audio`, `title`, …" | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.enable | Enable | film | command | film | check | **Enable** a track, or **disable** it. | - | film | track(node-ref)! help="The track.",on(bool)! help="Whether the track is enabled." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.get | Get | film | command | film | list | Read a **track**. | - | film | track(node-ref)! help="The track to read." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.track.label | Label | film | command | film | paint | **Put** a **label** on a track, or **take it off**. | - | film | track(node-ref)! help="The track.",label(text)! help="The label: `Violet`, `Iris`, … — or nothing to take it off." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.lock | Lock | film | command | film | lock | **Lock** a track, or unlock it. | - | film | track(node-ref)! help="The track.",locked(bool)! help="Whether the track is locked." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.mute | Mute | film | command | film | mute | **Mute** a track, or **unmute** it. | - | film | track(node-ref)! help="The track.",on(bool)! help="Whether the track is muted." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.pan | Pan | film | command | film | pan | **Set** the **pan** of a track. | - | film | track(node-ref)! help="The track.",pan(number)! help="The pan: between minus a hundred and a hundred." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.remove | Remove | film | command | film | minus | **Take away** a track. | - | film | track(node-ref)! help="The track." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.rename | Rename | film | command | film | pencil | **Rename** a track. | - | film | track(node-ref)! help="The track.",name(text)! help="The name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.solo | Solo | film | command | film | headphones | **Solo** a track, or **unsolo** it. | - | film | track(node-ref)! help="The track.",on(bool)! help="Whether the track is soloed." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.track.volume | Volume | film | command | film | speaker | **Set** the **volume** of a track, in decibels. | - | film | track(node-ref)! help="The track.",db(number)! help="The volume, in decibels." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.transition.add | Add | film | command | film | plus | **Add** a transition over the cut between two clips. | - | film | track(node-ref)! help="The track the transition is on.",from(node-ref)! help="The outgoing clip.",to(node-ref)! help="The incoming clip.",duration(number)! help="How long the transition is, in frames.",align(text)? help="Where it sits on the cut: `center`, `start` or `end`. The default is `center`.",effect(text)? help="The kind of transition: `cross_dissolve`. The default is that." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.transition.remove | Remove | film | command | film | minus | **Take away** a transition. | - | film | transition(node-ref)! help="The transition." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.transition.set | Set | film | command | film | pencil | **Change** a transition: its duration and its alignment. | - | film | transition(node-ref)! help="The transition.",duration(number)? help="How long it is, in frames. The default is the duration it has.",align(text)? help="Where it sits on the cut: `center`, `start` or `end`. The default is the alignment it has." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.workarea.clear | Clear | film | command | film | minus | **Take the work area away** from a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"film.workarea.get | Get | film | command | film | list | **Read** the work area of a sequence. | - | film | sequence(node-ref)! help="The sequence." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"film.workarea.set | Set | film | command | film | marker | **Set** the work area of a sequence. | - | film | sequence(node-ref)! help="The sequence.",start(number)! help="The in point, in frames.",end(number)! help="The out point, in frames." | mutates=true | undoable=true | journal=true | example=false"##,
];

/// The commands of the film battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "film").cloned().collect();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// One command, written down.
///
/// **Everything the registry carries that is data**: the id, the label, the
/// area, the capability, the layer, the icon, the documentation, the
/// chord, the menu path, the parameters — with each parameter's **help**,
/// its other spelling, its kind's own name and its unit — and the flags.
///
/// The `enabled` predicate is a **function**, and a function is not
/// something a snapshot can write down; the snapshot says everything else.
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            let mut text = format!(
                "{}({}){}",
                param.name,
                param.kind.json_name(),
                if param.required { "!" } else { "?" }
            );
            if !param.help.is_empty() {
                text.push_str(&format!(" help=\"{}\"", quoted(param.help)));
            }
            if let Some(alternative) = param.alternative {
                text.push_str(&format!(" or=\"{}\"", quoted(alternative)));
            }
            if let Some(type_name) = param.type_name {
                text.push_str(&format!(" type=\"{}\"", quoted(type_name)));
            }
            if let Some(unit) = param.unit {
                text.push_str(&format!(" unit=\"{}\"", quoted(unit)));
            }
            text
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={} | journal={} | example={}",
        command.id,
        command.label,
        command.area,
        command.capability,
        command.layer,
        command.icon,
        command.doc,
        command.shortcut.unwrap_or("-"),
        command.menu.join("/"),
        params.join(","),
        command.mutates,
        command.undoable,
        command.journal,
        command.example
    )
}

/// A text, with its backslashes and its quotes escaped.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r##\"{line}\"##,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
