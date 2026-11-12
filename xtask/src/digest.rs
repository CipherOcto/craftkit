//! `cargo xtask digest` — the conventions of the house, and of a battery.
//!
//! # The question this answers
//!
//! An arc keeps re-learning the same things: the deny list, the refusal
//! style, the shape of a commit message, the four headings a README
//! section has. Every one of them is written down somewhere — and every
//! one of them was **found** by searching, arc after arc, because there
//! was no single place to look.
//!
//! So there is one now, and it is **generated**: the digest reads the
//! rules out of the tree and prints where each one is written.
//!
//! ```text
//! cargo xtask digest --house          # the rules of the house
//! cargo xtask digest --battery grid   # the conventions of one battery
//! ```
//!
//! # Why the house digest is a gate
//!
//! A digest that prints a rule the tree no longer has is worse than no
//! digest: it is a rule that has been **retired** and is still being
//! obeyed. So the house digest **fails** when a rule it states cannot be
//! found where it says it is — a rule that has gone is a failure, and the
//! failure names the rule.
//!
//! That makes `cargo xtask digest` a phase of the gate, and it costs one
//! file read per rule.

use std::path::Path;

use crate::index::Index;

/// One item of the house digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    /// A rule, **quoted** from a file of the tree.
    ///
    /// The marker is the phrase that finds it; the quote is the marker and
    /// the lines that belong to it.
    Quoted {
        /// What the rule is.
        title: &'static str,
        /// The file it is written in, relative to the root.
        file: &'static str,
        /// The phrase that finds it.
        marker: &'static str,
    },
    /// The headings a README section has.
    ///
    /// **Generated**: the headings are read out of the README's last
    /// section, and the four are the house's four.
    SectionHeadings,
}

/// The rules of the house, and where each one is written.
///
/// **Every marker is a phrase that exists.** The digest is a gate, and a
/// marker that cannot be found is a rule that has gone.
pub const HOUSE: &[Item] = &[
    Item::Quoted { title: "Never crash", file: "CONTRIBUTING.md", marker: "**Never crash.**" },
    Item::Quoted {
        title: "The deny list",
        file: "crates/craftkit-gates/src/lib.rs",
        marker: "pub const LINTS",
    },
    Item::Quoted { title: "The refusal style", file: "AGENTS.md", marker: "**Errors are `Result<T, E>`**" },
    Item::Quoted { title: "The two trust boundaries", file: "AGENTS.md", marker: "`craftkit_sdk::guard`" },
    Item::Quoted {
        title: "Everything is a command",
        file: "CONTRIBUTING.md",
        marker: "**Everything is a command.**",
    },
    Item::Quoted {
        title: "The shape of a command",
        file: "AGENTS.md",
        marker: "dotted, lower case, no underscores",
    },
    Item::Quoted { title: "The layering holds", file: "CONTRIBUTING.md", marker: "**The layering holds.**" },
    Item::Quoted { title: "The layers", file: "AGENTS.md", marker: "L0 → L1 → L2 → L3 → L4 → L5" },
    Item::Quoted {
        title: "A plugin is a plugin",
        file: "AGENTS.md",
        marker: "**A plugin may not depend on an app or on another plugin.**",
    },
    Item::Quoted { title: "The UI is thin", file: "CONTRIBUTING.md", marker: "**The UI is thin.**" },
    Item::Quoted {
        title: "The rules of the modelling",
        file: "CONTRIBUTING.md",
        marker: "**The rules of the modelling.**",
    },
    Item::Quoted {
        title: "A node may have no place",
        file: "AGENTS.md",
        marker: "**A node may have no place.**",
    },
    Item::Quoted { title: "Refuse, do not clamp", file: "AGENTS.md", marker: "**Refuse, do not clamp.**" },
    Item::Quoted {
        title: "Every rule has a test",
        file: "CONTRIBUTING.md",
        marker: "**Every rule has a test.**",
    },
    Item::Quoted { title: "The clean room", file: "CONTRIBUTING.md", marker: "**Independent.**" },
    Item::Quoted { title: "No assets", file: "CONTRIBUTING.md", marker: "**No assets.**" },
    Item::Quoted { title: "Rust only", file: "CONTRIBUTING.md", marker: "**Rust only.**" },
    Item::Quoted {
        title: "The shape of a commit message",
        file: "CONTRIBUTING.md",
        marker: "feat(<scope>): <what it does> — §NNN",
    },
    Item::Quoted {
        title: "The message may not carry a shell",
        file: "CONTRIBUTING.md",
        marker: "may not contain a **backtick**",
    },
    Item::Quoted {
        title: "The prose goes in its own commit",
        file: "CONTRIBUTING.md",
        marker: "the prose and the code are reviewed by different eyes",
    },
    Item::Quoted {
        title: "The shared machine",
        file: "CONTRIBUTING.md",
        marker: "**This is a shared machine.**",
    },
    Item::Quoted {
        title: "The claim row's shape",
        file: "README.md",
        marker: "| the claim | the truth today | where |",
    },
    Item::SectionHeadings,
];

