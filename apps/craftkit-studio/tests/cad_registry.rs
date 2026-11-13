//! The **cad battery's registry**, pinned.
//!
//! # Why this test exists
//!
//! §862 converted this battery from one declaration per command to
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

#![cfg(feature = "cad")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the cad battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r##"cad.audit | Audit | cad | command | cad | list | **Audit** a drawing. | - | cad | drawing(node-ref)! help="The drawing." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.block.add | Add | cad | command | cad | plus | Put a **block** in a drawing. | - | cad | drawing(node-ref)! help="The drawing it goes in.",name(text)! help="The block's name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.constraint.add | Add | cad | command | cad | link | **Add** a **constraint** to a drawing. | - | cad | first(node-ref)! help="The first entity.",second(node-ref)! help="The second entity.",kind(text)! help="The kind of constraint: `coincident`, `parallel`, `perpendicular`, `horizontal`, `vertical`, `tangent`, `concentric` or `equal`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.constraint.list | List | cad | command | cad | list | **Read** the **constraints** of a drawing. | - | cad | drawing(node-ref)! help="The drawing." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.dimension.add | Add | cad | command | cad | ruler | **Add** a dimension to a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",entity(node-ref)! help="The entity it **measures**.",at(json)! help="Where it is written." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.dimension.text | Text | cad | command | cad | pencil | **Say** what a dimension says of its own. | - | cad | dimension(node-ref)! help="The dimension.",text(text)! help="What it says. Without it the measurement shows." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.drawing.add | Add | cad | command | cad | plus | Put a **drawing** in the document. | - | cad | name(text)! help="What to call it." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"cad.entity.arc.add | Add | cad | command | cad | plus | Put an **arc** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",center(json)! help="Its centre.",radius(number)! help="Its radius: above nothing.",start(number)! help="The angle it starts at, in radians.",end(number)! help="The angle it ends at, in radians." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.array.polar | Polar | cad | command | cad | array | **Array** an entity **about a centre**: the items go round it. | - | cad | entity(node-ref)! help="The entity to array.",centre(json)! help="The centre of the array.",count(number)! help="How many items the array holds.",degrees(number)? help="The angle the array goes through, in degrees. A whole turn when it is not said.",rotate(bool)? help="Whether the copies are turned with the array.",block(node-ref)? help="The block the copies go in. The block of the entity itself when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.array.rectangular | Rectangular | cad | command | cad | array | **Array** an entity in a **grid**: the rows and the columns. | - | cad | entity(node-ref)! help="The entity to array.",rows(number)! help="How many rows.",columns(number)! help="How many columns.",row_spacing(number)! help="The distance between the rows.",column_spacing(number)! help="The distance between the columns.",block(node-ref)? help="The block the copies go in. The block of the entity itself when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.back | Back | cad | command | cad | arrows | **Send** an entity to the **back**. | - | cad | entity(node-ref)! help="The entity." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.backward | Backward | cad | command | cad | arrows | **Move** an entity **one step** towards the back. | - | cad | entity(node-ref)! help="The entity." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.circle.add | Add | cad | command | cad | plus | Put a **circle** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",center(json)! help="Its centre.",radius(number)! help="Its radius." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.copy | Copy | cad | command | cad | copy | **Copy** an entity. | - | cad | entity(node-ref)! help="The entity to copy.",name(text)! help="What to call the copy.",by(json)! help="The vector the copy is made **by**.",block(node-ref)? help="The block the copy goes in. The block of the entity itself when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.ellipse.add | Add | cad | command | cad | plus | Put an **ellipse** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",center(json)! help="Its centre.",major(json)! help="Its **major axis**: a vector from the centre to the end of the major axis.",ratio(number)! help="The **ratio** of the minor axis to the major one: above nothing and at most one.",start(number)? help="The angle the arc runs from. The whole ellipse when it is not said.",end(number)? help="The angle the arc runs to. The whole ellipse when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.forward | Forward | cad | command | cad | arrows | **Move** an entity **one step** towards the front. | - | cad | entity(node-ref)! help="The entity." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.front | Front | cad | command | cad | arrows | **Bring** an entity to the **front**. | - | cad | entity(node-ref)! help="The entity." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.insert.add | Add | cad | command | cad | plus | Put an **insert** in a block: an instance of another block. | - | cad | block(node-ref)! help="The block the instance goes in.",name(text)! help="What to call it.",inserts(node-ref)! help="The block it inserts.",at(json)! help="Where it is put." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.layer | Layer | cad | command | cad | plus | Put an entity on a **layer**. | - | cad | entity(node-ref)! help="The entity.",layer(node-ref)! help="The layer." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.line.add | Add | cad | command | cad | plus | Put a **line** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",from(json)! help="Where it starts.",to(json)! help="Where it ends." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.list | List | cad | command | cad | list | **Read** the entities of a block, in the order they are drawn. | - | cad | block(node-ref)! help="The block." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.entity.mirror | Mirror | cad | command | cad | copy | **Mirror** an entity over a line. | - | cad | entity(node-ref)! help="The entity.",from(json)! help="A point on the line the entity is turned over.",to(json)! help="Another point on that line." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.move | Move | cad | command | cad | arrows | **Move** an entity by a vector. | - | cad | entity(node-ref)! help="The entity.",by(json)! help="The vector it moves by." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.polyline.add | Add | cad | command | cad | plus | Put a **polyline** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",points(json)! help="Its **points**: `[{ \"x\": …, \"y\": … }, …]`.",closed(bool)? help="Whether the polyline is **closed**." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.remove | Remove | cad | command | cad | minus | **Take** an entity **away**. | - | cad | entity(node-ref)! help="The entity." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.rotate | Rotate | cad | command | cad | rotate | **Turn** an entity about a point. | - | cad | entity(node-ref)! help="The entity.",about(json)! help="The point it turns about.",degrees(number)! help="The angle, in degrees." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.scale | Scale | cad | command | cad | scale | **Scale** an entity about a point. | - | cad | entity(node-ref)! help="The entity.",about(json)! help="The point it scales about.",factor(number)! help="The factor: above nothing." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.entity.text.add | Add | cad | command | cad | plus | Put a **text** in a block. | - | cad | block(node-ref)! help="The block it goes in.",name(text)! help="What to call it.",text(text)! help="What it says.",at(json)! help="Where it is written.",height(number)? help="How big it is." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.export.dxf | Dxf | cad | command | cad | file | **Write** a drawing as a **DXF file**. | - | cad | path(text)! help="Where to write the file.",drawing(node-ref)! help="The drawing to write." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.export.svg | Svg | cad | command | cad | file | **Write** a drawing as an **SVG file**. | - | cad | path(text)! help="Where to write the file.",drawing(node-ref)! help="The drawing to write." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.group.add | Add | cad | command | cad | group | **Make** a **group** of entities. | - | cad | name(text)! help="What to call the group.",block(node-ref)! help="The block the group stands in.",members(json)! help="The entities that are members of the group." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.group.list | List | cad | command | cad | list | **Read** the **groups** of a drawing. | - | cad | drawing(node-ref)! help="The drawing." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.group.move | Move | cad | command | cad | move | **Move** the **members** of a group. | - | cad | group(node-ref)! help="The group.",by(json)! help="The vector the members go by." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.import.dxf | Dxf | cad | command | cad | file | **Read** a DXF file and make the **drawing** it names. | - | cad | path(text)! help="The file to read.",name(text)? help="What to call the drawing." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.import.svg | Svg | cad | command | cad | file | **Read** an SVG file and make the **drawing** it names. | - | cad | path(text)! help="The file to read.",name(text)? help="What to call the drawing." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.add | Add | cad | command | cad | plus | Put a **layer** in a drawing. | - | cad | drawing(node-ref)! help="The drawing it goes in.",name(text)! help="The layer's name.",color(text)? help="Its colour, as an index or a name.",fill(text)? help="The colour it is drawn in, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.color | Color | cad | command | cad | paint | **Set** the **colour** of a layer. | - | cad | layer(node-ref)! help="The layer.",color(text)! help="The colour: an index or a name." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.fill | Fill | cad | command | cad | paint | **Set** the colour a layer is **drawn in**, as `#rrggbb`. | - | cad | layer(node-ref)! help="The layer.",fill(text)! help="The colour it is drawn in, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.freeze | Freeze | cad | command | cad | snow | **Freeze** a layer, or **unfreeze** it. | - | cad | layer(node-ref)! help="The layer.",on(bool)! help="Whether the layer is frozen." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.linetype | Linetype | cad | command | cad | line | **Set** the **line type** of a layer. | - | cad | layer(node-ref)! help="The layer.",linetype(text)! help="The line type: `Continuous`, `Dashed`, `Center` and so on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.lineweight | Lineweight | cad | command | cad | weight | **Set** the **line weight** of a layer, in millimetres. | - | cad | layer(node-ref)! help="The layer.",mm(number)! help="The line weight, in millimetres." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.list | List | cad | command | cad | list | **Read** the layers of a drawing. | - | cad | drawing(node-ref)! help="The drawing." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.layer.lock | Lock | cad | command | cad | lock | **Lock** a layer, or **unlock** it. | - | cad | layer(node-ref)! help="The layer.",on(bool)! help="Whether the layer is locked." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.layer.on | On | cad | command | cad | check | **Turn** a layer **on**, or **off**. | - | cad | layer(node-ref)! help="The layer.",on(bool)! help="Whether the layer is on." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.linetype.list | List | cad | command | cad | line | **Read** the line types of this tree. | - | cad |  | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.measure.angle | Angle | cad | command | cad | rotate | The **angle** at a point, between two directions. | - | cad | vertex(json)! help="The point the angle is at.",first(json)! help="The first direction: the point it points at.",second(json)! help="The second direction: the point it points at." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.measure.area | Area | cad | command | cad | rect | The **area** an entity encloses. | - | cad | entity(node-ref)! help="The entity." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.measure.length | Length | cad | command | cad | ruler | The **length** of an entity's path. | - | cad | entity(node-ref)! help="The entity." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.measure.radius | Radius | cad | command | cad | circle | The **radius** of an entity. | - | cad | entity(node-ref)! help="The entity." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"cad.table.add | Add | cad | command | cad | table | **Add** a **table** to a block. | - | cad | block(node-ref)! help="The block the table stands in.",name(text)! help="What to call it.",at(json)! help="Where its first corner is.",rows(number)! help="How many rows it has.",columns(number)! help="How many columns it has." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.table.delete.row | Row | cad | command | cad | trash | **Delete** a **row** of a table. | - | cad | table(node-ref)! help="The table the row is in.",row(number)! help="The row that goes." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.table.insert.row | Row | cad | command | cad | plus | **Insert** a **row** into a table. | - | cad | table(node-ref)! help="The table the row goes in.",row(number)? help="The row it goes before. It goes at the end when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.table.merge | Merge | cad | command | cad | merge | **Merge** the cells of a table into one. | - | cad | table(node-ref)! help="The table the cells are in.",row(number)! help="The row of the first cell.",column(number)! help="The column of the first cell.",rows(number)! help="How many rows the merge covers.",columns(number)! help="How many columns the merge covers." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.table.set | Set | cad | command | cad | text | **Write** the **text** of a cell. | - | cad | table(node-ref)! help="The table the cell is in.",row(number)! help="The row of the cell.",column(number)! help="The column of the cell.",text(text)! help="The text of the cell." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"cad.table.unmerge | Unmerge | cad | command | cad | unmerge | **Unmerge** the cells of a table. | - | cad | table(node-ref)! help="The table the cells are in.",row(number)! help="The row of the cell.",column(number)! help="The column of the cell." | mutates=true | undoable=true | journal=true | example=false"##,
];

/// The commands of the cad battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "cad").cloned().collect();
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
