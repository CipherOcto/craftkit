# The doors, side by side

What each of the tree's **six doors** can do, cell by cell, and the
**test** that proves each cell.

This is the honest first cut of the survey's **parity matrix** (O4.2): a
table, checked by hand. It is not a gate and it does not pretend to be
one. A table that drifts is worse than no table, so the way to keep it
true is to read it again — and the way to keep a **cell** true is to name
the test that proves it, because a named test does not drift when the
code moves and a line does. The outer gate reads this table and checks
that every test it names **exists**; what is not checked is the cells
themselves, which would need a harness that drives all six doors at once.

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
| the bridge | ✓ `tools/list`, from the live session | a **descriptor**, not a schema | ✓ `tools/call`, and only what it lists | ✓ the `plan` method | ✓ as a command | ✓ the `journal` method | — | ✓ |
| the a2ui door | a2ui's **catalogue** | a2ui's own | ✓ a renderer function | — | — | — | the **surfaces** | ✓ |

## How each cell was checked

Each cell below was **proved by a test**, and the test is named. A named
test does not drift when the code moves; a line does, and a line that has
moved is a line that says nothing. The outer gate checks that every test
named here **exists** — so the table cannot claim a proof that is not there.

**The terminal door** — `apps/craftkit-repl`.
`help` lists the commands by area and `commands` lists the ids, both
read from the **session** the door drives
(`apps/craftkit-repl/tests/repl.rs::help_lists_the_commands`,
`apps/craftkit-repl/tests/repl.rs::every_listed_command_is_a_real_command`);
`run_command` runs any command by its id
(`apps/craftkit-repl/tests/repl.rs::a_command_runs_and_the_document_changes`);
`plan` asks `doc.plan`
(`apps/craftkit-repl/tests/repl.rs::plan_comes_from_the_command`);
`memory` reports the journal's **size**
(`apps/craftkit-repl/tests/repl.rs::memory_comes_from_the_command`);
the undo is a **command**
(`apps/craftkit-repl/tests/repl.rs::the_undo_is_a_command`); a refusal is
answered with the refusal's own text
(`apps/craftkit-repl/tests/repl.rs::an_unknown_command_is_refused`). There is
no node list and no schema: a person at a prompt reads a line.

**The window** — `apps/craftkit-window`.
`commands` lists the commands
(`apps/craftkit-window/tests/window.rs::the_window_shows_every_command`)
and `commands_outside_panels` the ones no panel shows
(`apps/craftkit-window/tests/window.rs::the_panels_are_shown_and_their_commands_once`);
`open` builds a **form** from the command's own parameters
(`apps/craftkit-window/tests/window.rs::the_form_is_the_commands_own_parameters`);
`run` runs it
(`apps/craftkit-window/tests/window.rs::the_window_runs_a_command_through_the_registry`);
`tree` lists the nodes
(`apps/craftkit-window/tests/window.rs::the_window_draws_the_document_through_the_kinds`);
`plan` asks `doc.plan`
(`apps/craftkit-window/tests/window.rs::the_plan_is_the_commands_work`);
the undo is a **command**
(`apps/craftkit-window/tests/window.rs::the_undo_is_a_command`); the log holds
what the window has to say
(`apps/craftkit-window/tests/window.rs::a_command_that_refuses_says_so`).

**The browser door** — `apps/craftkit-wasm`.
It holds the terminal door's `Repl` and runs it
(`apps/craftkit-wasm/tests/door.rs::the_door_is_the_terminal_door`), so every
cell of that row is that door's cell: the browser door is the terminal
door in a page
(`apps/craftkit-wasm/tests/door.rs::the_door_lists_its_commands`,
`apps/craftkit-wasm/tests/door.rs::a_command_runs_and_the_answer_comes_back`,
`apps/craftkit-wasm/tests/door.rs::the_plan_is_a_command_the_page_can_ask_for`,
`apps/craftkit-wasm/tests/door.rs::an_unknown_command_is_refused`).

**The agent door** — `crates/craftkit-mcp`.
The tools are made from the session's registry, one per command
(`crates/craftkit-mcp/tests/tools.rs::the_tool_list_is_the_command_list`),
and the list is **read when it is asked for**
(`crates/craftkit-mcp/tests/tools.rs::a_command_made_at_run_time_is_a_tool`);
a tool's schema is the **JSON Schema**, and the mapping lives in the SDK
(`crates/craftkit-mcp/tests/tools.rs::a_tools_schema_is_the_json_schema_the_sdk_builds`);
a call runs the command
(`crates/craftkit-mcp/tests/tools.rs::a_tool_call_runs_the_command`) and a
refusal is told
(`crates/craftkit-mcp/tests/tools.rs::a_refused_call_says_so`). The plan, the
undo and the journal are **tools like any other**, because every command is a
tool.

**The bridge** — `apps/craftkit-bridge`.
`tools` is read from the **session** the tools run in, and every tool
carries the SDK's **descriptor**
(`apps/craftkit-bridge/tests/bridge.rs::a_command_made_at_run_time_is_listed_and_can_be_run`,
`apps/craftkit-bridge/tests/bridge.rs::the_tool_list_is_the_projection`);
`plan` asks the `doc.plan` command
(`apps/craftkit-bridge/tests/bridge.rs::the_plan_is_the_projects_plan`);
`journal` answers with the entries
(`apps/craftkit-bridge/tests/bridge.rs::the_journal_is_over_the_wire`);
`resources` is the kinds, the formats and the **manifest**, whose commands
are the session's
(`apps/craftkit-bridge/tests/bridge.rs::the_manifest_names_the_sessions_commands`);
`menu` is the menu tree of the **same** registry
(`apps/craftkit-bridge/tests/bridge.rs::the_menu_and_the_tool_list_are_one_list`);
and `call` runs only what it lists
(`apps/craftkit-bridge/tests/bridge.rs::the_bridge_runs_only_what_it_lists`,
`apps/craftkit-bridge/tests/bridge.rs::a_tool_call_runs_a_command`,
`apps/craftkit-bridge/tests/bridge.rs::a_refused_call_says_so`). Its protocol
is **its own**, so its tools carry `parameters` and not `inputSchema`.

**The a2ui door** — `apps/craftkit-a2ui-server`.
The methods are a2ui's own
(`apps/craftkit-a2ui-server/tests/door.rs::the_methods_are_a2uis`):
`createSurface`, `updateComponents`, `updateDataModel`, `deleteSurface` and
`callRendererFunction`, with `initialize` for the transport's handshake
(`apps/craftkit-a2ui-server/tests/door.rs::the_handshake_is_two_way`). Its
vocabulary is **a2ui's**, not the kit's: it publishes a catalogue of a2ui's
components
(`apps/craftkit-a2ui-server/tests/door.rs::the_catalogue_the_door_serves_is_the_layers_own`),
and a surface is a tree of those
(`apps/craftkit-a2ui-server/tests/door.rs::a_surface_that_carries_a_tree_gets_it_drawn`).

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

- **The a2ui door cannot undo and cannot read the plan.** a2ui has no
  plan, and the door has no history of its own; a surface is changed by
  the commands that make it, and the door does not keep a second door.
- **The browser door has no tree and no schema**, because it is the
  terminal door in a page.
- **The table is checked by hand.** Nothing keeps it true, and the next
  cut — if there is one — is a check that reads the doors and compares.
