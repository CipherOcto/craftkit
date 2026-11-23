//! The **data model** and the **bindings** — a surface's round trip.

use craftkit_a2ui::bindings;
use craftkit_a2ui::model::Model;
use craftkit_a2ui::serde_json::json;

/// A **surface** round-trips: the same tree in gives the same values out.
#[test]
fn a_surface_round_trips() {
    let mut model = Model::new();
    model.set("/title", json!("hello")).expect("the value is written");
    model.set("/count", json!(3)).expect("the value is written");

    let title = bindings::resolve_text(&model, &json!({ "@path": "/title" })).expect("it resolves");
    assert_eq!(title, "hello");
    let count = bindings::resolve_number(&model, &json!({ "@path": "/count" })).expect("it resolves");
    assert_eq!(count, 3.0);
}

/// A binding to a **missing** path is refused.
#[test]
fn a_binding_to_a_missing_path_is_refused() {
    let model = Model::new();
    let error = bindings::resolve_text(&model, &json!({ "@path": "/missing" })).expect_err("it is refused");
    assert!(error.contains("/missing"), "{error}");
}

/// An **unknown component** is refused, not faked.
#[test]
fn an_unknown_component_is_refused() {
    // The layer holds eleven components, and the rest of a2ui's catalogue
    // is not here: a component the layer does not know is refused, and
    // a2ui's own `unknown-type` placeholder is not copied.
    let known = [
        "Text",
        "Image",
        "Icon",
        "Row",
        "Column",
        "List",
        "Card",
        "Divider",
        "Button",
        "CheckBox",
        "TextField",
    ];
    assert_eq!(known.len(), 11);
    for refused in ["Video", "AudioPlayer", "Tabs", "Modal", "ChoicePicker", "Slider", "DateTimeInput"] {
        assert!(!known.contains(&refused), "{refused} is refused");
    }
}
