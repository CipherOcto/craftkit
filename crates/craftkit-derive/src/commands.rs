//! A **family** of commands, declared in one table.
//!
//! # What this is
//!
//! [`crate::commands`] takes a table of commands and writes, for every row,
//! the same work [`crate::derive_command`] writes for one command: the
//! struct, the reader, the registration and the spec. The difference is
//! that the table is **one declaration** rather than one declaration per
//! command, so a battery that is a family reads as a family.
//!
//! ```ignore
//! commands! {
//!     /// Add a workbook to the grid.
//!     grid.workbook.add {
//!         label: "Add workbook",
//!         area: "grid",
//!         mutates,
//!         undoable,
//!         params: {
//!             /// The workbook's name.
//!             name: Text,
//!         },
//!         run: {
//!             Ok(Outcome::changed(format!("added {}", self.name.0)))
//!         }
//!     }
//! }
//! ```
//!
//! The table's shape:
//!
//! | in the table | what it is |
//! |---|---|
//! | the doc comment | the command's one line, and the spec's `doc` |
//! | the id | `<area>.<thing>`, dotted and lower case |
//! | `label` | what the button says; the id's last part, if it is not given |
//! | `area` | the group it is listed under; the id's first part, if it is not given |
//! | `mutates`, `undoable` | whether it changes the document, and whether it is undone |
//! | `params` | the fields, and their types: the types are the parameters' kinds |
//! | `run` | the body: `self` is the command, `session` is the session |
//!
//! # What it does **not** do, and why
//!
//! **No battery is converted to use it.** That is the next arc, and it is a
//! separate decision: the house values a rule you can *read*, and a
//! declaration table trades that for the speed of a port. The table is here
//! so that the choice can be made with the machinery in hand — and it is
//! here so that the choice can be made by **reading both**: a battery that
//! is a table and a battery that is a struct are both readable, and only
//! one of them is a rule.
//!
//! The table covers the **shape** of a command — its id, its label, its
//! area, its parameters and its body — and not the whole of the derive's
//! vocabulary: `capability`, `layer`, `icon`, `menu` and `shortcut` are not
//! in the table yet, and a row that needs one is a row that wants the
//! derive.
//!
//! So this module writes the *mechanical* half — the struct, the reader,
//! the registration — and leaves the *semantic* half — the body of the
//! command — as the reader's own Rust. A rule that needs reading stays
//! readable, and a rule that does not becomes a row.
//!
//! # The checks it inherits, and the one it adds
//!
//! Every row goes through the same expansion as a derived command, so it
//! inherits the compiler's checks: the id is dotted and lower case, the id
//! is in its area, a parameter that is an `Either` needs its other spelling,
//! and a command that is `undoable` must `mutate`.
//!
//! The one check that is new is **the table's own**: two rows may not have
//! one id. The registry would refuse such a pair when it is built, and this
//! says so while the reader is still looking at the table.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Block, Error, Ident, LitStr, Result, Token, Type, Visibility, braced};

use crate::common::sdk_root;

/// One command of the table.
struct Row {
    /// The row's documentation: the command's one line.
    docs: Vec<Attribute>,
    /// The command's id.
    id: String,
    /// What the button says.
    label: Option<String>,
    /// The group it is listed under.
    area: Option<String>,
    /// The layer it belongs to.
    layer: Option<String>,
    /// The icon's name.
    icon: Option<String>,
    /// The chord it is advertised with.
    shortcut: Option<String>,
    /// What the command is for.
    capability: Option<String>,
    /// Where the command goes in the menu bar: a path.
    menu: Vec<String>,
    /// The predicate that says whether the command is enabled.
    enabled: Option<syn::Path>,
    /// Whether it changes the document.
    mutates: bool,
    /// Whether it is undone.
    undoable: bool,
    /// Whether a successful run is written in the journal.
    no_journal: bool,
    /// Whether the id's first part is dropped from the area.
    no_prefix: bool,
    /// Whether the battery marks this command as its example.
    example: bool,
    /// Its parameters: the fields, and their types.
    params: Vec<Field>,
    /// The command's body.
    run: Block,
    /// Whether the table gave the row a body.
    has_run: bool,
}

/// One parameter of a command.
struct Field {
    /// The parameter's documentation.
    docs: Vec<Attribute>,
    /// The parameter's name.
    name: Ident,
    /// The parameter's kind: the type says what it is.
    ty: Type,
}

/// The whole table.
struct Table {
    rows: Vec<Row>,
}

