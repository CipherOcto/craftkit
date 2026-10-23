//! The subcommands themselves.
//!
//! Every one of them ends in `Session::execute`, `Session::document` or a
//! `craftkit_engine` query. There is no path from an argument here to a change
//! in the document that does not go through a command, which is what makes the
//! CLI and the window the same application rather than two similar ones.

use std::path::Path;
use std::sync::{Arc, Mutex};

use craftkit_engine::Session;
use serde_json::{Value as Json, json};

use crate::args::{CrashAction, parse};

/// A parsed command line.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    command: crate::args::Command,
}

impl Cli {
    /// Parse `argv`, which does not include the program name.
    ///
    /// `Ok(None)` means help was printed and the process should stop.
    pub fn parse(argv: &[String]) -> Result<Option<Self>, crate::args::UsageError> {
        parse(argv).map(|c| c.map(|command| Cli { command }))
    }

    /// Do the work. Returns an error string the caller prints.
    pub fn run(&self) -> Result<(), String> {
        match &self.command {
            crate::args::Command::Help => {
                println!("{}", crate::usage());
                Ok(())
            }
            crate::args::Command::Commands { json: as_json } => commands(*as_json),
            crate::args::Command::Run { script, sample } => run_script(script, *sample),
            crate::args::Command::Eval { file, at } => eval(file, at.as_deref()),
            crate::args::Command::Snapshot { script, out, sample } => snapshot(script, out, *sample),
            crate::args::Command::Crashes { action } => crashes(action),
            crate::args::Command::Mcp => mcp(),
            crate::args::Command::Version => {
                println!("{}", version_line());
                Ok(())
            }
        }
    }
}

/// The line `--version` prints.
///
/// **The version is the crate's, not a written-out one.** It comes from
/// `env!("CARGO_PKG_VERSION")`, which is the crate's inherited
/// `[workspace.package] version` — so it is right by construction, and there
/// is no second copy to forget when the tree is released. §174.
pub fn version_line() -> String {
    format!("craftkit-cli {}", env!("CARGO_PKG_VERSION"))
}

// -------------------------------------------------------------- commands --

fn commands(as_json: bool) -> Result<(), String> {
    let catalogue = craftkit_engine::catalogue();
    if as_json {
        println!("{}", serde_json::to_string_pretty(&catalogue).map_err(|e| e.to_string())?);
        return Ok(());
    }
    let list = catalogue.as_array().map(Vec::as_slice).unwrap_or_default();
    let mut area = "";
    for spec in list {
        let spec_area = spec["area"].as_str().unwrap_or_default();
        if spec_area != area {
            area = spec_area;
            println!("\n{spec_area}:");
        }
        let id = spec["id"].as_str().unwrap_or("?");
        let label = spec["label"].as_str().unwrap_or("");
        println!("  {id:<22} {label}");
        println!("      {}", spec["doc"].as_str().unwrap_or(""));
        for line in param_lines(spec) {
            println!("{line}");
        }
    }
    Ok(())
}

/// The lines the catalogue prints for one command's parameters.
///
/// A **pair** of parameters of which one must be given prints as one line,
/// and the second spelling does not print a line of its own — two lines that
/// both say "may be left out" would say that neither is needed, and the
/// command refuses without one. §120.
fn param_lines(spec: &Json) -> Vec<String> {
    let Some(params) = spec["params"].as_array() else { return Vec::new() };
    if params.is_empty() {
        return Vec::new();
    }
    let groups: Vec<Vec<String>> = spec["one_of"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|g| {
            g.as_array().map(|names| names.iter().filter_map(|n| n.as_str().map(str::to_string)).collect())
        })
        .collect();
    let mut lines = Vec::new();
    for p in params {
        let name = p["name"].as_str().unwrap_or("");
        let group = groups.iter().find(|g| g.iter().any(|n| n == name));
        // The second spelling of a pair is part of the first's line.
        if group.is_some_and(|g| g.first().map(String::as_str) != Some(name)) {
            continue;
        }
        lines.push(param_line(p, group.map(|g| g.as_slice())));
    }
    lines
}

