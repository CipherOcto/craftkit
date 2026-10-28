//! The script is a script the tree reads.
//!
//! `craftkit-cli script --out` writes a script with the tree's own writer, and
//! the fuzzer's corpus for the script door is made of it. §180's rule is that
//! a fuzzer's corpus is written by the tree's own program — a corpus written
//! by hand would be a second copy of the script format, and it would drift.
//!
//! The tree had **no writer for the script format** until §182: the only
//! producer was `crashes replay --script`, which prints a report's replay to
//! stdout, and a report needs a crash. So this test is the half that matters:
//! the script the tree writes has to be one the tree reads.
//!
//! And the script is the **sample design's own commands** — one copy, in
//! `craftkit_engine::sample_steps` — so the sample and the script cannot
//! drift apart.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_craftkit-cli");

#[test]
fn the_script_is_written_and_read_back_by_the_tree() {
    let dir = craftkit_foundation::temp::dir("script").expect("a directory to write into");
    let out = dir.join("script.json");

    let written = Command::new(BIN)
        .args(["script", "--out", out.to_str().expect("a path")])
        .status()
        .expect("the CLI runs");
    assert!(written.success(), "`script` did not write the script");
    assert!(out.is_file(), "the script was not written to {}", out.display());

    // And the tree reads it back — and *runs* it. A script the reader refuses
    // is a corpus that tests nothing, and a script the reader accepts but
    // cannot run is worse.
    let read = Command::new(BIN).args(["run", out.to_str().expect("a path")]).status().expect("the CLI runs");
    assert!(read.success(), "the script the tree wrote does not run");

    let _ = std::fs::remove_dir_all(&dir);
}

/// A failure the CLI prints carries the kind an agent branches on.
///
/// The tree splits its failures in two — a refusal is a message someone can
/// act on, and an internal error is worth a crash report and a bug — and
/// *an agent branches on the kind* (`docs/agent-playbook.md`, rule 11). The
/// control channel's reply has a `kind` field and the MCP's tool error has
/// one; the CLI is the program an agent drives, and it printed the message
/// alone, so an agent could not tell a refusal from a bug. §197.
#[test]
fn a_failure_the_cli_prints_carries_the_kind() {
    let dir = craftkit_foundation::temp::dir("script-kind").expect("a directory to write into");
    let path = dir.join("bad-parameter.json");
    std::fs::write(
        &path,
        r#"{"name":"t","commands":[{"command":"node.add","params":{"name":"A","kind":"cue","bogus":1}}]}"#,
    )
    .expect("the script is written");

    let out = Command::new(BIN).args(["run", path.to_str().expect("a path")]).output().expect("the CLI runs");
    assert!(!out.status.success(), "a script with a bad parameter must be refused");
    let text = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(text.contains("bad-parameter"), "the failure did not name its kind: {text:?}");
    // And the message is still there: the kind says which sort of failure it
    // is, not what went wrong.
    assert!(text.contains("bogus"), "the failure did not say what went wrong: {text:?}");

    // **The other door, and the one that had the defect.** §275: `snapshot`
    // printed the message alone -- no kind, and no step number -- so an
    // agent asking for evidence could not tell a refusal from a bug, and
    // could not say which step refused.
    let shot = dir.join("shot.png");
    let out = Command::new(BIN)
        .args(["snapshot", path.to_str().expect("a path"), "--out", shot.to_str().expect("a path")])
        .output()
        .expect("the CLI runs");
    assert!(!out.status.success(), "a script with a bad parameter must be refused");
    let text = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(text.contains("bad-parameter"), "the failure did not name its kind: {text:?}");
    assert!(text.contains("bogus"), "the failure did not say what went wrong: {text:?}");
    assert!(text.contains("step 1/1"), "the failure did not say which step: {text:?}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_script_is_the_sample_designs_own_commands() {
    // One copy. The sample design is built through three `node.add`
    // commands, and the script is those three — so a change to the sample
    // changes the script, and the fuzzer's corpus stays the tree's format.
    let dir = craftkit_foundation::temp::dir("script-steps").expect("a directory to write into");
    let out = dir.join("script.json");

    let written = Command::new(BIN)
        .args(["script", "--out", out.to_str().expect("a path")])
        .status()
        .expect("the CLI runs");
    assert!(written.success(), "`script` did not write the script");

    let text = std::fs::read_to_string(&out).expect("the script reads");
    let value: serde_json::Value = serde_json::from_str(&text).expect("the script is JSON");
    let commands = value["commands"].as_array().expect("`commands` is an array");
    assert_eq!(commands.len(), 3, "the sample is three commands and the script is not");
    for step in commands {
        assert_eq!(step["command"], "node.add", "the sample is built with `node.add`");
    }
    // The names are the sample's own names.
    let names: Vec<&str> = commands.iter().filter_map(|s| s["params"]["name"].as_str()).collect();
    assert_eq!(names, ["Level", "Taper", "Out"], "the script is not the sample's");

    let _ = std::fs::remove_dir_all(&dir);
}