impl Parse for Table {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut rows = Vec::new();
        while !input.is_empty() {
            rows.push(input.parse()?);
        }
        Ok(Table { rows })
    }
}

impl Parse for Row {
    fn parse(input: ParseStream) -> Result<Self> {
        let docs = docs_of(input)?;
        let id = id_of(input)?;
        let mut row = Row {
            docs,
            id,
            label: None,
            area: None,
            layer: None,
            icon: None,
            shortcut: None,
            capability: None,
            menu: Vec::new(),
            enabled: None,
            mutates: false,
            undoable: false,
            no_journal: false,
            no_prefix: false,
            example: false,
            params: Vec::new(),
            run: Block { brace_token: Default::default(), stmts: Vec::new() },
            has_run: false,
        };

        let body;
        braced!(body in input);
        while !body.is_empty() {
            let key: Ident = body.parse()?;
            let name = key.to_string();
            if body.peek(Token![:]) {
                body.parse::<Token![:]>()?;
                match name.as_str() {
                    "label" => {
                        let value: LitStr = body.parse()?;
                        row.label = Some(value.value());
                    }
                    "area" => {
                        let value: LitStr = body.parse()?;
                        row.area = Some(value.value());
                    }
                    "layer" => {
                        let value: LitStr = body.parse()?;
                        row.layer = Some(value.value());
                    }
                    "icon" => {
                        let value: LitStr = body.parse()?;
                        row.icon = Some(value.value());
                    }
                    "shortcut" => {
                        let value: LitStr = body.parse()?;
                        row.shortcut = Some(value.value());
                    }
                    "capability" => {
                        let value: LitStr = body.parse()?;
                        row.capability = Some(value.value());
                    }
                    "menu" => {
                        row.menu = menu_of(&body)?;
                    }
                    "enabled" => {
                        row.enabled = Some(body.parse()?);
                    }
                    "params" => {
                        let fields;
                        braced!(fields in body);
                        row.params = params_of(&fields)?;
                    }
                    "run" => {
                        row.run = body.parse()?;
                        row.has_run = true;
                    }
                    other => {
                        return Err(Error::new(
                            key.span(),
                            format!(
                                "`{other}` is not an entry of a command: a command in a table has \
                                 `label`, `area`, `layer`, `icon`, `shortcut`, `capability`, \
                                 `menu`, `enabled`, `params` and `run`"
                            ),
                        ));
                    }
                }
            } else {
                match name.as_str() {
                    "mutates" => row.mutates = true,
                    "undoable" => row.undoable = true,
                    "no_journal" => row.no_journal = true,
                    "no_prefix" => row.no_prefix = true,
                    "example" => row.example = true,
                    other => {
                        return Err(Error::new(
                            key.span(),
                            format!(
                                "`{other}` is not a flag of a command: the flags are `mutates`, \
                                 `undoable`, `no_journal`, `no_prefix` and `example`"
                            ),
                        ));
                    }
                }
            }
            if body.is_empty() {
                break;
            }
            body.parse::<Token![,]>()?;
        }
        Ok(row)
    }
}

/// The doc comment of a row or of a parameter.
///
/// Only documentation is read: a row in a table is a declaration, and an
/// attribute that says something else has no place here.
fn docs_of(input: ParseStream) -> Result<Vec<Attribute>> {
    let mut docs = Vec::new();
    for attribute in Attribute::parse_outer(input)? {
        if !attribute.path().is_ident("doc") {
            return Err(Error::new(
                attribute.span(),
                "a command in a table is documented with a doc comment, and with nothing else",
            ));
        }
        docs.push(attribute);
    }
    Ok(docs)
}

/// A dotted id: `<area>.<thing>`.
fn id_of(input: ParseStream) -> Result<String> {
    let mut id = String::new();
    let first: Ident = input.parse()?;
    id.push_str(&first.to_string());
    while input.peek(Token![.]) {
        input.parse::<Token![.]>()?;
        let part: Ident = input.parse()?;
        id.push('.');
        id.push_str(&part.to_string());
    }
    Ok(id)
}

/// The menu of a command: the path from the bar down to it.
///
/// A tuple in the table — `menu: ("Path", "Points")` — and a call in the
/// attribute. The table's shape is the tree's own: a row says what a
/// thing is, and the attribute says what it does.
fn menu_of(input: ParseStream) -> Result<Vec<String>> {
    let content;
    syn::parenthesized!(content in input);
    let mut out = Vec::new();
    while !content.is_empty() {
        out.push(content.parse::<LitStr>()?.value());
        if content.is_empty() {
            break;
        }
        content.parse::<Token![,]>()?;
    }
    Ok(out)
}

