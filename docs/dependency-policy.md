# The dependency policy

What this tree may depend on, and the gate that checks it.

A dependency is a supply-chain decision. It is not reviewed once and
forgotten: the licences, the bans, the sources and the advisories are read
out of `deny.toml` by `cargo xtask deny`, on every push, and a tree that does
not hold the policy is a tree that does not pass.

---

## 1. The rule

**Only the strong copyleft is refused.** Every licence that does not make
the binary copyleft is allowed: the permissive ones, the file-level copyleft
(`MPL-2.0`), the font licences, and the licence of the crates that arrive
with the UI toolkit's clipboard support. The strong copyleft — `GPL`, `LGPL`,
`AGPL` — is never linked into a binary this project ships. A copyleft tool
may still be *run* as an external program, the way a reference implementation
may be run to produce numbers this tree then checks itself against; it is
never vendored, linked or compiled in.

**The list is wide on purpose.** The owner's decision (§215): the tree would
rather widen the policy than narrow the work, and a licence that only exists
because one dependency happens to carry it is a rule with a name on it. So
there are **no per-crate exceptions** in `deny.toml` — every allowed licence
is allowed for every crate.

**Every dependency comes from the crates.io registry.** A git dependency has
to be named in `deny.toml` and say why. An unknown registry or an unknown git
source is refused.

**No version requirement is a wildcard.** `foo = "*"` is refused, and so is a
path dependency that carries no version — because a crate that publishes a
path dependency without a version can only publish it as `*`.

**An advisory that is not fixed is named, with a reason.** A crate the
authors have stopped maintaining is refused unless the tree says why it is
tolerated and when that will change.

---

## 2. The licences

`deny.toml` holds the list. This table says what each entry is for, because
a list of identifiers is not a rule — a reviewer needs to be able to ask
"why this one?" and get an answer.

| Licence | Why it is allowed |
|---|---|
| `MIT` | the most common permissive licence in the tree |
| `Apache-2.0` | permissive, and the one the tree itself is published under |
| `BSD-2-Clause` | permissive |
| `BSD-3-Clause` | permissive |
| `ISC` | permissive |
| `Zlib` | permissive |
| `Unicode-3.0` | permissive; it covers the Unicode data files in the text stack |
| `CC0-1.0` | a dedication to the public domain; permissive |
| `MPL-2.0` | file-level copyleft, weak enough to link: the modified files stay open, the binary does not become copyleft |
| `BSL-1.0` | the Boost Software License; permissive, and the licence of `clipboard-win` and `error-code`, which arrive with the UI toolkit's clipboard support |
| `OFL-1.1` | the SIL Open Font License; permissive, and the licence of the fonts the UI toolkit ships |
| `Ubuntu-font-1.0` | permissive, and the licence of the Ubuntu font the UI toolkit ships |

**The fonts are inside a dependency, not in this tree**, so the tree's own
asset rule does not reach them and `ATTRIBUTION.md` cannot cover them: they
are inside a crate from the registry, and that crate's licence expression
says what they are. That expression is `(MIT OR Apache-2.0) AND OFL-1.1 AND
Ubuntu-font-1.0` — byte-for-byte the same in every version of the toolkit
from 0.33.0 to 0.37.0 — so the two font licences are allowed for **every**
crate and not for the one that carries them today. §173, §215.

---

## 3. What the gate checks, and what it cannot

**It checks** the dependency graph against `deny.toml`: that every licence in
the graph is allowed, that no requirement is a wildcard, that every source is
the crates.io registry, and that no advisory is unaccounted for.

**It cannot check** that a licence was read. A licence identifier in a
manifest is a claim by the author, and the tool checks the claim. That is
what a supply-chain gate can do, and it is why the table above says what each
identifier is: a human decides whether the claim is good enough.

**It says "skipped" when the tool is not installed.** A gate that cannot run
is not a gate that passed. On a machine without `cargo-deny` the command
prints what is missing and how to get it, and in CI — where the policy is
required — a missing tool is an error rather than a skip.

---

## 4. Running it

```bash
cargo xtask deny   # the policy, against deny.toml
```

It needs the network and a licence database, so it is not a phase of
`cargo xtask ci`: a gate that fails for want of a network gets switched off.
It is run by name in `.github/workflows/ci.yml`, and `cargo xtask lists`
checks that a command `ci` does not run is still run by name there — so the
exception cannot quietly become a check that nobody runs.

The tool itself:

```bash
cargo install cargo-deny --locked
```
