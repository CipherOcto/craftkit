//! `cargo xtask new-arc` — the ceremony of an arc, written by one call.
//!
//! # The question this answers
//!
//! An arc ends with three things that are **not the code**: a row in the
//! port manifest, a row in the README's claim table, and a section of the
//! README that says what the arc did. Each of them was found — *searched
//! for* — and written by hand, and the survey measured what that costs:
//! the searching is 16 greps an arc, and the writing is three files an
//! author has to open, in the right place, in the right order.
//!
//! This command writes all three, in one call, from the tree's own
//! conventions:
//!
//! ```text
//! cargo xtask new-arc --battery grid --id grid.pivot.live \
//!     --rule "the way a pivot shows its value" \
//!     --from gridcraft/crates/model/src/features/pivot.rs
//! ```
//!
//! # The rules
//!
//! - **The command must already be there.** The claim table says what is
//!   *true today*, and a row written before the code lands is a claim that
//!   is false. So the command is refused unless the registry holds the id.
//! - **The section is the README's own.** Without `--section`, the arc
//!   takes the next number the README is about to have.
//! - **The port row goes in section order**, and the claim row goes at the
//!   end of the claim table: both are places a person has to find again.
//! - **The code is the author's.** The command prints the skeleton of the
//!   command and of its test, and the author pastes them: the file a
//!   command lands in is a decision about the tree's shape, and not the
//!   generator's to take.

use std::path::Path;

/// The arc this call is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arc {
    /// The battery the arc is working in.
    pub battery: String,
    /// The id of the command the arc adds.
    pub id: String,
    /// The rule, in a sentence.
    pub rule: String,
    /// Where the rule came from.
    pub from: String,
    /// The label the command carries.
    pub label: String,
    /// The section the arc takes: `§1089`.
    pub section: String,
    /// The crate that carries the command.
    pub crate_name: String,
    /// The file of that crate the command lands in.
    pub module: String,
    /// The file that holds the command's test.
    pub test: String,
}

impl Arc {
    /// The **number** of the section.
    pub fn number(&self) -> Result<usize, String> {
        let digits: String = self.section.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.parse::<usize>().map_err(|_| format!("`{}` is not a section", self.section))
    }

    /// The **title** of the section: the rule, said as a title.
    pub fn title(&self) -> String {
        let mut words = self.rule.chars();
        let mut title = String::new();
        if let Some(first) = words.next() {
            title.extend(first.to_uppercase());
            title.push_str(words.as_str());
        }
        title
    }
}

/// Read the arc out of the arguments.
pub fn parse(args: &[String], next_section: &str) -> Result<Arc, String> {
    let mut battery = String::new();
    let mut id = String::new();
    let mut rule = String::new();
    let mut from = String::new();
    let mut label = String::new();
    let mut section = String::new();
    let mut crate_name = String::new();
    let mut module = String::new();
    let mut test = String::new();
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let mut value = || -> Result<String, String> {
            let value = args.get(index + 1).cloned();
            index += 2;
            value.ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match flag {
            "--battery" => battery = value()?,
            "--id" => id = value()?,
            "--rule" => rule = value()?,
            "--from" => from = value()?,
            "--label" => label = value()?,
            "--section" => section = value()?,
            "--crate" => crate_name = value()?,
            "--module" => module = value()?,
            "--test" => test = value()?,
            other => return Err(format!("`{other}` is not a flag of `new-arc`")),
        }
    }
    if battery.is_empty() {
        return Err("the arc needs a battery: `--battery grid`".to_string());
    }
    if id.is_empty() {
        return Err("the arc needs a command: `--id grid.pivot.live`".to_string());
    }
    if rule.is_empty() {
        return Err("the arc needs a rule: `--rule \"the way a pivot shows its value\"`".to_string());
    }
    let label = if label.is_empty() { rule.clone() } else { label };
    let section = if section.is_empty() { next_section.to_string() } else { section };
    let crate_name = if crate_name.is_empty() { format!("craftkit-plugin-{battery}") } else { crate_name };
    let module = if module.is_empty() { "src/lib.rs".to_string() } else { module };
    let test = if test.is_empty() { format!("crates/{crate_name}/tests/{battery}.rs") } else { test };
    Ok(Arc { battery, id, rule, from, label, section, crate_name, module, test })
}

