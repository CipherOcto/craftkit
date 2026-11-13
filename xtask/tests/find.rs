//! The tree's **answers**, and the index they come from.
//!
//! `cargo xtask find` answers the questions an arc asks before it writes a
//! line: where does this command live, what else is in this area, what is
//! this bound. Each of them was a search — three greps and a read, in the
//! worst case eleven — and each of them now costs one call.
//!
//! These tests are about the **index** the answers come from: it reads the
//! tree as text, so it has to read it *right*. The tree's own test compares
//! the registry with the source (§848); these tests say that the reader
//! finds the commands, their files and their lines.

use std::path::{Path, PathBuf};

use xtask::find::{Question, answer, parse};
use xtask::index;

/// The root of the tree.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").canonicalize().expect("the root is there")
}

/// A question, as it is asked.
fn ask(args: &[&str]) -> Question {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    parse(&args).expect("the question is a question")
}

#[test]
fn the_questions_are_read_out_of_the_arguments() {
    assert_eq!(ask(&["id", "grid.calc"]), Question::Id("grid.calc".to_string()));
    assert_eq!(ask(&["area", "grid"]), Question::Area("grid".to_string()));
    assert_eq!(ask(&["battery", "grid"]), Question::Battery("grid".to_string()));
    assert_eq!(ask(&["command", "pivot"]), Question::Command("pivot".to_string()));
    assert_eq!(ask(&["bound", "MAX_NODES"]), Question::Bound("MAX_NODES".to_string()));
    assert_eq!(ask(&["text", "pivot"]), Question::Text("pivot".to_string()));
}

#[test]
fn a_question_without_words_is_refused() {
    assert!(parse(&[]).is_err(), "find needs a question");
    assert!(parse(&["id".to_string()]).is_err(), "the question needs words");
    assert!(parse(&["nowhere".to_string(), "x".to_string()]).is_err(), "an unknown question is refused");
}

/// An attribute may hold a **path**, and the `)` of the path is not the end
/// of the attribute.
#[test]
fn an_attribute_with_a_path_in_it_is_read_whole() {
    let source = r#"
/// A command that has a menu.
#[derive(Command)]
#[command(id = "grid.menu.set", menu("Edit", "Trim"), label = "Set")]
pub struct SetMenu;
"#;
    let commands = index::commands_in(source, Path::new("crates/x/src/lib.rs"), "grid");
    assert_eq!(commands.len(), 1, "the command is found");
    assert_eq!(commands[0].id, "grid.menu.set");
    assert_eq!(commands[0].label.as_deref(), Some("Set"));
}

/// The documentation of a command is above its **derive**, not above its
/// attribute.
#[test]
fn the_documentation_is_the_one_above_the_derive() {
    let source = r#"
/// The **values** of a sheet: every formula, computed.
///
/// The same thing the evaluator says, as a command.
#[derive(Command)]
#[command(id = "grid.calc")]
pub struct CalcSheet;
"#;
    let commands = index::commands_in(source, Path::new("crates/x/src/lib.rs"), "grid");
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].doc.as_deref(),
        Some("The **values** of a sheet: every formula, computed."),
        "the first line of the documentation is the command's one line"
    );
}

/// A **table** of commands is read the same way a declaration is: the id,
/// the label, the documentation and the flag.
#[test]
fn a_table_of_commands_is_read() {
    let source = r#"
commands! {
    /// Switch a layer on or off.
    fx.layer.switch {
        label: "Switch",
        area: "fx",
        layer: "fx",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            Ok(Outcome::changed("switched"))
        }
    }
}
"#;
    let commands = index::table_commands_in(source, Path::new("crates/x/src/lib.rs"), "fx");
    assert_eq!(commands.len(), 1, "the row is found");
    assert_eq!(commands[0].id, "fx.layer.switch");
    assert_eq!(commands[0].label.as_deref(), Some("Switch"));
    assert_eq!(commands[0].doc.as_deref(), Some("Switch a layer on or off."));
    assert!(commands[0].line > 0, "the row has a line");
}

/// The **example** flag is read out of a row, as it is out of an attribute.
#[test]
fn the_example_flag_is_read_out_of_a_row() {
    let source = r#"
commands! {
    /// The example of the battery.
    fx.comp.add {
        example,
        params: {},
        run: {
            Ok(Outcome::changed("added"))
        }
    }
}
"#;
    let commands = index::table_commands_in(source, Path::new("crates/x/src/lib.rs"), "fx");
    assert_eq!(commands.len(), 1);
    assert!(commands[0].example, "the row is the example");
}

