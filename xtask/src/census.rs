//! `cargo xtask census` — what an arc costs, measured from the history.
//!
//! # The question this answers
//!
//! The survey measured what the v2 work cost **in the session that made
//! it**: the calls, the round trips, the machine time. Those numbers
//! describe a *model*, and a different model writes different numbers.
//!
//! The other half is a property of the **tree**: how big an arc is, how
//! many files it touches, how long it takes from one arc to the next. That
//! half generalises — it says how big an arc is, whatever wrote it — and it
//! is measured from the **git history**, which the tree owns.
//!
//! ```text
//! cargo xtask census
//! ```
//!
//! # What it reports
//!
//! - **the scope** — the commits it measured, and why those commits;
//! - **the arcs** — how many there were, and how big: the lines and the
//!   files each one touched, the median and the largest;
//! - **the gap** — the time between one arc and the next;
//! - **the tree** — the commands each battery carries, and the commands
//!   each file carries.
//!
//! # The scope, and why it is a fact of the tree
//!
//! The repository holds the v1 tree's arcs as well, and their § numbers
//! mean other things. So the census starts at **the commit that created
//! the v2 tree** — found by asking git which commit added `v2/Cargo.toml`,
//! which is a fact and not a number somebody remembered. When that commit
//! cannot be found, the census says so and measures the whole history
//! rather than quietly measuring the wrong thing.

use std::path::Path;

use crate::index::Index;

/// The separator between the parts of a log line.
///
/// **Not a newline and not a tab:** a subject may contain both, and a
/// parser that guesses where a subject ends is a parser that will be
/// wrong one day.
const SEPARATOR: char = '\u{1f}';

/// The path whose **first** commit is the v2 tree's beginning.
const THE_V2_TREE: &str = "v2/Cargo.toml";

/// One arc: a commit, and what it touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arc {
    /// The commit's hash.
    pub hash: String,
    /// When it was made, in seconds since the epoch.
    pub when: u64,
    /// Its subject.
    pub subject: String,
    /// The lines it added.
    pub added: usize,
    /// The lines it removed.
    pub removed: usize,
    /// The files it touched.
    pub files: usize,
    /// The § it belongs to, when it names one.
    pub section: Option<String>,
}

impl Arc {
    /// How big the arc was: the lines it added and removed.
    pub fn size(&self) -> usize {
        self.added + self.removed
    }
}

/// The census of a history: the arcs, and what they say.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Census {
    /// The arcs, oldest first.
    pub arcs: Vec<Arc>,
    /// The commit the census started at, when it found one.
    pub since: Option<String>,
    /// The commits it measured, arcs or not.
    pub commits: usize,
}

/// The census of the tree's history.
///
/// The **scope** is the v2 tree's own history: the commit that created
/// it, found by asking git which commit added `v2/Cargo.toml`.
pub fn census(root: &Path) -> Result<Census, String> {
    let since = the_first_v2_commit(root)?;
    let range = match &since {
        Some(hash) => format!("{hash}..HEAD"),
        None => "HEAD".to_string(),
    };
    let log = git(root, &["log", "--reverse", "--numstat", "--format=%H%x1f%ct%x1f%s", &range])?;
    Ok(parse(&log, since))
}

/// The commit that created the v2 tree, if the history has one.
fn the_first_v2_commit(root: &Path) -> Result<Option<String>, String> {
    let log = git(root, &["log", "--diff-filter=A", "--format=%H", "--", THE_V2_TREE])?;
    Ok(log.lines().last().map(|line| line.trim().to_string()))
}

/// The arcs of a log, oldest first.
///
/// The log is git's own: a header line per commit, then the numbers of
/// the files it touched.
pub fn parse(log: &str, since: Option<String>) -> Census {
    let mut census = Census { since, ..Census::default() };
    let mut current: Option<Arc> = None;
    for line in log.lines() {
        let mut parts = line.split(SEPARATOR);
        let hash = parts.next().unwrap_or("").trim().to_string();
        let when = parts.next().unwrap_or("").trim().parse::<u64>().unwrap_or(0);
        let subject = parts.next().unwrap_or("").trim().to_string();
        if hash.len() == 40 && !subject.is_empty() && parts.next().is_none() {
            if let Some(arc) = current.take() {
                census.arcs.push(arc);
            }
            census.commits += 1;
            current = Some(Arc {
                hash,
                when,
                section: the_section_of(&subject),
                subject,
                added: 0,
                removed: 0,
                files: 0,
            });
            continue;
        }
        // A numstat line: the added, the removed and the file.
        let cells: Vec<&str> = line.split('\t').collect();
        if cells.len() < 3 {
            continue;
        }
        let Some(arc) = current.as_mut() else { continue };
        let added = cells[0].trim().parse::<usize>().unwrap_or(0);
        let removed = cells[1].trim().parse::<usize>().unwrap_or(0);
        arc.added += added;
        arc.removed += removed;
        arc.files += 1;
    }
    if let Some(arc) = current {
        census.arcs.push(arc);
    }
    census
}

