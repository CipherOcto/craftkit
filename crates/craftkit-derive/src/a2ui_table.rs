//! A **family** of components, declared in one table.
//!
//! # What this is
//!
//! [`crate::components`] takes a table of components and writes, for every
//! row, the same work [`crate::derive_a2ui_component`] writes for one
//! component: the struct, the reader, the declarations, the published
//! schema, the typed view of a payload, the kind, the catalogue entry and
//! the two registrations.
//!
//! ```ignore
//! components!(
//!     table = "craftkit_a2ui::defs",
//!     shapes = "craftkit_a2ui::schema"
//! ) {
//!     /// A line of text.
//!     Text {
//!         kind: "a2ui.text",
//!         props: {
//!             /// The text to show.
//!             #[a2ui_prop(DynamicString)]
//!             text: Text,
//!             /// A hint for the base text size.
//!             variant: Option<Text>,
//!         },
//!         paint: {
//!             let Some(props) = TextComponent::of(node) else { return };
//!             let Some(rect) = node.rect else { return };
//!             canvas.text(Point::new(rect.x, rect.y), &props.text.0, Colour::BLACK, 10.0);
//!         },
//!         hit: {
//!             node.rect.map(|rect| rect.contains(point)).unwrap_or(false)
//!         },
//!     }
//! }
//! ```
//!
//! # The boundary
//!
//! **The painter stays hand-written.** It is ordinary craftkit code, and the
//! names in it are the tree's own — `node.rect`, `canvas.text`,
//! `Rect::contains`, `Point::new`, `Colour::BLACK`. The table writes the
//! **edges** and the body is the programmer's, exactly as `commands!` does
//! with `run:`.
//!
//! # What the table adds to the derive's own checks
//!
//! Two rows may not have one **a2ui name**, and two rows may not have one
//! **kind id**: the registry would refuse such a pair when it is built, and
//! the table says so while the reader is still looking at it.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Block, Error, Ident, LitStr, Result, Token, Type, Visibility, braced};

use crate::a2ui_component::{expand_component, props_of};
use crate::common::{check_id_text, sdk_root};

/// One component of the table.
struct Row {
    /// The row's documentation: the component's one line.
    docs: Vec<Attribute>,
    /// The **a2ui** name: `Text`. It is the key in the catalogue.
    name: String,
    /// The craftkit **kind** id: `a2ui.text`.
    kind: Option<String>,
    /// The layer the component belongs to.
    layer: Option<String>,
    /// The props: the fields, and their types.
    props: Vec<Field>,
    /// How it is drawn.
    paint: Option<Block>,
    /// Whether a point is inside it.
    hit: Option<Block>,
}

/// One prop of a row.
struct Field {
    /// The prop's documentation.
    docs: Vec<Attribute>,
    /// The prop's own attributes: `#[a2ui_prop(…)]`.
    attrs: Vec<Attribute>,
    /// The prop's name.
    name: Ident,
    /// The prop's type.
    ty: Type,
}

/// The whole table.
struct Table {
    /// The path of the layer's **table** — the property names.
    table: Option<LitStr>,
    /// The path of the layer's **shapes** — the re-derived schema text.
    shapes: Option<LitStr>,
    /// The rows.
    rows: Vec<Row>,
}

impl Parse for Table {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut table = Table { table: None, shapes: None, rows: Vec::new() };
        // The header: `table = "…", shapes = "…"`. It is optional — a table
        // without one names no layer, and the rows say so themselves.
        while !input.is_empty() {
            if input.peek(syn::Ident) && (input.peek2(Token![=]) || input.peek2(syn::token::Paren)) {
                let key: Ident = input.parse()?;
                let name = key.to_string();
                if !input.peek(Token![=]) {
                    return Err(Error::new(
                        key.span(),
                        "a table's header names a path: `table = \"…\"` and `shapes = \"…\"`",
                    ));
                }
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;
                match name.as_str() {
                    "table" => table.table = Some(value),
                    "shapes" => table.shapes = Some(value),
                    other => {
                        return Err(Error::new(
                            key.span(),
                            format!(
                                "`{other}` is not an entry of a table's header: a table has `table` \
                                 and `shapes`"
                            ),
                        ));
                    }
                }
                if input.is_empty() {
                    break;
                }
                input.parse::<Token![,]>()?;
                continue;
            }
            break;
        }
        // The rows, in their own braces — so that the table reads as one
        // item, and the header reads as its own part.
        if input.peek(syn::token::Brace) {
            let rows;
            braced!(rows in input);
            while !rows.is_empty() {
                table.rows.push(rows.parse()?);
            }
        } else {
            while !input.is_empty() {
                table.rows.push(input.parse()?);
            }
        }
        Ok(table)
    }
}

