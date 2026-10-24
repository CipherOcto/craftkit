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
/// frame 0 would hide a bug — and where an instant outside the range a reply
/// can carry has to be **refused** rather than clamped.
pub fn try_from_seconds_f64(what: impl Into<String>, s: f64) -> Result<Self, crate::CoreError> {
    crate::finite(what.clone(), s)?;
    let ticks = Self::ticks_of_seconds(s);
    if ticks > MAX_INSTANT_TICKS || ticks < MIN_INSTANT_TICKS {
        return Err(crate::CoreError::OutOfRange { .. });
    }
    Ok(Self::from_raw(ticks))
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
let rate = s.document().meta.frame_rate();
let to = match opt_time(p, "seconds", rate)? {
    Some(t) => t,
    None => match p.int("frame")? {
        // A frame number is a count of this document's timebase, so it is an
        // instant in disguise — and it is checked against the same range the
        // other three spellings are checked against. §188.
        Some(f) => Tick::try_from_frames_i64("frame", f, rate)?,
        None => /* the current playhead */,
    },
};
```

`opt_time` accepts a number of seconds *or* a string like `"12f"` for frames or
`"120t"` for raw ticks, and converts through the checked constructor:

```rust
fn opt_time(p: &Params<'_>, key: &str, rate: FrameRate) -> Result<Option<Tick>, EngineError> {
    let bad = |detail: String| {
        EngineError::Param { id: String::new(), source: craftkit_core::ParamError::new(key, detail) }
    };
    match p.raw(key) {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => {
            let secs = n.as_f64().ok_or_else(|| bad(format!("{n} is not a number of seconds")))?;
            // The number is the seconds spelling, and it is checked against
            // the tree's range like the other two. §188.
            Tick::try_from_seconds_f64("time", secs).map(Some).map_err(|e| bad(...))
        }
        Some(serde_json::Value::String(s)) => match parse_time_text(s, rate) {
            Ok(t) => Ok(Some(t)),
            // Two reasons, and they are different problems: the text does not
            // name an instant, or it names one that is too big.
            Err(TimeError::Shape) => Err(bad(format!(
                "`{s}` is not a time; use seconds (1.5), frames (12f) or ticks (120t)"
            ))),
            Err(TimeError::Range) => Err(bad(past_the_range(&format!("`{s}`")))),
        },
        Some(other) => Err(bad(format!("a {} is not a time", params::kind_of(other)))),
    }
}
```

The suffix is parsed, not guessed: `parse_time_text` strips a trailing `f`,
`F`, `t` or `T` and dispatches to `try_from_frames_i64` or `try_from_raw`. The
rate is a **parameter** of the reader, not something the reader knows: the
`12f` form is a frame *of this document*, so a 30 fps plan must not read it
at 24 fps.

**All three spellings answer with an instant inside the tree's range**, and
that range is the one a reply can carry — `Tick::MIN` to `Tick::MAX`, the
integers a JSON number holds (§60). The model already refuses a document
whose range is outside it (§91), so a parameter outside it is refused the
same way: the reading forms (`try_from_raw`, `try_from_frames_i64`,
`try_from_seconds_f64`) compare the **tick count** against that bound and
refuse, instead of taking the nearest representable instant. Before this the
three spellings had three different ranges, and the seconds spelling kept
±1e9 s — a range the tick type cannot hold at all, so every instant past the
real one was silently clamped. §188.

The seconds spelling converts in two steps — the whole seconds in integer
arithmetic, the fraction afterwards — so the count it is checked against is
exact to within a thousandth of a tick, and the bound is checked in the same
unit the tick spelling is checked in.

The counting forms also refuse a **fraction**: a frame is a count, a count
has to be exact, and `1.5f` truncated to one frame is a different instant than
the one that was asked for. The seconds spelling is not in that rule: `1.5`
seconds is a perfectly good instant, and the tree's own message offers it as
the example. §188.

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