/// The four headings a README section has.
///
/// **The house's four**, and the README's last section is where they are
/// checked: a section that has three headings is a section that has
/// forgotten one.
pub const SECTION_HEADINGS: &[&str] =
    &["What was built", "What the port found", "What it cost", "What it does not settle"];

/// The house digest, and the rules that have gone.
///
/// The error is the report: every rule that cannot be found, one per line,
/// and the line says which file it was looked for in.
pub fn house(root: &Path) -> Result<String, String> {
    house_of(root, HOUSE)
}

/// The house digest, for a given list of rules.
///
/// The list is a parameter so that a test can ask what happens when a rule
/// has gone — a gate that cannot fail is a gate that is not there.
pub fn house_of(root: &Path, items: &[Item]) -> Result<String, String> {
    let mut text = String::new();
    let mut missing: Vec<String> = Vec::new();
    for item in items {
        match item {
            Item::Quoted { title, file, marker } => {
                let path = root.join(file);
                let Ok(contents) = std::fs::read_to_string(&path) else {
                    missing.push(format!("**{title}** — {file} could not be read"));
                    continue;
                };
                match quote(&contents, marker) {
                    Some((line, quote)) => {
                        text.push_str(&format!("   {title}\n"));
                        text.push_str(&format!("     {file}:{line}\n"));
                        for part in quote.lines() {
                            text.push_str(&format!("       {part}\n"));
                        }
                        text.push('\n');
                    }
                    None => missing.push(format!("**{title}** — `{marker}` is not in {file}")),
                }
            }
            Item::SectionHeadings => {
                let path = root.join("README.md");
                let Ok(contents) = std::fs::read_to_string(&path) else {
                    missing.push("**The headings of a section** — README.md could not be read".to_string());
                    continue;
                };
                let headings = the_last_headings(&contents);
                let missing_one: Vec<&str> =
                    SECTION_HEADINGS.iter().filter(|h| !headings.contains(&h.to_string())).copied().collect();
                if !missing_one.is_empty() {
                    missing.push(format!(
                        "**The headings of a section** — the README's last section has no {}",
                        missing_one.join(", no ")
                    ));
                    continue;
                }
                text.push_str("   The headings of a README section\n");
                text.push_str("     README.md, the last section\n");
                for heading in SECTION_HEADINGS {
                    text.push_str(&format!("       {heading}\n"));
                }
                text.push('\n');
            }
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "the house's rules have gone, and a rule that has gone is still being obeyed:\n{}",
            missing.join("\n")
        ));
    }
    Ok(text)
}

/// The headings of the README's **last** section.
///
/// The last section is the newest one, and the newest one is the one the
/// house's shape is checked against.
pub fn the_last_headings(readme: &str) -> Vec<String> {
    let mut headings = Vec::new();
    for line in readme.lines() {
        if line.starts_with("## ") {
            // A new section: the headings of the one before it are not the
            // last section's headings.
            headings.clear();
            continue;
        }
        if line.starts_with("### ") {
            headings.push(line.trim_start_matches("### ").trim().to_string());
        }
    }
    headings
}

