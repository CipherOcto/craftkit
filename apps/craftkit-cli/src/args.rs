//! Argument parsing.
//!
//! Hand-written rather than derived. A CLI here has six subcommands and about
//! fifteen flags; a parser crate would be more code than this, and would make
//! the usage text a second place to keep in step.
//!
//! Two rules keep it honest:
//!
//! - **An unknown flag is an error.** A script that silently ignores
//!   `--output=foo.txt` and writes nothing is worse than one that stops.
//! - **Parsing never touches the filesystem.** It returns a [`Command`] and
//!   nothing else, so it can be tested without a disk.

use std::path::PathBuf;

/// Everything that can go wrong before any work starts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    #[error("`{0}` is not a subcommand; try help")]
    UnknownSubcommand(String),
    #[error("`{flag}` needs a value")]
    MissingValue { flag: String },
    #[error("`{value}` is not a value for `{flag}`")]
    BadValue { flag: String, value: String },
    #[error("`{0}` needs a file argument")]
    MissingArgument(String),
    #[error("`{flag}` was given twice")]
    Repeated { flag: String },
}

/// One subcommand, fully parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Print the command catalogue.
    Commands { json: bool },
    /// Run a script of commands.
    Run { script: PathBuf, sample: bool },
    /// Open a design and evaluate it.
    Eval { file: PathBuf, at: Option<String> },
    /// Write a sample design to a file.
    Sample { out: PathBuf },
    /// Write a script of commands to a file.
    Script { out: PathBuf },
    /// Run a script and render a frame.
    Snapshot { script: PathBuf, out: PathBuf, sample: bool },
    /// Read the crash folder.
    Crashes { action: CrashAction },
    /// Speak MCP over stdio.
    Mcp,
    /// Print the usage text.
    Help,
    /// Print the version.
    Version,
}

/// What to do with the crash folder — or with a report in a file.
#[derive(Debug, Clone, PartialEq)]
pub enum CrashAction {
    List,
    Show(String),
    /// Show the report in a **file** — one that arrived from another machine.
    ShowFile(PathBuf),
    Replay {
        id: String,
        /// Print the replay as a script instead of running it.
        script: bool,
    },
    /// Replay the report in a **file** — one that arrived from another machine.
    ReplayFile {
        path: PathBuf,
        /// Print the replay as a script instead of running it.
        script: bool,
    },
}

/// The subject of a crash door: a report in the crash folder, or a report in a
/// file.
///
/// An **id** names a report in *this machine's* crash folder. A **file** is a
/// report that arrived from somewhere else — the recipe asks for
/// `report.json` from whoever had the crash, and that file has no folder
/// around it and no id to look it up by. §195.
#[derive(Debug, Clone, PartialEq)]
enum Subject {
    Id(String),
    File(PathBuf),
}

