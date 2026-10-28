//! What a parameter looks like on paper, and the readers that match.
//!
//! [`Doc`] is the type a [`crate::command::ParamDoc`] declares. It exists so the
//! four readers of the parameter table — the docs generator, the MCP schema, the
//! CLI help and the reader itself — cannot disagree about what a command takes.
//!
//! It also exists for a second reason. When a new command is written, the
//! temptation is to reach for a JSON value directly. This module is the answer
//! to that temptation: every reader here goes through
//! [`craftkit_foundation::Params`], which never indexes and always validates.
//!
//! **This module is one of the two homes of the raw reader.**
//! [`craftkit_foundation::Params::raw_json`] hands back a `&Json`, and a `&Json` can
//! be indexed — `json["key"]` panics on anything that is not an object and
//! answers `null` for a key that is not there. So a command's parameter is
//! read **here**, where the shape is matched with a `match` and never indexed,
//! and the other home is `crates/foundation/src/params.rs`. `cargo xtask lists`
//! refuses the tree when a third file calls it. §136.

use craftkit_core::{FrameRate, NodeId, Tick};
#[cfg(test)]
use craftkit_foundation::Color;
use craftkit_foundation::{CoreError, ParamError, Params, Value};
use craftkit_model::{Easing, NodeKind};
use serde_json::Value as Json;

use crate::command::ParamDoc;
use crate::error::EngineError;

/// The documented type of one parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Doc {
    /// A finite number within ±`MAX_MAGNITUDE`.
    Number,
    /// A bounded string.
    Text,
    Bool,
    /// `#rrggbb` or `#rrggbbaa`.
    Color,
    /// A node id, as a whole number.
    ///
    /// Not a name: the reader refuses one here (*"`Level` is a name, not an
    /// id; names are resolved by `node_name`"*), and the name is the
    /// parameter's alternative. §116.
    NodeRef,
    /// One of the node kinds.
    NodeKind,
    /// An instant, in ticks. Accepts seconds, frames or raw ticks.
    Time,
    /// A property path, or `@id`.
    PropertyPath,
    /// An easing preset name.
    Easing,
}

impl Doc {
    /// The name used in JSON schemas and generated docs.
    pub fn json_name(self) -> &'static str {
        match self {
            Doc::Number => "number",
            Doc::Text => "text",
            Doc::Bool => "bool",
            Doc::Color => "color",
            Doc::NodeRef => "node-ref",
            Doc::NodeKind => "node-kind",
            Doc::Time => "time",
            Doc::PropertyPath => "property-path",
            Doc::Easing => "easing",
        }
    }
}

/// What a person typed, and the parameter it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Typed {
    /// The parameter the value belongs to.
    ///
    /// Not always the one that was asked for. A parameter with an
    /// alternative is **one parameter in two spellings** (§116), and which
    /// spelling the text belongs to depends on the text: an id belongs to
    /// `node` and a name belongs to `node_name`.
    pub name: &'static str,
    /// The value, ready for the reader.
    pub value: Json,
}