/// The line the catalogue prints for one parameter, or for a pair of them.
///
/// A `?` marks a parameter that may be left out. A **pair** of parameters of
/// which one must be given prints as one line with both names, because two
/// lines that both say "may be left out" say that neither is needed — and
/// the command refuses without one. §120.
fn param_line(param: &Json, group: Option<&[String]>) -> String {
    let name = param["name"].as_str().unwrap_or("?");
    let kind = param["type"].as_str().unwrap_or("?");
    let help = param["help"].as_str().unwrap_or("");
    if let Some(group) = group {
        let names = group.join("|");
        return format!("      {names}: {kind} — {help} (one of the two is required)");
    }
    let required = if param["required"].as_bool().unwrap_or(false) { "" } else { "?" };
    let alternative = match param["alternative"].as_str() {
        Some(other) => format!(" (or give `{other}`)"),
        None => String::new(),
    };
    format!("      {required}{name}: {kind} — {help}{alternative}")
}

// ------------------------------------------------------------------ run --

/// A script: a name, a list of commands, and optionally the document they
/// start from.
///
/// The same shape a crash report's `replay.json` uses, so a report can be run
/// back through `craftkit-cli run` with nothing in between — and the `base`
/// field is what makes that true for a report whose commands were relative to
/// a loaded document. A script without a base starts from an empty document.
#[derive(Debug, serde::Deserialize)]
struct Script {
    #[serde(default)]
    name: String,
    commands: Vec<Step>,
    /// The document the commands are relative to, if there is one.
    #[serde(default)]
    base: Option<Json>,
}

#[derive(Debug, serde::Deserialize)]
struct Step {
    command: String,
    /// An omitted `params` means "no parameters", which is the empty object —
    /// not `null`. Defaulting to `Json::Null` would hand the engine a value
    /// that every typed `get()` rejects, so `{"command":"plan.evaluate"}`
    /// would fail instead of running.
    #[serde(default = "empty_object")]
    params: Json,
}

fn empty_object() -> Json {
    Json::Object(Default::default())
}

/// The largest script, in bytes of text.
///
/// A limit on **text**. The parser then turns that text into a tree, and the
/// tree is up to [`craftkit_core::JSON_EXPANSION`] times bigger — a JSON
/// number is two bytes of text and thirty-two bytes of memory, and the
/// vector holding a long list grows to twice its length. Measured: a
/// 32 000 063-byte script cost 1 034 584 KiB of resident memory, a factor of
/// thirty-two, and the limit below is what keeps that inside
/// [`craftkit_core::MAX_JSON_MEMORY_BYTES`]. §64.
///
/// It used to be 32 MiB, which bought a gigabyte of memory for a file the
/// user pointed at. A script that carries a base document bigger than this
/// cannot be written any more — [`replay_as_script`] says so rather than
/// printing a script that would replay onto the wrong document.
pub const MAX_SCRIPT_BYTES: usize = craftkit_core::MAX_JSON_TEXT_BYTES;

fn load_script(path: &Path) -> Result<Script, String> {
    // Bounded, because a script is read from wherever a caller pointed at and a
    // hostile one is still hostile input. The limit is applied **while
    // reading**, by the shared bounded reader, so a script of any size costs a
    // buffer rather than its whole size in memory — a cap checked after the
    // read has already paid for what it was meant to prevent. §49.
    //
    // The limit bounds the *text*; the parse of it costs up to
    // `craftkit_core::JSON_EXPANSION` times as much, which is why the number
    // is the one the whole tree uses. §64.
    let body = craftkit_core::read_file_bounded(path, MAX_SCRIPT_BYTES)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    serde_json::from_str(&body).map_err(|e| format!("{} is not a script: {e}", path.display()))
}