/// A table's rows are found whatever the table holds around them: a
/// comment, a helper, a body with a brace in a string.
#[test]
fn a_table_with_a_string_in_it_is_read_whole() {
    let source = r#"
// A table of the battery's commands.
commands! {
    /// Say a word.
    fx.word.say {
        label: "Say",
        params: {
            /// The word.
            word: Text,
        },
        run: {
            let _ = "a } in a string";
            Ok(Outcome::changed("said"))
        }
    }
}
"#;
    let commands = index::table_commands_in(source, Path::new("crates/x/src/lib.rs"), "fx");
    assert_eq!(commands.len(), 1, "the row is found: {commands:?}");
    assert_eq!(commands[0].id, "fx.word.say");
}

/// The **counts** of the tree are the counts of the commands, however they
/// are written: a battery that is a table is still a battery.
#[test]
fn the_index_of_the_tree_holds_every_command_of_every_battery() {
    let index = index::build(&root()).expect("the index is built");
    assert!(index.commands.len() > 900, "the tree has {} commands", index.commands.len());
    // The grid battery's count, which the registry also says.
    let grid = index::commands_of(&index, "grid").len();
    assert_eq!(grid, 189, "the grid battery carries 189 commands");
    // Every command knows where it is.
    for command in &index.commands {
        assert!(command.line > 0, "{} has no line", command.id);
        assert!(
            command.file.to_string_lossy().starts_with("crates/"),
            "{} is in {}",
            command.id,
            command.file.display()
        );
    }
}

#[test]
fn the_answers_name_the_file_and_the_line() {
    let index = index::build(&root()).expect("the index is built");
    let lines = answer(&index, &Question::Id("grid.calc".to_string()));
    assert_eq!(lines.len(), 1, "there is one command called grid.calc");
    assert!(lines[0].contains("grid.calc"), "the answer names the command");
    assert!(lines[0].contains(".rs:"), "the answer names the file and the line: {}", lines[0]);
}

#[test]
fn an_area_holds_its_commands_and_nothing_else() {
    let index = index::build(&root()).expect("the index is built");
    let lines = answer(&index, &Question::Area("grid".to_string()));
    let header = lines.first().expect("there is a header").to_string();
    assert!(header.starts_with("189 commands"), "the header says how many: {header}");
    for line in lines.iter().skip(1) {
        assert!(line.contains("grid."), "every command of the area is in it: {line}");
    }
}

#[test]
fn a_battery_answers_with_its_commands_modules_and_tests() {
    let index = index::build(&root()).expect("the index is built");
    let lines = answer(&index, &Question::Battery("grid".to_string()));
    let header = lines.first().expect("there is a header").to_string();
    assert!(header.contains("189 commands"), "the header says how many: {header}");
    assert!(header.contains("modules"), "the header says about the modules: {header}");
    assert!(header.contains("test files"), "the header says about the tests: {header}");
    assert!(
        lines.iter().any(|l| l.contains("crates/craftkit-plugin-grid/src/pivots.rs")),
        "the battery's modules are named"
    );
}

#[test]
fn a_bound_is_a_number_and_says_where_it_is() {
    let index = index::build(&root()).expect("the index is built");
    let lines = answer(&index, &Question::Bound("MAX_NODES".to_string()));
    assert!(!lines.is_empty(), "the bound is found");
    assert!(lines[0].contains("MAX_NODES = 10_000"), "the answer says what it is: {}", lines[0]);
    assert!(lines[0].contains(".rs:"), "the answer says where it is: {}", lines[0]);
    // And a bound that is not there is said so, not guessed at.
    let missing = answer(&index, &Question::Bound("MAX_NOTHING".to_string()));
    assert!(missing[0].contains("no bound"), "a bound that is not there is named: {}", missing[0]);
}

#[test]
fn the_index_covers_every_battery_the_readme_counts() {
    // The index reads the source; the registry is the truth; the tree's own
    // test compares them (§848). This test says the index sees the nine
    // batteries the README counts, with the counts the README says.
    let index = index::build(&root()).expect("the index is built");
    for (battery, count) in [
        ("grid", 189),
        ("prose", 110),
        ("pdf", 119),
        ("film", 113),
        ("layout", 73),
        ("cad", 55),
        ("fx", 47),
        ("sound", 144),
        ("vector", 67),
    ] {
        let commands = index::commands_of(&index, battery).len();
        assert_eq!(commands, count, "the {battery} battery carries {count} commands");
    }
}
