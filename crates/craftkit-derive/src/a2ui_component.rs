//! The `#[derive(A2uiComponent)]` — and the `components! { … }` table beside it.
//!
//! # What a row is
//!
//! An a2ui **component** is a struct whose fields are its **props**. The
//! compiler writes:
//!
//! - the **reader** — from the fields' types, with the SDK's own
//!   `ParamKind` machinery;
//! - the **`ParamDoc`s** — the declarations, one per field;
//! - the **published** schema — a *call* into the layer's shapes, so that
//!   the layer owns the shapes and the row owns the names;
//! - the **typed view** of a node's payload — `of(node)`;
//! - the **kind** — a `NodeKindSpec`;
//! - the **catalogue entry** — a `CatalogEntry`; and
//! - the **two registrations**, so the linker puts both in their registries.
//!
//! # The two spellings
//!
//! Like the commands, there are two:
//!
//! - the **derive**, for a single component — the props are the struct's
//!   fields and the painter is the struct's methods; and
//! - the **`components!` table**, for a family of them — one row per
//!   component, and the compiler writes the same work for every row.
//!
//! # The layer is named by **path**, not by dependency
//!
//! This crate is **L0** and may not depend on the a2ui layer (**L3**), so it
//! cannot name `craftkit-a2ui` itself. The row names **two** paths — the
//! **table** (the property names) and the **shapes** (the re-derived schema
//! text) — and everything this emits resolves **where the macro is used**,
//! in the plugin (**L4**), where L4 → L3 → L2 is allowed.
//!
//! So the derive emits **no a2ui type**: it emits a path, a call and a
//! compile-time check, and nothing else. A binary without the plugin has no
//! `a2ui.*` kinds, and this crate knows nothing about them.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

use crate::common::{anchored_impl, check_id, check_id_text, doc_of, screaming_text, sdk_root};

