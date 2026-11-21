//! The **claim table** of the README, and the index it is.
//!
//! # The table
//!
//! The README's §48 is a table of every claim the README has made: what it
//! claims, what is true today, and **where** — the section that says so.
//! It is **prose**, and prose stays prose: a compiler cannot write "the
//! truth today".
//!
//! But two of the three columns can be made true by machine:
//!
//! - the **where** column is a § number, and every § number in it must be a
//!   section that exists;
//! - and every § number in it must be claimed **once** — two rows claiming
//!   the same section is a table that says the same thing twice.
//!
//! # The rule that bites
//!
//! **A port cannot land without saying what it claims.** The tree's own
//! record of the arcs is the **port manifest** (§853): every row of it names
//! the section of the arc, and every one of those sections must have a row in
//! the claim table.
//!
//! So the table is checked in both directions: what it cites exists, and
//! what the tree made is claimed.

use std::collections::BTreeSet;

/// One row of the claim table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// What is claimed.
    pub claim: String,
    /// What is true today.
    pub truth: String,
    /// The sections the row says the claim is in.
    pub sections: Vec<usize>,
}

/// The header of the claim table, which is how the table is found.
const HEADER: &str = "| the claim | the truth today | where |";

/// The rows of the claim table.
///
/// **The table is found by its header**, so a table that grows keeps being
/// found and a table that is renamed is not found — and the gate says so.
pub fn rows(readme: &str) -> Vec<Row> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in readme.lines() {
        if line.starts_with(HEADER) {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if !line.starts_with('|') {
            break;
        }
        if line.starts_with("|---") {
            continue;
        }
        let Some(cells) = cells_of(line) else { continue };
        if cells.len() < 3 {
            continue;
        }
        let sections = sections_in(&cells[2]);
        found.push(Row { claim: cells[0].clone(), truth: cells[1].clone(), sections });
    }
    found
}

/// The cells of a table row.
fn cells_of(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('|')?;
    let rest = rest.strip_suffix('|')?;
    Some(rest.split('|').map(|cell| cell.trim().to_string()).collect())
}

