//! The tree's **port manifest**: which rule came from where.
//!
//! # The problem
//!
//! A port is invisible. The grid battery carries a rule about pivots, and
//! nothing in the tree says that the rule came from `gridcraft`'s pivot
//! table, or that it landed in `crates/craftkit-plugin-grid/src/pivots.rs`,
//! or that it is tested by `tests/pivots.rs`. The port is legible only in
//! the commit history, in prose, one arc at a time — so the tree **cannot
//! check its own ports**, and nobody can ask "what of `gridcraft` is not
//! here yet?".
//!
//! # The manifest
//!
//! One row per ported rule, in Rust, in this file:
//!
//! ```text
//! rule     what the rule is
//! from     where it came from: a path in a family tree, or a document here
//! to       the crate that carries it
//! module  the file of that crate it landed in
//! command the command that carries it
//! test    the test file that holds its test
//! section the section of the README that says so
//! ```
//!
//! **Forward only.** The rows are added as the ports are made; the tree does
//! not go back and write a row for every command that was ever ported.
//! That would be a project rather than an arc, and it would be a project
//! of no use: a row that is written after the fact records a decision
//! nobody made.
//!
//! # What the manifest is checked against
//!
//! The command must be in the tree, and the test file must be there. A row
//! that names a command that was renamed away, or a test that was deleted,
//! is a row that has rotted — and the gate says so.

use std::path::Path;

use crate::index::Index;

/// One ported rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// What the rule is.
    pub rule: &'static str,
    /// Where it came from: a path in a family tree, or a document in this
    /// tree.
    pub from: &'static str,
    /// The crate that carries it.
    pub to: &'static str,
    /// The file of that crate it landed in.
    pub module: &'static str,
    /// What carries it: a **command's id**, or a **crate and a symbol** in
    /// it — `craftkit-sdk::Registry::of_area`.
    ///
    /// Two spellings, because a port does not always add a command: a port
    /// that adds an API adds a symbol, and the manifest must be able to say
    /// so.
    pub command: &'static str,
    /// The test file that holds its test.
    pub test: &'static str,
    /// The section of the README that says so.
    pub section: &'static str,
}

/// The ports of this tree.
///
/// **The rows are added as the ports are made.** Every arc that ports a rule
/// adds a row here, and the gate checks the rows that exist.
pub const PORTS: &[Port] = &[
    Port {
        rule: "the cost of the tree, and the plan to take it down",
        from: "docs/survey-cost-and-meta.md",
        to: "xtask",
        module: "src/verify.rs",
        command: "xtask.verify",
        test: "xtask/tests/verify.rs",
        section: "§844",
    },
    Port {
        rule: "the check that is the size of the change",
        from: "docs/survey-cost-and-meta.md",
        to: "xtask",
        module: "src/verify.rs",
        command: "xtask.verify",
        test: "xtask/tests/verify.rs",
        section: "§845",
    },
    Port {
        rule: "the gate, and what it says it did not check",
        from: "docs/survey-cost-and-meta.md",
        to: "xtask",
        module: "src/ci.rs",
        command: "xtask.ci",
        test: "xtask/tests/scope.rs",
        section: "§846",
    },
    Port {
        rule: "the delegation, priced",
        from: "docs/agent-playbook.md",
        to: "xtask",
        module: "src/lists.rs",
        command: "xtask.lists",
        test: "xtask/tests/delegation.rs",
        section: "§847",
    },
    Port {
        rule: "the counts, from the registry",
        from: "docs/survey-cost-and-meta.md",
        to: "craftkit-sdk",
        module: "src/contribution.rs",
        command: "craftkit-sdk::Registry::of_area",
        test: "apps/craftkit-mcp-server/tests/registry.rs",
        section: "§848",
    },
    Port {
        rule: "the tree's answers, in one call",
        from: "docs/survey-cost-and-meta.md",
        to: "xtask",
        module: "src/find.rs",
        command: "xtask.find",
        test: "xtask/tests/find.rs",
        section: "§851",
    },
    Port {
        rule: "the example of a battery",
        from: "docs/survey-cost-and-meta.md",
        to: "xtask",
        module: "src/example.rs",
        command: "xtask.example",
        test: "xtask/tests/example.rs",
        section: "§852",
    },
    Port {
        rule: "the counting of more than one condition",
        from: "gridcraft/crates/functions/src/math/criteria.rs",
        to: "craftkit-plugin-grid",
        module: "src/aggregates.rs",
        command: "craftkit-plugin-grid::aggregates",
        test: "crates/craftkit-plugin-grid/tests/aggregates.rs",
        section: "§874",
    },
    Port {
        rule: "the finding of a value, the tree's way",
        from: "gridcraft/crates/functions/src/lookup.rs",
        to: "craftkit-plugin-grid",
        module: "src/lookups.rs",
        command: "craftkit-plugin-grid::lookups",
        test: "crates/craftkit-plugin-grid/tests/xlookups.rs",
        section: "§875",
    },
    Port {
        rule: "the days between two dates",
        from: "gridcraft/crates/functions/src/date.rs",
        to: "craftkit-plugin-grid",
        module: "src/datetimes.rs",
        command: "craftkit-plugin-grid::datetimes",
        test: "crates/craftkit-plugin-grid/tests/intervals.rs",
        section: "§876",
    },
    Port {
        rule: "the fraction of a year, and the working days",
        from: "gridcraft/crates/functions/src/date.rs",
        to: "craftkit-plugin-grid",
        module: "src/datetimes.rs",
        command: "craftkit-plugin-grid::datetimes",
        test: "crates/craftkit-plugin-grid/tests/intervals.rs",
        section: "§877",
    },
    Port {
        rule: "the aggregates that count the text",
        from: "gridcraft/crates/functions/src/stat.rs",
        to: "craftkit-plugin-grid",
        module: "src/statistics.rs",
        command: "craftkit-plugin-grid::statistics",
        test: "crates/craftkit-plugin-grid/tests/counted.rs",
        section: "§878",
    },
];

