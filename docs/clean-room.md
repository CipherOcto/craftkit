# The clean-room rule

What this tree may not contain, and what the gate checks.

craftkit is copied into applications, so the rule written here travels with the
copy. It is written once, in one place, and `cargo xtask cleanroom` checks the
tree against it.

---

## 1. The rule

**Behaviour is designed from public specifications, general CAD conventions and
user feedback.** Nothing is copied from another project's source.

**No commercial product's material is used.** Not its code, icons, artwork,
help text, tooltips, templates, test data, measurements or generated files.

**And no commercial product is run.** No launching, scripting, automating,
screenshotting or measuring one, and no comparing our output against one. The
reason is not squeamishness: another product's terms of use forbid exactly
that, and a clean-room claim that rests on an act their terms forbid is not a
clean-room claim.

**And no copyleft project's source is read.** Not OpenCASCADE, not FreeCAD,
not SolveSpace, not CadQuery, not LibreCAD, not QCAD. Their code may not be
read, so it cannot be ported, and no part of this tree may carry their names
outside prose.

---

## 2. The products

`cargo xtask cleanroom` reads this table. A product's name may appear in
**prose** — in this file, in `AGENTS.md`, in a note about why something is
the way it is — and nowhere else in the tree: not in source, not in a test, not
in a fixture, not in a file name.

The names are written the way a person would not write them by accident:
`Fusion 360` and not `Fusion`, `Autodesk Inventor` and not `Inventor`. A
shorter name would match ordinary English and the gate would cry wolf.

| Product | Kind | Why it is here |
|---|---|---|
| AutoCAD | commercial | Autodesk's CAD product; the family competes with it |
| SolidWorks | commercial | Dassault's CAD product |
| Fusion 360 | commercial | Autodesk's CAD product |
| Onshape | commercial | PTC's browser CAD product |
| CATIA | commercial | Dassault's CAD product |
| Creo | commercial | PTC's CAD product |
| Solid Edge | commercial | Siemens' CAD product |
| SketchUp | commercial | Trimble's modelling product |
| Tinkercad | commercial | Autodesk's browser modelling product |
| Rhino 3D | commercial | McNeel's modelling product |
| ZBrush | commercial | Maxon's sculpting product |
| Autodesk Inventor | commercial | Autodesk's CAD product |
| Autodesk Maya | commercial | Autodesk's animation product |
| Adobe Illustrator | commercial | Adobe's vector product |
| Adobe Photoshop | commercial | Adobe's raster product |
| Adobe InDesign | commercial | Adobe's layout product |
| Adobe Premiere Pro | commercial | Adobe's video product |
| Adobe After Effects | commercial | Adobe's compositing product |
| Adobe Lightroom | commercial | Adobe's photo product |
| 3ds Max | commercial | Autodesk's modelling product |
| Cinema 4D | commercial | Maxon's animation product |
| Houdini | commercial | SideFX's effects product |
| OpenCASCADE | copyleft | LGPL; its source may not be read |
| FreeCAD | copyleft | LGPL; its source may not be read |
| SolveSpace | copyleft | GPL; its source may not be read |
| CadQuery | copyleft | LGPL; its source may not be read |
| LibreCAD | copyleft | GPL; its source may not be read |
| QCAD | copyleft | GPL; its source may not be read |
| OpenSCAD | copyleft | GPL; its source may not be read |
| Blender | copyleft | GPL; its source may not be read |

---

## 3. What the gate checks, and what it cannot

**It checks** that no name in that table appears, in any file of the tree that
is not prose, and that no file's *path* carries one either. A file called
`solidworks-export.dxf` is as much a copy of somebody else's work as a
function that writes one.

**It cannot check** that the tree contains nothing copied. Copied code does not
carry the product's name. A gate that checked that would be a gate that could
never pass, and a gate that can never pass is a rule written in prose.

So the gate is the part that can be mechanical, and the review is the rest.
The review is why this file exists: a reviewer who reads the table knows what
to look for, and a reviewer who has never been told does not.

---

## 4. The rule this tree was built from

Two repos in the family stated this rule, and they said opposite things about
whether the product may be *observed*:

- one allowed it — *"may be observed black-box: run it, use its UI with
  synthetic sessions, take screenshots by window id"* — with the screenshots
  kept out of the repository;
- the other forbade it, because the competitor's own terms forbid reverse
  engineering, *"including observing a program's inputs and outputs to recreate
  it"*.

The stricter one is the rule here. **The terms belong to the other party**, and
a rule that depends on what their licence happens to allow is a rule that
changes when their lawyers read it. §172 of `KNOWN-GAPS.md` has the
measurement.

---

## 5. Running it

```bash
cargo xtask cleanroom   # the tree against the table above
cargo xtask ci         # runs it with everything else
```

The gate fails with the file, the line and the name it found.
