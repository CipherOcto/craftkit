# The doors, side by side

What each of the tree's **six doors** can do, cell by cell, and where each
cell was read.

This is the honest first cut of the survey's **parity matrix** (O4.2): a
table, checked by hand. It is not a gate and it does not pretend to be
one. A table that drifts is worse than no table, so the way to keep it
true is to read it again — and the next cut, if there is one, is a check
that a door's answer matches the cell.

## What a door is

A **door** is a program that reaches the kit's registry and nothing else.
There are six:

| door | crate | who looks at it | transport |
|---|---|---|---|
| the terminal door | `apps/craftkit-repl` | a person | a line of text |
| the window | `apps/craftkit-window` | a person | a native window |
| the browser door | `apps/craftkit-wasm` | a person | four exports over a memory boundary |
| the agent door | `apps/craftkit-mcp-server` over `crates/craftkit-mcp` | an agent | stdio JSON-RPC (MCP) |
| the bridge | `apps/craftkit-bridge` | an agent | stdio JSON-RPC |
| the a2ui door | `apps/craftkit-a2ui-server` | an agent | stdio JSON-RPC (a2ui) |

Two crates are **not** doors, and the rule "a front end is thin" does not
reach them: `crates/craftkit-mcp` and `crates/craftkit-control` are the
**transports** two of the doors speak over, and a transport holds no
session of its own.

The **studio** is a **host**: its library is a command book, and its
entry point is a front end (`crates/craftkit-gates`'s `FRONT_END_FILES`).

## The matrix

**✓** means the door does it, **—** means it does not, and the cells that
say **as a command** mean the door runs the command like any other and
never opens the door behind it.

| door | lists the commands | publishes a schema | runs a command | the plan | the undo | the journal | the document's tree | a refusal is told |
|---|---|---|---|---|---|---|---|---|
| the terminal door | ✓ `help`, `commands` | — | ✓ | ✓ as a command | ✓ as a command | the size only | — | ✓ |
| the window | ✓ by area, and the panels | — a **form** per command | ✓ | ✓ as a command | ✓ as a command | — the window's own log | ✓ | ✓ |
| the browser door | ✓ the terminal door's | — | ✓ | ✓ as a command | ✓ as a command | the size only | — | ✓ |
| the agent door | ✓ `tools/list`, one tool per command | ✓ the **JSON Schema** | ✓ `tools/call` | ✓ as a command | ✓ as a command | ✓ as a command | — | ✓ |
| the bridge | ✓ `tools/list`, from the projection | a **descriptor**, not a schema | ✓ `tools/call` | ✓ the `plan` method | ✓ as a command | ✓ the `journal` method | — | ✓ |
| the a2ui door | a2ui's **catalogue** | a2ui's own | ✓ a renderer function | — | — | — | the **surfaces** | ✓ |

## How each cell was checked

Each cell below was read in the code, and the line is where it was read.

**The terminal door** — `apps/craftkit-repl/src/lib.rs`.
`help` lists the menu by area and `commands` lists the ids
(`:136`, `:145`); `run_command` runs any command by its id (`:208`);
`plan` asks `doc.plan` (`:154`); `memory` reports the journal's **size**
(`:176`); a refusal is answered with the refusal's own text (`:241`).
There is no node list and no schema: a person at a prompt reads a line.

**The window** — `apps/craftkit-window/src/lib.rs`.
`commands` lists the commands and `commands_outside_panels` the ones no
panel shows (`:132`, `:174`); `open` builds a **form** from the command's
own parameters (`:237`); `run` runs it (`:283`); `tree` lists the nodes
(`:191`); `plan` asks `doc.plan` (`:211`); the log holds what the window
has to say (`:459`).

**The browser door** — `apps/craftkit-wasm/src/lib.rs`.
It holds the terminal door's `Repl` and runs it (`:95`, `:112`), so every
cell of that row is that door's cell: the browser door is the terminal
door in a page.

**The agent door** — `crates/craftkit-mcp/src/lib.rs`.
The tools are made from the session's registry, one per command
(`:142`); a tool's schema is the **JSON Schema**, and the mapping lives
in the SDK (`craftkit_sdk::schema::to_json_schema`, `:445`); the methods
are `tools/list`, `tools/call`, `prompts/list`, `prompts/get`,
`resources/list`, `resources/read` and `completion/complete` (`:243`–`:306`).
The plan, the undo and the journal are **tools like any other**, because
every command is a tool.

**The bridge** — `apps/craftkit-bridge/src/lib.rs`.
`tools` is the **projection** and every tool carries the projection's
**descriptor** (`:183`); `plan` asks the `doc.plan` command (`:246`);
`journal` answers with the entries (`:258`); `resources` is the kinds, the
formats and the **manifest** (`:228`); `menu` is the menu tree (`:266`).
Its protocol is **its own**, so its tools carry `parameters` and not
`inputSchema`.

**The a2ui door** — `apps/craftkit-a2ui-server/src/lib.rs`.
The methods are a2ui's own (`:458`–`:464`): `createSurface`,
`updateComponents`, `updateDataModel`, `deleteSurface` and
`callRendererFunction`, with `initialize` for the transport's handshake.
Its vocabulary is **a2ui's**, not the kit's: it publishes a catalogue of
a2ui's components, and a surface is a tree of those.

## What the matrix says

- **Five doors speak the kit's vocabulary and one does not.** The a2ui
  door is another language: a catalogue of components, not a list of
  commands. It is a door *over* the kit, not a second view of the
  registry.
- **The plan is a command, and the undo is a command.** Every door that
  has a plan asks `doc.plan`, and no door opens the history's own door.
  That is the rule "everything is a command" reaching the surface.
- **Only the agent door publishes a JSON Schema**, because only an agent
  reads one; a person at a window gets a **form** built from the same
  parameters. The bridge publishes a **descriptor** — the projection's own
  shape — and the SDK says the two are not re-expressions of one another.
- **The tree is the window's.** The window shows the nodes; the other
  doors show the plan or the commands, and a program that wants the nodes
  asks for them.
- **The journal is the bridge's**, and the agent door has it as a tool;
  the terminal door reports its **size**, because a person at a prompt
  wants to know how much has happened and not to read it all.

## What this does not settle

- **The a2ui door cannot undo and cannot read the plan.** a2ui has no
  plan, and the door has no history of its own; a surface is changed by
  the commands that make it, and the door does not keep a second door.
- **The browser door has no tree and no schema**, because it is the
  terminal door in a page.
- **The table is checked by hand.** Nothing keeps it true, and the next
  cut — if there is one — is a check that reads the doors and compares.