fn run_script(path: &Path, sample: bool) -> Result<(), String> {
    let script = load_script(path)?;
    let mut session = if sample { Session::sample() } else { Session::new(&script.name) };
    // The base is moved, not lent: it is raw JSON, and raw JSON costs
    // thirty-two times its own size in memory, so a second copy of it is a
    // second copy of a gigabyte. §64.
    if let Some(base) = script.base {
        session.load_document(craftkit_engine::crash::document_from_json(base)?);
    }
    let steps = script.commands.len();
    for (i, step) in script.commands.iter().enumerate() {
        match session.execute(&step.command, &step.params) {
            Ok(outcome) => println!("{}/{steps} {}: {}", i + 1, step.command, outcome.message),
            Err(e) => {
                // The engine's own message already names the command, so this
                // prefix is only the step number — repeating "`key.set` failed:
                // `key.set` rejected…" helps nobody.
                return Err(format!("step {}/{steps}: {e}", i + 1));
            }
        }
    }
    println!(
        "{} — {} step(s), {} node(s)",
        if script.name.is_empty() { "script" } else { &script.name },
        steps,
        session.document().nodes.len()
    );
    Ok(())
}

// ----------------------------------------------------------------- eval --

fn eval(file: &Path, at: Option<&str>) -> Result<(), String> {
    let doc = craftkit_io::load(file).map_err(|e| e.to_string())?;
    let mut session = Session::new(doc.meta.name.clone());
    session.load_document(doc);

    let outcome = match at {
        Some(text) => session.execute("playback.seek", &json!({ "seconds": text })),
        None => Ok(craftkit_engine::Outcome::new("left where it was")),
    }
    .map_err(|e| e.to_string())?;

    let plan = session.execute("plan.evaluate", &json!({})).map_err(|e| e.to_string())?;
    let summary = session.execute("diagnostics.list", &json!({})).map(|o| o.data).unwrap_or(Json::Null);

    println!("{}", outcome.message);
    println!(
        "{} node(s), {} frame(s) at {} fps",
        session.document().nodes.len(),
        session.document().meta.duration().frames_i64(session.document().meta.frame_rate()),
        session.document().meta.fps
    );
    // The design's note, if it has one. §110: it was in the file and in the
    // model and in nothing else, so `eval` on a design with a note never
    // mentioned the note.
    if let Some(line) = note_line(&session.document().meta.comment) {
        println!("{line}");
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "plan": plan.data,
            "diagnostics": summary,
        }))
        .map_err(|e| e.to_string())?
    );
    Ok(())
}

/// The line `eval` prints about a design's note, if the design has one.
///
/// A note is a paragraph, so it is printed whole: the note is bounded (the
/// model's `MAX_COMMENT_BYTES`), so the line is bounded too. A note that is
/// only whitespace is no note at all. §110.
fn note_line(note: &str) -> Option<String> {
    let note = note.trim();
    if note.is_empty() { None } else { Some(format!("note: {note}")) }
}

// ------------------------------------------------------------- snapshot --

fn snapshot(script: &Path, out: &Path, sample: bool) -> Result<(), String> {
    let loaded = load_script(script)?;
    let mut session = if sample { Session::sample() } else { Session::new(&loaded.name) };
    if let Some(base) = loaded.base {
        session.load_document(craftkit_engine::crash::document_from_json(base)?);
    }
    for step in &loaded.commands {
        session
            .execute(&step.command, &step.params)
            // As above: the error names its own command.
            .map_err(|e| e.to_string())?;
    }
    render(&mut session, out)
}

/// The size a snapshot is taken at.
///
/// Fixed rather than derived from the document: a screenshot an agent can
/// compare against another one needs the same frame every time, and a design
/// that changed size would otherwise change the picture.
/// The frame size `snapshot` renders, matching a laptop window.
const SNAPSHOT_SIZE: (usize, usize) =
    (craftkit_ui::raster::DEFAULT_SIZE[0], craftkit_ui::raster::DEFAULT_SIZE[1]);

