//! The **format** of a field.
//!
//! A field may say how its text is **shown**: the file carries the
//! script on the field's **additional actions** — the `/AA`
//! dictionary — and the **format** event of that dictionary (`/F`)
//! holds it.
//!
//! The scripts that a form writes are the ones the tree knows:
//!
//! - `AFNumber_Format(dec, sepStyle, negStyle, currStyle, strCurrency,
//!   bCurrencyPrepend)` — a number: how many decimals it has, how
//!   its parts are separated, how a negative one is written, and the
//!   currency it is written in;
//! - `AFPercent_Format(dec, sepStyle)` — a percentage;
//! - `AFDate_FormatEx(cFormat)` and `AFTime_FormatEx(cFormat)` — a
//!   date and a time, written as the pattern the file says;
//! - `AFSpecial_Format(n)` — one of the special forms: the postal
//!   code, the postal code with four more digits, the telephone
//!   number and the social security number;
//! - `AFSpecial_KeystrokeEx(cMask)` — the mask a person types into.
//!
//! And a script that is none of those is **not understood**: it is
//! kept as it is and reported as unsupported. The two calls that take
//! a **preset** — a date or a time named by a number — are among them,
//! because this tree does not hold the table of presets they name.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

/// The **format** of a field: what its script says.
#[derive(Debug, Clone, PartialEq)]
pub enum Format {
    /// The field is shown as it is.
    Nothing,
    /// The field is shown as a number.
    Number {
        /// How many decimals the number has.
        decimals: i64,
        /// The style of the separator between the parts.
        separator: i64,
        /// The style of a negative number.
        negative: i64,
        /// The currency the number is written in.
        currency: String,
        /// Whether the currency is written before the number.
        prepend: bool,
    },
    /// The field is shown as a percentage.
    Percent {
        /// How many decimals the percentage has.
        decimals: i64,
        /// The style of the separator between the parts.
        separator: i64,
    },
    /// The field is shown as a date, in the pattern the file says.
    Date(String),
    /// The field is shown as a time, in the pattern the file says.
    Time(String),
    /// The field is shown as one of the special forms.
    Special(String),
    /// The field is typed into through a mask.
    Mask(String),
    /// The script is one this tree does not understand.
    Unsupported(String),
}

/// The **number** an argument holds: a whole one.
fn whole_of(argument: &str) -> Option<i64> {
    let number = argument.trim().parse::<f64>().ok()?;
    if !number.is_finite() || number.fract() != 0.0 {
        return None;
    }
    Some(number as i64)
}

/// The **flag** an argument holds: a truth or a lie.
fn flag_of(argument: &str) -> bool {
    argument.trim().eq_ignore_ascii_case("true")
}

/// The **special form** a number names.
fn special_of(number: i64) -> Option<&'static str> {
    match number {
        0 => Some("postal-code"),
        1 => Some("postal-code-plus-four"),
        2 => Some("telephone"),
        3 => Some("social-security"),
        _ => None,
    }
}

