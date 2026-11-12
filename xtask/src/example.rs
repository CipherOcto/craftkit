//! `cargo xtask example` — what a command of this battery looks like.
//!
//! # The question this answers
//!
//! "How is a command written in the grid battery?" was the **largest** of
//! the searches an arc made: three greps and a read, in the worst case
//! eleven calls, to find an example of the shape and copy it.
//!
//! The answer is **in the battery**, not in a file of prose: one command per
//! battery is marked `example`, and the marker is part of the declaration —
//! so the compiler checks that it is there and the reader of the source can
//! see it.
//!
//! ```text
//! cargo xtask example --battery grid
//! ```
//!
//! # What it prints
//!
//! - the **example command**, in full: its documentation, its attribute and
//!   its struct, as they are written;
//! - the **neighbouring commands** of the same battery, so the reader sees
//!   that a battery is not one command;
//! - the **test files** of the battery, and the first test of the first one,
//!   because a command without its test is half an arc.

use std::path::Path;

use crate::index::{self, Index};

/// The example command of a battery, if it has marked one.
pub fn example_of<'a>(index: &'a Index, battery: &str) -> Option<&'a index::Command> {
    index.commands.iter().find(|c| c.battery == battery && c.example)
}

/// The source of a command: its documentation, its attribute and its struct.
///
/// **The text as it is written**, so that a reader can copy it.
pub fn the_source(root: &Path, command: &index::Command) -> Result<String, String> {
    let path = root.join(&command.file);
    let text = crate::tree::read_rust(&path, crate::tree::Subject::Everything)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let lines: Vec<&str> = text.lines().collect();
    // The declaration starts at the attribute and runs to the end of the
    // struct: the line with the closing brace of the struct's fields.
    let start = command.line.saturating_sub(1);
    let mut end = start;
    let mut depth = 0usize;
    let mut seen = false;
    for (at, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            if c == '{' {
                depth += 1;
                seen = true;
            } else if c == '}' {
                depth -= 1;
            }
        }
        if seen && depth == 0 {
            end = at;
            break;
        }
        end = at;
    }
    // The documentation is above the derive, which is above the attribute.
    let mut first = start;
    while first > 0 {
        let line = lines[first - 1].trim();
        if line.is_empty() || line.starts_with("#[") {
            first -= 1;
            continue;
        }
        if line.starts_with("///") {
            first -= 1;
            continue;
        }
        break;
    }
    Ok(lines[first..=end.min(lines.len().saturating_sub(1))].join("\n"))
}

/// The first test of a battery: its name and its file.
///
/// The test is the other half of a command, and a reader who is writing one
/// needs to see how the tree tests one.
pub fn the_first_test(index: &Index, battery: &str) -> Option<(String, String)> {
    let tests = index.tests.get(battery)?;
    for test in tests {
        let text = crate::tree::read_rust(test, crate::tree::Subject::Everything).ok()?;
        for (number, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.starts_with("fn ") && !trimmed.starts_with("async fn ") {
                continue;
            }
            let name = trimmed
                .trim_start_matches("async fn ")
                .trim_start_matches("fn ")
                .split('(')
                .next()?
                .to_string();
            return Some((name, format!("{}:{}", test.display(), number + 1)));
        }
    }
    None
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let battery = battery_of(args)?;
    let index = index::build(root)?;
    crate::step("example");
    let Some(command) = example_of(&index, &battery) else {
        return Err(format!(
            "the `{battery}` battery has not marked an example: a battery that has not said what one \
             of its commands looks like cannot show one"
        ));
    };
    println!("   the example of the `{battery}` battery is `{}`", command.id);
    println!("   {}", command.where_());
    println!();
    println!("{}", the_source(root, command)?);
    println!();
    let neighbours: Vec<&index::Command> =
        index.commands.iter().filter(|c| c.battery == battery && c.id != command.id).take(5).collect();
    if !neighbours.is_empty() {
        println!("   and the battery's other commands:");
        for neighbour in neighbours {
            println!("     {:<34} {}", neighbour.id, neighbour.where_());
        }
        println!();
    }
    if let Some((name, where_)) = the_first_test(&index, &battery) {
        println!("   and a test of it:");
        println!("     {name}");
        println!("     {where_}");
    }
    Ok(crate::Outcome::Ran)
}

/// The battery the command was asked about.
fn battery_of(args: &[String]) -> Result<String, String> {
    let mut battery: Option<String> = None;
    let mut at = 0;
    while at < args.len() {
        match args[at].as_str() {
            "--battery" => {
                at += 1;
                battery = Some(
                    args.get(at)
                        .cloned()
                        .ok_or_else(|| "--battery needs the name of a battery".to_string())?,
                );
            }
            other => return Err(format!("unknown flag `{other}`: try --battery")),
        }
        at += 1;
    }
    battery.ok_or_else(|| "example needs a battery: try --battery grid".to_string())
}
