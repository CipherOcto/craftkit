# The doors, side by side

What each of the tree's **six doors** can do, cell by cell, and the
**test** that proves each cell.

This is the survey's **parity matrix** (O4.2): the six doors side by side.
A table that drifts is worse than no table, so the way to keep it true is
to name the **test** that proves each cell — a named test does not drift
when the code moves, and a line does.

**The outer gate reads this table**, and it says two things about it: every
cell that says something names a test, and every test it names is a test
that is in the tree.

And **one program compares the doors with each other**: the parity
harness (`apps/craftkit-parity`) drives five of the six doors with the same
five questions — which commands do you list, what do you say when you run
this command, what plan do you publish, what do you say when you undo it,
what do you say when the command is not there — and says where they
differ. It asks a **session of its own** the same questions, and compares
the doors with that as well: the session is the truth behind the doors, so
two doors that are wrong in the same way are still wrong. What the gate
does **not** do is read this table and check a cell against a door: the
cells are still read by a person.

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

**Every cell names the test that proves it.** The path is relative to the
crate the row names — the door's own crate, and the transport's crate for
the agent door — and a path that starts at `crates/` or `apps/` is the
tree's own.

| door | lists the commands | publishes a schema | runs a command | the plan | the undo | the journal | the document's tree | a refusal is told |
|---|---|---|---|---|---|---|---|---|
| the terminal door — `apps/craftkit-repl` | ✓ `help`, `commands` — `tests/repl.rs::help_lists_the_commands` | — | ✓ `tests/repl.rs::a_command_runs_and_the_document_changes` | ✓ as a command — `tests/repl.rs::plan_comes_from_the_command` | ✓ as a command — `tests/repl.rs::the_undo_is_a_command` | the size only — `tests/repl.rs::memory_comes_from_the_command` | — | ✓ `tests/repl.rs::an_unknown_command_is_refused` |
| the window — `apps/craftkit-window` | ✓ by area, and the panels — `tests/window.rs::the_window_shows_every_command` | — a **form** per command — `tests/window.rs::the_form_is_the_commands_own_parameters` | ✓ `tests/window.rs::the_window_runs_a_command_through_the_registry` | ✓ as a command — `tests/window.rs::the_plan_is_the_commands_work` | ✓ as a command — `tests/window.rs::the_undo_is_a_command` | — | ✓ `tests/window.rs::the_window_draws_the_document_through_the_kinds` | ✓ `tests/window.rs::a_command_that_refuses_says_so` |
| the browser door — `apps/craftkit-wasm` | ✓ the terminal door's — `tests/door.rs::the_door_lists_its_commands` | — | ✓ `tests/door.rs::a_command_runs_and_the_answer_comes_back` | ✓ as a command — `tests/door.rs::the_plan_is_a_command_the_page_can_ask_for` | ✓ as a command — `tests/door.rs::the_undo_is_a_command` | the size only — `tests/door.rs::the_journal_is_a_size` | — | ✓ `tests/door.rs::an_unknown_command_is_refused` |
| the agent door — `crates/craftkit-mcp` | ✓ `tools/list`, one tool per command — `tests/tools.rs::the_tool_list_is_the_command_list` | ✓ the **JSON Schema** — `tests/tools.rs::a_tools_schema_is_the_json_schema_the_sdk_builds` | ✓ `tools/call` — `tests/tools.rs::a_tool_call_runs_the_command` | ✓ as a command — `tests/tools.rs::the_plan_the_undo_and_the_journal_are_tools` | ✓ as a command — `tests/tools.rs::the_undo_tool_runs_the_command` | ✓ as a command — `tests/tools.rs::the_plan_the_undo_and_the_journal_are_tools` | — | ✓ `tests/tools.rs::a_refused_call_says_so` |
| the bridge — `apps/craftkit-bridge` | ✓ `tools/list`, from the live session — `tests/bridge.rs::a_command_made_at_run_time_is_listed_and_can_be_run` | a **descriptor**, not a schema — `tests/bridge.rs::the_tool_list_is_the_projection` | ✓ `tools/call`, and only what it lists — `tests/bridge.rs::the_bridge_runs_only_what_it_lists` | ✓ the `plan` method — `tests/bridge.rs::the_plan_is_the_projects_plan` | ✓ as a command — `tests/bridge.rs::the_undo_is_a_command` | ✓ the `journal` method — `tests/bridge.rs::the_journal_is_over_the_wire` | — | ✓ `tests/bridge.rs::a_refused_call_says_so` |
| the a2ui door — `apps/craftkit-a2ui-server` | a2ui's **catalogue** — `tests/door.rs::the_catalogue_the_door_serves_is_the_layers_own` | a2ui's own — `crates/craftkit-plugin-a2ui/tests/components.rs::each_published_schema_carries_exactly_the_tables_props` | ✓ a renderer function — `tests/door.rs::a_button_runs_its_command` | — | — | — | the **surfaces** — `tests/door.rs::a_surface_that_carries_a_tree_gets_it_drawn` | ✓ `tests/door.rs::the_agents_answer_is_refused_and_not_swallowed` |

