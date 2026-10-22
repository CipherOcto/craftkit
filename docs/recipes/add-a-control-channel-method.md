# Add a control-channel method

**When you need this** — something outside the process has to be able to do
something over the control channel that no existing method covers. A new
capability, a new diagnostic, a new UI interaction.

**Before you stop reading, read the rule.** In almost every case you do not
need this recipe:

> **A method is not a command.** If the thing you want to do changes or reads a
> document, it is a `CommandSpec`
> ([add-a-command.md](add-a-command.md)) and `engine.execute` already reaches
> it. A new method is for reaching something that is *not* a command: a
> crash report, a rendered screenshot, a click at a coordinate.

**Before you start**

```
cargo test -p craftkit-control
```

and read
[docs/patterns/one-trust-boundary.md](../patterns/one-trust-boundary.md) — a
control channel is a trust boundary, and every decision below follows from
where that line is drawn.

is green, and you have checked `Method::parse` in
`crates/control/src/protocol.rs` to confirm the name is genuinely unused.

---

## The contract, in full

`crates/control/src/lib.rs` states four rules. Every step below is one of them.

1. **One JSON object per line, in and out.** The line is the frame. No length
   prefix, no header, so `nc 127.0.0.1 PORT` is a usable debugger and a
   transcript is readable. `MAX_LINE` is `craftkit_core::MAX_JSON_TEXT_BYTES`
   (in `crates/control/src/protocol.rs`); over that the connection is closed
   (`ProtocolError::is_fatal`). That limit is on the **request** line: a
   reply is not capped, because a reply may be a picture — `ui.render` answers
   with a base64 PNG of the frame, measured at 4 008 255 bytes for a
   4096 × 4096 one — so a client reads a reply by its newline rather than by a
   fixed buffer. §59.

   The line limit is a limit on **text**, and the parse of it costs up to
   thirty-two times as much memory, so the number is derived from a memory
   budget rather than chosen on its own: `MAX_JSON_TEXT_BYTES` is
   `MAX_JSON_MEMORY_BYTES / JSON_EXPANSION`. The other two readers of
   untrusted JSON — the MCP server's message limit and the CLI's script limit
   — use the same number, and they used to be 4 MiB and 32 MiB. §64.
2. **Loopback only.** The listener binds `127.0.0.1`.
   `the_listener_is_loopback_only_by_default` asserts it. A control channel that accepts a remote connection is
   remote code execution wearing a friendly name.
3. **A token per launch.** Written `0600` to
   `~/.local/share/craftkit/control/PORT.token`, or supplied in
   `CRAFTKIT_CONTROL_TOKEN`. Only the *first* request on a connection carries
   it (`Request::token`, in `crates/control/src/protocol.rs`); a connection
   that does not present it is closed without a reply, so a port scanner learns
   nothing. `CRAFTKIT_CONTROL_DIR` relocates the token file — and only the
   token *file*: a directory that was already there keeps the permissions its
   owner gave it, because the file's own `0600` is what keeps the secret
   (`write_token`, `crates/control/src/token.rs`). §55.
   `the_token_is_only_needed_on_the_first_request` and
   `a_connection_without_the_token_is_closed_with_no_reply`, both in
   `crates/control/src/server.rs`, assert it.
4. **Every reply carries the `id`, including errors**
   (`a_reply_carries_the_id_even_when_it_failed` asserts it; the test name is
   the citation, because a line number here is a promise to keep it in step).

### The methods that exist today

| Wire name | `Method` | What it does |
| --- | --- | --- |
| `engine.execute` | `Execute` | `Session::execute(command, params)` |
| `engine.describe` | `Describe` | `craftkit_engine::catalogue()` — the whole registry |
| `session.state` | `State` | `Session::snapshot_json()` |
| `diagnostics.list` | `Diagnostics` | runs the `diagnostics.list` **command** |
| `diagnostics.crash_reports` | `Crashes` | `action`: `list`, `show`, `replay` |
| `ui`, `ui.click`, `ui.screenshot`, `ui.render` | `Ui` | window interactions; one method, four spellings |
| `ping` | `Ping` | liveness |
| `shutdown` | `Shutdown` | close the connection |

