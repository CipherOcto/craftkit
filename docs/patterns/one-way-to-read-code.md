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

Five properties, each of which was a defect first:

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
- **A raw string is a string.** `r"…"`, `r#"…"#` and `br#"…"#` end at their
  own delimiter, not at the first quote inside them, and a backslash inside
  one is a backslash and not an escape. A reader that ends a raw string at
  the first quote inside it reads the text after that quote as code and the
  rest of the file as string contents — and then a limit nobody enforces
  looks enforced, and a writer four lines below the raw string is not there
  any more. §158.

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

And it finds the **end** of a test item by matching its braces in the code
view — for a test-only method since §157, and for a top-level item since
§160. The end of an item used to be found in the *text*, by looking for a
`}` in the first column, and a `}` inside a string is not a brace: the tree's
own `xtask/src/web.rs` has a raw string with one, the cut ended there, and
**eighteen test functions** after it were read as production code. A cut that
finds the end of a thing the way it finds the beginning of it: in the code,
where a brace is a brace.

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

Three more gates are both, and had the shape half wrong: the registry's
command ids, the control channel's wire names and the control channel's
methods. A **name** is a string and there a string *is* the subject — but the
*shape* around the name is code, and a comment shows that shape perfectly.
A command that stands only inside a block comment made the registry one
command longer and told two documents that they were wrong about how many
commands there are; a method that stands only inside a line comment made
the gate believe the channel had a method it does not have; and a wire name
that stands only inside a block comment made a document's example resolve to
a method that does not exist, so the gate that says an example must resolve
was satisfied by a comment. §159.

**Pair the two views by line number, not by position in a list.** A
comment-only line is blank in the code view and not in the text, so the two
lists of *lines* are not the same length — and a gate that zips them by
index pairs the wrong lines. The two views have the same number of lines, so
a line number is a position in both.

## And the audit finds the gates that forgot

The rule above is easy to state and easy to forget, so it has to be checked
rather than remembered. §159 listed **every** gate that opens a Rust file —
twenty of them — and said for each whether it reads the code view or the
text. Sixteen did; **four still read the text**:

| Gate | What it was reading the text for |
|---|---|
| `area_variants` | the `Area` enum's variants, counted out of the text |
| `attributed_code_is_in_the_file` | a document's claim, checked against the file's text |
| `check_roots` | the deny attribute every crate root must carry |
| `mod_groups` | the sorted `mod` lines |

They are the same question in four places — *is this line code, or is it a
sentence about code?* — and §161 measured all four. Two were **false
negatives** (a crate root whose deny attribute stood only in a string was
accepted; a document's claim satisfied by a mention inside a string was
accepted) and two were **false positives** on the tree's own files: the
tree's `xtask/src/lib.rs` was reported as unsorted because a `pub mod` line
inside a raw string counted as a module, and the tree's
`docs/patterns/everything-is-a-command.md` was reported as saying the wrong
number of areas because a variant inside a block comment counted as one.

The list is the point. §159's audit is what made it possible to ask the
question of the four, and the four are the four the audit had already named.

## And a claim and a file are in the same clause

A gate that reads **prose** has the same problem in a different place: prose
is made of sentences, and a sentence can hold more than one claim.

The gate that says the code a document claims is in a file reads two
backticked spans out of a paragraph — the claim, and the `*.rs` path — and
pairs them. The first version paired them in **either** direction, on
proximity alone: two spans forty characters apart were a claim and a file,
whether the document had said so or not.

The tree's own sentence in `skills/craftkit-editing/SKILL.md` is the
counter-example:

> the grammar is in the module doc of `crates/model/src/path.rs`; the listing
> is `PropGroup::flatten` in `crates/model/src/property.rs`

One claim, in two clauses. The gate made it **two**, and reported that
`path.rs` does not have a `flatten` in it — a file that never claimed to.
It was green before, because the claim was checked against the file's
*text* and `path.rs` has a doc comment that mentions `flatten` in passing:
fixing the reader removed the luck and left the pairing standing there in
the open.

The rule is the one §39's rule already uses for test names: **the claim comes
first and the file follows it**, the way "the `X` in `f.rs`" reads. One rule
for the tree.

And the report has to name the **claim**. The gate built the file's code
view into a variable called `code`, which shadowed the claim it was
reporting about, and the message printed nineteen thousand characters of the
file instead of the eighteen characters of the claim. A report nobody can
read is a report nobody reads.

## And a name is a whole name

Reading the right part of a file is only half of it. A gate that asks "is
this name in this file?" has to know **where the name ends**, and the two
gates that ask it did not:

- the claim gate asked `code.contains(name)`, and `observe` has `serve`
  inside it;
- the symbol gate asked `code.contains("fn " + name)`, and
  `fn the_ghost_test_two` has `fn the_ghost_test` inside it.

Both accepted a document that was wrong. The rule is the same as the one
the reader obeys, applied to the name instead of the file: **a name is a
whole name**, so the byte before it and the byte after it must both be
something that is not part of one.

The tree already had that rule, in the same file: `has_word`, used by the
gate that derives the tree's parameter readers. The two gates above now ask
it, and the pattern holds — one helper, one rule.

```rust
fn has_word(code: &str, name: &str) -> bool {
    // the byte before and the byte after must not be part of an identifier
}
```

The bytes are enough: an identifier's bytes are ASCII letters, digits and
`_`, and a multi-byte character's bytes are all above that range, so a
character is a boundary for free. It works for a **path** as well, because
`:` and `::` are not identifier bytes. §162.

**And the same question has one answer.** §162 fixed the two gates that
asked it. §164 went looking for the rest of them and found four more — the
limits gate, the byte-bounds gate, the memory-budgets gate and the JSON
writer's gate — each with its own substring test and each fooled by a
longer name. All four ask the tree's `has_word` now. A fifth gate asks a
*different* question the same way (is a signature a reader's?) and is
written down rather than changed: the two directions of its complaint pull
against each other, and that is an owner's call.

## And the shape of a thing is the code's shape

§159 audited every gate that opens a Rust file and wrote down, for each,
which view it reads. Its table has a row for the gate that says every field
the model's types have is hashed:

> `fingerprint_sees_every_field` — code view — the shape and the contents

**That row was wrong.** The gate read the tree's source as **text**, in two
places — the impl blocks and the hash methods, and the type's declaration and
its fields — and it had been that way since the gate was written.

The audit looked at the gate's **subject** and filled in the answer the
subject implies. The fingerprint's source *is* code, so the row said "code
view", and nobody looked at the call that read it. A gate's subject is not
its reader: the question is not "is my subject code?" but "am I reading it
as code?".

Measured, against the real gate on a copy of the tree:

| Case | What was done | What the gate said |
|---|---|---|
| A | an `impl Ghost {` inside a raw string | **RED** — a hash for a type the tree does not have |
| B | a `}` inside a raw string, ending the impl block | **green** — the hash had stopped reading `Node`'s field `name`, and the gate said nothing |
| C | a `pub struct Node` inside a raw string | **RED** — a field the model does not have, while the field it does have went unchecked |

Case B is the one that matters. The gate exists because of §80: the
fingerprint is the evaluator's cache key, so a field the hash forgets is a
**silent wrong answer**, and when §80 removed the keyframe's easing from the
hash, every test in the workspace stayed green. A false negative in *this*
gate is the exact failure the gate was built for, reported as a green
build.

The gate needs no text view at all. Everything it looks for is a **shape**
(an impl block, a hash method, a type's declaration, where the type's fields
end) or a **name in the code** (a field's name, a `self.field` read), and
neither is ever a mention that should count. The reader blanks the string
*contents* and the comments and leaves the names alone, so both reads now
go through it:

```rust
let code = code_lines_kept(&source);
let lines: Vec<String> = code.lines().map(|l| l.to_string()).collect();
```

And the lesson is about the audit as much as about the gate: **an audit of
twenty gates is twenty claims, and nothing checked them.** The list found
four gates in §161 and a fifth here, and it was wrong about one of the other
fifteen.

## And the audit's question is per read

§165 found one gate that read the tree's source as text. The audit's
question — *is this line code, or is it a sentence about code?* — was then
asked of every gate in the list, and it found two more, plus a gate the list
had left out.

| Gate | The read that was still text | What it cost |
|---|---|---|
| `the_session_offers_no_second_path` | the `impl Session` block's **header** and its **end** | a public method after a `}` in a string went unchecked |
| `find_embedded` (the assets gate) | the **embed call** itself | a comment or a string that showed a call was reported as an embed |

The assets gate was **not in the list at all** — it opens every `.rs` file in
the tree — so the list was wrong about one row *and* missing a gate.

Two things came out of it.

**The question is per *read*, not per gate.** A gate that reads a file twice
reads it twice, and each read has to be on the right side of the line. The
fingerprint gate called the reader for one of its two reads, and the table
said "code view".

**A gate that reads a shape has to say where the shape ends.** The session
gate ended its block at the first `}` in the first column, on the reasoning
that "everything inside an `impl` is indented". That is a fact about how
people write Rust, used to answer a question about what the code *is*. The
braces answer the structure question, and they answer it in the code, where a
`}` inside a string is not a brace:

```rust
let code = code_lines_kept(source);
let start = code.find(header)?;
let Some(brace) = header.rfind('{') else { return None };
let close = matching(&code, start + brace, b'{', b'}')?;
let block = &source[start + header.len()..close];
```

The code view is the text, byte for byte, so the offsets are the same offsets
and the block is the same block in both.

And the assets gate reads both halves, because a **path** is a string and a
string is the subject there:

```rust
let code = crate::lists::code_lines_kept(&text);
for (number, (code_line, text_line)) in code.lines().zip(text.lines()).enumerate() {
    // "there is a call here" is code; "the path it names" is the text
}
```

That is why the reader had to become **public**: it was a private `fn` in
`lists.rs`, and the alternative was a second copy of the rule in
`assets.rs` — the thing the tree's one-reader test exists to stop.

## In the code

The reader, `xtask/src/lists.rs`:

```rust
pub fn code_lines_kept(source: &str) -> String {
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
- `a_raw_string_is_a_string` — the contents of a raw string are not code.
- `a_raw_string_ends_at_its_own_delimiter` — a raw string ends at its own
  delimiter, not at the first quote inside it.
- `a_backslash_in_a_raw_string_is_not_an_escape` — a backslash in a raw
  string is a backslash.
- `a_raw_string_without_hashes_is_a_raw_string` — `r"…"` is a raw string
  too, and the view keeps the file's length.
- `a_byte_string_is_a_string` — `b"…"` and `br"…"` are strings.
- `a_command_id_in_a_comment_is_not_a_command_id` — an `id:` in a comment is
  not a command.
- `a_wire_name_in_a_comment_is_not_a_wire_name` — an arm in a comment is not
  a wire name.
- `a_method_in_a_comment_is_not_a_control_method` — an arm in a comment is
  not a method.
- `a_brace_in_a_string_does_not_end_a_test_module` — a `}` inside a string
  does not end a test item.
- `a_deny_attribute_in_a_string_is_not_a_deny_attribute` — a deny attribute
  inside a string is not a deny attribute.
- `a_mod_line_in_a_string_is_not_a_module` — a `pub mod` line inside a raw
  string is not a module.
- `a_claim_satisfied_by_a_string_is_not_satisfied` — a mention inside a
  string does not satisfy a claim.
- `a_variant_in_a_comment_is_not_a_variant` — a variant inside a block
  comment is not a variant.
- `a_claim_after_a_clause_boundary_is_not_about_the_file_before_it` — a
  claim is about the file that follows it, not the one before it.
- `a_claim_satisfied_by_a_substring_is_not_satisfied` — a name inside a
  longer name does not satisfy a claim.
- `a_test_satisfied_by_a_longer_name_is_not_satisfied` — a test that is
  only the prefix of another test's name is not it.
- `an_empty_name_is_not_a_word` — an empty name is not a word, and the
  matcher does not spin on it.
- `a_limit_only_a_longer_name_uses_is_still_caught` — a limit used only
  inside a longer name is not enforced.
- `a_bound_only_a_longer_name_names_in_the_prose_is_not_named` — a bound
  named only inside a longer name is not named.
- `a_budget_that_names_only_a_longer_name_is_not_derived_from_the_model` —
  a budget that names the model's only inside a longer name is its own.
- `the_writer_is_not_a_path_that_starts_the_same_way` — the tree's writer is
  not a path that starts the same way.
- `an_impl_inside_a_string_is_not_an_impl_block` — an `impl` inside a string
  is not an impl block.
- `a_brace_inside_a_string_does_not_end_an_impl_block` — a `}` inside a
  string does not end an impl block.
- `a_struct_inside_a_string_is_not_the_struct` — a `struct` inside a string
  is not the type the tree has.
- `a_brace_inside_a_string_does_not_end_the_session_block` — a `}` inside a
  string does not end the session's impl block.
- `an_impl_inside_a_string_is_not_the_session_block` — an `impl Session {`
  inside a string is not the session's block.
- `an_embed_call_in_a_block_comment_is_not_an_embed` — an embed call inside
  a block comment is not an embed.
- `an_embed_call_in_a_string_is_not_an_embed` — an embed call inside a
  string is not an embed.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — the other rule that is
  easy to break in one place and apply in another.
- [`../architecture.md`](../architecture.md#the-never-crash-contract) — why
  the tree has gates that read its own source at all.
