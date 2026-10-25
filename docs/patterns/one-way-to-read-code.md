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
do it.** The one way is `code_lines_kept`, in `xtask/src/tree.rs`. It takes a
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

**And a cut that knows what a test item is has to know what a test file
is.** The cut removes a file's *test items* — a `mod tests { … }` block, a
top-level `#[cfg(test)]` item, a test-only method — and Rust puts a crate's
integration tests in `tests/`, where each file is a crate of its own: a file
there is a test from its first line to its last, with no item in it to
remove. The tree had **fourteen** `#[test]` functions in
`apps/craftkit-cli/tests/` that the cut left in, and every gate that reads
the tree's source and cuts it read them as production code.

Measured, against the real gates on a copy of the tree: a `catch_unwind` in
a test file was reported as a boundary the pattern document does not
declare, and a limit whose only use was in a test file was reported as
**enforced**. Both are the gates answering wrongly, in opposite directions.

So the cut takes the file's **path** as well as its text:

```rust
fn is_a_test_file(path: &Path) -> bool {
    path.components().any(|part| part.as_os_str() == std::ffi::OsStr::new("tests"))
}
```

and asks it line by line, so the line count stays the file's. §167.

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
longer name. All four ask the tree's `has_word` now. A fifth gate asked a
*different* question the same way — *is a signature a reader's?* — and was
**written down rather than changed**, on the grounds that the two directions
of its complaint pull against each other.

**§208 measured that claim, and it was wrong.** The complaint that was
supposed to pull the other way needs a tree that **does not compile**, and the
gate asks the **shape** of the argument now. See below.

## And a question about an argument is a question about its shape

The fifth gate §164 found — the one that derives the tree's parameter
readers — asked *is this signature a reader's?* with a substring:

```rust
let takes_params = function.signature.contains(&format!("&{PARAMS_TYPE}"));
```

`&ParamsExtra` holds `&Params`, so the answer was yes. **Measured**, against
a fabricated tree, the gate reported a helper taking `&ParamsExtra<'_>` as a
reader of the params.

**And the substring was wrong in the other direction too.** `&mut Params` does
**not** hold `&Params`. **Measured**: the old check answered **false** for
`fn value_for(p: &mut Params, key: &str)`, so a reader that takes the params
*mutably* was not a reader as far as the gate was concerned.

| the signature | the old check said | the gate says now |
|---|---|---|
| `p: &ParamsExtra<'_>` | a reader | not a reader |
| `p: &mut Params` | **not** a reader | a reader |

A substring check is wrong in **both** directions, and a defect in one of them
hides the other: the tree has neither `ParamsExtra` nor a `&mut Params`
reader, so it measured only the first.

**The gate asks the shape now.** A reader takes a **reference** to the params
type, and the type is named as a whole word. `takes_the_params` splits each
argument at its colon, requires the type to start with `&` (optionally
`mut `), and matches `Params` with the tree's own `has_word`:

```rust
fn takes_the_params(signature: &str) -> bool {
    arguments_of(signature).iter().any(|argument| {
        let Some((_, ty)) = argument.split_once(':') else { return false };
        let ty = ty.trim();
        let Some(reference) = ty.strip_prefix('&') else { return false };
        let reference = reference.strip_prefix("mut ").unwrap_or(reference);
        has_word(reference, PARAMS_TYPE)
    })
}
```

**And the reason the gate was left alone was worth measuring.** The gate also
reports **parameters a command declares and never reads**, so a command that
called a function which is not a reader would be reported as declaring a
parameter it does not read. That complaint looked like the price of fixing
the substring. **Measured** — on a scratch file outside the tree, compiled
with `rustc` — the tree that produces it **does not compile**: Rust has no
implicit conversion from `&Params` to `&ParamsExtra`:

```
error[E0308]: mismatched types
   |
   |     let _ = value_for(&p, "name");
   |             --------- ^^ expected `&ParamsExtra<'_>`, found `&Params`