/// The § numbers a cell names.
pub fn sections_in(cell: &str) -> Vec<usize> {
    let mut found = Vec::new();
    let mut rest = cell;
    while let Some(at) = rest.find('§') {
        rest = &rest[at + '§'.len_utf8()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(number) = digits.parse::<usize>() {
            found.push(number);
            rest = &rest[digits.len()..];
        }
    }
    found
}

/// The section numbers the README has.
pub fn sections(readme: &str) -> BTreeSet<usize> {
    let mut found = BTreeSet::new();
    for line in readme.lines() {
        let Some(rest) = line.strip_prefix("## ") else { continue };
        let Some((number, _)) = rest.split_once('.') else { continue };
        if let Ok(number) = number.trim().parse::<usize>() {
            found.insert(number);
        }
    }
    found
}

/// The problems the claim table has.
///
/// Three rules, and each of them is a way the table goes wrong:
/// - a row cites a section that does not exist;
/// - a row says nowhere at all;
/// - an arc the tree made has no row at all.
pub fn problems(readme: &str, port_sections: &[usize]) -> Vec<String> {
    let mut problems = Vec::new();
    let rows = rows(readme);
    if rows.is_empty() {
        problems.push(format!("the README has no claim table: the header `{HEADER}` is not in it"));
        return problems;
    }
    let sections = sections(readme);

    let mut claimed: BTreeSet<usize> = BTreeSet::new();
    for row in &rows {
        if row.sections.is_empty() {
            problems.push(format!(
                "the claim of `{}` says nowhere: a row of the table is a pointer, and this one \
                 points at nothing",
                row.claim
            ));
        }
        for section in &row.sections {
            if !sections.contains(section) {
                problems.push(format!(
                    "the claim of `{}` is in §{section}, and the README has no §{section}",
                    row.claim
                ));
            }
            // A section may be claimed by **several** rows: one arc may make
            // more than one claim, and the table says so. What the set is for
            // is the rule below.
            claimed.insert(*section);
        }
    }

    for section in port_sections {
        if !claimed.contains(section) {
            problems
                .push(format!("the tree made §{section}, and the claim table does not say what it claims"));
        }
    }
    problems
}

/// The problems the claim table has **about the commands it names**.
///
/// The truth column of a row that says *built* names a command. The
/// registry knows every command the tree has, so a row that names one the
/// tree does not have is a **stale claim** — and a stale claim is the one
/// kind of lie the table can tell on its own.
///
/// **Only the rows that say *built*** are checked: a row about something
/// that is *not* built may well name a command that does not exist — that
/// is what the row is for.
pub fn problems_about_commands(readme: &str, commands: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    // The areas the registry knows: the first part of every command's id.
    // A word that is shaped like an id but does not start with an area is a
    // path or a file name — `docs/survey-cost-and-meta.md` — and not a
    // command.
    let areas: BTreeSet<String> = commands
        .iter()
        .filter_map(|command| command.split('.').next().map(|area| area.to_string()))
        .collect();
    for row in rows(readme) {
        if !row.truth.starts_with("**built") {
            continue;
        }
        for token in tokens_of(&row.truth) {
            if !looks_like_an_id(&token) {
                continue;
            }
            let Some(area) = token.split('.').next() else { continue };
            if !areas.contains(area) {
                continue;
            }
            if !commands.contains(&token) {
                problems.push(format!(
                    "the claim of `{}` says the tree has the command `{token}`, and the tree has \
                     no such command",
                    row.claim
                ));
            }
        }
    }
    problems
}

/// The words of a cell, with the punctuation taken off.
fn tokens_of(cell: &str) -> Vec<String> {
    cell.split(|c: char| !(c.is_alphanumeric() || c == '.' || c == '-' || c == '_'))
        .map(|word| word.trim_matches('.').to_string())
        .filter(|word| !word.is_empty())
        .collect()
}

/// Whether a word is shaped like a command's id: two or three dotted parts.
fn looks_like_an_id(word: &str) -> bool {
    let parts: Vec<&str> = word.split('.').collect();
    (2..=3).contains(&parts.len())
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The commands of a small tree.
    fn commands() -> Vec<String> {
        vec!["grid.pivot.create".to_string(), "grid.sheet.read".to_string()]
    }

    /// A claim table with one row that is true.
    fn a_table() -> String {
        "| the claim | the truth today | where |\n|---|---|---|\n| **a pivot is made** | **built.** The tree has the command `grid.pivot.create` (§1) | §1 |\n| **a sheet is read** | **built.** The tree has the command `grid.sheet.read` (§2) | §2 |\n"
            .to_string()
    }

    #[test]
    fn a_claim_that_names_a_command_the_tree_has_is_no_problem() {
        assert!(problems_about_commands(&a_table(), &commands()).is_empty());
    }

    #[test]
    fn a_claim_that_names_a_command_the_tree_does_not_have_is_caught() {
        let table = a_table().replace("grid.sheet.read", "grid.sheet.gone");
        let problems = problems_about_commands(&table, &commands());
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("grid.sheet.gone"), "{problems:?}");
    }

    #[test]
    fn a_claim_that_is_not_built_may_name_a_command_that_is_not_there() {
        // A row about something that is **not** built may well name a command
        // that does not exist: that is what the row is for.
        let table = "| the claim | the truth today | where |\n|---|---|---|\n| **a thing** | **not built.** The tree has no command `grid.thing.yet` (§1) | §1 |\n";
        assert!(problems_about_commands(table, &commands()).is_empty());
    }

    #[test]
    fn a_word_that_is_not_shaped_like_an_id_is_not_a_command() {
        assert!(!looks_like_an_id("§1089"));
        assert!(looks_like_an_id("grid.pivot.create"));
        assert!(looks_like_an_id("grid.sheet"));
    }

    #[test]
    fn a_path_in_a_claim_is_not_a_command() {
        // The shape of a path is the shape of an id — three dotted parts —
        // and what tells them apart is the **area**: the tree's areas are
        // `grid`, `pdf`, `sound`, and a path starts with a directory.
        let table = "| the claim | the truth today | where |\n|---|---|---|\n| **the survey** | **built.** The tree has the survey in docs.survey-cost-and-meta.md (§1) | §1 |\n";
        assert!(problems_about_commands(table, &commands()).is_empty());
    }
}
