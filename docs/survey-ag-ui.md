# The survey of the agent-UI protocol: what an open event protocol does that this tree's doors do not

**The upstream** is an open protocol for agents and user-facing applications:
one request in, an ordered stream of typed events out. It is a **protocol**,
not an application, and it has a frozen 1.0 specification, a JSON Schema, a
conformance corpus and three generated SDKs.

**What this survey is.** A read of that protocol against **this tree's
doors** — the MCP door, the control channel, the a2ui door and the bridge —
looking for the patterns that would make this tree's architecture better,
and an honest verdict on each: **port**, **later**, or **skip**.

**What it is not.** A plan. Nothing here is built, and no section number is
reserved for any of it. A pattern worth taking becomes a plan, in
`docs/plans/`, the way everything else in this tree does.

---

## 0. The rule

**A pattern is worth taking when it makes a rule this tree already believes
enforceable, and not when it makes a rule this tree already has.** The tree
already has a registry, a guard, a journal, a history tree, a job pool, a
task registry and a resumable command; an upstream that repeats them is
telling us nothing new, and an upstream that contradicts them is telling us
something we have already decided against.

---

## 1. The method

- **The upstream** was read, not run: the specification's schema, its prose
  pages, its conformance harness's own documentation, and its fixture
  manifest. No build, no package manager, no test run.
- **This tree** was read in its own source: the four doors, the SDK's
  session, and the seams the SDK already owns.
- **The rule of the citations.** Every claim about the upstream carries a
  `path:line` from the upstream, and every claim about this tree carries a
  `path:line` from this tree. A claim without one is a guess, and a guess
  is not in this document.
- **The clean room.** The upstream's own reference frontend is not named
  here, and nothing of the upstream's code was copied: the survey is of
  **behaviour** and of **shapes**, and every shape below is re-derived in
  this tree's own words.

**What the survey did not read**: the upstream's draft specification tree
(a parallel, not-yet-frozen copy), its reconciliation notes, and the SDK
implementations beyond their type surfaces. The claims about the harness
come from the harness's own documentation, not from a green run.

---

## 2. The upstream's surface, in one table

| the part of the protocol | what it is | upstream path:line |
|---|---|---|
| the event union | one root, discriminated by `type`, 31 event definitions | `spec/1.0/schema.json:8` |
| the event types | a closed vocabulary of 31 names | `spec/1.0/schema.json:45` |
| the event envelope | `type`, `timestamp`, `rawEvent`, `metadata` | `spec/1.0/schema.json:83` |
| attribution | `subagentRunId`, absent meaning the parent produced it | `spec/1.0/schema.json:109` |
| the state value | any JSON, carried uninterpreted | `spec/1.0/schema.json:131` |
| the text stream | start / content / end, plus a compact chunk spelling | `spec/1.0/schema.json:155` |
| the tool-call stream | start / args / end, plus a result event | `spec/1.0/schema.json:252` |
| the tool call in history | `function.arguments` is a **JSON string**, not parsed JSON | `spec/1.0/schema.json:1284` |
| the state snapshot | replaces wholesale; a consumer must not merge it | `spec/1.0/schema.json:381` |
| the state delta | an RFC 6902 patch, applied atomically | `spec/1.0/schema.json:399` |
| the messages snapshot | the producer restating the conversation it owns | `spec/1.0/schema.json:417` |
| the activity snapshot and delta | structured progress as its own widget | `spec/1.0/schema.json:433` |
| raw and custom events | the escape hatches, with stated limits | `spec/1.0/schema.json:491` |
| the run lifecycle | started / finished / error, with success, interrupt and cancelled outcomes | `spec/1.0/schema.json:531` |
| the steps | named phases over a run, matched by name | `spec/1.0/schema.json:612` |
| the reasoning span | a bracketed stretch of thinking, and an encrypted artefact | `spec/1.0/schema.json:648` |
| the subagent events | started / finished / error, and an info record | `spec/1.0/schema.json:800` |
| the interrupt | what a run needs from outside: an id, a reason, a response schema | `spec/1.0/schema.json:967` |
| the resume entry | the answer to one interrupt, and its status | `spec/1.0/schema.json:1006` |
| the token usage | cost accounting, absent is not zero | `spec/1.0/schema.json:1032` |
| the message union | seven roles: developer, system, assistant, user, tool, activity, reasoning | `spec/1.0/schema.json:1084` |
| the run input | `threadId`, `runId`, `messages`, `state`, `tools`, `resume`, `forwardedProps` | `spec/1.0/schema.json:1555` |
| the advertised tool | a tool the **application** offers, with an opaque parameter schema | `spec/1.0/schema.json:1530` |
| the capabilities | what the agent declares about itself, all optional | `spec/1.0/schema.json:1918` |
| the JSON patch | the delta encoding: `add`, `remove`, `replace`, `move`, `copy`, `test` | `spec/1.0/schema.json:1971` |
| the transport | HTTP with Server-Sent Events; one event object per frame | `docs/spec/1.0/basic/transports/http-sse.mdx:13` |
| the pipeline order | producer → compatibility → middleware → enforcement → chunk expansion → verification → application | `docs/spec/1.0/basic/processing.mdx:121` |
| the receiver's rule | unrecognised material is not an error; a malformed known value is | `docs/spec/1.0/basic/processing.mdx:12` |
| the version negotiation | both sides declare the version they speak; neither echoes the other | `docs/spec/1.0/basic/versioning.mdx:71` |
| the capabilities' rule | omitted means **undeclared**, not unsupported | `docs/spec/1.0/basic/capabilities.mdx:26` |
| the snapshot rule | a patch that does not apply is warned about and skipped; the next snapshot repairs it | `docs/spec/1.0/basic/patterns/snapshots.mdx:60` |
| the interrupt rule | there is no mid-run channel: a run that needs a person **ends** with an interrupt | `docs/spec/1.0/basic/patterns/interrupt-resume.mdx:6` |
| the conformance rule | every fixture names the one-line change that would make it fail | `spec/1.0/conformance/README.md:176` |
| the manifest rule | the fixture listing is pinned, so a disappearing fixture is a diff | `spec/1.0/fixtures/MANIFEST.txt:1` |
| the number ledger | the binary transport's field numbers are append-only and generator-owned | `spec/1.0/proto-freeze.txt:1` |
| the trust rules | consent, model output is untrusted input, and data flows both ways | `docs/spec/1.0/index.mdx:89` |

