//! `cargo xtask brief` — everything one arc needs, in one call.
//!
//! # The question this answers
//!
//! An arc needs six things before it writes a line, and each of them was a
//! search:
//!
//! 1. **the rule** — what it is, and where it was ported from;
//! 2. **the landing site** — which module, which file, and what is already
//!    there;
//! 3. **the conventions** — how a command of this battery is written, and
//!    how it is tested;
//! 4. **the claims** — the row to add to the claim table, the § number to
//!    use, and the counts of the battery and of the tree;
//! 5. **the commands** — the exact checks to run, and the exact check to
//!    run at the end;
//! 6. **the counts** — how many commands the battery carries now.
//!
//! That was eleven calls and about half an arc's time. This is one call and
//! one page.
//!
//! ```text
//! cargo xtask brief --battery grid --id grid.workbook.add
//! ```

use std::path::Path;

use crate::index::{self, Index};

/// The brief of one arc.
pub struct Brief {
    /// The battery the arc is working in.
    pub battery: String,
    /// The command the arc is adding.
    pub id: String,
    /// The rule, and where it came from.
    pub rule: Option<(String, String)>,
    /// The file the command lands in.
    pub landing: Option<String>,
    /// The commands already in the battery.
    pub neighbours: Vec<String>,
    /// The example command of the battery.
    pub example: Option<String>,
    /// The example test of the battery.
    pub test: Option<String>,
    /// The § number the arc's section takes.
    pub section: String,
    /// The counts of the tree.
    pub counts: Vec<(String, usize)>,
}

/// The § number the next section takes.
///
/// **The highest the README has, plus one.** The numbers are the README's
/// own, so the number an arc takes is the number the README is about to
/// have.
pub fn next_section(readme: &str) -> String {
    let mut highest = 0usize;
    for line in readme.lines() {
        let Some(rest) = line.strip_prefix("## ") else { continue };
        let Some((number, _)) = rest.split_once('.') else { continue };
        if let Ok(number) = number.trim().parse::<usize>() {
            highest = highest.max(number);
        }
    }
    format!("§{}", highest + 1)
}

/// Build the brief of an arc.
pub fn build(root: &Path, index: &Index, battery: &str, id: &str) -> Result<Brief, String> {
    let readme = crate::tree::read_text(root.join("README.md"))
        .map_err(|e| format!("could not read the README: {e}"))?;
    let rule = crate::ports::PORTS
        .iter()
        .rev()
        .find(|port| port.command == id || port.to == battery)
        .map(|port| (port.rule.to_string(), port.from.to_string()));
    let landing =
        index.commands.iter().find(|c| c.id == id).map(|c| format!("{}:{}", c.file.display(), c.line));
    let neighbours: Vec<String> = index
        .commands
        .iter()
        .filter(|c| c.battery == battery)
        .take(6)
        .map(|c| format!("{:<34} {}", c.id, c.where_()))
        .collect();
    let example = crate::example::example_of(index, battery).map(|c| format!("{} · {}", c.id, c.where_()));
    let test =
        crate::example::the_first_test(index, battery).map(|(name, where_)| format!("{name} · {where_}"));
    let counts = crate::counts::counts(root)?;
    Ok(Brief {
        battery: battery.to_string(),
        id: id.to_string(),
        rule,
        landing,
        neighbours,
        example,
        test,
        section: next_section(&readme),
        counts,
    })
}

/// Print the brief.
pub fn print(brief: &Brief) {
    println!("── the brief of {} in the {} battery", brief.id, brief.battery);
    println!();
    println!("the rule");
    match &brief.rule {
        Some((rule, from)) => println!("   {rule}\n   from: {from}"),
        None => println!("   (not in the port manifest yet — add a row when the port is made)"),
    }
    println!();
    println!("the landing site");
    match &brief.landing {
        Some(landing) => println!("   {landing}"),
        None => println!("   (the command is new — the module is the battery's own)"),
    }
    if !brief.neighbours.is_empty() {
        println!("   the battery's commands:");
        for neighbour in &brief.neighbours {
            println!("     {neighbour}");
        }
    }
    println!();
    println!("the conventions");
    match &brief.example {
        Some(example) => println!("   the example: {example}"),
        None => println!("   (the battery has not marked an example)"),
    }
    match &brief.test {
        Some(test) => println!("   a test:     {test}"),
        None => println!("   (the battery has no test file)"),
    }
    println!();
    println!("the claims");
    println!("   the section: {} (README section {})", brief.section, brief.section.trim_start_matches('§'));
    println!("   the claim row goes in the table of §48, and says what the port is");
    println!("   the counts of the tree:");
    for (battery, count) in &brief.counts {
        println!("     {battery:<10} {count}");
    }
    println!();
    println!("the commands");
    println!("   cargo xtask verify --changed");
    println!("   cargo xtask ci");
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let (battery, id) = battery_and_id(args)?;
    let index = index::build(root)?;
    let brief = build(root, &index, &battery, &id)?;
    print(&brief);
    Ok(crate::Outcome::Ran)
}

/// The battery and the command the brief is for.
fn battery_and_id(args: &[String]) -> Result<(String, String), String> {
    let mut battery: Option<String> = None;
    let mut id: Option<String> = None;
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
            "--id" => {
                at += 1;
                id = Some(args.get(at).cloned().ok_or_else(|| "--id needs the id of a command".to_string())?);
            }
            other => return Err(format!("unknown flag `{other}`: try --battery and --id")),
        }
        at += 1;
    }
    let battery = battery.ok_or_else(|| "brief needs a battery: try --battery grid".to_string())?;
    let id = id.ok_or_else(|| "brief needs a command: try --id grid.workbook.add".to_string())?;
    Ok((battery, id))
}
