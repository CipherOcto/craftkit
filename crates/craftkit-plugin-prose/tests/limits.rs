//! The **limits of the prose battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.
//!
//! Every cap of the battery is **named in code** here: the test says
//! the name of the cap and uses its number, so a cap that changes is a
//! cap that changes the test and not only the prose.

use craftkit_plugin_prose::{
    MAX_AUTHORITY_ENTRY, MAX_BOOKMARK_NAME, MAX_CAPTION_LABEL, MAX_CAPTION_TEXT, MAX_CODE, MAX_COLUMN_SPACE,
    MAX_CONTENT_LEVEL, MAX_DATE, MAX_ENDNOTE_TEXT, MAX_EQUATION_DEPTH, MAX_EQUATION_TEXT, MAX_FIGURE_LABEL,
    MAX_FOOTNOTE_TEXT, MAX_INDENT, MAX_INDEX_ENTRY, MAX_LEVELS, MAX_LINE_SPACING, MAX_LINK_TARGET,
    MAX_OUTLINE_LEVEL, MAX_PAGE_COLUMNS, MAX_PARAGRAPH_SPACE, MAX_PARTS, MAX_POINTS, MAX_PROPERTY_NAME,
    MAX_REVISION_AUTHOR, MAX_SOURCE_AUTHOR, MAX_SOURCE_PART, MAX_START, MAX_STYLE_CHAIN, MAX_TAG, MAX_VALUE,
    MAX_YEAR,
};
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
    let code = "c".repeat(MAX_CODE + 1);
    let error = session
        .execute("prose.block.field.add", &json!({ "block": block, "start": 0, "end": 1, "code": code }))
        .expect_err("that code is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_CODE.to_string()), "the refusal names the bound: {message}");
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
        .execute(
            "prose.style.paragraph",
            &json!({ "style": style, "space_before": MAX_PARAGRAPH_SPACE + 1.0 }),
        )
        .expect_err("that space is too big");
    let message = error.to_string();
    assert!(message.contains("a number from zero to"), "{message}");
    assert!(message.contains(&MAX_PARAGRAPH_SPACE.to_string()), "the refusal names the bound: {message}");
}

/// A **line spacing** over `MAX_LINE_SPACING` is refused.
#[test]
fn a_line_spacing_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "line_spacing": MAX_LINE_SPACING + 1.0 }))
        .expect_err("that spacing is too big");
    let message = error.to_string();
    assert!(message.contains("from zero to"), "{message}");
    assert!(message.contains(&MAX_LINE_SPACING.to_string()), "the refusal names the bound: {message}");
}

/// A bookmark **name** over `MAX_BOOKMARK_NAME` is refused.
#[test]
fn a_bookmark_name_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let name = "b".repeat(MAX_BOOKMARK_NAME + 1);
    let error = session
        .execute("prose.block.bookmark", &json!({ "block": block, "name": name }))
        .expect_err("that name is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_BOOKMARK_NAME.to_string()), "the refusal names the bound: {message}");
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

/// A **label** over `MAX_CAPTION_LABEL` long is refused.
#[test]
fn a_caption_label_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let label = "l".repeat(MAX_CAPTION_LABEL + 1);
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "label": label }))
        .expect_err("that label is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_CAPTION_LABEL.to_string()), "the refusal names the bound: {message}");
}

