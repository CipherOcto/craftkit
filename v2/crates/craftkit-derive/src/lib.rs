//! `craftkit-derive` — the compiler writes the contribution.
//!
//! # What this does
//!
//! A command in v2 is a **struct whose fields are its parameters**, and a
//! node kind, a tool, a panel and a layer are **specs**. This crate writes
//! all five: the declaration, the reader where there is one, and the
//! registration that puts the contribution in the registry.
//!
//! ```ignore
//! /// Mute or unmute a node.
//! #[derive(Command)]
//! #[command(id = "node.mute", area = "node", icon = "mute", mutates, undoable)]
//! struct NodeMute {
//!     /// The node's id, or its name.
//!     #[param(or = "node_name")]
//!     node: Either<NodeRef, Text>,
//!     /// True to mute, false to unmute.
//!     muted: Option<Bool>,
//! }
//!
//! impl NodeMute {
//!     fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> { … }
//! }
//! ```
//!
//! and
//!
//! ```ignore
//! /// A rectangle.
//! #[derive(NodeKind)]
//! #[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry")]
//! struct RectKind;
//!
//! impl RectKind {
//!     fn paint(&self, node: &Node, canvas: &mut dyn Canvas) { … }
//!     fn hit(&self, node: &Node, point: Point) -> bool { … }
//! }
//! ```
//!
//! # What the compiler now knows that it did not before
//!
//! | Rule | Before | Now |
//! |---|---|---|
//! | a parameter a command reads is declared | a gate, reading the source | **impossible**: the body has no parameter object |
//! | a parameter a command declares is read | a gate, reading the source | **impossible**: the reader is built from the fields |
//! | two parameters may not share a name | a gate | a compile error |
//! | a parameter's kind | a hand-written table | the **type** of the field |
//! | whether a parameter is required | a hand-written table | whether the type is an `Option` |
//! | what a parameter is called | a hand-written table | the **name of the field** |
//! | what a command does | a hand-written sentence | the **doc comment** |
//! | a command's id and area agree | a gate | a compile error |
//! | `undoable` without `mutates` | a gate | a compile error |
//! | two contributions with one id | a gate | a refusal when the registry is built |
//! | a tool that runs a command that is not there | a gate | a refusal when the ecosystem is built |
//! | a contribution in a layer that is not there | a gate | a refusal when the ecosystem is built |
//!
//! The first two are the ones the tree this replaces spent two gates on, and
//! they are now *vacuous*: there is no way to write the mistake.
//!
//! # The one thing the macro cannot check
//!
//! That the contribution's **body** is right. A macro writes the edges; the
//! middle is still the programmer's.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proc_macro::TokenStream;

mod command;
mod common;
mod seams;

