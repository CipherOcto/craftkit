# Exact time, not float seconds

## The problem

A timeline has frames. A frame is a count, and counts have to be exact. Store
time as `f64` seconds and three things go wrong at once:

- `0.1` is not `0.1`. Summing a hundred frame durations drifts, and the drift is
  different on every machine.
- 29.97 fps is `30000/1001`, not a decimal. Rounding it to `29.97` puts every
  frame boundary in the wrong place, permanently.
- Key times stored as seconds cannot be compared for equality, so "did this
  node change?" — the entire basis of incremental evaluation — becomes a
  floating-point comparison.

## The rule

**Key times are `Tick`, never seconds. Seconds exist only as an input format.**

```rust exact
// crates/core/src/tick.rs
/// Ticks in one second.
pub const TICKS_PER_SECOND: i128 = 254_016_000_000;

/// An exact instant on the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tick(i128);
```

Three properties of that choice are doing the work:

| Choice | What it buys |
|---|---|
| `i128` integer ticks | `Tick` derives `Eq` and `Ord`, so equality and ordering are free and exact |
| `TICKS_PER_SECOND = 254_016_000_000` | an exact binary fraction of a second, so ticks↔frame-boundary conversion is integer arithmetic with no accumulated rounding — including the 29.97 NTSC timebase |
| `FrameRate` is a newtype over `u32` | the rate is validated at construction; `FrameRate(0)` is not constructible outside `FrameRate::new` |

And every arithmetic operation on `Tick` **saturates**. A hostile or nonsensical
time becomes the nearest representable time; it never wraps, never panics, and
never produces a time in the past from an addition. Code can rely on that: an
out-of-range time is clamped, not an error to handle at every call site.

**The range is the one a reply can hold**: `i64::MIN` to `u64::MAX`, the
integers a JSON number holds. It was wider — `i128::MIN / 4` to
`i128::MAX / 4` — and that broke the promise above: a saturating time could
land outside a JSON number, and the command that reported it built its reply
with `json!`, which converts an interpolated value with `to_value(…).unwrap()`
**inside the macro**, where the lint that forbids `unwrap` cannot see it. A
hostile `time` parameter panicked a command. The bounds are now the format's
own: the file format is wider (a bare integer is read straight into an
`i128`), so the reader refuses a file whose range the model will not hold.
§60.

```rust
// crates/core/src/tick.rs
/// Convert from seconds, reporting an unusable value instead of absorbing it.
///
/// Prefer this in command parameters, where silently turning `NaN` into
/// frame 0 would hide a bug.
pub fn try_from_seconds_f64(what: impl Into<String>, s: f64) -> Result<Self, crate::CoreError> {
    crate::finite(what, s)?;
    Ok(Self::from_seconds_f64(s))
}
```

The two constructors are a deliberate pair. `from_seconds_f64` maps `NaN` to
`Tick::ZERO`, which is right for read-only arithmetic where clamping is the
documented behaviour; `try_from_seconds_f64` reports it, which is right for a
command parameter somebody typed.

## In the code

A command takes an instant and a rate, never a number of seconds it will store:

```rust
// crates/engine/src/commands.rs — playback.seek
let to = match opt_time(p, "seconds")? {
    Some(t) => t,
    None => match p.int("frame")? {
        Some(f) => Tick::from_frames_i64(f, s.document().meta.frame_rate()),
        None => /* the current playhead */,
    },
};
```

`opt_time` accepts a number of seconds *or* a string like `"12f"` for frames or
`"120t"` for raw ticks, and converts through the checked constructor:

```rust
fn opt_time(p: &Params<'_>, key: &str) -> Result<Option<Tick>, EngineError> {
    let bad = |detail: String| {
        EngineError::Param { id: String::new(), source: craftkit_core::ParamError::new(key, detail) }
    };
    match p.raw(key) {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => {
            let secs = n.as_f64().ok_or_else(|| bad(format!("{n} is not a number of seconds")))?;
            Ok(Some(Tick::try_from_seconds_f64("time", secs).map_err(|e| bad(e.to_string()))?))
        }
        Some(serde_json::Value::String(s)) => parse_time_text(s).map(Some).ok_or_else(|| {
            bad(format!(
                "`{s}` is not a time; use seconds (1.5), frames (12f) or ticks (120t)"
            ))
        }),
        Some(other) => Err(bad(format!("a {} is not a time", params::kind_of(other)))),
    }
}
```

The suffix is parsed, not guessed: `parse_time_text` strips a trailing `f`,
`F`, `t` or `T` and dispatches to `from_frames_i64` or `from_raw`. Note that the
`12f` form is read at the format's default rate of 24 fps, while the numeric
`frame` parameter above is read at `s.document().meta.frame_rate()`. Prefer the
numeric parameter when the document's rate is not 24 — the string form is the
one that silently assumes.

Frames are the same idea one level up: `Tick::from_frames_i64`,
`frames_i64`, `round_to_frames` and `FrameRate::frame_of` all take a `FrameRate`
and use `div_euclid`, so a negative or fractional instant clamps to a frame
rather than panicking or truncating toward zero.

Interpolation follows the same rule. A keyframe's curve is described on the
fields it points *back* from (`in_*`), which keeps the keyframe *list* sorted by
time regardless of the curve — the property the evaluator's cache depends on.

That list is sorted because the model keeps it sorted, and a **file** does
not go through the model's constructors: it is deserialized straight into the
model, so the list's rules are *checked* by `Keyframes::check` rather than
kept on the way in. An unsorted list does not sample to a tidier answer, it
samples to the **wrong one** — the key that is earliest in time is never
reached — so the reader refuses such a file. A keyframe's **time** is checked
against the range a reply could carry, for the same reason the document's own
range is: a tick outside it would make every reply that mentions it fail
inside `json!`. §60, §91.

## When NOT to use it

- **For display.** Seconds, `MM:SS:FF` and "1.5 s" are output formats. Convert
  at the edge with `seconds_f64()` and never round-trip a `Tick` through a
  string.
- **For durations in a config file** that is not on the timeline — a timeout, a
  sample rate. Those are not instants and do not need a frame boundary.
- **To avoid writing the conversion once.** The conversion is the reason the
  type exists; a shortcut around it reintroduces the float it was meant to
  remove.
- **When a rate is genuinely unknown.** `FrameRate::new` refusing 0 is
  the point: a document with no frame rate has an unusable timeline, and saying
  so at construction beats dividing by zero later. How *fast* a document may run
  is the model's business — `MAX_FRAME_RATE` — and the same limit is what the
  `doc.range` command enforces, so the two can never disagree about a rate a
  document may carry. §89.

## See also

- [`../architecture.md`](../architecture.md#incremental-evaluation) — the
  equality of `Tick` that makes prefix reuse sound.
- [`never-index-a-parameter.md`](never-index-a-parameter.md) — reading the time
  parameter without indexing it.
