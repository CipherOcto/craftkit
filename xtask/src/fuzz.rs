//! `cargo xtask fuzz` — the design-file reader, fuzzed.
//!
//! # What it does
//!
//! It writes a corpus of **valid** design files with the tree's own program,
//! breaks each one in one of seven ways, and hands every broken one to the
//! headless CLI. What the CLI does with it is the question.
//!
//! A design file is the widest door in the program: it is the one thing a
//! user can hand the program by accident — a file from another program, a
//! file half-written by a crash, a file somebody edited in a text editor.
//! Every other input the tree takes is a parameter it checked first.
//!
//! # Why a child process, and not a call in this process
//!
//! Two reasons, and both of them are reasons a check has to be able to fail:
//!
//! - **A hang is only visible from outside.** A reader that loops forever
//!   cannot time itself out; a parent can kill it, and the killing is the
//!   evidence.
//! - **The exit code is the verdict, and it says where the panic was.** The
//!   engine runs every command under `catch_unwind`, so a panic *inside a
//!   command* is turned into an ordinary error and the process still exits
//!   cleanly. The one place a panic still shows is where nothing catches it:
//!   the reader, before the session exists. Running the reader in this
//!   process would hide exactly the thing this gate is for.
//!
//! # What counts as a crash
//!
//! Three things, and all three are the same thing seen from different sides:
//!
//! - the child died by a signal (no exit code at all);
//! - the child exited **101**, which is the code a Rust program exits with
//!   when it dies of a panic;
//! - the child's output contains **`panicked at`**, which is the panic
//!   message — and which appears even when the panic was caught, because the
//!   panic hook runs before the unwind does.
//!
//! The third one is the reason the classifier looks at the output and not
//! only at the code: a panic caught by the engine's guard leaves the process
//! exiting 1, and a reader that is "handled" by an error message is still a
//! reader that panicked.
//!
//! # What is not a crash
//!
//! **A refusal.** A hostile file that the reader turns away with a message is
//! the reader working. Most of what the fuzzer produces is a refusal, and a
//! run in which every input was refused is a good run.
//!
//! # The seed is written by the tree
//!
//! The corpus is one valid design, written by `craftkit-cli sample --out` —
//! the tree's own program, using the tree's own writer. A corpus hand-written
//! here would be a second copy of the file format, and it would drift: the
//! fuzzer would then be breaking a format that nothing else in the tree
//! speaks.
//!
//! **And the seed is not written here on purpose.** The xtask is the tool
//! that *checks* the tree, and a checker that cannot run when the tree does
//! not compile is a checker that is silent exactly when it is needed. So the
//! xtask depends on `serde` and `serde_json` and on nothing of the tree's
//! own, and the design comes from the tree's program instead. §180.
//!
//! # The run is reproducible
//!
//! One fixed seed, one fixed list of mutations, one fixed order. Two runs of
//! `cargo xtask fuzz` try the same inputs in the same order, so a crash found
//! on a laptop is the same crash on the build machine, and a red CI run can
//! be reproduced by the person who has to fix it.
//!
//! # What it leaves behind
//!
//! `fuzz-out/`, which is **not** in the repository: the working inputs go in
//! `fuzz-out/work/`, and every crash is minimised into `fuzz-out/findings/`
//! with a note beside it saying what happened. The directory is in
//! `.gitignore` and in the one walk's list of ignored directories, so a
//! mutated file — which is a file of bytes nobody wrote — never reaches the
//! gates that look at the tree.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::step;

/// How many broken inputs a run tries, when the caller does not say.
pub const DEFAULT_ITERATIONS: usize = 500;

/// The seed every run starts from.
///
/// Fixed, so a run is reproducible: the same inputs, in the same order, on
/// every machine. A fuzzer whose inputs depend on the clock is a fuzzer
/// whose crashes cannot be reproduced.
pub const DEFAULT_SEED: u64 = 0x5eed_f0f2_1a3b_5c7d;

/// How long one input may take before the run calls it a hang.
///
/// Ten seconds for reading a file of a few kilobytes and evaluating it. The
/// reader is bounded — the file size, the JSON depth and the memory are all
/// capped — so ten thousand times the work is already a hang.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// The exit code a Rust program uses when it dies of a panic.
pub const PANIC_EXIT_CODE: i32 = 101;

/// The words the Rust panic message starts with.
///
/// This is the marker that survives a caught panic: the hook prints the
/// message before the unwind starts, so a panic the engine turned into an
/// error is still visible in the output.
pub const PANIC_MARKER: &str = "panicked at";

/// How many times the minimiser may ask the child about a smaller input.
pub const MINIMISE_BUDGET: usize = 40;

/// The environment variable that turns the tree's own test binary into the
/// fuzzer's fake child.
///
/// The same shape as the tree's other child probes: the test does nothing
/// unless the variable is there, and the parent puts it there for the child
/// and only for the child.
pub const FAKE_CHILD_ENV: &str = "CRAFTKIT_FUZZ_FAKE_CHILD";

/// The name of the test that acts as the fake child.
///
/// It is a name in the child's own argument list, because the Rust test
/// harness runs the test whose name it is given and no other.
pub const FAKE_CHILD_TEST: &str = "fuzz::tests::the_child_fixture_panics_only_when_it_is_the_child";

/// What one broken input turned out to be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Verdict {
    /// The program read it and went on.
    Accepted,
    /// The program turned it away with a message. The reader working.
    Refused,
    /// The program died. A bug.
    Crash,
    /// The program never finished. A bug.
    Hang,
}

impl Verdict {
    /// The word the reports use.
    pub fn name(self) -> &'static str {
        match self {
            Verdict::Accepted => "accepted",
            Verdict::Refused => "refused",
            Verdict::Crash => "crash",
            Verdict::Hang => "hang",
        }
    }

    /// Whether this verdict is a bug worth failing over.
    pub fn is_a_bug(self) -> bool {
        matches!(self, Verdict::Crash | Verdict::Hang)
    }
}

/// The verdict on one input.
///
/// A **pure function**, so the rules can be tested without a child, a
/// compiler or a clock: the same three facts in, the same verdict out. That
/// matters because this is the part of the fuzzer that decides whether a
/// bug is a bug.
pub fn classify(code: Option<i32>, stdout: &str, stderr: &str, timed_out: bool) -> Verdict {
    // A hang is a hang, whatever the process did on its way out: the parent
    // killed it, so there is no exit code to read, and calling that a crash
    // would report a hang as the wrong kind of bug.
    if timed_out {
        return Verdict::Hang;
    }
    // No exit code at all: the child died by a signal.
    if code.is_none() {
        return Verdict::Crash;
    }
    // The code a Rust program exits with when it dies of a panic.
    if code == Some(PANIC_EXIT_CODE) {
        return Verdict::Crash;
    }
    // The panic message itself. This is the one that catches a panic the
    // engine's guard turned into an error: the process exits cleanly, and
    // only the output says what happened.
    if stdout.contains(PANIC_MARKER) || stderr.contains(PANIC_MARKER) {
        return Verdict::Crash;
    }
    if code == Some(0) {
        return Verdict::Accepted;
    }
    Verdict::Refused
}

/// One valid design file to break.
#[derive(Clone, Debug)]
pub struct Seed {
    /// What the seed is, for the note beside a finding.
    pub name: String,
    /// The file, exactly as the tree's writer wrote it.
    pub bytes: Vec<u8>,
}

/// The name the seed file has, in the working directory.
///
/// The name is part of the rule, not decoration: the fake child in this
/// file's own tests reads it to tell the seed from the mutants, and the
/// corpus check needs to know which file is the one it trusts.
pub const SEED_NAME: &str = "seed.craftkit";

/// The name the seed file has when the seed is a **script**.
pub const SCRIPT_SEED_NAME: &str = "seed.json";

/// The name the seed file has when the seed is a **crash report**.
///
/// Its own name, and not the script's: the three runs share one working
/// directory, so two targets that used one file name would overwrite each
/// other's corpus. §201.
pub const REPORT_SEED_NAME: &str = "seed-report.json";