/// The **text** of a caption over `MAX_CAPTION_TEXT` long is refused.
#[test]
fn a_caption_text_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let text = "t".repeat(MAX_CAPTION_TEXT + 1);
    let error = session
        .execute("prose.block.caption", &json!({ "block": block, "text": text }))
        .expect_err("that text is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_CAPTION_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// The **label** of a table of figures is cut at `MAX_FIGURE_LABEL`.
///
/// The label of a caption is refused when it is too long, and the label
/// of the **table** that is asked for is cut: the identifier of a label
/// is at most the cap long, and the rest of it is not part of it.
#[test]
fn a_figure_label_is_cut_at_the_limit() {
    let mut session = session();
    let document = a_document(&mut session);
    let label = "f".repeat(MAX_FIGURE_LABEL + 5);
    let outcome = session
        .execute("prose.document.figures", &json!({ "document": document, "label": label }))
        .expect("the table of figures is read");
    let ident = outcome.data["label"].as_str().expect("the label is text");
    assert_eq!(ident.len(), MAX_FIGURE_LABEL, "the label is cut at the limit: {ident}");
}

/// A page of **more columns** than `MAX_PAGE_COLUMNS` allows is refused.
#[test]
fn a_page_with_too_many_columns_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute(
            "prose.page.columns",
            &json!({ "document": document, "count": (MAX_PAGE_COLUMNS + 1) as f64 }),
        )
        .expect_err("that page has too many columns");
    let message = error.to_string();
    assert!(message.contains("columns"), "{message}");
    assert!(message.contains(&MAX_PAGE_COLUMNS.to_string()), "the refusal names the bound: {message}");
}

/// A **space** between the columns over `MAX_COLUMN_SPACE` is refused.
#[test]
fn a_column_space_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute(
            "prose.page.columns",
            &json!({ "document": document, "count": 2.0, "space": MAX_COLUMN_SPACE + 1.0 }),
        )
        .expect_err("that space is too big");
    let message = error.to_string();
    assert!(message.contains("a space is from zero to"), "{message}");
    assert!(message.contains(&MAX_COLUMN_SPACE.to_string()), "the refusal names the bound: {message}");
}

/// An **author** of a revision over `MAX_REVISION_AUTHOR` long is refused.
#[test]
fn a_revision_author_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let author = "a".repeat(MAX_REVISION_AUTHOR + 1);
    let runs = json!([{ "len": 1, "revision": { "kind": "insert", "author": author } }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that author is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_REVISION_AUTHOR.to_string()), "the refusal names the bound: {message}");
}

/// A **date** of a revision over `MAX_DATE` long is refused.
#[test]
fn a_revision_date_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let date = "d".repeat(MAX_DATE + 1);
    let runs = json!([{ "len": 1, "revision": { "kind": "insert", "author": "someone", "date": date } }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that date is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_DATE.to_string()), "the refusal names the bound: {message}");
}

/// A link **target** over `MAX_LINK_TARGET` long is refused.
#[test]
fn a_link_target_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let target = "https://example.com/".to_string() + &"a".repeat(MAX_LINK_TARGET);
    let runs = json!([{ "len": 1, "link": target }]);
    let error = session
        .execute("prose.block.runs", &json!({ "block": block, "runs": runs }))
        .expect_err("that target is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_LINK_TARGET.to_string()), "the refusal names the bound: {message}");
}

/// An equation with **more parts** than `MAX_PARTS` allows is refused.
///
/// A level of fractions holds two halves, and each half is a level of
/// its own, so the tree of parts is **wide** as well as deep. The
/// levels that break the cap are worked out from the cap itself.
#[test]
fn an_equation_with_too_many_parts_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let mut parts = json!({ "kind": "text", "text": "x" });
    let mut count = 1usize;
    while count <= MAX_PARTS {
        parts = json!({ "kind": "fraction", "top": parts, "bottom": parts });
        count = count * 2 + 1;
    }
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that equation has too many parts");
    let message = error.to_string();
    assert!(message.contains("parts"), "{message}");
    assert!(message.contains(&MAX_PARTS.to_string()), "the refusal names the bound: {message}");
}

/// An equation **deeper** than `MAX_EQUATION_DEPTH` is refused.
#[test]
fn an_equation_too_deep_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let mut parts = json!({ "kind": "text", "text": "x" });
    for _ in 0..(MAX_EQUATION_DEPTH + 2) {
        parts = json!({ "kind": "fraction", "top": parts, "bottom": { "kind": "text", "text": "y" } });
    }
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that equation is too deep");
    let message = error.to_string();
    assert!(message.contains("deep"), "{message}");
    assert!(message.contains(&MAX_EQUATION_DEPTH.to_string()), "the refusal names the bound: {message}");
}