```

A false complaint that needs a tree that does not build is the **compiler's**
false complaint, not the gate's. §208.

**And the gate that bites is the unit test, not the lists gate.**
**Measured**: with the substring check back, `cargo xtask lists` still says
**green** — the tree has no `ParamsExtra`, so the lists gate reads the same
readers either way. A false negative that only a **fabricated** tree can show
cannot be caught by a gate that reads the **real** one; it needs a test that
fabricates one.

## And a crate does not read its own source

**A check that reads the tree's own source measures the file, and the file can
be fooled.** §209 found the second of the four places §207 measured. The io
crate's pending-format check read **its own file's text** and asked whether a
pending change was declared and applied:

```rust
let source = std::fs::read_to_string(
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/format/pending.rs"),
).expect("read pending.rs");
assert!(source.contains(&format!("mod {name};")));
assert!(source.contains(&format!("{name}::step")));
```

**Measured**, with the table holding `"probe"` and the whole wiring inside a
**comment**, the check **passed**. The file was not declared, so Rust never
compiled it and the migration never ran.

**And the crate already held both claims.** The crate's own runtime test asks
`step_for(name).is_some()` — a fact about the compiled code, and it caught the
same tree. And the **compiler** holds the other half: a missing `mod` does not
resolve, so the crate does not build.

So the text check was **redundant**: it held two claims that were already
held, in the one view that can be fooled. The fix was not to add a second
holder. It was to **drop the foolable one** and let the claim live where the
code is read:

```rust
let text = read_rust(&path, Subject::Production)?;
let code = code_lines_kept(&text);
```

**The names come from the text and the table from the code.** A name is a
string literal, and a string is the subject there; the table it is in is
found in the code, so a `FILES` that stands only inside a comment is not a
table. The two views are the same length, byte for byte, so a name in one is a
name in the other.

**And a claim the compiler holds is still a claim a mutation cannot bite.** A
mutation that breaks the build is reported as *a build that did not compile*,
not as a caught gate. The gate in the build tool is what turns the claim into
a check.

## And a claim about a running thing is held by the running thing

**A check on the spelling of a danger is not a check on the danger.** The
control channel is a JSON endpoint that runs engine commands. The tree's
check that keeps it local read every Rust file in the tree as **text** and
refused two spellings: `allow_remote: true` and `allow_remote = true`.

**Measured, a positional `true` walks straight past it.** The production call
site was `bind(options.port, options.allow_remote)?`, and `bind` took a plain
`bool`. Change the call site to `bind(options.port, true)?` and the channel
binds `0.0.0.0` — every interface — and **all 62** of the crate's tests
passed. A value with no name beside it is invisible to a check that looks for
a name.

**And the test was asking the wrong thing.** The crate's loopback test bound
with its **own** call, not the production one. So the path a user reaches had
no test at all.

The claim that matters is not *"nothing writes the flag"* — it is *"the
channel a user reaches is local"*, and that is a fact about the **running
server**. So the server now reports the **address** it bound, and a test
starts it the way a user reaches it:

```rust
let (port, server) = serve(session, options(0)).expect("binds");
let addr = server.addr();
assert!(addr.ip().is_loopback(), "the control channel bound {addr}, which is reachable from another machine");
```

**The port said nothing about the address.** A control channel on `0.0.0.0` is
reachable from another machine, and `port()` cannot tell the two apart.

**And a residual that is stated is still a gap.** The audit that fixed the
default wrote down that the `true` branch of `bind()` was never executed by a
test, and moved on. The gap was not that branch — it was that the production
path had **no test at all**, and the residual's words had pointed at the wrong
place.

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
let code = crate::tree::code_lines_kept(&text);
for (number, (code_line, text_line)) in code.lines().zip(text.lines()).enumerate() {
    // "there is a call here" is code; "the path it names" is the text
}
```

That is why the reader had to become **public**: it was a private `fn` in
`lists.rs`, and the alternative was a second copy of the rule in
`assets.rs` — the thing the tree's one-reader test exists to stop.

## And a gate says which view of the file it reads

**A gate that opens a Rust file says whether it is asking about the code the
tree ships or about the file as it is.** The two are different questions, and
the tree's answer is an **argument**, so it cannot be forgotten.

```rust
pub enum Subject {
    /// The code the tree ships: the file without its tests.
    Production,
    /// The file as it is, tests included.
    Everything,
}
```

`Subject::Production` cuts the file's tests — the cut `production_of` makes —
and `Subject::Everything` keeps them. A gate whose question is about the
**file** (how many lines it has, which lines a citation names, which test it
holds) reads it as it is; a gate whose question is about the **code** reads the
production view.

And there is **one way to open a Rust file**, and **one way to read anything
else**, and the second one **refuses** a Rust file:

```rust
pub fn read_text(path: impl AsRef<Path>) -> Result<String, std::io::Error>
```

So a gate cannot read a Rust file by accident: the reader says so, and a file
that is not Rust is read as it is.