/// Which file the fuzzer breaks.
///
/// §182: the fuzzer had one shape — one file, one command — and the tree has
/// three doors that read a file. The design file was the first and the script
/// is the second. Each target carries the three things that make it a target:
/// the name its corpus is written under, the words its child is called with,
/// and how its corpus is written.
///
/// **The corpus of each is written by the tree's own program.** That is
/// §180's rule and it is why a target cannot be added without a writer: a
/// corpus written by hand would be a second copy of the format, and it would
/// drift — the fuzzer would be breaking a format nothing else in the tree
/// speaks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// A design file, read by `craftkit-cli eval`.
    Design,
    /// A script of commands, read by `craftkit-cli run`.
    Script,
    /// A crash report, read by `craftkit-cli crashes replay --file`.
    ///
    /// §201: the report is the format that **arrives from another machine** —
    /// the recipe asks for `report.json` from whoever had the crash, and that
    /// file has no folder around it and no id to look it up by. So it is the one
    /// format a hostile file arrives in, and the one the fuzzer was not
    /// watching. §195 gave it a door and §200 gave the tree a writer.
    Report,
}

impl Target {
    /// Every target, in the order the runs happen.
    pub const ALL: [Target; 3] = [Target::Design, Target::Script, Target::Report];

    /// The name the corpus is written under.
    pub fn seed_name(self) -> &'static str {
        match self {
            Target::Design => SEED_NAME,
            Target::Script => SCRIPT_SEED_NAME,
            Target::Report => REPORT_SEED_NAME,
        }
    }

    /// The name the broken inputs are written under.
    pub fn input_name(self) -> &'static str {
        match self {
            Target::Design => "input.craftkit",
            Target::Script => "input.json",
            Target::Report => "input-report.json",
        }
    }

    /// The extension a finding of this target carries.
    ///
    /// The target is in the name as well, so two targets' findings cannot
    /// collide in one folder — and the note beside a finding is always
    /// `.note.json`, so a script's finding (a `.json` file) and its note are
    /// two files and not one.
    pub fn finding_extension(self) -> &'static str {
        match self {
            Target::Design => "craftkit",
            Target::Script => "json",
            Target::Report => "json",
        }
    }

    /// The words the child is called with, before the file.
    pub fn child_args(self) -> Vec<String> {
        match self {
            Target::Design => vec!["eval".to_string()],
            Target::Script => vec!["run".to_string()],
            Target::Report => {
                vec!["crashes".to_string(), "replay".to_string(), "--file".to_string()]
            }
        }
    }

    /// The word the notes and the report use.
    pub fn name(self) -> &'static str {
        match self {
            Target::Design => "design",
            Target::Script => "script",
            Target::Report => "report",
        }
    }

    /// The corpus, written by the tree's own program.
    pub fn seed(self, cli: &Path, dir: &Path, options: &Options) -> Result<Vec<Seed>, String> {
        match self {
            Target::Design => seed_designs(cli, dir, options),
            Target::Script => seed_scripts(cli, dir, options),
            Target::Report => seed_reports(cli, dir, options),
        }
    }
}

/// A seed, from the command line's point of view.
fn parse_target(raw: &str) -> Result<Target, String> {
    match raw {
        "design" => Ok(Target::Design),
        "script" => Ok(Target::Script),
        "report" => Ok(Target::Report),
        other => {
            Err(format!("`--target` wants `design`, `script` or `report`, and `{other}` is none of them"))
        }
    }
}

/// Which targets a run covers.
///
/// **A target and a selection are different things.** A target is a file the
/// fuzzer breaks; a selection is how many of them the run breaks. The
/// selection is `all` by default, because a fuzzer that only watched one
/// door would be watching the door the tree already knew about. §182.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Selection {
    /// Every target.
    All,
    /// One named target.
    One(Target),
}

impl Selection {
    /// The targets this selection covers.
    pub fn targets(self) -> Vec<Target> {
        match self {
            Selection::All => Target::ALL.to_vec(),
            Selection::One(target) => vec![target],
        }
    }
}

/// The six ways a seed is broken.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutation {
    /// One bit turned over.
    Flip,
    /// The file cut short.
    Truncate,
    /// A piece of another design dropped into the middle.
    Splice,
    /// A piece of the file repeated.
    Duplicate,
    /// A piece of the file taken out.
    Delete,
    /// A number replaced by one the reader cannot hold.
    Hostile,
    /// The design's own shape broken, with the file still valid JSON.
    Structure,
}

/// Every mutation, in the order the list is written.
pub const MUTATIONS: &[Mutation] = &[
    Mutation::Flip,
    Mutation::Truncate,
    Mutation::Splice,
    Mutation::Duplicate,
    Mutation::Delete,
    Mutation::Hostile,
    Mutation::Structure,
];

impl Mutation {
    /// The word the notes use.
    pub fn name(self) -> &'static str {
        match self {
            Mutation::Flip => "flip",
            Mutation::Truncate => "truncate",
            Mutation::Splice => "splice",
            Mutation::Duplicate => "duplicate",
            Mutation::Delete => "delete",
            Mutation::Hostile => "hostile",
            Mutation::Structure => "structure",
        }
    }
}

/// One broken input, and where it came from.
#[derive(Clone, Debug)]
pub struct Mutated {
    pub mutation: Mutation,
    /// The seed it came from.
    pub seed: String,
    pub bytes: Vec<u8>,
}

/// A small, deterministic generator.
///
/// SplitMix64: a few lines, no dependency, and the same seed gives the same
/// numbers on every machine — which is the whole requirement. A fuzzer that
/// draws from the clock finds different bugs on different machines, and a
/// crash nobody can reproduce is a crash nobody fixes.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`, and `0` when `n` is zero.
    ///
    /// The zero is not a special case anybody asked for: it is what keeps a
    /// caller from dividing by zero, and it is checked by a test rather than
    /// assumed.
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % (n as u64)) as usize
    }
}

