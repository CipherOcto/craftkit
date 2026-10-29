//! `craftkit-derive` — the compiler writes the command.
//!
//! # What this does
//!
//! A command in v2 is a **struct whose fields are its parameters**:
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
//! and this macro writes the rest:
//!
//! - the **declaration** — the id, the area, the label, the documentation and
//!   the parameter table, taken from the attribute, the doc comments and the
//!   **types** of the fields;
//! - the **reader** — a function that reads the caller's values into the
//!   struct, so a command body never sees a parameter object;
//! - the **registration** — the command is offered to the registry by the
//!   linker, so a crate that contains a command contributes it without
//!   anybody naming it.
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
//! | two commands with one id | a gate | a refusal when the registry is built |
//!
//! The first two are the ones the tree this replaces spent two gates on, and
//! they are now *vacuous*: there is no way to write the mistake.
//!
//! # The one thing the macro cannot check
//!
//! That the command's **body** is right. A macro writes the edges; the
//! middle is still the programmer's.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token, parse_macro_input};

/// The `#[command(...)]` attribute.
#[derive(Default)]
struct CommandArgs {
    id: Option<LitStr>,
    label: Option<LitStr>,
    area: Option<LitStr>,
    icon: Option<LitStr>,
    shortcut: Option<LitStr>,
    enabled: Option<syn::Path>,
    mutates: bool,
    undoable: bool,
    no_prefix: bool,
}

