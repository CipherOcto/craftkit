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
    r#"cad.audit | Audit | cad | command | cad | list | **Audit** a drawing. | - | cad | drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.block.add | Add | cad | command | cad | plus | Put a **block** in a drawing. | - | cad | drawing(node-ref)!,name(text)! | mutates=true | undoable=true"#,
    r#"cad.constraint.add | Add | cad | command | cad | link | **Add** a **constraint** to a drawing. | - | cad | first(node-ref)!,second(node-ref)!,kind(text)! | mutates=true | undoable=true"#,
    r#"cad.constraint.list | List | cad | command | cad | list | **Read** the **constraints** of a drawing. | - | cad | drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.dimension.add | Add | cad | command | cad | ruler | **Add** a dimension to a block. | - | cad | block(node-ref)!,name(text)!,entity(node-ref)!,at(json)! | mutates=true | undoable=true"#,
    r#"cad.dimension.text | Text | cad | command | cad | pencil | **Say** what a dimension says of its own. | - | cad | dimension(node-ref)!,text(text)! | mutates=true | undoable=true"#,
    r#"cad.drawing.add | Add | cad | command | cad | plus | Put a **drawing** in the document. | - | cad | name(text)! | mutates=true | undoable=true"#,
    r#"cad.entity.arc.add | Add | cad | command | cad | plus | Put an **arc** in a block. | - | cad | block(node-ref)!,name(text)!,center(json)!,radius(number)!,start(number)!,end(number)! | mutates=true | undoable=true"#,
    r#"cad.entity.array.polar | Polar | cad | command | cad | array | **Array** an entity **about a centre**: the items go round it. | - | cad | entity(node-ref)!,centre(json)!,count(number)!,degrees(number)?,rotate(bool)?,block(node-ref)? | mutates=true | undoable=true"#,
    r#"cad.entity.array.rectangular | Rectangular | cad | command | cad | array | **Array** an entity in a **grid**: the rows and the columns. | - | cad | entity(node-ref)!,rows(number)!,columns(number)!,row_spacing(number)!,column_spacing(number)!,block(node-ref)? | mutates=true | undoable=true"#,
    r#"cad.entity.back | Back | cad | command | cad | arrows | **Send** an entity to the **back**. | - | cad | entity(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.backward | Backward | cad | command | cad | arrows | **Move** an entity **one step** towards the back. | - | cad | entity(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.circle.add | Add | cad | command | cad | plus | Put a **circle** in a block. | - | cad | block(node-ref)!,name(text)!,center(json)!,radius(number)! | mutates=true | undoable=true"#,
    r#"cad.entity.copy | Copy | cad | command | cad | copy | **Copy** an entity. | - | cad | entity(node-ref)!,name(text)!,by(json)!,block(node-ref)? | mutates=true | undoable=true"#,
    r#"cad.entity.ellipse.add | Add | cad | command | cad | plus | Put an **ellipse** in a block. | - | cad | block(node-ref)!,name(text)!,center(json)!,major(json)!,ratio(number)!,start(number)?,end(number)? | mutates=true | undoable=true"#,
    r#"cad.entity.forward | Forward | cad | command | cad | arrows | **Move** an entity **one step** towards the front. | - | cad | entity(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.front | Front | cad | command | cad | arrows | **Bring** an entity to the **front**. | - | cad | entity(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.insert.add | Add | cad | command | cad | plus | Put an **insert** in a block: an instance of another block. | - | cad | block(node-ref)!,name(text)!,inserts(node-ref)!,at(json)! | mutates=true | undoable=true"#,
    r#"cad.entity.layer | Layer | cad | command | cad | plus | Put an entity on a **layer**. | - | cad | entity(node-ref)!,layer(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.line.add | Add | cad | command | cad | plus | Put a **line** in a block. | - | cad | block(node-ref)!,name(text)!,from(json)!,to(json)! | mutates=true | undoable=true"#,
    r#"cad.entity.list | List | cad | command | cad | list | **Read** the entities of a block, in the order they are drawn. | - | cad | block(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.entity.mirror | Mirror | cad | command | cad | copy | **Mirror** an entity over a line. | - | cad | entity(node-ref)!,from(json)!,to(json)! | mutates=true | undoable=true"#,
    r#"cad.entity.move | Move | cad | command | cad | arrows | **Move** an entity by a vector. | - | cad | entity(node-ref)!,by(json)! | mutates=true | undoable=true"#,
    r#"cad.entity.polyline.add | Add | cad | command | cad | plus | Put a **polyline** in a block. | - | cad | block(node-ref)!,name(text)!,points(json)!,closed(bool)? | mutates=true | undoable=true"#,
    r#"cad.entity.remove | Remove | cad | command | cad | minus | **Take** an entity **away**. | - | cad | entity(node-ref)! | mutates=true | undoable=true"#,
    r#"cad.entity.rotate | Rotate | cad | command | cad | rotate | **Turn** an entity about a point. | - | cad | entity(node-ref)!,about(json)!,degrees(number)! | mutates=true | undoable=true"#,
    r#"cad.entity.scale | Scale | cad | command | cad | scale | **Scale** an entity about a point. | - | cad | entity(node-ref)!,about(json)!,factor(number)! | mutates=true | undoable=true"#,
    r#"cad.entity.text.add | Add | cad | command | cad | plus | Put a **text** in a block. | - | cad | block(node-ref)!,name(text)!,text(text)!,at(json)!,height(number)? | mutates=true | undoable=true"#,
    r#"cad.export.dxf | Dxf | cad | command | cad | file | **Write** a drawing as a **DXF file**. | - | cad | path(text)!,drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.export.svg | Svg | cad | command | cad | file | **Write** a drawing as an **SVG file**. | - | cad | path(text)!,drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.group.add | Add | cad | command | cad | group | **Make** a **group** of entities. | - | cad | name(text)!,block(node-ref)!,members(json)! | mutates=true | undoable=true"#,
    r#"cad.group.list | List | cad | command | cad | list | **Read** the **groups** of a drawing. | - | cad | drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.group.move | Move | cad | command | cad | move | **Move** the **members** of a group. | - | cad | group(node-ref)!,by(json)! | mutates=true | undoable=true"#,
    r#"cad.import.dxf | Dxf | cad | command | cad | file | **Read** a DXF file and make the **drawing** it names. | - | cad | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"cad.import.svg | Svg | cad | command | cad | file | **Read** an SVG file and make the **drawing** it names. | - | cad | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"cad.layer.add | Add | cad | command | cad | plus | Put a **layer** in a drawing. | - | cad | drawing(node-ref)!,name(text)!,color(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"cad.layer.color | Color | cad | command | cad | paint | **Set** the **colour** of a layer. | - | cad | layer(node-ref)!,color(text)! | mutates=true | undoable=true"#,
    r#"cad.layer.fill | Fill | cad | command | cad | paint | **Set** the colour a layer is **drawn in**, as `#rrggbb`. | - | cad | layer(node-ref)!,fill(text)! | mutates=true | undoable=true"#,
    r#"cad.layer.freeze | Freeze | cad | command | cad | snow | **Freeze** a layer, or **unfreeze** it. | - | cad | layer(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"cad.layer.linetype | Linetype | cad | command | cad | line | **Set** the **line type** of a layer. | - | cad | layer(node-ref)!,linetype(text)! | mutates=true | undoable=true"#,
    r#"cad.layer.lineweight | Lineweight | cad | command | cad | weight | **Set** the **line weight** of a layer, in millimetres. | - | cad | layer(node-ref)!,mm(number)! | mutates=true | undoable=true"#,
    r#"cad.layer.list | List | cad | command | cad | list | **Read** the layers of a drawing. | - | cad | drawing(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.layer.lock | Lock | cad | command | cad | lock | **Lock** a layer, or **unlock** it. | - | cad | layer(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"cad.layer.on | On | cad | command | cad | check | **Turn** a layer **on**, or **off**. | - | cad | layer(node-ref)!,on(bool)! | mutates=true | undoable=true"#,
    r#"cad.linetype.list | List | cad | command | cad | line | **Read** the line types of this tree. | - | cad |  | mutates=false | undoable=false"#,
    r#"cad.measure.angle | Angle | cad | command | cad | rotate | The **angle** at a point, between two directions. | - | cad | vertex(json)!,first(json)!,second(json)! | mutates=false | undoable=false"#,
    r#"cad.measure.area | Area | cad | command | cad | rect | The **area** an entity encloses. | - | cad | entity(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.measure.length | Length | cad | command | cad | ruler | The **length** of an entity's path. | - | cad | entity(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.measure.radius | Radius | cad | command | cad | circle | The **radius** of an entity. | - | cad | entity(node-ref)! | mutates=false | undoable=false"#,
    r#"cad.table.add | Add | cad | command | cad | table | **Add** a **table** to a block. | - | cad | block(node-ref)!,name(text)!,at(json)!,rows(number)!,columns(number)! | mutates=true | undoable=true"#,
    r#"cad.table.delete.row | Row | cad | command | cad | trash | **Delete** a **row** of a table. | - | cad | table(node-ref)!,row(number)! | mutates=true | undoable=true"#,
    r#"cad.table.insert.row | Row | cad | command | cad | plus | **Insert** a **row** into a table. | - | cad | table(node-ref)!,row(number)? | mutates=true | undoable=true"#,
    r#"cad.table.merge | Merge | cad | command | cad | merge | **Merge** the cells of a table into one. | - | cad | table(node-ref)!,row(number)!,column(number)!,rows(number)!,columns(number)! | mutates=true | undoable=true"#,
    r#"cad.table.set | Set | cad | command | cad | text | **Write** the **text** of a cell. | - | cad | table(node-ref)!,row(number)!,column(number)!,text(text)! | mutates=true | undoable=true"#,
    r#"cad.table.unmerge | Unmerge | cad | command | cad | unmerge | **Unmerge** the cells of a table. | - | cad | table(node-ref)!,row(number)!,column(number)! | mutates=true | undoable=true"#,
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