/// The corpus: one valid design, written by the tree's own program.
///
/// The tree's program is `craftkit-cli sample --out`, which writes the sample
/// design with the writer the desktop app uses. The fuzzer reads the bytes
/// back and breaks them.
///
/// **The seed is checked before it is broken.** The fuzzer hands the seed to
/// the same child it hands the mutants to, and refuses to go on unless the
/// child *accepts* it: a corpus the tree cannot read is a corpus that tests
/// nothing, and a fuzzer that tests nothing is green for the wrong reason.
pub fn seed_designs(cli: &Path, dir: &Path, options: &Options) -> Result<Vec<Seed>, String> {
    let path = dir.join(SEED_NAME);
    let args = vec!["sample".to_string(), "--out".to_string(), path.to_string_lossy().into_owned()];
    let outcome = crate::command_timed(cli, &args, dir, options.timeout, &options.child_env)?;
    if !outcome.timed_out && outcome.code != Some(0) {
        return Err(format!(
            "the tree did not write a seed design: `{} sample --out {}` said\n{}",
            cli.display(),
            path.display(),
            outcome.all()
        ));
    }
    let bytes =
        crate::tree::read_bytes(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(vec![Seed { name: "the sample design".to_string(), bytes }])
}

/// The corpus for the script door: one valid script, written by the tree's
/// own program.
///
/// The tree's program is `craftkit-cli script --out`, which writes a script
/// with the tree's own writer — the same writer a person would use. §182 gave
/// the tree that writer: before it, the only producer of a script printed a
/// report's replay to stdout, so this corpus could not be written at all.
///
/// **The seed is checked before it is broken**, like the design's: the fuzzer
/// hands the seed to the same child it hands the mutants to, and refuses to
/// go on unless the child *accepts* it.
fn seed_scripts(cli: &Path, dir: &Path, options: &Options) -> Result<Vec<Seed>, String> {
    let path = dir.join(SCRIPT_SEED_NAME);
    let args = vec!["script".to_string(), "--out".to_string(), path.to_string_lossy().into_owned()];
    let outcome = crate::command_timed(cli, &args, dir, options.timeout, &options.child_env)?;
    if !outcome.timed_out && outcome.code != Some(0) {
        return Err(format!(
            "the tree did not write a seed script: `{} script --out {}` said\n{}",
            cli.display(),
            path.display(),
            outcome.all()
        ));
    }
    let bytes =
        crate::tree::read_bytes(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(vec![Seed { name: "the sample script".to_string(), bytes }])
}

/// The corpus for the report door: one valid report, written by the tree's
/// own program.
///
/// The tree's program is `craftkit-cli crashes write --out` — the door §200
/// gave the tree, because before it the only producer of a report anywhere in
/// the tree was the desktop app's panic hook, and a report needs a crash. So
/// the tree could not write a corpus for the format at all, and the format is
/// the one that **arrives from another machine**. §180's rule: the corpus is
/// written by the tree's own program, so the format the fuzzer breaks is the
/// format the program speaks.
///
/// **The seed is checked before it is broken**, like the design's and the
/// script's: the fuzzer hands the seed to the same child it hands the mutants
/// to, and refuses to go on unless the child *accepts* it.
fn seed_reports(cli: &Path, dir: &Path, options: &Options) -> Result<Vec<Seed>, String> {
    let path = dir.join(REPORT_SEED_NAME);
    let args = vec![
        "crashes".to_string(),
        "write".to_string(),
        "--out".to_string(),
        path.to_string_lossy().into_owned(),
    ];
    let outcome = crate::command_timed(cli, &args, dir, options.timeout, &options.child_env)?;
    if !outcome.timed_out && outcome.code != Some(0) {
        return Err(format!(
            "the tree did not write a seed report: `{} crashes write --out {}` said\n{}",
            cli.display(),
            path.display(),
            outcome.all()
        ));
    }
    let bytes =
        crate::tree::read_bytes(&path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(vec![Seed { name: "the sample report".to_string(), bytes }])
}

/// Break one seed, in one way.
fn mutate(rng: &mut Rng, seeds: &[Seed], target: Target) -> Mutated {
    let seed = &seeds[rng.below(seeds.len())];
    let mutation = MUTATIONS[rng.below(MUTATIONS.len())];
    let bytes = match mutation {
        Mutation::Flip => flip(rng, &seed.bytes),
        Mutation::Truncate => truncate(rng, &seed.bytes),
        Mutation::Splice => splice(rng, &seed.bytes, seeds),
        Mutation::Duplicate => duplicate(rng, &seed.bytes),
        Mutation::Delete => delete(rng, &seed.bytes),
        Mutation::Hostile => hostile(rng, &seed.bytes),
        Mutation::Structure => structure(rng, &seed.bytes, target),
    };
    Mutated { mutation, seed: seed.name.clone(), bytes }
}

/// Turn one bit over.
fn flip(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if out.is_empty() {
        return out;
    }
    let at = rng.below(out.len());
    out[at] ^= 1 << rng.below(8);
    out
}

/// Cut the file short.
///
/// **Never to nothing.** An empty file is not a mutation, it is the absence
/// of one — and a mutation that empties the file is the one case where the
/// fuzzer would be testing the reader's "not a craftkit file" path over and
/// over instead of testing the reader. The cut therefore keeps at least one
/// byte.
fn truncate(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    if bytes.len() < 2 {
        return bytes.to_vec();
    }
    bytes[..1 + rng.below(bytes.len() - 1)].to_vec()
}

/// Drop a piece of another design into the middle.
fn splice(rng: &mut Rng, bytes: &[u8], seeds: &[Seed]) -> Vec<u8> {
    let other = &seeds[rng.below(seeds.len())].bytes;
    if bytes.is_empty() || other.is_empty() {
        return bytes.to_vec();
    }
    let from = rng.below(other.len());
    let to = (from + 1 + rng.below(other.len() - from)).min(other.len());
    let at = rng.below(bytes.len());
    let mut out = Vec::with_capacity(bytes.len() + (to - from));
    out.extend_from_slice(&bytes[..at]);
    out.extend_from_slice(&other[from..to]);
    out.extend_from_slice(&bytes[at..]);
    out
}

/// Repeat a piece of the file.
fn duplicate(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let from = rng.below(bytes.len());
    let to = (from + 1 + rng.below(bytes.len() - from)).min(bytes.len());
    let at = rng.below(bytes.len());
    let mut out = Vec::with_capacity(bytes.len() + (to - from));
    out.extend_from_slice(&bytes[..at]);
    out.extend_from_slice(&bytes[from..to]);
    out.extend_from_slice(&bytes[at..]);
    out
}

/// Take a piece of the file out.
fn delete(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let from = rng.below(bytes.len());
    let to = (from + 1 + rng.below(bytes.len() - from)).min(bytes.len());
    let mut out = bytes.to_vec();
    out.drain(from..to);
    out
}

/// Numbers the reader cannot hold.
///
/// Each one is a value the *file* can carry and the *model* cannot: a
/// number past the reader's magnitude limit, a negative one past it, a
/// number that is not finite, and one past the largest integer a machine
/// counts with. A reader that takes one of these and puts it into the model
/// has a bug; a reader that refuses it has done its job.
fn hostile(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    const HOSTILE: &[&str] = &[
        "999999999999999999999999999999999999",
        "-999999999999999999999999999999999999",
        "1e999999",
        "18446744073709551616",
    ];
    let text = String::from_utf8_lossy(bytes).into_owned();
    let value = HOSTILE[rng.below(HOSTILE.len())];
    let digits = text.as_bytes().iter().position(|b| b.is_ascii_digit());
    match digits {
        Some(start) => {
            let mut end = start;
            while end < text.len() && text.as_bytes()[end].is_ascii_digit() {
                end += 1;
            }
            let mut out = text.clone();
            out.replace_range(start..end, value);
            out.into_bytes()
        }
        None => {
            let mut out = text;
            out.push_str(value);
            out.into_bytes()
        }
    }
}

/// The file's own shape, broken — with the file still valid JSON.
///
/// The other six mutations break the *file*: the JSON stops parsing, and the
/// reader turns the file away at the first line. That is the reader working,
/// but it is only the outermost part of the reader. This one breaks the
/// *shape*: the file parses, and everything after that is the reader's own
/// rules.
///
/// Each breakage is one of the rules the reader states, pushed past it — and
/// **the rules are the target's own**. A design's shape is its document; a
/// script's shape is its list of commands. §182.
fn structure(rng: &mut Rng, bytes: &[u8], target: Target) -> Vec<u8> {
    match target {
        Target::Design => break_the_document(rng, bytes),
        Target::Script => break_the_script(rng, bytes),
        Target::Report => break_the_report(rng, bytes),
    }
}

/// The design's own shape, broken — with the file still valid JSON.
///
/// This one breaks the *document*: the file parses, the tag is right, and
/// everything after that is the reader's own rules — the node count, the
/// frame rate, the range, the ids, the property tree.
///
/// Each breakage is one of the rules the model states, pushed past it.
fn break_the_document(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(bytes) else { return bytes.to_vec() };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else { return bytes.to_vec() };
    // A file with no document in it is not a design, and there is nothing of
    // the design's shape to break: the mutation leaves it alone rather than
    // inventing a document for it.
    let Some(document) = value.get("document").cloned() else { return bytes.to_vec() };
    let mut document = document;
    match rng.below(8) {
        // More nodes than a document may hold.
        0 => {
            let node = document["nodes"][0].clone();
            if node.is_null() {
                return bytes.to_vec();
            }
            let Some(nodes) = document["nodes"].as_array_mut() else { return bytes.to_vec() };
            // Just over the model's limit of ten thousand: the reader has to
            // refuse it, and a file just over the limit says that as clearly
            // as one far over it.
            for _ in 0..12_000 {
                nodes.push(node.clone());
            }
        }
        // A frame rate of zero: the ticks per frame divide by it.
        1 => document["meta"]["fps"] = serde_json::json!(0),
        // A frame rate faster than the model allows.
        2 => document["meta"]["fps"] = serde_json::json!(1_000_000),
        // A range that is empty.
        3 => {
            document["meta"]["start"] = serde_json::json!(10);
            document["meta"]["end"] = serde_json::json!(10);
        }
        // A range outside what a time can be.
        4 => {
            document["meta"]["start"] = serde_json::json!(-1);
            document["meta"]["end"] = serde_json::json!(9_007_199_254_740_991i64);
        }
        // Two nodes with the same id.
        5 => {
            let first = document["nodes"][0]["id"].clone();
            if first.is_null() {
                return bytes.to_vec();
            }
            if let Some(nodes) = document["nodes"].as_array_mut()
                && nodes.len() > 1
            {
                nodes[1]["id"] = first;
            }
        }
        // A note longer than the model allows.
        6 => document["meta"]["comment"] = serde_json::json!("x".repeat(200_000)),
        // The document itself, replaced by something that is not one.
        _ => document = serde_json::json!({"nodes": "not an array"}),
    }
    if let Some(map) = value.as_object_mut() {
        map.insert("document".to_string(), document);
    }
    serde_json::to_string(&value).unwrap_or_else(|_| text.to_string()).into_bytes()
}

/// The script's own shape, broken — with the file still valid JSON.
///
/// A script is a **list of commands**, so the shape that matters is the
/// list, and the rules that matter are the ones the reader states about it:
/// a step is an object, a step names a command as a string, a step's
/// parameters are an object, the command is one the registry has, and the
/// parameters are ones the command's own readers accept.
///
/// The last one is the interesting one: a script carries **commands**, so a
/// broken script is not only a broken file — it is a broken *instruction*, and
/// the instruction is aimed at the engine. §177's property is the check that
/// matters here: the document must still be valid afterwards. §182.
fn break_the_script(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(bytes) else { return bytes.to_vec() };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else { return bytes.to_vec() };
    let Some(list) = value.get("commands").and_then(|c| c.as_array()) else { return bytes.to_vec() };
    let mut commands = list.clone();
    if commands.is_empty() {
        return bytes.to_vec();
    }
    match rng.below(6) {
        // A step that is not an object at all.
        0 => commands[0] = serde_json::json!("node.add"),
        // A step whose command is not a string.
        1 => set_in_step(&mut commands[0], "command", serde_json::json!(7)),
        // A step whose parameters are not an object.
        2 => set_in_step(&mut commands[0], "params", serde_json::json!([1, 2, 3])),
        // A command the registry does not have.
        3 => set_in_step(&mut commands[0], "command", serde_json::json!("node.explode")),
        // A parameter the command's own reader cannot hold: a magnitude past
        // the tree's own limit, which the reader has to refuse.
        4 => set_in_step(&mut commands[0], "params", serde_json::json!({ "amount": 1e300 })),
        // A base that is not a document.
        _ => {
            if let Some(map) = value.as_object_mut() {
                map.insert("base".to_string(), serde_json::json!({ "nodes": "not an array" }));
            }
        }
    }
    if let Some(map) = value.as_object_mut() {
        map.insert("commands".to_string(), serde_json::json!(commands));
    }
    serde_json::to_string(&value).unwrap_or_else(|_| text.to_string()).into_bytes()
}

/// The report's own shape, broken — with the file still valid JSON.
///
/// A report is a **design**, a **base**, and the **commands** between them, so
/// the shape that matters is the report's own: the two documents, the list of
/// steps, and the numbers around them.
///
/// Each breakage is one of the rules the reader states, pushed past it. And
/// the reader checks **both** of the report's documents, because the design is
/// the report's **target**: what the replay has to end up with. §183.
fn break_the_report(rng: &mut Rng, bytes: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(bytes) else { return bytes.to_vec() };
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) else { return bytes.to_vec() };
    match rng.below(9) {
        // The design replaced by something that is not a document.
        0 => set_in_report(&mut value, "design", serde_json::json!({ "nodes": "not an array" })),
        // The base replaced by something that is not a document.
        1 => set_in_report(&mut value, "base", serde_json::json!({ "nodes": "not an array" })),
        // A design the *model* refuses: a frame rate of zero.
        2 => zero_the_frame_rate(&mut value, "design"),
        // The replay not a list at all.
        3 => set_in_report(&mut value, "replay", serde_json::json!("not a list")),
        // A step that is not an object.
        4 => break_first_step(&mut value, serde_json::json!("node.add")),
        // A step whose command is not a string.
        5 => break_first_step(&mut value, serde_json::json!({ "command": 7 })),
        // A step whose parameters are not an object.
        6 => break_first_step(&mut value, serde_json::json!({ "command": "node.add", "params": [1, 2, 3] })),
        // The count of dropped commands is not a number.
        7 => set_in_report(&mut value, "dropped", serde_json::json!("many")),
        // The playhead is not a number.
        _ => set_in_report(&mut value, "at", serde_json::json!("now")),
    }
    serde_json::to_string(&value).unwrap_or_else(|_| text.to_string()).into_bytes()
}

/// Set one key of a report, if the report is an object.
///
/// A report that is not an object has no keys, and the value's own index
/// would panic on one — so the shape is asked first. The same rule as
/// [`set_in_step`], one level up.
fn set_in_report(report: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    let Some(map) = report.as_object_mut() else { return };
    map.insert(key.to_string(), value);
}

/// Replace the first step of a report's replay with something that is not a
/// step.
fn break_first_step(report: &mut serde_json::Value, step: serde_json::Value) {
    let Some(steps) = report.get_mut("replay").and_then(|r| r.as_array_mut()) else { return };
    if steps.is_empty() {
        return;
    }
    steps[0] = step;
}

/// A frame rate of zero, in a document inside a report.
///
/// The report's **design** is the report's *target*: what the replay has to
/// end up with. So a design the model refuses is a report the reader
/// refuses, and the reader says which document it was. §183.
fn zero_the_frame_rate(report: &mut serde_json::Value, key: &str) {
    let Some(document) = report.get_mut(key).and_then(|d| d.as_object_mut()) else { return };
    let Some(meta) = document.get_mut("meta").and_then(|m| m.as_object_mut()) else { return };
    meta.insert("fps".to_string(), serde_json::json!(0));
}

/// Set one key of a step, if the step is an object.
///
/// A step that is not an object has no keys, and the value's own index would
/// panic on one — so the shape is asked first.
fn set_in_step(step: &mut serde_json::Value, key: &str, value: serde_json::Value) {
    let Some(map) = step.as_object_mut() else { return };
    map.insert(key.to_string(), value);
}

/// Make a broken input smaller, while it still breaks the same way.
///
/// The result is what a person has to look at, so it is worth the child's
/// time: a crash in a 2 000-byte file is a puzzle, and a crash in a
/// 40-byte file is a sentence.
///
/// Two rounds. First the tail: cut the file in half, and keep the half if it
/// still breaks. Then the middle: take pieces out, biggest first, and keep
/// the file without them if it still breaks.
///
/// The budget is in *questions asked*, not in bytes: the child is the
/// expensive part, and a minimiser that never stops is worse than no
/// minimiser.
pub fn minimise(bytes: &[u8], budget: usize, mut still_breaks: impl FnMut(&[u8]) -> bool) -> Vec<u8> {
    let mut best = bytes.to_vec();
    let mut spent = 0usize;

    // The tail.
    let mut cut = best.len() / 2;
    while cut > 0 && spent < budget {
        spent += 1;
        if still_breaks(&best[..cut]) {
            best.truncate(cut);
            cut = best.len() / 2;
        } else {
            cut /= 2;
        }
    }

    // The middle.
    let mut size = best.len().max(2) / 2;
    while size > 0 && spent < budget {
        let mut at = 0usize;
        let mut changed = false;
        while at < best.len() && spent < budget {
            let end = (at + size).min(best.len());
            let mut candidate = best.clone();
            candidate.drain(at..end);
            spent += 1;
            if still_breaks(&candidate) {
                best = candidate;
                changed = true;
            } else {
                at = end.max(at + 1);
            }
        }
        if !changed {
            size /= 2;
        }
    }

    best
}

/// What a run did.
#[derive(Debug, Default)]
pub struct Report {
    /// How many broken inputs were tried.
    pub tried: usize,
    pub accepted: usize,
    pub refused: usize,
    pub crashes: usize,
    pub hangs: usize,
    /// Where the findings were written.
    pub findings: Vec<PathBuf>,
}

/// How the run is set up.
#[derive(Clone, Debug)]
pub struct Options {
    /// The program to fuzz. `None` means: find the tree's own CLI.
    pub cli: Option<PathBuf>,
    /// How many broken inputs to try. Must be more than zero.
    pub iterations: usize,
    /// The seed. Fixed by default, so the run is reproducible.
    pub seed: u64,
    /// How long one input may take.
    pub timeout: Duration,
    /// Where the working inputs and the findings go.
    pub out: PathBuf,
    /// Which targets the run covers.
    ///
    /// §182: the tree has more than one door that reads a file, and each
    /// needs its own corpus and its own child.
    pub selection: Selection,
    /// The words the child is called with, before the file.
    ///
    /// `None` means: **the target's own words** — `eval` for a design, `run`
    /// for a script. The self-test sets it, because its fake child is called
    /// by the test harness's own filter rather than by a subcommand.
    pub child_args: Option<Vec<String>>,
    /// The environment the child is given.
    pub child_env: Vec<(String, String)>,
    /// How many times the minimiser may ask about a smaller input.
    pub minimise_budget: usize,
    /// A design the caller already has, when it has one.
    ///
    /// The fuzzer's own seed comes from the tree's *program* — `craftkit-cli
    /// sample --out` — because a corpus written by hand would be a second
    /// copy of the file format. A caller that already has a design (the
    /// tests in this file do) hands it over instead, and the fuzzer writes it
    /// out and breaks that.
    pub seed_bytes: Option<Vec<u8>>,
}

impl Options {
    /// The options a plain run uses.
    pub fn defaults(root: &Path) -> Self {
        Options {
            cli: None,
            iterations: DEFAULT_ITERATIONS,
            seed: DEFAULT_SEED,
            timeout: DEFAULT_TIMEOUT,
            out: root.join("fuzz-out"),
            selection: Selection::All,
            child_args: None,
            child_env: Vec::new(),
            minimise_budget: MINIMISE_BUDGET,
            seed_bytes: None,
        }
    }
}

/// Where the tree's CLI may be, in the order it is looked for.
///
/// The `ci` profile is first because that is where `cargo xtask ci` builds
/// it, and the fuzz run is a phase of `ci`: by the time it runs, the binary
/// is there and nothing has to be built.
pub fn cli_candidates(root: &Path) -> Vec<PathBuf> {
    let name = if cfg!(windows) { "craftkit-cli.exe" } else { "craftkit-cli" };
    ["ci", "release", "debug"].iter().map(|profile| root.join("target").join(profile).join(name)).collect()
}

/// The tree's CLI, built and then found.
///
/// A fuzz run against a program that is not there is not a run, so this
/// builds the CLI rather than skipping. "Skipped, with a reason" is for a
/// gate that *cannot* run — a missing tool — and here the tool is one
/// command away.
pub fn find_cli(root: &Path) -> Result<PathBuf, String> {
    // **The build is not optional.** A fuzz run against a *stale* binary is a
    // run against a tree that is not the one in front of you: the binary in
    // `target/ci` may be from before the change being tested, and the fuzzer
    // would report on a program the tree no longer has. So the CLI is built
    // first — a no-op when it is already up to date — and only then looked
    // for.
    crate::cargo(&["build", "-p", "craftkit-cli", "--profile", "ci"], root)?;
    for path in cli_candidates(root) {
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(format!(
        "cannot find craftkit-cli to fuzz; looked in {}",
        cli_candidates(root).iter().map(|p| p.display().to_string()).collect::<Vec<String>>().join(", ")
    ))
}

/// Hand one file to the child and wait for it.
///
/// The child is called with the **target's** words — `eval` for a design,
/// `run` for a script — unless the caller has overridden them, which only
/// this file's own self-test does. §182.
fn run_child(
    cli: &Path,
    options: &Options,
    target: Target,
    file: &Path,
    dir: &Path,
) -> Result<crate::TimedOutput, String> {
    let mut args = match &options.child_args {
        Some(args) => args.clone(),
        None => target.child_args(),
    };
    args.push(file.to_string_lossy().into_owned());
    crate::command_timed(cli, &args, dir, options.timeout, &options.child_env)
}

/// The run, against one target.
pub fn fuzz(root: &Path, options: &Options, target: Target) -> Result<Report, String> {
    if options.iterations == 0 {
        return Err("a run of no inputs is not a run; give it at least one".to_string());
    }
    let cli = match &options.cli {
        Some(path) => path.clone(),
        None => find_cli(root)?,
    };

    let work = options.out.join("work");
    let findings = options.out.join("findings");
    std::fs::create_dir_all(&work).map_err(|e| format!("could not create {}: {e}", work.display()))?;
    std::fs::create_dir_all(&findings)
        .map_err(|e| format!("could not create {}: {e}", findings.display()))?;

    let seed_path = work.join(target.seed_name());
    let seeds = match &options.seed_bytes {
        Some(bytes) => {
            std::fs::write(&seed_path, bytes)
                .map_err(|e| format!("could not write {}: {e}", seed_path.display()))?;
            vec![Seed { name: "the seed the caller gave".to_string(), bytes: bytes.clone() }]
        }
        None => target.seed(&cli, &work, options)?,
    };
    if seeds.is_empty() {
        return Err("the corpus is empty, so there is nothing to break".to_string());
    }
    // The seed must be a file the tree reads. If it is not, every mutant of
    // it is refused at the first line, the run is green, and the green says
    // nothing: a fuzzer whose corpus is rubbish passes for ever.
    let outcome = run_child(&cli, options, target, &seed_path, root)?;
    let verdict = classify(outcome.code, &outcome.stdout, &outcome.stderr, outcome.timed_out);
    if verdict != Verdict::Accepted {
        return Err(format!(
            "the seed {} {} was not accepted by the tree ({}) — a corpus the reader turns \
             away tests nothing, so the run stops here",
            target.name(),
            seed_path.display(),
            verdict.name()
        ));
    }

    let mut rng = Rng::new(options.seed);
    let mut report = Report::default();

    for index in 0..options.iterations {
        let mutated = mutate(&mut rng, &seeds, target);
        // One working file, overwritten every time. The inputs are scratch:
        // a crash is minimised into `findings/`, and a run that found nothing
        // has nothing worth keeping. Five hundred files of several megabytes
        // each would be ninety megabytes of nothing on the build machine.
        let file = work.join(target.input_name());
        std::fs::write(&file, &mutated.bytes)
            .map_err(|e| format!("could not write {}: {e}", file.display()))?;

        let outcome = run_child(&cli, options, target, &file, root)?;
        let verdict = classify(outcome.code, &outcome.stdout, &outcome.stderr, outcome.timed_out);
        report.tried += 1;
        match verdict {
            Verdict::Accepted => report.accepted += 1,
            Verdict::Refused => report.refused += 1,
            Verdict::Crash | Verdict::Hang => {
                let smaller = minimise(&mutated.bytes, options.minimise_budget, |candidate| {
                    let path = work.join(target.input_name());
                    if std::fs::write(&path, candidate).is_err() {
                        return false;
                    }
                    match run_child(&cli, options, target, &path, root) {
                        Ok(outcome) => {
                            classify(outcome.code, &outcome.stdout, &outcome.stderr, outcome.timed_out)
                                == verdict
                        }
                        Err(_) => false,
                    }
                });
                let name = format!("{}-{}-{:05}", target.name(), verdict.name(), index);
                let path = findings.join(format!("{name}.{}", target.finding_extension()));
                std::fs::write(&path, &smaller)
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                let note = path.with_extension("note.json");
                std::fs::write(&note, note_text(&mutated, verdict, &smaller, &outcome))
                    .map_err(|e| format!("could not write {}: {e}", note.display()))?;
                report.findings.push(path);
                match verdict {
                    Verdict::Crash => report.crashes += 1,
                    _ => report.hangs += 1,
                }
            }
        }
    }

    Ok(report)
}

/// The note beside a finding.
///
/// It says what was broken, what the child did, and what the file is now —
/// so the person who opens the finding does not have to guess which of the
/// five hundred inputs it was.
fn note_text(mutated: &Mutated, verdict: Verdict, smaller: &[u8], outcome: &crate::TimedOutput) -> String {
    let note = serde_json::json!({
        "verdict": verdict.name(),
        "mutation": mutated.mutation.name(),
        "seed": mutated.seed,
        "bytes_before": mutated.bytes.len(),
        "bytes_after": smaller.len(),
        "exit": outcome.code,
        "timed_out": outcome.timed_out,
        "stderr_tail": tail(&outcome.stderr),
        "stdout_tail": tail(&outcome.stdout),
    });
    serde_json::to_string_pretty(&note).unwrap_or_else(|_| "{}".to_string())
}

/// The last few lines of a stream, so the note is a note and not a transcript.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(20);
    lines[start..].join("\n")
}

/// The command: `cargo xtask fuzz`.
pub fn command(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    step("fuzz");
    // The usage is asked for, not stumbled on: a command that answers
    // `--help` by printing an error is a command nobody can ask.
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print!("{}", usage());
        return Ok(crate::Outcome::Ran);
    }
    let options = options_from(root, args)?;
    // **Every target, unless one is named.** §182: the tree has more than one
    // door that reads a file, and a fuzzer that only watched one of them
    // would be watching the door the tree already knew about.
    let targets = options.selection.targets();
    let mut crashes = 0usize;
    let mut hangs = 0usize;
    let mut findings: Vec<PathBuf> = Vec::new();
    for target in targets {
        let report = fuzz(root, &options, target)?;
        println!(
            "   {}: {} input(s): {} accepted, {} refused, {} crash(es), {} hang(s)",
            target.name(),
            report.tried,
            report.accepted,
            report.refused,
            report.crashes,
            report.hangs
        );
        crashes += report.crashes;
        hangs += report.hangs;
        findings.extend(report.findings);
    }
    if !findings.is_empty() {
        for finding in &findings {
            println!("   finding: {}", finding.display());
        }
        return Err(format!(
            "the fuzzer found {crashes} crash(es) and {hangs} hang(s); the findings are in {}",
            findings[0].parent().map(|p| p.display().to_string()).unwrap_or_default()
        ));
    }
    Ok(crate::Outcome::Ran)
}

/// The usage text.
pub fn usage() -> String {
    [
        "usage: cargo xtask fuzz [options]",
        "",
        "  --cli PATH      the program to fuzz (default: the tree's craftkit-cli)",
        "  --iters N       how many broken inputs to try (default: 500)",
        "  --seed N        the seed; the same seed tries the same inputs (default: 0x5eedf0f21a3b5c7d)",
        "  --timeout SECS  how long one input may take (default: 10)",
        "  --out DIR       where the inputs and the findings go (default: fuzz-out)",
        "  --target WHICH  which file to break: design, script, report, or all (default: all)",
        "",
    ]
    .join("\n")
}

/// Read the options out of the command line.
fn options_from(root: &Path, args: &[String]) -> Result<Options, String> {
    let mut options = Options::defaults(root);
    let mut index = 0usize;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--cli" => {
                options.cli = Some(PathBuf::from(value_of(args, &mut index, "--cli")?));
            }
            "--iters" => {
                let raw = value_of(args, &mut index, "--iters")?;
                options.iterations = raw
                    .parse::<usize>()
                    .map_err(|_| format!("`--iters` wants a number, and `{raw}` is not one"))?;
            }
            "--seed" => {
                let raw = value_of(args, &mut index, "--seed")?;
                options.seed = parse_seed(&raw)?;
            }
            "--timeout" => {
                let raw = value_of(args, &mut index, "--timeout")?;
                let seconds = raw
                    .parse::<u64>()
                    .map_err(|_| format!("`--timeout` wants a number of seconds, and `{raw}` is not one"))?;
                options.timeout = Duration::from_secs(seconds);
            }
            "--out" => {
                options.out = PathBuf::from(value_of(args, &mut index, "--out")?);
            }
            "--target" => {
                let raw = value_of(args, &mut index, "--target")?;
                options.selection = match raw.as_str() {
                    "all" => Selection::All,
                    other => Selection::One(parse_target(other)?),
                };
            }
            other => return Err(format!("`cargo xtask fuzz` does not take `{other}`\n\n{}", usage())),
        }
        index += 1;
    }
    Ok(options)
}