/// Declare a command.
///
/// The struct's fields are its parameters, and the macro writes the
/// declaration, the reader and the registration.
#[proc_macro_derive(Command, attributes(param, command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match command::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declare a kind of node.
///
/// The kind's own methods say how it is painted and what is inside it, and
/// the core never matches on a kind.
#[proc_macro_derive(NodeKind, attributes(node_kind))]
pub fn derive_node_kind(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match seams::expand_node_kind(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declare a tool: a gesture that runs a command.
#[proc_macro_derive(Tool, attributes(tool))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match seams::expand_tool(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declare a panel: a window's worth of commands.
#[proc_macro_derive(Panel, attributes(panel))]
pub fn derive_panel(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match seams::expand_panel(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Declare a layer: a group of contributions that depend on each other.
#[proc_macro_derive(Layer, attributes(layer))]
pub fn derive_layer(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match seams::expand_layer(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::DeriveInput;

    /// Expand a declaration, and take the message of the error it earns.
    fn refuse(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match command::expand(input) {
            Ok(_) => panic!("the declaration was accepted: {source}"),
            Err(error) => error.to_string(),
        }
    }

    /// Expand a declaration, and take the tokens it produced.
    fn accept(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match command::expand(input) {
            Ok(tokens) => tokens.to_string(),
            Err(error) => panic!("the declaration was refused: {error}"),
        }
    }

    const GOOD: &str = r#"
/// Mute or unmute a node.
#[derive(Command)]
#[command(id = "node.mute", area = "node", icon = "mute", mutates, undoable)]
struct NodeMute {
    /// The node's id, or its name.
    #[param(or = "node_name")]
    node: Either<NodeRef, Text>,
    /// True to mute.
    muted: Option<Bool>,
}
"#;

    #[test]
    fn a_good_declaration_is_accepted() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("node.mute"), "{tokens}");
        assert!(tokens.contains("node_name"), "{tokens}");
        assert!(tokens.contains("Mute or unmute a node."), "{tokens}");
    }

    #[test]
    fn a_declaration_carries_its_capability_and_layer() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("capability"), "{tokens}");
        assert!(tokens.contains("layer"), "{tokens}");
        assert!(tokens.contains("Registration"), "{tokens}");
    }

    #[test]
    fn an_id_that_is_not_dotted_is_refused() {
        let message = refuse(&GOOD.replace("\"node.mute\"", "\"mute\""));
        assert!(message.contains("<area>.<thing>"), "{message}");
    }

    #[test]
    fn an_id_that_is_not_lower_case_is_refused() {
        let message = refuse(&GOOD.replace("\"node.mute\"", "\"node.mute_it\""));
        assert!(message.contains("lower case"), "{message}");
    }

    #[test]
    fn an_id_that_is_not_in_its_area_is_refused() {
        let message = refuse(&GOOD.replace("\"node.mute\"", "\"doc.mute\""));
        assert!(message.contains("area"), "{message}");
    }

    #[test]
    fn an_undoable_command_that_does_not_mutate_is_refused() {
        let message = refuse(&GOOD.replace("mutates, ", ""));
        assert!(message.contains("cannot be undoable"), "{message}");
    }

    #[test]
    fn an_either_without_its_other_spelling_is_refused() {
        let message = refuse(&GOOD.replace("#[param(or = \"node_name\")]\n    ", ""));
        assert!(message.contains("needs the other spelling"), "{message}");
    }

    #[test]
    fn a_parameter_with_an_other_spelling_that_is_not_an_either_is_refused() {
        let message = refuse(&GOOD.replace("node: Either<NodeRef, Text>,", "node: Text,"));
        assert!(message.contains("must be an"), "{message}");
    }

    #[test]
    fn a_command_with_no_id_is_refused() {
        let message = refuse(
            r#"
#[derive(Command)]
#[command(area = "node")]
struct NoId {
    name: Text,
}
"#,
        );
        assert!(message.contains("needs an id"), "{message}");
    }

    #[test]
    fn a_command_with_no_fields_is_accepted() {
        let tokens = accept(
            r#"
/// Count the nodes.
#[derive(Command)]
#[command(id = "hello.count", area = "hello")]
struct HelloCount {}
"#,
        );
        assert!(tokens.contains("hello.count"), "{tokens}");
    }

    #[test]
    fn the_label_comes_from_the_id_when_it_is_not_given() {
        let tokens = accept(
            r#"
#[derive(Command)]
#[command(id = "hello.count")]
struct HelloCount {}
"#,
        );
        assert!(tokens.contains("Count"), "{tokens}");
    }

    #[test]
    fn the_static_is_named_after_the_struct() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("NODE_MUTE"), "{tokens}");
    }

    #[test]
    fn the_sdk_is_named_in_full_for_a_crate_that_depends_on_it() {
        // The macro does not know the name the using crate knows the SDK by,
        // so it asks `proc-macro-crate`.
        let root = common::sdk_root().to_string();
        assert!(root.contains("craftkit_sdk") || root == "crate", "{root}");
    }

    #[test]
    fn a_node_kind_writes_its_paint_and_hit() {
        let input: DeriveInput = syn::parse_str(
            r#"
/// A rectangle.
#[derive(NodeKind)]
#[node_kind(kind = "geometry.rect", label = "Rectangle", layer = "geometry")]
struct RectKind;
"#,
        )
        .expect("the declaration parses");
        let tokens = seams::expand_node_kind(input).expect("the kind is accepted").to_string();
        assert!(tokens.contains("geometry.rect"), "{tokens}");
        assert!(tokens.contains("paint"), "{tokens}");
        assert!(tokens.contains("hit"), "{tokens}");
    }

    #[test]
    fn a_tool_needs_the_command_it_runs() {
        let input: DeriveInput = syn::parse_str(
            r#"
#[derive(Tool)]
#[tool(id = "tool.rect", label = "Rectangle")]
struct RectTool;
"#,
        )
        .expect("the declaration parses");
        let error = seams::expand_tool(input).expect_err("the tool is refused");
        assert!(error.to_string().contains("command it runs"), "{error}");
    }
}
