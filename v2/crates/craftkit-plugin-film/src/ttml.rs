//! TTML: the cues of a `.ttml` or a `.dfxp` file.
//!
//! TTML is XML: a `<tt>` root holds a `<body>`, the body holds `<div>`
//! elements and the divs hold `<p>` elements, and each of them may say
//! when it begins and when it ends. The rules are the tree's own.
//!
//! # The rules
//!
//! - The **timing parameters** of the file are its frame rate, its
//!   frame rate multiplier, its sub-frame rate and its tick rate. A
//!   parameter that is not a positive number is an error, and a file
//!   without them keeps the defaults: thirty frames a second, one
//!   sub-frame, and one tick a second.
//! - A **time expression** is a clock — `HH:MM:SS`, `HH:MM:SS.fff` or
//!   `HH:MM:SS:FF.f` — or an offset: `1h`, `2m`, `3s`, `4ms`, `5f` or
//!   `6t`. An expression that is not one is a warning, and the caption
//!   it is on is left out.
//! - The **times of a caption** are offsets in its container: the begin
//!   of a `<p>` is the begin of its container and its own begin, and
//!   nothing inside a container outlasts the end of the container.
//! - A `<p>` **with a time** of its own, or without timed spans in it,
//!   is one caption; a `<p>` without a time of its own and with timed
//!   spans in it is as many captions as it has spans.
//! - The **text** of a caption is its text with the whitespace
//!   collapsed to one space, and a `<br/>` is a line break.
//!
//! # What it does not do
//!
//! - **The styles are not kept**: the tree writes the `<i>`, `<b>` and
//!   `<u>` of a caption into its text, and here the text is the plain
//!   text of the caption.
//! - **The speakers are not kept**, and neither are the identifiers.

use craftkit_sdk::xml::Element;

use crate::captions::Cue;

/// The **timing** of a file: the rate of the frames, the rate of the
/// sub-frames and the rate of the ticks.
#[derive(Clone, Copy)]
struct Timing {
    /// The **milliseconds** of a frame.
    frame: f64,
    /// The **sub-frames** of a frame.
    sub_frames: i64,
    /// The **ticks** a second.
    ticks: f64,
    /// The **base** of the frames: the number of frames the labels of
    /// the clock count.
    base: i64,
}

/// The **attribute** of that name: the namespace of an attribute is not
/// part of its name.
fn attribute<'a>(element: &'a Element, local: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|(name, _)| name.rsplit(':').next().unwrap_or(name.as_str()) == local)
        .map(|(_, value)| value.as_str())
}

/// The **timing** of a file, by the tree's own rules.
fn timing_of(root: &Element) -> Result<Timing, String> {
    let integer = |name: &str, default: i64| -> Result<i64, String> {
        match attribute(root, name) {
            Some(value) => {
                let value = value.trim();
                let number: i64 =
                    value.parse().map_err(|_| format!("the {name} `{value}` is not a whole number"))?;
                if number <= 0 {
                    return Err(format!("the {name} `{value}` is not a positive number"));
                }
                Ok(number)
            }
            None => Ok(default),
        }
    };
    let base = integer("frameRate", 30)?;
    let sub_frames = integer("subFrameRate", 1)?;
    let (numerator, denominator) = match attribute(root, "frameRateMultiplier") {
        Some(value) => {
            let mut parts = value.split_whitespace();
            let numerator = parts.next().and_then(|part| part.parse::<i64>().ok());
            let denominator = parts.next().and_then(|part| part.parse::<i64>().ok());
            match (numerator, denominator, parts.next()) {
                (Some(numerator), Some(denominator), None) if numerator > 0 && denominator > 0 => {
                    (numerator, denominator)
                }
                _ => {
                    return Err("the frameRateMultiplier is not a pair of positive numbers".to_string());
                }
            }
        }
        None => (1, 1),
    };
    let ticks = match attribute(root, "tickRate") {
        Some(value) => {
            let number: f64 = value.trim().parse().map_err(|_| "the tickRate is not a number".to_string())?;
            if number <= 0.0 || !number.is_finite() {
                return Err("the tickRate is not a positive number".to_string());
            }
            number
        }
        // TTML1 §6.2.10: an absent tick rate is the effective frame
        // rate times the sub-frame rate.
        None if attribute(root, "frameRate").is_some() => {
            base as f64 * numerator as f64 / denominator as f64 * sub_frames as f64
        }
        None => 1.0,
    };
    let frame = 1000.0 * denominator as f64 / (base as f64 * numerator as f64);
    Ok(Timing { frame, sub_frames, ticks, base })
}

