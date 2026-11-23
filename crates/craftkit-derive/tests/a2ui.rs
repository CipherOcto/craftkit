//! The `#[derive(A2uiComponent)]` and the `components!` table, **used**.
//!
//! The token-level tests live in the crate itself — a `proc-macro` crate
//! may not export anything but its macros — and this test is the other
//! half: the macro **used**, against a **stub layer** written here.
//!
//! The stub layer is the two things the macro names: the **table** (the
//! property names) and the **shapes** (the re-derived schema text). They
//! are stubs, and they are in this test's own scope — so the macro's
//! paths resolve here, and the real layer is not needed to prove the
//! macro works.

use craftkit_derive::{A2uiComponent, Layer};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{CatalogEntry, Ecosystem, NodeKindSpec, Registry, Text};

/// The stub **layer**.
///
/// The two paths the macro names: the table of property names, and the
/// shapes.
mod layer {
    /// The **table**: each component's property names, as byte arrays.
    ///
    /// The real layer's table holds every component; this one holds the
    /// two this test uses.
    pub mod defs {
        /// The property names of `Text`.
        pub const TEXT: &[&[u8]] = &[b"text", b"variant", b"weight"];

        /// The property names of `Divider`.
        pub const DIVIDER: &[&[u8]] = &[b"axis"];

        /// Whether two name sets are the same.
        ///
        /// Byte-wise: a slice's `PartialEq` is **not** const-evaluable,
        /// and neither is `str`'s. This is the real layer's rule.
        pub const fn matches(table: &[&[u8]], names: &[&[u8]]) -> bool {
            let mut index = 0;
            while index < names.len() {
                if !holds(table, names[index]) {
                    return false;
                }
                index += 1;
            }
            let mut index = 0;
            while index < table.len() {
                if !holds(names, table[index]) {
                    return false;
                }
                index += 1;
            }
            true
        }

        /// Whether a set holds a name.
        const fn holds(set: &[&[u8]], name: &[u8]) -> bool {
            let mut index = 0;
            while index < set.len() {
                if same(set[index], name) {
                    return true;
                }
                index += 1;
            }
            false
        }

        /// Whether two byte slices are the same.
        const fn same(left: &[u8], right: &[u8]) -> bool {
            if left.len() != right.len() {
                return false;
            }
            let mut index = 0;
            while index < left.len() {
                if left[index] != right[index] {
                    return false;
                }
                index += 1;
            }
            true
        }
    }

    /// The **shapes**: the re-derived schema text.
    pub mod schema {
        use craftkit_sdk::serde_json::{Value as Json, json};

        /// The published schema of a component.
        ///
        /// The real layer stamps the discriminator, resolves the `$ref`s
        /// and writes the `required` list; this stub does the first and
        /// the last, and says so.
        /// The published schema, with the **row's** optionality.
        ///
        /// A prop the row calls optional is optional, and the stub says
        /// so: the real layer has its own rules as well.
        pub fn published_with(
            name: &str,
            names: &[&str],
            definitions: &[Option<&str>],
            optionals: &[bool],
            alternatives: &[Option<&str>],
        ) -> Json {
            let mut properties = craftkit_sdk::serde_json::Map::new();
            let mut required: Vec<Json> = Vec::new();
            for (index, prop) in names.iter().enumerate() {
                let shape = match definitions.get(index).and_then(|value| *value) {
                    Some(definition) => json!({ "$ref": format!("#/$defs/{definition}") }),
                    None => json!({ "type": "string" }),
                };
                properties.insert((*prop).to_string(), shape.clone());
                if let Some(alternative) = alternatives.get(index).copied().flatten() {
                    properties.insert(alternative.to_string(), shape);
                }
                if !optionals.get(index).copied().unwrap_or(false)
                    && alternatives.get(index).copied().flatten().is_none()
                {
                    required.push(Json::String((*prop).to_string()));
                }
            }
            properties.insert("component".to_string(), json!({ "const": name }));
            json!({
                "type": "object",
                "properties": properties,
                "required": required,
            })
        }
    }
}

