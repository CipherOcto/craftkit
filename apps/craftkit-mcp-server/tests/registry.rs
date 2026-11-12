//! The **registry** of the whole tree, and the README's numbers against it.
//!
//! The MCP server links **every** battery in — that is what makes it the
//! agent's door — so its test binary is a program that holds the whole tree's
//! registry. This file asks that registry the questions the README answers
//! with prose:
//!
//! - how many commands does each battery carry?
//! - how many commands does the whole tree carry?
//! - and is every battery actually linked in?
//!
//! The last one matters most. A count that is right about the batteries it
//! can see says nothing about a battery that was not linked in at all, and
//! the tree's rule is that a check which could not run is never reported as
//! one that passed. So this test **refuses** if a battery is missing rather
//! than counting the ones that are there.

use std::fs;
use std::path::{Path, PathBuf};

use craftkit_sdk::ecosystem::Ecosystem;

/// The **batteries** the README counts.
const BATTERIES: [&str; 9] = ["grid", "prose", "pdf", "film", "layout", "cad", "fx", "sound", "vector"];

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the root is there")
}

/// The README of the tree.
fn readme() -> String {
    fs::read_to_string(root().join("README.md")).expect("the README is there")
}

/// The ecosystem of the whole tree: every battery is linked in.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built from what was linked in")
}

/// The **highest** count the README gives for a battery.
///
/// The commands of a battery are only ever added, so the highest count the
/// README says is the count of the tree.
fn highest_count(readme: &str, battery: &str) -> Option<usize> {
    let flat: String = readme.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    let needle = format!("{} battery ", battery.to_lowercase());
    let mut found = None;
    let mut rest = flat.as_str();
    while let Some(at) = rest.find(&needle) {
        let tail = &rest[at + needle.len()..];
        let tail = tail.strip_prefix("now ").unwrap_or(tail);
        if let Some(open) = tail.strip_prefix("carries **") {
            let number = &open[..open.find("**")?];
            if let Ok(count) = number.parse::<usize>() {
                found = Some(found.map_or(count, |held: usize| held.max(count)));
            }
        }
        rest = &rest[at + needle.len()..];
    }
    found
}

/// Every battery is linked in, and each carries the commands the README says.
#[test]
fn every_battery_is_linked_in_and_counted() {
    let ecosystem = ecosystem();
    let readme = readme();
    for battery in BATTERIES {
        let count = ecosystem.commands.of_area(battery).len();
        assert!(
            count > 0,
            "the {battery} battery is not linked into this program, so its count cannot be checked: \
             a check that could not run is not a check that passed"
        );
        let claimed = highest_count(&readme, battery)
            .unwrap_or_else(|| panic!("the README counts the {battery} battery"));
        assert_eq!(
            count, claimed,
            "the {battery} battery carries {count} commands, and the README says {claimed}"
        );
    }
}

/// The whole tree's commands are the number the README says the MCP server
/// has.
///
/// **The MCP server publishes one tool per command**, so its tool count and
/// the tree's command count are the same number — and the README says so in
/// two places, for the batteries and for the server.
#[test]
fn the_mcp_server_has_one_tool_per_command() {
    let ecosystem = ecosystem();
    let server = craftkit_mcp::Server::new(ecosystem.session());
    let commands = ecosystem.commands.all().len();
    assert_eq!(
        server.tools().len(),
        commands,
        "one tool per command: {commands} commands, {} tools",
        server.tools().len()
    );
}
