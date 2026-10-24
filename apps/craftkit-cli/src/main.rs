//! # craftkit-cli
//!
//! Headless craftkit: the whole application, minus the window.
//!
//! ## Why a CLI is its own binary
//!
//! Not convenience — **testability**. A script, a CI job and an agent need to
//! run commands without a display, and they need the *same* commands the window
//! runs. If the CLI had its own shortcuts into the engine, "what the agent did"
//! and "what the person did" would be two different things, and every claim the
//! project makes about a single command registry would be conditional.
//!
//! So every subcommand here goes through `Session::execute`, exactly like the
//! toolbar.
//!
//! ## Subcommands
//!
//! ```text
//! craftkit-cli commands [--json]           list every command and its parameters
//! craftkit-cli run <script.json> [--sample] run a script of commands
//! craftkit-cli eval <file.craftkit>          open a design and report on it
//! craftkit-cli sample --out <file.craftkit>  write a sample design to a file
//! craftkit-cli script --out <file.json>      write a script of commands to a file
//! craftkit-cli snapshot <script.json> --out shot.png   run a script, draw a frame
//!
//! Text rasterises as solid glyph quads in the PNG (egui 0.33 exposes no texture
//! id per primitive); every other primitive draws as the window draws it.
//! craftkit-cli crashes list|show|replay <id>  read the crash folder
//! craftkit-cli crashes write --out <report.json>   write a crash report
//! craftkit-cli mcp                          speak MCP over stdio
//! ```
//!
//! ## The script format
//!
//! ```json
//! {
//!   "name": "a ramp",
//!   "commands": [
//!     { "command": "node.add", "params": { "name": "In", "kind": "source", "amount": 1 } },
//!     { "command": "node.add", "params": { "name": "Out", "kind": "sink" } }
//!   ]
//! }
//! ```
//!
//! A script is what you paste into a bug report, and a **crash report** is
//! what you paste back into the CLI: `craftkit-cli crashes replay --file
//! <report.json>` replays the file you were handed and says whether it
//! reproduced the design the report recorded. §195.
//!
//! And the tree can **write** one: `craftkit-cli script --out <file>` writes
//! the sample design's own commands. §182.
//!
//! And the tree can write a **crash report** too:
//! `craftkit-cli crashes write --out <file>` writes one, with the tree's own
//! writer, so the file holds the same bytes a report in the crash folder
//! holds. Before this the only way to get a report onto disk was to have a
//! **window** crash. §200.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod args;
mod cli;

use std::process::ExitCode;

use cli::Cli;

const USAGE: &str = "\
craftkit-cli — headless craftkit

USAGE:
    craftkit-cli <COMMAND> [OPTIONS]

COMMANDS:
    commands                 List every command and its parameters.
    run <script.json>        Run a script of commands.
    eval <file.craftkit>     Open a design and report what it evaluates to.
    sample --out <file.craftkit>
                             Write a sample design to a file.
    script --out <file.json>
                             Write a script of commands to a file.
    snapshot <script.json> --out <file.png>
                             Run a script and write a rendered frame.
    crashes <list|show|replay> [ID|--file <report.json>]
                             Read the crash folder, or a report in a file —
                             one that arrived from another machine.
    crashes write --out <report.json>
                             Write a crash report to a file.
    mcp                      Speak MCP over stdio, for an agent's tool loop.
    help                     Print this message.

OPTIONS:
    -V, --version            Print the version.

Run `craftkit-cli commands` for the full catalogue with every parameter
documented. Every one of them is the same command the window's toolbar runs.

COMMAND FORMAT (`commands`):
    A parameter marked `?` is optional. A parameter with an alternative is
    marked `?` too, and its line names the other spelling: give one of the
    two. `node` and `node_name` are one parameter in two spellings, and so
    are `seconds` and `frame`.

