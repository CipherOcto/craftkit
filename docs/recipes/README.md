# Recipes

Each recipe is a numbered, copy-pasteable procedure for one recurring change in
this repo. They are written against the code as it is today, not against a
planned shape of it; where something does not exist yet, the recipe says so in a
**Note** rather than describing it as if it were there.

Read `docs/agent-playbook.md` first. It carries the invariants every change has
to respect and the exact commands that check them.

## When to use which

| Recipe | Use it when |
| --- | --- |
| [add-a-command.md](add-a-command.md) | The behaviour you want is something a person does to a document — add, rename, set, evaluate, undo. Every user-visible behaviour is a command. |
| [add-a-saved-field.md](add-a-saved-field.md) | You are adding a field to a `#[derive(Serialize, Deserialize)]` struct in `craftkit-model` (or elsewhere that is written to disk) and every existing `.craftkit` file must still load. |
| [change-the-file-format.md](change-the-file-format.md) | Older designs on disk have to be rewritten into the new shape. A new file format, a renamed key, a structural rewrite. |
| [add-an-icon.md](add-an-icon.md) | A command needs a `Icon("…")` name the UI does not draw yet. |
| [add-a-control-channel-method.md](add-a-control-channel-method.md) | Something an agent needs to do to a running app that no existing method covers. |
| [add-a-crate.md](add-a-crate.md) | A genuinely new layer of code is needed — not a new module in an existing crate. |
| [fix-a-crash.md](fix-a-crash.md) | Someone said the app crashed, or a test panicked, or `Session::execute` returned `internal`. |

## Rules that apply to all of them

1. **Every user-visible behaviour is a command.** If you are about to write a
   behaviour that only one surface can trigger, it is a missing
   `CommandSpec`, not a missing UI button.
2. **Every command parameter is read through `craftkit_engine::params`.** Never
   index the parameter object, never `as_f64().unwrap()`.
3. **Every format change is one new file** in `crates/io/src/format/pending/`.
   Never edit `JSON_STEPS`, never edit an existing pending file, never edit a
   folded `v<N>/` step.
4. **No panics in non-test code.** `unwrap`, `expect`, `panic!`, `todo!`,
   `unreachable!` and `unsafe` are denied at every crate root.
5. **Every change lands with a test** whose name is stated in the recipe.

## Status of the concurrently written crates

`crates/control`, `crates/ui`, `crates/mcp`, `xtask` and `apps/*` are being
written in parallel with these documents. Where a recipe depends on one of them
it describes the **contract** that crate must satisfy and says plainly when the
code is not there yet — `cargo xtask ci`, `cargo xtask layers` and every
`craftkit-cli` subcommand do not exist as of this writing. Read the source when
it lands and reconcile any difference in the PR that carries it.
