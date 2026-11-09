//! The **patterns** of a redaction.
//!
//! The text that looks like a phone number or a card is found by the
//! pattern, and the pattern says what it is looking for. The rules
//! are the tree's own.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
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

/// The **matches** of a pattern in a text, as the answer holds them.
fn matches_of(session: &mut Session, pattern: &str, text: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("pdf.redact.pattern.find", &json!({ "pattern": pattern, "text": text }))
        .expect("the text is read");
    outcome.data["matches"].as_array().expect("the matches are a list").clone()
}

/// The **patterns** are the tree's five, and a pattern that is not one
/// of them is refused by name.
#[test]
fn the_patterns_are_the_trees_five() {
    let mut session = session();
    let outcome = session.execute("pdf.redact.patterns", &json!({})).expect("the patterns are read");
    let patterns = outcome.data["patterns"].as_array().expect("the patterns are a list");
    assert_eq!(patterns.len(), 5, "there are five patterns: {outcome:?}");
    assert_eq!(patterns[0]["id"], json!("phone"), "the first is the phone: {outcome:?}");
    assert_eq!(patterns[0]["label"], json!("Phone Numbers"), "with the tree's label: {outcome:?}");
    let error = session
        .execute("pdf.redact.pattern.find", &json!({ "pattern": "postcode", "text": "hello" }))
        .expect_err("the pattern is refused");
    assert!(error.to_string().contains("is not a pattern of a redaction"), "it says why: {error}");
}

/// A **phone number** has ten or eleven digits and a separator.
#[test]
fn a_phone_number_is_found() {
    let mut session = session();
    let matches = matches_of(&mut session, "phone", "Call 555-123-4567 or 555 123 4568 today");
    assert_eq!(matches.len(), 2, "there are two numbers: {matches:?}");
    assert_eq!(matches[0]["text"], json!("555-123-4567"), "the first is found: {matches:?}");
    assert_eq!(matches[1]["text"], json!("555 123 4568"), "and the second: {matches:?}");
}

/// An **email address** has a local part, an `@` and a domain.
#[test]
fn an_email_address_is_found() {
    let mut session = session();
    let matches = matches_of(&mut session, "email", "write to ann.smith@example.co.uk today");
    assert_eq!(matches.len(), 1, "there is one address: {matches:?}");
    assert_eq!(matches[0]["text"], json!("ann.smith@example.co.uk"), "and it is found: {matches:?}");
}

/// A **credit card** passes the Luhn check.
#[test]
fn a_credit_card_is_found() {
    let mut session = session();
    let matches = matches_of(
        &mut session,
        "credit-card",
        "the card is 4111 1111 1111 1111 and not 1234 5678 9012 3456",
    );
    assert_eq!(matches.len(), 1, "there is one card: {matches:?}");
    assert_eq!(matches[0]["text"], json!("4111 1111 1111 1111"), "and it is found: {matches:?}");
}

/// A **social security number** has three digits, two and four.
#[test]
fn a_social_security_number_is_found() {
    let mut session = session();
    let matches = matches_of(&mut session, "ssn", "the number is 123-45-6789 and not 000-45-6789");
    assert_eq!(matches.len(), 1, "there is one number: {matches:?}");
    assert_eq!(matches[0]["text"], json!("123-45-6789"), "and it is found: {matches:?}");
}

/// A **date** is written in one of the tree's ways.
#[test]
fn a_date_is_found() {
    let mut session = session();
    let matches = matches_of(&mut session, "date", "the day is 5 January 2024 and the other 2024-01-05");
    assert_eq!(matches.len(), 2, "there are two dates: {matches:?}");
    assert_eq!(matches[0]["text"], json!("5 January 2024"), "the first is found: {matches:?}");
    assert_eq!(matches[1]["text"], json!("2024-01-05"), "and the second: {matches:?}");
}