/// The value after a flag.
fn value_of(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index).cloned().ok_or_else(|| format!("`{flag}` needs a value"))
}

/// A seed, in decimal or in hexadecimal.
fn parse_seed(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    let parsed = match trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => trimmed.parse::<u64>(),
    };
    parsed.map_err(|_| format!("`--seed` wants a number, and `{raw}` is not one"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------- the verdict --

    #[test]
    fn a_child_that_died_by_a_signal_is_a_crash() {
        // No exit code at all: the child was killed.
        assert_eq!(classify(None, "", "", false), Verdict::Crash);
    }

    #[test]
    fn a_panic_exit_code_is_a_crash() {
        assert_eq!(classify(Some(PANIC_EXIT_CODE), "", "", false), Verdict::Crash);
    }

    #[test]
    fn a_panic_message_is_a_crash_even_when_the_process_survived() {
        // The engine runs every command under `catch_unwind`, so a panic
        // inside a command leaves the process exiting 1. The message is the
        // only thing that says what happened — and it is in the output,
        // because the panic hook runs before the unwind does.
        assert_eq!(classify(Some(1), "", "thread 'main' panicked at src/lib.rs:1:1", false), Verdict::Crash);
        // And the test harness prints it to stdout, not stderr.
        assert_eq!(classify(Some(1), "thread 'boom' panicked at lt.rs:2:13", "", false), Verdict::Crash);
    }

    #[test]
    fn a_refusal_is_not_a_crash() {
        // The reader turning a hostile file away is the reader working.
        assert_eq!(classify(Some(1), "", "this is not a craftkit file", false), Verdict::Refused);
        assert_eq!(classify(Some(2), "", "not a subcommand", false), Verdict::Refused);
    }

    #[test]
    fn a_file_the_program_read_is_accepted() {
        assert_eq!(classify(Some(0), "", "", false), Verdict::Accepted);
    }

    #[test]
    fn a_hang_is_a_hang_and_not_a_crash() {
        // The parent killed the child, so there is no exit code to read.
        // Calling that a crash would report a hang as the wrong bug.
        assert_eq!(classify(None, "", "", true), Verdict::Hang);
        assert_eq!(classify(Some(101), "", "", true), Verdict::Hang);
    }

    #[test]
    fn only_a_crash_and_a_hang_are_bugs() {
        assert!(Verdict::Crash.is_a_bug());
        assert!(Verdict::Hang.is_a_bug());
        assert!(!Verdict::Accepted.is_a_bug());
        assert!(!Verdict::Refused.is_a_bug());
    }

    // --------------------------------------------------------- the mutations --

    /// A design to break, for the tests that do not need the tree's own.
    ///
    /// The fuzzer's real corpus is written by `craftkit-cli sample --out` —
    /// see [`seed_designs`]. This one is here because the tests that check
    /// the *mutations* must run without a built binary: the tree's own
    /// file-format tests write a design by hand in exactly this way, and a
    /// test that needs a binary to have been built is a test that does not
    /// run where it is run.
    fn a_design() -> Seed {
        Seed {
            name: "a hand-written design".to_string(),
            bytes: br#"{
  "format": "craftkit/1",
  "meta": { "written_by": "a test", "written_at": 0, "pending": [] },
  "document": {
    "meta": { "name": "a test design", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" },
    "nodes": [
      { "id": 1, "name": "In", "kind": "source",
        "props": { "id": "props", "label": "Properties", "nodes": [] },
        "muted": false }
    ]
  }
}
"#
            .to_vec(),
        }
    }

    /// A report to break, for the tests that do not need the tree's own.
    ///
    /// The same shape the tree's own writer writes — `craftkit-cli crashes
    /// write --out` — and the CLI's own test suite is where the proof that the
    /// tree reads it lives, because the proof needs the binary. §200. This one
    /// is here so that the *mutations* can be checked without one. §201.
    fn a_report() -> Seed {
        Seed {
            name: "a hand-written report".to_string(),
            bytes: br#"{
  "id": "20250101-000000-0000",
  "unix": 0,
  "command": "node.add",
  "message": "a hand-written report",
  "design": {
    "meta": { "name": "a test report", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" },
    "nodes": []
  },
  "base": {
    "meta": { "name": "a test report", "fps": 24, "start": 0, "end": 1270080000000, "comment": "" },
    "nodes": []
  },
  "replay": [
    { "command": "node.add", "params": { "name": "In", "kind": "source", "amount": 1 } }
  ],
  "dropped": 0,
  "at": 0
}
"#
            .to_vec(),
        }
    }

    /// A script to break, for the tests that do not need the tree's own.
    ///
    /// The same shape the tree's own writer writes — `craftkit-cli script
    /// --out` — and the CLI's own test suite is where the proof that the
    /// tree reads it lives, because the proof needs the binary. This one is
    /// here so that the *mutations* can be checked without one. §182.
    fn a_script() -> Seed {
        Seed {
            name: "a hand-written script".to_string(),
            bytes: br#"{
  "name": "a hand-written script",
  "commands": [
    { "command": "node.add", "params": { "name": "In", "kind": "source", "amount": 1 } },
    { "command": "node.add", "params": { "name": "Out", "kind": "sink", "amount": 1 } }
  ]
}
"#
            .to_vec(),
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_mutations() {
        let seeds = vec![a_design()];
        let first: Vec<Mutated> = {
            let mut rng = Rng::new(DEFAULT_SEED);
            (0..20).map(|_| mutate(&mut rng, &seeds, Target::Design)).collect()
        };
        let second: Vec<Mutated> = {
            let mut rng = Rng::new(DEFAULT_SEED);
            (0..20).map(|_| mutate(&mut rng, &seeds, Target::Design)).collect()
        };
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.mutation, b.mutation, "the same seed chose two different mutations");
            assert_eq!(a.bytes, b.bytes, "the same seed produced two different inputs");
        }
    }

    /// The seed decides the run.
    ///
    /// Two halves, and the second is the one that catches a fuzzer that
    /// ignores its seed: a run that is the same every time is reproducible,
    /// but a run that is the same *whatever* the seed is not a search —
    /// it is the same twenty inputs, over and over.
    #[test]
    fn the_seed_decides_the_run() {
        let seeds = vec![a_design()];
        let mut first = Rng::new(DEFAULT_SEED);
        let mut second = Rng::new(DEFAULT_SEED);
        let mut other = Rng::new(DEFAULT_SEED + 1);
        let one: Vec<Vec<u8>> = (0..20).map(|_| mutate(&mut first, &seeds, Target::Design).bytes).collect();
        let two: Vec<Vec<u8>> = (0..20).map(|_| mutate(&mut second, &seeds, Target::Design).bytes).collect();
        let three: Vec<Vec<u8>> = (0..20).map(|_| mutate(&mut other, &seeds, Target::Design).bytes).collect();
        assert_eq!(one, two, "the same seed produced two different runs");
        assert_ne!(one, three, "two different seeds produced the same run");
    }

    #[test]
    fn every_mutation_changes_the_file() {
        let seeds = vec![a_design()];
        let mut rng = Rng::new(DEFAULT_SEED);
        let mut seen: Vec<Mutation> = Vec::new();
        for _ in 0..400 {
            let mutated = mutate(&mut rng, &seeds, Target::Design);
            let original = seeds.iter().find(|s| s.name == mutated.seed).expect("the seed is known");
            assert_ne!(
                mutated.bytes,
                original.bytes,
                "the `{}` mutation left the file alone",
                mutated.mutation.name()
            );
            if !seen.contains(&mutated.mutation) {
                seen.push(mutated.mutation);
            }
        }
        assert_eq!(seen.len(), MUTATIONS.len(), "a mutation was never chosen: {seen:?}");
    }

    #[test]
    fn a_mutation_never_makes_a_file_that_is_not_bytes() {
        let seeds = vec![a_design()];
        let mut rng = Rng::new(DEFAULT_SEED + 1);
        for _ in 0..200 {
            let mutated = mutate(&mut rng, &seeds, Target::Design);
            assert!(!mutated.bytes.is_empty(), "an empty file is not a mutation");
        }
    }

    // ------------------------------------------------------------ the corpus --

    /// The two targets break **their own** shapes.
    ///
    /// §182: a design's shape is its document and a script's shape is its
    /// list of commands, so the shape that is broken has to be the target's.
    /// A single shape mutation would break a design's document when it was
    /// handed a script — which is a mutation that changes nothing, and a
    /// mutation that changes nothing is a gate that cannot fail.
    #[test]
    fn the_two_targets_break_their_own_shapes() {
        let design = a_design();
        let script = a_script();

        // The script's mutation leaves a design alone: a design has no
        // commands to break.
        let mut rng = Rng::new(DEFAULT_SEED);
        assert_eq!(
            break_the_script(&mut rng, &design.bytes),
            design.bytes,
            "a design was broken as if it were a script"
        );

        // And the design's mutation leaves a script alone: a script has no
        // document to break.
        let mut rng = Rng::new(DEFAULT_SEED);
        assert_eq!(
            break_the_document(&mut rng, &script.bytes),
            script.bytes,
            "a script was broken as if it were a design"
        );
    }

    /// The script's shape is broken **with the file still valid JSON**.
    ///
    /// That is the whole point of the shape mutation: the other six break the
    /// file, and the reader turns it away at the first line — the reader
    /// working, but only its outermost part. This one has to reach the
    /// reader's own rules about commands.
    #[test]
    fn the_script_s_shape_is_broken_with_the_file_still_valid_json() {
        let script = a_script();
        let mut rng = Rng::new(DEFAULT_SEED);
        for _ in 0..200 {
            let broken = break_the_script(&mut rng, &script.bytes);
            assert_ne!(broken, script.bytes, "the mutation left the script alone");
            let value: serde_json::Value =
                serde_json::from_slice(&broken).expect("the broken script is still JSON");
            assert!(value.get("commands").is_some(), "the commands are still there");
        }
    }

    /// A run covers every target unless one is named.
    ///
    /// §182: a fuzzer that only watched one door would be watching the door
    /// the tree already knew about.
    #[test]
    fn a_run_covers_every_target_unless_one_is_named() {
        assert_eq!(Selection::All.targets().len(), Target::ALL.len());
        assert_eq!(Selection::One(Target::Script).targets(), vec![Target::Script]);

        let options =
            options_from(Path::new("."), &["--target".to_string(), "script".to_string()]).expect("read");
        assert_eq!(options.selection, Selection::One(Target::Script));
        assert!(
            options_from(Path::new("."), &["--target".to_string(), "nonsense".to_string()]).is_err(),
            "a target that is not one of the tree's doors was accepted"
        );
    }

    /// Each target's child is the command that reads **that** file.
    ///
    /// A design is read by `eval` and a script by `run`, and a target that
    /// called the wrong one would be handing a script to a reader that
    /// expects a design — which refuses it at the first line, and the run
    /// would be green for the wrong reason.
    #[test]
    fn each_target_s_child_reads_that_kind_of_file() {
        assert_eq!(Target::Design.child_args(), vec!["eval".to_string()]);
        assert_eq!(Target::Script.child_args(), vec!["run".to_string()]);
        // The report's door is the one that takes a **file**: a report that
        // arrived from another machine has no folder and no id. §195.
        assert_eq!(
            Target::Report.child_args(),
            vec!["crashes".to_string(), "replay".to_string(), "--file".to_string()]
        );
    }

    /// The three targets do not share a file name.
    ///
    /// The three runs share one working directory, so two targets that used
    /// one file name would overwrite each other's corpus: the script's seed
    /// would be the report's, and the fuzzer would break a report with the
    /// script's mutations and call it a pass. §201.
    #[test]
    fn the_targets_do_not_share_a_file_name() {
        let seeds: Vec<&str> = Target::ALL.iter().map(|t| t.seed_name()).collect();
        let mut unique = seeds.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), seeds.len(), "two targets use the same seed file: {seeds:?}");
        let inputs: Vec<&str> = Target::ALL.iter().map(|t| t.input_name()).collect();
        let mut unique = inputs.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), inputs.len(), "two targets use the same input file: {inputs:?}");
    }

    /// The report's shape is broken **with the file still valid JSON**.
    ///
    /// That is the whole point of the shape mutation: the other six break the
    /// file, and the reader turns it away at the first line — the reader
    /// working, but only its outermost part. This one has to reach the
    /// reader's own rules about the report's two documents and its steps.
    #[test]
    fn the_report_s_shape_is_broken_with_the_file_still_valid_json() {
        let report = a_report();
        let mut rng = Rng::new(DEFAULT_SEED);
        for _ in 0..200 {
            // **Through the dispatch**, not straight at the function: the
            // fuzzer reaches the report's shape by asking for the *target*,
            // and a target whose dispatch does not reach its own breakage
            // breaks nothing at all.
            let broken = structure(&mut rng, &report.bytes, Target::Report);
            assert!(
                broken != report.bytes,
                "the mutation left the report alone ({} bytes)",
                report.bytes.len()
            );
            let value: serde_json::Value =
                serde_json::from_slice(&broken).expect("the broken report is still JSON");
            assert!(value.get("replay").is_some(), "the report is still a report");
        }
    }

    #[test]
    fn the_seed_is_a_design_the_tree_reads() {
        // The hand-written design this file's tests break is a real design:
        // the tree reads it. The fuzzer's *own* corpus is written by the
        // tree's program — `craftkit-cli sample --out` — and the CLI's own
        // test suite is where that is proved, because the proof needs the
        // binary.
        let design = a_design();
        let text = String::from_utf8(design.bytes.clone()).expect("a design is text");
        let tag = format!("{}craftkit/1{}", '"', '"');
        assert!(text.contains(&tag), "the fixture is not a craftkit file");
        assert!(text.contains("nodes"), "the fixture has no nodes");
    }

    // ---------------------------------------------------------- the minimiser --

    #[test]
    fn the_minimiser_keeps_a_smaller_input_that_still_breaks() {
        let bytes = vec![b'a'; 100];
        // "Still breaks" means: not empty, and small.
        let still_breaks = |candidate: &[u8]| !candidate.is_empty() && candidate.len() <= 10;
        let smaller = minimise(&bytes, 40, still_breaks);
        assert!(smaller.len() < bytes.len(), "the minimiser kept the whole file");
        assert!(still_breaks(&smaller), "the minimiser kept something that does not break");
    }

    #[test]
    fn the_minimiser_keeps_the_input_when_nothing_smaller_breaks() {
        let bytes: Vec<u8> = (0..100u8).collect();
        let smaller = minimise(&bytes, 40, |_| false);
        assert_eq!(smaller, bytes, "the minimiser threw away an input that still broke");
    }

    #[test]
    fn the_minimiser_asks_no_more_than_its_budget() {
        let bytes = vec![b'a'; 200];
        let mut asked = 0usize;
        minimise(&bytes, 7, |_| {
            asked += 1;
            false
        });
        assert!(asked <= 7, "the minimiser asked {asked} times, and the budget was 7");
    }

    // ----------------------------------------------------------- the options --

    #[test]
    fn a_run_of_no_inputs_is_refused() {
        let options = Options { iterations: 0, ..Options::defaults(Path::new(".")) };
        let error =
            fuzz(Path::new("."), &options, Target::Design).expect_err("a run of nothing must not pass");
        assert!(error.contains("not a run"), "{error}");
    }

    #[test]
    fn an_unknown_flag_is_an_error() {
        let error =
            options_from(Path::new("."), &["--nonsense".to_string()]).expect_err("must not accept it");
        assert!(error.contains("--nonsense"), "{error}");
    }

    #[test]
    fn a_flag_without_a_value_is_an_error() {
        let error = options_from(Path::new("."), &["--iters".to_string()]).expect_err("must not accept it");
        assert!(error.contains("needs a value"), "{error}");
    }

    #[test]
    fn the_options_are_read_from_the_command_line() {
        let options = options_from(
            Path::new("/tmp"),
            &[
                "--cli".to_string(),
                "/bin/true".to_string(),
                "--iters".to_string(),
                "7".to_string(),
                "--seed".to_string(),
                "0x10".to_string(),
                "--timeout".to_string(),
                "3".to_string(),
                "--out".to_string(),
                "/tmp/elsewhere".to_string(),
            ],
        )
        .expect("the options are read");
        assert_eq!(options.cli, Some(PathBuf::from("/bin/true")));
        assert_eq!(options.iterations, 7);
        assert_eq!(options.seed, 0x10);
        assert_eq!(options.timeout, Duration::from_secs(3));
        assert_eq!(options.out, PathBuf::from("/tmp/elsewhere"));
    }

    #[test]
    fn a_seed_may_be_written_in_hexadecimal() {
        assert_eq!(parse_seed("0x5eedf0f21a3b5c7d").expect("reads"), 0x5eedf0f21a3b5c7d);
        assert_eq!(parse_seed("42").expect("reads"), 42);
        assert!(parse_seed("not a number").is_err());
    }

    // ------------------------------------------------------ the fake child --

    /// The fake child.
    ///
    /// It is a test in this file, and it does nothing unless the parent says
    /// so — the same shape as the tree's other child probes. When it *is*
    /// the child, it dies of a panic, and the test harness exits with 101:
    /// the code a Rust program exits with when it dies of a panic.
    ///
    /// The parent calls it by name, because the test harness runs the test
    /// whose name it is given.
    #[test]
    fn the_child_fixture_panics_only_when_it_is_the_child() {
        if std::env::var_os(FAKE_CHILD_ENV).is_none() {
            return;
        }
        // The seed is read, not broken: the fuzzer checks the corpus before
        // it breaks anything, and a child that died on the seed would fail
        // that check for the wrong reason. Everything else is a mutant.
        let args: Vec<String> = std::env::args().collect();
        if args.last().is_some_and(|last| last.ends_with(SEED_NAME)) {
            return;
        }
        panic!("the fake fuzz child, on purpose");
    }

    /// The fuzzer can find a crash.
    ///
    /// A fuzzer that has never found anything is a fuzzer nobody knows works.
    /// This runs the whole loop — mutate, hand it to a child, read the
    /// verdict, minimise it, write the finding out — against a child that
    /// dies on purpose, and checks that a finding came out the other end.
    ///
    /// The child is this very test binary, called with the name of the
    /// fixture above. That is portable: it needs no compiler, no shell and
    /// no second binary, and it is the same mechanism on every platform.
    #[test]
    fn the_fuzzer_finds_a_crash_it_made() {
        let out = scratch_dir("fuzz-self-test");
        let exe = std::env::current_exe().expect("the test binary");
        let options = Options {
            cli: Some(exe),
            iterations: 3,
            seed: DEFAULT_SEED,
            timeout: Duration::from_secs(120),
            out: out.clone(),
            selection: Selection::One(Target::Design),
            child_args: Some(vec!["--exact".to_string(), FAKE_CHILD_TEST.to_string(), "eval".to_string()]),
            child_env: vec![(FAKE_CHILD_ENV.to_string(), "1".to_string())],
            minimise_budget: 3,
            // The seed comes from here rather than from the tree's program:
            // the fake child cannot write a design, and the seed the tree
            // writes is proved by the CLI's own test suite.
            seed_bytes: Some(a_design().bytes),
        };

        let report = fuzz(Path::new("."), &options, Target::Design).expect("the fuzzer runs");
        assert_eq!(report.tried, 3, "the fuzzer tried {} inputs", report.tried);
        assert_eq!(report.crashes, 3, "the fake child dies on every input");
        assert_eq!(report.findings.len(), 3, "a crash was not written out");

        for finding in &report.findings {
            assert!(finding.is_file(), "{} was not written", finding.display());
            let bytes = crate::tree::read_bytes(finding).expect("reads");
            assert!(!bytes.is_empty(), "{} is empty", finding.display());
            let note = finding.with_extension("note.json");
            assert!(note.is_file(), "{} was not written", note.display());
            let text = crate::tree::read_text(&note).expect("reads");
            assert!(text.contains("\"verdict\": \"crash\""), "{text}");
        }

        let _ = std::fs::remove_dir_all(&out);
    }

    /// A directory of this test's own, so two runs cannot collide.
    fn scratch_dir(name: &str) -> PathBuf {
        crate::temp::dir(name).expect("a temporary folder")
    }
}
