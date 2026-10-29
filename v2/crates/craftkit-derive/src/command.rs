//! The `#[derive(Command)]` — the command's declaration, reader and
//! registration.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

use crate::common::{anchored_impl, check_id, doc_of, label_from_id, missing, screaming, sdk_root, snake};

/// The `#[command(...)]` attribute.
#[derive(Default)]
pub struct CommandArgs {
    pub id: Option<LitStr>,
    pub label: Option<LitStr>,
    pub area: Option<LitStr>,
    pub icon: Option<LitStr>,
    pub shortcut: Option<LitStr>,
    pub capability: Option<LitStr>,
    pub layer: Option<LitStr>,
    pub enabled: Option<syn::Path>,
    /// Where the command goes in the menu: `menu("Edit", "Trim")`.
    pub menu: Vec<LitStr>,
    pub mutates: bool,
    pub undoable: bool,
    pub no_prefix: bool,
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
                "capability" => {
                    input.parse::<Token![=]>()?;
                    args.capability = Some(input.parse::<LitStr>()?);
                }
                "layer" => {
                    input.parse::<Token![=]>()?;
                    args.layer = Some(input.parse::<LitStr>()?);
                }
                "enabled" => {
                    input.parse::<Token![=]>()?;
                    args.enabled = Some(input.parse::<syn::Path>()?);
                }
                "menu" => {
                    // A path: `menu("Edit", "Trim")`. The names are the
                    // path from the menu bar down to the command.
                    let content;
                    syn::parenthesized!(content in input);
                    while !content.is_empty() {
                        args.menu.push(content.parse::<LitStr>()?);
                        if content.is_empty() {
                            break;
                        }
                        content.parse::<Token![,]>()?;
                    }
                }
                "mutates" => args.mutates = true,
                "undoable" => args.undoable = true,
                "no_prefix" => args.no_prefix = true,
                other => {
                    return Err(Error::new(
                        field.span(),
                        format!(
                            "unknown command field `{other}`: a command has `id`, and optionally \
                             `label`, `area`, `icon`, `shortcut`, `capability`, `layer`, \
                             `enabled`, `menu`, `mutates`, `undoable` and `no_prefix`"
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
pub struct ParamArgs {
    /// The other spelling of this parameter.
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

/// Write the command.
pub fn expand(input: syn::DeriveInput) -> Result<TokenStream2> {
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
    let capability =
        args.capability.clone().map(|value| value.value()).unwrap_or_else(|| "command".to_string());
    let layer = args.layer.clone().map(|value| value.value()).unwrap_or_else(|| "core".to_string());
    let doc = doc_of(&input.attrs);

    // The fields, and what each one says.
    let mut names: Vec<String> = Vec::new();
    let mut idents: Vec<Ident> = Vec::new();
    let mut alternatives: Vec<String> = Vec::new();
    let mut reads: Vec<TokenStream2> = Vec::new();
    let mut declarations: Vec<TokenStream2> = Vec::new();

    for field in &fields {
        let field_name = field.ident.clone().ok_or_else(|| missing("command"))?;
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

        let (read, declaration) = match (&param.alternative, &shape) {
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
                quote!(<#ty as #sdk::ParamKind>::read(params, #field_name_string)),
                quote!(#sdk::ParamDoc::optional(
                    #field_name_string,
                    <#inner as #sdk::ParamKind>::DOC,
                    #help,
                )),
            ),
            (None, Shape::Plain) => (
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
        declarations.push(declaration);
    }

    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let run_ident = format_ident!("__craftkit_run_{}", snake(&name));
    let shortcut_expression = match &shortcut {
        Some(shortcut) => quote!(Some(#shortcut)),
        None => quote!(None),
    };
    let enabled = match &args.enabled {
        Some(enabled) => quote!(#enabled),
        None => quote!(#sdk::always),
    };
    // The menu path. An empty path means the command sits under its area,
    // which is where it sat before the path existed.
    let menu_names: Vec<LitStr> = args.menu.clone();
    let mutates = args.mutates;
    let undoable = args.undoable;
    let doc_literal = LitStr::new(&doc, id.span());
    let label_literal = LitStr::new(&label, id.span());
    let area_literal = LitStr::new(&area, id.span());
    let menu_expression =
        if menu_names.is_empty() { quote!(&[#area_literal]) } else { quote!(&[#(#menu_names),*]) };
    let icon_literal = LitStr::new(&icon, id.span());
    let capability_literal = LitStr::new(&capability, id.span());
    let layer_literal = LitStr::new(&layer, id.span());

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
            capability: #capability_literal,
            layer: #layer_literal,
            icon: #icon_literal,
            doc: #doc_literal,
            shortcut: #shortcut_expression,
            menu: #menu_expression,
            params: &[
                #(#declarations),*
            ],
            mutates: #mutates,
            undoable: #undoable,
            enabled: #enabled,
            run: #sdk::CommandRun::Static(#run_ident),
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}
