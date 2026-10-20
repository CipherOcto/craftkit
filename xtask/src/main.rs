//! `cargo xtask`.
//!
//! Everything is in the library; this is the binary that dispatches to it.
//! Run `cargo xtask --help` for the commands, or `cargo xtask doctor` to find
//! out whether this machine can build the project at all.

fn main() -> std::process::ExitCode {
    xtask::run()
}
