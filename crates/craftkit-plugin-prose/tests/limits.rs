//! The **limits of the prose battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_prose::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A document with a **paragraph** in it, and the block's id.
fn a_paragraph(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.block.paragraph.add", &json!({ "document": document, "text": "a paragraph" }))
        .expect("the paragraph is added");
    outcome.data["id"].as_u64().expect("the paragraph has an id")
}

/// A field **code** over `MAX_CODE` long is refused.
///
/// The code of a field is an instruction, and an instruction of any
/// length at all would be a hole in the document.
#[test]
fn a_field_code_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let code = "c".repeat(4097);
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 0, "end": 1, "code": code }))
        .expect_err("that code is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **space** over `MAX_PARAGRAPH_SPACE` is refused.
///
/// The room above and below a paragraph is a number from zero to the
/// limit, and a number outside it is not a space.
#[test]
fn a_space_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "space_before": 10_001.0 }))
        .expect_err("that space is too big");
    assert!(error.to_string().contains("a number from zero to"), "{error}");
}

/// A **line spacing** over `MAX_LINE_SPACING` is refused.
#[test]
fn a_line_spacing_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "line_spacing": 101.0 }))
        .expect_err("that spacing is too big");
    assert!(error.to_string().contains("from zero to"), "{error}");
}

/// A bookmark **name** over `MAX_BOOKMARK_NAME` is refused.
#[test]
fn a_bookmark_name_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let name = "b".repeat(41);
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": block, "name": name }))
        .expect_err("that name is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **style** in a document, and its id.
fn a_style(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the style" }))
        .expect("the style is added");
    outcome.data["id"].as_u64().expect("the style has an id")
}

/// A **document**, and its id.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("prose.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **label** over `MAX_LABEL` long is refused.
#[test]
fn a_caption_label_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let label = "l".repeat(41);
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "label": label }))
        .expect_err("that label is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// The **text** of a caption over `MAX_CAPTION_TEXT` long is refused.
#[test]
fn a_caption_text_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let text = "t".repeat(1025);
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "text": text }))
        .expect_err("that text is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A page of **more columns** than `MAX_COLUMNS` allows is refused.
#[test]
fn a_page_with_too_many_columns_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 13.0 }))
        .expect_err("that page has too many columns");
    assert!(error.to_string().contains("columns"), "{error}");
}

/// A **space** between the columns over `MAX_COLUMN_SPACE` is refused.
#[test]
fn a_column_space_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.columns", &json!({ "document": document, "count": 2.0, "space": 301.0 }))
        .expect_err("that space is too big");
    assert!(error.to_string().contains("a space is from zero to"), "{error}");
}

/// An **author** of a revision over `MAX_REVISION_AUTHOR` long is refused.
#[test]
fn a_revision_author_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let author = "a".repeat(256);
    let runs = json!([{ "len": 1, "revision": { "kind": "insert", "author": author } }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that author is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **date** of a revision over `MAX_DATE` long is refused.
#[test]
fn a_revision_date_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let date = "d".repeat(65);
    let runs = json!([{ "len": 1, "revision": { "kind": "insert", "author": "someone", "date": date } }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that date is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A link **target** over `MAX_TARGET` long is refused.
#[test]
fn a_link_target_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let target = "https://example.com/".to_string() + &"a".repeat(2049);
    let runs = json!([{ "len": 1, "link": target }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that target is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// An equation with **more parts** than `MAX_PARTS` allows is refused.
///
/// Eleven levels of fractions would be twenty-three parts, so the tree
/// of parts is **wide** as well as deep: a level holds two halves, and
/// each half is a level of its own.
#[test]
fn an_equation_with_too_many_parts_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    // Ten levels of halves: two thousand and forty-seven parts.
    let mut parts = json!({ "kind": "text", "text": "x" });
    for _ in 0..10 {
        parts = json!({ "kind": "fraction", "top": parts, "bottom": parts });
    }
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that equation has too many parts");
    assert!(error.to_string().contains("parts"), "{error}");
}

/// An equation **deeper** than `MAX_DEPTH` is refused.
#[test]
fn an_equation_too_deep_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let mut parts = json!({ "kind": "text", "text": "x" });
    for _ in 0..34 {
        parts = json!({ "kind": "fraction", "top": parts, "bottom": { "kind": "text", "text": "y" } });
    }
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that equation is too deep");
    assert!(error.to_string().contains("deep"), "{error}");
}

/// The **text** of an equation over `MAX_EQUATION_TEXT` long is refused.
#[test]
fn an_equation_text_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let parts = json!({ "kind": "text", "text": "t".repeat(4097) });
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that text is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// An **endnote** over `MAX_ENDNOTE_TEXT` long is refused.
#[test]
fn an_endnote_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let text = "e".repeat(32_768);
    let error = session
        .execute("prose.endnote.add", &json!({ "document": document, "text": text }))
        .expect_err("that endnote is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A **footnote** over `MAX_FOOTNOTE_TEXT` long is refused.
#[test]
fn a_footnote_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let text = "f".repeat(32_768);
    let error = session
        .execute("prose.footnote.add", &json!({ "document": document, "text": text }))
        .expect_err("that footnote is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// An **authority** entry over `MAX_ENTRY` long is refused.
#[test]
fn an_authority_entry_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let entry = "e".repeat(201);
    let error = session
        .execute("prose.authority.mark", &json!({ "block": block, "entry": entry }))
        .expect_err("that entry is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// An **index** entry over `MAX_ENTRY` long is refused.
#[test]
fn an_index_entry_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let entry = "e".repeat(201);
    let error = session
        .execute("prose.index.mark", &json!({ "block": block, "entry": entry }))
        .expect_err("that entry is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A source's **tag** over `MAX_TAG` long is refused.
#[test]
fn a_source_tag_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let tag = "t".repeat(65);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "tag": tag }))
        .expect_err("that tag is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A source's **author** over `MAX_SOURCE_AUTHOR` long is refused.
#[test]
fn a_source_author_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let author = "a".repeat(257);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "author": author }))
        .expect_err("that author is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A source's **year** over `MAX_YEAR` long is refused.
#[test]
fn a_source_year_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let year = "y".repeat(33);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "year": year }))
        .expect_err("that year is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A source's **title** over `MAX_PART` long is refused.
///
/// The title is a part of a source, and it is bounded like every other
/// part: a source had a title of any length at all, and that was a hole
/// in the document.
#[test]
fn a_source_title_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let title = "t".repeat(1025);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "title": title }))
        .expect_err("that title is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A page's **margin** over `MAX_POINTS` is refused.
#[test]
fn a_page_margin_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute(
            "prose.page.set",
            &json!({ "document": document, "width": 595.0, "height": 842.0, "top": 20_001.0 }),
        )
        .expect_err("that margin is too big");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A custom property's **name** over `MAX_PROPERTY_NAME` long is refused.
#[test]
fn a_property_name_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let name = "n".repeat(256);
    let custom = json!({ name: "a value" });
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": custom }))
        .expect_err("that name is too long");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A custom property's **value** over `MAX_VALUE` long is refused.
#[test]
fn a_property_value_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let custom = json!({ "a name": "v".repeat(1024 * 1024 + 1) });
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": custom }))
        .expect_err("that value is too big");
    assert!(error.to_string().contains("at most"), "{error}");
}

