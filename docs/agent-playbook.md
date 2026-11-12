# The agent playbook

How to drive craftkit from a tool, and how to read what comes back.

Every door in this tree reaches the **same** registry: the same commands, the
same session, the same document. Which door you use is a question of who is
driving and how much you need to see.

| door | program | speaks | use it when |
|---|---|---|---|
| terminal | `apps/craftkit-repl` | lines on a terminal | you want to try a command by hand |
| agent | `apps/craftkit-mcp-server` | MCP over stdio | an agent is driving, and the protocol is MCP |
| bridge | `apps/craftkit-bridge` | JSON-RPC over stdio | you want MCP's shape with your own client |
| browser | `apps/craftkit-wasm` | a page | the kit has to run in a browser |
| control channel | `crates/craftkit-control` | JSON lines over loopback | a program is **already running** and you want to drive it |

There is no sixth path. If you find yourself wanting to write to the document
directly, you want a command.

---

## 1. The terminal door

```bash
cargo run -p craftkit-repl
```

The door has a few built-ins:

| line | what it does |
|---|---|
| `help` | the commands, by area |
| `commands` | the commands' ids |
| `plan` | what the document means (this is `doc.plan`) |
| `memory` | what the session is holding (this is `session.memory`) |
| `snapshot <path>` | a picture of the document — the door asks `doc.snapshot` for it, and **no such command is in the registry in this tree**, so the door refuses it |
| `quit` | end the conversation |

Anything else is a **command id**, and the rest of the line is **JSON**:

```text
hello.greet {"name": "world"}
```

The JSON is not optional when the command has parameters, and it is the
parameters the command declared — the same names the schema publishes. A
parameter the command did not declare is **refused**, not ignored.

A refusal is printed as `refused: <why>`, and the door carries on.

The door's built-ins are commands of the kit with a friendlier shape: `plan`
is `doc.plan` and `memory` is `session.memory`. The door changes nothing
itself — it asks, and it prints.

The door bounds the line it reads: a line that is too long is refused rather
than read, because a person may paste anything into a prompt.

---

## 2. The agent door (MCP)

```bash
cargo run -p craftkit-mcp-server
```

It speaks MCP over its standard output — and **only** the protocol: a log line
on stdout would break the client. It writes its banner to stderr
(`craftkit-mcp-server: N commands, one tool each`).

**Every command in the registry is a tool of its own.** The tool's name is the
command's id with the dots turned into underscores, and the prefix
`craftkit_`:

```text
hello.greet  →  craftkit_hello_greet
```

The server answers these methods:

| method | what it does |
|---|---|
| `initialize` | the handshake; the server is called `craftkit` |
| `ping` | nothing |
| `shutdown` | stop |
| `tools/list` | one tool per command, with a JSON schema for its parameters |
| `tools/call` | run one command; the tool's name is the command's id |
| `prompts/list`, `prompts/get` | the prompts a person would type |
| `completion/complete` | complete a prompt's argument |
| `resources/list`, `resources/templates/list`, `resources/read` | the documents and node templates |

The versions the server understands are `2025-06-18`, `2025-03-26` and
`2024-11-05`.

A tool call that fails comes back as a result with `isError` set and a
sentence saying why. The sentences are written to be **fixable**: a missing
parameter is answered with the parameter's name, the kind of value it takes,
and the command's own line about it.

### The bridge

`apps/craftkit-bridge` is the other agent door: JSON-RPC over stdio, with
`initialize`, `tools/list`, `tools/call`, `resources/list`, `plan`, `journal`,
`menu` and `shutdown`. It calls itself `craftkit-bridge` and speaks protocol
version `2025-06-18`.

---

## 3. The control channel

The control channel is for driving a program that is **already running** — the
window. It is JSON lines over a loopback TCP socket.

### Opening it

The window opens one when it is told to:

```bash
CRAFTKIT_CONTROL_TOKEN=secret cargo run -p craftkit-window -- --control 4711
```

- `--control <port>` is the port to listen on. The window prints
  `the control channel is listening on 127.0.0.1:4711`.
- The **token** is asked for, not invented: it comes from the environment
  variable `CRAFTKIT_CONTROL_TOKEN`. A channel with a token a program can
  guess is a channel with no token.
- The channel serves the **window's own session**, so a command an agent runs
  over the channel changes what the window is drawing. There is one document,
  not two.

### The wire format

One JSON object per line, in both directions.

A request:

```json
{"id": 1, "token": "secret", "method": "engine.execute", "params": {"command": "hello.greet", "params": {"name": "world"}}}
```

- `id` correlates the reply (any JSON value; `null` is allowed).
- `token` — **only the first request on a connection carries it**. After that
  the connection is authenticated.