/// The § a subject names, if it names one.
pub fn the_section_of(subject: &str) -> Option<String> {
    let at = subject.find('§')?;
    let rest = &subject[at + '§'.len_utf8()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    Some(digits)
}

/// The middle of a list of numbers.
///
/// The **median**, and not the mean: one arc of five thousand lines would
/// drag a mean away from what most arcs look like.
pub fn median(values: &mut [usize]) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        return Some(values[middle]);
    }
    Some((values[middle - 1] + values[middle]) / 2)
}

/// The report of a census.
pub fn report(root: &Path, census: &Census, index: &Index) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("   the scope\n");
    match &census.since {
        Some(hash) => {
            out.push_str(&format!("     the v2 tree's own history, from {}\n", &hash[..hash.len().min(8)]))
        }
        None => {
            out.push_str("     the whole history: the commit that created the v2 tree could not be found\n")
        }
    }
    out.push_str(&format!(
        "     {} commits, of which {} name a section\n",
        census.commits,
        census.arcs.iter().filter(|arc| arc.section.is_some()).count()
    ));

    let sizes: Vec<usize> = census.arcs.iter().map(|arc| arc.size()).collect();
    let files: Vec<usize> = census.arcs.iter().map(|arc| arc.files).collect();
    let mut sizes_sorted = sizes.clone();
    let mut files_sorted = files.clone();
    out.push_str("\n   an arc\n");
    out.push_str(&format!("     the median size: {} lines\n", median(&mut sizes_sorted).unwrap_or(0)));
    out.push_str(&format!("     the median files: {}\n", median(&mut files_sorted).unwrap_or(0)));
    out.push_str(&format!("     the largest: {} lines\n", sizes.iter().copied().max().unwrap_or(0)));
    if let Some(largest) = census.arcs.iter().max_by_key(|arc| arc.size()) {
        out.push_str(&format!("       {}\n", largest.subject));
        out.push_str(&format!("       {} lines over {} files\n", largest.size(), largest.files));
    }
    let total: usize = sizes.iter().sum();
    out.push_str(&format!("     the whole history: {total} lines\n"));

    out.push_str("\n   the gap between two arcs\n");
    let gap = the_median_gap(census);
    out.push_str(&format!("     the median: {} {}\n", gap, if gap == 1 { "minute" } else { "minutes" }));

    out.push_str("\n   the tree\n");
    out.push_str("     the crates, and the commands each one carries\n");
    for battery in batteries(index) {
        out.push_str(&format!("     {battery:<10} {} commands\n", commands_of(index, &battery)));
    }
    out.push_str(&format!("     the median commands in a file: {}\n", the_median_per_file(index)));
    out.push_str(&format!("     the commands in all: {}\n", index.commands.len()));
    let _ = root;
    Ok(out)
}

/// The batteries the tree has, sorted.
fn batteries(index: &Index) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for command in &index.commands {
        if !names.contains(&command.battery) {
            names.push(command.battery.clone());
        }
    }
    names.sort();
    names
}

/// How many commands a battery carries.
fn commands_of(index: &Index, battery: &str) -> usize {
    index.commands.iter().filter(|command| command.battery == battery).count()
}

/// The median number of commands in a file.
///
/// **The file, not the command.** A battery that keeps its commands in one
/// file and a battery that spreads them over twenty are different trees,
/// and the count per file is what says which.
fn the_median_per_file(index: &Index) -> usize {
    let mut per_file: Vec<(String, usize)> = Vec::new();
    for command in &index.commands {
        let name = command.file.display().to_string();
        match per_file.iter_mut().find(|(other, _)| *other == name) {
            Some((_, count)) => *count += 1,
            None => per_file.push((name, 1)),
        }
    }
    let mut counts: Vec<usize> = per_file.iter().map(|(_, count)| *count).collect();
    median(&mut counts).unwrap_or(0)
}

/// The median gap between two arcs, in minutes.
///
/// **Minutes, not hours:** most arcs of this tree were made within the
/// hour of the one before, and an hour is too coarse a unit to say so.
fn the_median_gap(census: &Census) -> u64 {
    let mut gaps: Vec<usize> = Vec::new();
    let mut previous: Option<u64> = None;
    for arc in &census.arcs {
        if let Some(before) = previous {
            gaps.push(((arc.when.saturating_sub(before)) / 60) as usize);
        }
        previous = Some(arc.when);
    }
    median(&mut gaps).unwrap_or(0) as u64
}

/// Run git in the tree, and give its output back.
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = crate::command("git", args, root)?;
    if output.code != 0 {
        return Err(format!("git {} failed: {}", args.join(" "), output.stderr.trim()));
    }
    Ok(output.stdout)
}

/// Run the command.
pub fn run(root: &Path, _args: &[String]) -> Result<crate::Outcome, String> {
    let census = census(root)?;
    let index = crate::index::build(root)?;
    crate::step("census");
    print!("{}", report(root, &census, &index)?);
    Ok(crate::Outcome::Ran)
}
