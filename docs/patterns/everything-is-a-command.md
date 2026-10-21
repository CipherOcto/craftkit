# Everything is a command

## The problem

An editor can change a document through many doors: a toolbar button, a
keyboard shortcut, a menu item, a drag on a timeline, a script file, a socket
message from an agent, an MCP tool call. Each door is a place to forget a
guard, and each is a separate thing to document, test and keep working. Six
front ends and six code paths is six times the surface and one guarantee.

The other half of the problem is discoverability. An agent that can only read a
source tree cannot tell which operations exist, what they take, whether they
are undoable, or whether they are legal right now.

## The rule

**Every user-visible behaviour is a `CommandSpec` with an id, and the only way to
run one is `Session::execute`.**

```rust exact
// crates/engine/src/command.rs
pub struct CommandSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub area: Area,
    pub icon: Icon,
    pub shortcut: Option<&'static str>,
    pub params: &'static [ParamDoc],
    pub doc: &'static str,
    pub mutates: bool,
    pub undoable: bool,
    pub enabled: fn(&Session) -> bool,
    pub run: fn(&mut Session, &Params<'_>) -> Result<Outcome, EngineError>,
}
```

Three fields do the load-bearing work:

- **`params`** is the single declaration of what the command takes. The reader,
  the docs generator, the MCP schema and the CLI help all read it, so they cannot
  disagree. `Session::execute` feeds `CommandSpec::param_names()` to
  `Params::deny_unknown`, which means an undeclared key is refused by the
  registry rather than by each command remembering to check.
- **`mutates`** decides whether the snapshot becomes an undo entry, whether the
  document is marked dirty, and whether the evaluator re-runs. A read-only
  command must not mark the project dirty.
- **And it is a promise the engine keeps.** A command that declares
  `mutates: false` and changes the document is refused and rolled back. The
  flag is published to agents in the command schema, and an agent that reads
  it as *"safe to use as a probe"* has to be right: the change would not be
  validated, would not be undoable, and would not mark the project dirty, so
  the design would change and the app would not know that it had. §96.
- **`undoable`** is separate from `mutates` because `history.undo` and
  `history.redo` change the document *by* restoring a snapshot. Pushing that as
  a new step would push an entry pointing at the state just left behind, so a
  second undo would appear to do nothing and the redo branch would be cleared by
  its own undo.
- **A command that does not mutate cannot push an undo entry**, so `undoable`
  is only meaningful together with `mutates`. The field's own comment says
  *"whether a successful run pushes an undo entry"*, and that is not true for a
  command that does not mutate: the push happens inside the `mutates` branch.
  Four commands — `playback.seek`, `plan.evaluate`, `diagnostics.list` and
  `session.stats` — declared `undoable: true` and can never push one, and the
  flag is published to agents in the command schema. They are `false` now, and
  `no_command_that_does_not_mutate_is_undoable` keeps them that way. §94.

A command, in full:

```rust
// crates/engine/src/commands.rs
pub static DOC_RENAME: CommandSpec = CommandSpec {
    id: "doc.rename",
    label: "Rename plan",
    area: Area::Document,
    icon: Icon("pencil"),
    shortcut: None,
    doc: "Give the plan a new name.",
    params: &[ParamDoc::req("name", Doc::Text, "The new name.")],
    mutates: true,
    undoable: true,
    enabled: always,
    run: |s, p| {
        let name = required_text(p, "name")?;
        check_name(&name)?;
        s.doc_mut().meta.name = name.clone();
        Ok(Outcome::with(format!("renamed to `{name}`"), serde_json::json!({ "name": name })))
    },
};
```

The registry currently holds 23 commands across 7 areas — `document`,
`node`, `property`, `keyframes`, `playback`, `history`, `debug` — from
`doc.rename` to `session.stats`. Both numbers are checked: `cargo xtask lists`
fails if either of them stops matching the registry, because a number written
by hand about a thing that grows is stale by construction (§77). `Icon` is just a name: the UI draws it from its
own icon set, so a command cannot acquire an implicit dependency on a particular
front end.

The second half of the rule:

**`execute` never opens a dialog, prompts, or touches the clipboard.** Those are
interactive concerns and belong to `ui.start`, which a front end calls instead.
A programmatic caller — a script, an agent, the CLI — has no way to answer a
modal question, so a command that asks one is a command that hangs.

## In the code

`Session::execute` is where the id, the enabled check, the parameter check, the
snapshot and the single `catch_unwind` all live, in that order:

```rust
pub fn execute(&mut self, id: &str, params: &Json) -> Result<Outcome, EngineError> {
    let Some(spec) = command::lookup(id) else {
        let err = EngineError::Unknown(id.to_string());
        self.log.record(id, false, &err.to_string());
        return Err(err);
    };
    if !(spec.enabled)(self) {
        let err = EngineError::Disabled { id: id.to_string(), reason: disabled_reason(self, id) };
        self.log.record(id, false, &err.to_string());
        return Err(err);
    }
    let p = Params::new(params);
    p.deny_unknown(&spec.param_names())
        .map_err(|source| EngineError::Param { id: id.to_string(), source })?;
    // … snapshot, then catch_unwind around (spec.run)(self, &p)
}
```

Everything the front ends share is above that line; everything they cannot skip
is below it.

## When NOT to use it

- **A pure read that computes rather than inspects** — a preview that measures
  without touching the document. If it is worth calling from an agent, it is
  still a command (`plan.evaluate`, `diagnostics.list`), just with
  `mutates: false`.
- **Anything that mutates the document outside `run`.** A command body that
  reaches past `Session::doc_mut` into a shared handle has opted out of undo,
  the dirty flag and the panic restore.
- **Continuous interaction.** Dragging a playhead or a slider is a stream of
  commands, not one special "drag" operation. Give it a command that takes a
  time, and let the front end own the gesture.
- **As a reason to make every internal function a command.** `History::push`,
  `Evaluator::invalidate` and `Document::add_node` stay ordinary functions. The
  rule is about *user-visible behaviour*, not about making every line of state
  reachable over the wire.

## See also

- [`clone-before-edit.md`](clone-before-edit.md) — the snapshot `execute` takes,
  and the three jobs it does.
- [`never-index-a-parameter.md`](never-index-a-parameter.md) — reading `params`.
- [`../architecture.md`](../architecture.md#data-flow)
