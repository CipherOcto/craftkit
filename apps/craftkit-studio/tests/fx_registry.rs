//! The **fx battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §860 converted this battery from one declaration per command to **one
//! table per file** — the `commands!` table of the derive. A conversion
//! that changed a single id, a label, a layer, an icon, a menu path, a
//! parameter or a flag would be a conversion that changed the tree.
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

#![cfg(feature = "fx")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the fx battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r##"fx.comp.add | Add | fx | command | fx | plus | Put a **composition** in the document. | - | fx | width(number)! help="How wide it is, in pixels.",height(number)! help="How tall it is, in pixels.",duration(number)! help="How long it is, in frames." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"fx.comp.switch | Switch | fx | command | fx | eye | Put the composition's **hide shy layers** switch on or off. | - | fx | comp(node-ref)! help="The composition.",switch(text)! help="The switch: `hide_shy`.",value(bool)? help="Whether it is on. Without it, the switch is toggled." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.add | Add | fx | command | fx | plus | Put an **effect** on a layer. | - | fx | layer(node-ref)! help="The layer the effect goes on.",effect(text)! help="Which effect it is: `blur`, `glow`, `levels`, `noise`, `posterize` or `sepia`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.duplicate | Duplicate | fx | command | fx | copy | **Copy** an effect of a layer. | - | fx | layer(node-ref)! help="The layer the effect is on.",effect(number)! help="Which effect it is, counting from one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.list | List | fx | command | fx | list | Say what an **effect** is. | - | fx |  | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.effect.remove | Remove | fx | command | fx | trash | Take an **effect** away from a layer. | - | fx | layer(node-ref)! help="The layer the effect is on.",index(number)! help="Which effect it is, counting from one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.reorder | Reorder | fx | command | fx | arrow | **Move** an effect to another place in the stack of a layer. | - | fx | layer(node-ref)! help="The layer the effect is on.",effect(number)! help="Which effect it is, counting from one.",to(number)! help="Which place it goes to, counting from one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.reset | Reset | fx | command | fx | undo | Put the parameters of an effect back to their **defaults**. | - | fx | layer(node-ref)! help="The layer the effect is on.",effect(number)! help="Which effect it is, counting from one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.effect.toggle | Toggle | fx | command | fx | power | **Switch** one effect of a layer off or on. | - | fx | layer(node-ref)! help="The layer the effect is on.",effect(number)! help="Which effect it is, counting from one.",on(bool)? help="Whether it runs. Without it the switch is toggled." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.auto | Auto | fx | command | fx | pencil | **Set** or **clear** the **auto bezier** of a keyframe. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on.",auto(bool)! help="Whether the keyframe is an auto bezier." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.continuous | Continuous | fx | command | fx | pencil | **Lock** the two sides of a keyframe together: a **continuous** keyframe. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on.",continuous(bool)? help="Whether the keyframe is continuous. The default is that it is.",speed(number)? help="The speed of both sides, in value units per frame. The default is the speed the keyframe has on the side it is entered on.",influence(number)? help="The influence of both sides: the fraction of the segment each handle takes." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.easy | Easy | fx | command | fx | pencil | Give a keyframe the **easy ease**. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on.",side(text)? help="Which **side** of the keyframe: `both`, `in` or `out`. The default is `both`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.hold | Hold | fx | command | fx | pencil | Turn a keyframe's **hold** on, or off again. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.interp | Interp | fx | command | fx | pencil | The **interpolation** of a keyframe. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on.",interp(text)! help="`linear`, `bezier` or `hold`.",speed(number)? help="The speed of a bezier, in value units per frame.",influence(number)? help="The influence of a bezier: the fraction of the segment its handle takes.",side(text)? help="Which **side** of the keyframe: `out` for the side it leaves on and `in` for the side it is entered on. The default is `out`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.remove | Remove | fx | command | fx | minus | **Take away** a keyframe of a property. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.keyframe.set | Set | fx | command | fx | plus | **Set** a keyframe on a property. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame the keyframe is on.",value(json)! help="What the property is at that frame." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.add | Add | fx | command | fx | plus | Put a **layer** in a composition. | - | fx | comp(node-ref)! help="The composition it goes in.",name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.arrange | Arrange | fx | command | fx | arrows | **Move** a layer in the stack. | - | fx | comp(node-ref)! help="The composition.",layer(node-ref)! help="The layer.",to(text)! help="Where to: `forward`, `backward`, `front` or `back`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.blend | Blend | fx | command | fx | paint | Set the **blending mode** of a layer. | - | fx | layer(node-ref)! help="The layer.",mode(text)? help="The mode: `Normal`, `Multiply`, `Screen`, …",step(number)? help="The step: plus one for the next mode, minus one for the one before." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.deselect | Deselect | fx | command | fx | select | **Take** layers **out** of the selection. | - | fx | layers(json)! help="The layers: their ids." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.layer.list | List | fx | command | fx | list | **Read** the stack of layers of a composition. | - | fx | comp(node-ref)! help="The composition." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.layer.parent | Parent | fx | command | fx | link | Put a layer **under** another layer. | - | fx | layer(node-ref)! help="The layer to put under the parent.",parent(node-ref)! help="The parent." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.parent.delete | Delete | fx | command | fx | minus | Take a layer **out** of its parent. | - | fx | layer(node-ref)! help="The layer to take out." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.rename | Rename | fx | command | fx | pencil | **Rename** a layer. | - | fx | layer(node-ref)! help="The layer.",name(text)! help="The name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.layer.select | Select | fx | command | fx | select | **Select** layers. | - | fx | layers(json)! help="The layers: their ids." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.layer.switch | Switch | fx | command | fx | eye | Put a **switch** on a layer, or off. | - | fx | layer(node-ref)! help="The layer the switch is on.",switch(text)! help="The switch: `video`, `audio`, `solo`, `lock`, `shy`, …",value(bool)? help="Whether it is on. Without it, the switch is toggled." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.marker.add | Add | fx | command | fx | plus | Put a **marker** in a composition. | - | fx | comp(node-ref)! help="The composition.",frame(number)! help="Where the marker is, in frames.",comment(text)? help="What it says." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.marker.set | Set | fx | command | fx | pencil | Change a **marker**: where it is, or what it says. | - | fx | marker(node-ref)! help="The marker to change.",frame(number)? help="Its new place, in frames.",comment(text)? help="Its new comment." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.mask.add | Add | fx | command | fx | mask | **Make** a **mask** on a layer. | - | fx | layer(node-ref)! help="The layer the mask is on.",vertices(json)! help="The vertices of the mask: `[[x, y], …]` in the layer's space.",closed(bool)? help="Whether the path is closed.",mode(text)? help="The mode: `None`, `Add`, `Subtract`, `Intersect`, `Lighten`, `Darken` or `Difference`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.mask.invert | Invert | fx | command | fx | mask | **Invert** a mask, or **uninvert** it. | - | fx | layer(node-ref)! help="The layer the mask is on.",mask(node-ref)! help="The mask.",value(bool)? help="Whether the mask is inverted." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.mask.list | List | fx | command | fx | list | **Read** the masks of a layer. | - | fx | layer(node-ref)! help="The layer." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.mask.lock | Lock | fx | command | fx | lock | **Lock** a mask, or **unlock** it. | - | fx | layer(node-ref)! help="The layer the mask is on.",mask(node-ref)! help="The mask.",value(bool)? help="Whether the mask is locked." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.mask.mode | Mode | fx | command | fx | mask | **Set** the **mode** of a mask. | - | fx | layer(node-ref)! help="The layer the mask is on.",mask(node-ref)! help="The mask.",mode(text)! help="The mode: `None`, `Add`, `Subtract`, `Intersect`, `Lighten`, `Darken` or `Difference`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.mask.remove | Remove | fx | command | fx | minus | **Take away** a mask of a layer. | - | fx | layer(node-ref)! help="The layer the mask is on.",mask(node-ref)! help="The mask." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.prop.at | At | fx | command | fx | list | The **value** of a property at a frame. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",frame(number)! help="The frame." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.prop.bake | Bake | fx | command | fx | plus | **Bake** the expression of a property into **keyframes**. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",from(number)! help="The first frame.",to(number)! help="The last frame.",step(number)? help="The frames between the samples. It is one unless the file says another." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.prop.expression | Expression | fx | command | fx | pencil | Put an **expression** on a property. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",expression(text)! help="The expression itself." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.prop.expression.delete | Delete | fx | command | fx | minus | Take the **expression** off a property. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.prop.get | Get | fx | command | fx | list | Read a property, by its path. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.prop.set | Set | fx | command | fx | pencil | Set a property, by its path. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",value(json)! help="What it is now." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.prop.spatial | Spatial | fx | command | fx | path | Make a property **spatial**. | - | fx | layer(node-ref)! help="The layer the property is on.",path(text)! help="The path of the property.",spatial(bool)! help="Whether the property is spatial." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.selection.get | Get | fx | command | fx | list | **Read** the selection. | - | fx | detailed(bool)? help="Whether to say which layers they are and not only how many." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.time.end | End | fx | command | fx | forward | **Go to** the end of a composition. | - | fx | comp(node-ref)! help="The composition." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.time.get | Get | fx | command | fx | list | **Read** the time of a composition. | - | fx | comp(node-ref)! help="The composition." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"fx.time.set | Set | fx | command | fx | clock | **Set** the time of a composition. | - | fx | comp(node-ref)! help="The composition.",frame(number)! help="The frame." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.time.start | Start | fx | command | fx | rewind | **Go to** the start of a composition. | - | fx | comp(node-ref)! help="The composition." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"fx.time.step | Step | fx | command | fx | step | **Step** the time of a composition. | - | fx | comp(node-ref)! help="The composition.",frames(number)! help="The frames: a negative number steps back." | mutates=true | undoable=true | journal=true | example=false"##,
];

/// The commands of the fx battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "fx").cloned().collect();
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