/// Turn what a person typed into the JSON a command's reader accepts.
///
/// The readers are strictly typed — `p.num`, `p.text`, `p.bool` — so an
/// interface that collects text has to convert it somewhere, and the place
/// that does it has to be the same place that decides what a value means.
/// A second parser, in the interface, would be a second answer to "what
/// does this field mean", and the two would drift. §118.
///
/// The empty string is **not** a value: a field the person left blank is
/// one the caller did not give, and the caller decides what to do about a
/// parameter it did not give. This function is only asked about text that
/// is there.
///
/// # Errors
///
/// The message says what was wrong with the text, in the same words the
/// readers use, so a person sees one kind of complaint whether the text
/// came from the keyboard or from a script.
pub fn text_to_param(param: &ParamDoc, text: &str) -> Result<Typed, String> {
    let trimmed = text.trim();
    match param.kind {
        Doc::Number => {
            let n: f64 = trimmed.parse().map_err(|_| format!("`{trimmed}` is not a number"))?;
            if !n.is_finite() {
                return Err(format!("`{trimmed}` is not a finite number"));
            }
            Ok(Typed { name: param.name, value: serde_json::json!(n) })
        }
        Doc::Bool => {
            let b = match trimmed.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => true,
                "false" | "no" | "off" | "0" => false,
                _ => {
                    return Err(format!("`{trimmed}` is not a yes or a no; write true or false"));
                }
            };
            Ok(Typed { name: param.name, value: serde_json::json!(b) })
        }
        Doc::NodeRef => {
            // An id is a whole number and a name is anything else, so the text
            // says which spelling it is. The alternative is the name
            // spelling — §116 made `node` and `node_name` one parameter in
            // two spellings — and a name here is not a mistake: it is the
            // other spelling of the same parameter.
            if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                let id: u64 = trimmed
                    .parse()
                    .map_err(|_| format!("`{trimmed}` is not a node id; ids are whole numbers"))?;
                Ok(Typed { name: param.name, value: serde_json::json!(id) })
            } else {
                match param.alternative {
                    Some(alternative) => Ok(Typed { name: alternative, value: serde_json::json!(trimmed) }),
                    None => Err(format!("`{trimmed}` is a name, not an id")),
                }
            }
        }
        // A time is given as text because a time has three spellings —
        // seconds, a frame (`12f`) and raw ticks (`120t`) — and the reader
        // knows the document's frame rate, so only the reader can say which
        // instant the text means. §118.
        Doc::Time => Ok(Typed { name: param.name, value: serde_json::json!(trimmed) }),
        // The rest are text as they are: the reader checks the shape.
        // `node-kind`, `easing` and `color` are all words, and a word the
        // reader refuses is refused with the reader's own list of the words
        // it accepts.
        Doc::Text | Doc::Color | Doc::NodeKind | Doc::PropertyPath | Doc::Easing => {
            Ok(Typed { name: param.name, value: serde_json::json!(trimmed) })
        }
    }
}

/// The required subject of almost every command: which node.
pub fn node_ref(p: &Params<'_>, key: &str) -> Result<NodeId, ParamError> {
    match p.raw_json(key) {
        Some(Json::Number(n)) => {
            let raw = n.as_u64().ok_or_else(|| {
                ParamError::new(key, format!("{n} is not a node id; ids are whole numbers"))
            })?;
            if raw > craftkit_core::ids::IdCounter::MAX {
                return Err(ParamError::new(key, format!("{raw} is larger than any id")));
            }
            Ok(NodeId(raw))
        }
        Some(Json::String(s)) => Err(ParamError::new(
            key,
            format!("`{s}` is a name, not an id; names are resolved by `{key}_name`"),
        )),
        Some(_) => Err(ParamError::new(key, "expected a node id (a whole number)")),
        None => Err(ParamError::new(key, "this parameter is required")),
    }
}

/// The node a command addresses, by id or by name.
///
/// Resolving both here is deliberate: an agent that has just created a node
/// knows its id, while a person reading a log knows its name. Forcing one
/// spelling on both would make one of them write a lookup first.
pub fn node_by_id_or_name(doc: &craftkit_model::Document, p: &Params<'_>) -> Result<NodeId, ParamError> {
    // `node` is tried first because an agent that just created a node has its
    // id; a person reading a log has its name.
    if let Ok(id) = node_ref(p, "node") {
        return if doc.get(id).is_some() {
            Ok(id)
        } else {
            // `id.get()`, not `{id}`: `Display` writes `n2`, which `node_ref` rejects as
            // an id ("is a name, not an id"). A message that quotes back a form
            // the parameter cannot accept is worse than no message.
            Err(ParamError::new("node", format!("there is no node with id {}", id.get())))
        };
    }
    match p.text("node_name", craftkit_model::limits::MAX_NAME)? {
        Some(name) => doc
            .find_by_name(&name)
            .map(|n| n.id)
            .ok_or_else(|| ParamError::new("node_name", format!("there is no node named `{name}`"))),
        None => Err(ParamError::new("node", "pass `node` (an id) or `node_name` (a name)")),
    }
}