**Measured at §204**, over the production part of the lists gate: **69** file
reads, **36** of them of a Rust file, made by **35** gates; **11** already cut at
the test module and **25** took the file as it is. The list is
`RUST_READING_GATES`, and it says which of the gates read the code the tree
ships (**28**) and which read the file as it is (**7**). §205 added three gates
to it — `find_embedded`, `inspect` and `the_anchors_are_in_the_tree_of`, all of
which read the file as it is — so it held **38**: **28** and **10**. §206 moved
the two registry gates to the code the tree ships, and it holds **38** now:
**30** and **8**.

**The list is a promise, so it is checked.**
`the_gates_that_read_rust_say_what_they_read` reads the lists gate's own
source and holds three things: that the tree has two ways to read a file and
no third; that every gate that opens a Rust file is in the list; and that the
subject at its call site is the one the list names.

**And the cut found a false claim.** The tree's own playbook says
"`Session::execute` does the rest" in the third column of one row of a table,
and the next row's second column names the file `crates/engine/src/commands.rs`
— a file that does not hold it.

The gate that checks a document's claims paired the two anyway, and reported
the method as living in `crates/engine/src/commands.rs`. It lives in
`crates/engine/src/session.rs`. The claim was **false**, and it was true only
because the file's **test module** said `execute` — fifty-four times, against
once in a doc comment, which the code view blanks. §204.

**And a table's columns are different things.** A table row is a breath of its
own: a claim in one row and a file in the next are not about each other, and
the columns of a table are not one sentence.

## And a claim about the tree is a claim about all of it

**A rule that says "the tree reads a file this way" is a claim about every
module of the tree, and a gate that checks it in one module has checked one
module.**

§204's gate said the tree had two ways to read a file and no third, and it
held the promise by counting `std::fs::read_to_string` in `xtask/src/lists.rs`
and expecting **two**. The claim was about *the tree*. The count was about one
module of the xtask's **fourteen**, and about **one spelling** of the read.

Measured at §205, against the xtask at §204:

| What was measured | What was found |
|---|---|
| raw reads in the xtask's production code | **17** — **13** spelled `std::fs::read_to_string`, **4** spelled `std::fs::read` |
| modules they stood in | **8** of the xtask's **14** |
| the clean-room gate's reads | **every file in the tree** — **135** files, **77** of them Rust — as bytes |
| the gate that checked the claim | **one** module, **one** spelling |
| the test that said "the tree reads code one way" | named **8** of the **14** modules |

So the promise was true for the module it was measured in and said nothing
about the other seven. `anchors.rs`, `cleanroom.rs`, `deny.rs`, `fuzz.rs`,
`main.rs` and `tree.rs` were never looked at by the test that named eight
files, and the four `std::fs::read` calls the gate could not see at all were
the fuzzer's seeds and the clean-room gate's bytes.

**A list of files is a list that goes stale.** The test named eight files
because someone wrote eight names down; a module added later was not in the
list, and the list is not a gate. The tree already had the answer for this
kind of thing: **one walk** — `tree::walk`, in `xtask/src/tree.rs` — and §170
put every gate's directory walk through it. The same shape fits the modules:
walk `xtask/src` and read every module the walk finds, rather than name them.

**A claim is wider than the thing it computes.** The gate was not wrong about
the module it read. It was wrong about the scope of the sentence it was
holding up, and a scope that is narrower than the sentence is the same defect
as a scope that is wider.

**And the raw read has two spellings.** `std::fs::read_to_string` and
`std::fs::read` are both ways to open a file, and a gate that counts one of
them is a gate that can be walked around with the other.

So the port did three things:

- **The readers came home.** `read_text`, `read_rust`, the cut
  (`production_of`) and the code reader (`code_lines_kept`) moved out of the
  gates module and into `xtask/src/tree.rs`, beside the walk. A reader is a
  way of seeing the tree and not a gate, and the walk was already there for
  the same reason. §170's shape, applied to the readers.
- **The gate walked the xtask.** It now reads **every** module under
  `xtask/src`, and it checks **both** spellings of a raw read. A raw read
  outside `xtask/src/tree.rs` is reported, with the module and the line.
- **The test walked the xtask too.** `the_tree_has_one_code_reader` no longer
  names eight files; it walks them, and it reads each one as the tree's own
  reader reads it.

