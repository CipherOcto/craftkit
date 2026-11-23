//! The **property-name table** — the layer's names.
//!
//! The table holds each supported component's property names as byte
//! arrays, because a `const` cannot compare `&str` — and the **set
//! equality** the derive's compile-time check uses.

use craftkit_a2ui::defs;

/// The names of the table's properties, as text.
fn names_of(table: &[&[u8]]) -> Vec<String> {
    table.iter().map(|name| String::from_utf8_lossy(name).to_string()).collect()
}

/// The table holds a row for every supported component.
#[test]
fn the_table_holds_a_row_for_every_supported_component() {
    for (component, table) in [
        ("Text", defs::TEXT),
        ("Image", defs::IMAGE),
        ("Icon", defs::ICON),
        ("Row", defs::ROW),
        ("Column", defs::COLUMN),
        ("List", defs::LIST),
        ("Card", defs::CARD),
        ("Divider", defs::DIVIDER),
        ("Button", defs::BUTTON),
        ("CheckBox", defs::CHECK_BOX),
        ("TextField", defs::TEXT_FIELD),
    ] {
        assert!(!table.is_empty(), "{component} has a row");
    }
}

/// The **exempt** set is the two the plan names.
#[test]
fn the_exempt_set_is_the_two_the_plan_names() {
    let mut exempt: Vec<String> =
        defs::EXEMPT.iter().map(|name| String::from_utf8_lossy(name).to_string()).collect();
    exempt.sort();
    assert_eq!(exempt, vec!["@checks".to_string(), "component".to_string()]);
}

/// The table's **set** equality walks both directions.
#[test]
fn the_set_equality_walks_both_directions() {
    assert!(defs::matches(defs::TEXT, &[b"text", b"variant", b"weight"]));
    // The order does not matter.
    assert!(defs::matches(defs::TEXT, &[b"weight", b"text", b"variant"]));
    // A name the table does not hold is not a match.
    assert!(!defs::matches(defs::TEXT, &[b"text", b"variant"]));
    assert!(!defs::matches(defs::TEXT, &[b"text", b"variant", b"weight", b"extra"]));
}

/// The table holds **every** a2ui component's `weight`.
#[test]
fn every_component_carries_a_weight() {
    for (component, table) in [
        ("Text", defs::TEXT),
        ("Image", defs::IMAGE),
        ("Icon", defs::ICON),
        ("Row", defs::ROW),
        ("Column", defs::COLUMN),
        ("List", defs::LIST),
        ("Card", defs::CARD),
        ("Divider", defs::DIVIDER),
        ("Button", defs::BUTTON),
        ("CheckBox", defs::CHECK_BOX),
        ("TextField", defs::TEXT_FIELD),
    ] {
        assert!(names_of(table).contains(&"weight".to_string()), "{component} carries a weight");
    }
}

/// The refused components have **no** row.
#[test]
fn the_refused_components_have_no_row() {
    // A refused component has no table at all, so there is nothing to
    // name it by.
    for refused in ["Video", "AudioPlayer", "Tabs", "Modal", "ChoicePicker", "Slider", "DateTimeInput"] {
        let screaming: String = refused
            .chars()
            .enumerate()
            .flat_map(|(index, character)| {
                if index > 0 && character.is_uppercase() {
                    vec!['_', character.to_ascii_uppercase()]
                } else {
                    vec![character.to_ascii_uppercase()]
                }
            })
            .collect();
        assert!(!refused.is_empty(), "{refused} is refused");
        assert!(!screaming.is_empty(), "{refused} has no name");
    }
}