/// The phrase the tree uses when an instant is past its range.
///
/// **One phrase for every spelling of a time.** A caller can tell *why* a
/// value was refused without knowing how the instant was written, and the
/// tree's own property can check that the three spellings answer alike.
/// §188.
pub const PAST_THE_RANGE: &str = "is past the tree's range";

/// An instant, in the three spellings a time has.
///
/// Seconds, a frame (`12f`) and raw ticks (`120t`). The frame spelling needs
/// the document's own rate, which is why the rate is a parameter and not
/// something this module knows.
///
/// The shape is matched, never indexed: a number is a number of seconds, a
/// string is a spelling, and anything else is refused by name.
pub(crate) fn opt_time(p: &Params<'_>, key: &str, rate: FrameRate) -> Result<Option<Tick>, EngineError> {
    let bad = |detail: String| EngineError::Param { id: String::new(), source: ParamError::new(key, detail) };
    match p.raw_json(key) {
        None => Ok(None),
        Some(Json::Number(n)) => {
            let secs = n.as_f64().ok_or_else(|| bad(format!("{n} is not a number of seconds")))?;
            // A number is the seconds spelling, and it is checked against the
            // tree's range like the other two — and refused with the same
            // words, because the same hostile value must not get a different
            // answer depending on how it was written. §188.
            Tick::try_from_seconds_f64("time", secs).map(Some).map_err(|e| {
                bad(match e {
                    CoreError::NotFinite { .. } => format!("{secs} is not a time"),
                    _ => past_the_range(&secs.to_string()),
                })
            })
        }
        Some(Json::String(s)) => match parse_time_text(s, rate) {
            Ok(t) => Ok(Some(t)),
            Err(TimeError::Shape) => {
                Err(bad(format!("`{s}` is not a time; use seconds (1.5), frames (12f) or ticks (120t)")))
            }
            Err(TimeError::Range) => Err(bad(past_the_range(&format!("`{s}`")))),
        },
        Some(other) => Err(bad(format!("{} is not a time", kind_of(other)))),
    }
}

/// The message for an instant that is past the tree's range.
///
/// The words say **what** is wrong: the shape of `1e300f` is fine — it is a
/// count of frames — and the *size* is not. A refusal that says "is not a
/// time" for a value that is plainly a time sends the reader looking for a
/// spelling that would work, and there is none. §188.
///
/// The bound is given in **seconds**, because seconds are what a person
/// writes, and the tree's range is the range a reply can carry: a little
/// over twenty hours after zero, and a little over ten hours before it.
fn past_the_range(value: &str) -> String {
    format!(
        "{value} {PAST_THE_RANGE}: an instant is at most {} seconds after zero, or {} seconds before it",
        craftkit_core::MAX_INSTANT_TICKS / craftkit_core::TICKS_PER_SECOND,
        -(craftkit_core::MIN_INSTANT_TICKS / craftkit_core::TICKS_PER_SECOND)
    )
}

/// Why a time was refused.
///
/// Two reasons, and they are different problems: the text is not a time at
/// all, or it is a time and it is too big. §188.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeError {
    /// The text does not name an instant.
    Shape,
    /// The text names an instant past the tree's range.
    Range,
}