impl Parse for Row {
    fn parse(input: ParseStream) -> Result<Self> {
        let docs = docs_of(input)?;
        let name = name_of(input)?;
        let mut row = Row { docs, name, kind: None, layer: None, props: Vec::new(), paint: None, hit: None };

        let body;
        braced!(body in input);
        while !body.is_empty() {
            let key: Ident = body.parse()?;
            let key_name = key.to_string();
            if body.peek(Token![:]) {
                body.parse::<Token![:]>()?;
                match key_name.as_str() {
                    "kind" => {
                        let value: LitStr = body.parse()?;
                        row.kind = Some(value.value());
                    }
                    "layer" => {
                        let value: LitStr = body.parse()?;
                        row.layer = Some(value.value());
                    }
                    "props" => {
                        let fields;
                        braced!(fields in body);
                        row.props = fields_of(&fields)?;
                    }
                    "paint" => {
                        row.paint = Some(body.parse()?);
                    }
                    "hit" => {
                        row.hit = Some(body.parse()?);
                    }
                    other => {
                        return Err(Error::new(
                            key.span(),
                            format!(
                                "`{other}` is not an entry of a component: a component in a table has \
                                 `kind`, `layer`, `props`, `paint` and `hit`"
                            ),
                        ));
                    }
                }
            } else {
                return Err(Error::new(
                    key.span(),
                    format!(
                        "`{key_name}` is not an entry of a component: a component in a table has \
                         `kind`, `layer`, `props`, `paint` and `hit`"
                    ),
                ));
            }
            if body.is_empty() {
                break;
            }
            body.parse::<Token![,]>()?;
        }
        Ok(row)
    }
}

/// The doc comment of a row or of a prop.
///
/// Only documentation is read: a row in a table is a declaration, and an
/// attribute that says something else has no place here.
fn docs_of(input: ParseStream) -> Result<Vec<Attribute>> {
    let mut docs = Vec::new();
    for attribute in Attribute::parse_outer(input)? {
        if !attribute.path().is_ident("doc") {
            return Err(Error::new(
                attribute.span(),
                "a component in a table is documented with a doc comment, and with nothing else",
            ));
        }
        docs.push(attribute);
    }
    Ok(docs)
}

/// The **a2ui** name of a row.
///
/// It is PascalCase, and it is **not** a Rust path: `TextField` is a name
/// in a catalogue, and the table reads it as text.
fn name_of(input: ParseStream) -> Result<String> {
    let ident = Ident::parse_any(input)?;
    Ok(ident.to_string())
}

/// The props of a row: the fields, and their types.
///
/// A prop may carry the derive's own attributes — `#[a2ui_prop(…)]`, the
/// a2ui definition it uses — and the table carries them, so a prop that
/// says something about itself is not a reason to leave the row behind.
fn fields_of(input: ParseStream) -> Result<Vec<Field>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let mut docs = Vec::new();
        let mut attrs = Vec::new();
        for attribute in Attribute::parse_outer(input)? {
            if attribute.path().is_ident("doc") {
                docs.push(attribute);
            } else {
                attrs.push(attribute);
            }
        }
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty: Type = input.parse()?;
        out.push(Field { docs, attrs, name, ty });
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(out)
}

/// The struct's name, from the a2ui name.
///
/// `Text` is `TextComponent`: the name, and `Component` after it.
fn struct_name(name: &str) -> Ident {
    Ident::new(&format!("{name}Component"), Span::call_site())
}

