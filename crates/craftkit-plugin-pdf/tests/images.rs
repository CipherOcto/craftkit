//! The **images** of a file.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-images-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// An **image**: a stream that says it is one.
fn an_image(session: &mut Session, name: &str, entries: Json) {
    let directory = directory(name);
    let path = format!("{directory}/image.bin");
    std::fs::write(&path, [0_u8; 4]).expect("the bytes are written");
    let mut all = vec![json!(["Subtype", "Image"])];
    if let Some(list) = entries.as_array() {
        all.extend(list.iter().cloned());
    }
    session
        .execute("pdf.stream.add", &json!({ "object": 2, "entries": all, "stream": path }))
        .expect("the image is added");
}

/// The **clauses** the checker reports.
fn clauses_of(session: &mut Session) -> Vec<String> {
    let outcome = session.execute("pdf.preflight", &json!({})).expect("the checker runs");
    outcome.data["issues"]
        .as_array()
        .expect("the issues are a list")
        .iter()
        .filter_map(|issue| issue["clause"].as_str().map(|clause| clause.to_string()))
        .collect()
}

/// The **entries** of the image, as the document holds them.
fn entries_of(session: &Session) -> Json {
    session
        .document()
        .walk()
        .find(|node| node.kind == "pdf.stream")
        .expect("the image is there")
        .data["entries"]
        .clone()
}

/// The **key** an entry holds, and whether it is there.
fn has_key(entries: &Json, key: &str) -> bool {
    entries.as_array().expect("the entries are a list").iter().any(|entry| {
        entry.as_array().and_then(|pair| pair.first()).and_then(|name| name.as_str()) == Some(key)
    })
}

/// An image that asks to be **interpolated** breaks the rule.
#[test]
fn an_image_that_asks_to_be_interpolated_breaks_the_rule() {
    let mut session = session();
    an_image(&mut session, "interpolate", json!([["Interpolate", true]]));
    assert!(clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is broken");
}

/// An image with **alternates** breaks the rule.
#[test]
fn an_image_with_alternates_breaks_the_rule() {
    let mut session = session();
    an_image(&mut session, "alternates", json!([["Alternates", { "ref": [3, 0] }]]));
    assert!(clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is broken");
}

/// An image with **OPI information** breaks the rule.
#[test]
fn an_image_with_opi_information_breaks_the_rule() {
    let mut session = session();
    an_image(&mut session, "opi", json!([["OPI", { "ref": [3, 0] }]]));
    assert!(clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is broken");
}

/// An image that asks for **nothing** breaks nothing.
#[test]
fn an_image_that_asks_for_nothing_breaks_nothing() {
    let mut session = session();
    an_image(&mut session, "plain", json!([]));
    assert!(!clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is not broken");
}

/// The conversion **clears the interpolation**.
#[test]
fn the_conversion_clears_the_interpolation() {
    let mut session = session();
    an_image(&mut session, "cleared", json!([["Interpolate", true]]));
    session.execute("pdf.convert", &json!({})).expect("the file is converted");
    let entries = entries_of(&session);
    assert!(has_key(&entries, "Interpolate"), "the flag is still there");
    let value = entries
        .as_array()
        .expect("the entries are a list")
        .iter()
        .find_map(|entry| {
            let pair = entry.as_array()?;
            if pair.first()?.as_str()? != "Interpolate" {
                return None;
            }
            Some(pair.get(1)?.clone())
        })
        .expect("the flag is there");
    assert_eq!(value, json!(false), "and it is cleared");
    assert!(!clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is not broken");
}

/// The conversion takes the **alternates** and the **OPI** away.
#[test]
fn the_conversion_takes_the_alternates_and_the_opi_away() {
    let mut session = session();
    an_image(&mut session, "taken", json!([["Alternates", { "ref": [3, 0] }], ["OPI", { "ref": [3, 0] }]]));
    session.execute("pdf.convert", &json!({})).expect("the file is converted");
    let entries = entries_of(&session);
    assert!(!has_key(&entries, "Alternates"), "the alternates are gone");
    assert!(!has_key(&entries, "OPI"), "the OPI information is gone");
    assert!(!clauses_of(&mut session).contains(&"6.2.8".to_string()), "the rule is not broken");
}

/// The conversion says what it **fixed**.
#[test]
fn the_conversion_says_what_it_fixed() {
    let mut session = session();
    an_image(&mut session, "said", json!([["Interpolate", true]]));
    let outcome = session.execute("pdf.convert", &json!({})).expect("the file is converted");
    let fixed = outcome.data["fixed"].as_array().expect("the fixes are a list");
    assert!(
        fixed.iter().any(|entry| entry.as_str().unwrap_or_default().contains("image")),
        "the conversion says so: {fixed:?}"
    );
}