> **Note** — `ui.click`, `ui.screenshot` and `ui.render` parse to `Method::Ui`,
> but they are **not** interchangeable: `Request::ui_action` reads the spelling
> to decide what to do, and `params.action` is consulted only for the bare
> `ui`. A new spelling belongs in `UiAction::method` as well, or the name is
> decoration — which is how `ui.click` came to render instead of click.
> There is no separate `commands.list` method, because
> `engine.describe` already returns the catalogue. `Method::Ui` is answered by
> the host, not by `dispatch`: `ServerOptions::with_ui` takes a handler, the
> desktop app installs one, and a process with no UI leaves it unset — which is
> what answers `no-window`. Do not invent a ninth name where one of these fits.

---

## Steps

### 1. Which shape is this?

There are **two** shapes, and they are not the same work.

**A new method** — a new variant of `Method`. `Method` is a closed enum on
purpose: a typo becomes an error naming the methods that do exist, instead of
a dispatch that silently does nothing. Three places, and all three of them:

- `Method::as_str` — the name on the wire,
- `Method::parse` — the name coming back,
- `Method::all` — the list an unknown method's error message offers.

`every_method_round_trips_through_its_own_name` checks the first two. The
third is what a typo is told to try, and a method that is not in `all()` is a
method nobody is offered.

**A new `ui.*` spelling** — **not** a new variant. `ui`, `ui.render`,
`ui.screenshot` and `ui.click` all reach `Method::Ui`: one method, four
spellings, and the spelling is what says which action is wanted.

> **The method name decides.** `params.action` is consulted only for the bare
> `ui`, which has no spelling of its own. The reverse order is what this
> replaced, and the reason is worth reading: the host used to read the action
> out of `params.action` and ignore the method the caller had typed, so
> `{"method":"ui.click","params":{"x":400,"y":12}}` rendered a 1180×720 PNG and
> answered `rendered 1180x720` — `ok: true`, no error, and no click. §32.

Splitting the spellings into variants of their own brings that back.
`ui.render` stops reaching the host that draws the frame, and answers a
refusal instead — and only two of this crate's tests send a `ui.render`, so
nothing else in the tree notices.

> **The guard grows; it is never edited to let a spelling out.**
> `the_ui_aliases_all_reach_one_method` names all four spellings and asserts
> that they reach one method. A new spelling is **added** to it. Changing it
> to let one out is how the family gets split.

So a new spelling is an `UiAction`'s business, and there are four places:

- `UiAction::parse` — the action's own name, `click` and the rest,
- `UiAction::as_str` — the same name, the other way,
- `UiAction::method` — the method spelling that asks for it on its own, or
  `None` for an action that has no spelling of its own,
- `Request::ui_action` — what the typed method means.

`UiAction::method` returning `None` is not a gap waiting to be filled.
`widgets` is the honest exception: it has always been spelled
`{"method":"ui","params":{"action":"widgets"}}`, and a `ui.widgets` that
`Method::parse` does not accept would be a name that resolves to nothing.

The third of those four, as the tree has it:

```rust
    /// The spelling of the method that asks for this action on its own, or `None`
    /// for [`UiAction::Widgets`], which has always been spelled
    /// `{"method":"ui","params":{"action":"widgets"}}`.
    pub fn method(self) -> Option<&'static str> {
        Some(match self {
            UiAction::Render => "ui.render",
            UiAction::Screenshot => "ui.screenshot",
            UiAction::Click => "ui.click",
            UiAction::Widgets => return None,
        })
    }
```

An unknown action is refused **by name**, and the refusal says which actions
exist — otherwise an agent has nothing to correct. `hostile_ui_actions_are_refused_by_name`
is the test that says so.