- `method` — one of the methods below.
- `params` — an object; absent and `null` both mean `{}`.

A reply:

```json
{"id": 1, "ok": true, "message": "Hello, world.", "data": {"message": "Hello, world.", "data": null, "changed": false, "needs_input": null}}
```

- `ok` — whether it worked.
- `message` — one line for a person.
- `data` — what the caller gets back.
- `kind` — a stable, machine-readable failure kind, present when `ok` is
  `false` (`unknown-method`, `unauthorized`, `bad-request`, `refused`,
  `no-window`, `failed`, `internal`).

### The methods

| method | parameters | what it does |
|---|---|---|
| `ping` | — | answers `pong` |
| `engine.describe` | — | the catalogue: every command (id, label, area, capability, doc, shortcut) and every node kind |
| `engine.execute` | `command`, `params` | run one command |
| `session.state` | — | the node count, the selection, the history, the pending question |
| `diagnostics.list` | — | the session's log and the journal's length |
| `ui.render` | — | the picture the program is showing, as PNG bytes, base64-encoded |
| `ui.click` | `x`, `y` | a click, in the world's units |
| `ui.drag` | `from`, `to` | a drag, in the world's units |
| `shutdown` | — | stop the program |

`engine.execute` takes the command's id in `params.command` and the command's
own parameters in `params.params`:

```json
{"id": 2, "method": "engine.execute", "params": {"command": "hello.greet", "params": {"name": "world"}}}
```

A command that is **waiting** for an answer takes the answer under the
question's own name, in the same `params.params`:

```json
{"id": 3, "method": "engine.execute", "params": {"command": "the.command", "params": {"the_answer": "yes"}}}
```

The `ui.*` methods need a program with a window: a channel over a program
without one answers `no-window`.

### A client, in twenty lines

The channel is a plain socket. Connect, send the token once, and then send one
line per request:

```python
import json, socket

s = socket.create_connection(("127.0.0.1", 4711))
f = s.makefile("rw", encoding="utf-8", newline="\n")

def call(method, params=None, first=False):
    request = {"id": call.n, "method": method, "params": params or {}}
    if first:
        request["token"] = "secret"
    call.n += 1
    f.write(json.dumps(request) + "\n")
    f.flush()
    return json.loads(f.readline())

call.n = 1
print(call("engine.describe", first=True))
```

---

## 4. The browser door

`apps/craftkit-wasm` is the kit in a page. It has two non-Rust files —
`index.html` and `browser-check.mjs` — because the browser insists on
JavaScript, and they are the only ones in the tree. The page drives the
terminal door's own logic (`apps/craftkit-wasm` depends on `craftkit-repl` and
`craftkit-studio`), so the page and the prompt say the same things for the
same reasons.

There is no bundler: the module is the file cargo built, and the page is the
file the tree holds. To build and serve it:

```bash
cargo build -p craftkit-wasm --target wasm32-unknown-unknown --release
cargo run -p craftkit-wasm --bin bundle -- \
    target/wasm32-unknown-unknown/release/craftkit_wasm.wasm
```

The bundle is written to `apps/craftkit-wasm/dist/`, and that directory has to
be **served** — a page loads its module with `fetch`, and `fetch` does not
read a file off the disk. The outer gate's `bundle` command still points at
the **v1** layout (`apps/craftkit-web`) — see `KNOWN-GAPS.md` §1.

---

## 5. How to read the results

Every door answers with the same thing underneath.

**A successful command** gives an outcome:

| field | what it is |
|---|---|
| `message` | one line for a person |
| `data` | what the caller gets back, if anything |
| `changed` | whether the document changed |
| `needs_input` | the question the command is waiting for, if it is waiting |

**A refusal** is an error with a kind and a sentence. The kinds an agent will
meet are: there is no such command; the command is not available; a parameter
was wrong (missing, of the wrong kind, or not one the command declared); the
command failed; the command failed inside (it panicked, and the document was
put back).

**A command that asks** is not a refusal and not a change. It stops with
`needs_input` set, and the document is as it was — a command that asks must
ask **before** it changes anything, so that it can be run again when the
answer comes. Answer it by running the same command with the answer under the
question's name.

**The plan** (`plan`, `doc.plan`) is what the document means: the node kinds
say what their own nodes are, and the evaluators say what the document is as
a whole. The plan carries **failures** too — what an evaluator could not say,
usually because it panicked. A plan with a failure is a plan that is missing
something, and the failure says what.

**The journal** is the list of commands that ran, with the values they were
given. It is a program: it can be run again.

---

## 6. The rules for whoever drives it

- **Drive it through commands.** A door that writes to the document is a
  second path, and the tree does not have one.
- **Read the parameters from the catalogue** (`engine.describe`,
  `tools/list`) rather than guessing: the schema is written from the **types**
  of the command's fields, so it cannot disagree with the code that reads
  them.
