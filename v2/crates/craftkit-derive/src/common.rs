//! The shared helpers of the derives.
//!
//! Every derive answers the same three questions — what is the SDK called
//! from here, what does the doc comment say, and is the id well formed —
//! and they are answered once, here.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Error, LitStr, Result};

/// The SDK's name, from the point of view of the crate using the macro.
pub fn sdk_root() -> TokenStream2 {
    match proc_macro_crate::crate_name("craftkit-sdk") {
        Ok(proc_macro_crate::FoundCrate::Itself) => quote!(crate),
        Ok(proc_macro_crate::FoundCrate::Name(name)) => {
            let ident = format_ident!("{}", name);
            quote!(::#ident)
        }
        Err(_) => quote!(::craftkit_sdk),
    }
}

/// The **first paragraph** of an item's doc comment, as one line.
///
/// The first paragraph, and not the whole comment: a command's
/// documentation is one line in a catalogue and one line in a schema, and a
/// comment that explains the implementation below it is for the reader of
/// the source, not for the caller.
pub fn doc_of(attrs: &[syn::Attribute]) -> String {
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

/// The static's name: the struct's name in `SCREAMING_SNAKE_CASE`.
///
/// `NodeMute` is `NODE_MUTE`, and `HelloGreet` is `HELLO_GREET` — the
/// camel case becomes the screaming case, one underscore at a time.
pub fn screaming(name: &syn::Ident) -> Ident {
    format_ident!("{}", screaming_text(&name.to_string()))
}

/// `NodeMute` as `NODE_MUTE`.
pub fn screaming_text(text: &str) -> String {
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
pub fn snake(name: &syn::Ident) -> String {
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
pub fn label_from_id(id: &str) -> String {
    let last = id.rsplit('.').next().unwrap_or(id);
    let mut characters = last.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => last.to_string(),
    }
}

/// Check an id: dotted, lower case, at least two parts.
///
/// The rule is the SDK's, and the SDK checks it too — but a contribution can
/// also be made at run time, and one made at run time has no compiler
/// behind it. So both check it, and both use the same words.
pub fn check_id(id: &LitStr) -> Result<Vec<String>> {
    let value = id.value();
    let parts: Vec<String> = value.split('.').map(|part| part.to_string()).collect();
    if parts.len() < 2 {
        return Err(Error::new(
            id.span(),
            format!("`{value}` is not a contribution id: an id is `<area>.<thing>`"),
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

/// Check a layer's id: one word, lower case.
///
/// A layer is not an area: it names a group of contributions, and the group
/// is not somewhere on a screen. So the id is a word — `core`, `geometry` —
/// and not `area.thing`.
pub fn check_layer_id(id: &LitStr) -> Result<()> {
    let value = id.value();
    if value.is_empty() {
        return Err(Error::new(id.span(), "a layer's id is a word, and it is empty"));
    }
    if !value.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err(Error::new(id.span(), "a layer's id is one word, in lower case"));
    }
    Ok(())
}

/// The error a missing field makes.
pub fn missing(what: &str) -> Error {
    Error::new(Span::call_site(), format!("a {what} needs this field"))
}