/// The quote of the marker: the line it is on, and the lines that belong
/// to it.
///
/// A line belongs to the marker when it is **indented** — a bullet's
/// continuation, a constant's items — so the quote is the rule and not
/// the file.
fn quote(text: &str, marker: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.iter().position(|line| line.contains(marker))?;
    let mut out = vec![lines[at].trim().to_string()];
    let mut next = at + 1;
    while next < lines.len() {
        let line = lines[next];
        if line.trim().is_empty() || !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        out.push(line.trim().to_string());
        next += 1;
    }
    Some((at + 1, out.join("\n")))
}

/// The digest of a battery: its crate, its commands, its modules and its
/// tests.
///
/// The four are the four an arc needs before it writes a line: where the
/// command goes, what the battery already has, and how the battery's tests
/// are laid out.
pub fn battery(root: &Path, name: &str) -> Result<String, String> {
    let index = crate::index::build(root)?;
    let crate_name = format!("craftkit-plugin-{name}");
    let directory = root.join("crates").join(&crate_name);
    if !directory.is_dir() {
        return Err(format!(
            "the tree has no `{crate_name}` crate: the batteries are the crates named `craftkit-plugin-*`"
        ));
    }
    let mut text = String::new();
    let layer = crate::lists::the_layers_table(root).ok().and_then(|table| table.get(&crate_name).copied());
    text.push_str(&format!("   the `{name}` battery is `{crate_name}`"));
    if let Some(layer) = layer {
        text.push_str(&format!(", and it is L{layer}"));
    }
    text.push('\n');

    let commands: Vec<&crate::index::Command> =
        index.commands.iter().filter(|command| command.battery == name).collect();
    text.push_str(&format!("\n   its commands: {}\n", commands.len()));
    match crate::example::example_of(&index, name) {
        Some(command) => {
            text.push_str(&format!("     its example is `{}`\n", command.id));
            text.push_str(&format!("       {}\n", command.where_()));
        }
        None => text.push_str("     it has not marked an example\n"),
    }

    text.push_str("\n   its modules:\n");
    for file in index.modules.get(name).map(|files| files.as_slice()).unwrap_or(&[]) {
        text.push_str(&format!("     {:<52} {}\n", relative(root, file), lines_of(file)));
    }
    text.push_str("\n   its tests:\n");
    for file in index.tests.get(name).map(|files| files.as_slice()).unwrap_or(&[]) {
        text.push_str(&format!("     {:<52} {}\n", relative(root, file), lines_of(file)));
    }
    Ok(text)
}

/// The path of a file, relative to the root.
fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root).unwrap_or(file).display().to_string()
}

/// How many lines a file has.
fn lines_of(file: &Path) -> usize {
    std::fs::read_to_string(file).map(|text| text.lines().count()).unwrap_or(0)
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let mut battery: Option<String> = None;
    let mut house = true;
    let mut at = 0;
    while at < args.len() {
        match args[at].as_str() {
            "--house" => house = true,
            "--battery" => {
                at += 1;
                let name = args
                    .get(at)
                    .cloned()
                    .ok_or_else(|| "--battery needs the name of a battery".to_string())?;
                battery = Some(name);
                house = false;
            }
            other => return Err(format!("unknown flag `{other}`: try --house or --battery")),
        }
        at += 1;
    }
    if let Some(name) = battery {
        crate::step("digest");
        print!("{}", battery_digest(root, &name)?);
        return Ok(crate::Outcome::Ran);
    }
    if !house {
        return Err("the digest needs a battery: try --battery grid".to_string());
    }
    crate::step("digest");
    print!("{}", house_digest(root)?);
    Ok(crate::Outcome::Ran)
}

/// The house digest, printed.
fn house_digest(root: &Path) -> Result<String, String> {
    let text = house(root)?;
    Ok(format!("   the rules of the house:\n\n{text}"))
}

/// The battery digest, printed.
fn battery_digest(root: &Path, name: &str) -> Result<String, String> {
    let text = battery(root, name)?;
    Ok(format!("   the conventions of the `{name}` battery:\n\n{text}"))
}

/// The index of the tree, for a caller that wants it.
pub fn the_index(root: &Path) -> Result<Index, String> {
    crate::index::build(root)
}
