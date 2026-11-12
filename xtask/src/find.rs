//! `cargo xtask find` — the tree's answers, in one call.
//!
//! # The question this answers
//!
//! Before an arc writes a line, it asks the tree three questions: **where
//! does this command live**, **what else is in this area**, and **what does
//! the tree already say about this**. Each of them was a search — three
//! greps and a read, in the worst case eleven — and each of them has an
//! answer that the tree can give in one call.
//!
//! ```text
//! cargo xtask find id grid.pivot.set      # where does this command live?
//! cargo xtask find area grid              # what is in this area?
//! cargo xtask find battery grid           # what is in this battery?
//! cargo xtask find command pivot          # which commands mention this?
//! cargo xtask find bound MAX_NODES        # what is this bound, and where?
//! cargo xtask find text pivot             # which commands say this?
//! ```
//!
//! Every answer names the **file and the line**, because a name without a
//! place is a name you have to look up again.

use std::path::Path;

use crate::index::{self, Index};

/// The questions `find` answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// A command's id, exactly.
    Id(String),
    /// Every command of an area.
    Area(String),
    /// Everything about a battery.
    Battery(String),
    /// The commands whose id contains the words.
    Command(String),
    /// A bound, by name.
    Bound(String),
    /// The commands whose documentation contains the words.
    Text(String),
}

impl Question {
    /// The question, as it was asked.
    pub fn describe(&self) -> String {
        match self {
            Question::Id(id) => format!("the command `{id}`"),
            Question::Area(area) => format!("the area `{area}`"),
            Question::Battery(battery) => format!("the battery `{battery}`"),
            Question::Command(words) => format!("the commands that mention `{words}`"),
            Question::Bound(name) => format!("the bound `{name}`"),
            Question::Text(words) => format!("the commands that say `{words}`"),
        }
    }
}

/// Read the question out of the arguments.
///
/// **Two words, or one.** The question is what to look for and the words are
/// what to look for it in — `find id grid.pivot.set` is one question with
/// one answer, and `find command pivot` is one question with many.
pub fn parse(args: &[String]) -> Result<Question, String> {
    let Some(what) = args.first() else {
        return Err("find needs a question: id, area, battery, command, bound or text".to_string());
    };
    let words = args[1..].join(" ");
    if words.trim().is_empty() {
        return Err(format!("`find {what}` needs the words to look for"));
    }
    match what.as_str() {
        "id" => Ok(Question::Id(words)),
        "area" => Ok(Question::Area(words)),
        "battery" => Ok(Question::Battery(words)),
        "command" => Ok(Question::Command(words)),
        "bound" => Ok(Question::Bound(words)),
        "text" => Ok(Question::Text(words)),
        other => Err(format!("unknown question `{other}`: id, area, battery, command, bound or text")),
    }
}

/// The answer to a question: the lines a person reads.
pub fn answer(index: &Index, question: &Question) -> Vec<String> {
    match question {
        Question::Id(id) => index
            .commands
            .iter()
            .filter(|c| c.id == id.as_str())
            .map(|c| {
                format!(
                    "{}\n     {} · {}\n     {}",
                    c.id,
                    c.battery,
                    c.doc.clone().unwrap_or_else(|| "(no documentation)".to_string()),
                    c.where_()
                )
            })
            .collect(),
        Question::Area(area) => {
            let commands = index::commands_in_area(index, area);
            let mut lines = vec![format!("{} commands in the area `{area}`:", commands.len())];
            for command in commands {
                lines.push(format!("     {:<34} {}", command.id, command.where_()));
            }
            lines
        }
        Question::Battery(battery) => {
            let commands = index::commands_of(index, battery);
            let modules = index.modules.get(battery).cloned().unwrap_or_default();
            let tests = index.tests.get(battery).cloned().unwrap_or_default();
            let mut lines = vec![format!(
                "the battery `{battery}`: {} commands, {} modules, {} test files",
                commands.len(),
                modules.len(),
                tests.len()
            )];
            for command in commands {
                lines.push(format!("     {:<34} {}", command.id, command.where_()));
            }
            lines.push("     modules:".to_string());
            for module in modules {
                lines.push(format!("       {}", module.display()));
            }
            lines.push("     tests:".to_string());
            for test in tests {
                lines.push(format!("       {}", test.display()));
            }
            lines
        }
        Question::Command(words) => {
            let found: Vec<&index::Command> =
                index.commands.iter().filter(|c| c.id.contains(words)).collect();
            let mut lines = vec![format!("{} commands whose id holds `{words}`:", found.len())];
            for command in found {
                lines.push(format!("     {:<34} {}", command.id, command.where_()));
            }
            lines
        }
        Question::Bound(name) => {
            let found: Vec<&index::Bound> = index.bounds.iter().filter(|b| b.name == *name).collect();
            if found.is_empty() {
                return vec![format!("no bound is called `{name}`")];
            }
            let mut lines = Vec::new();
            for bound in found {
                lines.push(format!(
                    "{} = {}\n     {} · {}:{}",
                    bound.name,
                    bound.value,
                    bound.battery,
                    bound.file.display(),
                    bound.line
                ));
            }
            lines
        }
        Question::Text(words) => {
            let needle = words.to_lowercase();
            let found: Vec<&index::Command> = index
                .commands
                .iter()
                .filter(|c| c.doc.as_ref().map(|d| d.to_lowercase().contains(&needle)).unwrap_or(false))
                .collect();
            let mut lines = vec![format!("{} commands that say `{words}`:", found.len())];
            for command in found {
                lines.push(format!("     {:<34} {}", command.id, command.doc.clone().unwrap_or_default()));
            }
            lines
        }
    }
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let question = parse(args)?;
    let index = index::build(root)?;
    crate::step("find");
    println!("   {}", question.describe());
    let lines = answer(&index, &question);
    for line in lines {
        println!("{line}");
    }
    Ok(crate::Outcome::Ran)
}