/// Parse `argv` (without the program name).
///
/// `Ok(None)` means "help was printed and the process should stop".
pub fn parse(argv: &[String]) -> Result<Option<Command>, UsageError> {
    let Some(first) = argv.first() else {
        return Ok(Some(Command::Help));
    };

    match first.as_str() {
        "help" | "-h" | "--help" => Ok(Some(Command::Help)),
        // The version, the way the window and the browser build answer it.
        // The CLI is the program an agent drives, so it is the one whose
        // version matters most. §174.
        "-V" | "--version" => Ok(Some(Command::Version)),
        "commands" => {
            let (mut seen_json, mut json) = (false, false);
            for arg in &argv[1..] {
                match arg.as_str() {
                    "--json" => {
                        once(&mut seen_json, "--json")?;
                        json = true;
                    }
                    other => {
                        return Err(UsageError::BadValue { flag: "commands".into(), value: other.into() });
                    }
                }
            }
            Ok(Some(Command::Commands { json }))
        }
        "run" => {
            let script = positional(&argv[1..], "run")?;
            let (mut seen_sample, mut sample) = (false, false);
            for arg in &argv[2..] {
                match arg.as_str() {
                    "--sample" => {
                        once(&mut seen_sample, "--sample")?;
                        sample = true;
                    }
                    other => return Err(UsageError::BadValue { flag: "run".into(), value: other.into() }),
                }
            }
            Ok(Some(Command::Run { script, sample }))
        }
        "eval" => {
            let file = positional(&argv[1..], "eval")?;
            let (mut seen_at, mut at) = (false, None);
            let mut i = 2;
            while i < argv.len() {
                match argv[i].as_str() {
                    "--at" => {
                        let value = argv
                            .get(i + 1)
                            .ok_or_else(|| UsageError::MissingValue { flag: "--at".into() })?
                            .clone();
                        once(&mut seen_at, "--at")?;
                        at = Some(value);
                        i += 2;
                    }
                    other => {
                        return Err(UsageError::BadValue { flag: "eval".into(), value: other.into() });
                    }
                }
            }
            Ok(Some(Command::Eval { file, at }))
        }
        "sample" => {
            // Write a design to a file. The fuzzer's corpus comes from here:
            // the seed is written by the tree's own writer, through the
            // tree's own program, so the format the fuzzer breaks is the
            // format the program speaks. §180.
            let (mut seen_out, mut out) = (false, None);
            let mut i = 1;
            while i < argv.len() {
                match argv[i].as_str() {
                    "--out" => {
                        let value = argv
                            .get(i + 1)
                            .ok_or_else(|| UsageError::MissingValue { flag: "--out".into() })?;
                        once(&mut seen_out, "--out")?;
                        out = Some(PathBuf::from(value));
                        i += 2;
                    }
                    other => {
                        return Err(UsageError::BadValue { flag: "sample".into(), value: other.into() });
                    }
                }
            }
            let out = out.ok_or(UsageError::MissingValue { flag: "--out".into() })?;
            Ok(Some(Command::Sample { out }))
        }
        "script" => {
            // Write a script of commands. The tree had no writer for the
            // script format — the only producer printed a report's replay to
            // stdout — and §180's rule is that a fuzzer's corpus is written
            // by the tree's own program. §182.
            let (mut seen_out, mut out) = (false, None);
            let mut i = 1;
            while i < argv.len() {
                match argv[i].as_str() {
                    "--out" => {
                        let value = argv
                            .get(i + 1)
                            .ok_or_else(|| UsageError::MissingValue { flag: "--out".into() })?;
                        once(&mut seen_out, "--out")?;
                        out = Some(PathBuf::from(value));
                        i += 2;
                    }
                    other => {
                        return Err(UsageError::BadValue { flag: "script".into(), value: other.into() });
                    }
                }
            }
            let out = out.ok_or(UsageError::MissingValue { flag: "--out".into() })?;
            Ok(Some(Command::Script { out }))
        }
        "snapshot" => {
            let script = positional(&argv[1..], "snapshot")?;
            let (mut seen_out, mut out) = (false, None);
            let (mut seen_sample, mut sample) = (false, false);
            let mut i = 2;
            while i < argv.len() {
                match argv[i].as_str() {
                    "--out" => {
                        let value = argv
                            .get(i + 1)
                            .ok_or_else(|| UsageError::MissingValue { flag: "--out".into() })?;
                        once(&mut seen_out, "--out")?;
                        out = Some(PathBuf::from(value));
                        i += 2;
                    }
                    "--sample" => {
                        once(&mut seen_sample, "--sample")?;
                        sample = true;
                        i += 1;
                    }
                    other => {
                        return Err(UsageError::BadValue { flag: "snapshot".into(), value: other.into() });
                    }
                }
            }
            let out = out.ok_or(UsageError::MissingValue { flag: "--out".into() })?;
            Ok(Some(Command::Snapshot { script, out, sample }))
        }
        "crashes" => {
            let action = match argv.get(1).map(String::as_str) {
                None | Some("list") => CrashAction::List,
                Some("show") => {
                    let (subject, script) = crash_subject(argv, 2, "show")?;
                    if script {
                        return Err(UsageError::BadValue {
                            flag: "crashes show".into(),
                            value: "--script".into(),
                        });
                    }
                    match subject {
                        Subject::Id(id) => CrashAction::Show(id),
                        Subject::File(path) => CrashAction::ShowFile(path),
                    }
                }
                Some("replay") => {
                    let (subject, script) = crash_subject(argv, 2, "replay")?;
                    match subject {
                        Subject::Id(id) => CrashAction::Replay { id, script },
                        Subject::File(path) => CrashAction::ReplayFile { path, script },
                    }
                }
                Some(other) => {
                    return Err(UsageError::BadValue { flag: "crashes".into(), value: other.into() });
                }
            };
            Ok(Some(Command::Crashes { action }))
        }
        "mcp" => {
            if argv.len() > 1 {
                return Err(UsageError::BadValue { flag: "mcp".into(), value: argv[1].clone() });
            }
            Ok(Some(Command::Mcp))
        }
        other => Err(UsageError::UnknownSubcommand(other.to_string())),
    }
}

