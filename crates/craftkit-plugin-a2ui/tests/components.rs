//! The **eleven components** — they are craftkit kinds and catalogue
//! entries.
//!
//! The test says two things: that the components reach the **registry**,
//! and that each one's **published schema** carries exactly the props the
//! layer's table holds.

use craftkit_a2ui::defs;
use craftkit_a2ui::schema;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{CatalogEntry, Ecosystem, NodeKindSpec};

// The plugin's anchor, held by the test: without it the linker drops the
// plugin's items and the registry is empty.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_a2ui::CRAFTKIT_PLUGIN_ANCHOR;

/// The **eleven** components, and the table each one is checked against.
const COMPONENTS: &[(&str, &str, &[&[u8]])] = &[
    ("Text", "a2ui.text", defs::TEXT),
    ("Image", "a2ui.image", defs::IMAGE),
    ("Icon", "a2ui.icon", defs::ICON),
    ("Row", "a2ui.row", defs::ROW),
    ("Column", "a2ui.column", defs::COLUMN),
    ("List", "a2ui.list", defs::LIST),
    ("Card", "a2ui.card", defs::CARD),
    ("Divider", "a2ui.divider", defs::DIVIDER),
    ("Button", "a2ui.button", defs::BUTTON),
    ("CheckBox", "a2ui.checkbox", defs::CHECK_BOX),
    ("TextField", "a2ui.textfield", defs::TEXT_FIELD),
];

/// The ecosystem with the plugin linked in.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// The **eleven** components reach the registry as **kinds**.
#[test]
fn the_eleven_components_reach_the_registry_as_kinds() {
    let ecosystem = ecosystem();
    for (_, kind, _) in COMPONENTS {
        assert!(ecosystem.kinds.all().iter().any(|entry| entry.kind == *kind), "{kind} is a kind");
    }
    // And there are **eleven** of them, and no more.
    let a2ui: Vec<&str> = ecosystem
        .kinds
        .all()
        .iter()
        .filter(|entry| entry.kind.starts_with("a2ui."))
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(a2ui.len(), 11, "and nothing else: {a2ui:?}");
}

/// And as **catalogue entries**, by their **a2ui** names.
#[test]
fn the_eleven_components_reach_the_registry_as_catalogue_entries() {
    let ecosystem = ecosystem();
    for (name, _, _) in COMPONENTS {
        assert!(
            ecosystem.catalog.all().iter().any(|entry| entry.name == *name),
            "{name} is a catalogue entry"
        );
    }
}

/// The **layer** is declared, so that a contribution in it is allowed.
#[test]
fn the_layer_is_declared() {
    let ecosystem = ecosystem();
    assert!(ecosystem.layers.all().iter().any(|layer| layer.id == "a2ui"), "the a2ui layer is there");
}

/// Each component's **published schema** carries exactly the props the
/// layer's table holds.
///
/// This is the third of the three checks: the **row** against the
/// **published** schema. The first — the row against the table — is the
/// derive's **compile-time** assertion, and the second — the table
/// against the schema text — is the layer's own test.
#[test]
fn each_published_schema_carries_exactly_the_tables_props() {
    let ecosystem = ecosystem();
    for (name, _, table) in COMPONENTS {
        let entry = ecosystem
            .catalog
            .all()
            .iter()
            .find(|entry| entry.name == *name)
            .unwrap_or_else(|| panic!("{name} is a catalogue entry"));
        let published = (entry.schema)();
        let properties = schema::properties_of(&published);
        let expected: Vec<String> =
            table.iter().map(|prop| String::from_utf8_lossy(prop).to_string()).collect();
        // The **discriminator** is stamped on top of the row's props.
        assert_eq!(properties.len(), expected.len() + 1, "{name} holds {properties:?}");
        for prop in &expected {
            assert!(properties.contains(prop), "{name} is missing `{prop}`");
        }
    }
}

/// The **discriminator** is stamped on every one of them.
#[test]
fn every_component_carries_the_discriminator() {
    let ecosystem = ecosystem();
    for (name, _, _) in COMPONENTS {
        let entry = ecosystem
            .catalog
            .all()
            .iter()
            .find(|entry| entry.name == *name)
            .unwrap_or_else(|| panic!("{name} is a catalogue entry"));
        let published = (entry.schema)();
        let text = published.to_string();
        assert!(text.contains(name), "{name} carries its own name: {text}");
    }
}