**And the readers grew a third.** `read_bytes` reads a file that is not text —
the fuzzer's seeds, the clean-room gate's look at every byte of every file —
and it **refuses** a Rust file, the way `read_text` does. A Rust file is read
with a subject, so a gate cannot open one by accident, whichever reader it
reaches for.

**The claim is about the xtask, and it says so.** The tree's own product
crates read designs, tokens and crash reports with the standard library, and
they are not gates; a rule that forbade it would be a false claim. The
message names the xtask: *"the xtask reads a file through
`xtask/src/tree.rs` and nowhere else."* §204's lesson — a claim is wider than
the thing it computes — applies to the fix as well as to the defect.

## And a decision says why

**A list that says what a thing does, and not why, records an accident.**

`RUST_READING_GATES` names every gate that opens a Rust file and the subject
each one reads for. At §205 that was the whole of it: **38** gates, **28**
reading the code the tree ships and **10** reading the file as it is. A reader
who came to `("citation_drift", Subject::Everything)` saw a decision and could
not tell whether it was a decision or a leftover.

**And the subject does not imply the reason.** Two gates can read the same
view for opposite reasons, and the list cannot tell them apart.

So the list grew a third column — the reason — and writing the reasons down
found something the subject had been hiding:

| Gate | The subject said | What the reason says | Measured |
|---|---|---|---|
| `registry_command_ids` | the file as it is | the ids are the **shipped** registry's, and a command only a test declares is not one of them | the two views of the file are the same today — **23** ids and **23** `pub static` under either subject |
| `registry_commands` | the file as it is | the commands are the **shipped** registry's, and a command only a test declares is not one of them | the same |

Both gates now read the code the tree ships, and both say why. Their own
doc comments already said it — *"Every command id the **shipped registry**
defines"* — and the subject in the list disagreed with the gate's own words.

**And the filter that was supposed to tell the test commands apart never
fired.** `registry_command_ids` skipped every id that begins with `test.`,
and there are **none**: **23** ids before the filter and **23** after it, under
either subject. The filter was the residue of a rule that had nothing to do,
and a filter that cannot fire is not a decision — it is a habit.

**And the reasons are held up.** The gate checks that every row says why, and
the unit test checks that a reason is not *only* the subject's own phrase: a
reason may use it as one of its clauses, because that is how it explains the
choice, but it has to add something.

**And the tree's house style already had the shape.** `NOT_GATES` says of
`bundle` *"it writes files into the tree"*, and `SESSION_PUBLIC_METHODS`
gained `sample_recorded` **with its reason** when §200 added it. The list of
the gates that read Rust was the one list in the tree that recorded a
decision without it.

## And a question about code asked of text is answered by a sentence

**A check that asks what the code *is* reads the code.** The rule has a second
half that is easy to forget, and §207 found it: *a mention in a string is not
code* — and **a mention in a comment is not code either**, and a check that
reads the file's **text** cannot tell the difference.

The engine's test `the_trace_budget_is_derived_from_the_memory_budget` held
that the trace's byte budget is **derived** from the memory budget and not
written out. A derived number and a literal have the same value, so the test
could not see it from the value — it read the engine's own source and looked
for the names of the derivation on the constant's line.

**Measured**, the check was fooled by a sentence:

```rust
pub const MAX_TRACE_BYTES: usize = 256 * 1024; // MAX_TRACE_MEMORY_BYTES / craftkit_core::JSON_EXPANSION
```

The number was a literal, the derivation was a comment, and the test
**passed**. The tree's own gates for the file cap and the memory budgets have
read the **code** since §148 and §144, and say so: *"a cap that stands only
inside a string literal is not a cap."*

**And the check belongs where the reader is.** The tree has **one** code
reader and it lives in the build tool. A crate that cannot have it without
breaking the one-reader rule cannot ask a question about code — so the
question goes to the tool, and the crate keeps the half it *can* hold: the
**value**. §207 moved the trace budget's derivation to
`the_trace_budget_is_derived_from_its_memory_budget` in `xtask/src/lists.rs`,
and the engine's test now holds only that the two numbers agree.

**And a test that reads the tree's source measures the file, not the code.**
The tree has **four** places that read the tree's own source at run time, and
**every** file they read is a file the mutation suite rewrites. The file and
the code are the same file until something rewrites one of them.

## And there is one way to find the files

**A gate that walks the tree walks it one way.** The one way is `walk`, in
`xtask/src/tree.rs`. It takes a root, a directory to start in and a predicate,
and it gives back the files under that root the predicate accepts — and
nothing in the build output, nothing under a dot-directory, and nothing in
`target`, `.git`, `node_modules` or `dist`.