/// A page's **start** over `MAX_START` is refused.
#[test]
fn a_page_start_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.numbers", &json!({ "document": document, "start": 100_001.0 }))
        .expect_err("that start is too big");
    assert!(error.to_string().contains("from one to"), "{error}");
}

/// A style's **level** over `MAX_LEVEL` is refused.
#[test]
fn a_style_level_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.level", &json!({ "style": style, "level": 10.0 }))
        .expect_err("that level is too deep");
    assert!(error.to_string().contains("level"), "{error}");
}

/// An **indent** over `MAX_INDENT` is refused.
#[test]
fn an_indent_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "indent_left": 10_001.0 }))
        .expect_err("that indent is too big");
    assert!(error.to_string().contains("a number from zero to"), "{error}");
}

/// A **level** over `MAX_LEVELS` is refused.
///
/// A list has nine levels, and a block at the tenth is in no level the
/// list has.
#[test]
fn a_list_level_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.list.add", &json!({ "document": document, "kind": "numbered" }))
        .expect("the list is added");
    let list = outcome.data["id"].as_u64().expect("the list has an id");
    let block = a_paragraph(&mut session);
    let error = session
        .execute("prose.block.numbering", &json!({ "block": block, "list": list, "level": 9.0 }))
        .expect_err("that level is too deep");
    assert!(error.to_string().contains("levels"), "{error}");
}

/// A **chain** of styles stops at `MAX_CHAIN`.
///
/// The chain of a style is walked from the style to its base, and the
/// walk is bounded: a chain of forty styles is cut at the limit, and the
/// base beyond it is not inherited. The chain a reader sees is the
/// bases the walk reached and the style itself.
#[test]
fn a_chain_of_styles_stops_at_the_limit() {
    let mut session = session();
    let document = a_document(&mut session);
    let outcome = session
        .execute("prose.style.add", &json!({ "document": document, "name": "the base" }))
        .expect("the base style is added");
    let base = outcome.data["id"].as_u64().expect("the style has an id");
    let mut style = base;
    for index in 1..40 {
        let outcome = session
            .execute(
                "prose.style.add",
                &json!({ "document": document, "name": format!("the style {index}"), "based_on": style }),
            )
            .expect("the style is added");
        style = outcome.data["id"].as_u64().expect("the style has an id");
    }
    let outcome = session
        .execute("prose.style.character.read", &json!({ "style": style }))
        .expect("the character is read");
    let chain = outcome.data["chain"].as_array().expect("the chain is a list");
    assert_eq!(chain.len(), 33, "the chain stops at the limit: {chain:?}");
    assert!(!chain.iter().any(|id| id.as_u64() == Some(base)), "the walk was cut before the base: {chain:?}");
}