SCRIPT FORMAT (`run` and `snapshot`):
    {\"name\": \"optional\",
     \"commands\": [{\"command\": \"node.add\",
                    \"params\": {\"name\": \"Box\", \"kind\": \"filter\"}}]}
    `params` may be omitted and means no parameters. `crashes replay <ID>
    --script` prints the replay as this same shape, with the document it
    starts from, so `snapshot` can render it. A crash report is not a script:
    run it with `crashes replay --file <report.json>`, which says whether the
    replay reproduced the design the report recorded.
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let cli = match Cli::parse(&argv) {
        Ok(Some(cli)) => cli,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("craftkit-cli: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };

    install_logger();
    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("craftkit-cli: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Print the usage text.
pub fn usage() -> &'static str {
    USAGE
}

/// A logger to stderr, so `--json` output on stdout stays machine-readable.
fn install_logger() {
    struct Stderr;
    impl log::Log for Stderr {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            eprintln!("[{}] {}", record.level(), record.args());
        }

        fn flush(&self) {}
    }
    static LOGGER: Stderr = Stderr;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Warn);
}

#[cfg(test)]
mod usage_tests {
    use super::USAGE;

    /// The subcommand names the parser accepts, read out of `args.rs`.
    ///
    /// The parser is a `match first.as_str() { … }`, so the arms are the names.
    /// The scan walks the braces to find the end of that match, and takes the
    /// first quoted word of each arm — skipping the ones that start with `-`,
    /// which are flags (`-h`, `--help`) rather than subcommands.
    fn parser_subcommands(source: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut depth = 0i64;
        let mut inside = false;
        for line in source.lines() {
            if !inside {
                if line.contains("match first.as_str()") {
                    inside = true;
                    depth = brace_depth(line);
                }
                continue;
            }
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix('"')
                && let Some(end) = rest.find('"')
            {
                let name = &rest[..end];
                if !name.starts_with('-') && trimmed[end + 2..].contains("=>") {
                    names.push(name.to_string());
                }
            }
            depth += brace_depth(line);
            if depth <= 0 {
                break;
            }
        }
        names
    }

    /// The number of `{` minus the number of `}` on a line.
    fn brace_depth(line: &str) -> i64 {
        let opens = line.matches('{').count() as i64;
        let closes = line.matches('}').count() as i64;
        opens - closes
    }

    /// The subcommand names the help lists.
    ///
    /// The `COMMANDS:` block, and one name per line: the first word of every
    /// line indented by exactly four spaces. The deeper lines are the
    /// continuation lines of a command whose own line was too long.
    fn help_subcommands(usage: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut in_block = false;
        for line in usage.lines() {
            if !in_block {
                in_block = line.trim() == "COMMANDS:";
                continue;
            }
            if line.trim().is_empty() {
                break;
            }
            let indent = line.len() - line.trim_start().len();
            if indent != 4 {
                continue;
            }
            if let Some(name) = line.split_whitespace().next() {
                let name = name.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
        names
    }

    /// The script `USAGE` shows, taken out of `USAGE` itself.
    ///
    /// There is deliberately no second constant holding the example: two
    /// copies of one fact is the bug this whole document is about, and the
    /// first version of this test compared them and caught the drift within
    /// seconds of being written. One source, parsed.
    fn script_in_the_help() -> String {
        let after = USAGE.split("SCRIPT FORMAT").nth(1).expect("USAGE names the script format");
        let block = after.split("`params` may be omitted").next().expect("the example comes first");
        let start = block.find('{').expect("the example is a JSON object");
        let end = block.rfind('}').expect("the example is closed");
        block[start..=end].replace("\\\"", "\"")
    }

    /// The help and the parser are two spellings of one list, and nothing kept
    /// them in step.
    ///
    /// `USAGE` is a hand-written string; the subcommands are match arms in
    /// `args.rs`. A subcommand could be added to the parser and never appear in
    /// the help — and then no agent would ever find it, because the help is
    /// where an agent looks. §28 found the same shape for the script format:
    /// the one thing a reader must know, in the one place a reader looks.
    ///
    /// So the names are read out of the parser's own source, and the help has
    /// to mention every one of them — and, the other way, may not mention one
    /// the parser does not have.
    #[test]
    fn the_help_lists_every_subcommand_the_parser_accepts() {
        let parser = parser_subcommands(include_str!("args.rs"));
        let help = help_subcommands(super::USAGE);

        assert!(parser.len() >= 5, "the scan found {parser:?}; it is probably looking at nothing");
        for name in &parser {
            assert!(
                help.contains(name),
                "the parser accepts `{name}` and the help does not mention it: {help:?}"
            );
        }
        for name in &help {
            assert!(
                parser.contains(name),
                "the help mentions `{name}` and the parser does not accept it: {parser:?}"
            );
        }
    }

    /// The version the CLI prints is the crate's own version, and the line
    /// names the program.
    ///
    /// The version is the one string a bug report quotes, and the CLI is the
    /// program an agent drives — so it is the one whose version matters most.
    /// It comes from `env!("CARGO_PKG_VERSION")`, which is the inherited
    /// workspace version, so it cannot be stale; a version written out by
    /// hand would be, and nothing would say so. §174.
    #[test]
    fn the_version_line_names_the_program_and_its_version() {
        let line = crate::cli::version_line();
        assert!(line.starts_with("craftkit-cli "), "{line}");
        assert!(line.ends_with(env!("CARGO_PKG_VERSION")), "{line}");
        assert_ne!(line.trim(), "craftkit-cli", "the version line has no version in it");
    }

    /// The help says the version can be asked for.
    #[test]
    fn the_help_names_the_version_flag() {
        assert!(USAGE.contains("--version"), "the help does not mention the version flag");
    }

    /// The one thing anybody must know to write a `script.json`, and it used to
    /// live only in this file's module documentation — which a reader of
    /// `--help`, or of an error message, never sees. This tool exists to be
    /// driven by an agent, and the agent's first question is what a script is.
    #[test]
    fn the_help_carries_a_script_that_is_actually_a_script() {
        assert!(USAGE.contains("SCRIPT FORMAT"), "the help does not mention the script format at all");

        // Copied out of the help and pasted into a file, it must work. Parsed
        // rather than pattern-matched, because that is what the reader does.
        let value: serde_json::Value =
            serde_json::from_str(&script_in_the_help()).expect("the example in USAGE is not valid JSON");
        let commands = value["commands"].as_array().expect("`commands` is an array");
        assert!(!commands.is_empty(), "the example runs nothing");
        assert!(commands.iter().all(|c| c["command"].is_string()), "every step names a command");
        assert!(commands[0]["params"].is_object(), "the example shows `params`");
    }

    /// The deserialiser is the one that decides, and it lives in another file.
    /// Pin the shape the help shows against the struct it has to satisfy.
    #[test]
    fn the_help_example_has_the_shape_the_loader_expects() {
        let value: serde_json::Value = serde_json::from_str(&script_in_the_help()).expect("valid JSON");
        assert!(value.get("name").is_some(), "`name` is part of the documented shape");
        assert!(value.get("commands").is_some(), "`commands` is the required key");
    }
}
