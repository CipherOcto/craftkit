# A format change is one new file

## The problem

A `.craftkit` file has a version in its header (`"format": "craftkit/N"`). When
the saved shape changes — a new field, a renamed field, a node kind that must
now carry something — somebody has to bump `N`, teach the reader about the old
shape, and keep files written by every earlier build readable.

The obvious way to do that is one diff: edit `CURRENT`, edit the reader, edit
the writer. It reads well right up until two people do it at once. Then one of
them has to resolve a conflict *in the one file everybody edits*, and the losing
change silently disappears — not loudly, not in review, just gone from the
migration path and discovered by a user with an old file.

The other failure mode is guessing. A reader that reads a future file "best
effort" and writes it back has destroyed the parts it did not understand, and
the person finds out months later.

## The rule

**A change that alters what is saved adds exactly one file under
`crates/io/src/format/pending/`.** It does not edit `CURRENT`, it does not edit
the reader, and it does not touch an already-released version. The release folds
everything in `pending/` into the next version and empties it.

```rust
// crates/io/src/format/pending.rs
/// One entry per file in `format/pending/`.
///
/// Add a line here **and** a `mod <name>;` when you add a file. The
/// `the_pending_list_is_sorted` test keeps the table honest.
const FILES: &[&str] = &[];

/// Applying every pending step, oldest first.
///
/// `step` functions run in order, each seeing what the previous one produced.
pub fn apply(doc: &mut craftkit_model::Document, already: &[String]) { /* … */ }
```

## What a pending file contains

- A `//!` note saying what changes and why.
- Optionally `pub fn step(doc: &mut Document)`, rewriting a document written by
  the *previous* version into the shape this one expects. A change that needs no
  rewrite — a new optional field, say — says so and leaves `step` out.
- An entry in `FILES` **and** a `mod <name>;`.

`FILES` is sorted, and `names()` sorts again at runtime so that a mistake in the
table is caught by `the_pending_list_is_sorted` rather than by a release.
`every_pending_name_is_a_plausible_file_name` additionally asserts that a name
contains no `/` and is `snake_case` — which is what enforces "one change is one
file", and what makes the name a Rust module. The file, the `FILES` entry and
the `mod` are **the same word**: `mute_flag_to_node_field.rs`,
`"mute_flag_to_node_field"` and `mod mute_flag_to_node_field;`. §81 is the
story of three documents that said three different things here, and of the
result — a name Rust could compile was refused by the test, and a name the test
accepted could not be compiled.

## Why this is worth the ceremony

One file per change cannot conflict that way: two pending changes touch two
files, and the only shared state is one line in a sorted table and one `mod`.
The release becomes a `mv` instead of a judgement call about which migration
steps have already shipped.

The reader side has the matching rule, and it is not optional either:

```rust
// crates/io/src/format.rs — Format::check
/// Refuse a file this build cannot read correctly.
pub fn check(&self) -> Result<(), FormatError> {
    let v = self.version()?;
    if v > CURRENT {
        return Err(FormatError::TooNew { found: v });
    }
    Ok(())
}
```

A file from a newer craftkit is refused with a message that says so. Reading it
best-effort is how a design gets quietly rewritten.

## The recipe

1. Write `crates/io/src/format/pending/<snake-case-name>.rs` with the `//!`
   note, and `pub fn step(doc: &mut Document)` if a rewrite is needed.
2. Add the name to `FILES` in `pending.rs`, in sorted position, and add
   `mod <name>;` beside it.
3. Add the match arm in `pending::step_for`:
   ```rust
   match name {
       "example_name" => Some(example_name::step),
       _ => None,
   }
   ```
4. Do **not** touch `CURRENT`, the reader or the writer. They describe the
   format as released; the pending list describes everything after it.
5. Run the tests. `the_pending_list_is_sorted` and
   `every_pending_name_is_a_plausible_file_name` fail on a mistake in steps 1–3;
   `every_pending_file_is_wired_into_apply` fails if a listed change has no
   `step`; and the **build** fails if step 2's `mod` is missing, because the
   match arm in step 3 names the module. All four of them used to fail **on
   correct work** — §81.
6. Run `cargo xtask ci`. Its `lists` gate holds the same claim in the **code**:
   every change in `FILES` must have a `mod <name>;` and a `<name>::step` in the
   module the tree ships. A `mod` in a comment is not a declaration. §209.

At a release, everything in `pending/` is folded into a `v<N>/` folder, the
numbering advances, and the directory empties. `pending::apply` running an empty
list and returning the document untouched is the normal state between releases,
not an oversight.

**Who applies a step to whom.** A file records the changes it has had in its
header, and a reader applies the changes this build has that the file has not
had. Without that rule the reader applies every pending step to every file —
including the files it has just written — and the file does not read back as it
was written. §81 measured that: the round-trip tests caught it the moment a
real pending change existed.

**And the other direction.** A file that names a change this build has never
heard of was written by a build whose shape this build does not know — and the
reader used to read it anyway, dropping the parts of the document it did not
recognise without a word and writing them back without them on the next save.
A pending change does not bump the version, so the tag cannot catch this: the
file says `craftkit/1`, like every file this build writes. So the reader
refuses it, with `FormatError::UnknownChange` and the name of the change in
the message. §95.

A file from an **earlier** version may name a change that was folded into a
later version, and this build does not keep the folded names — so it reads
those files rather than refusing every design written before the last release.
What a folded change is stays the open question below.

**What happens to a folded step is still open.** The release script moves the
files into `pending/v<N>/` and empties the wiring, so nothing calls them
afterwards; the recipe describes a version directory the reader dispatches to
instead. The two do not agree yet, and `KNOWN-GAPS.md` §81 records the
question.

## When NOT to use it

- **For a fix that needs no migration.** Correcting the *reader* so an existing
  file is understood correctly is not a format change; the version did not move
  and no `step` is required.
- **To add a field with a default.** A new `#[serde(default)]` field is
  readable by every earlier build's reader without a rewrite. Say so in the
  `//!` note and leave `step` out.
- **To bundle two changes into one file.** Two logically separate migrations in
  one file are one file that will conflict with itself. The test cannot catch it
  because the name is still plausible.
- **To make an old build able to read a new file.** That is impossible and the
  format tag is what makes it fail cleanly instead of silently.

## See also

- [`../architecture.md`](../architecture.md#where-things-live) — where the
  reader, the writer and the pending list live.
- [`../design/`](../design/) — the format itself.