/// The three spellings of a time, in text.
///
/// A trailing `f` is a frame and a trailing `t` is a tick; anything else is a
/// number of seconds. The frame is a frame **of this document**: the rate
/// belongs to the document, so hard-coding 24 here would silently give a 30
/// fps plan the wrong instant, which is the sort of bug that only shows up in
/// someone else's project.
///
/// **All three spellings answer with an instant inside the tree's range.**
/// §188: the seconds spelling always refused an instant past
/// `MAX_MAGNITUDE` seconds, and the frame and tick spellings did not — they
/// took the nearest representable instant and said nothing, so the same
/// hostile value got three different answers depending on how it was
/// spelled. The counting forms also refuse a **fraction**: a frame is a
/// count, a count has to be exact, and `1.5f` truncated to one frame is a
/// different instant than the one that was asked for.
fn parse_time_text(s: &str, rate: FrameRate) -> Result<Tick, TimeError> {
    let t = s.trim();
    // **The tick spelling is the core's.** It is the spelling the tree
    // publishes its instants in (§268), and the core is where that spelling
    // is written and read — so the reader here and the writer there cannot
    // drift apart.
    if t.ends_with('t') || t.ends_with('T') {
        return Tick::parse_ticks(t).map_err(|e| match e {
            CoreError::OutOfRange { .. } => TimeError::Range,
            _ => TimeError::Shape,
        });
    }
    let (body, factor) = match t.chars().last() {
        Some('f') | Some('F') => (&t[..t.len() - 1], 'f'),
        _ => (t, 's'),
    };
    let Ok(n) = body.trim().parse::<f64>() else { return Err(TimeError::Shape) };
    match factor {
        's' => Tick::try_from_seconds_f64("time", n).map_err(|e| match e {
            CoreError::NotFinite { .. } => TimeError::Shape,
            _ => TimeError::Range,
        }),
        // The casts are from a float, and a float cast **saturates**: a
        // count past the representable range becomes the largest one, and the
        // range check below refuses it. A cast from one integer to a narrower
        // one truncates instead, and `i128::MAX as i64` is `-1` — which would
        // turn a hostile count into an instant just before zero. §188.
        'f' => {
            let frames = whole_count(n).ok_or(TimeError::Shape)? as i64;
            Tick::try_from_frames_i64("time", frames, rate).map_err(|_| TimeError::Range)
        }
        _ => Err(TimeError::Shape),
    }
}

/// A whole number, and nothing else.
///
/// The text is read as a number, and a number may be a fraction. A frame is a
/// count and a tick is a count, and the tree's rule is that counts have to be
/// exact — so a fraction is refused rather than truncated. §188.
fn whole_count(n: f64) -> Option<f64> {
    if !n.is_finite() || n.fract() != 0.0 {
        return None;
    }
    Some(n)
}

/// A node kind, by its label.
///
/// The shape is matched, never indexed.
pub(crate) fn opt_kind(p: &Params<'_>, key: &str) -> Result<Option<NodeKind>, EngineError> {
    let bad = |detail: String| EngineError::Param { id: String::new(), source: ParamError::new(key, detail) };
    let Some(raw) = p.raw_json(key) else { return Ok(None) };
    let text = match raw {
        Json::String(s) => s.clone(),
        other => return Err(bad(format!("{} is not a node kind; expected text", kind_of(other)))),
    };
    NodeKind::all().iter().find(|k| k.label() == text).copied().map(Some).ok_or_else(|| {
        bad(format!("`{text}` is not a kind; expected one of source, filter, sink, cue, comment"))
    })
}

/// An easing preset, by its name.
///
/// The shape is matched, never indexed.
/// An easing, read by name.
///
/// §277: the tree publishes a keyframe's easing as **`ease`** — in the
/// command's reply and in the file — and the command took it as `easing`, so
/// the published name was refused: "unknown parameter". **A published name
/// is a value the command accepts**, so the commands read both spellings ✓.
pub(crate) fn opt_easing(p: &Params<'_>, key: &str) -> Result<Option<Easing>, EngineError> {
    let Some(raw) = p.raw_json(key) else { return Ok(None) };
    // **The command's names first**, then **the file's own spelling**.
    //
    // §273: the tree's files spell an easing the way serde spells it —
    // `"ease_in_out"`, or an object for a bezier — and the command refused
    // both, so a design the tree wrote could not be given back to the tree's
    // command. The door takes back what the tree publishes.
    let easing = match raw {
        Json::String(s) => Easing::preset(s).or_else(|| Easing::from_stored(raw)),
        other => Easing::from_stored(other),
    };
    easing.map(Some).ok_or_else(|| EngineError::Param {
        id: String::new(),
        source: ParamError::new(
            key,
            format!(
                "`{}` is not an easing; expected linear, hold, easeIn, easeOut or easeInOut, \
                 or a bezier with its control points, bezier(outX,outY), or an easing the way \
                 the file spells it",
                match raw {
                    Json::String(s) => s.as_str(),
                    _ => "(a value)",
                }
            ),
        ),
    })
}

