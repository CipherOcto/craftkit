# Add an icon

**When you need this** — a command needs an icon the UI's set does not draw
yet. `every_command_has_an_icon_this_set_can_draw`
(in `crates/ui/src/icons.rs`) fails the moment you write `Icon("power")`
without adding `power` to `crates/ui/src/icons.rs`.

**Before you start**

```
cargo test -p craftkit-ui
```

is green, and you have looked at `ALL` in `crates/ui/src/icons.rs` and
confirmed the name you want is not already there under a different spelling.

---

## The rule that comes first

**`Icon("name")` is a name, not a drawing, and it stays that way.**

`craftkit_engine::Icon` is `pub struct Icon(pub &'static str)`
(the `Icon` struct in `crates/engine/src/command.rs`). The engine holds no image data, no SVG
path and no coordinates. The UI owns every shape. That is deliberate — it is
what lets everything below L5 be tested without a window, and what stops the
engine from growing a rendering concern.

So a command never draws an icon. It names one, and `crates/ui/src/icons.rs`
turns that name into strokes.

**And every shape in this project is drawn here, in code, from original
coordinates.** The rule at the top of `crates/ui/src/icons.rs` is the whole
policy: no icon, glyph or piece of artwork may come from another product,
commercial or free. Not a font, not a downloaded SVG, not a PNG traced from a
screenshot. That is the single most serious mistake an agent can make in this
repo, and "we only used a few" is exactly how it starts.

---

## Steps

### 1. Pick the name

Lower case ASCII and `-` only, one or more segments. `every_name_is_dotted_lower_case`
(`a_test`) enforces it. Existing names are
`arrows, broom, copy, curve, info, key, key-off, mute, pencil, play, plus,
range, redo, swap, trash, undo, value, warning`.

Name the *meaning*, not the shape. `mute`, not `speaker-with-slash`. The name
is what a reviewer reads in `crates/engine/src/commands.rs` and what the UI
matches on; a shape description makes both worse.

### 2. Add the name to `ALL`, in sorted position

`crates/ui/src/icons.rs`. `the_name_list_is_sorted_and_free_of_duplicates`
asserts the list is sorted and duplicate-free, so insert by
alphabetical position, not at the end:

```rust
pub const ALL: &[&str] = &[
    "arrows", "broom", "copy", "curve", "info", "key", "key-off", "mute", "pencil", "play",
    "plus", "range", "redo", "swap", "trash", "undo", "value", "warning",
];
```

### 3. Write the painter

Every icon is a method on `Painter` (`impl Painter` in `crates/ui/src/icons.rs`) built from
`line`, `path`, `circle`, `disc` and `rect`. Coordinates are in `0.0..=1.0`
relative to the icon's rect, mapped by `p`, so nothing can escape its box.
`mute` is the shortest useful example:

```rust
fn mute(&self) {
    // A speaker and a slash. The speaker is three segments so it has a
    // silhouette; the slash is what makes it "muted" rather than "volume".
    self.rect((0.24, 0.38), (0.36, 0.62), true);
    self.path(&[(0.36, 0.38), (0.52, 0.24), (0.52, 0.76), (0.36, 0.62)]);
    self.line((0.60, 0.34), (0.84, 0.66));
}
```

So, for a new `power`:

```rust
fn power(&self) {
    // A broken ring with a stem: the universal "on" mark, drawn as three arcs
    // and a line so it reads at 12 px.
    self.path(&[(0.50, 0.18), (0.28, 0.30), (0.24, 0.58), (0.38, 0.78),
                (0.62, 0.78), (0.76, 0.58), (0.72, 0.30), (0.50, 0.18)]);
    self.line((0.50, 0.14), (0.50, 0.46));
}
```

Three constraints:

- **One stroke weight.** Use `self.line`, `self.path`, `self.circle`,
  `self.rect`, `self.disc` — never a literal `Stroke` or `Color32`. `WEIGHT`
  (`Painter::stroke`) is the only place a weight is decided.
- **One grid.** Coordinates in `0.0..=1.0`, so a row of icons lines up at any
  size.