/// The **catalogue** holds the eleven components and nothing else.
#[test]
fn the_catalogue_holds_the_eleven_components() {
    let ecosystem = ecosystem();
    let projection = ecosystem.projection();
    let catalogue = craftkit_a2ui::catalog::catalogue(&projection.entries());
    let components = catalogue["components"].as_object().expect("there are components");
    assert_eq!(components.len(), 11, "and nothing else: {components:?}");
    assert!(components.contains_key("Text"));
    assert!(components.contains_key("TextField"));
    // And a refused component is not there.
    assert!(!components.contains_key("Video"));
    assert!(!components.contains_key("Modal"));
}

/// The **kind** and the **entry** are the same component, under two
/// names.
#[test]
fn the_kind_and_the_entry_are_the_same_component() {
    let ecosystem = ecosystem();
    for (name, kind, _) in COMPONENTS {
        let spec: Option<&NodeKindSpec> = ecosystem.kinds.all().iter().find(|entry| entry.kind == *kind);
        let entry: Option<&CatalogEntry> = ecosystem.catalog.all().iter().find(|entry| entry.name == *name);
        assert!(spec.is_some(), "{kind} is a kind");
        assert!(entry.is_some(), "{name} is an entry");
        assert_eq!(spec.expect("the kind").layer, "a2ui");
        assert_eq!(entry.expect("the entry").layer, "a2ui");
    }
}

/// The **icon** is a path, not one of the fifty-nine names.
#[test]
fn the_icon_is_a_path() {
    let ecosystem = ecosystem();
    let entry = ecosystem
        .catalog
        .all()
        .iter()
        .find(|entry| entry.name == "Icon")
        .expect("the icon is a catalogue entry");
    let published = (entry.schema)();
    let branches = published["properties"]["name"]["oneOf"].as_array().expect("it is a oneOf");
    assert_eq!(branches.len(), 2, "the fifty-nine names are gone: {branches:?}");
    // The first branch is the **path**, and the second is a **binding**.
    assert_eq!(branches[0]["required"], json!(["svgPath"]));
    assert_eq!(branches[0]["properties"]["svgPath"], json!({ "$ref": "#/$defs/DynamicString" }));
    assert_eq!(branches[1], json!({ "$ref": "#/$defs/DataBinding" }));
}

/// The **catalogue** carries one `catalogId`, and it is the layer's own.
#[test]
fn the_catalogue_carries_one_catalog_id() {
    let ecosystem = ecosystem();
    let projection = ecosystem.projection();
    let catalogue = craftkit_a2ui::catalog::catalogue(&projection.entries());
    assert_eq!(catalogue["catalogId"], json!(craftkit_a2ui::catalog::CATALOG_ID));
}

/// **Each component paints something**, and the picture is not empty.
///
/// A component that draws nothing is a component the tree cannot show,
/// so the eleven are drawn one at a time and the canvas is asked what
/// it holds.
#[test]
fn each_component_paints_something() {
    // The **eleven**, each with the props it needs.
    let cases: Vec<(&str, craftkit_sdk::serde_json::Value)> = vec![
        ("Text", json!({ "component": "Text", "text": "hello" })),
        ("Image", json!({ "component": "Image", "url": "https://example.com/i.png" })),
        ("Icon", json!({ "component": "Icon", "name": "M 0 0 L 10 0 L 10 10 Z" })),
        ("Row", json!({ "component": "Row", "children": [] })),
        ("Column", json!({ "component": "Column", "children": [] })),
        ("List", json!({ "component": "List", "children": [] })),
        ("Card", json!({ "component": "Card", "child": "a2ui.text" })),
        ("Divider", json!({ "component": "Divider" })),
        ("Button", json!({ "component": "Button", "child": "press" })),
        ("CheckBox", json!({ "component": "CheckBox", "label": "tick" })),
        ("TextField", json!({ "component": "TextField", "label": "name" })),
    ];
    assert_eq!(cases.len(), COMPONENTS.len(), "every component is drawn");
    for (name, component) in cases {
        let mut session = ecosystem().session();
        session.execute("a2ui.surface.create", &json!({ "surface_id": name })).expect("the surface is made");
        // The **surface alone**, so the count before is the surface's own
        // drawing and not the component's.
        let before = {
            let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
            session.paint_view(&mut canvas);
            canvas.drawn.len()
        };
        session
            .execute("a2ui.surface.apply", &json!({ "surface_id": name, "components": [component] }))
            .expect("the component is drawn");
        let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
        session.paint_view(&mut canvas);
        assert!(canvas.drawn.len() > before, "{name} paints nothing");
    }
}