/// Reject a flag given twice.
///
/// A repeated `--out a --out b` is almost always a script that meant to do two
/// things, and silently honouring the last one is how a snapshot ends up in the
/// wrong place.
fn once(seen: &mut bool, flag: &str) -> Result<(), UsageError> {
    if *seen {
        return Err(UsageError::Repeated { flag: flag.to_string() });
    }
    *seen = true;
    Ok(())
}

fn positional(rest: &[String], command: &str) -> Result<PathBuf, UsageError> {
    rest.iter()
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| UsageError::MissingArgument(command.to_string()))
}

/// Parse the subject of a crash door, and whether the replay is wanted as a
/// script.
///
/// The subject is either an **id** — a report in this machine's crash folder —
/// or a **file**: a report that arrived from another machine. Exactly one of
/// them, and never both: an id and a file are not two spellings of one thing,
/// and a door that guesses which one it was handed cannot say what it read.
/// §195.
///
/// The `--script` flag may come before or after the subject: a person types
/// `crashes replay --script ID` as readily as `crashes replay ID --script`,
/// and a parser that insists on one order is a parser that refuses a command
/// that is perfectly clear.
fn crash_subject(argv: &[String], from: usize, command: &str) -> Result<(Subject, bool), UsageError> {
    let flag = format!("crashes {command}");
    let mut id: Option<String> = None;
    let mut file: Option<PathBuf> = None;
    let (mut seen_script, mut script) = (false, false);
    let mut seen_file = false;
    let mut i = from;
    while i < argv.len() {
        match argv[i].as_str() {
            "--script" => {
                once(&mut seen_script, "--script")?;
                script = true;
                i += 1;
            }
            "--file" => {
                once(&mut seen_file, "--file")?;
                let value =
                    argv.get(i + 1).ok_or_else(|| UsageError::MissingValue { flag: "--file".into() })?;
                file = Some(PathBuf::from(value));
                i += 2;
            }
            other if other.starts_with('-') => {
                return Err(UsageError::BadValue { flag: flag.clone(), value: other.into() });
            }
            other => {
                if id.is_some() {
                    return Err(UsageError::BadValue { flag: flag.clone(), value: other.into() });
                }
                id = Some(other.to_string());
                i += 1;
            }
        }
    }
    match (id, file) {
        (Some(id), None) => Ok((Subject::Id(id), script)),
        (None, Some(path)) => Ok((Subject::File(path), script)),
        (None, None) => Err(UsageError::MissingArgument(format!("crashes {command} <id> or --file <path>"))),
        (Some(_), Some(_)) => Err(UsageError::BadValue {
            flag: format!("crashes {command} <id> or --file <path>"),
            value: "an id and --file together".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn no_arguments_prints_help() {
        assert_eq!(parse(&argv(&[])), Ok(Some(Command::Help)));
    }

    #[test]
    fn help_is_asked_for_in_three_ways() {
        for form in ["help", "-h", "--help"] {
            assert_eq!(parse(&argv(&[form])), Ok(Some(Command::Help)), "{form}");
        }
    }

    /// The version is asked for in the two ways the window and the browser
    /// build accept it. §174.
    #[test]
    fn the_version_is_asked_for_in_two_ways() {
        for form in ["-V", "--version"] {
            assert_eq!(parse(&argv(&[form])), Ok(Some(Command::Version)), "{form}");
        }
    }

    #[test]
    fn commands_takes_an_optional_json_flag() {
        assert_eq!(parse(&argv(&["commands"])), Ok(Some(Command::Commands { json: false })));
        assert_eq!(parse(&argv(&["commands", "--json"])), Ok(Some(Command::Commands { json: true })));
    }

    #[test]
    fn run_needs_a_script_and_accepts_sample() {
        assert_eq!(
            parse(&argv(&["run", "s.json", "--sample"])),
            Ok(Some(Command::Run { script: PathBuf::from("s.json"), sample: true }))
        );
        assert!(matches!(parse(&argv(&["run"])), Err(UsageError::MissingArgument(_))));
    }

    #[test]
    fn eval_needs_a_file_and_takes_an_optional_at() {
        assert_eq!(
            parse(&argv(&["eval", "p.craftkit", "--at", "1.5"])),
            Ok(Some(Command::Eval { file: PathBuf::from("p.craftkit"), at: Some("1.5".into()) }))
        );
        assert!(matches!(parse(&argv(&["eval"])), Err(UsageError::MissingArgument(_))));
    }

    #[test]
    fn snapshot_needs_a_script_and_an_output() {
        assert_eq!(
            parse(&argv(&["snapshot", "s.json", "--out", "shot.png", "--sample"])),
            Ok(Some(Command::Snapshot {
                script: PathBuf::from("s.json"),
                out: PathBuf::from("shot.png"),
                sample: true,
            }))
        );
        assert_eq!(
            parse(&argv(&["snapshot", "s.json"])),
            Err(UsageError::MissingValue { flag: "--out".into() })
        );
    }

    #[test]
    fn sample_needs_an_output() {
        assert_eq!(
            parse(&argv(&["sample", "--out", "d.craftkit"])),
            Ok(Some(Command::Sample { out: PathBuf::from("d.craftkit") }))
        );
        assert_eq!(parse(&argv(&["sample"])), Err(UsageError::MissingValue { flag: "--out".into() }));
    }

    /// The script writer needs a file to write to, and takes nothing else.
    ///
    /// §182: the tree had no writer for the script format, and this is it.
    #[test]
    fn script_needs_an_output_and_takes_nothing_else() {
        assert_eq!(
            parse(&argv(&["script", "--out", "s.json"])),
            Ok(Some(Command::Script { out: PathBuf::from("s.json") }))
        );
        assert_eq!(parse(&argv(&["script"])), Err(UsageError::MissingValue { flag: "--out".into() }));
        assert!(parse(&argv(&["script", "--out", "s.json", "--wat"])).is_err());
        assert!(parse(&argv(&["script", "--out", "a.json", "--out", "b.json"])).is_err());
    }

    #[test]
    fn crashes_covers_list_show_and_replay() {
        assert_eq!(parse(&argv(&["crashes"])), Ok(Some(Command::Crashes { action: CrashAction::List })));
        assert_eq!(
            parse(&argv(&["crashes", "list"])),
            Ok(Some(Command::Crashes { action: CrashAction::List }))
        );
        assert_eq!(
            parse(&argv(&["crashes", "show", "20250907-0000-0000"])),
            Ok(Some(Command::Crashes { action: CrashAction::Show("20250907-0000-0000".into()) }))
        );
        assert_eq!(
            parse(&argv(&["crashes", "replay", "abc"])),
            Ok(Some(Command::Crashes { action: CrashAction::Replay { id: "abc".into(), script: false } }))
        );
    }

    /// The script flag may come before or after the id.
    #[test]
    fn the_script_flag_is_accepted_on_either_side_of_the_id() {
        let want = |id: &str| {
            Ok(Some(Command::Crashes { action: CrashAction::Replay { id: id.into(), script: true } }))
        };
        assert_eq!(parse(&argv(&["crashes", "replay", "abc", "--script"])), want("abc"));
        assert_eq!(parse(&argv(&["crashes", "replay", "--script", "abc"])), want("abc"));
    }

    /// A second id, an unknown flag, and a repeated flag are all refused.
    #[test]
    fn a_crash_replay_with_two_ids_or_a_strange_flag_is_refused() {
        assert!(parse(&argv(&["crashes", "replay", "abc", "def"])).is_err());
        assert!(parse(&argv(&["crashes", "replay", "abc", "--wat"])).is_err());
        assert!(parse(&argv(&["crashes", "replay", "abc", "--script", "--script"])).is_err());
    }

    /// A report in a file has its own door, and it is not the id's door.
    ///
    /// The recipe asks for `report.json` from whoever had the crash, so the
    /// file arrives from another machine — with no crash folder around it and
    /// no id to look it up by. §195.
    #[test]
    fn a_report_in_a_file_is_parsed_as_its_own_door() {
        assert_eq!(
            parse(&argv(&["crashes", "show", "--file", "report.json"])),
            Ok(Some(Command::Crashes { action: CrashAction::ShowFile(PathBuf::from("report.json")) }))
        );
        assert_eq!(
            parse(&argv(&["crashes", "replay", "--file", "report.json"])),
            Ok(Some(Command::Crashes {
                action: CrashAction::ReplayFile { path: PathBuf::from("report.json"), script: false }
            }))
        );
    }

    /// The file flag may come before or after the script flag, as the script
    /// flag may come before or after the id.
    #[test]
    fn the_file_flag_is_accepted_on_either_side_of_the_script_flag() {
        let want = |script: bool| {
            Ok(Some(Command::Crashes {
                action: CrashAction::ReplayFile { path: PathBuf::from("report.json"), script },
            }))
        };
        assert_eq!(parse(&argv(&["crashes", "replay", "--file", "report.json", "--script"])), want(true));
        assert_eq!(parse(&argv(&["crashes", "replay", "--script", "--file", "report.json"])), want(true));
    }

    /// An id and a file at once is refused.
    ///
    /// They are not two spellings of one thing — an id names a report in this
    /// machine's crash folder, and a file is a report that arrived from
    /// somewhere else — and a door that guesses which one it was handed
    /// cannot say what it read. §195.
    #[test]
    fn a_crash_door_with_both_an_id_and_a_file_is_refused() {
        assert!(parse(&argv(&["crashes", "replay", "abc", "--file", "report.json"])).is_err());
        assert!(parse(&argv(&["crashes", "show", "abc", "--file", "report.json"])).is_err());
    }

    /// The file flag needs a file, and `--script` is not a thing `show` does.
    #[test]
    fn a_crash_door_with_a_file_flag_and_no_file_is_refused() {
        assert!(parse(&argv(&["crashes", "replay", "--file"])).is_err());
        assert!(parse(&argv(&["crashes", "show", "--script", "abc"])).is_err());
        assert!(parse(&argv(&["crashes", "show", "--file"])).is_err());
    }

    #[test]
    fn show_without_an_id_is_refused_rather_than_defaulting() {
        assert!(parse(&argv(&["crashes", "show"])).is_err());
        assert!(parse(&argv(&["crashes", "replay"])).is_err());
    }

    #[test]
    fn an_unknown_subcommand_says_what_the_options_are() {
        let err = parse(&argv(&["frobnicate"])).expect_err("refused");
        assert!(matches!(err, UsageError::UnknownSubcommand(_)));
        assert!(err.to_string().contains("help"), "{err}");
    }

    #[test]
    fn an_unknown_flag_is_refused_rather_than_ignored() {
        assert!(parse(&argv(&["run", "s.json", "--output=x"])).is_err());
        assert!(parse(&argv(&["commands", "--jsn"])).is_err());
    }

    #[test]
    fn a_flag_given_twice_is_refused() {
        assert_eq!(
            parse(&argv(&["commands", "--json", "--json"])),
            Err(UsageError::Repeated { flag: "--json".into() })
        );
        assert!(parse(&argv(&["snapshot", "s.json", "--out", "a", "--out", "b"])).is_err());
    }

    #[test]
    fn mcp_takes_no_arguments() {
        assert_eq!(parse(&argv(&["mcp"])), Ok(Some(Command::Mcp)));
        assert!(parse(&argv(&["mcp", "--verbose"])).is_err());
    }

    #[test]
    fn every_error_names_the_thing_that_was_wrong() {
        for list in [
            vec!["frobnicate"],
            vec!["run"],
            vec!["eval"],
            vec!["snapshot", "s.json"],
            vec!["crashes", "explode"],
            vec!["commands", "--jsn"],
        ] {
            let err = parse(&argv(&list)).expect_err("refused");
            assert!(!err.to_string().is_empty(), "{list:?} produced an empty message");
        }
    }
}