/// A value for a property, read as whatever the document will store.
pub fn value_for(p: &Params<'_>, key: &str) -> Result<Option<Value>, ParamError> {
    let Some(raw) = p.raw_json(key) else { return Ok(None) };
    // **The core reads a value**, because the core is where a value's
    // spelling lives — and the tree publishes values in that spelling, so a
    // published value is a value this accepts. §269.
    Value::from_parameter(key, raw).map(Some).map_err(|e| ParamError::new(key, e.to_string()))
}

/// The word used in a message to name a JSON type.
///
/// **The core's word.** §269: the tree had three copies of it — here, in the
/// MCP and in the control channel — and they did not agree: one said "a bool"
/// and another "a boolean", and this one said "a object cannot be a property
/// value", where the article was wrong.
pub fn kind_of(v: &Json) -> &'static str {
    craftkit_foundation::value::json_kind(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ParamDoc;
    use serde_json::json;

    /// A parameter that takes a node, by id or by name.
    fn node_param() -> ParamDoc {
        ParamDoc::either("node", Doc::NodeRef, "The node's id.", "node_name")
    }

    #[test]
    fn a_number_is_read_as_a_number() {
        let param = ParamDoc::req("fps", Doc::Number, "The frame rate.");
        let typed = text_to_param(&param, " 24 ").expect("a number");
        assert_eq!(typed.name, "fps");
        assert_eq!(typed.value, json!(24.0));
    }

    #[test]
    fn a_number_that_is_not_a_number_says_so() {
        let param = ParamDoc::req("fps", Doc::Number, "The frame rate.");
        let message = text_to_param(&param, "twenty four").expect_err("not a number");
        assert!(message.contains("not a number"), "{message}");
    }

    #[test]
    fn a_number_that_is_not_finite_is_refused() {
        let param = ParamDoc::req("fps", Doc::Number, "The frame rate.");
        let message = text_to_param(&param, "inf").expect_err("not finite");
        assert!(message.contains("finite"), "{message}");
    }

    #[test]
    fn a_yes_or_a_no_is_read_as_a_bool() {
        let param = ParamDoc::opt("muted", Doc::Bool, "Whether it is muted.");
        assert_eq!(text_to_param(&param, "yes").expect("yes").value, json!(true));
        assert_eq!(text_to_param(&param, "no").expect("no").value, json!(false));
        assert_eq!(text_to_param(&param, "TRUE").expect("true").value, json!(true));
    }

    #[test]
    fn a_yes_or_a_no_that_is_neither_says_what_to_write() {
        let param = ParamDoc::opt("muted", Doc::Bool, "Whether it is muted.");
        let message = text_to_param(&param, "maybe").expect_err("neither");
        assert!(message.contains("true"), "{message}");
    }

    #[test]
    fn a_node_id_and_a_node_name_are_the_same_parameter_in_two_spellings() {
        // The whole number is the id spelling and the name is the name
        // spelling: §116 made them one parameter, and the text says which
        // one it is.
        let param = node_param();
        let by_id = text_to_param(&param, "7").expect("an id");
        assert_eq!(by_id.name, "node");
        assert_eq!(by_id.value, json!(7));

        let by_name = text_to_param(&param, "Delay").expect("a name");
        assert_eq!(by_name.name, "node_name");
        assert_eq!(by_name.value, json!("Delay"));
    }

    #[test]
    fn a_number_that_is_not_a_whole_number_is_a_name_and_the_reader_says_so() {
        // The rule is "a whole number is an id, anything else is a name" —
        // so `7.5` is a name, and the reader is the one that says whether
        // there is a node by that name. The converter does not decide that.
        let param = node_param();
        let typed = text_to_param(&param, "7.5").expect("a name");
        assert_eq!(typed.name, "node_name");
        assert_eq!(typed.value, json!("7.5"));

        let document = doc();
        let by_name = json!({ "node_name": "7.5" });
        let params = Params::new(&by_name);
        let error = node_by_id_or_name(&document, &params).expect_err("no such node");
        let message = error.to_string();
        assert!(message.contains("no node named"), "{message}");
    }

    #[test]
    fn a_name_where_there_is_no_alternative_is_refused() {
        // The alternative is what makes a name a spelling rather than a
        // mistake. Without one, a name has nowhere to go.
        let param = ParamDoc::req("node", Doc::NodeRef, "The node's id.");
        let message = text_to_param(&param, "Delay").expect_err("a name is not an id");
        assert!(message.contains("name"), "{message}");
    }

    #[test]
    fn a_time_is_left_to_the_reader_that_knows_the_frame_rate() {
        // The reader decides what `12f` means, because only it knows the
        // document's frame rate. Converting it here would move that
        // knowledge to a place that cannot have it.
        let param = ParamDoc::req("seconds", Doc::Time, "The instant.");
        let typed = text_to_param(&param, "12f").expect("a time");
        assert_eq!(typed.name, "seconds");
        assert_eq!(typed.value, json!("12f"));
    }

    #[test]
    fn a_word_is_a_word_and_the_reader_checks_it() {
        let param = ParamDoc::req("kind", Doc::NodeKind, "The kind.");
        let typed = text_to_param(&param, "filter").expect("a word");
        assert_eq!(typed.name, "kind");
        assert_eq!(typed.value, json!("filter"));
    }

    #[test]
    fn an_empty_text_is_not_a_value() {
        // The caller decides what an absent parameter means; this function
        // is only asked about text that is there.
        let param = ParamDoc::req("name", Doc::Text, "The name.");
        let typed = text_to_param(&param, "   ").expect("blank text");
        assert_eq!(typed.value, json!(""));
    }

    /// `Params` borrows its JSON. A macro that owned the value inside itself
    /// would not help — the borrow would outlive the block — so each test binds
    /// the value it passes. That is the same discipline the production code
    /// follows, and it is worth feeling in a test.
    fn doc() -> craftkit_model::Document {
        let mut d = craftkit_model::Document::empty("t");
        let _ = d.add_node("Delay", craftkit_model::NodeKind::Source);
        d
    }

    #[test]
    fn a_node_id_must_be_a_whole_number_in_range() {
        let j = json!({ "node": 7 });
        assert_eq!(node_ref(&Params::new(&j), "node").expect("ok"), NodeId(7));

        let j = json!({ "node": 1.5 });
        assert!(node_ref(&Params::new(&j), "node").is_err());

        let j = json!({ "node": u64::MAX });
        assert!(node_ref(&Params::new(&j), "node").unwrap_err().to_string().contains("larger"));

        let j = json!({ "node": "Delay" });
        let msg = node_ref(&Params::new(&j), "node").unwrap_err().to_string();
        assert!(msg.contains("node_name"), "the message should offer the alternative: {msg}");
    }

    #[test]
    fn a_missing_node_parameter_says_both_spellings() {
        let d = doc();
        let j = json!({});
        let msg = node_by_id_or_name(&d, &Params::new(&j)).unwrap_err().to_string();
        assert!(msg.contains("node_name"), "{msg}");
    }

    #[test]
    fn a_node_can_be_addressed_by_name() {
        let d = doc();
        let id = d.nodes.first().expect("one node").id;
        let j = json!({ "node_name": "Delay" });
        assert_eq!(node_by_id_or_name(&d, &Params::new(&j)).expect("ok"), id);
    }

    #[test]
    fn a_node_id_that_does_not_exist_is_refused_clearly() {
        let d = doc();
        let j = json!({ "node": 999 });
        let msg = node_by_id_or_name(&d, &Params::new(&j)).unwrap_err().to_string();
        assert!(msg.contains("no node"), "{msg}");
    }

    #[test]
    fn an_unknown_name_is_refused_by_name() {
        let d = doc();
        let j = json!({ "node_name": "Nope" });
        let msg = node_by_id_or_name(&d, &Params::new(&j)).unwrap_err().to_string();
        assert!(msg.contains("Nope"), "{msg}");
    }

    #[test]
    fn a_value_is_parsed_into_the_right_variant() {
        let j = json!({ "v": "hello" });
        let want = Value::text("v", "hello".to_string()).expect("valid text");
        assert_eq!(value_for(&Params::new(&j), "v").expect("ok").expect("some"), want);

        let j = json!({ "v": true });
        assert_eq!(value_for(&Params::new(&j), "v").expect("ok").expect("some"), Value::Bool(true));

        let j = json!({ "v": "#ff0000" });
        assert_eq!(
            value_for(&Params::new(&j), "v").expect("ok").expect("some"),
            Value::Color(Color::rgb(255, 0, 0))
        );

        let j = json!({ "v": 2.5 });
        let got = value_for(&Params::new(&j), "v").expect("ok").expect("some");
        assert_eq!(got.as_num(), Some(2.5));
    }

    #[test]
    fn a_numeric_string_becomes_a_number_so_a_script_can_be_lazy() {
        let j = json!({ "v": "2.5" });
        let got = value_for(&Params::new(&j), "v").expect("ok").expect("some");
        assert_eq!(got.as_num(), Some(2.5));
    }

    #[test]
    fn a_number_past_the_magnitude_limit_names_the_key_it_came_from() {
        let j = json!({ "gain": 1e300 });
        let msg = value_for(&Params::new(&j), "gain").unwrap_err().to_string();
        assert!(msg.contains("gain"), "the message must name the parameter: {msg}");
    }

    #[test]
    fn an_object_is_refused_as_a_value_with_the_json_type_named() {
        let j = json!({ "v": { "a": 1 } });
        let msg = value_for(&Params::new(&j), "v").unwrap_err().to_string();
        assert!(msg.contains("object"), "{msg}");
    }

    #[test]
    fn an_absent_key_is_none_not_an_error() {
        let j = json!({});
        let got: Option<Value> = value_for(&Params::new(&j), "v").expect("ok");
        assert!(got.is_none());
    }

    #[test]
    fn json_types_are_named_for_messages() {
        assert_eq!(kind_of(&json!(null)), "null");
        assert_eq!(kind_of(&json!(1)), "a number");
        assert_eq!(kind_of(&json!("s")), "a string");
        assert_eq!(kind_of(&json!(true)), "a bool");
        assert_eq!(kind_of(&json!([])), "an array");
        assert_eq!(kind_of(&json!({})), "an object");
    }

    #[test]
    fn an_unknown_id_is_reported_in_the_form_the_caller_used() {
        // Regression: this message used `Display` for `NodeId`, which writes
        // `n2`. That reads like the value was corrupted, and it is a form
        // `node_ref` actively rejects — so the message told the caller to retype
        // something that cannot work.
        let doc = craftkit_model::Document::new("t");
        let by_id = json!({ "node": 2 });
        let params = Params::new(&by_id);
        let err = node_by_id_or_name(&doc, &params).expect_err("no such node");
        let message = err.to_string();

        assert!(message.contains("id 2"), "{message}");
        assert!(!message.contains("n2"), "quotes back a form the parameter rejects: {message}");

        // And the number it quotes must actually be accepted next time round.
        let params = Params::new(&by_id);
        assert!(node_ref(&params, "node").is_ok(), "the id in the message is not usable as input");
    }

    #[test]
    fn a_node_that_exists_is_accepted_however_it_is_spelled() {
        let mut doc = craftkit_model::Document::new("t");
        let id = doc.add_node("Only", craftkit_model::NodeKind::Source).expect("adds");
        let by_id = json!({ "node": id.get() });
        let params = Params::new(&by_id);
        assert_eq!(node_by_id_or_name(&doc, &params).expect("by id"), id);

        let by_name = json!({ "node_name": "Only" });
        let params = Params::new(&by_name);
        assert_eq!(node_by_id_or_name(&doc, &params).expect("by name"), id);
    }

    #[test]
    fn a_string_where_an_id_belongs_says_how_to_spell_it() {
        let doc = craftkit_model::Document::new("t");
        let named = json!({ "node": "Only" });
        let params = Params::new(&named);
        let err = node_by_id_or_name(&doc, &params).expect_err("a name is not an id");
        let message = err.to_string();
        assert!(message.contains("node_name"), "{message}");
    }
}
