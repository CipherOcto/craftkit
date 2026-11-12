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
    r#"film.bin.add | Add | film | command | film | plus | Put a **bin** in the project. | - | film | name(text)!,parent(node-ref)? | mutates=true | undoable=true"#,
    r#"film.bin.get | Get | film | command | film | list | Read a **bin**. | - | film | bin(node-ref)! | mutates=false | undoable=false"#,
    r#"film.bin.remove | Remove | film | command | film | minus | **Take away** a bin. | - | film | bin(node-ref)! | mutates=true | undoable=true"#,
    r#"film.bin.rename | Rename | film | command | film | pencil | **Name** a bin. | - | film | bin(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"film.caption.add | Add | film | command | film | plus | Add a **cue** to a caption track. | - | film | track(node-ref)!,start(number)!,end(number)!,text(text)! | mutates=true | undoable=true"#,
    r#"film.caption.burn | Burn | film | command | film | image | **Burn** a caption into a frame: the caption is drawn, and the picture of it is written. | - | film | text(text)!,track(node-ref)?,width(number)!,height(number)!,settings(text)?,to(text)! | mutates=false | undoable=false"#,
    r#"film.caption.duplicate | Duplicate | film | command | film | copy | **Duplicate** a cue: the copy stands by the delta that is given. | - | film | caption(node-ref)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.caption.import | Import | film | command | film | plus | **Import** a caption file as a new caption track of a sequence. | - | film | sequence(node-ref)!,path(text)!,format(text)?,name(text)? | mutates=true | undoable=true"#,
    r#"film.caption.layout | Layout | film | command | film | list | **Lay** a **caption** out on a frame of a given size. | - | film | text(text)!,track(node-ref)?,width(number)!,height(number)!,settings(text)? | mutates=false | undoable=false"#,
    r#"film.caption.merge | Merge | film | command | film | merge | **Merge** the cues that are next to each other into one. | - | film | captions(json)? | mutates=true | undoable=true"#,
    r#"film.caption.move | Move | film | command | film | arrow | **Move** a cue: it stays at or after the start of the sequence and it does not run into its neighbours. | - | film | caption(node-ref)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.caption.read | Read | film | command | film | list | The **cues** of a caption track, in the order they start in. | - | film | track(node-ref)! | mutates=false | undoable=false"#,
    r#"film.caption.remove | Remove | film | command | film | minus | Take a **cue** away from a caption track. | - | film | caption(node-ref)! | mutates=true | undoable=true"#,
    r#"film.caption.set | Set | film | command | film | pencil | **Set** the text of a cue, and the **speaker** of it. | - | film | caption(node-ref)!,text(text)?,speaker(text)? | mutates=true | undoable=true"#,
    r#"film.caption.split | Split | film | command | film | split | **Split** a cue in two at a frame. | - | film | caption(node-ref)!,at(number)! | mutates=true | undoable=true"#,
    r#"film.caption.times | Times | film | command | film | clock | **Set** the times of a cue. | - | film | caption(node-ref)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"film.caption.track.add | Add | film | command | film | plus | Add a **caption track** to a sequence. | - | film | name(text)!,sequence(node-ref)!,format(text)? | mutates=true | undoable=true"#,
    r#"film.caption.track.delete | Delete | film | command | film | minus | **Delete** a caption track, and its cues go with it. | - | film | track(node-ref)! | mutates=true | undoable=true"#,
    r#"film.caption.track.set | Set | film | command | film | pencil | **Set** the track of the captions: its name, its format, its language and its flags. | - | film | track(node-ref)!,name(text)?,format(text)?,language(text)?,enabled(bool)?,locked(bool)?,sync_lock(bool)? | mutates=true | undoable=true"#,
    r#"film.caption.track.style | Style | film | command | film | text | **Set** the style of a caption track. | - | film | track(node-ref)!,font(text)?,size(number)?,colour(text)?,background(bool)?,background_colour(text)?,align(text)?,anchor(text)?,margin(number)?,line_spacing(number)?,outline(number)?,outline_colour(text)?,reset(bool)? | mutates=true | undoable=true"#,
    r#"film.caption.trim | Trim | film | command | film | trim | **Trim** a cue at its **in** edge or at its **out** edge. | - | film | caption(node-ref)!,edge(text)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.captions.detect | Detect | film | command | film | search | **Detect** the format of a caption file. | - | film | path(text)! | mutates=false | undoable=false"#,
    r#"film.captions.export | Export | film | command | film | plus | **Export** the cues of a caption track as a caption file. | - | film | track(node-ref)!,path(text)!,format(text)?,cea_608(bool)?,cea_708(bool)? | mutates=true | undoable=true"#,
    r#"film.captions.read | Read | film | command | film | list | **Read** the **cues** of a caption file. | - | film | path(text)!,format(text)? | mutates=false | undoable=false"#,
    r#"film.captions.write | Write | film | command | film | plus | **Write** a caption file. | - | film | path(text)!,format(text)?,cues(json)!,blocks(json)?,drop_frame(bool)?,cea_608(bool)?,cea_708(bool)? | mutates=true | undoable=true"#,
    r#"film.clip.add | Add | film | command | film | plus | Put a **clip** on a track. | - | film | track(node-ref)!,item(node-ref)!,start(number)!,duration(number)!,source_in(number)? | mutates=true | undoable=true"#,
    r#"film.clip.enable | Enable | film | command | film | power | **Enable** a clip, or disable it. | - | film | clip(node-ref)!,on(bool)? | mutates=true | undoable=true"#,
    r#"film.clip.extract | Extract | film | command | film | minus | **Extract** a range of a track: take the clips that are in it away and close the gap. | - | film | track(node-ref)!,from(number)!,to(number)! | mutates=true | undoable=true"#,
    r#"film.clip.get | Get | film | command | film | list | Read a **clip**. | - | film | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"film.clip.hold | Hold | film | command | film | pin | **Hold** a frame of a clip. | - | film | clip(node-ref)!,at(number)! | mutates=true | undoable=true"#,
    r#"film.clip.hold.delete | Delete | film | command | film | unpin | **Take away** the frame hold of a clip. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.insert | Insert | film | command | film | arrow | **Insert** a clip at a place on a track. | - | film | clip(node-ref)!,track(node-ref)!,start(number)! | mutates=true | undoable=true"#,
    r#"film.clip.label | Label | film | command | film | paint | Put a **label** on a clip. | - | film | clip(node-ref)!,label(text)! | mutates=true | undoable=true"#,
    r#"film.clip.label.delete | Delete | film | command | film | minus | Take the **label** off a clip. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.label.group | Group | film | command | film | list | The **group** of a clip: every clip that carries its label. | - | film | clip(node-ref)! | mutates=false | undoable=false"#,
    r#"film.clip.lift | Lift | film | command | film | minus | **Lift** a range of a track: take the clips that are in it away. | - | film | track(node-ref)!,from(number)!,to(number)! | mutates=true | undoable=true"#,
    r#"film.clip.link | Link | film | command | film | link | **Link** two clips together. | - | film | clip(node-ref)!,with(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.move | Move | film | command | film | arrow | **Move** a clip to a place on a track. | - | film | clip(node-ref)!,track(node-ref)!,start(number)! | mutates=true | undoable=true"#,
    r#"film.clip.nest | Nest | film | command | film | group | **Nest** the clips of a sequence into a clip of their own. | - | film | clips(json)!,name(text)! | mutates=true | undoable=true"#,
    r#"film.clip.remove | Remove | film | command | film | minus | **Take away** a clip. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.rename | Rename | film | command | film | pencil | **Name** a clip. | - | film | clip(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"film.clip.replace | Replace | film | command | film | swap | **Replace** the media a clip plays. | - | film | clip(node-ref)!,item(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.reverse | Reverse | film | command | film | repeat | **Reverse** a clip: play its media backwards. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.ripple | Ripple | film | command | film | minus | **Take away** a clip and close the gap. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.roll | Roll | film | command | film | split | **Roll** the cut between two clips. | - | film | left(node-ref)!,right(node-ref)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.clip.slide | Slide | film | command | film | slide | **Slide** a clip between its neighbours. | - | film | clip(node-ref)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.clip.slip | Slip | film | command | film | move | **Slip** a clip: show a different part of its media. | - | film | clip(node-ref)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.clip.speed | Speed | film | command | film | repeat | **Set** the speed of a clip. | - | film | clip(node-ref)!,speed(number)!,ripple(bool)? | mutates=true | undoable=true"#,
    r#"film.clip.split | Split | film | command | film | scissors | **Split** a clip at a place. | - | film | clip(node-ref)!,at(number)! | mutates=true | undoable=true"#,
    r#"film.clip.stretch | Stretch | film | command | film | expand | **Stretch** one edge of a clip. | - | film | clip(node-ref)!,edge(text)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.clip.synchronise | Synchronise | film | command | film | link | **Synchronise** a clip with another one. | - | film | clip(node-ref)!,reference(node-ref)!,method(text)? | mutates=true | undoable=true"#,
    r#"film.clip.trim | Trim | film | command | film | scissors | **Trim** one edge of a clip. | - | film | clip(node-ref)!,edge(text)!,delta(number)! | mutates=true | undoable=true"#,
    r#"film.clip.ungroup | Ungroup | film | command | film | ungroup | **Take a nested clip apart** and put its clips back where they were. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.clip.unlink | Unlink | film | command | film | unlink | **Unlink** a clip from its group. | - | film | clip(node-ref)! | mutates=true | undoable=true"#,
    r#"film.export.ale | Ale | film | command | film | list | **Write** the Avid log exchange of a sequence. | - | film | sequence(node-ref)! | mutates=false | undoable=false"#,
    r#"film.export.edl | Edl | film | command | film | list | **Write** the edit decision list of a sequence. | - | film | sequence(node-ref)!,title(text)? | mutates=false | undoable=false"#,
    r#"film.export.fcpxml | Fcpxml | film | command | film | list | The **text** of the FCPXML of a sequence. | - | film | sequence(node-ref)!,width(number)!,height(number)! | mutates=false | undoable=false"#,
    r#"film.export.otio | Otio | film | command | film | list | The **text** of the OTIO of a sequence. | - | film | sequence(node-ref)! | mutates=false | undoable=false"#,
    r#"film.import.ale | Ale | film | command | film | plus | **Read** an Avid log exchange and make the **sequence** it names. | - | film | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"film.import.edl | Edl | film | command | film | plus | **Read** an edit decision list and make the **sequence** it names. | - | film | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"film.import.fcp7 | Fcp7 | film | command | film | plus | **Read** an FCP7 document and make the **sequence** it names. | - | film | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"film.import.fcpxml | Fcpxml | film | command | film | plus | **Read** an FCPXML file and make the **sequence** it names. | - | film | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"film.import.otio | Otio | film | command | film | plus | **Read** an OTIO file and make the **sequence** it names. | - | film | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"film.item.add | Add | film | command | film | plus | Put an **item** in a bin. | - | film | name(text)!,bin(node-ref)!,item(number)!,path(text)?,duration(number)?,offline(bool)? | mutates=true | undoable=true"#,
    r#"film.item.generator | Generator | film | command | film | plus | Put a **generated** item in a bin. | - | film | name(text)!,bin(node-ref)!,item(number)!,kind(text)!,seconds(number)?,rate(text)?,colour(text)? | mutates=true | undoable=true"#,
    r#"film.item.get | Get | film | command | film | list | Read a **media item**. | - | film | item(node-ref)! | mutates=false | undoable=false"#,
    r#"film.item.label | Label | film | command | film | paint | Put a **label** on a media item. | - | film | item(node-ref)!,label(text)! | mutates=true | undoable=true"#,
    r#"film.item.label.delete | Delete | film | command | film | minus | Take the **label** off a media item. | - | film | item(node-ref)! | mutates=true | undoable=true"#,
    r#"film.item.label.group | Group | film | command | film | list | The **group** of an item: every item that carries its label. | - | film | item(node-ref)! | mutates=false | undoable=false"#,
    r#"film.item.mark | Mark | film | command | film | flag | Put the **marks** on a media. | - | film | item(node-ref)!,mark_in(number)?,mark_out(number)? | mutates=true | undoable=true"#,
    r#"film.item.matte.colour | Colour | film | command | film | paint | The **colour** of a colour matte is changed. | - | film | item(node-ref)!,colour(text)! | mutates=true | undoable=true"#,
    r#"film.item.remove | Remove | film | command | film | minus | **Take away** a media item. | - | film | item(node-ref)! | mutates=true | undoable=true"#,
    r#"film.item.rename | Rename | film | command | film | pencil | **Name** a media item. | - | film | item(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"film.marker.add | Add | film | command | film | flag | **Add** a marker to a sequence. | - | film | sequence(node-ref)!,start(number)!,duration(number)?,name(text)?,comment(text)?,color(text)?,kind(text)? | mutates=true | undoable=true"#,
    r#"film.marker.clear | Clear | film | command | film | trash | **Take away** every marker of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.marker.next | Next | film | command | film | flag | **Move the playhead to the next marker** of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.marker.previous | Previous | film | command | film | flag | **Move the playhead to the marker before** the one it is at. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.marker.remove | Remove | film | command | film | trash | **Take** a marker away. | - | film | marker(node-ref)! | mutates=true | undoable=true"#,
    r#"film.marker.set | Set | film | command | film | pencil | **Change** a marker. | - | film | marker(node-ref)!,name(text)?,comment(text)?,color(text)?,kind(text)?,duration(number)? | mutates=true | undoable=true"#,
    r#"film.playhead.end | End | film | command | film | forward | **Go to** the end of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.playhead.get | Get | film | command | film | list | **Read** the playhead of a sequence. | - | film | sequence(node-ref)! | mutates=false | undoable=false"#,
    r#"film.playhead.goto.mark | Mark | film | command | film | playhead | **Go to** the **mark** of a sequence: the mark in or the mark out. | - | film | sequence(node-ref)!,which(text)! | mutates=true | undoable=true"#,
    r#"film.playhead.nextedit | Nextedit | film | command | film | next | **Go to** the next edit point of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.playhead.prevedit | Prevedit | film | command | film | previous | **Go to** the previous edit point of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.playhead.set | Set | film | command | film | playhead | **Set** the playhead of a sequence. | - | film | sequence(node-ref)!,frame(number)! | mutates=true | undoable=true"#,
    r#"film.playhead.start | Start | film | command | film | rewind | **Go to** the start of a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.playhead.step | Step | film | command | film | step | **Step** the playhead of a sequence. | - | film | sequence(node-ref)!,frames(number)! | mutates=true | undoable=true"#,
    r#"film.project | Project | film | command | film | list | The **project**: the bins, the items, and the clips that play them. | - | film |  | mutates=false | undoable=false"#,
    r#"film.sequence.mark.clear | Clear | film | command | film | minus | **Take** the **marks** of a sequence away. | - | film | sequence(node-ref)!,which(text)! | mutates=true | undoable=true"#,
    r#"film.sequence.mark.in | In | film | command | film | flag | **Set** the **mark in** of a sequence. | - | film | sequence(node-ref)!,frame(number)! | mutates=true | undoable=true"#,
    r#"film.sequence.mark.out | Out | film | command | film | flag | **Set** the **mark out** of a sequence. | - | film | sequence(node-ref)!,frame(number)! | mutates=true | undoable=true"#,
    r#"film.sequence.rate.get | Get | film | command | film | list | **Read** the rate of a sequence. | - | film | sequence(node-ref)! | mutates=false | undoable=false"#,
    r#"film.sequence.rate.set | Set | film | command | film | clock | **Set** the rate of a sequence. | - | film | sequence(node-ref)!,rate(text)! | mutates=true | undoable=true"#,
    r#"film.timeline | Timeline | film | command | film | list | The timeline of the document. | - | film |  | mutates=false | undoable=false"#,
    r#"film.timeline.closegap | Closegap | film | command | film | collapse | **Close** the gap that holds a place on a track. | - | film | track(node-ref)!,time(number)! | mutates=true | undoable=true"#,
    r#"film.timeline.razor | Razor | film | command | film | razor | **Cut** the clip that is under the playhead with the **razor**. | - | film | sequence(node-ref)!,track(node-ref)!,at(number)? | mutates=true | undoable=true"#,
    r#"film.track.add | Add | film | command | film | plus | Put a **track** in a sequence. | - | film | name(text)!,sequence(node-ref)!,kind(text)! | mutates=true | undoable=true"#,
    r#"film.track.enable | Enable | film | command | film | check | **Enable** a track, or **disable** it. | - | film | track(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"film.track.get | Get | film | command | film | list | Read a **track**. | - | film | track(node-ref)! | mutates=false | undoable=false"#,
    r#"film.track.label | Label | film | command | film | paint | **Put** a **label** on a track, or **take it off**. | - | film | track(node-ref)!,label(text)! | mutates=true | undoable=true"#,
    r#"film.track.lock | Lock | film | command | film | lock | **Lock** a track, or unlock it. | - | film | track(node-ref)!,locked(bool)! | mutates=true | undoable=true"#,
    r#"film.track.mute | Mute | film | command | film | mute | **Mute** a track, or **unmute** it. | - | film | track(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"film.track.pan | Pan | film | command | film | pan | **Set** the **pan** of a track. | - | film | track(node-ref)!,pan(number)! | mutates=true | undoable=true"#,
    r#"film.track.remove | Remove | film | command | film | minus | **Take away** a track. | - | film | track(node-ref)! | mutates=true | undoable=true"#,
    r#"film.track.rename | Rename | film | command | film | pencil | **Rename** a track. | - | film | track(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"film.track.solo | Solo | film | command | film | headphones | **Solo** a track, or **unsolo** it. | - | film | track(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"film.track.volume | Volume | film | command | film | speaker | **Set** the **volume** of a track, in decibels. | - | film | track(node-ref)!,db(number)! | mutates=true | undoable=true"#,
    r#"film.transition.add | Add | film | command | film | plus | **Add** a transition over the cut between two clips. | - | film | track(node-ref)!,from(node-ref)!,to(node-ref)!,duration(number)!,align(text)?,effect(text)? | mutates=true | undoable=true"#,
    r#"film.transition.remove | Remove | film | command | film | minus | **Take away** a transition. | - | film | transition(node-ref)! | mutates=true | undoable=true"#,
    r#"film.transition.set | Set | film | command | film | pencil | **Change** a transition: its duration and its alignment. | - | film | transition(node-ref)!,duration(number)?,align(text)? | mutates=true | undoable=true"#,
    r#"film.workarea.clear | Clear | film | command | film | minus | **Take the work area away** from a sequence. | - | film | sequence(node-ref)! | mutates=true | undoable=true"#,
    r#"film.workarea.get | Get | film | command | film | list | **Read** the work area of a sequence. | - | film | sequence(node-ref)! | mutates=false | undoable=false"#,
    r#"film.workarea.set | Set | film | command | film | marker | **Set** the work area of a sequence. | - | film | sequence(node-ref)!,start(number)!,end(number)! | mutates=true | undoable=true"#,
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