/// The `#[a2ui_component(...)]` attribute.
#[derive(Default)]
pub struct ComponentArgs {
    /// The **a2ui** name: `Text`. It is the key in the catalogue, and it is
    /// **PascalCase**, so it is exempt from the dotted lower-case id rule.
    pub name: Option<LitStr>,
    /// The craftkit **kind** id: `a2ui.text`.
    pub kind: Option<LitStr>,
    /// The layer the component belongs to.
    pub layer: Option<LitStr>,
    /// The path of the **table** — the property names.
    pub table: Option<LitStr>,
    /// The path of the **shapes** — the re-derived schema text.
    pub shapes: Option<LitStr>,
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ComponentArgs::default();
        while !input.is_empty() {
            let field: Ident = input.parse()?;
            match field.to_string().as_str() {
                "name" => {
                    input.parse::<Token![=]>()?;
                    args.name = Some(input.parse::<LitStr>()?);
                }
                "kind" => {
                    input.parse::<Token![=]>()?;
                    args.kind = Some(input.parse::<LitStr>()?);
                }
                "layer" => {
                    input.parse::<Token![=]>()?;
                    args.layer = Some(input.parse::<LitStr>()?);
                }
                "table" => {
                    input.parse::<Token![=]>()?;
                    args.table = Some(input.parse::<LitStr>()?);
                }
                "shapes" => {
                    input.parse::<Token![=]>()?;
                    args.shapes = Some(input.parse::<LitStr>()?);
                }
                other => {
                    return Err(Error::new(
                        field.span(),
                        format!(
                            "unknown component field `{other}`: a component has `name`, and \
                             optionally `kind`, `layer`, `table` and `shapes`"
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

/// The `#[a2ui_prop(...)]` attribute on a field.
#[derive(Default)]
pub struct PropArgs {
    /// The **a2ui definition** the prop uses, literally: `DynamicString`,
    /// `Weight`, `Child`, `ChildList`, `Action`, `DynamicBoolean`.
    pub definition: Option<LitStr>,
}

impl Parse for PropArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = PropArgs::default();
        while !input.is_empty() {
            // Two spellings: `#[a2ui_prop(DynamicString)]` — the bare name,
            // which is how a row reads — and `#[a2ui_prop(definition =
            // "DynamicString")]`, which says the same thing at more length.
            if input.peek(syn::Ident) && !input.peek2(Token![=]) {
                let name: Ident = input.parse()?;
                args.definition = Some(LitStr::new(&name.to_string(), name.span()));
            } else {
                let field: Ident = input.parse()?;
                match field.to_string().as_str() {
                    "definition" => {
                        input.parse::<Token![=]>()?;
                        args.definition = Some(input.parse::<LitStr>()?);
                    }
                    other => {
                        return Err(Error::new(
                            field.span(),
                            format!(
                                "unknown prop field `{other}`: a prop names its definition, \
                                 as `#[a2ui_prop(DynamicString)]`"
                            ),
                        ));
                    }
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

/// One prop: the field it came from, and what it says.
pub struct Prop {
    /// The field's name — the prop's name.
    pub name: String,
    /// The field's ident.
    pub ident: Ident,
    /// The field's type.
    pub ty: syn::Type,
    /// The **inner** type, when the field is an `Option`.
    pub inner: Option<syn::Type>,
    /// The **left** type, when the field is an `Either`.
    pub left: Option<syn::Type>,
    /// The **right** type, when the field is an `Either`.
    ///
    /// The other spelling's kind: a pair is two kinds, and the
    /// declaration says what each one takes.
    pub right: Option<syn::Type>,
    /// The **other spelling** of the prop, when it has one.
    pub alternative: Option<String>,
    /// The a2ui definition, if the prop names one.
    pub definition: Option<String>,
    /// The doc comment — the prop's one line about itself.
    pub help: String,
}

/// `#[param(...)]` on a field.
///
/// The only field it has is `or` — the **other spelling** of a prop that is
/// one value written two ways. An `Either` prop needs it, and a prop that
/// is not an `Either` may not have it.
#[derive(Default)]
pub struct ParamArgs {
    /// The other spelling of this prop.
    pub alternative: Option<LitStr>,
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
                        format!("unknown prop field `{other}`: a prop has `or`"),
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

/// The **second** type argument of a path type, if it has one.
fn second_argument(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .filter_map(|argument| match argument {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .nth(1),
        _ => None,
    }
}

/// Whether a type is an `Option<T>`, and what the `T` is.
fn optional_inner(ty: &syn::Type) -> Option<syn::Type> {
    match last_segment(ty).map(Ident::to_string).as_deref() {
        Some("Option") => first_argument(ty).cloned(),
        _ => None,
    }
}

/// Whether a type is an `Either<A, B>`, and what the `A` is.
///
/// An `Either` is one prop written two ways, so it needs the **other
/// spelling** — `#[param(or = "…")]` — and without it the prop cannot be
/// read.
fn either_left(ty: &syn::Type) -> Option<syn::Type> {
    match last_segment(ty).map(Ident::to_string).as_deref() {
        Some("Either") => first_argument(ty).cloned(),
        _ => None,
    }
}

/// Whether a type is an `Either<A, B>`, and what the `B` is.
///
/// The **second** half is the other spelling's kind: a pair is two
/// kinds, and the declaration says what each one takes.
fn either_right(ty: &syn::Type) -> Option<syn::Type> {
    match last_segment(ty).map(Ident::to_string).as_deref() {
        Some("Either") => second_argument(ty).cloned(),
        _ => None,
    }
}

/// Read the props out of a set of named fields.
pub fn props_of(fields: &syn::FieldsNamed) -> Result<Vec<Prop>> {
    let mut props: Vec<Prop> = Vec::new();
    for field in &fields.named {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(Span::call_site(), "a component is a struct with named fields"))?;
        let name = ident.to_string();
        if props.iter().any(|prop| prop.name == name) {
            return Err(Error::new(ident.span(), format!("`{name}` is declared twice")));
        }
        let attribute = field
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("a2ui_prop"))
            .map(|attribute| attribute.parse_args_with(PropArgs::parse))
            .transpose()?;
        let definition = attribute.and_then(|attribute| attribute.definition).map(|value| value.value());
        let ty = field.ty.clone();
        let inner = optional_inner(&ty);
        let left = either_left(&ty);
        let right = either_right(&ty);
        // The **other spelling**: `#[param(or = "…")]`.
        let alternative = field
            .attrs
            .iter()
            .find(|attribute| attribute.path().is_ident("param"))
            .map(|attribute| attribute.parse_args_with(ParamArgs::parse))
            .transpose()?
            .and_then(|args| args.alternative)
            .map(|value| value.value());
        // An `Either` needs the other spelling, and a prop that is not an
        // `Either` may not have one: the shape and the declaration say the
        // same thing, or they are both wrong.
        match (&left, &alternative) {
            (None, Some(_)) => {
                return Err(Error::new(
                    ident.span(),
                    format!("`{name}` has another spelling, so its type must be an `Either`"),
                ));
            }
            (Some(_), None) => {
                return Err(Error::new(
                    ident.span(),
                    format!(
                        "`{name}` is an `Either`, so it needs the other spelling: \
                         `#[param(or = \"…\")]`"
                    ),
                ));
            }
            _ => {}
        }
        // An `Either` is **two** kinds: the declaration says what each
        // spelling takes, so both types have to be there.
        if left.is_some() && right.is_none() {
            return Err(Error::new(
                ident.span(),
                format!("`{name}` is an `Either`, so it needs two types: `Either<A, B>`"),
            ));
        }
        props.push(Prop {
            name,
            ident,
            ty,
            inner,
            left,
            right,
            alternative,
            definition,
            help: doc_of(&field.attrs),
        });
    }
    Ok(props)
}

/// The **kind id** of a component: the a2ui name, lower-cased, with the
/// layer's prefix.
///
/// `Text` → `a2ui.text`, `TextField` → `a2ui.textfield`, `Divider` →
/// `a2ui.divider`. The id is **never** the a2ui name itself, because the
/// registry refuses an id that is not dotted and lower case — and the a2ui
/// name is PascalCase.
fn kind_of(name: &str, prefix: &str) -> String {
    format!("{prefix}.{}", name.to_lowercase())
}

/// The prefix of the a2ui kind ids: `a2ui`.
const A2UI_PREFIX: &str = "a2ui";

/// The name of the struct a component's row makes: `Text` → `TextComponent`.
///
/// The name is **checked**, and not formatted: a name that is not a name
/// a struct can be called is refused with a message that says so — and
/// not a panic in the compiler's middle.
fn struct_of(name: &str) -> Result<Ident> {
    let candidate = format!("{name}Component");
    syn::parse_str::<Ident>(&candidate).map_err(|_| {
        Error::new(
            Span::call_site(),
            format!("the a2ui name `{name}` is not a name a component's struct can be called"),
        )
    })
}

/// Write one component from a row.
///
/// The **props** are the fields; the **painter** and the **hit test** are
/// the struct's own methods, and the macro calls them.
pub fn expand(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "a component is a struct with named fields: the fields are its props",
                ));
            }
        },
        _ => {
            return Err(Error::new(Span::call_site(), "a component is a struct"));
        }
    };
    let props = props_of(&fields)?;

    let args = input
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("a2ui_component"))
        .map(|attribute| attribute.parse_args_with(ComponentArgs::parse))
        .transpose()?
        .unwrap_or_default();

    let name = args
        .name
        .clone()
        .ok_or_else(|| {
            Error::new(
                Span::call_site(),
                "a component needs an a2ui name: `#[a2ui_component(name = \"Text\")]`",
            )
        })?
        .value();
    let layer = args.layer.clone().map(|value| value.value()).unwrap_or_else(|| A2UI_PREFIX.to_string());
    let table = args.table.clone().ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "a component needs the layer's table: `#[a2ui_component(table = \"…::defs\")]`",
        )
    })?;
    let shapes = args.shapes.clone().ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "a component needs the layer's shapes: `#[a2ui_component(shapes = \"…::schema\")]`",
        )
    })?;
    // The **kind** is the a2ui name under the layer's own prefix: the
    // layer says what it is called, and the prefix is the layer's id.
    let kind = match &args.kind {
        Some(kind) => {
            check_id(kind)?;
            kind.value()
        }
        None => kind_of(&name, &layer),
    };
    check_id_text(&kind, Span::call_site())?;
    // And the **struct** is the one the derive writes into: the struct's
    // own name is the name the a2ui name makes, and a struct with
    // another name would get an implementation of a struct that is not
    // there.
    let struct_name = struct_of(&name)?;
    if input.ident != struct_name {
        return Err(Error::new(
            input.ident.span(),
            format!(
                "the struct is called `{}`, and the a2ui name `{name}` makes it `{}`",
                input.ident, struct_name
            ),
        ));
    }
    // The doc comment is the component's one line about itself: it is
    // what the catalogue shows and what the schema says.
    let doc = doc_of(&input.attrs);

    expand_component(&sdk, &Row { struct_name, name, kind, layer, table, shapes, props, doc })
}