/// Write the table.
pub fn expand(input: TokenStream2) -> Result<TokenStream2> {
    let table: Table = syn::parse2(input)?;
    if table.rows.is_empty() {
        return Err(Error::new(Span::call_site(), "a table of components is not empty"));
    }

    // Two rows may not have one name, and two may not have one kind: the
    // registry would refuse such a pair when it is built, and this says so
    // while the reader is still looking at the table.
    let mut names: Vec<&str> = Vec::new();
    let mut kinds: Vec<&str> = Vec::new();
    for row in &table.rows {
        if names.contains(&row.name.as_str()) {
            return Err(Error::new(
                Span::call_site(),
                format!("`{}` is declared twice in this table", row.name),
            ));
        }
        names.push(row.name.as_str());
        if let Some(kind) = &row.kind {
            if kinds.contains(&kind.as_str()) {
                return Err(Error::new(
                    Span::call_site(),
                    format!("the kind `{kind}` is declared twice in this table"),
                ));
            }
            kinds.push(kind.as_str());
        }
    }

    let sdk = sdk_root();
    let mut out = TokenStream2::new();
    for row in &table.rows {
        out.extend(expand_row(&sdk, &table, row)?);
    }
    Ok(out)
}

/// Write one row: the struct, the bodies, and everything the derive writes.
fn expand_row(sdk: &TokenStream2, table: &Table, row: &Row) -> Result<TokenStream2> {
    let name = struct_name(&row.name);

    // The props, as the derive reads them: the fields, with their
    // attributes, and the doc comments.
    let fields: Vec<syn::Field> = row
        .props
        .iter()
        .map(|field| syn::Field {
            attrs: field.docs.clone().into_iter().chain(field.attrs.clone()).collect(),
            vis: Visibility::Inherited,
            ident: Some(field.name.clone()),
            colon_token: Default::default(),
            ty: field.ty.clone(),
            mutability: syn::FieldMutability::None,
        })
        .collect();
    let named = syn::FieldsNamed { brace_token: Default::default(), named: fields.into_iter().collect() };
    let props = props_of(&named)?;

    // The layer's paths: the table's header, or the row's own.
    let table_path = table.table.clone().ok_or_else(|| {
        Error::new(Span::call_site(), "a table of components needs the layer's table: `table = \"…::defs\"`")
    })?;
    let shapes_path = table.shapes.clone().ok_or_else(|| {
        Error::new(
            Span::call_site(),
            "a table of components needs the layer's shapes: `shapes = \"…::schema\"`",
        )
    })?;

    // The **layer** comes first: the kind's prefix is the layer's own
    // id, and not a name the table hard-codes.
    let layer = row.layer.clone().unwrap_or_else(|| "a2ui".to_string());
    // And the **kind** is checked: the registry refuses an id that is not
    // dotted and lower case, and the table says so while the reader is
    // still looking at it.
    let kind = match &row.kind {
        Some(kind) => {
            check_id_text(kind, Span::call_site())?;
            kind.clone()
        }
        None => format!("{layer}.{}", row.name.to_lowercase()),
    };
    let doc = crate::common::doc_of(&row.docs);

    // The **bodies**: the painter and the hit test, hand-written.
    let paint = row.paint.clone().ok_or_else(|| {
        Error::new(
            Span::call_site(),
            format!("`{}` has no `paint`: a component says how it is drawn", row.name),
        )
    })?;
    let hit = row.hit.clone().ok_or_else(|| {
        Error::new(
            Span::call_site(),
            format!("`{}` has no `hit`: a component says what is inside it", row.name),
        )
    })?;

    // The declarations of the struct, with the props' own attributes.
    let declarations: Vec<TokenStream2> = row
        .props
        .iter()
        .map(|field| {
            let docs = &field.docs;
            let name = &field.name;
            let ty = &field.ty;
            quote!(#(#docs)* pub #name: #ty)
        })
        .collect();
    let docs = &row.docs;

    let expansion = expand_component(
        sdk,
        &crate::a2ui_component::Row {
            struct_name: name.clone(),
            name: row.name.clone(),
            kind,
            layer,
            table: table_path,
            shapes: shapes_path,
            props,
            doc,
        },
    )?;

    Ok(quote! {
        #(#docs)*
        pub struct #name {
            #(#declarations),*
        }

        impl #name {
            /// How it is drawn.
            ///
            /// Hand-written: the macro writes the edges, and the middle is
            /// the programmer's.
            fn paint(node: &#sdk::Node, document: &#sdk::Document, canvas: &mut dyn #sdk::Canvas) {
                #paint
            }

            /// Whether a point is inside it.
            fn hit(node: &#sdk::Node, document: &#sdk::Document, point: #sdk::Point) -> bool {
                #hit
            }
        }

        #expansion
    })
}

