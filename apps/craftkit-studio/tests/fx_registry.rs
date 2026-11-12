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
    r#"fx.comp.add | Add | fx | command | fx | plus | Put a **composition** in the document. | - | fx | width(number)!,height(number)!,duration(number)! | mutates=true | undoable=true"#,
    r#"fx.comp.switch | Switch | fx | command | fx | eye | Put the composition's **hide shy layers** switch on or off. | - | fx | comp(node-ref)!,switch(text)!,value(bool)? | mutates=true | undoable=true"#,
    r#"fx.effect.add | Add | fx | command | fx | plus | Put an **effect** on a layer. | - | fx | layer(node-ref)!,effect(text)! | mutates=true | undoable=true"#,
    r#"fx.effect.duplicate | Duplicate | fx | command | fx | copy | **Copy** an effect of a layer. | - | fx | layer(node-ref)!,effect(number)! | mutates=true | undoable=true"#,
    r#"fx.effect.list | List | fx | command | fx | list | Say what an **effect** is. | - | fx |  | mutates=false | undoable=false"#,
    r#"fx.effect.remove | Remove | fx | command | fx | trash | Take an **effect** away from a layer. | - | fx | layer(node-ref)!,index(number)! | mutates=true | undoable=true"#,
    r#"fx.effect.reorder | Reorder | fx | command | fx | arrow | **Move** an effect to another place in the stack of a layer. | - | fx | layer(node-ref)!,effect(number)!,to(number)! | mutates=true | undoable=true"#,
    r#"fx.effect.reset | Reset | fx | command | fx | undo | Put the parameters of an effect back to their **defaults**. | - | fx | layer(node-ref)!,effect(number)! | mutates=true | undoable=true"#,
    r#"fx.effect.toggle | Toggle | fx | command | fx | power | **Switch** one effect of a layer off or on. | - | fx | layer(node-ref)!,effect(number)!,on(bool)? | mutates=true | undoable=true"#,
    r#"fx.keyframe.auto | Auto | fx | command | fx | pencil | **Set** or **clear** the **auto bezier** of a keyframe. | - | fx | layer(node-ref)!,path(text)!,frame(number)!,auto(bool)! | mutates=true | undoable=true"#,
    r#"fx.keyframe.continuous | Continuous | fx | command | fx | pencil | **Lock** the two sides of a keyframe together: a **continuous** keyframe. | - | fx | layer(node-ref)!,path(text)!,frame(number)!,continuous(bool)?,speed(number)?,influence(number)? | mutates=true | undoable=true"#,
    r#"fx.keyframe.easy | Easy | fx | command | fx | pencil | Give a keyframe the **easy ease**. | - | fx | layer(node-ref)!,path(text)!,frame(number)!,side(text)? | mutates=true | undoable=true"#,
    r#"fx.keyframe.hold | Hold | fx | command | fx | pencil | Turn a keyframe's **hold** on, or off again. | - | fx | layer(node-ref)!,path(text)!,frame(number)! | mutates=true | undoable=true"#,
    r#"fx.keyframe.interp | Interp | fx | command | fx | pencil | The **interpolation** of a keyframe. | - | fx | layer(node-ref)!,path(text)!,frame(number)!,interp(text)!,speed(number)?,influence(number)?,side(text)? | mutates=true | undoable=true"#,
    r#"fx.keyframe.remove | Remove | fx | command | fx | minus | **Take away** a keyframe of a property. | - | fx | layer(node-ref)!,path(text)!,frame(number)! | mutates=true | undoable=true"#,
    r#"fx.keyframe.set | Set | fx | command | fx | plus | **Set** a keyframe on a property. | - | fx | layer(node-ref)!,path(text)!,frame(number)!,value(json)! | mutates=true | undoable=true"#,
    r#"fx.layer.add | Add | fx | command | fx | plus | Put a **layer** in a composition. | - | fx | comp(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"fx.layer.arrange | Arrange | fx | command | fx | arrows | **Move** a layer in the stack. | - | fx | comp(node-ref)!,layer(node-ref)!,to(text)! | mutates=true | undoable=true"#,
    r#"fx.layer.blend | Blend | fx | command | fx | paint | Set the **blending mode** of a layer. | - | fx | layer(node-ref)!,mode(text)?,step(number)? | mutates=true | undoable=true"#,
    r#"fx.layer.deselect | Deselect | fx | command | fx | select | **Take** layers **out** of the selection. | - | fx | layers(json)! | mutates=false | undoable=false"#,
    r#"fx.layer.list | List | fx | command | fx | list | **Read** the stack of layers of a composition. | - | fx | comp(node-ref)! | mutates=false | undoable=false"#,
    r#"fx.layer.parent | Parent | fx | command | fx | link | Put a layer **under** another layer. | - | fx | layer(node-ref)!,parent(node-ref)! | mutates=true | undoable=true"#,
    r#"fx.layer.parent.delete | Delete | fx | command | fx | minus | Take a layer **out** of its parent. | - | fx | layer(node-ref)! | mutates=true | undoable=true"#,
    r#"fx.layer.rename | Rename | fx | command | fx | pencil | **Rename** a layer. | - | fx | layer(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"fx.layer.select | Select | fx | command | fx | select | **Select** layers. | - | fx | layers(json)! | mutates=false | undoable=false"#,
    r#"fx.layer.switch | Switch | fx | command | fx | eye | Put a **switch** on a layer, or off. | - | fx | layer(node-ref)!,switch(text)!,value(bool)? | mutates=true | undoable=true"#,
    r#"fx.marker.add | Add | fx | command | fx | plus | Put a **marker** in a composition. | - | fx | comp(node-ref)!,frame(number)!,comment(text)? | mutates=true | undoable=true"#,
    r#"fx.marker.set | Set | fx | command | fx | pencil | Change a **marker**: where it is, or what it says. | - | fx | marker(node-ref)!,frame(number)?,comment(text)? | mutates=true | undoable=true"#,
    r#"fx.mask.add | Add | fx | command | fx | mask | **Make** a **mask** on a layer. | - | fx | layer(node-ref)!,vertices(json)!,closed(bool)?,mode(text)? | mutates=true | undoable=true"#,
    r#"fx.mask.invert | Invert | fx | command | fx | mask | **Invert** a mask, or **uninvert** it. | - | fx | layer(node-ref)!,mask(node-ref)!,value(bool)? | mutates=true | undoable=true"#,
    r#"fx.mask.list | List | fx | command | fx | list | **Read** the masks of a layer. | - | fx | layer(node-ref)! | mutates=false | undoable=false"#,
    r#"fx.mask.lock | Lock | fx | command | fx | lock | **Lock** a mask, or **unlock** it. | - | fx | layer(node-ref)!,mask(node-ref)!,value(bool)? | mutates=true | undoable=true"#,
    r#"fx.mask.mode | Mode | fx | command | fx | mask | **Set** the **mode** of a mask. | - | fx | layer(node-ref)!,mask(node-ref)!,mode(text)! | mutates=true | undoable=true"#,
    r#"fx.mask.remove | Remove | fx | command | fx | minus | **Take away** a mask of a layer. | - | fx | layer(node-ref)!,mask(node-ref)! | mutates=true | undoable=true"#,
    r#"fx.prop.at | At | fx | command | fx | list | The **value** of a property at a frame. | - | fx | layer(node-ref)!,path(text)!,frame(number)! | mutates=false | undoable=false"#,
    r#"fx.prop.bake | Bake | fx | command | fx | plus | **Bake** the expression of a property into **keyframes**. | - | fx | layer(node-ref)!,path(text)!,from(number)!,to(number)!,step(number)? | mutates=true | undoable=true"#,
    r#"fx.prop.expression | Expression | fx | command | fx | pencil | Put an **expression** on a property. | - | fx | layer(node-ref)!,path(text)!,expression(text)! | mutates=true | undoable=true"#,
    r#"fx.prop.expression.delete | Delete | fx | command | fx | minus | Take the **expression** off a property. | - | fx | layer(node-ref)!,path(text)! | mutates=true | undoable=true"#,
    r#"fx.prop.get | Get | fx | command | fx | list | Read a property, by its path. | - | fx | layer(node-ref)!,path(text)! | mutates=false | undoable=false"#,
    r#"fx.prop.set | Set | fx | command | fx | pencil | Set a property, by its path. | - | fx | layer(node-ref)!,path(text)!,value(json)! | mutates=true | undoable=true"#,
    r#"fx.prop.spatial | Spatial | fx | command | fx | path | Make a property **spatial**. | - | fx | layer(node-ref)!,path(text)!,spatial(bool)! | mutates=true | undoable=true"#,
    r#"fx.selection.get | Get | fx | command | fx | list | **Read** the selection. | - | fx | detailed(bool)? | mutates=false | undoable=false"#,
    r#"fx.time.end | End | fx | command | fx | forward | **Go to** the end of a composition. | - | fx | comp(node-ref)! | mutates=true | undoable=true"#,
    r#"fx.time.get | Get | fx | command | fx | list | **Read** the time of a composition. | - | fx | comp(node-ref)! | mutates=false | undoable=false"#,
    r#"fx.time.set | Set | fx | command | fx | clock | **Set** the time of a composition. | - | fx | comp(node-ref)!,frame(number)! | mutates=true | undoable=true"#,
    r#"fx.time.start | Start | fx | command | fx | rewind | **Go to** the start of a composition. | - | fx | comp(node-ref)! | mutates=true | undoable=true"#,
    r#"fx.time.step | Step | fx | command | fx | step | **Step** the time of a composition. | - | fx | comp(node-ref)!,frames(number)! | mutates=true | undoable=true"#,
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