/// Draw one frame of the real shell and write it as a PNG.
///
/// The shell is drawn with nothing selected: a snapshot is a picture of a
/// design, not of a person looking at one, so there is no selection to carry.
/// The properties panel is therefore empty in the picture, and it says so. §106.
#[cfg(not(target_arch = "wasm32"))]
fn render(session: &mut Session, out: &Path) -> Result<(), String> {
    let (width, height) = SNAPSHOT_SIZE;
    let state = craftkit_ui::shell::ShellState::default();
    let frame = craftkit_ui::raster::render(session, width, height, &state)?;
    let png = frame.png;

    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(out, png).map_err(|e| format!("could not write {}: {e}", out.display()))?;
    println!("wrote {}x{} to {}", width, height, out.display());
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn render(_session: &mut Session, _out: &Path) -> Result<(), String> {
    Err("snapshot needs a rasteriser; run the native build".to_string())
}

// -------------------------------------------------------------- crashes --

/// Print a report's replay as a script.
///
/// The script carries the report's **base** as well as its commands, so
/// running it reproduces the document the crash happened in — not a document
/// that merely came from the same commands. A script without a base starts
/// from an empty document, and a report whose commands were relative to a
/// loaded file would then build something else entirely. §47.
fn replay_as_script(id: &str) -> Result<(), String> {
    let report = craftkit_engine::crash::show(id)?;
    let commands: Vec<Json> =
        report.replay.iter().map(|step| json!({ "command": step.command, "params": step.params })).collect();
    let script = json!({
        "name": format!("replay of {id}"),
        "base": report.base,
        "commands": commands,
    });
    let text = serde_json::to_string_pretty(&script).map_err(|e| e.to_string())?;
    // The script has to be one `craftkit-cli run` can read back, and the
    // reader's limit is a limit on text. A report whose base is too big to
    // carry in a script says so here, rather than printing a script that
    // would be refused when it was run — and rather than printing one
    // without its base, which would replay onto the wrong document. §64.
    if text.len() > MAX_SCRIPT_BYTES {
        return Err(format!(
            "the replay of {id} is {} bytes of script, and the limit is {MAX_SCRIPT_BYTES}: \
             the base document is too large to carry in a script. \
             Run `craftkit-cli crashes replay {id}` to replay it directly.",
            text.len()
        ));
    }
    println!("{text}");
    Ok(())
}

fn crashes(action: &CrashAction) -> Result<(), String> {
    match action {
        CrashAction::List => {
            let ids = craftkit_engine::crash::list();
            if ids.is_empty() {
                println!("no crash reports in {}", craftkit_engine::crash::report_dir().display());
            }
            for id in ids {
                println!("{id}");
            }
            Ok(())
        }
        CrashAction::Show(id) => {
            let report = craftkit_engine::crash::show(id)?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
            Ok(())
        }
        CrashAction::Replay { id, script } => {
            if *script {
                return replay_as_script(id);
            }
            let outcome = craftkit_engine::crash::reproduce(id)?;
            for step in &outcome.results {
                if step.ok {
                    println!("{}: {}", step.command, step.message);
                } else {
                    println!("{}: FAILED — {}", step.command, step.message);
                }
            }
            println!("\n{} step(s) replayed onto the design the report recorded", outcome.steps);
            if outcome.dropped > 0 {
                println!(
                    "the report had to drop {} command(s), so this is the tail of the session and not all of it",
                    outcome.dropped
                );
            }
            if outcome.reproduced {
                println!("the replay reproduced the design in the report");
            } else {
                println!("the replay DIVERGED from the design in the report:");
                for difference in &outcome.differences {
                    println!("  • {difference}");
                }
            }
            Ok(())
        }
    }
}

// ------------------------------------------------------------------ mcp --

fn mcp() -> Result<(), String> {
    let session = Arc::new(Mutex::new(Session::sample()));
    craftkit_mcp::serve(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A pair of parameters prints as one line that says one of the two is
    /// required.
    ///
    /// §120: the pair used to print as two lines, each marked with a `?` —
    /// and two lines that both say "may be left out" say that neither is
    /// needed, which is the opposite of the truth. The command refuses when
    /// it has neither.
    #[test]
    fn a_pair_prints_as_one_line_that_says_one_of_the_two_is_required() {
        let group = vec!["node".to_string(), "node_name".to_string()];
        let primary = json!({
            "name": "node",
            "type": "node-ref",
            "required": false,
            "help": "The node's id.",
            "alternative": "node_name",
        });
        let line = param_line(&primary, Some(&group));
        assert!(line.contains("node|node_name"), "the line does not name both spellings: {line}");
        assert!(
            line.contains("one of the two is required"),
            "the line does not say the pair is needed: {line}"
        );
        assert!(!line.contains('?'), "the line marks the pair as optional: {line}");
    }

    /// A plain parameter prints as it always did.
    #[test]
    fn a_plain_parameter_prints_as_before() {
        let plain = json!({
            "name": "name",
            "type": "text",
            "required": true,
            "help": "The new name.",
        });
        let line = param_line(&plain, None);
        assert!(line.starts_with("      name:"), "the line does not mark it required: {line}");
        assert!(!line.contains("or give"), "a plain parameter grew an alternative: {line}");
        assert!(!line.contains("one of the two"), "a plain parameter grew a pair: {line}");
    }

    /// An optional parameter is still marked with a `?`.
    #[test]
    fn an_optional_parameter_is_marked_with_a_question_mark() {
        let optional = json!({
            "name": "muted",
            "type": "bool",
            "required": false,
            "help": "True to mute.",
        });
        let line = param_line(&optional, None);
        assert!(line.starts_with("      ?muted:"), "the line does not mark it optional: {line}");
    }

    /// A parameter with an alternative says which other spelling may be given
    /// instead.
    ///
    /// §116: the catalogue marked the id as required, so the line said
    /// nothing about the name — and the name is what the command accepts.
    /// §120 prints the pair as one line, so the test drives the whole
    /// listing rather than one parameter's line.
    #[test]
    fn a_parameter_with_an_alternative_says_which_one() {
        let spec = json!({
            "params": [
                { "name": "node", "type": "node-ref", "required": false,
                  "help": "The node's id, or its name.", "alternative": "node_name" },
                { "name": "node_name", "type": "text", "required": false,
                  "help": "…or its name." }
            ],
            "one_of": [["node", "node_name"]],
        });
        let lines = param_lines(&spec);
        assert!(lines[0].contains("node_name"), "the line does not name the alternative: {lines:?}");
    }

    /// A pair prints as one line, and the second spelling does not print a
    /// line of its own.
    ///
    /// §120: the pair used to print as two lines, each marked with a `?` —
    /// and two lines that both say "may be left out" say that neither is
    /// needed, which is the opposite of the truth.
    #[test]
    fn a_pair_prints_as_one_line_and_the_second_spelling_does_not_print_its_own() {
        let spec = json!({
            "id": "node.mute",
            "params": [
                { "name": "node", "type": "node-ref", "required": false,
                  "help": "The node's id, or its name.", "alternative": "node_name" },
                { "name": "node_name", "type": "text", "required": false,
                  "help": "…or its name." }
            ],
            "one_of": [["node", "node_name"]],
        });
        let lines = param_lines(&spec);
        assert_eq!(lines.len(), 1, "the pair printed more than one line: {lines:?}");
        assert!(lines[0].contains("node|node_name"), "{}", lines[0]);
        assert!(lines[0].contains("one of the two is required"), "{}", lines[0]);
    }

    /// The catalogue says, for every command, which pairs one of must be
    /// given.
    ///
    /// §120: the catalogue's JSON said `required: false` for both spellings
    /// of a pair and nothing else, so the only face of the catalogue that
    /// said "one of the two" was the MCP's JSON Schema.
    #[test]
    fn the_catalogue_says_which_pairs_one_of_must_be_given() {
        let catalogue = craftkit_engine::catalogue();
        let list = catalogue.as_array().expect("the catalogue is a list");
        let mut pairs = 0;
        for spec in list {
            let groups = spec["one_of"].as_array().expect("every command says its pairs");
            for group in groups {
                let names = group.as_array().expect("a group is a list of names");
                assert!(names.len() >= 2, "a group of one is not a pair: {names:?}");
                pairs += 1;
                // Every name in a pair is a parameter of the command, and
                // neither half is required on its own.
                for name in names {
                    let name = name.as_str().expect("a name");
                    let param = spec["params"]
                        .as_array()
                        .expect("params")
                        .iter()
                        .find(|p| p["name"] == name)
                        .unwrap_or_else(|| panic!("`{name}` is not a parameter of {}", spec["id"]));
                    assert_eq!(
                        param["required"],
                        json!(false),
                        "`{name}` is required on its own, so it is not a pair"
                    );
                }
            }
        }
        assert!(pairs >= 13, "only {pairs} pair(s) in the catalogue");
    }

    fn temp(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join("craftkit-cli-tests").join(format!("{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write_script(tag: &str, body: &str) -> PathBuf {
        let path = temp(tag).join("script.json");
        std::fs::write(&path, body).expect("write");
        path
    }

    #[test]
    fn a_script_runs_its_commands_in_order() {
        let path = write_script(
            "run",
            r#"{"name":"ramp","commands":[
                {"command":"node.add","params":{"name":"In","kind":"source","amount":1}},
                {"command":"node.add","params":{"name":"Out","kind":"sink"}}
            ]}"#,
        );
        let mut session = Session::new("test");
        let script = load_script(&path).expect("loads");
        assert_eq!(script.name, "ramp");
        for step in &script.commands {
            session.execute(&step.command, &step.params).expect("runs");
        }
        assert_eq!(session.document().nodes.len(), 2);
    }

    #[test]
    fn a_script_may_omit_the_name() {
        let path = write_script("noname", r#"{"commands":[]}"#);
        let script = load_script(&path).expect("loads");
        assert!(script.name.is_empty());
        assert!(script.commands.is_empty());
    }

    #[test]
    fn a_script_may_omit_the_params() {
        let path = write_script("noparams", r#"{"commands":[{"command":"plan.evaluate"}]}"#);
        let script = load_script(&path).expect("loads");
        assert_eq!(script.commands[0].params, Json::Object(Default::default()));
    }

    #[test]
    fn a_malformed_script_is_refused_by_name() {
        let path = write_script("bad", "{ not json");
        let err = load_script(&path).expect_err("refused");
        assert!(err.contains("is not a script"), "{err}");
        assert!(err.contains("script.json"), "{err}");
    }

    /// The script limit is the tree's limit, not a number of its own.
    ///
    /// The three readers of untrusted JSON had three unrelated limits — 1 MiB,
    /// 4 MiB and 32 MiB — and the largest of them was thirty-two times
    /// looser than the memory it was meant to bound: a 32 000 063-byte script
    /// cost 1 034 584 KiB of resident memory. §64.
    #[test]
    fn the_script_limit_is_the_tree_s_json_limit() {
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same trick the model's limit test uses.
        let (script, expansion, budget) =
            (MAX_SCRIPT_BYTES, craftkit_core::JSON_EXPANSION, craftkit_core::MAX_JSON_MEMORY_BYTES);
        assert_eq!(script, craftkit_core::MAX_JSON_TEXT_BYTES);
        assert!(
            script * expansion <= budget,
            "a script of {script} bytes can cost {} bytes",
            script * expansion
        );
    }

    /// A script over the limit is refused, and the message says what the limit
    /// is.
    ///
    /// The old limit was 32 MiB, and a script at the limit cost a gigabyte of
    /// memory. Now the limit is the one the whole tree uses, and the message
    /// names it so that the number in the message is the number in the code.
    /// §64.
    #[test]
    fn an_oversized_script_is_refused_with_the_limit_in_the_message() {
        let big =
            format!("{{\"commands\":[{{\"command\":\"node.add\",\"params\":[{}]}}]}}", "1,".repeat(600_000));
        let path = write_script("oversized", &big);
        let err = load_script(&path).expect_err("refused");
        assert!(err.contains(&MAX_SCRIPT_BYTES.to_string()), "{err}");
        assert!(err.contains("could not read"), "{err}");
    }

    #[test]
    fn a_missing_script_is_refused_by_name() {
        let err = load_script(Path::new("/nonexistent/script.json")).expect_err("refused");
        assert!(err.contains("script.json"), "{err}");
    }

    #[test]
    fn a_script_step_that_is_not_an_object_is_refused() {
        let path = write_script("steps", r#"{"commands":["node.add"]}"#);
        assert!(load_script(&path).is_err());
    }

    #[test]
    fn a_script_without_commands_is_refused_rather_than_running_nothing() {
        let path = write_script("nocommands", r#"{"name":"nothing"}"#);
        assert!(load_script(&path).is_err(), "a script with no steps is a mistake, not a no-op");
    }

    #[test]
    fn a_replay_file_and_a_script_are_the_same_shape() {
        // This is the property that makes a crash report runnable: the report
        // writes a list of `{command, params}` and this is exactly that shape.
        let report = json!([{ "command": "node.add", "params": { "name": "A" } }]);
        let as_script = json!({ "name": "from a report", "commands": report });
        let path = write_script("replayshape", &as_script.to_string());
        let script = load_script(&path).expect("a replay.json runs as a script");
        assert_eq!(script.commands.len(), 1);
        assert_eq!(script.commands[0].command, "node.add");
    }

    #[test]
    fn an_oversized_script_is_refused_before_it_is_parsed() {
        let path = temp("huge").join("big.json");
        let filler = "x".repeat(33 * 1024 * 1024);
        std::fs::write(&path, format!(r#"{{"commands":[],"name":"{filler}"}}"#)).expect("write");
        let err = load_script(&path).expect_err("refused");
        assert!(err.contains("limit is"), "{err}");
    }

    #[test]
    fn the_cli_exposes_the_whole_catalogue() {
        // `commands` prints; the point of the test is that the catalogue it
        // prints is the registry, with nothing filtered out.
        let list = craftkit_engine::catalogue();
        let arr = list.as_array().expect("an array");
        assert_eq!(arr.len(), craftkit_engine::all().len());
    }

    /// `eval` says what the design's note is.
    ///
    /// §110: the note was in the file and in the model and in nothing else,
    /// so `eval` on a design with a note printed the node count and the frame
    /// rate and never the note. A note that only the file can show is a note
    /// nobody reads.
    #[test]
    fn eval_says_what_the_design_s_note_is() {
        assert_eq!(note_line(""), None, "a design with no note has no note line");
        assert_eq!(note_line("   \n  "), None, "whitespace is not a note");
        assert_eq!(
            note_line("why the design is the way it is"),
            Some("note: why the design is the way it is".to_string())
        );
        assert_eq!(
            note_line("  a note with its edges trimmed  "),
            Some("note: a note with its edges trimmed".to_string())
        );
    }

    // ------------------------------------------------------- the dispatcher --
    //
    // Every test above reaches past `Cli::run` and calls `load_script` or builds
    // a `Session` directly, which is comfortable and proves the *format* loads.
    // It proves nothing about whether `craftkit-cli run script.json` runs — the
    // function an agent actually invokes was called from `main.rs` and nowhere
    // else, so `cli.rs` sat at 38% coverage with `run_script`, `eval`,
    // `snapshot` and `crashes` never executed by a test at all.
    //
    // These go through `parse` as well, so the argv an agent types is the argv
    // under test. The one arm left out is `mcp`: it blocks on stdio, and a test
    // that had to be killed would prove less than the gap it closed.

    /// Parse `words` and run the result, the way `main` does.
    fn dispatch(words: &[&str]) -> Result<(), String> {
        let argv: Vec<String> = words.iter().map(|w| (*w).to_string()).collect();
        let cli = Cli::parse(&argv).map_err(|e| e.to_string())?.expect("not help");
        cli.run()
    }

    #[test]
    fn the_dispatcher_runs_a_script_from_argv() {
        let path = write_script(
            "dispatch-run",
            r#"{"name":"via argv","commands":[
                {"command":"node.add","params":{"name":"In","kind":"source","amount":1}},
                {"command":"node.add","params":{"name":"Out","kind":"sink"}}
            ]}"#,
        );
        dispatch(&["run", path.to_str().expect("utf-8")]).expect("the script runs");
    }

    #[test]
    fn the_dispatcher_reports_which_step_failed() {
        // The step number is the only thing the engine's message does not
        // already carry, so it is the part worth asserting.
        let path = write_script(
            "dispatch-fail",
            r#"{"name":"half","commands":[
                {"command":"node.add","params":{"name":"In","kind":"source","amount":1}},
                {"command":"node.add","params":{"name":"Broken","kind":"nonsense"}}
            ]}"#,
        );
        let err = dispatch(&["run", path.to_str().expect("utf-8")]).expect_err("the second step fails");
        assert!(err.starts_with("step 2/2:"), "{err}");
    }

    #[test]
    fn the_dispatcher_renders_a_snapshot_to_disk() {
        let path = write_script(
            "dispatch-snap",
            r#"{"name":"shot","commands":[
                {"command":"node.add","params":{"name":"In","kind":"source","amount":1}},
                {"command":"node.add","params":{"name":"Out","kind":"sink"}}
            ]}"#,
        );
        let out = temp("dispatch-snap").join("frame.png");
        dispatch(&["snapshot", path.to_str().expect("utf-8"), "--out", out.to_str().expect("utf-8")])
            .expect("renders");

        let png = std::fs::read(&out).expect("a file was written");
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "not a PNG: {} bytes", png.len());
        assert!(
            png.len() > 1024,
            "a {width}x{height} frame that small is blank",
            width = SNAPSHOT_SIZE.0,
            height = SNAPSHOT_SIZE.1
        );
    }

    #[test]
    fn the_dispatcher_opens_a_saved_design() {
        // `eval` is the documented way to check a `.craftkit` file without the
        // window, so it is the CLI path a design file's format is judged by.
        let session = Session::sample();
        let file = temp("dispatch-eval").join("sample.craftkit");
        craftkit_io::save(&file, session.document()).expect("a design this CLI wrote back");

        dispatch(&["eval", file.to_str().expect("utf-8")]).expect("the saved design opens");
    }

    #[test]
    fn the_dispatcher_reads_the_crash_folder() {
        // `list` is the first thing anybody runs after hearing "it crashed".
        dispatch(&["crashes", "list"]).expect("the crash folder is readable");
    }

    #[test]
    fn the_dispatcher_refuses_a_crash_id_it_does_not_have() {
        let err = dispatch(&["crashes", "show", "no-such-report"]).expect_err("an id that was never written");
        assert!(!err.is_empty(), "an empty message helps nobody: {err:?}");
    }

    #[test]
    fn a_script_that_a_person_typed_reaches_the_same_errors() {
        // parse + run, on something that is not a script at all: the two halves
        // together are what a mistyped path actually goes through.
        let err = dispatch(&["run", "/nonexistent/script.json"]).expect_err("refused");
        assert!(err.contains("script.json"), "{err}");
    }
}