---

## 3. What this tree already has

The tree has four doors and a set of seams, and the comparison is only fair
if the tree's side is stated first.

| the door or the seam | what it is | this tree's path:line |
|---|---|---|
| the MCP door | one tool per command; the tool list **is** the command list | `crates/craftkit-mcp/src/lib.rs:1` |
| the control channel | newline-delimited JSON, a request and a reply, a launch token on the first request | `crates/craftkit-control/src/lib.rs:31` |
| the a2ui door | the a2ui methods, with a capability handshake in both directions | `apps/craftkit-a2ui-server/src/lib.rs:458` |
| the bridge | JSON-RPC over stdio: a second front end over the same registry | `apps/craftkit-bridge/src/lib.rs:1` |
| the registry | the commands, their typed parameters, their predicates | `crates/craftkit-sdk/src/command.rs:1` |
| the guard | a panic in code the kit does not own becomes an error | `crates/craftkit-sdk/src/guard.rs:1` |
| the journal | every command that ran, with the values it was given; bounded | `crates/craftkit-sdk/src/journal.rs:1` |
| the history | a **tree** of states, not a line; undo and redo across branches | `crates/craftkit-sdk/src/history.rs:1` |
| the job pool | work that takes seconds, cancellable, results collected when ready | `crates/craftkit-sdk/src/jobs.rs:1` |
| the task registry | work with a **durable handle**, finished by an event from outside | `crates/craftkit-sdk/src/tasks.rs:1` |
| the resumable command | a command that asks the host while it runs, and carries on | `crates/craftkit-sdk/src/resume.rs:1` |
| the question | a name, a kind and a line of help, answered by a host | `crates/craftkit-sdk/src/ask.rs:53` |
| the bundle | a document and the files it needs, in one file, with a checksum | `crates/craftkit-sdk/src/bundle.rs:1` |

Three facts about the tree's shape matter for every verdict below.

1. **The tree's doors are request and reply.** The control channel's methods
   are `ping`, `engine.describe`, `engine.execute`, `session.state`,
   `diagnostics.list` and `shutdown`
   (`crates/craftkit-control/src/lib.rs:144`), and there is no `subscribe`,
   no `watch` and no notification anywhere in it.
