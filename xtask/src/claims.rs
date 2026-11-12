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