/// The two paths, as a `syn::Path`.
///
/// A path is a **string** in the row, and it is resolved **where the macro
/// is used** — so the derive never names the layer itself.
fn path_of(literal: &LitStr) -> Result<syn::Path> {
    syn::parse_str::<syn::Path>(&literal.value())
        .map_err(|error| Error::new(literal.span(), format!("`{}` is not a path: {error}", literal.value())))
}

/// What one component's row says.
///
/// The row's facts, in one place, so that the expansion has one argument
/// and not nine.
pub struct Row {
    /// The struct the row makes: `TextComponent`.
    pub struct_name: Ident,
    /// The **a2ui** name: `Text`.
    pub name: String,
    /// The craftkit **kind** id: `a2ui.text`.
    pub kind: String,
    /// The layer the component belongs to.
    pub layer: String,
    /// The path of the layer's **table** — the property names.
    pub table: LitStr,
    /// The path of the layer's **shapes** — the re-derived schema text.
    pub shapes: LitStr,
    /// The props: the fields, and their types.
    pub props: Vec<Prop>,
    /// The component's one line about itself.
    pub doc: String,
}

/// Write one component: the struct's methods, the two statics, the two
/// registrations and the compile-time check.
pub fn expand_component(sdk: &TokenStream2, row: &Row) -> Result<TokenStream2> {
    let Row { struct_name, name, kind, layer, table, shapes, props, doc } = row;
    let shapes_path = path_of(shapes)?;

    // The **reader**: the fields are the params, and the types say what
    // they are.
    let mut reads = Vec::new();
    let mut declarations = Vec::new();
    let mut idents = Vec::new();
    for prop in props.iter() {
        let prop_name = &prop.name;
        let ident = &prop.ident;
        let ty = &prop.ty;
        let help = prop.help.as_str();
        // An `Either` is one prop written two ways: it is read with the
        // other spelling, and it is declared with it.
        if let (Some(left), Some(right), Some(alternative)) = (&prop.left, &prop.right, &prop.alternative) {
            let alternative = alternative.as_str();
            reads.push(quote! {
                let #ident = <#ty>::read_either(params, #prop_name, #alternative)
                    .map_err(|source| #sdk::EngineError::Param {
                        id: #prop_name.to_string(),
                        source,
                    })?;
            });
            declarations.push(quote! {
                #sdk::ParamDoc::either(
                    #prop_name,
                    <#left as #sdk::ParamKind>::DOC,
                    <#right as #sdk::ParamKind>::DOC,
                    #help,
                    #alternative,
                )
                .with_type_name(<#left as #sdk::ParamKind>::NAME)
                .with_unit(<#left as #sdk::ParamKind>::UNIT)
                .with_accepts_text(<#left as #sdk::ParamKind>::ACCEPTS_TEXT)
                .with_alternative_accepts_text(<#right as #sdk::ParamKind>::ACCEPTS_TEXT)
            });
            idents.push(ident.clone());
            continue;
        }
        reads.push(quote! {
            let #ident = <#ty as #sdk::ParamKind>::read(params, #prop_name)
                .map_err(|source| #sdk::EngineError::Param {
                    id: #prop_name.to_string(),
                    source,
                })?;
        });
        match &prop.inner {
            Some(inner) => declarations.push(quote! {
                #sdk::ParamDoc::optional(
                    #prop_name,
                    <#inner as #sdk::ParamKind>::DOC,
                    #help,
                )
                .with_type_name(<#inner as #sdk::ParamKind>::NAME)
                .with_unit(<#inner as #sdk::ParamKind>::UNIT)
            }),
            None => declarations.push(quote! {
                #sdk::ParamDoc::required(
                    #prop_name,
                    <#ty as #sdk::ParamKind>::DOC,
                    #help,
                )
                .with_type_name(<#ty as #sdk::ParamKind>::NAME)
                .with_unit(<#ty as #sdk::ParamKind>::UNIT)
            }),
        }
        idents.push(ident.clone());
    }

    // The **names**, and the **definitions** — the published schema's
    // half. A plain prop has no definition, and the layer owns its shape.
    let prop_names: Vec<LitStr> =
        props.iter().map(|prop| LitStr::new(&prop.name, Span::call_site())).collect();
    // A prop's definition, as a **call**: `Some("Weight")` or `None`.
    // A plain prop has no definition, and the layer owns its shape.
    let prop_definitions: Vec<TokenStream2> = props
        .iter()
        .map(|prop| match &prop.definition {
            Some(value) => {
                let literal = LitStr::new(value, Span::call_site());
                quote!(::core::option::Option::Some(#literal))
            }
            None => quote!(::core::option::Option::None),
        })
        .collect();

    // The row's own **optionality**: a prop declared as `Option<T>` is
    // optional, and the published schema says so — the layer knows the
    // names and the shapes, and the row knows this.
    let prop_optionals: Vec<bool> =
        props.iter().map(|prop| prop.inner.is_some() || prop.left.is_some()).collect();
    // And the **other spelling** of a prop written two ways: the schema
    // must name it, or the schema forbids half the prop.
    let prop_alternatives: Vec<TokenStream2> = props
        .iter()
        .map(|prop| match &prop.alternative {
            Some(value) => {
                let literal = LitStr::new(value, Span::call_site());
                quote!(::core::option::Option::Some(#literal))
            }
            None => quote!(::core::option::Option::None),
        })
        .collect();

    // The **byte-array** names, for the compile-time check against the
    // layer's table. A `const` cannot compare `&str`, so the table holds
    // bytes and the check walks them.
    let byte_names: Vec<syn::LitByteStr> =
        props.iter().map(|prop| syn::LitByteStr::new(prop.name.as_bytes(), Span::call_site())).collect();

    // The statics are named after the **a2ui** name: `Text` is
    // `TEXT_COMPONENT` and `TEXT_CATALOG`.
    let screaming = screaming_text(name);
    let static_ident = Ident::new(&format!("{screaming}_COMPONENT"), Span::call_site());
    let catalog_ident = Ident::new(&format!("{screaming}_CATALOG"), Span::call_site());
    let anchor = anchored_impl(sdk, struct_name, &static_ident);

    // The layer's **table** holds one row per component, and the row is
    // named after the component — `Text` is `TEXT`. So the check names
    // the row the component owns.
    let table_row = Ident::new(&screaming_text(name), Span::call_site());
    let mut table_path = path_of(table)?;
    table_path.segments.push(syn::PathSegment { ident: table_row, arguments: syn::PathArguments::None });
    let mut matches_path = path_of(table)?;
    matches_path.segments.push(syn::PathSegment {
        ident: Ident::new("matches", Span::call_site()),
        arguments: syn::PathArguments::None,
    });
    let doc_literal = LitStr::new(doc, Span::call_site());
    let name_literal = LitStr::new(name, Span::call_site());
    let kind_literal = LitStr::new(kind, Span::call_site());
    let layer_literal = LitStr::new(layer, Span::call_site());
    let label_literal = LitStr::new(name, Span::call_site());

    Ok(quote! {
        impl #struct_name {
            /// Read the component's props.
            ///
            /// Written by the macro: the fields are the props, and the
            /// types say what they are.
            pub fn read(params: &#sdk::Params<'_>) -> Result<Self, #sdk::EngineError> {
                #(#reads)*
                Ok(Self { #(#idents,)* })
            }

            /// The props' declarations.
            ///
            /// The **craftkit** side of the component: the names, the
            /// kinds and whether each is required.
            pub fn doc() -> Vec<#sdk::ParamDoc> {
                vec![ #(#declarations,)* ]
            }

            /// The **published** schema: a2ui's shape.
            ///
            /// The layer owns the shapes, and the row owns the names —
            /// so this is a **call** into the layer, not a copy of it.
            pub fn schema() -> #sdk::serde_json::Value {
                #shapes_path::published_with(
                    #name_literal,
                    &[#(#prop_names),*],
                    &[#(#prop_definitions),*],
                    &[#(#prop_optionals),*],
                    &[#(#prop_alternatives),*],
                )
            }

            /// The typed view of a node's **payload**.
            ///
            /// A node whose payload does not fit is `None`: it is drawn
            /// as nothing, which is what any other kind's missing
            /// payload means.
            pub fn of(node: &#sdk::Node) -> Option<Self> {
                let params = #sdk::Params::new(&node.data);
                Self::read(&params).ok()
            }
        }

        #[doc = #doc_literal]
        pub static #static_ident: #sdk::NodeKindSpec = #sdk::NodeKindSpec {
            kind: #kind_literal,
            label: #label_literal,
            layer: #layer_literal,
            paint: |node, document, canvas| #struct_name::paint(node, document, canvas),
            hit: |node, document, point| #struct_name::hit(node, document, point),
            evaluate: None,
            place: None,
            part: None,
        };

        #[doc = #doc_literal]
        pub static #catalog_ident: #sdk::CatalogEntry = #sdk::CatalogEntry {
            id: #kind_literal,
            name: #name_literal,
            label: #label_literal,
            layer: #layer_literal,
            doc: #doc_literal,
            schema: #struct_name::schema,
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#catalog_ident }
        }

        /// **The compile-time check.**
        ///
        /// The row and the layer's table must hold the **same set** of
        /// property names — a prop the row declares and the table lacks,
        /// and a prop the table holds and the row lacks, are both compile
        /// errors. And it runs here, in the plugin's own compilation unit,
        /// so the mistake is reported where it was made.
        const _: () = {
            // The row's names, as bytes. The **type** is written out, so
            // that each byte string is unsized into a `&[u8]` — a byte
            // string is `&[u8; N]`, and two of them with different
            // lengths are not an array of one type.
            let row: &[&[u8]] = &[#(#byte_names),*];
            assert!(
                #matches_path(#table_path, row),
                "the row's props and the layer's table do not hold the same names",
            );
        };
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expand a declaration, and take the tokens it produced.
    fn accept(source: &str) -> String {
        let input: syn::DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match expand(input) {
            Ok(tokens) => tokens.to_string(),
            Err(error) => panic!("the declaration was refused: {error}"),
        }
    }

    /// Expand a declaration, and take the message of the error it earns.
    fn refuse(source: &str) -> String {
        let input: syn::DeriveInput = syn::parse_str(source).expect("the declaration parses");
        match expand(input) {
            Ok(_) => panic!("the declaration was accepted: {source}"),
            Err(error) => error.to_string(),
        }
    }

    const GOOD: &str = r#"
/// A line of text.
///
/// The doc comment is the component's one line about itself.
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", kind = "a2ui.text", layer = "a2ui",
                 table = "craftkit_a2ui::defs",
                 shapes = "craftkit_a2ui::schema")]
pub struct TextComponent {
    /// The text to show.
    #[a2ui_prop(DynamicString)]
    pub text: Text,
    /// A hint for the base text size.
    pub variant: Option<Text>,
    /// How much of the row this takes.
    #[a2ui_prop(Weight)]
    pub weight: Option<Number>,
}
"#;

    /// A name that is **not a name** is refused — and not a panic in the
    /// compiler's middle.
    #[test]
    fn a_name_that_is_not_a_name_is_refused() {
        let source = GOOD.replace("name = \"Text\"", "name = \"Text Field\"");
        let message = refuse(&source);
        assert!(message.contains("not a name"), "{message}");
    }

    /// The struct's **own name** is the name the a2ui name makes, and a
    /// struct with another name is refused.
    #[test]
    fn a_struct_with_another_name_is_refused() {
        let source = GOOD.replace("pub struct TextComponent", "pub struct SomethingElse");
        let message = refuse(&source);
        assert!(message.contains("SomethingElse"), "{message}");
    }

    /// An `Either` with **another spelling** is accepted: the attribute
    /// is declared, and the prop is read both ways.
    #[test]
    fn an_either_with_another_spelling_is_accepted() {
        let source = r#"
/// A prop written two ways.
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", kind = "a2ui.text", layer = "a2ui",
                 table = "craftkit_a2ui::defs",
                 shapes = "craftkit_a2ui::schema")]
pub struct TextComponent {
    /// The text to show.
    #[param(or = "label")]
    pub text: Either<Text, String>,
}
"#;
        let tokens = accept(source);
        assert!(tokens.contains("read_either"), "{tokens}");
        assert!(tokens.contains("label"), "{tokens}");
    }

    /// The macro writes the **reader**.
    #[test]
    fn the_reader_is_written() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("fn read"), "{tokens}");
        assert!(tokens.contains("ParamKind"), "{tokens}");
        assert!(tokens.contains("Params"), "{tokens}");
    }

    /// And the **declarations** — the `ParamDoc`s.
    #[test]
    fn the_declarations_are_written() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("ParamDoc"), "{tokens}");
        assert!(tokens.contains("required"), "{tokens}");
        assert!(tokens.contains("optional"), "{tokens}");
    }

    /// And the **published** schema — a call into the layer's shapes.
    #[test]
    fn the_published_schema_is_a_call_into_the_layer() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("craftkit_a2ui :: schema :: published"), "{tokens}");
        assert!(tokens.contains("DynamicString"), "{tokens}");
        assert!(tokens.contains("Weight"), "{tokens}");
    }

    /// And the **typed view** of a payload.
    #[test]
    fn the_typed_view_of_a_payload_is_written() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("fn of"), "{tokens}");
        assert!(tokens.contains("node . data"), "{tokens}");
    }

    /// And the **two statics** — the kind and the catalogue entry.
    #[test]
    fn the_two_statics_are_written() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("TEXT_COMPONENT"), "{tokens}");
        assert!(tokens.contains("TEXT_CATALOG"), "{tokens}");
        assert!(tokens.contains("NodeKindSpec"), "{tokens}");
        assert!(tokens.contains("CatalogEntry"), "{tokens}");
    }

    /// And the **two registrations**.
    #[test]
    fn the_two_registrations_are_written() {
        let tokens = accept(GOOD);
        assert_eq!(tokens.matches("inventory :: submit").count(), 2, "{tokens}");
    }

    /// And the **compile-time check** against the layer's table.
    #[test]
    fn the_compile_time_check_is_written() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("matches"), "{tokens}");
        assert!(tokens.contains("craftkit_a2ui :: defs"), "{tokens}");
        // The row's names, as byte arrays.
        assert!(tokens.contains("b\"text\""), "{tokens}");
        assert!(tokens.contains("b\"variant\""), "{tokens}");
        assert!(tokens.contains("b\"weight\""), "{tokens}");
    }

    /// The **kind id** is the a2ui name, lower-cased, with the prefix.
    #[test]
    fn the_kind_id_is_the_a2ui_name_lower_cased() {
        let tokens = accept(
            r#"
/// A line of text.
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(tokens.contains("\"a2ui.text\""), "{tokens}");
        // And the a2ui name itself is the **published** name.
        assert!(tokens.contains("name : \"Text\""), "{tokens}");
    }

    /// A component with **no** kind is given the rule's one.
    #[test]
    fn a_component_with_no_kind_is_given_the_rule() {
        let tokens = accept(
            r#"
/// A field of text.
#[derive(A2uiComponent)]
#[a2ui_component(name = "TextField", table = "a::defs", shapes = "a::schema")]
pub struct TextFieldComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(tokens.contains("\"a2ui.textfield\""), "{tokens}");
    }

    /// A component with **no** layer is in the a2ui layer.
    #[test]
    fn a_component_with_no_layer_is_in_the_a2ui_layer() {
        let tokens = accept(
            r#"
/// A line of text.
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(tokens.contains("layer : \"a2ui\""), "{tokens}");
    }

    /// An `Option<T>` is optional, and a `T` is not.
    #[test]
    fn an_option_is_optional_and_a_plain_type_is_not() {
        let tokens = accept(GOOD);
        // `text` is required, `variant` and `weight` are optional.
        let required = tokens.find("required").expect("there is a required");
        let optional = tokens.find("optional").expect("there is an optional");
        assert!(required < optional, "the required come first: {tokens}");
    }

    /// A component with **no name** is refused.
    #[test]
    fn a_component_with_no_name_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(message.contains("needs an a2ui name"), "{message}");
    }

    /// A component with **no table** is refused: the check needs one.
    #[test]
    fn a_component_with_no_table_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(message.contains("needs the layer's table"), "{message}");
    }

    /// A component with **no shapes** is refused: the schema needs them.
    #[test]
    fn a_component_with_no_shapes_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(message.contains("needs the layer's shapes"), "{message}");
    }

    /// A kind that is **not** dotted and lower case is refused.
    #[test]
    fn a_kind_that_is_not_well_formed_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", kind = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
}
"#,
        );
        assert!(message.contains("is not a contribution id"), "{message}");
    }

    /// A component that is **not** a struct with named fields is refused.
    #[test]
    fn a_component_that_is_not_a_struct_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent;