#[cfg(test)]
mod tests {

    /// The **table** macro writes the same work for every row.
    #[test]
    fn the_table_writes_the_same_work_for_every_row() {
        let tokens = crate::a2ui_table::expand(
            r#"
        table = "craftkit_a2ui::defs",
        shapes = "craftkit_a2ui::schema",
    {
        /// A line of text.
        Text {
            kind: "a2ui.text",
            props: {
                /// The text to show.
                #[a2ui_prop(DynamicString)]
                text: Text,
            },
            paint: {
                let _ = node;
            },
            hit: {
                false
            },
        }
    }
"#
            .parse()
            .expect("the table parses"),
        )
        .expect("the table is accepted")
        .to_string();
        assert!(tokens.contains("TextComponent"), "{tokens}");
        assert!(tokens.contains("TEXT_COMPONENT"), "{tokens}");
        assert!(tokens.contains("TEXT_CATALOG"), "{tokens}");
        assert!(tokens.contains("fn paint"), "{tokens}");
        assert!(tokens.contains("fn hit"), "{tokens}");
        assert!(tokens.contains("craftkit_a2ui :: defs"), "{tokens}");
        assert!(tokens.contains("craftkit_a2ui :: schema"), "{tokens}");
    }

    /// A row whose **kind** is not an id is refused: the registry would
    /// refuse it when it is built, and the table says so while the
    /// reader is still looking at it.
    #[test]
    fn a_row_with_a_kind_that_is_not_an_id_is_refused() {
        let error = crate::a2ui_table::expand(
            r#"
        table = "a::defs",
        shapes = "a::schema",
    {
        /// A line of text.
        Text {
            kind: "Text",
            props: {},
            paint: {},
            hit: {},
        }
    }
"#
            .parse()
            .expect("the table parses"),
        )
        .expect_err("the table is refused");
        assert!(error.to_string().contains("contribution id"), "{error}");
    }

    /// Two rows with one **name** are refused.
    #[test]
    fn two_rows_with_one_name_are_refused() {
        let error = crate::a2ui_table::expand(
            r#"
        table = "a::defs",
        shapes = "a::schema",
    {
        /// A line of text.
        Text {
            kind: "a2ui.text",
            props: {},
            paint: {},
            hit: {},
        }
        /// A line of text, again.
        Text {
            kind: "a2ui.text2",
            props: {},
            paint: {},
            hit: {},
        }
    }
"#
            .parse()
            .expect("the table parses"),
        )
        .expect_err("the table is refused");
        assert!(error.to_string().contains("declared twice"), "{error}");
    }

    /// A row with **no painter** is refused.
    #[test]
    fn a_row_with_no_painter_is_refused() {
        let error = crate::a2ui_table::expand(
            r#"
        table = "a::defs",
        shapes = "a::schema",
    {
        /// A line of text.
        Text {
            kind: "a2ui.text",
            props: {},
            hit: {},
        }
    }
"#
            .parse()
            .expect("the table parses"),
        )
        .expect_err("the table is refused");
        assert!(error.to_string().contains("no `paint`"), "{error}");
    }

    /// A row with **no hit test** is refused.
    #[test]
    fn a_row_with_no_hit_test_is_refused() {
        let error = crate::a2ui_table::expand(
            r#"
        table = "a::defs",
        shapes = "a::schema",
    {
        /// A line of text.
        Text {
            kind: "a2ui.text",
            props: {},
            paint: {},
        }
    }
"#
            .parse()
            .expect("the table parses"),
        )
        .expect_err("the table is refused");
        assert!(error.to_string().contains("no `hit`"), "{error}");
    }
}
