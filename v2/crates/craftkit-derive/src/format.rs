//! The `#[derive(Format)]` and `#[derive(Migration)]` — the file format, as
//! contributions.
//!
//! # What they are for
//!
//! The tree v2 replaces changes its format by adding one file per change and
//! wiring it in four places. Here:
//!
//! - a `#[derive(Format)]` gives a type the two functions a file needs —
//!   `to_json` and `from_json` — and implements the SDK's [`FormatValue`],
//!   so the type can be a field of another type that is;
//! - a `#[derive(Migration)]` gives a **step**: how a document moves from
//!   one version to the next. The step is a contribution, so the list of
//!   steps is the list of contributions, and the link fills it.
//!
//! A migration is a step and not a jump: it moves one version forward. The
//! ecosystem checks that, because a step that skipped a version would leave
//! a file that nothing can read.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Error, Ident, LitStr, Result, Token};

use crate::common::{anchored_impl, check_id, doc_of, screaming, sdk_root};

/// The `#[migration(...)]` attribute.
#[derive(Default)]
pub struct MigrationArgs {
    pub id: Option<LitStr>,
    pub family: Option<LitStr>,
    pub from: Option<syn::LitInt>,
    pub to: Option<syn::LitInt>,
    pub layer: Option<LitStr>,
}

impl Parse for MigrationArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = MigrationArgs::default();
        while !input.is_empty() {
            let field: Ident = input.parse()?;
            match field.to_string().as_str() {
                "id" => {
                    input.parse::<Token![=]>()?;
                    args.id = Some(input.parse::<LitStr>()?);
                }
                "family" => {
                    input.parse::<Token![=]>()?;
                    args.family = Some(input.parse::<LitStr>()?);
                }
                "from" => {
                    input.parse::<Token![=]>()?;
                    args.from = Some(input.parse::<syn::LitInt>()?);
                }
                "to" => {
                    input.parse::<Token![=]>()?;
                    args.to = Some(input.parse::<syn::LitInt>()?);
                }
                "layer" => {
                    input.parse::<Token![=]>()?;
                    args.layer = Some(input.parse::<LitStr>()?);
                }
                other => {
                    return Err(Error::new(
                        field.span(),
                        format!(
                            "unknown migration field `{other}`: a migration has `id`, `family`, `from`, \
                             `to` and `layer`"
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

/// Write a type's format functions.
///
/// The fields are written under their own names, and the values are read
/// through the [`FormatValue`] trait, so a field of any type that
/// implements it works.
pub fn expand_format(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => fields.named.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "a format is a struct with named fields: the fields are what is written",
                ));
            }
        },
        _ => {
            return Err(Error::new(Span::call_site(), "a format is a struct"));
        }
    };

    let mut names: Vec<String> = Vec::new();
    let mut idents: Vec<Ident> = Vec::new();
    let mut types: Vec<syn::Type> = Vec::new();
    let mut defaults: Vec<bool> = Vec::new();
    for field in &fields {
        let Some(ident) = field.ident.clone() else {
            return Err(Error::new(Span::call_site(), "a format's fields are named"));
        };
        let name = ident.to_string();
        if names.contains(&name) {
            return Err(Error::new(ident.span(), format!("`{name}` is written twice")));
        }
        names.push(name);
        idents.push(ident);
        types.push(field.ty.clone());
        defaults.push(has_default(&field.attrs)?);
    }

    let writes = idents.iter().zip(types.iter()).map(|(ident, ty)| {
        let name = ident.to_string();
        let key = LitStr::new(&name, ident.span());
        quote! {
            #key: <#ty as #sdk::FormatValue>::to_json(&self.#ident),
        }
    });
    let reads = idents.iter().zip(types.iter()).zip(defaults.iter()).map(|((ident, ty), default)| {
        let name = ident.to_string();
        if *default {
            // A field that may be **absent**: a file written before the
            // field existed is read as the field's default rather than
            // refused. That is what makes a new field a version and a
            // step rather than a break.
            quote! {
                #ident: {
                    let __field = value.get(#name).cloned()
                        .unwrap_or_else(|| <#ty as #sdk::FormatValue>::default_value());
                    <#ty as #sdk::FormatValue>::from_json(&__field)?
                },
            }
        } else {
            quote! {
                #ident: <#ty as #sdk::FormatValue>::from_json(
                    #sdk::format::field(value, #name)?
                )?,
            }
        }
    });

    Ok(quote! {
        impl #name {
            /// The value as it is written.
            pub fn to_json(&self) -> #sdk::serde_json::Value {
                #sdk::serde_json::json!({ #(#writes)* })
            }

            /// The value as it is read.
            pub fn from_json(value: &#sdk::serde_json::Value) -> Result<Self, String> {
                Ok(Self { #(#reads)* })
            }
        }

        impl #sdk::FormatValue for #name {
            fn to_json(&self) -> #sdk::serde_json::Value {
                #name::to_json(self)
            }

            fn from_json(value: &#sdk::serde_json::Value) -> Result<Self, String> {
                #name::from_json(value)
            }
        }
    })
}

/// Whether a field may be **absent** from a file.
///
/// The attribute is `#[format(default)]`: the field is still written, and
/// a file that does not have it is read as the field's default rather than
/// refused. It is how a new field becomes a version and a step rather than
/// a break in the chain.
fn has_default(attrs: &[syn::Attribute]) -> Result<bool> {
    for attribute in attrs {
        if !attribute.path().is_ident("format") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                return Ok(());
            }
            Err(meta.error("a format field knows one thing: `default`"))
        })?;
        return Ok(true);
    }
    Ok(false)
}

/// Write a migration.
///
/// The step is the type's own `apply` method, and the macro writes the
/// registration: the list of steps is the list of contributions.
pub fn expand_migration(input: syn::DeriveInput) -> Result<TokenStream2> {
    let sdk = sdk_root();
    let name = input.ident.clone();
    let args = input
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("migration"))
        .map(|attribute| attribute.parse_args_with(MigrationArgs::parse))
        .transpose()?
        .unwrap_or_default();

    let id = args.id.clone().ok_or_else(|| {
        Error::new(Span::call_site(), "a migration needs an id: `#[migration(id = \"…\")]`")
    })?;
    check_id(&id)?;
    let family = match &args.family {
        Some(family) => LitStr::new(&family.value(), family.span()),
        None => LitStr::new("core", Span::call_site()),
    };
    let from = args
        .from
        .clone()
        .ok_or_else(|| Error::new(Span::call_site(), "a migration needs the version it reads"))?;
    let to = args
        .to
        .clone()
        .ok_or_else(|| Error::new(Span::call_site(), "a migration needs the version it writes"))?;
    let layer = match &args.layer {
        Some(layer) => LitStr::new(&layer.value(), layer.span()),
        None => LitStr::new("core", Span::call_site()),
    };
    let static_ident = screaming(&name);
    let anchor = anchored_impl(&sdk, &name, &static_ident);
    let doc = doc_of(&input.attrs);
    let doc_literal = LitStr::new(&doc, id.span());

    Ok(quote! {
        #[doc = #doc_literal]
        pub static #static_ident: #sdk::MigrationSpec = #sdk::MigrationSpec {
            id: #id,
            family: #family,
            from: #from,
            to: #to,
            layer: #layer,
            apply: |value| #name::apply(value),
        };

        #anchor

        #sdk::inventory::submit! {
            #sdk::Registration { item: &#static_ident }
        }
    })
}
