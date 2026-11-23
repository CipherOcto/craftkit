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
        // The **discriminator** and the **id** are stamped on top of the
        // row's props.
        assert_eq!(properties.len(), expected.len() + 2, "{name} holds {properties:?}");
        assert!(properties.contains(&"component".to_string()), "{name} has the discriminator");
        assert!(properties.contains(&"id".to_string()), "{name} has the id");
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
    let catalogue = craftkit_a2ui::catalog::catalogue(&projection.entries()).expect("the catalogue is built");
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
    let catalogue = craftkit_a2ui::catalog::catalogue(&projection.entries()).expect("the catalogue is built");
    assert_eq!(catalogue["catalogId"], json!(craftkit_a2ui::catalog::CATALOG_ID));
}

/// A picture of **one pixel**, as a PNG.
///
/// The bytes are the tree's own test bytes — a file the scratch hands
/// out and the plugin's reader reads.
const A_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00,
    0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc, 0xcf, 0xc0, 0x50, 0x0f, 0x00, 0x04,
    0x85, 0x01, 0x80, 0x84, 0xa9, 0x8c, 0x21, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42,
    0x60, 0x82,
];

/// **Each component paints something**, and the picture is not empty.
///
/// A component that draws nothing is a component the tree cannot show,
/// so the eleven are drawn one at a time and the canvas is asked what
/// it holds.
#[test]
fn each_component_paints_something() {
    // The **eleven**, each with the props it needs.
    let cases: Vec<(&str, craftkit_sdk::serde_json::Value)> = vec![
        ("Text", json!([{ "component": "Text", "id": "root", "text": "hello" }])),
        ("Image", json!([{ "component": "Image", "id": "root", "url": "a picture that is not there yet" }])),
        ("Icon", json!([{ "component": "Icon", "id": "root", "name": "M 0 0 L 10 0 L 10 10 Z" }])),
        ("Row", json!([{ "component": "Row", "id": "root", "children": [] }])),
        ("Column", json!([{ "component": "Column", "id": "root", "children": [] }])),
        ("List", json!([{ "component": "List", "id": "root", "children": [] }])),
        (
            "Card",
            json!([
                { "component": "Card", "id": "root", "child": "t1" },
                { "component": "Text", "id": "t1", "text": "inside" },
            ]),
        ),
        ("Divider", json!([{ "component": "Divider", "id": "root" }])),
        (
            "Button",
            json!([
                { "component": "Button", "id": "root", "child": "t1" },
                { "component": "Text", "id": "t1", "text": "press" },
            ]),
        ),
        ("CheckBox", json!([{ "component": "CheckBox", "id": "root", "label": "tick" }])),
        ("TextField", json!([{ "component": "TextField", "id": "root", "label": "name" }])),
    ];
    assert_eq!(cases.len(), COMPONENTS.len(), "every component is drawn");
    // The **image** is a file, and the file is a PNG — so the test asks
    // the scratch for one and writes it.
    let picture = craftkit_scratch::file("a2ui-image.png").expect("the scratch is made");
    std::fs::write(&picture, A_PNG).expect("the picture is written");
    let cases = cases
        .into_iter()
        .map(|(name, component)| {
            if name != "Image" {
                return (name, component);
            }
            let mut component = component;
            component[0]["url"] = json!(picture.to_string_lossy());
            (name, component)
        })
        .collect::<Vec<_>>();
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
            .execute("a2ui.surface.apply", &json!({ "surface_id": name, "components": component }))
            .expect("the component is drawn");
        let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
        session.paint_view(&mut canvas);
        assert!(canvas.drawn.len() > before, "{name} paints nothing");
    }
}

/// A **bound** prop is resolved against the model, and the component
/// draws the value the binding names.
///
/// The props are a2ui's types and the component holds the **resolved**
/// values — so a bound text is the text in the model, and not an empty
/// picture because the reader refused to guess.
#[test]
fn a_bound_prop_is_resolved_against_the_model() {
    let mut session = ecosystem().session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s1" })).expect("the surface is made");
    session
        .execute("a2ui.data.set", &json!({ "surface_id": "s1", "path": "/greeting", "value": "hello" }))
        .expect("the model holds it");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s1",
                "components": [{ "component": "Text", "id": "root", "text": { "@path": "/greeting" } }],
            }),
        )
        .expect("the tree is drawn");
    let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.contains("hello")),
        "the bound text is drawn: {:?}",
        canvas.drawn
    );
}

/// And a binding that names a path that is **not there** draws nothing
/// — and says so, rather than drawing the path.
#[test]
fn a_binding_that_names_nothing_draws_nothing() {
    let mut session = ecosystem().session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s2" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s2",
                "components": [{ "component": "Text", "id": "root", "text": { "@path": "/nowhere" } }],
            }),
        )
        .expect("the tree is drawn");
    let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn.contains("nowhere")),
        "the path is not drawn: {:?}",
        canvas.drawn
    );
}

/// The **icon** is a path in a2ui's own shape: an object with an
/// `svgPath`, which is what the catalogue publishes.
#[test]
fn the_icon_is_drawn_from_its_own_shape() {
    let mut session = ecosystem().session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s3" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s3",
                "components": [{
                    "component": "Icon",
                    "id": "root",
                    "name": { "svgPath": "M 0 0 L 10 0 L 10 10 Z" },
                }],
            }),
        )
        .expect("the icon is drawn");
    let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("path(")),
        "the path is drawn: {:?}",
        canvas.drawn
    );
}

/// And a picture that is **not there** is refused: nothing is fetched and
/// nothing is guessed at.
#[test]
fn a_picture_that_is_not_there_is_refused() {
    let mut session = ecosystem().session();
    session.execute("a2ui.surface.create", &json!({ "surface_id": "s4" })).expect("the surface is made");
    session
        .execute(
            "a2ui.surface.apply",
            &json!({
                "surface_id": "s4",
                "components": [{
                    "component": "Image",
                    "id": "root",
                    "url": "https://example.com/not-fetched.png",
                }],
            }),
        )
        .expect("the image is drawn");
    let mut canvas = craftkit_sdk::document::RecordingCanvas::default();
    session.paint_view(&mut canvas);
    assert!(
        !canvas.drawn.iter().any(|drawn| drawn.starts_with("picture(")),
        "nothing is fetched: {:?}",
        canvas.drawn
    );
}
