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

The view is also the **same length** as the file, byte for byte: a block
comment's end, a string's contents and a character that is more than one byte
all keep their place. That is what lets a gate read the *shape* of a thing
from the code and its *contents* from the text — see the next section.

## And the tests are not the production code

A gate that reads the tree's source has to know **what part of it is
production**. A test is code, and a test that names a thing is a sentence
about it: a test that reads a limit to check that the limits are ordered
enforces nothing, and a test that calls a reader to prove the reader works
does not make the reader a third home.

So a gate whose subject is a *production* property cuts the file at its test
module first. The tree's `production_of` helper does it, and the gates that
read the tree's source all use it now: the limits gate, the raw reader's
gate, the reader list, the panic-boundary check, the JSON writer's check,
and the two that say the shell does not evaluate the document and does not
write it. The session's gate — which counts the session's public methods —
skips a method marked `#[cfg(test)]` instead, because a helper inside an
impl block is marked rather than put in a module.

The helper **removes** the test items rather than truncating the file at the
first one: production code after a test item is still production code, and a
cut that hid it would hide a limit used further down and a panic caught
further down. §153 found the truncating cut and fixed the helper; §156
found three gates that still had a cut of their own, and they were all
truncating.

It removes the test-only **method** too — the method an *indented*
`#[cfg(test)]` marks, whole and line by line, so the production methods
around it stay visible. §157: a name in a test-only method is a name in a
test, and three gates that read names were counting it.

Measured: the model's limit on the number of properties in a node had its one
production use replaced by a literal, and its only remaining mention was in a
test that compares the limits with each other. The gate that says every limit
is enforced said **green**. And a third file whose *test* called the raw
parameter reader — to prove the reader works — was reported as a third home
of a reader that is allowed two. And a test-only method in the session's impl
block was reported as "a public way to change the design is a second path".
And a test written outside a module that caught a panic was reported as an
undeclared panic boundary. And a production call to the pretty printer
*after* a test item was invisible to the gate that says the tree writes its
JSON with one writer, while an indented test attribute — which marks a
method inside an impl block — truncated that file and made the gate report
that the design file was not written with the tree's writer, which it is.
And a test written outside a module was reported by the two gates that say
the shell never evaluates the document and never writes it.

And the gate that **derives** the tree's parameter readers — the gate whose
whole point is that a reader is covered the day it is added — counted a test
helper as a reader. A test helper that shares a name with a production reader
is the worst shape: the gate believes the reader has no name argument of its
own, reads the *test's* body, and tells the commands that they read a
parameter the test invented.

## And a string is sometimes the subject

A gate that reads the tree's source can be **two gates at once**: one that
asks *what shape is this?* and one that asks *what does it say?*. The first
is about code and the second can be about a string.

The gate that says every parameter a command reads is declared is both. It
derives two lists out of the tree's source — the **readers** (the functions
that take a `&Params`) and the **commands** (the `pub static` specs) — and
then it compares what each command *declares* with what it *reads*. The
names on both sides are strings: a command's id, a parameter's name. There
the string **is** the subject, and a reader that blanks it has nothing left
to compare.

So the gate reads each file twice:

```rust
let text = std::fs::read_to_string(&path)?;
let code = code_lines_kept(&text);
```

- The **shape** comes from `code`: which `fn` is a function, which `pub
  static` is a command, which `ParamDoc::` is a declaration, which call is a
  call. A `fn` in a doc comment is not a function, and a `pub static` in a
  doc comment is not a command.
- The **contents** come from `text`: the command's id, the parameter's
  name.

The two views are the same length, so a position in one is a position in the
other: the gate finds *where* a thing is in the code and reads *what* it
says in the text.

Measured, against the gate on a copy of the tree: a doc comment that showed
a reader made the gate report two commands reading a parameter called
`ghost` — a parameter that exists only in the comment. A doc comment that
showed a whole command made it report a command that does not exist. The
controls, with the same prose and no signature, were green.

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
let code = code_lines_kept(production_of(&text));
```

And a gate that needs both the shape and the contents reads both views:

```rust
let Ok(text) = std::fs::read_to_string(&path) else { continue };
let code = code_lines_kept(&text);
// the shape from `code`, the contents from `text`, one position in both
```

## When NOT to use it

- **For prose.** A document is prose. Its backticks are text, and a gate that
  reads a document reads the document — the rule about strings is about Rust
  source, not about Markdown.
- **For the contents of a string.** Sometimes the string *is* the subject: a
  command id, a file name, a JSON payload. Read those with the file's own
  reader, not with this one. When a gate needs both — the shape from the code
  and the contents from the text — read the file twice and take one from each,
  by position.
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
- `a_limit_only_a_test_mentions_is_still_caught` — a limit that only a test
  mentions is not enforced.
- `a_test_that_calls_the_raw_reader_is_not_a_third_home` — a test is not a
  third home of the raw reader.
- `a_test_only_method_is_not_a_second_path_into_the_design` — a test's method
  is not a public path into the design.
- `a_test_that_catches_a_panic_is_not_a_boundary` — a test's catch is not a
  boundary.
- `production_code_after_a_test_item_is_still_production_code` — the cut
  removes the test items and keeps the rest.
- `a_test_helper_is_not_a_reader` — a test helper that takes a `&Params` is a
  function, and it is not a reader the commands use.
- `a_reader_that_stands_only_in_a_comment_is_not_a_reader` — a `fn` in a
  comment is not a function.
- `a_command_that_stands_only_in_a_comment_is_not_a_command` — a `pub static`
  in a comment is not a command.
- `the_code_view_is_the_same_length_as_the_file` — the two views are the
  same length, byte for byte, so a position in one is a position in the
  other.
- `a_writer_after_a_test_item_is_still_a_writer` — a cut that truncates the
  file at the first test item hides the production code after it.
- `an_indented_test_attribute_does_not_truncate_the_file` — an indented
  test attribute marks a method inside an impl block, not the end of the
  file.
- `a_test_outside_a_module_is_not_the_shell` — a test written outside a
  module is a test too.
- `a_test_only_method_is_not_a_reader` — a test-only method that takes a
  `&Params` is a function, and it is not a reader the commands use.
- `a_test_only_method_does_not_enforce_a_limit` — a test-only method that
  uses a limit does not enforce it.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — the other rule that is
  easy to break in one place and apply in another.
- [`../architecture.md`](../architecture.md#the-never-crash-contract) — why
  the tree has gates that read its own source at all.