"#,
        );
        assert!(message.contains("named fields"), "{message}");
    }

    /// A **duplicate prop** is refused.
    #[test]
    fn a_duplicate_prop_is_refused() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Text,
    /// The text again.
    pub text: Text,
}
"#,
        );
        assert!(message.contains("declared twice"), "{message}");
    }

    /// An `Either` **needs** the other spelling: it is one prop written
    /// two ways, and without the second the prop cannot be read.
    #[test]
    fn an_either_needs_the_other_spelling() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    pub text: Either<Text, Number>,
}
"#,
        );
        assert!(message.contains("needs the other spelling"), "{message}");
    }

    /// And a prop that is **not** an `Either` may not have one.
    #[test]
    fn a_prop_that_is_not_an_either_may_not_have_another_spelling() {
        let message = refuse(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    #[param(or = "content")]
    pub text: Text,
}
"#,
        );
        assert!(message.contains("must be an `Either`"), "{message}");
    }

    /// And an `Either` **with** the other spelling is read with it.
    #[test]
    fn an_either_with_the_other_spelling_is_read_with_it() {
        let tokens = accept(
            r#"
#[derive(A2uiComponent)]
#[a2ui_component(name = "Text", table = "a::defs", shapes = "a::schema")]
pub struct TextComponent {
    /// The text to show.
    #[param(or = "content")]
    pub text: Either<Text, Number>,
}
"#,
        );
        assert!(tokens.contains("read_either"), "{tokens}");
        assert!(tokens.contains("either"), "{tokens}");
        assert!(tokens.contains("\"content\""), "{tokens}");
    }
}
