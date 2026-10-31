//! The derives for the other seams: node kinds, tools, panels and layers.
//!
//! Each one writes a **spec** and a **registration**, and each one is a
//! contribution — the same shape, the same registry, the same checks. That
//! is what makes a seam a seam and not a special case.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

use crate::common::{anchored_impl, check_id, check_layer_id, doc_of, screaming, sdk_root};

/// The fields every seam's attribute has.
#[derive(Default)]
pub struct SeamArgs {
    pub id: Option<LitStr>,
    pub label: Option<LitStr>,
    pub layer: Option<LitStr>,
    pub icon: Option<LitStr>,
    pub shortcut: Option<LitStr>,
    pub command: Option<LitStr>,
    pub kind: Option<LitStr>,
    pub name: Option<LitStr>,
    pub list: Vec<LitStr>,
    /// A flag: the kind has an evaluation of its own.
    pub evaluates: bool,
}

impl Parse for SeamArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = SeamArgs::default();
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
                "layer" => {
                    input.parse::<Token![=]>()?;
                    args.layer = Some(input.parse::<LitStr>()?);
                }
                "icon" => {
                    input.parse::<Token![=]>()?;
                    args.icon = Some(input.parse::<LitStr>()?);
                }
                "shortcut" => {
                    input.parse::<Token![=]>()?;
                    args.shortcut = Some(input.parse::<LitStr>()?);
                }
                "command" => {
                    input.parse::<Token![=]>()?;
                    args.command = Some(input.parse::<LitStr>()?);
                }
                "kind" => {
                    input.parse::<Token![=]>()?;
                    args.kind = Some(input.parse::<LitStr>()?);
                }
                "name" => {
                    input.parse::<Token![=]>()?;
                    args.name = Some(input.parse::<LitStr>()?);
                }
                "evaluates" => {
                    // A flag: the kind brings its own evaluation.
                    args.evaluates = true;
                }
                other => {
                    // A list: `requires("core")` or `commands("a", "b")`.
                    let body;
                    syn::parenthesized!(body in input);
                    while !body.is_empty() {
                        args.list.push(body.parse::<LitStr>()?);
                        if body.is_empty() {
                            break;
                        }
                        body.parse::<Token![,]>()?;
                    }
                    let _ = other;
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

/// The attribute of a seam, found by name.
fn args_of(input: &syn::DeriveInput, name: &str) -> Result<SeamArgs> {
    match input.attrs.iter().find(|attribute| attribute.path().is_ident(name)) {
        Some(attribute) => attribute.parse_args_with(SeamArgs::parse),
        None => Ok(SeamArgs::default()),
    }
}

/// The id, checked.
fn the_id(args: &SeamArgs, attribute: &str) -> Result<LitStr> {
    let id =
        args.id.clone().ok_or_else(|| Error::new(Span::call_site(), format!("a {attribute} needs an id")))?;
    check_id(&id)?;
    Ok(id)
}

/// The layer, defaulting to `core`.
fn the_layer(args: &SeamArgs) -> LitStr {
    match &args.layer {
        Some(layer) => LitStr::new(&layer.value(), layer.span()),
        None => LitStr::new("core", Span::call_site()),
    }
}

/// Write a node kind.
///
/// The kind's **paint** and **hit** are the kind's own methods, and the
/// macro calls them: the core never matches on a kind, so a plugin's kind
/// is drawn by the plugin.
///
/// The paint is given the **document**: a kind that follows a reference
/// cannot follow it without one.
pub fn expand_node_kind(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = args_of(&input, "node_kind")?;
    let kind = args
        .kind
        .clone()
        .or_else(|| args.id.clone())
        .ok_or_else(|| Error::new(Span::call_site(), "a node kind needs a `kind`"))?;
    check_id(&kind)?;
    let label = match &args.label {
        Some(label) => LitStr::new(&label.value(), label.span()),
        None => LitStr::new(&kind.value(), kind.span()),
    };
    let layer = the_layer(&args);
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, kind.span());

    // The evaluation, if the kind has one. A kind that does not evaluate
    // says nothing in the plan, and the node is simply not in it.
    let evaluate = if args.evaluates {
        quote! {
            evaluate: Some(|node, document| #name::evaluate(node, document)),
        }
    } else {
        quote! {
            evaluate: None,
        }
    };

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::NodeKindSpec = #sdk::NodeKindSpec {
            kind: #kind,
            label: #label,
            layer: #layer,
            paint: |node, document, canvas| #name::paint(node, document, canvas),
            hit: |node, document, point| #name::hit(node, document, point),
            #evaluate
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}

/// Write a tool.
pub fn expand_tool(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = args_of(&input, "tool")?;
    let id = the_id(&args, "tool")?;
    let label = match &args.label {
        Some(label) => LitStr::new(&label.value(), label.span()),
        None => LitStr::new(&id.value(), id.span()),
    };
    let layer = the_layer(&args);
    let icon = match &args.icon {
        Some(icon) => LitStr::new(&icon.value(), icon.span()),
        None => LitStr::new("", id.span()),
    };
    let shortcut = match &args.shortcut {
        Some(shortcut) => quote!(Some(#shortcut)),
        None => quote!(None),
    };
    let command = args
        .command
        .clone()
        .ok_or_else(|| Error::new(Span::call_site(), "a tool needs the command it runs"))?;
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, id.span());

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::ToolSpec = #sdk::ToolSpec {
            id: #id,
            label: #label,
            layer: #layer,
            icon: #icon,
            shortcut: #shortcut,
            command: #command,
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}

/// Write a panel.
pub fn expand_panel(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = args_of(&input, "panel")?;
    let id = the_id(&args, "panel")?;
    let label = match &args.label {
        Some(label) => LitStr::new(&label.value(), label.span()),
        None => LitStr::new(&id.value(), id.span()),
    };
    let layer = the_layer(&args);
    let commands = &args.list;
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, id.span());

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::PanelSpec = #sdk::PanelSpec {
            id: #id,
            label: #label,
            layer: #layer,
            commands: &[#(#commands),*],
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}

/// Write an evaluator.
///
/// The evaluator's own `evaluate` method is the evaluation, and the macro
/// writes the spec and the registration.
pub fn expand_evaluator(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = args_of(&input, "evaluator")?;
    let id = the_id(&args, "evaluator")?;
    let label = match &args.label {
        Some(label) => LitStr::new(&label.value(), label.span()),
        None => LitStr::new(&id.value(), id.span()),
    };
    let layer = the_layer(&args);
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, id.span());

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::EvaluatorSpec = #sdk::EvaluatorSpec {
            id: #id,
            label: #label,
            layer: #layer,
            doc: #doc_literal,
            evaluate: |document| #name::evaluate(document),
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}

/// Write a layer.
pub fn expand_layer(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = args_of(&input, "layer")?;
    let id = args.id.clone().ok_or_else(|| Error::new(Span::call_site(), "a layer needs an id"))?;
    check_layer_id(&id)?;
    let layer_name = match &args.name {
        Some(layer_name) => LitStr::new(&layer_name.value(), layer_name.span()),
        None => LitStr::new(&id.value(), id.span()),
    };
    let requires = &args.list;
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, id.span());

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::LayerSpec = #sdk::LayerSpec {
            id: #id,
            name: #layer_name,
            requires: &[#(#requires),*],
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}
