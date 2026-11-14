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
    Port {
        rule: "the rounding to a multiple",
        from: "gridcraft/crates/functions/src/math.rs",
        to: "craftkit-plugin-grid",
        module: "src/arithmetic.rs",
        command: "craftkit-plugin-grid::arithmetic",
        test: "crates/craftkit-plugin-grid/tests/arithmetic.rs",
        section: "§879",
    },
    Port {
        rule: "the text around a mark",
        from: "gridcraft/crates/functions/src/text.rs",
        to: "craftkit-plugin-grid",
        module: "src/texts.rs",
        command: "craftkit-plugin-grid::texts",
        test: "crates/craftkit-plugin-grid/tests/texts.rs",
        section: "§880",
    },
    Port {
        rule: "the text of a number",
        from: "gridcraft/crates/functions/src/text.rs",
        to: "craftkit-plugin-grid",
        module: "src/texts.rs",
        command: "craftkit-plugin-grid::textvalue",
        test: "crates/craftkit-plugin-grid/tests/textvalue.rs",
        section: "§881",
    },
    Port {
        rule: "the lookup across",
        from: "gridcraft/crates/functions/src/lookup.rs",
        to: "craftkit-plugin-grid",
        module: "src/lookups.rs",
        command: "craftkit-plugin-grid::lookups",
        test: "crates/craftkit-plugin-grid/tests/lookups.rs",
        section: "§882",
    },
    Port {
        rule: "the worth today",
        from: "gridcraft/crates/functions/src/financial/tvm.rs",
        to: "craftkit-plugin-grid",
        module: "src/cashflows.rs",
        command: "craftkit-plugin-grid::cashflows",
        test: "crates/craftkit-plugin-grid/tests/cashflows.rs",
        section: "§883",
    },
    Port {
        rule: "a thing bought on a day",
        from: "gridcraft/crates/functions/src/financial/depreciation.rs",
        to: "craftkit-plugin-grid",
        module: "src/depreciation.rs",
        command: "craftkit-plugin-grid::depreciation",
        test: "crates/craftkit-plugin-grid/tests/depreciation.rs",
        section: "§884",
    },
    Port {
        rule: "the tree's arrays",
        from: "gridcraft/crates/functions/src/dynamic.rs",
        to: "craftkit-plugin-grid",
        module: "src/arrays.rs",
        command: "craftkit-plugin-grid::arrays",
        test: "crates/craftkit-plugin-grid/tests/arrays.rs",
        section: "§885",
    },
    Port {
        rule: "the determinant of a square",
        from: "gridcraft/crates/functions/src/math/matrix.rs",
        to: "craftkit-plugin-grid",
        module: "src/matrices.rs",
        command: "craftkit-plugin-grid::matrices",
        test: "crates/craftkit-plugin-grid/tests/matrices.rs",
        section: "§886",
    },
    Port {
        rule: "the tree's other names, and the negative binomial",
        from: "gridcraft/crates/functions/src/stat/distributions.rs",
        to: "craftkit-plugin-grid",
        module: "src/distributions.rs",
        command: "craftkit-plugin-grid::distributions",
        test: "crates/craftkit-plugin-grid/tests/distributions.rs",
        section: "§887",
    },
    Port {
        rule: "the world outside the workbook",
        from: "gridcraft/crates/functions/src/info.rs",
        to: "craftkit-plugin-grid",
        module: "src/outside.rs",
        command: "craftkit-plugin-grid::outside",
        test: "crates/craftkit-plugin-grid/tests/outside.rs",
        section: "§888",
    },
    Port {
        rule: "the address of a cell, and the link to it",
        from: "gridcraft/crates/functions/src/lookup.rs",
        to: "craftkit-plugin-grid",
        module: "src/lookups.rs",
        command: "craftkit-plugin-grid::lookups",
        test: "crates/craftkit-plugin-grid/tests/addresses.rs",
        section: "§889",
    },
    Port {
        rule: "the Bessel and the euro",
        from: "gridcraft/crates/functions/src/engineering/bessel.rs",
        to: "craftkit-plugin-grid",
        module: "src/bessels.rs",
        command: "craftkit-plugin-grid::bessels",
        test: "crates/craftkit-plugin-grid/tests/bessels.rs",
        section: "§890",
    },
    Port {
        rule: "the forecast of a line",
        from: "gridcraft/crates/functions/src/stat.rs",
        to: "craftkit-plugin-grid",
        module: "src/regression.rs",
        command: "craftkit-plugin-grid::regression",
        test: "crates/craftkit-plugin-grid/tests/forecasts.rs",
        section: "§891",
    },
    Port {
        rule: "the odds and ends, and the weekend of the tree's own",
        from: "gridcraft/crates/functions/src/math.rs",
        to: "craftkit-plugin-grid",
        module: "src/odds.rs",
        command: "craftkit-plugin-grid::odds",
        test: "crates/craftkit-plugin-grid/tests/odds.rs",
        section: "§892",
    },
    Port {
        rule: "the pattern and the cube",
        from: "gridcraft/crates/functions/src/info.rs",
        to: "craftkit-plugin-grid",
        module: "src/refused.rs",
        command: "craftkit-plugin-grid::refused",
        test: "crates/craftkit-plugin-grid/tests/refused.rs",
        section: "§893",
    },
    Port {
        rule: "the model of a series in time",
        from: "gridcraft/crates/functions/src/ets.rs",
        to: "craftkit-plugin-grid",
        module: "src/ets.rs",
        command: "craftkit-plugin-grid::ets",
        test: "crates/craftkit-plugin-grid/tests/ets.rs",
        section: "§894",
    },
    Port {
        rule: "the registry of what the battery knows",
        from: "gridcraft/crates/functions/src/lib.rs",
        to: "craftkit-plugin-grid",
        module: "src/inventory.rs",
        command: "craftkit-plugin-grid::inventory",
        test: "crates/craftkit-plugin-grid/tests/inventory.rs",
        section: "§895",
    },
    Port {
        rule: "the host's names, and the places they name",
        from: "gridcraft/crates/calc/src/eval/call.rs",
        to: "craftkit-plugin-grid",
        module: "src/host.rs",
        command: "craftkit-plugin-grid::host",
        test: "crates/craftkit-plugin-grid/tests/host.rs",
        section: "§896",
    },
    Port {
        rule: "the host's tables, and its lambdas, and its places",
        from: "gridcraft/crates/calc/src/eval/call.rs",
        to: "craftkit-plugin-grid",
        module: "src/host.rs",
        command: "craftkit-plugin-grid::host",
        test: "crates/craftkit-plugin-grid/tests/host.rs",
        section: "§897",
    },
    Port {
        rule: "the two heavy ones: the cell, and the aggregate",
        from: "gridcraft/crates/calc/src/eval/reference_fns.rs",
        to: "craftkit-plugin-grid",
        module: "src/host.rs",
        command: "craftkit-plugin-grid::host",
        test: "crates/craftkit-plugin-grid/tests/host.rs",
        section: "§898",
    },
    Port {
        rule: "the tree's subtotal, and its error",
        from: "gridcraft/crates/calc/src/eval/reference_fns.rs",
        to: "craftkit-plugin-grid",
        module: "src/formula.rs",
        command: "craftkit-plugin-grid::subtotal",
        test: "crates/craftkit-plugin-grid/tests/subtotal.rs",
        section: "§900",
    },
    Port {
        rule: "the package: a ZIP archive, read",
        from: "gridcraft/crates/xlsx/src/package.rs",
        to: "craftkit-zip",
        module: "src/lib.rs",
        command: "craftkit-zip::Package",
        test: "crates/craftkit-zip/tests/package.rs",
        section: "§902",
    },
    Port {
        rule: "the XML reader: a part of a file, read as elements",
        from: "gridcraft/crates/xlsx/src/xml.rs",
        to: "craftkit-xml",
        module: "src/lib.rs",
        command: "craftkit-xml::parse",
        test: "crates/craftkit-xml/tests/parts.rs",
        section: "§903",
    },
    Port {
        rule: "the workbook reader: a workbook file, read as a workbook",
        from: "gridcraft/crates/xlsx/src/read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§904",
    },
    Port {
        rule: "the command that imports a workbook file",
        from: "gridcraft/crates/xlsx/src/read.rs",
        to: "craftkit-plugin-grid",
        module: "src/xlsximport.rs",
        command: "grid.import.xlsx",
        test: "crates/craftkit-plugin-grid/tests/xlsximport.rs",
        section: "§905",
    },
    Port {
        rule: "the styles part: the number formats, so a date becomes a date",
        from: "gridcraft/crates/xlsx/src/styles.rs",
        to: "craftkit-xlsx",
        module: "src/styles.rs",
        command: "craftkit-xlsx::read_styles",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§906",
    },
    Port {
        rule: "the defined names of a workbook: the names the formulas use",
        from: "gridcraft/crates/xlsx/src/read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§907",
    },
    Port {
        rule: "the auto filter of a sheet: the filter the file holds",
        from: "gridcraft/crates/xlsx/src/sheet_read/parts.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§908",
    },
    Port {
        rule: "the links of a sheet: the places its cells go to",
        from: "gridcraft/crates/xlsx/src/sheet_read/parts.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§909",
    },
    Port {
        rule: "the comments of a sheet: the notes a cell carries",
        from: "gridcraft/crates/xlsx/src/sheet_read/parts.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§910",
    },
    Port {
        rule: "the tables of a sheet: the ranges with a name and a way of being totalled",
        from: "gridcraft/crates/xlsx/src/sheet_read/parts.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§911",
    },
    Port {
        rule: "the styles of a cell: the font, the fill, the alignment and the borders",
        from: "gridcraft/crates/xlsx/src/styles.rs",
        to: "craftkit-xlsx",
        module: "src/styles.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§912",
    },
    Port {
        rule: "the protection of a sheet: what may still be done to it",
        from: "gridcraft/crates/xlsx/src/sheet_read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§913",
    },
    Port {
        rule: "the lock of a cell: whether it is locked and whether its text is hidden",
        from: "gridcraft/crates/xlsx/src/styles.rs",
        to: "craftkit-xlsx",
        module: "src/styles.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§913",
    },
    Port {
        rule: "the rows and the columns of a sheet: their heights, their widths and which are hidden",
        from: "gridcraft/crates/xlsx/src/sheet_read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§914",
    },
    Port {
        rule: "the merges of a sheet: the ranges that are one cell",
        from: "gridcraft/crates/xlsx/src/sheet_read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§916",
    },
    Port {
        rule: "the view of a sheet: what is drawn of it, and what is not",
        from: "gridcraft/crates/xlsx/src/sheet_read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§917",
    },
    Port {
        rule: "the conditional formats of a sheet: the rules that are about a range of cells",
        from: "gridcraft/crates/xlsx/src/sheet_read/rules.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§918",
    },
    Port {
        rule: "the shared formulas of a sheet: the group that shares one formula",
        from: "gridcraft/crates/xlsx/src/sheet_read.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§919",
    },
    Port {
        rule: "the data validations of a sheet: the rules on what a cell may hold",
        from: "gridcraft/crates/xlsx/src/sheet_read/rules.rs",
        to: "craftkit-xlsx",
        module: "src/lib.rs",
        command: "craftkit-xlsx::read",
        test: "crates/craftkit-xlsx/tests/workbook.rs",
        section: "§920",
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