/// The **text** of an equation over `MAX_EQUATION_TEXT` long is refused.
#[test]
fn an_equation_text_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let parts = json!({ "kind": "text", "text": "t".repeat(MAX_EQUATION_TEXT + 1) });
    let error = session
        .execute("prose.block.equation.add", &json!({ "document": document, "parts": parts }))
        .expect_err("that text is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_EQUATION_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// An **endnote** over `MAX_ENDNOTE_TEXT` long is refused.
#[test]
fn an_endnote_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let text = "e".repeat(MAX_ENDNOTE_TEXT + 1);
    let error = session
        .execute("prose.endnote.add", &json!({ "document": document, "text": text }))
        .expect_err("that endnote is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_ENDNOTE_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// A **footnote** over `MAX_FOOTNOTE_TEXT` long is refused.
#[test]
fn a_footnote_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let text = "f".repeat(MAX_FOOTNOTE_TEXT + 1);
    let error = session
        .execute("prose.footnote.add", &json!({ "document": document, "text": text }))
        .expect_err("that footnote is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_FOOTNOTE_TEXT.to_string()), "the refusal names the bound: {message}");
}

/// An **authority** entry over `MAX_AUTHORITY_ENTRY` long is refused.
#[test]
fn an_authority_entry_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let entry = "e".repeat(MAX_AUTHORITY_ENTRY + 1);
    let error = session
        .execute("prose.authority.mark", &json!({ "block": block, "entry": entry }))
        .expect_err("that entry is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_AUTHORITY_ENTRY.to_string()), "the refusal names the bound: {message}");
}

/// An **index** entry over `MAX_INDEX_ENTRY` long is refused.
#[test]
fn an_index_entry_over_the_limit_is_refused() {
    let mut session = session();
    let block = a_paragraph(&mut session);
    let entry = "e".repeat(MAX_INDEX_ENTRY + 1);
    let error = session
        .execute("prose.index.mark", &json!({ "block": block, "entry": entry }))
        .expect_err("that entry is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_INDEX_ENTRY.to_string()), "the refusal names the bound: {message}");
}

/// A source's **tag** over `MAX_TAG` long is refused.
#[test]
fn a_source_tag_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let tag = "t".repeat(MAX_TAG + 1);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "tag": tag }))
        .expect_err("that tag is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_TAG.to_string()), "the refusal names the bound: {message}");
}

/// A source's **author** over `MAX_SOURCE_AUTHOR` long is refused.
#[test]
fn a_source_author_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let author = "a".repeat(MAX_SOURCE_AUTHOR + 1);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "author": author }))
        .expect_err("that author is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_SOURCE_AUTHOR.to_string()), "the refusal names the bound: {message}");
}

/// A source's **year** over `MAX_YEAR` long is refused.
#[test]
fn a_source_year_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let year = "y".repeat(MAX_YEAR + 1);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "year": year }))
        .expect_err("that year is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_YEAR.to_string()), "the refusal names the bound: {message}");
}

/// A source's **title** over `MAX_SOURCE_PART` long is refused.
///
/// The title is a part of a source, and it is bounded like every other
/// part: a source had a title of any length at all, and that was a hole
/// in the document.
#[test]
fn a_source_title_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let title = "t".repeat(MAX_SOURCE_PART + 1);
    let error = session
        .execute("prose.document.source.set", &json!({ "document": document, "title": title }))
        .expect_err("that title is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_SOURCE_PART.to_string()), "the refusal names the bound: {message}");
}