To add an *alias* of an existing method instead, touch `parse` only — that is
what the `ui.*` aliases do, and no other file changes.

### 2. Dispatch it

`dispatch` (in `crates/control/src/server.rs`) is pure: no I/O, a
`&mut Session` and a `&Request` in, a `Response` out. That purity is what makes
the whole method layer testable without a socket.

```rust
Method::Ping => Response::ok(&request.id, "pong", json!({ "at": session.at().raw() })),
```

**The `ui.*` family is not dispatched here.** It is answered by the host,
because only the host knows whether there is a window. The shape of that arm:

```rust
Method::Ui => match ui {
    Some(handler) => handler(session, request),
    None => Response::error(&request.id, "no-window", "this process has no window, so `ui.*` cannot run"),
},
```

The tree's arm wraps the handler in a `catch_unwind` as well, because a panic
in host code would otherwise drop the connection with no reply at all.

**The size of a frame is the host's bound, not this crate's.** `ui.render` and
`ui.screenshot` take a `width` and a `height`, and the limits are
`craftkit_ui::raster::MAX_SIZE` (a side) and `craftkit_ui::raster::MAX_PIXELS`
(the area). A method that turns a client-supplied number straight into an
allocation is a denial of service on the process that holds the user's
document — so the bound belongs to whoever draws, and it is a bound, not a
range check invented here.

Rules the surrounding code already follows:

- **Reuse `Response::ok` / `Response::ok_empty` / `Response::error`.** Never hand
  build a `Response` — `message`, `data` and `kind` are skipped when `None`, and
  clients depend on that shape.
- **Errors carry a `kind`.** The kinds the engine produces come from
  `EngineError::kind`: `unknown-command`, `disabled`, `bad-parameter`,
  `invalid-value`, `invalid-document`, `cannot-evaluate`, `kernel`, `internal`.
  A method adds its own only when none of those fit (`no-window`, `poisoned`,
  `not-found`, `unknown-method`).
- **Read params defensively, with a default and a range check.** Never
  `request.params["width"].as_u64().unwrap()` — the JSON is hostile input, and
  it reached you from a socket a port scanner found.
- **Map onto something that already exists.** `Method::Diagnostics` is the
  model: it calls `session.execute("diagnostics.list", …)` rather than
  reimplementing evaluation. `Method::Crashes` calls
  `craftkit_engine::crash::list` / `show` / `replay`. If your method needs new
  behaviour, that behaviour belongs in the engine and gets its own
  [recipe](add-a-command.md) — not here.

### 3. Do not touch `handle`

`handle` does auth, framing and the `shutdown` check. A new method that reaches past it is a new method that works
without a token.

---

## The test you must add

Two, and the second is the one that matters.

**A. Protocol-level.** In `crates/control/src/protocol.rs`'s test module,
beside `a_request_round_trips`:

```rust
#[test]
fn a_render_request_round_trips() {
    let line = r#"{"id":3,"method":"ui.render","params":{"width":640,"height":480}}"#;
    let req = parse_request(line).expect("parses");
    // `ui.render` is a spelling of `Method::Ui`, not a method of its own.
    assert_eq!(req.method(), Ok(Method::Ui));
    assert_eq!(req.params["width"], json!(640));
}
```

**B. Dispatch-level.** In `crates/control/src/server.rs`'s test module. `dispatch`
needs no socket, so this is a unit test:

```rust
#[test]
fn a_ui_request_without_a_window_says_so_rather_than_returning_nothing() {
    let mut s = Session::sample();
    let req = Request { id: json!(3), token: None, method: "ui.render".into(), params: json!({}) };
    let r = dispatch(&mut s, &req);
    // Whatever the outcome, it is one of: a rendered frame, or a refusal that
    // names why. It is never an empty success.
    if !r.ok {
        let kind = r.kind.expect("a failure always names its kind");
        assert!(matches!(kind, "no-window" | "bad-parameter" | "internal"), "{kind}");
        assert!(!r.message.expect("a message").is_empty());
    }
}
```