The rule exists because the copies had already drifted. §168 folded the
assets gate's two walks into this one and wrote that *"there is now one, in
`xtask/src/tree.rs`, and both gates use it"*. **That was false when it was
written**: the lists gate still had three of its own — `collect_manifests`,
`collect_markdown` and `collect_rust` — and they did not agree about what to
skip. `collect_manifests` did not ignore `dist`; the other two did.

So the gate that holds it down is `the_walk_has_one_home`: every recursive
directory walk in the tree's own build tool must be `tree::walk`. A **test**
is not a second home — a test that walks a temporary directory is testing the
walk, not holding down a rule about what a gate sees.

The rule and the copy live in the same file. `xtask/src/tree.rs` says where
the walk came from, which copies it replaced and what each of them skipped.
**A copy that forgets where it came from is a copy that drifts**, so the
record is in the file and not only in the history.
## And a rule that every caller needs has one home they can all reach

**A rule written once per place is not one rule.** It is N rules that happen
to agree today, and the place that needs it N+1st has no memory of the N
before it. §149's shape — a rule written three times is three chances to
forget it — and the tree's temporary folders were written **twenty-four**
times.

**The home goes where the callers can reach it.** §212 put the home for a
test's temporary folder in the **lowest layer**, `craftkit_core::temp`, and
not in the crate that happened to need it first. A home in one crate is a
home twenty-three crates cannot use, and a rule in a crate that cannot be
reached is a rule that is written again.

**And the gate says which lines are the callers.** The claim is *"a test does
not choose where its temporary folder lives"*, and the tree's own cut already
knows which lines are tests: `production_of` blanks a file's test items and
keeps the file's line numbering, so a line that is blank in the production
view and present in the file belongs to a test. **No list of test files, no
list of call sites** — the tree's one cut answers it.

**And a call is recognised by its shape.** `std::env::temp_dir()` takes **no
arguments**, and that is what tells it apart from the tree's own wrappers,
which take the tag: `temp_dir(tag)`, `scratch(tag)`. §208's lesson, in a
second place: a question about a call is a question about the **shape** of the
call.

**And a tree that cannot see another tree's home gets one of its own.** The
xtask cannot reach the core's home, because it depends on **nothing of the
tree's own** — on purpose, since a checker that cannot be built when the tree
does not compile is a checker that is silent exactly when it is needed. So
the honest statement of the rule is **one home per tree that can see it**,
and the second home is a **named** decision rather than an accident: a gate
holds up both, and says which is which.

**And a copy is proved by its own test.** The two trees cannot share a check
either, so each home carries its own test and its own mutations — and the
copy says where it came from, **in the file**: a copy that forgets where it
came from is a copy that drifts.


## And a file the tree writes goes through one home

**A rule that is written down once is not a rule that is obeyed once.** The
tree states how a file is written — *into a temporary name, then rename* — in
the crash report's own header, and it was written **three** times, in three
crates, with three different temporary names, while **two** of the tree's own
writers did not follow it at all. §215.

**The home goes where every writer can reach it.** `craftkit_core::atomic` is
in the **lowest layer**, so the io crate, the engine, the control channel and
the command line all reach it — the same shape as §212's temporary folder.

**And the home holds the whole rule, not half of it.** A temporary name
beside the target; the file created with the mode the caller asks for; the
mode **looked at** before the bytes go in, so a filesystem that does not keep
permissions is refused rather than trusted; the finished file renamed into
place; and the temporary removed if anything fails.

**And the caller says which kind of file it is.** `OwnerOnly` makes the file
`0600` from the moment it exists; `AsCreated` leaves it with the process's
own default. The design the user saves where they chose is `AsCreated`,
because a document the user put somewhere is the user's business — so the
decision is made at the call site, where the knowledge is.

**And a writer is recognised by its shape.** `std::fs::write`, `std::fs::copy`,
`std::fs::File::create`, `std::fs::OpenOptions::new` — four shapes of "this
line creates a file", and a gate that asks the shape catches a writer whatever
it is called. §208's lesson, in a third place.

**And the question is asked of the code the tree ships.** A test writes
fixtures, and a fixture is not a file the tree writes; so the gate reads the
**production** view, the tree's own cut, and a mention in a comment or in a
string is not a call.

