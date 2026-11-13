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
    r##"vector.colour.convert | Convert | vector | command | vector | list | **Convert** a colour from one space into another. | - | vector | colour(text)! help="The colour: a hex text, `#rrggbb`.",from(text)! help="The space the colour is in.",to(text)! help="The space the colour is put into." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"vector.colour.gamut | Gamut | vector | command | vector | list | **Say** whether a colour is inside the **gamut** of a space. | - | vector | colour(text)! help="The colour: a hex text, `#rrggbb`.",from(text)! help="The space the colour is in.",to(text)! help="The space whose gamut is tested.",threshold(number)? help="The threshold of the measure: the tree's own is four." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"vector.ellipse | Ellipse | vector | command | vector | ellipse | Put an ellipse in the document. | - | Create | x(number)! help="Its centre, across.",y(number)! help="Its centre, down.",width(number)! help="How wide it is, across the centre.",height(number)! help="How tall it is, down the centre.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.export.eps | Eps | vector | command | vector | file | The **EPS file** of the drawing. | - | vector | path(text)! help="The file to write." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.export.svg | Svg | vector | command | vector | file | **Write** a drawing as an **SVG file**. | - | vector | path(text)! help="Where to write the file." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"vector.gradient.read | Read | vector | command | vector | paint | **Read** the gradient of a path. | - | vector | path(node-ref)! help="The path." | mutates=false | undoable=false | journal=true | example=false"##,
    r##"vector.gradient.stop.add | Add | vector | command | vector | plus | **Add** a stop to a gradient. | - | vector | path(node-ref)! help="The path.",offset(number)! help="Where the stop is along the gradient: from zero to one.",colour(text)! help="The colour of the stop.",opacity(number)? help="How solid the stop is: from zero to one. The default is one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.gradient.stop.remove | Remove | vector | command | vector | minus | **Take away** a stop of a gradient. | - | vector | path(node-ref)! help="The path.",stop(number)! help="The stop that goes: its number in the list." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.gradient.stop.set | Set | vector | command | vector | pencil | **Change** a stop of a gradient. | - | vector | path(node-ref)! help="The path.",stop(number)! help="The stop to change: its number in the list.",offset(number)? help="Where the stop is along the gradient: from zero to one. It stays where it is when it is not said.",colour(text)? help="The colour of the stop. It stays what it is when it is not said.",opacity(number)? help="How solid the stop is: from zero to one. It stays what it is when it is not said." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.import.svg | Svg | vector | command | vector | file | **Read** an **SVG file** into the document. | - | vector | path(text)! help="The file to read.",name(text)? help="The name to give the paths." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.line | Line | vector | command | vector | line | A **line**: a contour of two points. | - | Create | x1(number)! help="Where it starts, across.",y1(number)! help="Where it starts, down.",x2(number)! help="Where it ends, across.",y2(number)! help="Where it ends, down.",name(text)? help="What to call it.",stroke(text)? help="The colour it is stroked with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.anchor.add | Add | vector | command | vector | path | **Add an anchor** to a path: a segment of a contour is split in two. | - | Path | node(node-ref)! help="The path.",subpath(number)? help="Which contour of the path.",segment(number)! help="Which segment of the contour.",t(number)? help="Where along the segment the anchor goes: from zero to one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.anchor.remove | Remove | vector | command | vector | path | Take an **anchor** away. | - | Path | node(node-ref)! help="The path.",subpath(number)? help="Which contour of the path.",anchor(number)! help="Which anchor of the contour." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.anchors.convert | Convert | vector | command | vector | path | Turn points of a path into smooth points or into corner points. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",points(json)! help="The points, counting from zero: a list of whole numbers.",to(text)! help="What the points become: `smooth` or `corner`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.anchors.middle | Middle | vector | command | vector | path | Put a point in the **middle of every side** of a path. | - | Path/Anchors | path(node-ref)! help="The path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.anchors.set | Set | vector | command | vector | path | Set the **anchors** of a path. | - | Path/Anchors | path(node-ref)! help="The path.",subpaths(json)! help="The contours of the path: each a list of anchors and whether it is closed." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.average | Average | vector | command | vector | path | **Average** the points of a path. | - | Path/Points | path(node-ref)! help="The path.",points(text)! help="The points to average, one per line, each `contour:point`.",axis(text)? help="The axis: `horizontal`, `vertical` or `both`. The default is `both`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.blend | Blend | vector | command | vector | paint | Set the **blend mode** of a path. | - | Path/Paint | path(node-ref)! help="The path.",mode(text)! help="The mode: `Normal`, `Multiply`, `Screen`, `Overlay` and the rest." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.blob | Blob | vector | command | vector | path | Paint a brush stroke onto a shape. | - | Path/Paint | path(node-ref)! help="The path the brush paints on.",points(json)! help="The brush's stroke: a run of points, `[[x, y], …]`.",size(number)? help="How wide the brush is. 10 if it is not given.",fidelity(number)? help="The fidelity the brush's stroke is fitted at." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.cap | Cap | vector | command | vector | path | Say how the **ends** of a path's line are finished. | - | Path/Paint | path(node-ref)! help="The path.",cap(text)! help="The cap: `butt`, `round` or `square`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.close | Close | vector | command | vector | path | **Close** a path: its last open contour is closed. | - | Path | node(node-ref)! help="The path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.corner | Corner | vector | command | vector | path | Make a point of a path a **corner**. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",point(number)! help="Which point of the contour, counting from zero." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.create | Create | vector | command | vector | path | Put a path in the document, made by hand. | - | Create | d(text)? help="The path data, in the SVG spelling: the moves, the lines and the curves of the path. An **arc** is refused.",anchors(json)? help="The anchors of the path: a list of `{x, y}`, each with an optional `in` and `out` handle, each a pair of numbers.",closed(bool)? help="Whether the path is closed.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.crop | Crop | vector | command | vector | path | **Crop** a path with another: the part of the path behind that lies inside the path in front. | - | Path/Combine | front(node-ref)? help="The path in front.",back(node-ref)? help="The path behind." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.cut | Cut | vector | command | vector | path | Cut a path at a **point**. | - | Path/Cut | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",point(number)! help="The point of the contour to cut at, counting from zero." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.cut.line | Line | vector | command | vector | path | Split a closed shape along a line. | - | Path/Cut by line | path(node-ref)! help="The path.",from(json)! help="The line's **first** point, `{\"x\": …, \"y\": …}` or `[x, y]`.",to(json)! help="The line's **second** point.",name(text)? help="What to call the pieces." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.cut.mirror | Mirror | vector | command | vector | path | Mirror a closed shape in an axis, and join it to its mirror. | - | Path/Mirror and cut | path(node-ref)! help="The path.",axis(text)? help="The axis: `vertical` (the default), `horizontal` or `free`.",from(json)? help="A point on the axis — where a vertical or horizontal axis runs.",to(json)? help="The other point of the axis, when it is `free`.",keep(text)? help="The side that is kept: `left`, `right`, `top` or `bottom`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.cut.rect | Rect | vector | command | vector | path | Cut a closed shape to a rectangle. | - | Path/Cut by rectangle | path(node-ref)! help="The path.",rect(json)? help="The rectangle: `[x, y, width, height]`.",from(json)? help="A **corner** of the rectangle, when it is not given as four numbers.",to(json)? help="The **other** corner.",name(text)? help="What to call the shape that is left." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.dash | Dash | vector | command | vector | path | **Dash** the line of a path. | - | Path/Paint | path(node-ref)! help="The path.",pattern(json)? help="The pattern: a dash, a gap, a dash, a gap — the lengths in the world's units. Without it the line is whole.",offset(number)? help="Where in the pattern the line starts." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.divide | Divide | vector | command | vector | path | **Divide** two paths: every **face** of them. | - | Path/Combine | front(node-ref)? help="The path in front.",back(node-ref)? help="The path behind." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.erase | Erase | vector | command | vector | path | Take away the parts of a path that are **under a drag**. | - | Path/Erase | path(node-ref)! help="The path.",points(json)! help="The drag: a run of points, `[[x, y], …]`.",width(number)? help="How wide the eraser is. 8 if it is not given.",name(text)? help="What to call the paths that are left." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.erase.region | Region | vector | command | vector | path | Take the area under a shaped tip away. | - | Path/Erase | path(node-ref)! help="The path.",points(json)! help="The drag: a run of points, `[[x, y], …]`.",size(number)? help="How long the tip's long axis is. 10 if it is not given.",angle(number)? help="The angle of the tip's long axis, in degrees. 0 if it is not given.",roundness(number)? help="How round the tip is, in percent of its long axis. 100 if it is not given.",name(text)? help="What to call the paths that are left." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.exclude | Exclude | vector | command | vector | path | Keep what is in one path and not in the other. | - | Path/Combine | first(node-ref)? help="The first path.",second(node-ref)? help="The second path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.fill | Fill | vector | command | vector | paint | Set the **fill** of a path. | - | Path/Paint | path(node-ref)! help="The path.",colour(text)? help="The colour, as `#rrggbb`.",none(bool)? help="Take the fill away instead: the tree's **None** paint." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.freehand | Freehand | vector | command | vector | path | Draw a **stroke by hand** in the document. | - | Create | points(json)! help="The samples of the stroke: `[[x, y], …]` or `[{x, y}, …]`.",fidelity(number)? help="The **fidelity** the stroke is fitted at: how far the stroke may be from the samples. 1.5 if it is not given.",fit(bool)? help="Whether the stroke is fitted at all.",closed(bool)? help="Whether the stroke is closed.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.gradient | Gradient | vector | command | vector | paint | Fill a path with a **gradient**. | - | Path/Paint | path(node-ref)! help="The path to fill with the gradient.",kind(text)! help="`linear` — along a line — or `radial` — out from a centre.",stops(json)! help="The stops: a list of `{\"offset\": 0, \"colour\": \"#rrggbb\"}`.",from(json)? help="Where a **linear** gradient starts: `{\"x\": 0, \"y\": 0}`.",to(json)? help="Where a **linear** gradient ends.",centre(json)? help="Where a **radial** gradient is centred.",radius(number)? help="How far a **radial** gradient reaches." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.grid | Grid | vector | command | vector | plus | Make a **grid** of copies of a path. | - | Path/Grid | path(node-ref)! help="The path to copy.",columns(number)! help="How many copies across, the original among them.",rows(number)! help="How many copies down, the original among them.",step_x(number)! help="How far apart they are, across.",step_y(number)! help="How far apart they are, down." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.handle | Handle | vector | command | vector | path | Set the **handles** of a point. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",point(number)! help="Which point of the contour, counting from zero.",c1(json)? help="The handle the curve leaves the point by.",c2(json)? help="The handle the curve arrives at the point by." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.intersect | Intersect | vector | command | vector | path | Keep what both paths cover. | - | Path/Combine | first(node-ref)? help="The first path.",second(node-ref)? help="The second path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.join | Join | vector | command | vector | path | Say how the **corners** of a path's line meet. | - | Path/Paint | path(node-ref)! help="The path.",join(text)! help="The join: `miter`, `round` or `bevel`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.join.ends | Ends | vector | command | vector | path | Join two open contours of a path into one. | - | Path/Points | path(node-ref)! help="The path.",first(number)! help="Which contour of the path, counting from zero.",second(number)! help="The other contour, counting from zero.",ends(json)? help="Which end of each contour is joined: `[\"first\"|\"last\", \"first\"|\"last\"]`.",to(text)? help="The joined point: `corner` or `smooth`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.join.scrub | Scrub | vector | command | vector | path | Join the ends that are near a drag. | - | Path/Points | path(node-ref)! help="The path.",points(json)! help="The scrub: a run of points, `[[x, y], …]`.",tolerance(number)? help="How near the ends have to be. 12 if it is not given." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.knife | Knife | vector | command | vector | path | Cut a closed shape along a run of points. | - | Path/Cut | path(node-ref)! help="The path.",points(json)! help="The knife's run: a list of points, `[[x, y], …]`.",name(text)? help="What to call the pieces." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.merge | Merge | vector | command | vector | path | **Merge** two paths: the parts that are the same are joined. | - | Path/Combine | front(node-ref)? help="The path in front.",back(node-ref)? help="The path behind." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.mirror | Mirror | vector | command | vector | plus | Make a **mirrored** copy of a path. | - | Path/Mirror | path(node-ref)! help="The path to copy.",from_x(number)! help="Where the line starts, across.",from_y(number)! help="Where the line starts, down.",to_x(number)! help="Where the line ends, across.",to_y(number)! help="Where the line ends, down." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.move | Move | vector | command | vector | path | The points of a contour are the point it starts at and the point each segment ends at, counting from zero. The handles of the curves move with the point they belong to, as they do in the tree: an anchor carries its two handles. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",point(number)! help="Which point of the contour, counting from zero.",to(json)! help="Where the point goes: a point, `{\"x\": …, \"y\": …}`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.move.points | Points | vector | command | vector | path | Move many points of a path by one delta. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",points(json)! help="The points, counting from zero: a list of whole numbers.",dx(number)! help="How far across the points go.",dy(number)! help="How far down the points go." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.offset | Offset | vector | command | vector | path | Push a path out, or in. | - | Path | node(node-ref)! help="The path.",distance(number)! help="How far to push it." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.opacity | Opacity | vector | command | vector | paint | Put the **opacity** of a path. | - | vector | path(node-ref)! help="The path.",opacity(number)! help="The opacity, as a percentage: from zero to a hundred." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.order | Order | vector | command | vector | list | Put a path **in front** of the others, or **behind** them. | - | Path/Order | path(node-ref)! help="The path.",place(text)! help="`front` or `back`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.outline | Outline | vector | command | vector | path | The **outline** of a path's line. | - | Path | path(node-ref)! help="The path whose line is outlined.",name(text)? help="What to call the new path.",fill(text)? help="The colour of the new path, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.radial | Radial | vector | command | vector | plus | Make a **ring** of copies of a path. | - | Path/Radial | path(node-ref)! help="The path to copy.",count(number)! help="How many copies there are, the original among them.",centre_x(number)! help="Where the centre is, across.",centre_y(number)! help="Where the centre is, down." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.reshape | Reshape | vector | command | vector | path | **Reshape** a path: grab it at a point and drag it. | - | Path/Reshape | path(node-ref)! help="The path.",x(number)! help="Where the path is grabbed: the x of the point.",y(number)! help="Where the path is grabbed: the y of the point.",dx(number)! help="How far the grab is dragged: the x.",dy(number)! help="How far the grab is dragged: the y.",tol(number)? help="How near an anchor must be for it to be the one that is grabbed. The tree's own default is three.",radius(number)? help="How far the anchors around follow, measured along the path. The default is half the size of the contour, and it is never less than one." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.reverse | Reverse | vector | command | vector | path | **Turn** a path the other way round. | - | Path | node(node-ref)! help="The path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.rule | Rule | vector | command | vector | path | Say which parts of a path are **inside**. | - | Path/Rule | path(node-ref)! help="The path.",rule(text)! help="The rule: `nonzero` or `evenodd`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.simplify | Simplify | vector | command | vector | path | Take the points a path does not need. | - | Path | node(node-ref)! help="The path.",tolerance(number)? help="How far the result may be from the original." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.smooth | Smooth | vector | command | vector | path | Make a point of a path **smooth**. | - | Path/Points | path(node-ref)! help="The path.",contour(number)? help="Which contour of the path, counting from zero. The first one if it is left out.",point(number)! help="Which point of the contour, counting from zero." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.spline | Spline | vector | command | vector | path | Put a **curve through the points** in the document. | - | Create | points(json)! help="The points the curve goes through: `{x, y}`, each with an optional `corner`, or a pair of numbers `[x, y]`.",closed(bool)? help="Whether the curve is closed.",path(node-ref)? help="The path to curve, if there is one already.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.stroke | Stroke | vector | command | vector | paint | Set the **stroke** of a path, or take it away. | - | Path/Paint | path(node-ref)! help="The path.",colour(text)! help="The colour of the line, as `#rrggbb`.",width(number)! help="How wide the line is.",none(bool)? help="Take the line away instead." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.subtract | Subtract | vector | command | vector | path | Take one path from another. | - | Path/Combine | first(node-ref)? help="The path to take from.",second(node-ref)? help="The path to take away." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.transform | Transform | vector | command | vector | path | Place a path with a **transform**. | - | Path/Transform | path(node-ref)! help="The path.",matrix(json)? help="The six numbers of the matrix." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.trim | Trim | vector | command | vector | path | **Trim** two paths: the path behind loses the part the path in front was covering. | - | Path/Combine | front(node-ref)? help="The path in front.",back(node-ref)? help="The path behind." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.path.unite | Unite | vector | command | vector | path | Combine two paths into one. | - | Path/Combine | first(node-ref)? help="The first path.",second(node-ref)? help="The second path." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.polygon | Polygon | vector | command | vector | polygon | Put a regular polygon in the document. | - | Create | x(number)! help="Its centre, across.",y(number)! help="Its centre, down.",radius(number)! help="How far its corners are from the centre.",sides(number)! help="How many corners it has.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.rectangle | Rectangle | vector | command | vector | rect | Put a rectangle in the document. | - | Create | x(number)! help="Where it starts, across.",y(number)! help="Where it starts, down.",width(number)! help="How wide it is.",height(number)! help="How tall it is.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=true"##,
    r##"vector.spiral | Spiral | vector | command | vector | spiral | A **spiral**: quarter arcs that get smaller and smaller. | - | Create | x(number)! help="Its centre, across.",y(number)! help="Its centre, down.",radius(number)! help="How far out it reaches.",decay(number)? help="How much of the radius is left after each quarter turn, in percent.",segments(number)? help="How many quarter turns it makes.",clockwise(bool)? help="Whether it winds clockwise.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
    r##"vector.star | Star | vector | command | vector | star | A **star**: points around a centre, at two radii. | - | Create | x(number)! help="Its centre, across.",y(number)! help="Its centre, down.",radius(number)! help="How far its points are from the centre.",inner_radius(number)? help="How far its inner corners are from the centre.",points(number)? help="How many points it has.",rotation(number)? help="How far it is turned, in degrees.",name(text)? help="What to call it.",fill(text)? help="The colour it is filled with, as `#rrggbb`." | mutates=true | undoable=true | journal=true | example=false"##,
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