**C. The spelling, refused by name.** In `crates/control/src/protocol.rs`'s test
module, beside `hostile_ui_actions_are_refused_by_name`:

```rust
#[test]
fn a_ui_action_that_is_not_an_action_is_refused_by_name() {
    let req = Request {
        id: json!(4), token: None, method: "ui".into(),
        params: json!({ "action": "explode" }),
    };
    let r = req.ui_action().expect_err("refused");
    assert_eq!(r.kind, Some("bad-parameter"));
    assert!(r.message.expect("a message").contains("explode"));
}
```

The second of those three is the one that earns its place: a control channel is
reachable by anything that finds the port, and a method that answers
`ok: true` without having done anything is how an agent comes to believe it
clicked something. `MAX_LINE` bounds the *line*; nothing bounds the *answer*
unless your code does.

### Why dispatch-level is the one that matters

A protocol test proves the line parses. It does not prove the method is
reachable, that it answers, or that it fails loudly. `dispatch` being pure is
what makes the second test free — no port, no thread, no token, no sleep. The
socket tests (`round_trip`) are for the
framing and the auth, and you should add one of those only if you changed the
transport.

The assertion to copy is the *shape* of the failure: **a method must always
answer, and an inability must always be named.** Compare
`a_ui_method_without_a_window_says_so_rather_than_pretending`
(in `crates/control/src/server.rs`), which exists for exactly this reason.

---

## The failure this recipe is designed to prevent

### A method that silently succeeds

The worst outcome in this crate is a reply with `ok: true` and no effect. An
agent that receives it will believe it clicked, or rendered, or selected, and
will build everything after that on a false fact. It will not find out for
twenty steps.

`Method::Ui`'s `no-window` arm is the model: *"without one this is an error rather than a silent success, because a
silent success here is how an agent ends up believing it clicked something."*
Copy that. Every `Method` arm must be exhaustive — the enum has no `_` arm, and
adding a variant without a dispatch arm is a compile error, which is the point.

### A method that reaches past the engine

Rule 4 in `crates/control/src/lib.rs`: *"Nothing reaches past the engine. Every
method maps onto something the engine already does. There is no method that
opens a file, spawns a process, or evaluates arbitrary code."* A method that
opens a design file is remote arbitrary file access on a socket. A method that
runs a command id from a parameter that was not validated is arbitrary code
execution wearing a JSON body. If the capability is genuinely needed, it belongs
in `craftkit-engine` behind a command, where it gets the deny list, the
`deny_unknown` check and the panic recovery for free.

### An index in a place that thinks params are trusted

They are not. They arrive from a socket that a port scanner found.
`execute` is careful — `and_then(Json::as_str)`,
`.cloned().unwrap_or(empty_object)`, and an explicit `is_object` check before
`Session::execute` sees them. A new method that writes
`request.params["x"].as_str().unwrap()` has reintroduced the exact panic the
crate-root deny list forbids, in the one place a remote peer can reach.

### A token bypass in the transport

The token is checked once per connection, in `handle`, before `answer` and
therefore before `dispatch`. If you add a code path that serves a request
without going through `handle`, you have removed authentication from it. There
is no second check that will save you.

### A token check that is not the check

The transport can be perfect and the *comparison* can still be wrong. `Token::matches`
is constant-time, and being constant-time is not the same as being exact: it
folded the length difference into a `u8` accumulator, so every length
difference that was a multiple of 256 was lost, and a peer that presented the
token followed by 256 NUL bytes was authenticated. §63.

If you change that comparison — or write another one — the property to keep is
not "it does not take longer when the prefix is wrong". It is **"it says yes
to this and to nothing else"**, and the test has to say so: a test that checks
one wrong token is refused has not checked the space of tokens that are
accepted.