/// A page's **margin** over `MAX_POINTS` is refused.
#[test]
fn a_page_margin_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute(
            "prose.page.set",
            &json!({ "document": document, "width": 595.0, "height": 842.0, "top": MAX_POINTS + 1.0 }),
        )
        .expect_err("that margin is too big");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_POINTS.to_string()), "the refusal names the bound: {message}");
}

/// A custom property's **name** over `MAX_PROPERTY_NAME` long is refused.
#[test]
fn a_property_name_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let name = "n".repeat(MAX_PROPERTY_NAME + 1);
    let custom = json!({ name: "a value" });
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": custom }))
        .expect_err("that name is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_PROPERTY_NAME.to_string()), "the refusal names the bound: {message}");
}

/// A custom property's **value** over `MAX_VALUE` long is refused.
#[test]
fn a_property_value_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let custom = json!({ "a name": "v".repeat(MAX_VALUE + 1) });
    let error = session
        .execute("prose.document.properties", &json!({ "document": document, "custom": custom }))
        .expect_err("that value is too big");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_VALUE.to_string()), "the refusal names the bound: {message}");
}

/// A page's **start** over `MAX_START` is refused.
#[test]
fn a_page_start_over_the_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute("prose.page.numbers", &json!({ "document": document, "start": (MAX_START + 1) as f64 }))
        .expect_err("that start is too big");
    let message = error.to_string();
    assert!(message.contains("from one to"), "{message}");
    assert!(message.contains(&MAX_START.to_string()), "the refusal names the bound: {message}");
}

/// A style's **level** over `MAX_OUTLINE_LEVEL` is refused.
#[test]
fn a_style_level_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.level", &json!({ "style": style, "level": (MAX_OUTLINE_LEVEL + 1) as f64 }))
        .expect_err("that level is too deep");
    let message = error.to_string();
    assert!(message.contains("level"), "{message}");
    assert!(message.contains(&MAX_OUTLINE_LEVEL.to_string()), "the refusal names the bound: {message}");
}

/// A table of contents asked for **more levels** than `MAX_CONTENT_LEVEL`
/// is refused.
///
/// The levels of a table of contents are the levels of the headings in
/// it, and a level that is not one of the tree's is not one of them.
#[test]
fn a_table_of_contents_over_the_level_limit_is_refused() {
    let mut session = session();
    let document = a_document(&mut session);
    let error = session
        .execute(
            "prose.document.contents",
            &json!({ "document": document, "levels": (MAX_CONTENT_LEVEL + 1) as f64 }),
        )
        .expect_err("that level is too deep");
    let message = error.to_string();
    assert!(message.contains("level"), "{message}");
    assert!(message.contains(&MAX_CONTENT_LEVEL.to_string()), "the refusal names the bound: {message}");
}

/// An **indent** over `MAX_INDENT` is refused.
#[test]
fn an_indent_over_the_limit_is_refused() {
    let mut session = session();
    let style = a_style(&mut session);
    let error = session
        .execute("prose.style.paragraph", &json!({ "style": style, "indent_left": MAX_INDENT + 1.0 }))
        .expect_err("that indent is too big");
    let message = error.to_string();
    assert!(message.contains("a number from zero to"), "{message}");
    assert!(message.contains(&MAX_INDENT.to_string()), "the refusal names the bound: {message}");
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
        .execute(
            "prose.block.numbering",
            &json!({ "block": block, "list": list, "level": MAX_LEVELS as f64 }),
        )
        .expect_err("that level is too deep");
    let message = error.to_string();
    assert!(message.contains("levels"), "{message}");
    assert!(message.contains(&MAX_LEVELS.to_string()), "the refusal names the bound: {message}");
}

/// A **chain** of styles stops at `MAX_STYLE_CHAIN`.
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
    assert_eq!(chain.len(), MAX_STYLE_CHAIN + 1, "the chain stops at the limit: {chain:?}");
    assert!(!chain.iter().any(|id| id.as_u64() == Some(base)), "the walk was cut before the base: {chain:?}");
}