/// The **format** a script says.
pub fn parse(script: &str) -> Format {
    let arguments = |name: &str| crate::scripts::arguments_of(script, name);
    if let Some(arguments) = arguments("AFNumber_Format") {
        let whole = |index: usize| arguments.get(index).and_then(|argument| whole_of(argument)).unwrap_or(0);
        let currency =
            arguments.get(4).and_then(|argument| crate::scripts::text_of(argument)).unwrap_or_default();
        // The currency is written before the number unless the file
        // says otherwise.
        let prepend = arguments.get(5).map(|argument| flag_of(argument)).unwrap_or(true);
        return Format::Number {
            decimals: whole(0),
            separator: whole(1),
            negative: whole(2),
            currency,
            prepend,
        };
    }
    if let Some(arguments) = arguments("AFPercent_Format") {
        let whole = |index: usize| arguments.get(index).and_then(|argument| whole_of(argument)).unwrap_or(0);
        return Format::Percent { decimals: whole(0), separator: whole(1) };
    }
    if let Some(arguments) = arguments("AFDate_FormatEx") {
        let pattern =
            arguments.first().and_then(|argument| crate::scripts::text_of(argument)).unwrap_or_default();
        return Format::Date(pattern);
    }
    if let Some(arguments) = arguments("AFTime_FormatEx") {
        let pattern =
            arguments.first().and_then(|argument| crate::scripts::text_of(argument)).unwrap_or_default();
        return Format::Time(pattern);
    }
    if let Some(arguments) = arguments("AFSpecial_Format") {
        let number = arguments.first().and_then(|argument| whole_of(argument)).unwrap_or(-1);
        return match special_of(number) {
            Some(name) => Format::Special(name.to_string()),
            None => Format::Unsupported(script.to_string()),
        };
    }
    if let Some(arguments) = arguments("AFSpecial_KeystrokeEx") {
        let mask =
            arguments.first().and_then(|argument| crate::scripts::text_of(argument)).unwrap_or_default();
        return Format::Mask(mask);
    }
    Format::Unsupported(script.to_string())
}

/// The **format** of a field.
pub fn format_of(document: &Document, node: &Node) -> Format {
    match crate::scripts::script_of(document, node, "F") {
        Some(script) => parse(&script),
        None => Format::Nothing,
    }
}

commands! {
    /// **Read** the **format** of a field.
    ///
    /// A query: it does not change the document. A field that says nothing
    /// about how its text is shown is shown as it is, and the answer says
    /// so.
    pdf.field.format {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The field to read.

            field: NodeRef,
        },
        run: {

            let id = "pdf.field.format".to_string();
            let document = session.document();
            let node = document.node(self.field.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{}`", self.field.0),
            })?;
            let format = format_of(document, node);
            let name = crate::fields::qualified_name(document, node);
            let mut map = craftkit_sdk::serde_json::Map::new();
            map.insert("field".to_string(), name.clone().map(|name| json!(name)).unwrap_or(Json::Null));
            let text = match &format {
                Format::Nothing => {
                    map.insert("kind".to_string(), json!("nothing"));
                    "the field is shown as it is".to_string()
                }
                Format::Number { decimals, separator, negative, currency, prepend } => {
                    map.insert("kind".to_string(), json!("number"));
                    map.insert("decimals".to_string(), json!(decimals));
                    map.insert("separator".to_string(), json!(separator));
                    map.insert("negative".to_string(), json!(negative));
                    map.insert("currency".to_string(), json!(currency.clone()));
                    map.insert("prepend".to_string(), json!(prepend));
                    "the field is shown as a number".to_string()
                }
                Format::Percent { decimals, separator } => {
                    map.insert("kind".to_string(), json!("percent"));
                    map.insert("decimals".to_string(), json!(decimals));
                    map.insert("separator".to_string(), json!(separator));
                    "the field is shown as a percentage".to_string()
                }
                Format::Date(pattern) => {
                    map.insert("kind".to_string(), json!("date"));
                    map.insert("pattern".to_string(), json!(pattern.clone()));
                    "the field is shown as a date".to_string()
                }
                Format::Time(pattern) => {
                    map.insert("kind".to_string(), json!("time"));
                    map.insert("pattern".to_string(), json!(pattern.clone()));
                    "the field is shown as a time".to_string()
                }
                Format::Special(name) => {
                    map.insert("kind".to_string(), json!("special"));
                    map.insert("form".to_string(), json!(name.clone()));
                    "the field is shown as a special form".to_string()
                }
                Format::Mask(mask) => {
                    map.insert("kind".to_string(), json!("mask"));
                    map.insert("mask".to_string(), json!(mask.clone()));
                    "the field is typed into through a mask".to_string()
                }
                Format::Unsupported(script) => {
                    map.insert("kind".to_string(), json!("unsupported"));
                    map.insert("script".to_string(), json!(script.clone()));
                    "the field has a script this tree does not understand".to_string()
                }
            };
            Ok(Outcome::new(text).with_data(Json::Object(map)))

        }
    }
}