/// The **row of the port manifest** this arc adds.
pub fn port_row(arc: &Arc) -> String {
    format!(
        "    Port {{\n\
         \x20       rule: {rule:?},\n\
         \x20       from: {from:?},\n\
         \x20       to: {crate_name:?},\n\
         \x20       module: {module:?},\n\
         \x20       command: {id:?},\n\
         \x20       test: {test:?},\n\
         \x20       section: {section:?},\n\
         \x20   }},\n",
        rule = arc.rule,
        from = arc.from,
        crate_name = arc.crate_name,
        module = arc.module,
        id = arc.id,
        test = arc.test,
        section = arc.section,
    )
}

/// The **row of the claim table** this arc adds.
///
/// **Written from the registry.** The truth column says what is true today:
/// the tree has the command, and the section says where. That is a fact the
/// registry knows, so it is written by machine — and the gate checks it
/// against the registry, so a row that names a command the tree does not
/// have is caught.
pub fn claim_row(arc: &Arc) -> String {
    format!(
        "| **{rule}** | **built.** The tree has the command `{id}` ({section}) | {section} |\n",
        rule = arc.rule,
        id = arc.id,
        section = arc.section,
    )
}

/// The **section of the README** this arc adds.
pub fn section_text(arc: &Arc) -> String {
    format!(
        "\n## {number}. {title}\n\n\
         {rule}.\n\n\
         ### What was built\n\n\
         - **TODO** — what the arc built.\n\n\
         ### What the port found\n\n\
         - **TODO** — what the port found.\n\n\
         ### What it cost\n\n\
         - **TODO** — what it cost.\n\n\
         ### What it does not settle\n\n\
         - **TODO** — what it does not settle.\n",
        number = arc.number().unwrap_or(0),
        title = arc.title(),
        rule = arc.rule,
    )
}

/// The **skeleton of the command**, in the battery's own table.
pub fn command_skeleton(arc: &Arc) -> String {
    format!(
        "    /// **{label}**.\n\
         \x20   ///\n\
         \x20   /// {rule}.\n\
         \x20   {id} {{\n\
         \x20       area: \"{battery}\",\n\
         \x20       layer: \"{battery}\",\n\
         \x20       icon: \"{battery}\",\n\
         \x20       mutates,\n\
         \x20       undoable,\n\
         \x20       params: {{\n\
         \x20           /// TODO — what it takes.\n\
         \x20           value: Text,\n\
         \x20       }},\n\
         \x20       run: |_session, _params| {{\n\
         \x20           // TODO — what it does.\n\
         \x20           Err(\"not supported yet: {id}\".into())\n\
         \x20       }},\n\
         \x20   }},\n",
        label = arc.label,
        rule = arc.rule,
        id = arc.id,
        battery = arc.battery,
    )
}

/// The **skeleton of the test**.
pub fn test_skeleton(arc: &Arc) -> String {
    format!(
        "\n/// **{rule}.**\n\
         #[test]\n\
         fn {name}() {{\n\
         \x20   let mut session = session();\n\
         \x20   session\n\
         \x20       .execute(\n\
         \x20           \"{id}\",\n\
         \x20           &json!({{ /* TODO — the parameters. */ }}),\n\
         \x20       )\n\
         \x20       .expect(\"the command runs\");\n\
         \x20   // TODO — what it asserts.\n\
         }}\n",
        rule = arc.rule,
        name = the_test_name(&arc.id),
        id = arc.id,
    )
}

/// The name of a command's test: the id, without its dots.
pub fn the_test_name(id: &str) -> String {
    id.replace('.', "_")
}

/// Put the **port row** into the manifest, in section order.
pub fn insert_port_row(source: &str, row: &str, section: usize) -> Result<String, String> {
    let mut rows: Vec<(usize, usize)> = Vec::new();
    let mut offset = 0usize;
    let mut row_start = 0usize;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed == "Port {" {
            row_start = offset;
        }
        if let Some(rest) = trimmed.strip_prefix("section: \"§") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(number) = digits.parse::<usize>() {
                rows.push((number, row_start));
            }
        }
        offset += line.len();
    }
    let at = rows.iter().find(|(number, _)| *number > section).map(|(_, at)| *at);
    match at {
        Some(at) => {
            let mut out = String::with_capacity(source.len() + row.len());
            out.push_str(&source[..at]);
            out.push_str(row);
            out.push_str(&source[at..]);
            Ok(out)
        }
        None => {
            // The last row of the manifest: the new one goes after it.
            let marker = "];";
            let at = source
                .find(marker)
                .ok_or("the port manifest has no end: the `];` is not in it".to_string())?;
            let mut out = String::with_capacity(source.len() + row.len());
            out.push_str(&source[..at]);
            out.push_str(row);
            out.push_str(&source[at..]);
            Ok(out)
        }
    }
}