2. **The tree already asks and answers.** A command that needs something it
   alone knows asks the host, and the answer comes back in the parameters
   (`crates/craftkit-control/src/lib.rs:705`), with thirty-two questions and
   thirty-two answers as the bound (`crates/craftkit-sdk/src/resume.rs:64`).
   The upstream's human-in-the-loop is the same seam, arrived at from the
   other side.
3. **The tree's state is typed and the agent does not own it.** The document
   is a tree of typed nodes, and what an agent may do is a **command**. The
   upstream's state is a blob of JSON the agent owns
   (`spec/1.0/schema.json:131`), and this tree's answer to that is the
   registry.

---

## 4. The patterns, one by one

| the pattern | upstream path:line | this tree's state | verdict | the rule, in this tree's words |
|---|---|---|---|---|
| a run answers with a **stream** of events | `spec/1.0/schema.json:8` | absent: every door answers once | **later** | a run that takes a minute says what it is doing while it does it |
| the state is a **snapshot plus a delta**, with a resync rule | `spec/1.0/schema.json:399`, `docs/spec/1.0/basic/patterns/snapshots.mdx:60` | partial: `session.state` gives counts, a selection and a history summary, not the document | **port** | a change is a patch; a patch that does not apply is warned about and skipped, and the next snapshot repairs it |
| a run that needs a person **ends** with an interrupt, and the answer comes in the next run | `spec/1.0/schema.json:967`, `:1006` | **present and stronger**: the command keeps waiting, the answer comes in the parameters | **skip** (the tree's model) | a command that asks is restarted with the answer, and it changed nothing |
| every interrupt must be **covered** | `docs/spec/1.0/basic/patterns/interrupt-resume.mdx:54` | absent: a pending command dropped by another command is dropped **silently** | **port** | a question that is dropped is a **cancelled** question, and it says so |
| the **capabilities** are optional, and absent means undeclared | `spec/1.0/schema.json:1918`, `docs/spec/1.0/basic/capabilities.mdx:26` | the opposite in one place: the a2ui door **refuses** a client that declares neither catalogue route (`apps/craftkit-a2ui-server/src/lib.rs:479`) | **port** | an absent capability is undeclared, not unsupported; a client that says nothing is not a client to refuse |
| the **version** is declared by both sides and never echoed | `spec/1.0/schema.json:545`, `docs/spec/1.0/basic/versioning.mdx:71` | absent on the control door: there is no version anywhere in it; the MCP door carries the standard field; the a2ui door declares its own in the handshake (`apps/craftkit-a2ui-server/src/lib.rs:498`) | **port** | the first request carries the client's version and the answer carries the door's own, so a downgrade is visible the moment it happens |
| the request passes through a **fixed order**: compatibility, then enforcement, then the command | `docs/spec/1.0/basic/processing.mdx:121` | implicit: the session's guard and the gate's layering; the doors decode and execute | **port** | a command never sees what the rules would have removed |
| a receiver is **forgiving** and a known value is **exact** | `docs/spec/1.0/basic/processing.mdx:12` | already true: unknown JSON fields are ignored (no `deny_unknown_fields` in the tree) and unknown command ids are refused | **skip** (keep it) | a newer peer is not an error; a corrupt known value is |
| the application offers **tools to the agent** | `spec/1.0/schema.json:1530` | partial: the control channel has `ui.render`, `ui.click` and `ui.drag`; the MCP door has no such direction | **later** | a front end can offer the agent something to call |
| the agent's own **tools** are a separate list from the client's | `spec/1.0/schema.json:1687` | one list: the registry | **skip** | one registry, one vocabulary |
| the **subagent** attribution must not disagree with its opener | `spec/1.0/schema.json:800`, `docs/spec/1.0/events/subagents.mdx:63` | absent: no subagent model | **later** | a continuation names the run it continues, and a disagreement is refused |
| the **custom and raw** events are escape hatches with stated limits | `spec/1.0/schema.json:512`, `docs/spec/1.0/events/passthrough.mdx:54` | the opposite: an unknown method is refused by name | **skip** | a name that is not known is an error |
| the **state is opaque** to the protocol | `spec/1.0/schema.json:131` | the opposite: the document is typed nodes and the agent gets commands | **skip** | derived values are derived, and the document is not a bag |
| the **token usage** is reported | `spec/1.0/schema.json:1032` | absent, and it belongs to the host: the kit makes no model call | **skip** | the model call belongs to the host |
| the **reasoning** and the **activity** are their own events | `spec/1.0/schema.json:648`, `:433` | absent; they are a rendering surface | **skip** | the tree draws its own |
| the **messages** have seven roles | `spec/1.0/schema.json:1084` | the journal is a machine-readable record of commands, not a conversation | **skip** | a record of what was done is not a dialogue |
| the **conformance corpus** names the change that would make each fixture fail | `spec/1.0/conformance/README.md:176` | partial: the tree's gate is the corpus, and the doors' protocol tests live in the crates | **port** | a fixture that cannot fail is worse than no fixture |
| the fixture **listing is pinned** | `spec/1.0/fixtures/MANIFEST.txt:1` | the tree's port manifest is forward-only; there is no manifest for the doors' protocol fixtures | **port** | a fixture that disappears is a diff, not a smaller corpus |
| the **wire numbers** are append-only and generator-owned | `spec/1.0/proto-freeze.txt:1` | the tree's ids are strings, and the port manifest is forward-only | **later** | a retired id is never reused, and the list of retired ids is kept |
| the protocol's **trust rules** are written down | `docs/spec/1.0/index.mdx:89` | the tree has two trust boundaries and a guard, and no written statement of the agent-facing one | **port** | what crosses the door is untrusted input, and the door says so |

---

## 5. The ones worth taking

Seven arcs, in the order they would land. Each is a small change to a door
or to the SDK, and each is written as a **rule**, because that is what this
tree ports.

### 5.1 The control door says what version it is

The control channel has no version anywhere in it
(`crates/craftkit-control/src/lib.rs:1`), and it already has the right place
for one: the first request on a connection carries the launch token
(`crates/craftkit-control/src/lib.rs:33`). The rule:

> **The first request carries the client's protocol version, and the first
> answer carries the door's own. Neither is an echo of the other.**

The upstream's reason is worth keeping: a downgrade is visible the moment it
happens, because the pair is the whole negotiation
(`docs/spec/1.0/basic/versioning.mdx:71`).

### 5.2 A change is a patch

`session.state` today answers with the number of nodes, the selection, the
history's depth and the pending question
(`crates/craftkit-control/src/lib.rs:731`). An agent that wants to know
*what changed* has to ask the whole document again, or read the picture
through `ui.render`. The rule:

> **A change is a patch against the document as it was, and a patch that
> does not apply is warned about and skipped — the prior value stands, and
> the next snapshot repairs the divergence.**

The upstream's delta is an RFC 6902 patch (`spec/1.0/schema.json:1971`) and
its resync rule is the sentence above
(`docs/spec/1.0/basic/patterns/snapshots.mdx:60`). The tree has the two
things a patch needs: a **history tree** of whole states
(`crates/craftkit-sdk/src/history.rs:1`) to diff against, and a **journal**
of every command that ran (`crates/craftkit-sdk/src/journal.rs:1`) to say
*which* command moved what.

### 5.3 A dropped question is a cancelled question

A command that asks waits (`crates/craftkit-sdk/src/resume.rs:1`), and
another command may run while it waits — and then the waiting one is
dropped. Today that drop is silent. The upstream names the same thing: a run
that is stopped on purpose and did not fail is **cancelled**, and it must
not read as success (`spec/1.0/schema.json:924`). The rule:

> **A question that is dropped is a cancelled question, and the answer says
> so.**

This is one field on the outcome the doors already send
(`crates/craftkit-control/src/lib.rs:721`).

### 5.4 An absent capability is undeclared

The a2ui door refuses a client that declares neither catalogue route
(`apps/craftkit-a2ui-server/src/lib.rs:479`), which is right for a client
that says "I take no catalogue" and wrong for a client that says nothing at
all. The upstream's rule is the careful one: omitted means **undeclared**,
not unsupported, and a declaration never gates a run
(`docs/spec/1.0/basic/capabilities.mdx:26`). The rule:

> **A capability that is not declared is not a capability that is refused; a
> door that needs one asks for it in the handshake's answer.**

### 5.5 The request's order is written down

The upstream's pipeline is load-bearing and stated as such
(`docs/spec/1.0/basic/processing.mdx:121`): compatibility runs before
enforcement, and application code never sees what enforcement would have
removed. This tree already believes both halves — the gate's layering, and
the "the UI is thin" rule — and neither is written down at the door. The
rule:

> **A request is decoded, then made compatible, then checked, then run; and
> a command never sees what the checks would have removed.**

### 5.6 Every protocol fixture names its kill

The upstream's conformance corpus carries, per fixture, the one-line change
that must make it fail, and says why: "a fixture that cannot fail is worse
than no fixture" (`spec/1.0/conformance/README.md:176`). The tree's own
gate has the same spirit — the count rules were "proven to bite by breaking
the numbers on purpose" (§1180) — and the doors' protocol tests do not have
the discipline written down. The rule:

> **Every protocol fixture names the one change that would make it fail, and
> the test asserts that the change does.**

### 5.7 The fixture listing is pinned

The upstream keeps a sorted listing of every fixture file so that a
disappearing fixture is a diff rather than a silently smaller corpus
(`spec/1.0/fixtures/MANIFEST.txt:1`). The tree has the same idea in two
places — the port manifest is forward-only, and the counted block is
gate-checked — and the doors' protocol fixtures are not in either. The
rule:

> **The doors' protocol fixtures are listed in a manifest, and the list is
> checked in both directions.**

### 5.8 What the door says about trust

The upstream's security section is three principles
(`docs/spec/1.0/index.mdx:89`): the application decides what runs and
obtains consent for side effects; model output is untrusted input and must
be validated before it is acted on; and data flows both ways, so no secrets
go into the conversation. This tree has the two trust boundaries
(`craftkit_sdk::guard`, and `Session::execute`'s restore) and a written
posture on the model call — it belongs to the host (§1178) — and **no
statement about what crosses a door**. The rule:

> **What comes in over a door is untrusted input: it is bounded before it
> is read, typed before it is run, and never trusted because it came from
> an agent.**

---

## 6. What not to take, and why

- **The opaque state blob.** The upstream lets the agent own a value the
  protocol carries without interpreting (`spec/1.0/schema.json:131`). This
  tree's document is typed, and the agent's power is the registry. A blob
  would be a second way to change the document, which is the one thing this
  tree does not have.
- **The escape hatches.** `RAW` and `CUSTOM` exist so a producer does not
  smuggle private signalling through the standard stream
  (`docs/spec/1.0/events/passthrough.mdx:30`). This tree's rule is the
  opposite and deliberate: a name that is not known is an error, and a
  method that is not there says so, by name
  (`crates/craftkit-control/src/lib.rs:31`).
- **The conversation.** Seven roles, a message union, an encrypted
  reasoning artefact and a token count
  (`spec/1.0/schema.json:1084`, `:768`, `:1032`). These belong to the host
  that talks to a model. The kit's own record of what happened is the
  **journal**, and it is a list of commands, not a dialogue.
- **The two tool lists.** The upstream keeps the agent's tools and the
  application's tools apart (`spec/1.0/schema.json:1687`). This tree has one
  registry and one vocabulary, and the MCP door's whole argument is that the
  tool list *is* the command list (`crates/craftkit-mcp/src/lib.rs:12`).

---

## 7. What this survey does not settle

- **Whether a run should answer with a stream.** The upstream's shape is
  one request and many events; this tree's doors answer once, and its
  **tasks** already carry a durable handle and a state that moves one way
  (`crates/craftkit-sdk/src/tasks.rs:1`). The question — does an agent need
  progress, or does it need a handle it can ask about? — is a decision
  about the doors, not about the protocol, and it is **not** made here.
- **Whether the control channel's `ui.*` methods are the tree's
  "application offers tools to the agent"**, or a private way into a
  program. They exist (`crates/craftkit-control/src/lib.rs:150`) and the
  upstream's shape for the same idea is an advertised tool with a parameter
  schema (`spec/1.0/schema.json:1530`).
- **Whether a patch is the right delta.** The upstream's is RFC 6902 over
  JSON. This tree's document is a tree of typed nodes with a bundle beside
  it (`crates/craftkit-sdk/src/bundle.rs:1`), and a patch over *nodes* is a
  different shape from a patch over JSON. Which one a client finds easier
  is not measured here.
- **Whether the upstream's conformance harness would pass against this
  tree's doors.** It was not run, and its fixtures are about a protocol this
  tree does not speak.
- **The subagent model.** The upstream nests runs and requires a
  continuation's attribution to agree with its opener's
  (`docs/spec/1.0/events/subagents.mdx:63`). This tree has plugins and a
  registry, and no notion of a run inside a run.
