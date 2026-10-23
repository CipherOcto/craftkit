//! The sample design is a design the tree reads.
//!
//! `craftkit-cli sample --out` writes the sample design with the tree's own
//! writer, and the fuzzer's corpus is made of it. That is the point of the
//! command: a corpus written by hand would be a second copy of the file
//! format, and it would drift — the fuzzer would be breaking a format that
//! nothing else in the tree speaks.
//!
//! So the sample has to be a design the reader accepts. A corpus the reader
//! turns away tests nothing, and a fuzzer whose corpus is rubbish is green
//! for the wrong reason. §180.

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_craftkit-cli");

#[test]
fn the_sample_design_is_written_and_read_back_by_the_tree() {
    let dir = std::env::temp_dir().join(format!("craftkit-sample-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory to write into");
    let out = dir.join("sample.craftkit");

    let written = Command::new(BIN)
        .args(["sample", "--out", out.to_str().expect("a path")])
        .status()
        .expect("the CLI runs");
    assert!(written.success(), "`sample` did not write the design");
    assert!(out.is_file(), "the design was not written to {}", out.display());

    // And the tree reads it back. This is the half that matters: a sample
    // the reader refuses is a corpus that tests nothing.
    let read =
        Command::new(BIN).args(["eval", out.to_str().expect("a path")]).status().expect("the CLI runs");
    assert!(read.success(), "the sample design does not read back");

    let _ = std::fs::remove_dir_all(&dir);
}
