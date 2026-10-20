# The `.craftkit` file, and the plan it evaluates to

Two things this document describes in full: the bytes on disk, and the value the
evaluator produces from them. Everything here was read out of the code and
checked by loading a hand-written file, so if it disagrees with
`crates/io/src/format.rs` or `crates/model/src/`, the code is right and this file
is out of date.

---

## The file

```json
{
  "format": "craftkit/1",
  "meta": { "written_by": "hand", "written_at": 0, "pending": [] },
  "document": {
    "meta": { "name": "Hand written", "fps": 24, "start": 0, "end": 127008000000, "comment": "" },
    "nodes": [
      {
        "id": 1,
        "name": "Start",
        "kind": "source",
        "props": {
          "id": "props",
          "label": "Properties",
          "nodes": [
            {
              "Prop": {
                "id": 1,
                "label": "amount",
                "value": { "t": "num", "v": 1.0 },
                "hint": { "Slider": { "min": 0.0, "max": 1.0 } },
                "keys": [],
                "locked": false
              }
            }
          ]
        },
        "muted": false
      }
    ]
  }
}
```

That is a complete, loadable file. `craftkit-cli eval` on it reports one node,
12 frames at 24 fps, and no diagnostics.

### The four shapes that surprise people

**`format` is a string, not a number.** `"craftkit/1"` — a tag plus a version.
A reader refuses a version it does not know (`FormatError::TooNew`) rather than
guessing, because a guessed read of a newer file silently corrupts a design.

**`value` is tagged, not bare.** Every value is `{"t": …, "v": …}`:

| `t` | `v` |
|---|---|
| `bool` | `true` |
| `num` | a finite number, at most 1 × 10⁹ in magnitude |
| `text` | a string, at most 65 536 bytes |
| `color` | `[r, g, b, a]` |
| `list` | an array of values, at most 4096 — and every value in it is checked too |
| `none` | — |

The reader holds every value to these limits, not only the constructors that
make them: a file is deserialized straight into the model, so the model checks
what it reads (§73). A file that carries a number a hundred million times past
the limit is **refused**, with the property's name in the message.

A bare `2.0` where a value belongs is the single most common mistake when writing
a file by hand, and the error says so: *"invalid type: floating point `2.0`,
expected adjacently tagged enum Value"*.

**The property tree is two enums deep.** `PropGroup` holds `nodes: Vec<PropNode>`,
and `PropNode` is externally tagged: a leaf is `{"Prop": { … }}` and a nested
group is `{"Group": { … }}`. Groups nest arbitrarily.

**`hint` is an externally tagged enum too.** `Number`, `Slider { min, max }`,
`Toggle`, `Text`, `Color`, `Scaled` — capitalised, as Rust's default serde
external tagging is. `"number"` is not a variant.

### The four default properties

Every node created through the engine gets these, from `Node::set_defaults`:

| `label` | type | meaning |
|---|---|---|
| `enabled` | `bool` | whether the node participates |
| `amount` | `num`, hint `Slider` 0..1 | the value this node carries |
| `tint` | `color` | how it is drawn |
| `label` | `text` | free text shown with it |

Their `id`s are 1..4. Ids matter: `Property::id` is stable across rename and
reorder, which is why it exists alongside the label.

---

## What is *not* in the file

- **The plan.** It is computed. Nothing about an evaluated result is saved, so
  two files that evaluate the same way are genuinely the same plan.
- **The id counter.** `Document::next_id` is `#[serde(skip)]` and is recomputed
  on load by `reseat_id_counter()`, which scans for the highest id actually
  present. A file therefore cannot make the next `node.add` collide with an
  existing node, and cannot hand out an id it has already used.
- **Anything derived.** No cached frames, no cached evaluations, no layout.
- **Where you were.** The playhead and the undo history are *session* state, not
  document state: a file records the range (`start`, `end`) but not the instant
  inside it, and it records no history at all. So a reopened design starts at its
  first frame with nothing to undo. This is a decision, and it is the reason
  `apps/craftkit-cli/tests/command_effects_survive_the_file.rs` lists
  `playback.seek`, `history.undo` and `history.redo` as deliberately absent
  rather than quietly leaving them out.

---

## Property paths

Commands and the UI address properties by **label**, not by index and not by id:

```text
amount                 the first property labelled `amount` among its siblings
amount#2               the second
trigger/lead           `lead`, inside the sibling group whose id is `trigger`
#3                     the third child of the root group
@7f3a                  by stable id
```

`PathSeg::Match(name, n)` counts **siblings**, not the whole tree. That matters:
a counter that ran across levels would emit `amount#2` for a property that has no
second sibling, and the path would resolve to nothing. `PropGroup::flatten` is
where this is enforced.

Paths are `/`-separated. An empty path means the node itself, which is how
`node.rename` and `node.mute` are written.

---

## Time

`start` and `end` are **`Tick`s, never seconds.**

```text
1 second  = 254 016 000 000 ticks
1 frame   = 10 584 000 000 ticks at 24 fps
5 seconds = 1 270 080 000 000
```