/// The **layer** the components belong to.
///
/// The ecosystem refuses a contribution whose layer is not linked in, so
/// the layer is declared here — with the same derive every plugin uses.
#[derive(Layer)]
#[layer(id = "a2ui", name = "A2UI", requires("core"))]
pub struct A2uiLayer;

/// A divider: a line.
///
/// The doc comment is the component's one line about itself.
#[derive(A2uiComponent)]
#[a2ui_component(
    name = "Divider",
    kind = "a2ui.divider",
    layer = "a2ui",
    table = "crate::layer::defs",
    shapes = "crate::layer::schema"
)]
pub struct DividerComponent {
    /// Which way it goes.
    pub axis: Option<Text>,
}

impl DividerComponent {
    /// How it is drawn.
    fn paint(
        node: &craftkit_sdk::Node,
        _document: &craftkit_sdk::Document,
        _canvas: &mut dyn craftkit_sdk::Canvas,
    ) {
        let _ = node;
    }

    /// Whether a point is inside it.
    fn hit(
        node: &craftkit_sdk::Node,
        _document: &craftkit_sdk::Document,
        point: craftkit_sdk::Point,
    ) -> bool {
        node.rect.map(|rect| rect.contains(point)).unwrap_or(false)
    }
}

/// The **kind** the derive wrote.
#[test]
fn the_derive_writes_the_kind() {
    let spec: &NodeKindSpec = &DIVIDER_COMPONENT;
    assert_eq!(spec.kind, "a2ui.divider");
    assert_eq!(spec.label, "Divider");
    assert_eq!(spec.layer, "a2ui");
}

/// And the **catalogue entry**.
#[test]
fn the_derive_writes_the_catalogue_entry() {
    let entry: &CatalogEntry = &DIVIDER_CATALOG;
    assert_eq!(entry.id, "a2ui.divider", "the id is craftkit's");
    assert_eq!(entry.name, "Divider", "and the name is the published one");
    assert_eq!(entry.layer, "a2ui");
    assert!(entry.doc.contains("A divider"), "{}", entry.doc);
}

/// The **published schema** is the layer's shape, with the discriminator
/// stamped and the `$ref`s resolved.
#[test]
fn the_published_schema_is_the_layers_shape() {
    let schema = (from_the_table::TEXT_CATALOG.schema)();
    assert_eq!(schema["properties"]["component"], json!({ "const": "Text" }));
    assert_eq!(schema["properties"]["text"], json!({ "$ref": "#/$defs/DynamicString" }));
    assert_eq!(schema["properties"]["weight"], json!({ "$ref": "#/$defs/Weight" }));
    // A plain prop has no definition, so the layer owns its shape.
    assert_eq!(schema["properties"]["variant"], json!({ "type": "string" }));
}

/// And the **declarations** — the craftkit side.
#[test]
fn the_declarations_are_the_craftkit_side() {
    let docs = from_the_table::TextComponent::doc();
    assert_eq!(docs.len(), 3, "there are three props");
    assert_eq!(docs[0].name, "text");
    assert!(docs[0].required, "a plain type is required");
    assert_eq!(docs[1].name, "variant");
    assert!(!docs[1].required, "an Option is not");
    assert_eq!(docs[2].name, "weight");
    assert!(!docs[2].required, "an Option is not");
}

/// And the **reader** — the props are read out of a caller's values.
#[test]
fn the_reader_reads_the_props() {
    let values = json!({ "text": "hello", "variant": "body" });
    let params = craftkit_sdk::Params::new(&values);
    let props = from_the_table::TextComponent::read(&params).expect("the props are read");
    assert_eq!(props.text.0, "hello");
    assert_eq!(props.variant.map(|value| value.0), Some("body".to_string()));
    assert!(props.weight.is_none(), "and the absent one is None");
}

