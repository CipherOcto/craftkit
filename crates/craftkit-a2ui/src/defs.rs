//! The property-**name** table of every component the layer supports.
//!
//! This module owns one thing: the **names** of the properties each
//! supported a2ui component carries. The **shapes** live in the `schema`
//! module; the **values** live in the plugin's rows. Three owners, three
//! artefacts.
//!
//! # Why bytes
//!
//! The table is compared against a row's declared properties by a **`const`
//! assertion** — the derive emits `const _: () = assert!(…)` — and a
//! `const` context cannot use `PartialEq` on `str` or on a slice: neither
//! is const-evaluable. Bytes are, so the table holds `&'static [u8]` and
//! the comparison walks them by hand.
//!
//! # What is not here
//!
//! * `component` — the discriminator is **stamped** by the compiler, so it
//!   is not a property a row declares. It is in [`EXEMPT`].
//! * `id` — a2ui's `ComponentCommon` gives **every** component an id, and
//!   it is the id a container's children are named by. The layer stamps it
//!   too, so it is not a property a row declares. It is in [`EXEMPT`].
//! * `@checks` — a2ui gives it to `Button`, `CheckBox` and `TextField`
//!   through the `Checkable` definition. The layer evaluates no check rule,
//!   so it **removes** the property. It is in [`EXEMPT`].
//! * the seven components the layer refuses (`Video`, `AudioPlayer`,
//!   `Tabs`, `Modal`, `ChoicePicker`, `Slider`, `DateTimeInput`) — a
//!   refused component has no table at all.
//!
//! # The naming rule
//!
//! One constant per supported component, named after the component in
//! `SCREAMING_SNAKE_CASE`: the a2ui name upper-cased, with `_` before each
//! capital. `Text` → `TEXT`, `CheckBox` → `CHECKBOX`, `TextField` →
//! `TEXTFIELD`.

/// The property names of `Text`.
pub const TEXT: &[&[u8]] = &[b"text", b"variant", b"weight"];

/// The property names of `Image`.
pub const IMAGE: &[&[u8]] = &[b"url", b"description", b"fit", b"variant", b"weight"];

/// The property names of `Icon`.
pub const ICON: &[&[u8]] = &[b"name", b"weight"];

/// The property names of `Row`.
pub const ROW: &[&[u8]] = &[b"children", b"justify", b"align", b"weight"];

/// The property names of `Column`.
pub const COLUMN: &[&[u8]] = &[b"children", b"justify", b"align", b"weight"];

/// The property names of `List`.
pub const LIST: &[&[u8]] = &[b"children", b"direction", b"align", b"weight"];

/// The property names of `Card`.
pub const CARD: &[&[u8]] = &[b"child", b"weight"];

/// The property names of `Divider`.
pub const DIVIDER: &[&[u8]] = &[b"axis", b"weight"];

/// The property names of `Button`.
///
/// `Button` is an `allOf` of a2ui's `Checkable` and its own object. The
/// own object's properties are the row's; `Checkable` contributes only
/// `@checks`, which the layer removes — so `@checks` is not here.
pub const BUTTON: &[&[u8]] = &[b"child", b"variant", b"action", b"weight"];

/// The property names of `CheckBox`.
///
/// As [`BUTTON`]: the `Checkable` part contributes only the removed
/// `@checks`.
pub const CHECK_BOX: &[&[u8]] = &[b"label", b"value", b"weight"];

/// The property names of `TextField`.
///
/// As [`BUTTON`]: the `Checkable` part contributes only the removed
/// `@checks`.
pub const TEXT_FIELD: &[&[u8]] = &[b"label", b"value", b"placeholder", b"variant", b"weight"];

/// The names the layer does not publish.
///
/// `component` and `id` are stamped by the compiler, and `@checks` is
/// removed by the layer — so none of them is a prop a row declares.
pub const EXEMPT: &[&[u8]] = &[b"component", b"id", b"@checks"];

/// The components the layer **refuses**, and why.
///
/// a2ui's basic catalogue has **eighteen** components. Eleven are drawn
/// here, and **seven** are refused — and a refusal that says why is a
/// declaration waiting for an application that knows what the component
/// means.
///
/// The seven are refused for two reasons, and the reasons are not the
/// same:
///
/// - **two** of them need media, and craftkit's `Canvas` seam has none:
///   `Video` and `AudioPlayer`;
/// - **five** of them are a **widget's** — a tab that is chosen, a modal
///   that is open, a choice that is picked, a value that is dragged, a
///   date that is typed — and craftkit's document has no node kind that
///   means any of those. They *could* be drawn: a slider is two
///   rectangles. They are refused because the tree does not know what
///   they **mean**, and a picture of a widget that does nothing is worse
///   than a refusal.
pub const REFUSED: &[(&str, &str)] = &[
    ("Video", "the tree's seam draws a box, a text, a path and a picture, and it has no media"),
    ("AudioPlayer", "the tree's seam draws a box, a text, a path and a picture, and it has no media"),
    ("Tabs", "a tab is a widget's choice, and the tree has no notion of one"),
    ("Modal", "a modal is a widget's state, and the tree has no notion of one"),
    ("ChoicePicker", "a choice is a widget's state, and the tree has no notion of one"),
    ("Slider", "a dragged value is a widget's state, and the tree has no notion of one"),
    ("DateTimeInput", "a typed date is a widget's state, and the tree has no notion of one"),
];

/// Whether the layer **draws** a component.
///
/// The eleven the layer supports; everything else is either refused —
/// and says why — or is not a component at all.
pub fn is_supported(name: &str) -> bool {
    matches!(
        name,
        "Text"
            | "Image"
            | "Icon"
            | "Row"
            | "Column"
            | "List"
            | "Card"
            | "Divider"
            | "Button"
            | "CheckBox"
            | "TextField"
    )
}

/// The reason a component is **refused**, if it is one.
///
/// A name that is not refused is not refused for being unknown — it is
/// simply not a component the layer draws, and the caller says so.
pub fn refusal_of(name: &str) -> Option<&'static str> {
    REFUSED.iter().find(|(component, _)| *component == name).map(|(_, reason)| *reason)
}

/// Whether the table and the names hold the same **set**.
///
/// Order-insensitive, and byte-wise: a slice's `PartialEq` is **not**
/// const-evaluable, and neither is `str`'s. One call covers both
/// directions — every name in `names` is in `table`, **and** every name
/// in `table` is in `names`.
pub const fn matches(table: &[&[u8]], names: &[&[u8]]) -> bool {
    let mut i = 0;
    while i < names.len() {
        if !contains(table, names[i]) {
            return false;
        }
        i += 1;
    }
    let mut j = 0;
    while j < table.len() {
        if !contains(names, table[j]) {
            return false;
        }
        j += 1;
    }
    true
}

/// Whether the table holds the name.
const fn contains(table: &[&[u8]], name: &[u8]) -> bool {
    let mut i = 0;
    while i < table.len() {
        if bytes_eq(table[i], name) {
            return true;
        }
        i += 1;
    }
    false
}

/// Whether two byte slices are equal.
const fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut i = 0;
    while i < left.len() {
        if left[i] != right[i] {
            return false;
        }
        i += 1;
    }
    true
}