/// The **milliseconds** of a number and the fraction that follows it.
fn decimal(integer: &str, fraction: &str, unit: f64) -> Option<f64> {
    let integer: f64 = if integer.is_empty() { 0.0 } else { integer.parse().ok()? };
    if !integer.is_finite() || integer < 0.0 {
        return None;
    }
    let mut value = integer * unit;
    if !fraction.is_empty() {
        if fraction.len() > 12 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let part: f64 = fraction.parse().ok()?;
        value += part * unit / 10f64.powi(fraction.len() as i32);
    }
    Some(value)
}

/// The **milliseconds** of a time expression, or `None` when it is not
/// one.
fn time_of(text: &str, timing: &Timing) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains(':') {
        // A **clock**: `HH:MM:SS`, `HH:MM:SS.fff` or `HH:MM:SS:FF.f`.
        let parts: Vec<&str> = text.split(':').collect();
        let number = |part: &str| -> Option<f64> {
            if part.is_empty() || part.len() > 6 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        let hours = number(parts.first()?)?;
        let minutes = number(parts.get(1)?)?;
        let seconds_part = parts.get(2)?;
        let (seconds_text, fraction) = seconds_part.split_once('.').unwrap_or((seconds_part, ""));
        let seconds = number(seconds_text)?;
        if minutes >= 60.0 || seconds >= 60.0 {
            return None;
        }
        let mut value = (hours * 3600.0 + minutes * 60.0) * 1000.0;
        value += decimal(seconds_text, fraction, 1000.0)?;
        if let Some(frames_part) = parts.get(3) {
            if parts.len() > 4 {
                return None;
            }
            let (frames, sub) = frames_part.split_once('.').unwrap_or((frames_part, ""));
            let frames = number(frames)?;
            let sub = if sub.is_empty() { 0.0 } else { number(sub)? };
            if frames >= timing.base as f64 || sub >= timing.sub_frames as f64 {
                return None;
            }
            value += frames * timing.frame + sub * timing.frame / timing.sub_frames as f64;
        }
        return Some(value);
    }
    // An **offset**: a number and a unit.
    let at = text.find(|character: char| character.is_ascii_alphabetic())?;
    let (number, unit) = text.split_at(at);
    let (integer, fraction) = number.split_once('.').unwrap_or((number, ""));
    if integer.is_empty() || integer.len() > 15 || !integer.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(match unit {
        "h" => decimal(integer, fraction, 3_600_000.0)?,
        "m" => decimal(integer, fraction, 60_000.0)?,
        "s" => decimal(integer, fraction, 1000.0)?,
        "ms" => decimal(integer, fraction, 1.0)?,
        "f" => {
            let frames: f64 = integer.parse().ok()?;
            frames * timing.frame + decimal("0", fraction, timing.frame)?
        }
        "t" => decimal(integer, fraction, 1000.0 / timing.ticks)?,
        _ => return None,
    })
}

/// The **active interval** of the containers around an element.
#[derive(Clone, Copy)]
struct Scope {
    /// The **begin** of the container, in milliseconds.
    begin: f64,
    /// The **end** of the container, when it has one.
    end: Option<f64>,
}

/// The **times** of an element in its container.
struct Times {
    /// The begin, when the element sets one.
    begin: Option<f64>,
    /// Its own end — the end, or the begin and the duration, the
    /// earlier of the two — clipped to the end of the container.
    end: Option<f64>,
    /// A time of the element did not parse.
    bad: bool,
}

/// The **times** of an element in its container, by the tree's own
/// rules.
fn times_of(element: &Element, parent: Scope, timing: &Timing, warnings: &mut Vec<String>) -> Times {
    let mut bad = false;
    let mut time = |key: &str| -> Option<f64> {
        let value = attribute(element, key)?;
        match time_of(value, timing) {
            Some(millis) => Some(millis),
            None => {
                bad = true;
                warnings.push(format!("bad time expression \"{value}\""));
                None
            }
        }
    };
    let begin = time("begin").map(|millis| parent.begin + millis);
    let own_end = match (
        time("end").map(|millis| parent.begin + millis),
        time("dur").map(|millis| begin.unwrap_or(parent.begin) + millis),
    ) {
        (Some(end), Some(duration)) => Some(end.min(duration)),
        (end, duration) => end.or(duration),
    };
    let end = match (own_end, parent.end) {
        (Some(end), Some(parent_end)) => Some(end.min(parent_end)),
        (end, parent_end) => end.or(parent_end),
    };
    Times { begin, end, bad }
}

/// Whether an element has a **time** of its own.
fn has_timing(element: &Element) -> bool {
    ["begin", "end", "dur"].iter().any(|key| attribute(element, key).is_some())
}