impl Parse for CommandArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = CommandArgs::default();
        while !input.is_empty() {
            let field: Ident = input.parse()?;
            match field.to_string().as_str() {
                "id" => {
                    input.parse::<Token![=]>()?;
                    args.id = Some(input.parse::<LitStr>()?);
                }
                "label" => {
                    input.parse::<Token![=]>()?;
                    args.label = Some(input.parse::<LitStr>()?);
                }
                "area" => {
                    input.parse::<Token![=]>()?;
                    args.area = Some(input.parse::<LitStr>()?);
                }
                "icon" => {
                    input.parse::<Token![=]>()?;
                    args.icon = Some(input.parse::<LitStr>()?);
                }
                "shortcut" => {
                    input.parse::<Token![=]>()?;
                    args.shortcut = Some(input.parse::<LitStr>()?);
                }
                "enabled" => {
                    input.parse::<Token![=]>()?;
                    args.enabled = Some(input.parse::<syn::Path>()?);
                }
                "mutates" => args.mutates = true,
                "undoable" => args.undoable = true,
                "no_prefix" => args.no_prefix = true,
                other => {
                    return Err(Error::new(
                        field.span(),
                        format!(
                            "unknown command field `{other}`: a command has `id`, and optionally \
                             `label`, `area`, `icon`, `shortcut`, `enabled`, `mutates`, \
                             `undoable` and `no_prefix`"
                        ),
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(args)
    }
}

/// `#[param(...)]` on a field.
#[derive(Default)]
struct ParamArgs {
    /// The other spelling of this parameter.
    alternative: Option<LitStr>,
}

impl Parse for ParamArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ParamArgs::default();
        while !input.is_empty() {
            let field: Ident = input.parse()?;
            match field.to_string().as_str() {
                "or" => {
                    input.parse::<Token![=]>()?;
                    args.alternative = Some(input.parse::<LitStr>()?);
                }
                other => {
                    return Err(Error::new(
                        field.span(),
                        format!("unknown parameter field `{other}`: a parameter has `or`"),
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(args)
    }
}

/// How a field's type says the parameter is optional, or has two spellings.
enum Shape {
    /// A plain kind: the parameter is required.
    Plain,
    /// An `Option<T>`: the parameter is optional.
    Optional(syn::Type),
    /// An `Either<A, B>`: one parameter, two spellings.
    Either(syn::Type),
}

/// The last segment of a type's path, if it has one.
fn last_segment(ty: &syn::Type) -> Option<&Ident> {
    let syn::Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    Some(&segment.ident)
}

/// The first generic argument of a type, if it has one.
fn first_argument(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => {
            arguments.args.iter().find_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
        }
        _ => None,
    }
}

/// How a field's type is shaped.
fn shape_of(ty: &syn::Type) -> Shape {
    match last_segment(ty).map(Ident::to_string).as_deref() {
        Some("Option") => match first_argument(ty) {
            Some(inner) => Shape::Optional(inner.clone()),
            None => Shape::Plain,
        },
        Some("Either") => match first_argument(ty) {
            Some(inner) => Shape::Either(inner.clone()),
            None => Shape::Plain,
        },
        _ => Shape::Plain,
    }
}

/// The **first paragraph** of an item's doc comment, as one line.
///
/// The first paragraph, and not the whole comment: a command's
/// documentation is one line in a catalogue and one line in a schema, and a
/// comment that explains the implementation below it is for the reader of
/// the source, not for the caller.
fn doc_of(attrs: &[syn::Attribute]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for attribute in attrs {
        if !attribute.path().is_ident("doc") {
            continue;
        }
        let syn::Meta::NameValue(name_value) = &attribute.meta else { continue };
        let syn::Expr::Lit(expression) = &name_value.value else { continue };
        let syn::Lit::Str(text) = &expression.lit else { continue };
        let line = text.value().trim().to_string();
        if line.is_empty() {
            // The end of the first paragraph.
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        lines.push(line);
    }
    lines.join(" ")
}

/// The SDK's name, from the point of view of the crate using the macro.
fn sdk_root() -> TokenStream2 {
    match proc_macro_crate::crate_name("craftkit-sdk") {
        Ok(proc_macro_crate::FoundCrate::Itself) => quote!(crate),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name);
            quote!(::#ident)
        }
        Err(_) => quote!(::craftkit_sdk),
    }
}

/// The static's name: the struct's name in `SCREAMING_SNAKE_CASE`.
///
/// `NodeMute` is `NODE_MUTE`, and `HelloGreet` is `HELLO_GREET` — the
/// camel case becomes the screaming case, one underscore at a time.
fn static_name(name: &Ident) -> Ident {
    format_ident!("{}", screaming(name))
}

/// `NodeMute` as `NODE_MUTE`.
fn screaming(name: &Ident) -> String {
    let text = name.to_string();
    let mut out = String::new();
    for (index, character) in text.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(character.to_uppercase());
    }
    out
}

/// `NodeMute` as `node_mute`.
fn snake(name: &Ident) -> String {
    let text = name.to_string();
    let mut out = String::new();
    for (index, character) in text.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(character.to_lowercase());
    }
    out
}

/// A label from an id: `node.mute` is "Mute".
fn label_from_id(id: &str) -> String {
    let last = id.rsplit('.').next().unwrap_or(id);
    let mut characters = last.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => last.to_string(),
    }
}

/// Check an id: dotted, lower case, at least two parts.
fn check_id(id: &LitStr) -> Result<Vec<String>> {
    let value = id.value();
    let parts: Vec<String> = value.split('.').map(|part| part.to_string()).collect();
    if parts.len() < 2 {
        return Err(Error::new(
            id.span(),
            format!("`{value}` is not a command id: an id is `<area>.<thing>`"),
        ));
    }
    for part in &parts {
        if part.is_empty() {
            return Err(Error::new(id.span(), format!("`{value}` has an empty part")));
        }
        if !part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Err(Error::new(
                id.span(),
                format!("`{value}` is not dotted and lower case, and it has no underscores"),
            ));
        }
    }
    Ok(parts)
}

/// Write the command.
#[proc_macro_derive(Command, attributes(param, command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Write everything.
fn expand(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "a command is a struct with named fields: the fields are its parameters",
                ));
            }
        },
        _ => {
            return Err(Error::new(Span::call_site(), "a command is a struct"));
        }
    };

    let args = input
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("command"))
        .map(|attribute| attribute.parse_args_with(CommandArgs::parse))
        .transpose()?
        .unwrap_or_default();

    let id = args.id.clone().ok_or_else(|| {
        Error::new(Span::call_site(), "a command needs an id: `#[command(id = \"area.thing\")]`")
    })?;
    let parts = check_id(&id)?;
    let area = args.area.clone().map(|area| area.value()).unwrap_or_else(|| parts[0].clone());
    if !args.no_prefix && parts[0] != area {
        return Err(Error::new(
            id.span(),
            format!("`{}` is not in the `{area}` area: that area's commands are `{area}.*`", id.value()),
        ));
    }
    if args.undoable && !args.mutates {
        return Err(Error::new(
            id.span(),
            "a command that does not change the document cannot be undoable: there is nothing \
             to undo",
        ));
    }

    let label = args.label.clone().map(|label| label.value()).unwrap_or_else(|| label_from_id(&id.value()));
    let icon = args.icon.clone().map(|icon| icon.value()).unwrap_or_default();
    let shortcut = args.shortcut.clone();
    let doc = doc_of(&input.attrs);

    // The fields, and what each one says.
    let mut names: Vec<String> = Vec::new();
    let mut idents: Vec<Ident> = Vec::new();
    let mut alternatives: Vec<String> = Vec::new();
    let mut reads: Vec<TokenStream2> = Vec::new();
    let mut declarations: Vec<TokenStream2> = Vec::new();

    for field in &fields {
        let field_name = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(Span::call_site(), "a command's fields are named"))?;
        let field_name_string = field_name.to_string();
        if names.contains(&field_name_string) {
            return Err(Error::new(field_name.span(), format!("`{field_name_string}` is declared twice")));
        }
        names.push(field_name_string.clone());
        idents.push(field_name.clone());

        let param = field
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("param"))
            .map(|attribute| attribute.parse_args_with(ParamArgs::parse))
            .transpose()?
            .unwrap_or_default();

        let ty = field.ty.clone();
        let shape = shape_of(&ty);
        let help = doc_of(&field.attrs);

        let (kind, read, declaration) = match (&param.alternative, &shape) {
            (Some(alternative), Shape::Either(left)) => {
                let alternative_value = alternative.value();
                if names.contains(&alternative_value) {
                    return Err(Error::new(
                        alternative.span(),
                        format!(
                            "`{alternative_value}` is a parameter of this command and also the \
                             other spelling of `{field_name_string}`: a caller could not say \
                             which one it meant"
                        ),
                    ));
                }
                if alternatives.contains(&alternative_value) {
                    return Err(Error::new(
                        alternative.span(),
                        format!("`{alternative_value}` is the other spelling of two parameters"),
                    ));
                }
                alternatives.push(alternative_value.clone());
                (
                    quote!(<#left as #sdk::ParamKind>::DOC),
                    quote!(<#ty>::read_either(params, #field_name_string, #alternative_value)),
                    quote!(#sdk::ParamDoc::either(
                        #field_name_string,
                        <#left as #sdk::ParamKind>::DOC,
                        #help,
                        #alternative_value,
                    )),
                )
            }
            (Some(_), _) => {
                return Err(Error::new(
                    field_name.span(),
                    format!(
                        "`{field_name_string}` has another spelling, so its type must be an \
                         `Either`"
                    ),
                ));
            }
            (None, Shape::Either(_)) => {
                return Err(Error::new(
                    field_name.span(),
                    format!(
                        "`{field_name_string}` is an `Either`, so it needs the other spelling: \
                         `#[param(or = \"…\")]`"
                    ),
                ));
            }
            (None, Shape::Optional(inner)) => (
                quote!(<#inner as #sdk::ParamKind>::DOC),
                quote!(<#ty as #sdk::ParamKind>::read(params, #field_name_string)),
                quote!(#sdk::ParamDoc::optional(
                    #field_name_string,
                    <#inner as #sdk::ParamKind>::DOC,
                    #help,
                )),
            ),
            (None, Shape::Plain) => (
                quote!(<#ty as #sdk::ParamKind>::DOC),
                quote!(<#ty as #sdk::ParamKind>::read(params, #field_name_string)),
                quote!(#sdk::ParamDoc::required(
                    #field_name_string,
                    <#ty as #sdk::ParamKind>::DOC,
                    #help,
                )),
            ),
        };

        reads.push(quote! {
            let #field_name = #read.map_err(|source| #sdk::EngineError::Param {
                id: #id.to_string(),
                source,
            })?;
        });
        let _ = kind;
        declarations.push(declaration);
    }

    let static_ident = static_name(&name);
    let run_ident = format_ident!("__craftkit_run_{}", snake(&name));
    let shortcut_expression = match &shortcut {
        Some(shortcut) => quote!(Some(#shortcut)),
        None => quote!(None),
    };
    let enabled = match &args.enabled {
        Some(enabled) => quote!(#enabled),
        None => quote!(#sdk::always),
    };
    let mutates = args.mutates;
    let undoable = args.undoable;
    let doc_literal = LitStr::new(&doc, id.span());
    let label_literal = LitStr::new(&label, id.span());
    let area_literal = LitStr::new(&area, id.span());
    let icon_literal = LitStr::new(&icon, id.span());

    Ok(quote! {
        impl #name {
            /// Read the command's parameters.
            ///
            /// Written by the macro: the fields are the parameters, and the
            /// types say what they are.
            fn read(params: &#sdk::Params<'_>) -> Result<Self, #sdk::EngineError> {
                #(#reads)*
                Ok(Self { #(#idents,)* })
            }
        }

        /// The generated entry point: read the parameters, then run.
        fn #run_ident(
            session: &mut #sdk::Session,
            params: &#sdk::Params<'_>,
        ) -> Result<#sdk::Outcome, #sdk::EngineError> {
            let command = #name::read(params)?;
            command.run(session)
        }

        #[doc = #doc_literal]
        pub static #static_ident: #sdk::CommandSpec = #sdk::CommandSpec {
            id: #id,
            label: #label_literal,
            area: #area_literal,
            icon: #icon_literal,
            doc: #doc_literal,
            shortcut: #shortcut_expression,
            params: &[
                #(#declarations),*
            ],
            mutates: #mutates,
            undoable: #undoable,
            enabled: #enabled,
            run: #run_ident,
        };

        #sdk::inventory::submit! {
            #sdk::CommandRegistration { spec: &#static_ident }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use syn::DeriveInput;

    /// Expand a declaration, and take the message of the error it earns.
    fn refuse(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match expand(input) {
            Ok(_) => panic!("the declaration was accepted: {source}"),
            Err(error) => error.to_string(),
        }
    }

    /// Expand a declaration, and take the tokens it produced.
    fn accept(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match expand(input) {
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
        // The macro does not know the name the using crate knows the SDK by, so
        // it asks `proc-macro-crate`. The answer is a path, and the path is
        // what the generated code says.
        let root = sdk_root().to_string();
        assert!(root.contains("craftkit_sdk") || root == "crate", "{root}");
    }
}