The number above (`end: 127008000000`) is **0.5 s** at 24 fps — 12 frames —
which is what the evaluation reported. It is an exact integer because a frame
boundary is exactly representable and "the same number twice" is a property worth
having. See
[`../patterns/exact-time-not-float-seconds.md`](../patterns/exact-time-not-float-seconds.md).

`fps` is read into a `FrameRate`, which refuses 0 — the ticks per frame divide
by it — and the model refuses a rate above `MAX_FRAME_RATE`: a document claiming
a million frames per second would otherwise overflow a tick conversion
silently. The same limit is what the `doc.range` command enforces, so a
document can never hold a rate no command can set. §89.

### Keyframes

A keyframe is a **time**, a **value** and an **easing**; a property's keyframe
list is **sorted**, **unique** and within `MAX_KEYS`.

The model keeps those rules when it *builds* a list. A file does not build
one — it is deserialized straight into the model — so the rules are **checked**
by `Keyframes::check` instead, and the reader refuses a file that breaks one:

- a keyframe's time outside the range a reply could carry (the same range the
  document's own `start` and `end` are checked against — §60);
- an easing the model refuses, such as a bezier whose `out_x` is outside
  `0..=1` or is not finite;
- a list that is not sorted by time, or that has two keys at the same time;
- more keys than `MAX_KEYS`.

The last two are not tidiness. An unsorted list does not sample to an untidy
answer — it samples to the **wrong one**: the key that is earliest in time is
never reached, and the animation starts at the later key's value. Measured:
keys at 0 and 500, sampled at tick 50, gave `1.0` where the sorted list gives
`1.9`. §91.

---

## Versions, and changing what is saved

`CURRENT` is 1 and nothing is pending. The rule for changing it:

> **A change that alters what is saved adds ONE file** in
> `crates/io/src/format/pending/`, with a `//!` note saying what changes and
> why, and a `pub fn step` if older designs must be rewritten.

Never edit `CURRENT`, never edit the reader to understand the old shape, never
touch a `v<N>/` folder. `scripts/fold-format-pending.sh` is the release step; it
moves the pending files into `v<N>/` and tells you the three edits that need
judgement.

`meta.pending` records the format changes the file **has had** — the ones that
had been applied to it when it was written. A reader applies the changes this
build has that the file has *not* had, so a file this build wrote is not
rewritten when it is read back. It is `#[serde(default)]`, because a file
written before the field existed must still load, and it has had none of the
changes.

The same list, read the other way, is what says whether this build understands
the file at all. A pending change does not bump the version — that is the whole
point of the scheme — so the version tag says nothing about the *shape*. A file
at the current version that names a change this build has never heard of was
written by a different build, and its document is in a shape this build does not
know: the parts of it this build does not recognise are dropped without a word,
and the next save writes the file back without them. The reader refuses such a
file (`FormatError::UnknownChange`), naming the change. §95.

A file from an **earlier** version may name a change that was folded into a
later version, and this build does not keep the folded names — so it reads
those files rather than refusing every design written before the last release.
What a folded change *is*, and whether the reader has to know the older shapes
by hand, is still open; `KNOWN-GAPS.md` §81 records it.

See [`../recipes/change-the-file-format.md`](../recipes/change-the-file-format.md).

---

## The plan

`craftkit-eval` walks the node chain and produces a `Plan`: one entry per node,
in document order, each with its evaluated value at the tick asked for. The
evaluator also keeps a **fingerprint** of each node, to decide whether the
node's state from the last evaluation can be reused.

Three properties matter, and they are the reason the evaluator lives where it
does:

- **Prefix reuse.** Asking for two nearby ticks re-evaluates only what the second
  tick's inputs actually changed. The trace says so: `0t at 0.000s — 3 reused, 1
  evaluated, 0 problem(s)`.
- **Diagnostics, not failures.** A node with no `amount` produces a warning and
  a default, not an error:

  ```json
  { "message": "has no `amount` property; using 1.0", "node": 1, "severity": "warning" }
  ```

  Diagnostics are capped at 256 entries so a hostile document cannot make the
  report unbounded.
- **A fingerprint that catches nothing moving.** It is FNV-1a over the node's
  *fields* — `Node::fingerprint` in `craftkit-model` — so two nodes that differ
  in anything, including a single keyframe, cannot share one. It used to hash
  the node's serialised form, which cannot drift but cost 52 ms on a node with
  200 000 keyframes: more than the evaluation the cache was meant to save. A
  hand-written hash has to be updated by hand when the model grows a field, and
  a forgotten field would be a silent wrong answer, so a test beside the hash
  changes each part of a node **on its own** and a gate reads the tree to see
  that every field the hash walks is a field it reads.
  `KNOWN-GAPS.md` §53 and §80.

Everything below `craftkit-eval` goes through `craftkit_kernel::guard`, which
turns a panic in code craftkit does not own into `KernelError::Internal` rather
than a crash.

---

## Reading one by hand

```bash
python3 -m json.tool plan.craftkit | less
craftkit-cli eval plan.craftkit          # what it evaluates to, as JSON
```

If `eval` refuses it, the error names what is wrong and where: a missing field
names the field, a bad variant lists the ones that exist. Those messages are
written for exactly this situation — somebody looking at a file they did not
write.