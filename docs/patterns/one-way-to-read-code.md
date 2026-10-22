# One way to read code

## The problem

The tree's gates read the tree's own source. One checks that every limit in
the model's table is enforced somewhere; another checks that every module path
a document names is a module the tree has; another checks that the test a
document says is in a file really is in it. Each of them opens a `.rs` file and
asks what is **in** it.

A source file is mostly not code. It is comments, string literals and char
literals, and each of those can hold the name of a thing the tree is supposed
to have:

```rust
// The tree's own tests fabricate source files, in strings.
const FABRICATED: &str = "fn the_ghost_test() {}";
```

A reader that does not know what a string is reads that as a definition. A
document's claim that a test is in a file is then satisfied by a sentence
*about* a test, and the gate that exists to catch a false claim is happy.

The tree makes this worse on purpose: a gate that reads source has to be
tested, and the test fabricates a source file — **in a string**. A reader of
the text cannot tell a fabricated file from a real one.

Measured, against the gates themselves, on a copy of the tree:

| Case | What was done | What the gate said |
|---|---|---|
| A | the claimed test stood only inside a string | **green** |
| B | the claimed test stood only inside a comment | **green** |
| C | the claimed module stood only inside a string | **green** |
| D | the limit's only use stood only inside a string | **green** |
| E | a block comment in front of a constant | the gate named **line 284** for a constant on **line 293** |

A to D are false greens: the thing the gate checks was not in the code at all,
and the gate was satisfied by text. E is a wrong line: the reader dropped the
block comment's newlines, so every line after it moved, and the gate that
reports a line reported the wrong one.

## The rule

**A gate that reads the tree's source reads the code, and there is one way to
do it.** The one way is `code_lines_kept`, in `xtask/src/lists.rs`. It takes a
file's source and gives back the same source with the comments, the string
contents and the char literals blanked out — and with **every line kept**.

Four properties, each of which was a defect first:

- **The comments go.** A comment is a sentence about the code, and a sentence
  is not a definition.
- **The string contents go.** A mention in a string is not a call, a
  constant, a module or a test. The quotes stay, as spaces, so that what is
  left is still the file's shape.
- **The lines stay.** A gate that names a line has to name the **file's** line.
  Dropping a block comment's newlines moved everything after it, and a string
  written across two lines — the way `rustfmt` leaves one — lost its newline
  too. The tree's own `crates/io/src/format.rs` has three such strings.
- **A char literal goes, and a lifetime does not.** `'{'` is a brace that is
  not a brace, and the scanner that reads a type's members counts braces. `'a`
  is a lifetime, and treating it as a literal would swallow the rest of the
  file.

There is one reader, and that is the point. The rule was written three times
before — as three different helpers — and the gates that read a *symbol* or a
*module* used none of them. A rule written three times is three chances to
forget it, and the tree forgot it in three places at once.

## In the code

The reader, `xtask/src/lists.rs`:

```rust
fn code_lines_kept(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_block = false;
    let mut in_line = false;
    // ... the scan itself ...
    out
}
```

And the call sites. A gate that reads a source file does this and nothing else:

```rust
let Ok(text) = std::fs::read_to_string(&path) else { continue };
let code = code_lines_kept(&text);
```

## When NOT to use it

- **For prose.** A document is prose. Its backticks are text, and a gate that
  reads a document reads the document — the rule about strings is about Rust
  source, not about Markdown.
- **For the contents of a string.** Sometimes the string *is* the subject: a
  command id, a file name, a JSON payload. Read those with the file's own
  reader, not with this one.
- **To find where a string ends.** Two other scanners in the tree skip strings
  rather than blanking them — one finds the bracket that matches another, one
  reads a call's arguments. They want to know *where the string is*, not what
  the code around it says. They are not readers of code and they are not
  alternatives to this one.

## The checks that hold it

- `the_tree_has_one_code_reader` — counts the readers in the tree's own build
  tool, and there is exactly one.
- `a_symbol_that_stands_only_in_a_string_is_not_a_symbol_of_the_tree`, and the
  three beside it — each one takes the rule out of one gate and checks that
  the gate notices.
- `a_gate_that_names_a_line_names_the_file_s_own_line` — the line a gate
  reports is the line in the file.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — the other rule that is
  easy to break in one place and apply in another.
- [`../architecture.md`](../architecture.md#the-never-crash-contract) — why
  the tree has gates that read its own source at all.