/// Put the **claim row** into the claim table, at the end of it.
pub fn insert_claim_row(readme: &str, row: &str) -> Result<String, String> {
    let header = "| the claim | the truth today | where |";
    let start =
        readme.find(header).ok_or("the README has no claim table: the header is not in it".to_string())?;
    // The first line **after** the header: the header is found by its text,
    // and the text stops in the middle of its line.
    let after = readme[start..]
        .find('\n')
        .map(|at| start + at + 1)
        .ok_or("the claim table has no lines after its header".to_string())?;
    // The end of the table: the last line that starts with a bar.
    let mut end = after;
    for line in readme[after..].split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n');
        if trimmed.starts_with('|') {
            end += line.len();
            continue;
        }
        if trimmed.trim().is_empty() {
            continue;
        }
        break;
    }
    let mut out = String::with_capacity(readme.len() + row.len());
    out.push_str(&readme[..end]);
    out.push_str(row);
    out.push_str(&readme[end..]);
    Ok(out)
}

/// Put the **section** at the end of the README.
pub fn append_section(readme: &str, text: &str) -> String {
    let mut out = String::with_capacity(readme.len() + text.len());
    out.push_str(readme.trim_end());
    out.push('\n');
    out.push_str(text);
    out
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let print_only = args.iter().any(|a| a == "--print");
    let rest: Vec<String> = args.iter().filter(|a| a.as_str() != "--print").cloned().collect();
    let readme = crate::tree::read_text(root.join("README.md"))
        .map_err(|e| format!("could not read the README: {e}"))?;
    let next = crate::brief::next_section(&readme);
    let arc = parse(&rest, &next)?;

    if print_only {
        println!("── the command\n{}", command_skeleton(&arc));
        println!("── the test\n{}", test_skeleton(&arc));
        return Ok(crate::Outcome::Ran);
    }

    // **A port says where the rule came from.** Without it, the manifest
    // records a rule with no origin, and the origin is the half of the row
    // that makes it auditable.
    if arc.from.is_empty() {
        return Err("the arc needs to say where the rule came from: `--from <path>`".to_string());
    }

    // **The command must already be there.** The claim table says what is
    // true today, and a row written before the code lands is a claim that
    // is false.
    let index = crate::index::build(root).map_err(|e| format!("the tree could not be indexed: {e}"))?;
    if !index.commands.iter().any(|command| command.id == arc.id) {
        return Err(format!(
            "the tree has no command `{}`: the claim table says what is true today, and a row \
             written before the code lands is a claim that is false. Write the command first, and \
             use `--print` to see its skeleton.",
            arc.id
        ));
    }

    let manifest_path = root.join("xtask/src/ports.rs");
    let manifest = crate::tree::read_rust(&manifest_path, crate::tree::Subject::Everything)
        .map_err(|e| format!("could not read the port manifest: {e}"))?;
    let manifest = insert_port_row(&manifest, &port_row(&arc), arc.number()?)
        .map_err(|e| format!("the port manifest could not be written: {e}"))?;
    std::fs::write(&manifest_path, &manifest)
        .map_err(|e| format!("the port manifest could not be written: {e}"))?;

    let readme_path = root.join("README.md");
    let readme = insert_claim_row(&readme, &claim_row(&arc))?;
    let readme = append_section(&readme, &section_text(&arc));
    std::fs::write(&readme_path, &readme).map_err(|e| format!("the README could not be written: {e}"))?;

    println!("── the arc");
    println!("   the command: {}", arc.id);
    println!("   the section: {}", arc.section);
    println!("   the manifest: xtask/src/ports.rs");
    println!("   the claim:   README.md");
    println!("   the section: README.md");
    println!();
    println!("the claim says what is true today, so read it once more:");
    println!("{}", claim_row(&arc).trim_end());
    Ok(crate::Outcome::Ran)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The arguments of an arc.
    fn args_of(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| part.to_string()).collect()
    }

    /// A whole arc, as the command reads it.
    fn an_arc() -> Arc {
        parse(
            &args_of(&[
                "--battery",
                "grid",
                "--id",
                "grid.pivot.live",
                "--rule",
                "the way a pivot shows its value",
                "--from",
                "gridcraft/crates/model/src/features/pivot.rs",
            ]),
            "§1089",
        )
        .expect("the arc is read")
    }

    #[test]
    fn an_arc_takes_the_section_the_readme_is_about_to_have() {
        assert_eq!(an_arc().section, "§1089");
    }

    #[test]
    fn an_arc_needs_its_rule() {
        let error =
            parse(&args_of(&["--battery", "grid", "--id", "grid.x"]), "§1").expect_err("it is refused");
        assert!(error.contains("needs a rule"), "{error}");
        let error = parse(&args_of(&["--battery", "grid"]), "§1").expect_err("it is refused");
        assert!(error.contains("needs a command"), "{error}");
    }

    #[test]
    fn a_port_row_is_the_manifests_own_shape() {
        let row = port_row(&an_arc());
        assert!(row.contains("rule: \"the way a pivot shows its value\""), "{row}");
        assert!(row.contains("command: \"grid.pivot.live\""), "{row}");
        assert!(row.contains("section: \"§1089\""), "{row}");
        assert!(row.trim_end().ends_with("},"), "{row}");
    }

    #[test]
    fn a_claim_row_says_what_is_true_today() {
        let row = claim_row(&an_arc());
        assert!(row.contains("grid.pivot.live"), "{row}");
        assert!(row.contains("§1089"), "{row}");
        assert!(row.starts_with("| **the way a pivot shows its value** |"), "{row}");
    }

    #[test]
    fn a_section_has_the_four_headings_the_house_asks_for() {
        let text = section_text(&an_arc());
        for heading in [
            "### What was built",
            "### What the port found",
            "### What it cost",
            "### What it does not settle",
        ] {
            assert!(text.contains(heading), "the section has no `{heading}`");
        }
        assert!(text.starts_with("\n## 1089. "), "{text}");
    }

    #[test]
    fn a_port_row_goes_in_section_order() {
        let manifest = "pub const PORTS: &[Port] = &[\n    Port {\n        section: \"§10\",\n    },\n    Port {\n        section: \"§20\",\n    },\n];\n";
        let out = insert_port_row(manifest, "    Port {\n        section: \"§15\",\n    },\n", 15)
            .expect("the row is placed");
        let ten = out.find("§10").expect("ten is there");
        let fifteen = out.find("§15").expect("fifteen is there");
        let twenty = out.find("§20").expect("twenty is there");
        assert!(ten < fifteen && fifteen < twenty, "{out}");
    }

    #[test]
    fn a_port_row_after_the_last_one_goes_at_the_end() {
        let manifest = "pub const PORTS: &[Port] = &[\n    Port {\n        section: \"§10\",\n    },\n];\n";
        let out = insert_port_row(manifest, "    Port {\n        section: \"§20\",\n    },\n", 20)
            .expect("the row is placed");
        let ten = out.find("§10").expect("ten is there");
        let twenty = out.find("§20").expect("twenty is there");
        assert!(ten < twenty, "{out}");
        assert!(out.trim_end().ends_with("];"), "{out}");
    }

    #[test]
    fn a_claim_row_goes_at_the_end_of_the_table() {
        let readme = "# The tree\n\n## 48. The list\n\n| the claim | the truth today | where |\n|---|---|---|\n| **one** | built (§1) | §1 |\n| **two** | built (§2) | §2 |\n\n### After the table\n";
        let out = insert_claim_row(readme, "| **three** | built (§3) | §3 |\n").expect("the row is placed");
        let one = out.find("**one**").expect("one is there");
        let two = out.find("**two**").expect("two is there");
        let three = out.find("**three**").expect("three is there");
        assert!(one < two && two < three, "{out}");
        assert!(out.contains("### After the table"), "the table ends where it did: {out}");
    }

    #[test]
    fn a_claim_row_starts_on_a_line_of_its_own() {
        // The failure this catches: the row was written at the end of the
        // last row of the table, on the same line, and the table stopped
        // being a table.
        let readme = "| the claim | the truth today | where |\n|---|---|---|\n| **one** | built (§1) | §1 |\n\n### After\n";
        let out = insert_claim_row(readme, "| **two** | built (§2) | §2 |\n").expect("the row is placed");
        assert!(out.contains("| §1 |\n| **two** |"), "the row is on a line of its own: {out}");
    }

    #[test]
    fn a_readme_without_a_claim_table_is_refused() {
        let error = insert_claim_row("# The tree\n", "| a | b | c |\n").expect_err("it is refused");
        assert!(error.contains("no claim table"), "{error}");
    }
}