**And a promise is proved by being broken.** The home's own test runs a child
process under a file-size limit and watches the file that was there survive —
and the child's output goes nowhere, because a limit on a file's size is a
limit on whatever else writes to that file, and a test that fails at random
is worse than no test.


## In the code

The reader, `xtask/src/tree.rs`:

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
let Ok(text) = read_rust(&path, Subject::Production) else { continue };
let code = code_lines_kept(&text);
```

And a gate that needs both the shape and the contents reads both views:

```rust
let Ok(text) = read_rust(&path, Subject::Production) else { continue };
let code = code_lines_kept(&text);
// the shape from `code`, the contents from `text`, one position in both
```

And a gate whose question is about the **file** rather than the code reads it
as it is:

```rust
let Ok(text) = read_rust(&path, Subject::Everything) else { continue };
let lines: Vec<&str> = text.lines().collect();
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
- `the_tree_has_one_walk` — counts the recursive directory walks in the tree's
  own build tool, and there is exactly one. §170.
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
- `the_gates_that_read_rust_say_what_they_read` — every gate that opens a
  Rust file is in the list, the subject at its call site is the one the list
  names, every row says **why** it reads that view, and the xtask reads a file
  through `xtask/src/tree.rs` and nowhere else. §204, extended by §205 and
  §206.
- `every_row_of_the_list_says_why` — a reason is there, and it is not only
  the subject's own phrase. §206.
- `a_type_that_starts_with_params_is_not_the_params_type` — the reader gate asks
  the **shape** of the argument, and `&ParamsExtra` is not the params type.
  §208.
- `a_pending_change_that_stands_only_in_a_comment_is_not_declared` — a pending
  change is declared and applied in the **code**, and a `mod` in a comment is
  not a declaration. §209.
- `the_server_binds_loopback_on_the_path_a_user_reaches` — the control channel
  a user reaches binds loopback, and the port alone does not say so. §210.
- `the_tree_has_one_home_for_reading_a_file` — the readers, the cut and the
  code reader all live in `xtask/src/tree.rs`. §205.
- `a_rust_file_is_read_with_a_subject_and_the_rest_as_it_is` — `read_text` and
  `read_bytes` refuse a Rust file, and `read_rust` asks which view it wants.
  §204, extended by §205.
- `a_bound_that_only_a_test_declares_is_not_a_bound_of_the_tree` — a constant
  in a test is not a bound of the tree.
- `a_constant_that_only_a_test_declares_is_not_the_codes_constant` — a test's
  own constant is not one of the tree's numbers.
- `a_claim_in_one_row_of_a_table_and_a_file_in_the_next_are_not_a_claim` — a
  table row is a breath of its own.
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
- `a_file_under_a_tests_directory_is_a_test_file` — a file under a `tests`
  directory is a test file from its first line to its last.
- `a_test_file_does_not_catch_a_panic_as_a_boundary` — a test that catches a
  panic is not a boundary.
- `the_tree_has_one_temporary_folder` — a test does not choose where its
  temporary folder lives; the tree's one home does, and the tree's own cut
  says which lines are tests. §212.
- `a_test_that_builds_its_folder_by_hand_is_caught` — the gate bites, and it
  is caught for one reason. §212.
- `a_test_that_uses_the_one_home_is_not_reported` — the gate is quiet when
  the tree is right, and a production use of the environment's folder is not
  a test's business. §212.
- `an_xtask_test_that_builds_its_folder_by_hand_is_caught` — the gate holds
  up the xtask's home as well as the core's, and names the right one. §213.
- `every_call_gets_a_folder_of_its_own` in the xtask — the xtask's home is
  proved by its own test, because two trees that cannot see each other cannot
  share a check. §213.
- `the_tree_writes_its_files_through_one_home` — a file the tree writes goes
  through one home, and the gate asks the *shape* of the writer. §215.
- `a_writer_outside_the_one_home_is_caught` — the gate bites, and it is
  caught for one reason. §215.
- `a_test_that_writes_a_fixture_is_not_reported` — a test's fixture is not a
  file the tree writes. §215.
- `the_tree_writes_its_files_through_the_one_home` — the gate is quiet when
  the tree is right. §215.
- `the_file_that_was_there_survives_a_write_that_fails` — the home's promise,
  proved by a child process that is stopped where the limit is. §215.

## See also

- [`one-trust-boundary.md`](one-trust-boundary.md) — the other rule that is
  easy to break in one place and apply in another.
- [`../architecture.md`](../architecture.md#the-never-crash-contract) — why
  the tree has gates that read its own source at all.