/// The text being **collected**: the whitespace collapses to one space,
/// which is written only before the next visible character and never
/// at the start of a line.
struct Out {
    /// The text so far.
    text: String,
    /// Whether a space is waiting to be written.
    pending: bool,
}

impl Out {
    /// Write the **space** that is waiting, if there is one.
    fn flush(&mut self) {
        if self.pending && !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push(' ');
        }
        self.pending = false;
    }
}

/// Collect the **text** of an element, in the order it was written.
fn collect(element: &Element, out: &mut Out) {
    for (at, child) in element.children.iter().enumerate() {
        if let Some(text) = element.texts.get(at) {
            push_text(text, out);
        }
        if child.name == "br" {
            out.pending = false;
            out.text.push('\n');
            continue;
        }
        collect(child, out);
    }
    if let Some(text) = element.texts.last() {
        push_text(text, out);
    }
}

/// Push the **text** of an element into the text being collected.
fn push_text(text: &str, out: &mut Out) {
    for character in text.chars() {
        if character.is_whitespace() {
            out.pending = true;
        } else {
            out.flush();
            out.text.push(character);
        }
    }
}

/// The **text** of a caption: the lines of it, trimmed.
fn clean(text: &str) -> String {
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// The **cues** of an element and of the elements under it.
fn walk(element: &Element, parent: Scope, timing: &Timing, cues: &mut Vec<Cue>, warnings: &mut Vec<String>) {
    for child in &element.children {
        match child.name.as_str() {
            "div" => {
                let times = times_of(child, parent, timing, warnings);
                walk(
                    child,
                    Scope { begin: times.begin.unwrap_or(parent.begin), end: times.end },
                    timing,
                    cues,
                    warnings,
                );
            }
            "p" => {
                let times = times_of(child, parent, timing, warnings);
                if times.bad {
                    continue;
                }
                let (begin, end) = (times.begin.unwrap_or(parent.begin), times.end);
                let spans: Vec<&Element> = child
                    .children
                    .iter()
                    .filter(|grandchild| grandchild.name == "span" && has_timing(grandchild))
                    .collect();
                // A paragraph with a time of its own, or without timed
                // spans in it, is one cue.
                let whole = if has_timing(child) || spans.is_empty() { end } else { None };
                match whole {
                    Some(end) => {
                        let mut out = Out { text: String::new(), pending: false };
                        collect(child, &mut out);
                        let text = clean(&out.text);
                        if end > begin && !text.is_empty() {
                            cues.push(Cue { start: rounded(begin), end: rounded(end), text });
                        }
                    }
                    None => {
                        // A paragraph without a time of its own: its
                        // timed spans are the cues.
                        let scope = Scope { begin, end };
                        for span in spans {
                            let times = times_of(span, scope, timing, warnings);
                            let begin = times.begin.unwrap_or(scope.begin);
                            if let (false, Some(end)) = (times.bad, times.end) {
                                let mut out = Out { text: String::new(), pending: false };
                                collect(span, &mut out);
                                let text = clean(&out.text);
                                if end > begin && !text.is_empty() {
                                    cues.push(Cue { start: rounded(begin), end: rounded(end), text });
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// The **milliseconds** of a time, rounded to the nearest one.
fn rounded(millis: f64) -> i64 {
    if !millis.is_finite() {
        return 0;
    }
    millis.round() as i64
}

/// The **cues** of a TTML file, and the warnings of the reading.
pub fn cues_of(text: &str) -> Result<(Vec<Cue>, Vec<String>), String> {
    let root = craftkit_sdk::xml::parse(text).map_err(|message| format!("the file is not XML: {message}"))?;
    if root.name != "tt" {
        return Err("not a TTML file".to_string());
    }
    let timing = timing_of(&root)?;
    let mut warnings = Vec::new();
    let mut cues: Vec<Cue> = Vec::new();
    if let Some(body) = root.children.iter().find(|child| child.name == "body") {
        // The body is a time container like a div: its begin offsets,
        // and its end clips, everything inside it.
        let times = times_of(body, Scope { begin: 0.0, end: None }, &timing, &mut warnings);
        walk(
            body,
            Scope { begin: times.begin.unwrap_or(0.0), end: times.end },
            &timing,
            &mut cues,
            &mut warnings,
        );
    }
    cues.sort_by_key(|cue| (cue.start, cue.end));
    Ok((cues, warnings))
}

/// The **cues** of a TTML file, as the answer holds them.
pub fn cues_as_json(cues: &[Cue]) -> Vec<craftkit_sdk::serde_json::Value> {
    crate::scc::cues_as_json(cues)
}