/// The parameters of a command: the fields, and their types.
fn params_of(input: ParseStream) -> Result<Vec<Field>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let docs = docs_of(input)?;
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        out.push(Field { docs, name, ty });
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(out)
}

/// The struct's name, from the id.
///
/// `grid.workbook.add` is `GridWorkbookAdd`: the id's parts, each one
/// capitalised, and the parts joined. The name is the id's own shape, so a
/// reader can find the struct from the id without being told.
fn struct_name(id: &str) -> Ident {
    let mut out = String::new();
    for part in id.split('.') {
        let mut characters = part.chars();
        if let Some(first) = characters.next() {
            out.push(first.to_ascii_uppercase());
            out.extend(characters);
        }
    }
    Ident::new(&out, proc_macro2::Span::call_site())
}

/// Write the table.
pub fn expand(input: TokenStream2) -> Result<TokenStream2> {
    let table: Table = syn::parse2(input)?;
    if table.rows.is_empty() {
        return Err(Error::new(proc_macro2::Span::call_site(), "a table of commands is not empty"));
    }

    // Two rows may not have one id: the registry would refuse the pair when
    // it is built, and this says so while the reader is looking at the
    // table.
    let mut seen: Vec<&str> = Vec::new();
    for row in &table.rows {
        if seen.contains(&row.id.as_str()) {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                format!("`{}` is declared twice in this table", row.id),
            ));
        }
        seen.push(row.id.as_str());
    }

    let sdk = sdk_root();
    let mut out = TokenStream2::new();
    for row in &table.rows {
        out.extend(expand_row(&sdk, row)?);
    }
    Ok(out)
}