/// The problems the manifest has: a row that names a command that is not in
/// the tree, or a test file that is not there.
pub fn problems(root: &Path, index: &Index) -> Vec<String> {
    problems_of(root, index, PORTS)
}

/// The problems a set of ports has.
///
/// **A function of its own, so that the rule can be pinned by a test.** The
/// rule it came from reads a constant, and a test cannot put a broken row
/// into a constant.
pub fn problems_of(root: &Path, index: &Index, ports: &[Port]) -> Vec<String> {
    let mut problems = Vec::new();
    for port in ports {
        // The command must be in the tree — unless the port names a symbol
        // of a crate, which the crate's own directory answers.
        let known = if let Some((crate_name, _)) = port.command.split_once("::") {
            root.join("crates").join(crate_name).is_dir()
        } else {
            index.commands.iter().any(|c| c.id == port.command)
                || crate::COMMANDS.iter().any(|command| port.command == format!("xtask.{}", command.name))
        };
        if !known {
            problems.push(format!(
                "the port of `{}` names the command `{}`, and the tree has no such command",
                port.rule, port.command
            ));
        }
        let test = root.join(port.test);
        if !test.is_file() {
            problems.push(format!(
                "the port of `{}` names the test `{}`, and the tree has no such file",
                port.rule, port.test
            ));
        }
        // The crate may be a crate of the tree or the tool itself: the
        // batteries live in `crates/`, and the xtask is at the root.
        let in_crates = root.join("crates").join(port.to).join(port.module);
        let at_the_root = root.join(port.to).join(port.module);
        if !in_crates.is_file() && !at_the_root.is_file() {
            problems.push(format!(
                "the port of `{}` names the module `{}` of the crate `{}`, and the tree has no such \
                 file",
                port.rule, port.module, port.to
            ));
        }
    }
    problems
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    crate::step("ports");
    let index = crate::index::build(root)?;
    for port in PORTS {
        println!("   {}", port.section);
        println!("      rule:    {}", port.rule);
        println!("      from:    {}", port.from);
        println!("      to:      {} · {}", port.to, port.module);
        println!("      command: {}", port.command);
        println!("      test:    {}", port.test);
    }
    if args.iter().any(|a| a == "--check") {
        let problems = problems(root, &index);
        if !problems.is_empty() {
            for problem in &problems {
                println!("   ✗ {problem}");
            }
            return Err(format!("{} problem(s) in the port manifest", problems.len()));
        }
        println!("   ok — {} ports, and every one of them is in the tree", PORTS.len());
    }
    Ok(crate::Outcome::Ran)
}