Three notes on the cells that are not a tick:

- **The window has no journal** — it keeps its own log, and the log is the
  window's business and not a command.
- **The terminal door and the browser door report the journal's size**,
  and not the entries themselves: a person at a prompt wants to know how
  much has happened.
- **The a2ui door has no plan, no undo and no journal**: a2ui has no plan,
  the door has no history of its own, and a surface is changed by the
  commands that make it.

## How each door reaches the kit

**The terminal door** reads its list from the **session** it drives, and
`run_command` runs any command by its id. `plan`, `memory` and `snapshot`
are the door's own **built-ins**, and each of them asks a command
(`doc.plan`, `session.memory`, `doc.snapshot`) — so the door prints the
answer and never makes one. There is no node list and no schema: a person
at a prompt reads a line.

**The window** builds a **form** from the command's own parameters and
runs the command through the registry. Its tree is the document's, drawn
through the kinds.

**The browser door** holds the terminal door's `Repl` and runs it, so
every cell of that row is the terminal door's cell: the browser door is
the terminal door in a page.

**The agent door** makes its tools from the session's registry, one per
command, and the list is read **when it is asked for** — so a command that
joins the session while the program runs is a tool with the rest. A tool's
schema is the **JSON Schema**, and the mapping lives in the SDK. The plan,
the undo and the journal are **tools like any other**, because every
command is a tool.

**The bridge** reads its tools from the **session** the tools run in, and
prunes the list to what this door shows: the commands, the effects and the
tools. The registry stays whole, and a command outside the pruning is
**not runnable through this door** — the door runs only what it lists. Its
protocol is **its own**, so its tools carry `parameters` and not
`inputSchema`.

**The a2ui door** speaks a2ui's own vocabulary: a catalogue of a2ui's
components, and a surface is a tree of those. Its methods are a2ui's —
`createSurface`, `updateComponents`, `updateDataModel`, `deleteSurface`
and `callRendererFunction`, with `initialize` for the transport's
handshake.

## What the matrix says

- **Five doors speak the kit's vocabulary and one does not.** The a2ui
  door is another language: a catalogue of components, not a list of
  commands. It is a door *over* the kit, not a second view of the
  registry.
- **The plan is a command, and the undo is a command.** Every door that
  has a plan asks `doc.plan`, and no door opens the history's own door.
  That is the rule "everything is a command" reaching the surface.
- **Every door lists what the session has.** The list is read from the
  session the commands run in, so a command that joins the session while
  the program runs is listed with the rest — and a door never offers a
  command the session does not have. A listed command can still be
  **refused**: a command whose `enabled` predicate is false right now
  is refused when it is run, and the predicate is over the session's
  state, so the state can change between the list and the call.
- **Only the agent door publishes a JSON Schema**, because only an agent
  reads one; a person at a window gets a **form** built from the same
  parameters. The bridge publishes a **descriptor** — the SDK's own shape
  for one command — and the SDK says the two are not re-expressions of
  one another.
- **The tree is the window's.** The window shows the nodes; the other
  doors show the plan or the commands, and a program that wants the nodes
  asks for them.
- **The journal is the bridge's**, and the agent door has it as a tool;
  the terminal door reports its **size**, because a person at a prompt
  wants to know how much has happened and not to read it all.

## What this does not settle

- **The gate does not drive the doors.** It reads this table and checks
  that every cell names a test and that the test is there. It does not
  check that a named test is *about* the cell it is in — that is still a
  reader's job.
- **The harness compares the doors with each other, and with the
  session, and not with this table.** It drives five of the six doors and
  says where they differ, and it asks a **session of its own** the same
  questions — the session is the truth behind the doors, so a door that
  says something the session does not say is a difference even when
  every other door says it too. It does not read a cell and check it
  against a door. The a2ui door is not in it at all: it speaks a2ui's
  own vocabulary, and there is nothing to compare.
- **A negative cell has no proof to name.** "The terminal door has no
  schema" is an absence, and an absence has no test to point at.
- **The a2ui door cannot undo and cannot read the plan.** a2ui has no
  plan, and the door has no history of its own; a surface is changed by
  the commands that make it, and the door does not keep a second door.
- **The browser door has no tree and no schema**, because it is the
  terminal door in a page.
