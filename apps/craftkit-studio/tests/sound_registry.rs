//! The **sound battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §868 converted this battery from one declaration per command to
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

#![cfg(feature = "sound")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the sound battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r#"sound.bus.add | Add | sound | command | sound | plus | Make a bus. | - | Track/New | name(text)!,channels(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.add | Add | sound | command | sound | plus | Put a clip on a track. | - | Track/New | track(node-ref)!,playlist(text)?,start(number)!,length(number)!,stretch(number)?,source(node-ref)?,name(text)? | mutates=true | undoable=true"#,
    r#"sound.clip.autoname | Autoname | sound | command | sound | sound | **Name the clips that editing separates.** | - | Clip/Auto Rename | clips(json)!,name(text)?,number(number)?,zeros(number)?,suffix(text)? | mutates=true | undoable=true"#,
    r#"sound.clip.autoname.read | Read | sound | command | sound | list | **Read the scheme** a clip is in. | - | sound | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"sound.clip.conform | Conform | sound | command | sound | sound | **Conform** a clip to the tempo of the document. | - | Clip/Conform to Tempo | clip(node-ref)!,source_bpm(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.fade | Fade | sound | command | sound | music | Fade a clip. | - | Clip/Fade | clip(node-ref)!,fade_in(number)?,fade_out(number)?,shape(text)?,curvature(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.fade.to | To | sound | command | sound | music | **Fade** a clip **to a place**: up to it, or down to it. | - | Clip/Fade | clip(node-ref)!,at(number)!,to(text)? | mutates=true | undoable=true"#,
    r#"sound.clip.gain | Gain | sound | command | sound | sound | **Set the gain** of clips. | - | Clip/Gain | clips(json)!,db(number)?,delta_db(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.gain.bypass | Bypass | sound | command | sound | sound | **Bypass the gain** of clips. | - | Clip/Bypass Gain | clips(json)!,bypassed(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.group | Group | sound | command | sound | sound | **Put clips in a group.** | - | Clip/Group | name(text)!,clips(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.group.read | Read | sound | command | sound | list | **Read the group** a clip is in. | - | sound | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"sound.clip.heal | Heal | sound | command | sound | trim | **Heal** a track: the clips that were separated are joined. | - | Clip/Heal | track(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.clip.lock.edit | Edit | sound | command | sound | lock | Lock a clip's **content**, or take the lock off. | - | Clip/Edit Lock | clip(node-ref)!,locked(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.lock.time | Time | sound | command | sound | lock | Lock a clip's **place**, or take the lock off. | - | Clip/Time Lock | clip(node-ref)!,locked(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.loop | Loop | sound | command | sound | link | Loop a clip: the copies of it that follow it. | - | Clip/Loop | clip(node-ref)!,count(number)?,length(number)?,fill(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.loudness.read | Read | sound | command | sound | list | The **loudness** of a clip. | - | sound | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"sound.clip.midi.add | Add | sound | command | sound | music | **Make a MIDI clip** with no notes in it. | - | Clip/New MIDI Clip | track(node-ref)!,start(number)!,length(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.ctrl.add | Add | sound | command | sound | music | **Add a controller** to a MIDI clip. | - | Clip/Add Controller | clip(node-ref)!,kind(text)!,at(number)!,value(number)!,number(number)?,channel(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.ctrl.edit | Edit | sound | command | sound | music | **Edit a controller** of a MIDI clip. | - | Clip/Edit Controller | clip(node-ref)!,index(number)!,kind(text)?,at(number)?,value(number)?,number(number)?,channel(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.ctrl.remove | Remove | sound | command | sound | music | **Take controllers** off a MIDI clip. | - | Clip/Remove Controller | clip(node-ref)!,indices(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.duration | Duration | sound | command | sound | sound | **Change the length** of the notes of a clip. | - | Clip/Duration | clip(node-ref)!,set(number)?,delta(number)?,scale(number)?,gap(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.export | Export | sound | command | sound | music | **Export the MIDI** of a clip to a file. | - | Clip/Export MIDI | clip(node-ref)!,path(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.import | Import | sound | command | sound | music | **Import a MIDI file** as a clip. | - | Clip/Import MIDI | track(node-ref)!,start(number)!,path(text)!,tempo_map(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.names | Names | sound | command | sound | list | **Name the notes** of a MIDI clip. | - | sound | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"sound.clip.midi.note.add | Add | sound | command | sound | music | **Add a note** to a MIDI clip. | - | Clip/Add Note | clip(node-ref)!,pitch(number)!,start(number)!,length(number)?,velocity(number)?,channel(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.note.edit | Edit | sound | command | sound | music | **Edit a note** of a MIDI clip. | - | Clip/Edit Note | clip(node-ref)!,index(number)!,pitch(number)?,start(number)?,length(number)?,velocity(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.note.remove | Remove | sound | command | sound | music | **Remove notes** from a MIDI clip. | - | Clip/Remove Note | clip(node-ref)!,indices(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.notes.dedupe | Dedupe | sound | command | sound | music | **Take the duplicate notes** out of a MIDI clip. | - | Clip/Remove Duplicate Notes | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.notes.move | Move | sound | command | sound | music | **Move notes** of a MIDI clip. | - | Clip/Move Notes | clip(node-ref)!,indices(json)!,ticks(number)?,semitones(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.midi.quantize | Quantize | sound | command | sound | sound | **Quantise the notes** of a clip onto the session's **grid**. | - | Clip/Quantize MIDI | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.read | Read | sound | command | sound | list | **Read the MIDI** of a clip. | - | sound | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"sound.clip.midi.transpose | Transpose | sound | command | sound | sound | **Transpose** the notes of a clip. | - | Clip/Transpose MIDI | clip(node-ref)!,semitones(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.midi.velocity | Velocity | sound | command | sound | sound | **Change the velocity** of the notes of a clip. | - | Clip/Velocity | clip(node-ref)!,set(number)?,delta(number)?,scale(number)?,min(number)?,max(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.move | Move | sound | command | sound | move | **Move** clips along the timeline. | - | Clip/Move | clips(json)!,delta(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.mute | Mute | sound | command | sound | sound | **Mute** a clip, or unmute it. | - | Track/Mute | clip(node-ref)!,muted(bool)? | mutates=true | undoable=true"#,
    r#"sound.clip.order | Order | sound | command | sound | list | Put a clip **in front** of the others on its track, or **behind** them. | - | Clip/Order | clip(node-ref)!,place(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.pitch | Pitch | sound | command | sound | music | Move the **pitch** of a clip. | - | Clip/Pitch | clip(node-ref)!,semitones(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.pitch.read | Read | sound | command | sound | list | The **pitch** of the notes of a clip. | - | sound | clip(node-ref)!,min_hz(number)?,max_hz(number)? | mutates=false | undoable=false"#,
    r#"sound.clip.process | Process | sound | command | sound | music | Do something to a clip's **sound**. | - | Clip/Process | clip(node-ref)!,process(text)!,params(json)? | mutates=true | undoable=true"#,
    r#"sound.clip.process.remove | Remove | sound | command | sound | music | Take a process off a clip. | - | Clip/Unprocess | clip(node-ref)!,process(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.quantize | Quantize | sound | command | sound | sound | **Put clips on the grid**. | - | Clip/Quantize to Grid | clips(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.rating | Rating | sound | command | sound | sound | Set the **rating** of a clip. | - | Clip/Rating | clip(node-ref)!,rating(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.remove | Remove | sound | command | sound | minus | **Take away** a clip. | - | Clip/Remove | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.clip.rename | Rename | sound | command | sound | pencil | **Rename** a clip. | - | Track/Rename Clip | clip(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.render | Render | sound | command | sound | sound | **Render** a clip: the sound of it, written to a file. | - | Track/Render Clip | clip(node-ref)!,directory(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.separate | Separate | sound | command | sound | trim | Separate a clip into two. | - | Clip/Separate | clip(node-ref)!,at(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.separate.grid | Grid | sound | command | sound | trim | Separate a clip on the **grid**. | - | Clip/Separate on Grid | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.clip.separate.transients | Transients | sound | command | sound | trim | Separate a clip at its **transients**. | - | Clip/Separate on Transients | clip(node-ref)!,sensitivity(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.snap | Snap | sound | command | sound | trim | Snap a clip to its **neighbour**. | - | Clip/Snap to | clips(json)!,direction(text)! | mutates=true | undoable=true"#,
    r#"sound.clip.space | Space | sound | command | sound | trim | Space the **clips** against each other. | - | Clip | clips(json)!,gap(number)!,mode(text)? | mutates=true | undoable=true"#,
    r#"sound.clip.stretch | Stretch | sound | command | sound | music | Stretch a clip. | - | Clip/Stretch | clip(node-ref)!,stretch(number)! | mutates=true | undoable=true"#,
    r#"sound.clip.strip.silence | Silence | sound | command | sound | trim | **Strip the silence** out of a clip. | - | Clip/Strip Silence | clip(node-ref)!,threshold_db(number)?,min_length_ms(number)?,pad_before_ms(number)?,pad_after_ms(number)?,mode(text)? | mutates=true | undoable=true"#,
    r#"sound.clip.strip.silence.read | Read | sound | command | sound | list | **Read** the **loud parts** of a clip: where the silence is. | - | sound | clip(node-ref)!,threshold_db(number)?,min_length_ms(number)?,pad_before_ms(number)?,pad_after_ms(number)? | mutates=false | undoable=false"#,
    r#"sound.clip.sync.read | Read | sound | command | sound | list | Read the **beat** of a clip. | - | sound | clip(node-ref)? | mutates=false | undoable=false"#,
    r#"sound.clip.sync.remove | Remove | sound | command | sound | sound | Take the **beat** out of a clip. | - | Clip/No Sync Point | clip(node-ref)? | mutates=true | undoable=true"#,
    r#"sound.clip.sync.set | Set | sound | command | sound | sound | Set the **beat** of a clip. | - | Clip/Sync Point | clip(node-ref)?,at(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.trim | Trim | sound | command | sound | trim | Trim a clip. | - | Clip/Trim | clip(node-ref)?,from(number)?,to(number)? | mutates=true | undoable=true"#,
    r#"sound.clip.trim.to.file | File | sound | command | sound | trim | **Trim** clips to the **material** they play. | - | Clip/Trim to Material | clips(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.ungroup | Ungroup | sound | command | sound | sound | **Take clips out of their groups.** | - | Clip/Ungroup | clips(json)! | mutates=true | undoable=true"#,
    r#"sound.clip.unloop | Unloop | sound | command | sound | music | Unloop a clip: take the copies away. | - | Clip/Unloop | clip(node-ref)!,mode(text)? | mutates=true | undoable=true"#,
    r#"sound.grid.read | Read | sound | command | sound | list | **Read the grid** of the session. | - | sound | at(number)? | mutates=false | undoable=false"#,
    r#"sound.grid.set | Set | sound | command | sound | sound | **Set the grid** of the session. | - | Grid/Set Grid | value(text)!,dotted(bool)?,triplet(bool)? | mutates=true | undoable=true"#,
    r#"sound.insert.add | Add | sound | command | sound | plus | Put a plugin in a slot on a track. | - | Track/Inserts | track(node-ref)?,clip(node-ref)?,plugin(text)!,bypass(bool)? | mutates=true | undoable=true"#,
    r#"sound.insert.move | Move | sound | command | sound | move | **Move** an insert: it and another one swap their places in the chain. | - | Track/Inserts | insert(node-ref)!,other(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.insert.remove | Remove | sound | command | sound | minus | **Take away** an insert: the plugin is not on the thing any more. | - | Track/Inserts | insert(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.insert.set | Set | sound | command | sound | pencil | Change an **insert**: its parameters, or whether it is bypassed. | - | Track/Inserts | insert(node-ref)!,parameters(json)?,bypass(bool)? | mutates=true | undoable=true"#,
    r#"sound.key.read | Read | sound | command | sound | list | Read the **keys** of the session. | - | sound | tick(number)? | mutates=false | undoable=false"#,
    r#"sound.key.remove | Remove | sound | command | sound | minus | **Take away** the key at a tick. | - | Event/Key | tick(number)! | mutates=true | undoable=true"#,
    r#"sound.key.set | Set | sound | command | sound | music | Set the **key** of the session at a place. | - | Event/Key | key(text)!,tick(number)? | mutates=true | undoable=true"#,
    r#"sound.lane.add | Add | sound | command | sound | plus | Automate a parameter of a track. | - | Track/Automation | track(node-ref)?,clip(node-ref)?,param(text)!,points(json)? | mutates=true | undoable=true"#,
    r#"sound.lane.clear | Clear | sound | command | sound | minus | **Clear** the breakpoints of a lane. | - | Track/Automation | lane(node-ref)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"sound.lane.point.remove | Remove | sound | command | sound | minus | **Take a breakpoint** off a lane. | - | Track/Automation | lane(node-ref)!,at(number)! | mutates=true | undoable=true"#,
    r#"sound.lane.point.set | Set | sound | command | sound | plus | **Set a breakpoint** on a lane. | - | Track/Automation | lane(node-ref)!,at(number)!,value(number)! | mutates=true | undoable=true"#,
    r#"sound.lane.thin | Thin | sound | command | sound | minus | **Thin** a lane: take away the breakpoints the curve does not need. | - | Track/Automation | lane(node-ref)!,tolerance(number)? | mutates=true | undoable=true"#,
    r#"sound.lane.thin.all | All | sound | command | sound | minus | **Thin** every lane of a track: take away the breakpoints the curves do not need. | - | Track/Automation | track(node-ref)!,tolerance(number)? | mutates=true | undoable=true"#,
    r#"sound.lane.write | Write | sound | command | sound | pencil | **Write** a value over a range of a lane. | - | Track/Automation | lane(node-ref)!,start(number)!,end(number)!,value(number)! | mutates=true | undoable=true"#,
    r#"sound.lane.write.all | All | sound | command | sound | pencil | **Write** the static value of every lane of a track over a range. | - | Track/Automation | track(node-ref)!,start(number)!,end(number)! | mutates=true | undoable=true"#,
    r#"sound.marker.add | Add | sound | command | sound | plus | Put a marker in the document. | - | Marker | name(text)!,start(number)!,end(number)? | mutates=true | undoable=true"#,
    r#"sound.marker.next | Next | sound | command | sound | play | **Step** to the next marker. | - | Marker |  | mutates=true | undoable=true"#,
    r#"sound.marker.previous | Previous | sound | command | sound | play | **Step** to the previous marker. | - | Marker |  | mutates=true | undoable=true"#,
    r#"sound.marker.recall | Recall | sound | command | sound | play | **Recall** a marker: the playhead goes to it. | - | Marker | marker(node-ref)? | mutates=true | undoable=true"#,
    r#"sound.marker.remove | Remove | sound | command | sound | minus | **Take away** a marker. | - | Marker/Remove | marker(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.marker.set | Set | sound | command | sound | pencil | **Move** a marker, or **rename** it. | - | Marker | marker(node-ref)!,name(text)?,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"sound.meter.remove | Remove | sound | command | sound | minus | **Take away** the meter at a tick. | - | Event/Meter | tick(number)! | mutates=true | undoable=true"#,
    r#"sound.meter.set | Set | sound | command | sound | music | Set the meter. | - | Event/Meter | numerator(number)!,denominator(number)!,tick(number)? | mutates=true | undoable=true"#,
    r#"sound.playhead.set | Set | sound | command | sound | play | Put the **playhead** where it is, or move it. | - | Playhead | position(number)! | mutates=true | undoable=true"#,
    r#"sound.playlist.active | Active | sound | command | sound | check | Say which **take** of a track is played. | - | Track/Takes | track(node-ref)!,playlist(text)! | mutates=true | undoable=true"#,
    r#"sound.playlist.add | Add | sound | command | sound | plus | Add a **take** to a track. | - | Track/Takes | track(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"sound.render | Render | sound | command | sound | play | Render the document into a **sound file**. | - | Render | path(text)! | mutates=false | undoable=false"#,
    r#"sound.render.stems | Stems | sound | command | sound | play | Render the **stems**: one file for each track. | - | Render | directory(text)! | mutates=false | undoable=false"#,
    r#"sound.route.add | Add | sound | command | sound | link | Send a track to a bus. | - | Track/Route | track(node-ref)!,bus(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.route.remove | Remove | sound | command | sound | link | **Take away** a route: the track stops feeding the bus. | - | Track/Route | track(node-ref)!,bus(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.ruler.add | Add | sound | command | sound | plus | Put a **ruler** in the document: the row above the tracks, and the scale it counts by. | - | Ruler | name(text)!,scale(text)? | mutates=true | undoable=true"#,
    r#"sound.send.add | Add | sound | command | sound | plus | Send a track to a bus at a level. | - | Track/Sends | track(node-ref)!,bus(node-ref)!,level_db(number)?,pan(number)?,mute(bool)?,pre_fader(bool)? | mutates=true | undoable=true"#,
    r#"sound.send.remove | Remove | sound | command | sound | minus | **Take away** a send. | - | Track/Send | send(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.send.set | Set | sound | command | sound | sliders | **Change** a send: its level, its pan, whether it is silent, and whether it is taken before the track's fader. | - | Track/Send | send(node-ref)!,level_db(number)?,pan(number)?,mute(bool)?,pre_fader(bool)? | mutates=true | undoable=true"#,
    r#"sound.session.set | Set | sound | command | sound | music | Set the session: the sample rate and the timecode. | - | Options | sample_rate(number)?,timecode_start(number)?,metronome(bool)?,click_volume_db(number)?,click_note(number)?,click_accent_note(number)?,layout(text)?,destructive_record(bool)?,track_punch(bool)? | mutates=true | undoable=true"#,
    r#"sound.source.add | Add | sound | command | sound | plus | Add a source: a reference to media. | - | File/Import | path(text)!,channels(number)?,frames(number)? | mutates=true | undoable=true"#,
    r#"sound.source.peaks | Peaks | sound | command | sound | list | **Measure** the peaks of a source. | - | sound | source(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.source.remove | Remove | sound | command | sound | minus | **Take away** a source. | - | File/Remove Source | source(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.tempo.constant | Constant | sound | command | sound | music | Make a **range** of the tempo map constant. | - | Event/Tempo | bpm(number)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"sound.tempo.remove | Remove | sound | command | sound | minus | **Take away** the tempo at a tick. | - | Event/Tempo | tick(number)! | mutates=true | undoable=true"#,
    r#"sound.tempo.scale | Scale | sound | command | sound | music | **Scale** the tempos of the document. | - | Event/Tempo | factor(number)! | mutates=true | undoable=true"#,
    r#"sound.tempo.set | Set | sound | command | sound | music | Set the tempo. | - | Event/Tempo | bpm(number)!,tick(number)? | mutates=true | undoable=true"#,
    r#"sound.time.cut | Cut | sound | command | sound | trim | **Cut time** out of the session: a range of the timeline is thrown away and everything after it moves earlier. | - | Event/Time | start(number)!,end(number)! | mutates=true | undoable=true"#,
    r#"sound.time.insert | Insert | sound | command | sound | move | **Insert time** into the session: silence at a place, and everything after it moves later. | - | Event/Time | at(number)!,length(number)! | mutates=true | undoable=true"#,
    r#"sound.track.add | Add | sound | command | sound | plus | Make a track. | - | Track/New | name(text)!,track_kind(text)?,channels(number)? | mutates=true | undoable=true"#,
    r#"sound.track.clipboard.read | Read | sound | command | sound | list | The **clipboard** of the document, read: how many clips it holds and how long the range was. | - | sound |  | mutates=false | undoable=false"#,
    r#"sound.track.commit | Commit | sound | command | sound | sound | **Commit** a track: write its sound to a file and make a new track of it. | - | Track/Commit | track(node-ref)!,directory(text)! | mutates=true | undoable=true"#,
    r#"sound.track.consolidate | Consolidate | sound | command | sound | play | **Consolidate** a track: the clips of a track become one clip of a file that holds their sound. | - | Track/Consolidate | track(node-ref)!,start(number)?,end(number)?,directory(text)! | mutates=true | undoable=true"#,
    r#"sound.track.copy | Copy | sound | command | sound | copy | **Copy** the material of a range of a track. | - | Edit/Copy | track(node-ref)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"sound.track.cut | Cut | sound | command | sound | cut | **Cut** the material of a range of a track. | - | Edit/Cut | track(node-ref)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"sound.track.duplicate | Duplicate | sound | command | sound | copy | **Duplicate** a track: a copy of it, next to it. | - | Track/Duplicate | track(node-ref)!,playlists(bool)?,automation(bool)?,inserts(bool)?,sends(bool)? | mutates=true | undoable=true"#,
    r#"sound.track.elastic | Elastic | sound | command | sound | music | Set a track's **elastic algorithm**. | - | Track/Elastic | track(node-ref)!,algorithm(text)! | mutates=true | undoable=true"#,
    r#"sound.track.fade | Fade | sound | command | sound | music | The **fades** of a range of a track. | - | Track/Fade | track(node-ref)!,from(number)!,to(number)!,shape(text)? | mutates=true | undoable=true"#,
    r#"sound.track.fade.delete | Delete | sound | command | sound | minus | **Take away** the fades of a range of a track. | - | Track/Fade | track(node-ref)!,from(number)!,to(number)! | mutates=true | undoable=true"#,
    r#"sound.track.fader | Fader | sound | command | sound | sliders | Set a track's **fader**. | - | Track/Fader | track(node-ref)!,db(number)! | mutates=true | undoable=true"#,
    r#"sound.track.freeze | Freeze | sound | command | sound | lock | **Freeze** a track, or let it be. | - | Track/Freeze | track(node-ref)!,frozen(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.group | Group | sound | command | sound | group | **Put tracks in a group**. | - | Track/Group | name(text)!,tracks(json)! | mutates=true | undoable=true"#,
    r#"sound.track.group.set | Set | sound | command | sound | group | **Set the flags** of a group of tracks. | - | Track/Group Settings | group(node-ref)!,mix(bool)?,active(bool)? | mutates=true | undoable=true"#,
    r#"sound.track.inactive | Inactive | sound | command | sound | eye | **Make** a track inactive, or make it play again. | - | Track/Make Inactive | track(node-ref)!,inactive(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.input | Input | sound | command | sound | link | Make a track **read** a bus. | - | Track/Input | track(node-ref)!,bus(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.input.remove | Remove | sound | command | sound | link | **Take away** the reading of a bus: the track stops reading it. | - | Track/Input | track(node-ref)!,bus(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.midi.import | Import | sound | command | sound | music | **Import a MIDI file as tracks**: a track for every track of the file. | - | File/Import MIDI as Tracks | path(text)!,start(number)!,tempo_map(bool)? | mutates=true | undoable=true"#,
    r#"sound.track.monitor | Monitor | sound | command | sound | record | Let a track **hear its input**, or not. | - | Track/Monitor | track(node-ref)!,monitoring(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.mute | Mute | sound | command | sound | music | Mute a track, or let it be heard. | - | Track/Mute | track(node-ref)!,muted(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.mute.clear | Clear | sound | command | sound | music | The **mutes** of every track cleared. | - | Track/Mute |  | mutates=true | undoable=true"#,
    r#"sound.track.pan | Pan | sound | command | sound | sliders | Set a track's **pan**. | - | Track/Pan | track(node-ref)!,pan(number)! | mutates=true | undoable=true"#,
    r#"sound.track.paste | Paste | sound | command | sound | paste | **Paste** the clipboard onto a track. | - | Edit/Paste | track(node-ref)!,at(number)? | mutates=true | undoable=true"#,
    r#"sound.track.record | Record | sound | command | sound | record | Arm a track for **recording**, or let it be. | - | Track/Record | track(node-ref)!,armed(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.remove | Remove | sound | command | sound | minus | **Take away** a track. | - | Track/Remove | track(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.rename | Rename | sound | command | sound | pencil | **Rename** a track. | - | Track/Rename | track(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"sound.track.repeat | Repeat | sound | command | sound | loop | **Repeat** the material of a range of a track. | - | Track/Repeat | track(node-ref)!,start(number)?,end(number)?,count(number)? | mutates=true | undoable=true"#,
    r#"sound.track.sends.mute | Mute | sound | command | sound | music | **Mute** the sends of a track, or let them be heard. | - | Track/Sends | track(node-ref)!,muted(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.set | Set | sound | command | sound | sliders | Set a track's **trim** and **phase**. | - | Track/Trim | track(node-ref)!,trim_db(number)?,phase(bool)? | mutates=true | undoable=true"#,
    r#"sound.track.solo | Solo | sound | command | sound | music | **Solo** a track. | - | Track/Solo | track(node-ref)!,solo(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.solo.clear | Clear | sound | command | sound | music | The **solos** of every track cleared. | - | Track/Solo |  | mutates=true | undoable=true"#,
    r#"sound.track.solo.safe | Safe | sound | command | sound | music | Make a track **solo safe**, or not. | - | Track/Solo Safe | track(node-ref)!,solo_safe(bool)! | mutates=true | undoable=true"#,
    r#"sound.track.split.mono | Mono | sound | command | sound | split | **Split** a track into **mono** tracks. | - | Track/Split into Mono | track(node-ref)!,directory(text)! | mutates=true | undoable=true"#,
    r#"sound.track.surround | Surround | sound | command | sound | sliders | lie about where the sound is. | - | Track/Surround | track(node-ref)!,x(number)?,y(number)?,z(number)?,divergence(number)?,center(number)? | mutates=true | undoable=true"#,
    r#"sound.track.surround.delete | Delete | sound | command | sound | minus | Take a track's **surround pan** away. | - | Track/Surround | track(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.ungroup | Ungroup | sound | command | sound | group | **Take a group** of tracks away. | - | Track/Ungroup | group(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.vca | Vca | sound | command | sound | sliders | Put a track under a **VCA master**, or take it out again. | - | Track/VCA | track(node-ref)!,vca(node-ref)! | mutates=true | undoable=true"#,
    r#"sound.track.vca.delete | Delete | sound | command | sound | minus | Take a track out of its **VCA master**. | - | Track/VCA | track(node-ref)! | mutates=true | undoable=true"#,
];

/// The commands of the sound battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "sound").cloned().collect();
    commands.sort_by(|left, right| left.id.cmp(right.id));
    commands
}

/// One command, written down.
fn written(command: &CommandSpec) -> String {
    let params: Vec<String> = command
        .params
        .iter()
        .map(|param| {
            format!("{}({}){}", param.name, param.kind.json_name(), if param.required { "!" } else { "?" })
        })
        .collect();
    format!(
        "{} | {} | {} | {} | {} | {} | {} | {} | {} | {} | mutates={} | undoable={}",
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
        command.undoable
    )
}

/// The registry says what the snapshot says.
#[test]
fn the_registry_is_the_snapshot() {
    let commands = the_battery();
    assert!(!commands.is_empty(), "the battery has no commands");
    let found: Vec<String> = commands.iter().map(written).collect();
    if std::env::var("CRAFTKIT_DUMP").is_ok() {
        for line in &found {
            println!("    r#\"{line}\"#,");
        }
        return;
    }
    assert_eq!(found, EXPECTED, "the registry is not what it was");
}