- **Refuse, do not clamp** — a parameter out of range is an error, and the
  error is the answer. Do not "fix" it by sending a value in range; the
  command is telling you something.
- **A missing parameter is a question, not a failure.** Answer it.
- **One command at a time** unless the command says otherwise: the session is
  one document, and the history is one undo stack.

---

## 7. When to use a sub-agent, and when not to

A sub-agent is a **round trip that costs money and returns an answer**. The
rule is short, and it is a rule about the *brief*, not about the task:

> **A sub-agent gets a bounded, mechanical brief, or it does not get
> spawned.**

- **One survey per battery, with a written contract.** "Survey the grid
  battery against gridcraft and report the gaps" is a brief. "Survey grid"
  is not: it has no boundary, no deliverable and no acceptance, and it will
  be re-asked.
- **Mechanical work is the good case.** A rewrite that touches every site of
  one shape, a set of files that must agree, a document that must be written
  from a list — these are the briefs that pay.
- **A question the tree can answer is not a brief.** Before spawning anything
  for "where does this command live" or "what does the claim table say",
  ask the tree: `cargo xtask find` and `cargo xtask brief` exist so that
  these questions cost one call and not eleven.
- **Reading what a rule *means* stays with the reader.** A survey can point
  at a rule in a reference tree; it cannot decide what that rule means for
  this tree, and a sub-agent's summary of a meaning is a second source of
  truth.

The cost of getting this wrong was measured on this tree's own history:
**41 sub-agents, six of which paid for themselves**, four that errored and
two that were aborted. The rest were one-off surveys of a tree the reader
could read directly.

---

## 8. What to read when something does not work

- `KNOWN-GAPS.md` — what this tree does not do, and what its gates cannot see.
- `docs/architecture.md` — the layers and the seams.
- `skills/craftkit-checking/SKILL.md` — the checklist for a broken tree.

---

## 9. How the next session is measured

The plan's exit numbers are **session-side**: the calls, the round trips and
the machine time are properties of the *session* that did the work, not of
the tree. The tree measures its own half — `cargo xtask census` (§859) says
how big an arc is, how many files it touches and how long it takes from one
arc to the next — and **this section is the procedure for the other half**.

### What is measured

| the number | what it is | where it comes from |
|---|---|---|
| the calls an arc makes | every tool call of the arc | the session's own record |
| the split | how many of the calls read or search, and how many write | the same record, by what the call did |
| the round trips | how many times the work went out and came back | the same record |
| the machine time | the wall clock of the builds and the suites | the same record, timed |
| the workspace runs | how many times the **whole** workspace was tested | the same record |
| the sub-agents | how many were spawned, and how many paid for themselves | the same record |

**And the tree's half**, which is in the tree and is measured by the tree:
`cargo xtask census` reports the arcs, the lines and the files each one
touched, the gap between them, and the commands per battery and per file.

### The baseline

The survey that made the plan is the baseline, and its numbers are in
`docs/survey-cost-and-meta.md`. The ones the waves have to move:

| the number | the baseline |
|---|---|
| the calls an arc makes | 35.4 |
| of them, reading and searching | 17.8 (50.3 %) |
| of them, the reference tree | 3.73 |
| of them, finding the test conventions | 1.465 (9.6 %) |
| the machine time an arc takes | 3.9 min |
| the workspace test runs | 959 (26.8 h) |
| the sub-agents that paid for themselves | 6 of 41 |

### The rule

- **A shortcut that does not move a number did not happen.** The waves are
  not a list of features; they are a claim about the cost of an arc, and
  the number is the claim. A shortcut that leaves the number where it was
  is a shortcut that did not happen, whatever it did to the tree.
- **The baseline is read, not remembered.** The numbers above are the ones
  the survey measured; a session that quotes a different baseline has
  invented a comparison.
- **The tree's half is measured by the tree.** When the question is "how
  big is an arc", the answer is `cargo xtask census` and not an estimate —
  and the same is true of the counts (`cargo xtask counts`) and of the
  bounds.

### The wave exits

| wave | the number that must move | from | to |
|---|---|---|---|
| 1 | workspace test runs | 959 (26.8 h) | < 100 (≈ 4 h) |
| 1 | calls per arc for verify | ~9 | 1 |
| 2 | claim and count searches per arc | 1.81 | 0 |
| 3 | reading and searching per arc | 17.8 | < 4 |
| 3 | calls per arc | 35.4 | < 12 |
| all | machine time per arc | 3.9 min | < 1 min |

The tree-side baselines — the lines in an arc and the gap between two arcs
— are in `docs/plan-acceleration.md`, and they are measured by
`cargo xtask census`.
