//! The **vector battery's registry**, pinned.
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

#![cfg(feature = "vector")]

use craftkit_sdk::command::CommandSpec;

/// The commands of the vector battery, as the registry held them.
///
/// **The snapshot**: taken before the conversion, from the registry.
const EXPECTED: &[&str] = &[
    r#"vector.colour.convert | Convert | vector | command | vector | list | **Convert** a colour from one space into another. | - | vector | colour(text)!,from(text)!,to(text)! | mutates=false | undoable=false"#,
    r#"vector.colour.gamut | Gamut | vector | command | vector | list | **Say** whether a colour is inside the **gamut** of a space. | - | vector | colour(text)!,from(text)!,to(text)!,threshold(number)? | mutates=false | undoable=false"#,
    r#"vector.ellipse | Ellipse | vector | command | vector | ellipse | Put an ellipse in the document. | - | Create | x(number)!,y(number)!,width(number)!,height(number)!,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.export.eps | Eps | vector | command | vector | file | The **EPS file** of the drawing. | - | vector | path(text)! | mutates=true | undoable=true"#,
    r#"vector.export.svg | Svg | vector | command | vector | file | **Write** a drawing as an **SVG file**. | - | vector | path(text)! | mutates=false | undoable=false"#,
    r#"vector.gradient.read | Read | vector | command | vector | paint | **Read** the gradient of a path. | - | vector | path(node-ref)! | mutates=false | undoable=false"#,
    r#"vector.gradient.stop.add | Add | vector | command | vector | plus | **Add** a stop to a gradient. | - | vector | path(node-ref)!,offset(number)!,colour(text)!,opacity(number)? | mutates=true | undoable=true"#,
    r#"vector.gradient.stop.remove | Remove | vector | command | vector | minus | **Take away** a stop of a gradient. | - | vector | path(node-ref)!,stop(number)! | mutates=true | undoable=true"#,
    r#"vector.gradient.stop.set | Set | vector | command | vector | pencil | **Change** a stop of a gradient. | - | vector | path(node-ref)!,stop(number)!,offset(number)?,colour(text)?,opacity(number)? | mutates=true | undoable=true"#,
    r#"vector.import.svg | Svg | vector | command | vector | file | **Read** an **SVG file** into the document. | - | vector | path(text)!,name(text)? | mutates=true | undoable=true"#,
    r#"vector.line | Line | vector | command | vector | line | A **line**: a contour of two points. | - | Create | x1(number)!,y1(number)!,x2(number)!,y2(number)!,name(text)?,stroke(text)? | mutates=true | undoable=true"#,
    r#"vector.path.anchor.add | Add | vector | command | vector | path | **Add an anchor** to a path: a segment of a contour is split in two. | - | Path | node(node-ref)!,subpath(number)?,segment(number)!,t(number)? | mutates=true | undoable=true"#,
    r#"vector.path.anchor.remove | Remove | vector | command | vector | path | Take an **anchor** away. | - | Path | node(node-ref)!,subpath(number)?,anchor(number)! | mutates=true | undoable=true"#,
    r#"vector.path.anchors.convert | Convert | vector | command | vector | path | Turn points of a path into smooth points or into corner points. | - | Path/Points | path(node-ref)!,contour(number)?,points(json)!,to(text)! | mutates=true | undoable=true"#,
    r#"vector.path.anchors.middle | Middle | vector | command | vector | path | Put a point in the **middle of every side** of a path. | - | Path/Anchors | path(node-ref)! | mutates=true | undoable=true"#,
    r#"vector.path.anchors.set | Set | vector | command | vector | path | Set the **anchors** of a path. | - | Path/Anchors | path(node-ref)!,subpaths(json)! | mutates=true | undoable=true"#,
    r#"vector.path.average | Average | vector | command | vector | path | **Average** the points of a path. | - | Path/Points | path(node-ref)!,points(text)!,axis(text)? | mutates=true | undoable=true"#,
    r#"vector.path.blend | Blend | vector | command | vector | paint | Set the **blend mode** of a path. | - | Path/Paint | path(node-ref)!,mode(text)! | mutates=true | undoable=true"#,
    r#"vector.path.blob | Blob | vector | command | vector | path | Paint a brush stroke onto a shape. | - | Path/Paint | path(node-ref)!,points(json)!,size(number)?,fidelity(number)? | mutates=true | undoable=true"#,
    r#"vector.path.cap | Cap | vector | command | vector | path | Say how the **ends** of a path's line are finished. | - | Path/Paint | path(node-ref)!,cap(text)! | mutates=true | undoable=true"#,
    r#"vector.path.close | Close | vector | command | vector | path | **Close** a path: its last open contour is closed. | - | Path | node(node-ref)! | mutates=true | undoable=true"#,
    r#"vector.path.corner | Corner | vector | command | vector | path | Make a point of a path a **corner**. | - | Path/Points | path(node-ref)!,contour(number)?,point(number)! | mutates=true | undoable=true"#,
    r#"vector.path.create | Create | vector | command | vector | path | Put a path in the document, made by hand. | - | Create | d(text)?,anchors(json)?,closed(bool)?,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.path.crop | Crop | vector | command | vector | path | **Crop** a path with another: the part of the path behind that lies inside the path in front. | - | Path/Combine | front(node-ref)?,back(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.cut | Cut | vector | command | vector | path | Cut a path at a **point**. | - | Path/Cut | path(node-ref)!,contour(number)?,point(number)! | mutates=true | undoable=true"#,
    r#"vector.path.cut.line | Line | vector | command | vector | path | Split a closed shape along a line. | - | Path/Cut by line | path(node-ref)!,from(json)!,to(json)!,name(text)? | mutates=true | undoable=true"#,
    r#"vector.path.cut.mirror | Mirror | vector | command | vector | path | Mirror a closed shape in an axis, and join it to its mirror. | - | Path/Mirror and cut | path(node-ref)!,axis(text)?,from(json)?,to(json)?,keep(text)? | mutates=true | undoable=true"#,
    r#"vector.path.cut.rect | Rect | vector | command | vector | path | Cut a closed shape to a rectangle. | - | Path/Cut by rectangle | path(node-ref)!,rect(json)?,from(json)?,to(json)?,name(text)? | mutates=true | undoable=true"#,
    r#"vector.path.dash | Dash | vector | command | vector | path | **Dash** the line of a path. | - | Path/Paint | path(node-ref)!,pattern(json)?,offset(number)? | mutates=true | undoable=true"#,
    r#"vector.path.divide | Divide | vector | command | vector | path | **Divide** two paths: every **face** of them. | - | Path/Combine | front(node-ref)?,back(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.erase | Erase | vector | command | vector | path | Take away the parts of a path that are **under a drag**. | - | Path/Erase | path(node-ref)!,points(json)!,width(number)?,name(text)? | mutates=true | undoable=true"#,
    r#"vector.path.erase.region | Region | vector | command | vector | path | Take the area under a shaped tip away. | - | Path/Erase | path(node-ref)!,points(json)!,size(number)?,angle(number)?,roundness(number)?,name(text)? | mutates=true | undoable=true"#,
    r#"vector.path.exclude | Exclude | vector | command | vector | path | Keep what is in one path and not in the other. | - | Path/Combine | first(node-ref)?,second(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.fill | Fill | vector | command | vector | paint | Set the **fill** of a path. | - | Path/Paint | path(node-ref)!,colour(text)?,none(bool)? | mutates=true | undoable=true"#,
    r#"vector.path.freehand | Freehand | vector | command | vector | path | Draw a **stroke by hand** in the document. | - | Create | points(json)!,fidelity(number)?,fit(bool)?,closed(bool)?,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.path.gradient | Gradient | vector | command | vector | paint | Fill a path with a **gradient**. | - | Path/Paint | path(node-ref)!,kind(text)!,stops(json)!,from(json)?,to(json)?,centre(json)?,radius(number)? | mutates=true | undoable=true"#,
    r#"vector.path.grid | Grid | vector | command | vector | plus | Make a **grid** of copies of a path. | - | Path/Grid | path(node-ref)!,columns(number)!,rows(number)!,step_x(number)!,step_y(number)! | mutates=true | undoable=true"#,
    r#"vector.path.handle | Handle | vector | command | vector | path | Set the **handles** of a point. | - | Path/Points | path(node-ref)!,contour(number)?,point(number)!,c1(json)?,c2(json)? | mutates=true | undoable=true"#,
    r#"vector.path.intersect | Intersect | vector | command | vector | path | Keep what both paths cover. | - | Path/Combine | first(node-ref)?,second(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.join | Join | vector | command | vector | path | Say how the **corners** of a path's line meet. | - | Path/Paint | path(node-ref)!,join(text)! | mutates=true | undoable=true"#,
    r#"vector.path.join.ends | Ends | vector | command | vector | path | Join two open contours of a path into one. | - | Path/Points | path(node-ref)!,first(number)!,second(number)!,ends(json)?,to(text)? | mutates=true | undoable=true"#,
    r#"vector.path.join.scrub | Scrub | vector | command | vector | path | Join the ends that are near a drag. | - | Path/Points | path(node-ref)!,points(json)!,tolerance(number)? | mutates=true | undoable=true"#,
    r#"vector.path.knife | Knife | vector | command | vector | path | Cut a closed shape along a run of points. | - | Path/Cut | path(node-ref)!,points(json)!,name(text)? | mutates=true | undoable=true"#,
    r#"vector.path.merge | Merge | vector | command | vector | path | **Merge** two paths: the parts that are the same are joined. | - | Path/Combine | front(node-ref)?,back(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.mirror | Mirror | vector | command | vector | plus | Make a **mirrored** copy of a path. | - | Path/Mirror | path(node-ref)!,from_x(number)!,from_y(number)!,to_x(number)!,to_y(number)! | mutates=true | undoable=true"#,
    r#"vector.path.move | Move | vector | command | vector | path | The points of a contour are the point it starts at and the point each segment ends at, counting from zero. The handles of the curves move with the point they belong to, as they do in the tree: an anchor carries its two handles. | - | Path/Points | path(node-ref)!,contour(number)?,point(number)!,to(json)! | mutates=true | undoable=true"#,
    r#"vector.path.move.points | Points | vector | command | vector | path | Move many points of a path by one delta. | - | Path/Points | path(node-ref)!,contour(number)?,points(json)!,dx(number)!,dy(number)! | mutates=true | undoable=true"#,
    r#"vector.path.offset | Offset | vector | command | vector | path | Push a path out, or in. | - | Path | node(node-ref)!,distance(number)! | mutates=true | undoable=true"#,
    r#"vector.path.opacity | Opacity | vector | command | vector | paint | Put the **opacity** of a path. | - | vector | path(node-ref)!,opacity(number)! | mutates=true | undoable=true"#,
    r#"vector.path.order | Order | vector | command | vector | list | Put a path **in front** of the others, or **behind** them. | - | Path/Order | path(node-ref)!,place(text)! | mutates=true | undoable=true"#,
    r#"vector.path.outline | Outline | vector | command | vector | path | The **outline** of a path's line. | - | Path | path(node-ref)!,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.path.radial | Radial | vector | command | vector | plus | Make a **ring** of copies of a path. | - | Path/Radial | path(node-ref)!,count(number)!,centre_x(number)!,centre_y(number)! | mutates=true | undoable=true"#,
    r#"vector.path.reshape | Reshape | vector | command | vector | path | **Reshape** a path: grab it at a point and drag it. | - | Path/Reshape | path(node-ref)!,x(number)!,y(number)!,dx(number)!,dy(number)!,tol(number)?,radius(number)? | mutates=true | undoable=true"#,
    r#"vector.path.reverse | Reverse | vector | command | vector | path | **Turn** a path the other way round. | - | Path | node(node-ref)! | mutates=true | undoable=true"#,
    r#"vector.path.rule | Rule | vector | command | vector | path | Say which parts of a path are **inside**. | - | Path/Rule | path(node-ref)!,rule(text)! | mutates=true | undoable=true"#,
    r#"vector.path.simplify | Simplify | vector | command | vector | path | Take the points a path does not need. | - | Path | node(node-ref)!,tolerance(number)? | mutates=true | undoable=true"#,
    r#"vector.path.smooth | Smooth | vector | command | vector | path | Make a point of a path **smooth**. | - | Path/Points | path(node-ref)!,contour(number)?,point(number)! | mutates=true | undoable=true"#,
    r#"vector.path.spline | Spline | vector | command | vector | path | Put a **curve through the points** in the document. | - | Create | points(json)!,closed(bool)?,path(node-ref)?,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.path.stroke | Stroke | vector | command | vector | paint | Set the **stroke** of a path, or take it away. | - | Path/Paint | path(node-ref)!,colour(text)!,width(number)!,none(bool)? | mutates=true | undoable=true"#,
    r#"vector.path.subtract | Subtract | vector | command | vector | path | Take one path from another. | - | Path/Combine | first(node-ref)?,second(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.transform | Transform | vector | command | vector | path | Place a path with a **transform**. | - | Path/Transform | path(node-ref)!,matrix(json)? | mutates=true | undoable=true"#,
    r#"vector.path.trim | Trim | vector | command | vector | path | **Trim** two paths: the path behind loses the part the path in front was covering. | - | Path/Combine | front(node-ref)?,back(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.path.unite | Unite | vector | command | vector | path | Combine two paths into one. | - | Path/Combine | first(node-ref)?,second(node-ref)? | mutates=true | undoable=true"#,
    r#"vector.polygon | Polygon | vector | command | vector | polygon | Put a regular polygon in the document. | - | Create | x(number)!,y(number)!,radius(number)!,sides(number)!,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.rectangle | Rectangle | vector | command | vector | rect | Put a rectangle in the document. | - | Create | x(number)!,y(number)!,width(number)!,height(number)!,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.spiral | Spiral | vector | command | vector | spiral | A **spiral**: quarter arcs that get smaller and smaller. | - | Create | x(number)!,y(number)!,radius(number)!,decay(number)?,segments(number)?,clockwise(bool)?,name(text)?,fill(text)? | mutates=true | undoable=true"#,
    r#"vector.star | Star | vector | command | vector | star | A **star**: points around a centre, at two radii. | - | Create | x(number)!,y(number)!,radius(number)!,inner_radius(number)?,points(number)?,rotation(number)?,name(text)?,fill(text)? | mutates=true | undoable=true"#,
];

/// The commands of the vector battery, in the order of their ids.
fn the_battery() -> Vec<CommandSpec> {
    let ecosystem = craftkit_studio::ecosystem().expect("the ecosystem is not a mess");
    let mut commands: Vec<CommandSpec> =
        ecosystem.commands.all().iter().filter(|command| command.area == "vector").cloned().collect();
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