- **Comment the design decision.** The comment above `mute` explains why three
  segments and not one. A future reader comparing a candidate patch to yours
  needs to know what the shape was chosen to *say*, not just what it draws.

**Legible at 12 logical pixels.** That is the toolbar size. If the shape needs
a detail smaller than a stroke width to be read, it is the wrong shape.

### 4. Add the dispatch arm

`icons::draw` (`crates/ui/src/icons.rs`) is a `match` on the name with `_ =>
return false`:

```rust
"range" => p.range(),
"power" => p.power(),
_ => return false,
```

The fallback is what makes a typo an empty slot rather than a panic in the
middle of a frame — `an_unknown_icon_is_reported_not_drawn` asserts
exactly that. Do not remove the arm.

### 5. Point the command at it

`crates/engine/src/commands.rs`, in the spec:

```rust
icon: Icon("power"),
```

### 6. Run the gates

```
cargo test -p craftkit-ui -p craftkit-engine
cargo xtask ci
```

---

## The test you must add

The catalogue-wide check already exists:

- `every_command_has_an_icon_this_set_can_draw`
  (in `crates/ui/src/icons.rs`) — iterates `craftkit_engine::all()` and asserts
  `has(spec.icon.0)`.
- `every_icon_draws_inside_its_rect` (`every_icon_draws_inside_its_rect`) — runs the
  whole set through a headless `egui::Context` and asserts each name draws.

**Add a test only for the part they cannot see: that your shape is actually
legible, and that the arms and the list agree.** The existing tests prove the
name is reachable and the rect is non-empty; they do not prove you drew a
power symbol rather than a scribble.

```rust
#[test]
fn a_name_in_all_is_reachable_from_draw() {
    // The failure this catches: `ALL` lists a name but the `draw` match has no
    // arm for it, so `has` says yes and the button is blank.
    let ctx = egui::Context::default();
    let results = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ALL.iter()
                .map(|name| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    draw(ui, name, rect, Color32::WHITE)
                })
                .collect::<Vec<_>>()
        })
    });
    for (name, drawn) in ALL.iter().zip(results) {
        assert!(drawn, "`{name}` is listed but `draw` has no arm for it");
    }
}
```

Then **look at it**. Draw the set at 12 px in the toolbar and read the row. If
you cannot tell which icon is which at that size, the shape is wrong and no
test will tell you so.

---

## The failure this recipe is designed to prevent

### Adding the name but not the arm

`has` is `ALL.contains(&name)`. It says nothing about `draw`. A name in `ALL`
with no `match` arm means:

- `every_command_has_an_icon_this_set_can_draw` passes.
- `every_icon_draws_inside_its_rect` fails — but only *after* you have already
  pushed the command and told someone the button works.

It is the same class of bug as a declared-but-unread command parameter: the
declaration is what is tested, the use is what runs. Hence the explicit
agreement test above.

### Reaching for artwork that is not ours

This is the failure the recipe exists to prevent, and it is the worst mistake
available in this repo. It looks like a time-saver — a nice SVG, an icon font
glyph, a shape off a screenshot of another CAD product — and it is not
recoverable by editing the diff, because the licence and the provenance are the
problem.

The practical rules:

- **Draw it.** Every icon here is five to ten `line`/`path` calls in
  `0.0..=1.0`. If a shape is genuinely hard to draw, choose a different meaning
  that is easy.
- **Never screenshot, trace or copy a shape from another product's interface.**
  A 12-pixel toolbar icon is still someone's design.
- **No icon font.** A glyph is artwork, and every glyph in a font was designed
  by someone.
- **Any file asset must be original or openly licensed, and must have a row in
  the attribution list.** There is no commit in this project's history that
  should need fixing later for this.

If you are unsure whether a shape is too close to another product's, pick a
different shape. There is no cost to choosing again and no cost to a slow icon
PR; the cost is asymmetric.

### Making the engine aware of icons

The tempting shortcut is to store a path in `Icon` so both sides agree by
construction. That moves rendering into L4, makes `craftkit-engine` depend on
drawing concepts it should not have, and is the first step towards the engine
knowing what a window is. `Icon(pub &'static str)` is the whole design.