/// Write one row: the struct, the body, and everything the derive writes.
fn expand_row(sdk: &TokenStream2, row: &Row) -> Result<TokenStream2> {
    if !row.has_run {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("`{}` has no `run`: a command in a table says what it does", row.id),
        ));
    }
    let name = struct_name(&row.id);

    // The command's own attributes, and the struct's declaration.
    let id = LitStr::new(&row.id, proc_macro2::Span::call_site());
    let mut arguments = vec![quote!(id = #id)];
    if let Some(label) = &row.label {
        let label = LitStr::new(label, proc_macro2::Span::call_site());
        arguments.push(quote!(label = #label));
    }
    if let Some(area) = &row.area {
        let area = LitStr::new(area, proc_macro2::Span::call_site());
        arguments.push(quote!(area = #area));
    }
    if let Some(layer) = &row.layer {
        let layer = LitStr::new(layer, proc_macro2::Span::call_site());
        arguments.push(quote!(layer = #layer));
    }
    if let Some(icon) = &row.icon {
        let icon = LitStr::new(icon, proc_macro2::Span::call_site());
        arguments.push(quote!(icon = #icon));
    }
    if let Some(shortcut) = &row.shortcut {
        let shortcut = LitStr::new(shortcut, proc_macro2::Span::call_site());
        arguments.push(quote!(shortcut = #shortcut));
    }
    if let Some(capability) = &row.capability {
        let capability = LitStr::new(capability, proc_macro2::Span::call_site());
        arguments.push(quote!(capability = #capability));
    }
    if !row.menu.is_empty() {
        let menu: Vec<LitStr> =
            row.menu.iter().map(|name| LitStr::new(name, proc_macro2::Span::call_site())).collect();
        arguments.push(quote!(menu(#(#menu),*)));
    }
    if let Some(enabled) = &row.enabled {
        arguments.push(quote!(enabled = #enabled));
    }
    if row.mutates {
        arguments.push(quote!(mutates));
    }
    if row.undoable {
        arguments.push(quote!(undoable));
    }
    if row.no_journal {
        arguments.push(quote!(no_journal));
    }
    if row.no_prefix {
        arguments.push(quote!(no_prefix));
    }
    if row.example {
        arguments.push(quote!(example));
    }
    let attribute: Attribute = syn::parse_quote!(#[command(#(#arguments),*)]);

    let mut attrs = row.docs.clone();
    attrs.push(attribute);

    let fields: Vec<syn::Field> = row
        .params
        .iter()
        .map(|field| syn::Field {
            attrs: field.docs.clone(),
            vis: Visibility::Inherited,
            ident: Some(field.name.clone()),
            colon_token: Default::default(),
            ty: field.ty.clone(),
            mutability: syn::FieldMutability::None,
        })
        .collect();

    let declaration = syn::DeriveInput {
        attrs,
        vis: Visibility::Public(<Token![pub]>::default()),
        ident: name.clone(),
        generics: Default::default(),
        data: syn::Data::Struct(syn::DataStruct {
            struct_token: Default::default(),
            fields: syn::Fields::Named(syn::FieldsNamed {
                brace_token: Default::default(),
                named: fields.into_iter().collect(),
            }),
            semi_token: None,
        }),
    };

    // The struct itself, and the body of the command. The declaration above
    // is what the derive reads; the struct and the body are written here.
    let declarations: Vec<TokenStream2> = row
        .params
        .iter()
        .map(|field| {
            let docs = &field.docs;
            let name = &field.name;
            let ty = &field.ty;
            quote!(#(#docs)* #name: #ty)
        })
        .collect();
    let run = &row.run;
    let docs = &row.docs;

    // The reader, the registration and the spec: the same expansion the
    // derive writes for one command.
    let expansion = crate::command::expand(declaration)?;

    Ok(quote! {
        #(#docs)*
        pub struct #name {
            #(#declarations),*
        }

        impl #name {
            /// The command's body.
            ///
            /// Written by the arc: the macro writes the edges, and the
            /// middle is the programmer's.
            fn run(&self, session: &mut #sdk::Session) -> Result<#sdk::Outcome, #sdk::EngineError> {
                // The body of a row may not use the session, and the
                // session is named here: a borrow says that it is not
                // needed, and the warning that would follow it is not a
                // warning about the row.
                let _ = &session;
                #run
            }
        }

        #expansion
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Expand a table, and take the message of the error it earns.
    fn refuse(source: &str) -> String {
        match expand(source.parse().expect("the table parses")) {
            Ok(_) => panic!("the table was accepted: {source}"),
            Err(error) => error.to_string(),
        }
    }

    /// Expand a table, and take the tokens it produced.
    fn accept(source: &str) -> String {
        match expand(source.parse().expect("the table parses")) {
            Ok(tokens) => tokens.to_string(),
            Err(error) => panic!("the table was refused: {error}"),
        }
    }

    const GOOD: &str = r#"
/// Add a workbook.
grid.workbook.add {
    label: "Add workbook",
    area: "grid",
    mutates,
    undoable,
    params: {
        /// The workbook's name.
        name: Text,
    },
    run: {
        Ok(Outcome::changed("added"))
    }
}
"#;

    #[test]
    fn a_good_table_is_accepted() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("grid.workbook.add"), "{tokens}");
        assert!(tokens.contains("Add workbook"), "{tokens}");
        assert!(tokens.contains("GridWorkbookAdd"), "{tokens}");
    }

    #[test]
    fn the_table_writes_the_reader_and_the_registration() {
        let tokens = accept(GOOD);
        assert!(tokens.contains("Registration"), "{tokens}");
        assert!(tokens.contains("CommandSpec"), "{tokens}");
        assert!(tokens.contains("fn read"), "{tokens}");
    }

    #[test]
    fn two_rows_with_one_id_are_refused() {
        let message = refuse(&format!("{GOOD}\n{GOOD}"));
        assert!(message.contains("declared twice"), "{message}");
    }

    #[test]
    fn a_row_without_a_body_is_refused() {
        let message = refuse(
            r#"
grid.workbook.add {
    label: "Add workbook"
}
"#,
        );
        assert!(message.contains("has no `run`"), "{message}");
    }

    #[test]
    fn an_entry_that_is_not_an_entry_is_refused() {
        let message = refuse(
            r#"
grid.workbook.add {
    colour: "red",
    run: {
        Ok(Outcome::new("added"))
    }
}
"#,
        );
        assert!(message.contains("is not an entry of a command"), "{message}");
    }

    #[test]
    fn a_flag_that_is_not_a_flag_is_refused() {
        let message = refuse(
            r#"
grid.workbook.add {
    loudly,
    run: {
        Ok(Outcome::new("added"))
    }
}
"#,
        );
        assert!(message.contains("is not a flag of a command"), "{message}");
    }

    #[test]
    fn an_id_that_is_not_dotted_is_refused() {
        let message = refuse(&GOOD.replace("grid.workbook.add", "add"));
        assert!(message.contains("<area>.<thing>"), "{message}");
    }

    #[test]
    fn an_id_that_is_not_in_its_area_is_refused() {
        let message = refuse(&GOOD.replace("area: \"grid\"", "area: \"doc\""));
        assert!(message.contains("is not in the `doc` area"), "{message}");
    }
}