/// And the **typed view** of a node's payload.
#[test]
fn the_typed_view_reads_a_payload() {
    let node = craftkit_sdk::Node {
        id: 1,
        name: "text".to_string(),
        kind: "a2ui.text".to_string(),
        rect: None,
        muted: false,
        hidden: false,
        locked: false,
        appearance: None,
        appearance_from: None,
        data: json!({ "text": "hello" }),
        children: Vec::new(),
        stream: None,
        unknown: Json::Null,
    };
    let props = from_the_table::TextComponent::of(&node).expect("the payload fits");
    assert_eq!(props.text.0, "hello");
}

/// And a node whose payload does **not** fit is `None`.
#[test]
fn a_payload_that_does_not_fit_is_none() {
    let node = craftkit_sdk::Node {
        id: 1,
        name: "text".to_string(),
        kind: "a2ui.text".to_string(),
        rect: None,
        muted: false,
        hidden: false,
        locked: false,
        appearance: None,
        appearance_from: None,
        data: json!({ "variant": "body" }),
        children: Vec::new(),
        stream: None,
        unknown: Json::Null,
    };
    assert!(from_the_table::TextComponent::of(&node).is_none(), "the missing prop is missing");
}

/// The two registrations reach the **registry**.
#[test]
fn the_two_registrations_reach_the_registry() {
    let mut kinds: Registry<NodeKindSpec> = Registry::new();
    kinds.register(DIVIDER_COMPONENT).expect("the kind is accepted");
    let mut catalog: Registry<CatalogEntry> = Registry::new();
    catalog.register(DIVIDER_CATALOG.clone()).expect("the entry is accepted");
    assert_eq!(kinds.all().len(), 1);
    assert_eq!(catalog.all().len(), 1);
    assert_eq!(catalog.all()[0].name, "Divider");
}

/// The **table** macro writes the same work for every row.
///
/// It is in its own module, because it writes the same names the derive
/// wrote — and the two are compared, not collided.
mod from_the_table {
    use craftkit_derive::components;
    use craftkit_sdk::{Number, Text};

    components! {
        table = "crate::layer::defs",
        shapes = "crate::layer::schema",
        {
        /// A line of text.
        Text {
            kind: "a2ui.text",
            props: {
                /// The text to show.
                #[a2ui_prop(DynamicString)]
                text: Text,
                /// A hint for the base text size.
                variant: Option<Text>,
                /// How much of the row this takes.
                #[a2ui_prop(Weight)]
                weight: Option<Number>,
            },
            paint: {
                let _ = node;
            },
            hit: {
                node.rect.map(|rect| rect.contains(point)).unwrap_or(false)
            },
        }
        }
    }
}

/// The table's row makes the same statics.
#[test]
fn the_table_makes_the_same_statics() {
    let spec: &NodeKindSpec = &from_the_table::TEXT_COMPONENT;
    assert_eq!(spec.kind, "a2ui.text");
    let entry: &CatalogEntry = &from_the_table::TEXT_CATALOG;
    assert_eq!(entry.name, "Text");
}

/// And the table's row is the **same** component as a derive's would be.
#[test]
fn the_table_and_the_derive_agree() {
    assert_eq!(from_the_table::TextComponent::doc().len(), 3, "the same props");
    let from_table = (from_the_table::TEXT_CATALOG.schema)();
    assert_eq!(from_table["properties"]["text"], json!({ "$ref": "#/$defs/DynamicString" }));
    assert_eq!(from_table["properties"]["weight"], json!({ "$ref": "#/$defs/Weight" }));
    assert_eq!(from_table["properties"]["variant"], json!({ "type": "string" }));
}

/// The contributions reach the **ecosystem**.
#[test]
fn the_contributions_reach_the_ecosystem() {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    assert!(ecosystem.kinds.all().iter().any(|kind| kind.kind == "a2ui.divider"), "the kind is there");
    assert!(ecosystem.catalog.all().iter().any(|entry| entry.name == "Divider"), "and the catalogue entry");
}
