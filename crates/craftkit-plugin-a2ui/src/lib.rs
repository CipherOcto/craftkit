//! `craftkit-plugin-a2ui` — the **a2ui plugin**.
//!
//! # What this is
//!
//! **Eleven components** and **five commands**: the a2ui surface, in
//! craftkit's own terms.
//!
//! - a **component** is a craftkit **kind** (`a2ui.text` and the rest)
//!   and a **catalogue entry**, so it is in the vocabulary view as a kind
//!   and in the a2ui catalogue as a component; and
//! - a **surface** is made, drawn onto, mirrored into and taken away by
//!   **commands** — the tree's one way of changing the document.
//!
//! # Where it sits
//!
//! **L4** — a plugin. It depends on the **SDK** (**L2**) and on the
//! **a2ui layer** (**L3**) and on nothing else: a plugin may not depend
//! on an app, and it may not depend on another plugin. **That is why the
//! box layout lives in the layer** and not here — a layout in a plugin
//! could never be shared with a second one.
//!
//! # What it does not do
//!
//! - It does **not** fetch anything over a network, and it does not read
//!   a file an agent names.
//! - It does **not** write the document behind the door's back: every
//!   change is a **command**.
//! - It does **not** know what a2ui is beyond the layer's shapes.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod commands;
pub mod components;
pub mod model;
pub mod picture;
pub mod props;
pub mod svg;
pub mod tree;

use craftkit_derive::Layer;

/// The **a2ui layer**.
///
/// The layer is declared **here**, by the plugin that needs it: the
/// registry refuses a contribution whose layer is not linked in, and the
/// layer is what the components belong to.
#[derive(Layer)]
#[layer(id = "a2ui", name = "A2UI", requires("core"))]
pub struct A2uiLayer;

// The linker keeps the plugin's items on every target.
//
// Every item the plugin contributes is named here: the layer, and the
// eleven components. A type that does not implement `Anchored` is a
// compile error, and the compiler checks the list.
craftkit_sdk::plugin_anchor!(
    A2uiLayer,
    components::TextComponent,
    components::ImageComponent,
    components::IconComponent,
    components::RowComponent,
    components::ColumnComponent,
    components::ListComponent,
    components::CardComponent,
    components::DividerComponent,
    components::ButtonComponent,
    components::CheckBoxComponent,
    components::TextFieldComponent,
);
