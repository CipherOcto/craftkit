//! Crash reports: what a crash leaves behind.
//!
//! # Why this exists
//!
//! "It crashed" is the least actionable bug report there is. When a user says
//! the application closed unexpectedly, the only thing worth asking for is the
//! **reproduction**: the design as it was, and the commands that produced it.
//! So the application writes both, automatically, when it catches a panic —
//! the panic hook for a panic that escapes, and the render path for a panic
//! in the drawing code, which the hook cannot reach because that thread holds
//! the session's lock. §76.
//!
//! There is no unclean-exit path: a process that is killed, or that loses the
//! power, writes nothing, because catching a signal needs `unsafe` FFI and
//! this tree forbids `unsafe`. What *is* written is a report; what is not
//! written is a report for a kill.
//!
//! # What is written, and where
//!
//! ```text
//! <folder>/<id>/
//! └── report.json    what happened, which command, the id, the timestamp,
//!                    the document as it was, and the command trace
//! ```
//!
//! **One file.** The folder used to hold `design.json` and `replay.json` as
//! well — copies of parts of the report. Nothing read the design's copy, and
//! the replay's copy was read by a door nothing called, so a report that
//! arrived with two copies that disagreed had two answers, and the tree did
//! not notice. §185.
//!
//! The folder is [`report_dir`], which honours `CRAFTKIT_CRASH_DIR` so a test
//! can point it somewhere temporary and a CI job can collect it.
//!
//! # What is read, and from where
//!
//! A report is **input**, and it arrives from somewhere else: the recipe asks
//! for `report.json` from whoever had the crash. So a report is read from a
//! **file** as well as from a folder — [`read_report`] and [`reproduce_file`]
//! are the doors for the file in your hand, and they use the same reader and
//! the same replay the folder doors use, so the two cannot answer differently
//! about the same report. §195.
//!
//! # The rules
//!
//! - **Never panic.** This code runs *because* something else went wrong. Every
//!   fallible step returns `Result`, and a failure to write a report is logged
//!   and swallowed — losing the report is far better than losing the process.
//! - **Write atomically.** Into a temporary name, then rename. A crash halfway
//!   through must not leave a half-written file that looks like a real one —
//!   and the *folder* is written the same way, so a folder in the crash
//!   directory is a report that is finished. §185. The file part of that rule
//!   is the tree's **one** writer, `craftkit_core::atomic`, since every file
//!   the tree writes has to obey it and not three of them. §215.
//! - **Write privately.** A report holds the user's design. The files are
//!   created `0600` and the report's own folder is `0700`, where the platform
//!   has POSIX permissions, and with the process's own default where it does
//!   not; a folder that was already there keeps the permissions its owner gave
//!   it. §57.
//! - **Read it as a document.** A report is **input** — it arrives from
//!   another machine — so it is deserialized into the model's own types, not
//!   into a JSON tree. A tree costs thirty-two times the text it was parsed
//!   from; a document costs about four, and a report whose design is not a
//!   document is refused by the format instead of being materialised first.
//!   §72.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};

use crate::session::Session;

/// The environment variable that relocates the crash folder.
pub const CRASH_DIR_ENV: &str = "CRAFTKIT_CRASH_DIR";

/// How many bytes of file one byte of a design's memory costs, at the worst.
///
/// A report **carries** the design it was made from, so a report's size is the
/// design's file plus the report's own fields, and the design's file is the
/// design's memory times the format's expansion. The expansion is a fact about
/// the machine: measured, the report of a design the model accepts is
/// **43.4 MiB** on a 64-bit host and **65.7 MiB** on a 32-bit ARM target, for
/// designs whose own estimates are 31.7 MiB and 32.0 MiB. §231.
///
/// So the number is **3** — the worst case over the platforms, and a constant
/// rather than a measurement.
pub const REPORT_EXPANSION: usize = 3;

/// How many bytes a crash report file may be.
///
/// A report is **input**, not output: `docs/recipes/fix-a-crash.md` asks for
/// `report.json` from whoever had the crash, so the file arrives from another
/// machine. The design reader in `crates/io` caps its own files at 96 MiB for
/// the same reason — *"a hostile or truncated file must not reach
/// `serde_json::from_str` with a megabyte of input"* — and the script reader
/// in the CLI caps at [`craftkit_core::MAX_JSON_TEXT_BYTES`]. The crash reader
/// capped at nothing.
///
/// Measured: a 104 857 690-byte report was read in 105 ms without a word.
///
/// **And the cap has to hold the reports the tree writes itself**, or the app
/// writes crash reports it cannot read back. It did: on a 32-bit target the
/// report of a design the model accepts is 65.7 MiB, past the 64 MiB the
/// reader stopped at, and the reader said *"the input is longer than the limit
/// of 67108864 bytes"*. The cap is now derived from the same expansion the
/// design file uses, so it is the worst case over the platforms rather than
/// one machine's. §231.
///
/// The cap is a cap on **text**, and what the reader spends on the text is
/// another number. Before §72 the design was read as a JSON *tree*, and a
/// tree costs [`craftkit_core::JSON_EXPANSION`] — thirty-two — times its
/// text. Measured: a 66 000 101-byte report cost 1 260 292 KiB of resident
/// memory and 2.37 s to read. Read as a document, the same report costs
/// 67 796 KiB and 0.04 s, and a report whose design is not a document is
/// refused by the format instead of being materialised first.
pub const MAX_REPORT_BYTES: usize = craftkit_model::limits::MAX_DOCUMENT_BYTES * REPORT_EXPANSION;

/// How many bytes of memory reading one report may take.
///
/// The report's design is a **document**, and one document may hold
/// [`craftkit_model::limits::MAX_DOCUMENT_BYTES`]. Reading the report costs
/// the text as well — it is held while it is parsed — and the reader's own
/// room: the parser's buffers, and the process it runs in.
///
/// So the budget is the text plus twice the document's: one for the document,
/// one for the reader. A reader that builds a tree instead crosses it long
/// before the tree is finished — a tree costs thirty-two times the text,
/// and this budget allows eight. §72.
pub fn report_memory_budget(text_bytes: usize) -> usize {
    text_bytes.saturating_add(2 * craftkit_model::limits::MAX_DOCUMENT_BYTES)
}

/// One replayable step.
///
/// A trace step and a replay step are the same thing: a command id and the
/// parameters it was given, exactly. They used to be different — the trace
/// held a line of text that was parsed back — and the round trip was where the
/// reproduction quietly stopped matching. §47.
pub use crate::session::TraceStep as ReplayStep;

/// Where reports go.
///
/// `CRAFTKIT_CRASH_DIR` first, then the platform's data directory. A missing
/// or unwritable location is not an error here: the caller is already reporting
/// a failure and must not be handed a second one.
pub fn report_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(CRASH_DIR_ENV) {
        return PathBuf::from(dir);
    }
    base_dir().join("crashes")
}

/// One crash report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrashReport {
    /// A sortable identifier: `20250907-093012-7f3a`.
    pub id: String,
    /// Seconds since the Unix epoch.
    pub unix: u64,
    /// Which command was running, when one was.
    pub command: String,
    /// What went wrong, in a form a person can read.
    pub message: String,
    /// The document at the moment of the crash.
    ///
    /// This is the **target**: what the replay has to end up with.
    ///
    /// A **document**, not a JSON tree. A report is input — it arrives from
    /// another machine — and a tree of `serde_json::Value` costs up to
    /// thirty-two times the text it was parsed from. Read as a document, the
    /// same report costs a third of that, and a report whose "document" is
    /// not a document is refused by the format instead of being materialised
    /// first. §72.
    pub design: craftkit_model::Document,
    /// The document the trace starts from.
    ///
    /// The commands in `replay` are relative to this, not to an empty
    /// session — a session that opened a file and then ran three commands has
    /// a trace that only makes sense on top of that file. Without the base,
    /// the replay rebuilt a session that never existed. §47.
    ///
    /// A **document**, for the reason [`CrashReport::design`] is one. §72.
    #[serde(default)]
    pub base: craftkit_model::Document,
    /// Every command that had run, as `{ "command": …, "params": { … } }`.
    pub replay: Vec<ReplayStep>,
    /// How many commands are **missing** from `replay`.
    ///
    /// The trace is bounded, so a long session cannot keep every command.
    /// A report that dropped some says so here, rather than letting the
    /// reader believe the replay is the whole story.
    #[serde(default)]
    pub dropped: usize,
    /// The playhead, in ticks.
    #[serde(default)]
    pub at: i64,
}

fn base_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME")
        && !dir.trim().is_empty()
    {
        return PathBuf::from(dir).join("craftkit");
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        return PathBuf::from(home).join(".local").join("share").join("craftkit");
    }
    // A container with no home and no XDG: somewhere writable is better than
    // giving up, and the caller is told the path in the log either way.
    std::env::temp_dir().join("craftkit")
}

/// Build a report from the session as it stands.
///
/// Called from the panic handler, and from the render path for a panic in the
/// drawing code. It reads only; it never touches the session, because the
/// session may be mid-mutation. §76.
///
/// The report carries **three** things about the session: the document as it
/// was (the target), the document the trace starts from (the base), and the
/// commands between them. A report with only the first two is a snapshot; a
/// report with all three is a reproduction.
pub fn build(session: &Session, command: &str, message: &str) -> CrashReport {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let unix = (now / 1_000_000_000) as u64;
    let nanos = (now % 1_000_000_000) as u64;
    CrashReport {
        id: new_id(unix, nanos),
        unix,
        command: command.to_string(),
        message: message.to_string(),
        design: session.document().clone(),
        base: session.trace_base().clone(),
        replay: session.trace().iter().cloned().collect(),
        dropped: session.trace_dropped(),
        at: i64::try_from(session.at().raw()).unwrap_or(0),
    }
}

/// The id of a crash report: `YYYYMMDD-HHMMSS-xxxx`.
///
/// **UTC**, so the same crash gets the same id on every machine, and four hex
/// digits of sub-second position, so two crashes in the same second do not
/// usually share an id and the ids sort into the order things happened.
///
/// It used to be `{unix % 100_000_000:08}-{xxxx}` — a number with no date in
/// it, which restarted every 1 157 days. That made the ids unsortable across
/// the restart and said nothing about when anything happened, and it was not
/// the format this type's own documentation promised. §58.
fn new_id(unix: u64, nanos: u64) -> String {
    let (year, month, day, hour, minute, second) = civil(unix);
    // The high bits of the sub-second part, so the suffix grows with the
    // second and the ids sort into the order the crashes happened.
    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}-{:04x}", (nanos >> 8) & 0xffff)
}

/// The UTC civil date and time of a Unix timestamp.
///
/// Written out rather than pulled in: the tree has no date library, and this
/// is arithmetic rather than a dependency. The result is
/// `(year, month, day, hour, minute, second)`.
fn civil(unix: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix / 86_400) as i64;
    let rest = unix % 86_400;
    let (year, month, day) = civil_from_days(days);
    (year, month, day, (rest / 3600) as u32, ((rest % 3600) / 60) as u32, (rest % 60) as u32)
}

/// Days since 1970-01-01 to a civil `(year, month, day)`.
///
/// The epoch is shifted to 0000-03-01, which puts the leap day at the end of
/// a year and makes every month the same length, so the arithmetic below has
/// no special cases in it. The era is 400 years wide because 400 years hold
/// exactly 146 097 days — 365 × 400 + 97 leap days — so the leap pattern
/// repeats and the era length is a whole number.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Write a report to the crash folder.
///
/// Returns the directory written to, or the reason it could not be written.
/// **Never panics**: this is called from a failure path, and a second panic
/// there loses the process *and* the evidence.
pub fn write(report: &CrashReport) -> Result<PathBuf, String> {
    write_to(&report_dir(), report)
}

/// Write a report into a named folder.
///
/// The folder is a parameter rather than read from the environment inside,
/// because the environment is process-wide: a test that set it would race every
/// other test in the binary. Making it explicit is what lets the whole crash
/// path be tested at all.
pub fn write_to(base: &Path, report: &CrashReport) -> Result<PathBuf, String> {
    // A report is the only evidence a crash happened, so a second crash must
    // never destroy the first. Two crashes in the same second can land on the
    // same id — measured: 200 crashes in one second produced 198 folders — and
    // the second then overwrote the first's files. When the name is taken the
    // report is filed beside the one that is there, not on top of it. §58.
    let mut report = report.clone();
    let original = report.id.clone();
    let mut dir = base.join(sanitize(&original));
    let mut n = 2;
    while dir.exists() {
        report.id = format!("{original}-{n}");
        dir = base.join(sanitize(&report.id));
        n += 1;
    }
    // A base that was already there belongs to whoever made it — and
    // `CRAFTKIT_CRASH_DIR` can name one that other people use too. Only a
    // folder this call creates is ours to make private. §55's rule, applied
    // here.
    let base_is_ours = !base.exists();
    std::fs::create_dir_all(base).map_err(|e| format!("could not create {}: {e}", base.display()))?;
    if base_is_ours {
        restrict(base, true);
    }

    // The report is written into a folder **beside** the one it will live in,
    // and that folder is renamed into place. A folder in the crash directory
    // is then a report that is finished — the crash report is the only
    // evidence a crash happened, and half of one says nothing.
    //
    // The writer runs in a panic hook, on a process that is on its way out,
    // so the window between "the folder exists" and "the report is in it"
    // used to be the whole write: up to forty-four MiB of text, with the
    // folder already on disk and already listed as a report. §185.
    let staging = base.join(format!("{}{STAGING_SUFFIX}", sanitize(&report.id)));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| format!("could not create {}: {e}", staging.display()))?;
    // The report's own folder is always ours: it is where the design sits.
    restrict(&staging, true);

    // The report is written with **the tree's one writer** — the same layout
    // the design file is written in. It used to be written with
    // `serde_json::to_string_pretty`, which is the same layout *without* the
    // bound on the indentation: a report of a design the model accepts was
    // 73.5 MiB of text, and the crash reader refuses anything past 64 MiB, so
    // the app wrote crash reports it could not read back. §142.
    //
    // And it is the **only** file. The folder used to hold `design.json` and
    // `replay.json` too — copies of parts of the report. Nothing read the
    // design's copy, and the replay's copy was read by a door that nothing
    // called, so a report that arrived with two copies that disagreed had two
    // answers, and the tree noticed neither. §185.
    let report_json = to_json(&report)?;
    write_atomic(&staging.join(REPORT_FILE), &report_json)?;

    std::fs::rename(&staging, &dir).map_err(|e| {
        let _ = std::fs::remove_dir_all(&staging);
        format!("could not file the report in {}: {e}", dir.display())
    })?;

    Ok(dir)
}

/// Write a report to **one file**, named by the caller.
///
/// The twin of [`write_to`]: that one files a report into the crash folder,
/// where every report gets a folder of its own; this one writes the report
/// itself, to the path the caller names.
///
/// **The same bytes in both.** One serializer ([`to_json`]) and one atomic
/// writer ([`write_atomic`]) serve both, so a report that arrived in a file
/// and a report that was filed in the crash folder are the same report, and
/// the tree's one reader ([`read_report`]) reads both without being told
/// which it is holding.
///
/// The file door is what a headless program needs. Before this, the only way
/// to get a report onto disk was to have a **window** crash: the tree's four
/// production writers were all in the desktop binary's panic hook, and a
/// headless run left nothing behind. §200.
pub fn write_report_to(path: &Path, report: &CrashReport) -> Result<(), String> {
    let report_json = to_json(report)?;
    write_atomic(path, &report_json)
}

/// The command that was running, for a report that has to name it.
///
/// "None recorded" when nothing ran — a session that broke before its first
/// command has no command to name, and a report that says so is honest.
///
/// **One copy.** The desktop app's hook asked its own session for this, and
/// the CLI's writer asks the same question; two answers would be two
/// sentences about the same crash. §200.
pub fn last_command(session: &Session) -> String {
    session.log().last_command().map(str::to_string).unwrap_or_else(|| "none recorded".to_string())
}

/// The tree's JSON layout, for one value.
///
/// The same writer the design file uses, so a design is the same bytes in the
/// crash folder as it is on disk. §142.
fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let mut out: Vec<u8> = Vec::new();
    craftkit_core::writer::write_json(&mut out, value).map_err(|e| format!("could not serialize: {e}"))?;
    String::from_utf8(out).map_err(|e| format!("could not serialize: {e}"))
}

/// Write a file, atomically and privately.
///
/// The tree's **one** writer (§215): into a temporary name beside the target,
/// created private from the moment it exists, and then moved into place. A
/// report that is half written is not a report, and a report that holds the
/// user's design is not a file the world may read.
fn write_atomic(path: &Path, body: &str) -> Result<(), String> {
    craftkit_core::atomic::write_file(path, body.as_bytes(), craftkit_core::atomic::Visibility::OwnerOnly)
        .map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Make a folder private, where the platform has permissions to say so.
///
/// Not an error when it fails: this code runs because something else went
/// wrong, and a report that could not be protected is still better than no
/// report. The files' own modes are set at creation, so this only ever adds.
#[cfg(unix)]
fn restrict(path: &Path, dir: bool) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if dir { 0o700 } else { 0o600 };
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)) {
        log::warn!("could not restrict {}: {e}", path.display());
    }
}

#[cfg(not(unix))]
fn restrict(_path: &Path, _dir: bool) {}

/// Record a crash and log where it went.
///
/// The one function the app's panic hook and its exit path both call. It returns
/// the path so the CLI can print it; the app just logs it.
pub fn record(session: &Session, command: &str, message: &str) -> Option<PathBuf> {
    record_in(&report_dir(), session, command, message)
}

/// [`record`], into a named folder.
pub fn record_in(dir: &Path, session: &Session, command: &str, message: &str) -> Option<PathBuf> {
    let report = build(session, command, message);
    match write_to(dir, &report) {
        Ok(dir) => {
            let name =
                dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| report.id.clone());
            log::error!("crash report `{name}` written to {}", dir.display());
            Some(dir)
        }
        Err(e) => {
            log::error!("a crash happened ({message}) but no report could be written: {e}");
            None
        }
    }
}

/// List the reports in the crash folder, newest id last.
///
/// Never fails: an unreadable folder returns an empty list, because this is
/// shown in a menu, not in a test.
pub fn list() -> Vec<String> {
    list_in(&report_dir())
}

/// [`list`], in a named folder.
///
/// A folder in the crash directory is a report only if it **holds** one. The
/// writer renames the folder into place when the report is finished, so a
/// folder that is there is a report that is there — but a folder can also
/// arrive from another machine, and a directory is not a report. Listing one
/// offered the reader a report that could not be read. §185.
pub fn list_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter(|e| e.path().join(REPORT_FILE).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        // A folder that is being written into is not a report.
        .filter(|name| !name.ends_with(STAGING_SUFFIX))
        .collect();
    ids.sort_unstable();
    ids
}

/// The name of the one file a report is made of.
const REPORT_FILE: &str = "report.json";

/// The suffix of the folder a report is written into before it is filed.
///
/// A folder with this suffix is a report being written, not a report. §185.
const STAGING_SUFFIX: &str = ".partial";

/// Read one report back.
pub fn show(id: &str) -> Result<CrashReport, String> {
    show_in(&report_dir(), id)
}

/// [`show`], in a named folder.
///
/// The report is read by [`read_report`], which is also the door for a report
/// in a file — one copy of the rules, checked once. §195.
pub fn show_in(dir: &Path, id: &str) -> Result<CrashReport, String> {
    read_report(&report_path(dir, id)?)
}

/// Read one report from a **file** — one that arrived from another machine.
///
/// A report is **input**: the recipe asks for `report.json` from whoever had
/// the crash, so the file arrives with no crash folder around it and no id to
/// look it up by. Every other door in the tree reads a report by id, out of a
/// folder; this is the door for the file in your hand. §195.
///
/// It is the **same reader** the folder door uses, so the two cannot answer
/// differently about the same report: one reader, one answer. §185.
///
/// The report's **documents** are checked the way a design file is, and the
/// design is checked as well as the base. §183: the reader checked the base
/// and not the design, and the design is the report's **target** — what the
/// replay has to end up with. A target the model refuses is not a target,
/// and the replay's verdict ("reproduced", or "diverged") is a claim about
/// it.
///
/// So a report that holds a document the model would refuse is **refused with
/// the reason**, by the reader, and not shown. The same document in a design
/// file is refused the same way: the tree has one promise about what a
/// document is, and it keeps it in every reader.
pub fn read_report(path: &Path) -> Result<CrashReport, String> {
    let body = read_bounded(path, MAX_REPORT_BYTES)?;
    let mut report: CrashReport = serde_json::from_str(&body)
        .map_err(|e| format!("{} is not a readable report: {e}", path.display()))?;
    report.design = checked_document(report.design, "the report's design")?;
    report.base = checked_document(report.base, "the report's base design")?;

    // A step whose parameters are not an object is a broken instruction, and
    // a report is **input**: it arrives from another machine. §202.
    for (index, step) in report.replay.iter().enumerate() {
        if !step.params.is_object() {
            return Err(format!(
                "{} is not a readable report: step {} has a `params` that is not an object",
                path.display(),
                index + 1
            ));
        }
    }
    Ok(report)
}

/// The path of a report's one file, or the reason there is not one.
///
/// A folder in the crash directory is a report only if it holds one. The
/// writer renames the folder into place when the report is finished, so a
/// folder that is there is a report that is there — but a folder can also
/// arrive from another machine, and a directory is not a report. Saying so is
/// better than the I/O error the reader used to give, which named a file that
/// was never there and said nothing about the folder. §185.
fn report_path(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let folder = dir.join(sanitize(id));
    let path = folder.join(REPORT_FILE);
    if !path.is_file() {
        return Err(format!("there is no crash report in {}: it has no {REPORT_FILE}", folder.display()));
    }
    Ok(path)
}

/// Read a file, refusing one that is larger than `max`.
///
/// The limit is applied **while reading**, by the shared bounded reader: the
/// file is cut off at the limit, so a ten-gigabyte report costs one buffer
/// rather than ten gigabytes of memory. A file that is not text is an error
/// here, not a panic — the crash folder can hold whatever somebody copied
/// into it.
fn read_bounded(path: &Path, max: usize) -> Result<String, String> {
    craftkit_core::read_file_bounded(path, max).map_err(|e| format!("could not read {}: {e}", path.display()))
}

/// Read a report's replay steps.
pub fn replay(id: &str) -> Result<Vec<ReplayStep>, String> {
    replay_in(&report_dir(), id)
}

/// [`replay`], in a named folder.
///
/// The steps are in the report, not in a file of their own. The folder used to
/// hold a `replay.json` beside the report, and the two could disagree: one
/// folder, two answers — `replay_in` said the report held one command and
/// `show_in` said it held nine — and the tree noticed neither. The steps come
/// from the report now, and this door reads the same file every other door
/// does. §185.
pub fn replay_in(dir: &Path, id: &str) -> Result<Vec<ReplayStep>, String> {
    Ok(show_in(dir, id)?.replay)
}

/// What one replayed step did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepResult {
    /// The command that was run.
    pub command: String,
    /// Whether it succeeded.
    pub ok: bool,
    /// What it said — the outcome's message, or the refusal.
    pub message: String,
}

/// What a replay produced, and whether it reproduced the document.
///
/// The verdict is the point. A replay that rebuilds a *different* document and
/// reports nothing is the defect this whole path had: it looked like a
/// reproduction and was not one. §47.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayOutcome {
    /// How many steps were replayed.
    pub steps: usize,
    /// What each step did, in order.
    pub results: Vec<StepResult>,
    /// The steps that refused, with the reason.
    pub failures: Vec<String>,
    /// Whether the replayed document **is** the document in the report.
    pub reproduced: bool,
    /// How the replayed document differs from the recorded one.
    pub differences: Vec<String>,
    /// How many commands the report's trace had to drop.
    pub dropped: usize,
    /// The replayed session, for a caller that wants to show it.
    pub session: serde_json::Value,
}

/// Replay a report and say whether it reproduces the document that crashed.
///
/// This is the honest form of the replay. It starts from the report's **base**
/// — the document the recorded commands were relative to — runs the steps, and
/// then compares the result with the design the report recorded. §47.
pub fn reproduce(id: &str) -> Result<ReplayOutcome, String> {
    reproduce_in(&report_dir(), id)
}

/// [`reproduce`], in a named folder.
pub fn reproduce_in(dir: &Path, id: &str) -> Result<ReplayOutcome, String> {
    reproduce_report(show_in(dir, id)?)
}

/// Replay a report you already hold.
///
/// The same replay the folder door runs: the same bound on the steps, the
/// same start on the report's **base**, and the same comparison with the
/// design the report recorded. One replay, one answer. §185.
///
/// The report is taken by value because the base is **moved**, not cloned: a
/// document costs memory, and a second copy of it is a second copy of it.
/// §64.
pub fn reproduce_report(report: CrashReport) -> Result<ReplayOutcome, String> {
    // The step count is checked **before** the first step runs. A report with
    // a million steps is not a report this tree wrote, and the cost of
    // running it is quadratic in the document it builds.
    //
    // The bound is the tree's own, and the script door shares it: a replay
    // and a script are the same shape — a list of commands — and the tree
    // hands one to the other. §181.
    if report.replay.len() > crate::MAX_COMMAND_STEPS {
        return Err(format!(
            "the report has {} step(s), and a replay runs at most {}",
            report.replay.len(),
            crate::MAX_COMMAND_STEPS
        ));
    }
    let mut session = Session::new("replay");
    // The base is moved, not cloned: it is raw JSON, and raw JSON costs
    // thirty-two times its own size in memory. §64.
    //
    // It was checked by the reader, which is where the check belongs: one
    // reader, one rule, and the reason names the report's base. §183.
    session.load_document(report.base);
    session.seek_to(report.at);

    let mut results = Vec::with_capacity(report.replay.len());
    let mut failures = Vec::new();
    for step in &report.replay {
        // A step whose parameters are bigger than the trace's budget is
        // refused by the engine (§70): the trace could not hold it, and a
        // command that cannot be recorded cannot be reproduced. A report
        // written before that rule can hold such a step, and the replay
        // reports it as a failure rather than pretending it ran.
        match session.execute(&step.command, &step.params) {
            Ok(outcome) => {
                results.push(StepResult {
                    command: step.command.clone(),
                    ok: true,
                    message: outcome.message,
                });
            }
            Err(e) => {
                failures.push(format!("{}: {e}", step.command));
                results.push(StepResult { command: step.command.clone(), ok: false, message: e.to_string() });
            }
        }
    }

    let replayed = session.document().clone();
    let differences = differences(&report.design, &replayed);
    Ok(ReplayOutcome {
        steps: report.replay.len(),
        results,
        failures,
        reproduced: differences.is_empty(),
        differences,
        dropped: report.dropped,
        session: session.snapshot_json(),
    })
}

/// Replay the report in a **file** — one that arrived from another machine.
///
/// The door for the file you were handed. It reads the file with the tree's
/// one report reader and replays it with the tree's one replay, so a report
/// that arrives and a report that is filed are replayed the same way. §195.
pub fn reproduce_file(path: &Path) -> Result<ReplayOutcome, String> {
    reproduce_report(read_report(path)?)
}

/// What a crash-report action can fail with.
///
/// Two kinds, and they are the two the tree already uses for the same two
/// failures: a refusal about the request (`bad-parameter`) and a report that
/// is not there (`not-found`). §113.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionError {
    /// The action is not one of the three.
    Unknown(String),
    /// The action needs a report's id and did not get one.
    MissingId,
    /// The report is not there.
    NotFound(String),
}

impl ActionError {
    /// The kind of failure, for the front end that reports it.
    pub fn kind(&self) -> &'static str {
        match self {
            ActionError::Unknown(_) | ActionError::MissingId => "bad-parameter",
            ActionError::NotFound(_) => "not-found",
        }
    }

    /// What a person reads.
    pub fn message(&self) -> String {
        match self {
            ActionError::Unknown(action) => {
                format!("`{action}` is not a crash action; try list, show or replay")
            }
            ActionError::MissingId => "`id` is required".to_string(),
            ActionError::NotFound(e) => e.clone(),
        }
    }
}

/// Run a crash-report action, and answer with JSON.
///
/// **One dispatcher for every front end.** The CLI prints the crash reports
/// for a person, the control channel answers with JSON over a socket, and the
/// MCP server answers with JSON to a tool-calling agent — and the control
/// channel had its own copy of the three branches, with its own wording for a
/// missing id and its own idea of what a replay answer looked like, while the
/// MCP server had no way to ask at all. The rules about *what* an action does
/// live here, so the front ends cannot drift apart. §113.
///
/// `from` is the name of the design the caller is looking at, and the replay's
/// answer says which one — a replay runs against the report's own base, and
/// the caller should not have to guess whether it replaced their design.
pub fn action_in(dir: &Path, action: &str, id: Option<&str>, from: &str) -> Result<Json, ActionError> {
    match action {
        "list" => Ok(json!({ "ids": list_in(dir) })),
        "show" => {
            let id = id.ok_or(ActionError::MissingId)?;
            let report = show_in(dir, id).map_err(ActionError::NotFound)?;
            Ok(serde_json::to_value(report).unwrap_or(Json::Null))
        }
        "replay" => {
            let id = id.ok_or(ActionError::MissingId)?;
            let outcome = reproduce_in(dir, id).map_err(ActionError::NotFound)?;
            Ok(json!({
                "steps": outcome.steps,
                "failures": outcome.failures,
                "reproduced": outcome.reproduced,
                "differences": outcome.differences,
                "dropped": outcome.dropped,
                "session": outcome.session,
                "from": from,
            }))
        }
        other => Err(ActionError::Unknown(other.to_string())),
    }
}

/// Read a document out of a report's JSON.
///
/// The id counter is **not** part of a document's serialization — it is
/// derived from the ids in use, so a hand-edited file cannot make the document
/// hand out an id that already exists. The reader therefore reseats it, and
/// so does this: a base document that was not reseated would hand out ids
/// that the original session had already used, and every later step would
/// name the wrong node.
///
/// The document is also **validated**, the way the design reader validates a
/// design. A base with two nodes sharing an id is refused by
/// `crates/io` — *"two objects share an id"* — and accepted here until §48,
/// where a step naming that id edited the first of the two.
///
/// The value is taken **by value**. It used to be taken by reference and
/// cloned, and the clone was a second copy of a structure that can be a
/// gigabyte — the base of a script is raw JSON, and raw JSON costs thirty-two
/// times its own size in memory. §64.
pub fn document_from_json(value: serde_json::Value) -> Result<craftkit_model::Document, String> {
    let doc: craftkit_model::Document = serde_json::from_value(value)
        .map_err(|e| format!("the base design is not readable ({e}) — it cannot be used"))?;
    checked_document(doc, "the base design")
}

/// Check a document that arrived from outside.
///
/// The id counter is derived from the ids in use, so a document that has been
/// edited by hand can hand out an id that is already in use — and a replay
/// built on it would name the wrong node. So the counter is reseated and the
/// document is validated before anything is done with it. §48.
///
/// **The name says which document it is**, because the tree has three readers
/// and three documents: a report's design, a report's base, and a script's
/// base. They are checked the same way and they are named differently, so
/// the reason a reader gives names the thing that is wrong. §183.
pub fn checked_document(
    mut doc: craftkit_model::Document,
    which: &str,
) -> Result<craftkit_model::Document, String> {
    doc.reseat_id_counter();
    doc.validate().map_err(|e| format!("{which} is not usable: {e}"))?;
    Ok(doc)
}

/// How two designs differ, in words.
///
/// Empty means they are the same. Never empty when they are not: the verdict
/// in [`ReplayOutcome`] is read off this, so a difference nobody can describe
/// still has to say *that* it is there.
fn differences(recorded: &craftkit_model::Document, replayed: &craftkit_model::Document) -> Vec<String> {
    // The id counter is **not** part of a document's serialization — it is
    // derived from the ids in use — so two documents with the same nodes and
    // the same settings are the same document, whatever their counters say.
    // §72: the comparison used to be of the serialized forms, which skipped
    // the counter, and it is now of the documents themselves.
    if recorded.meta == replayed.meta && recorded.nodes == replayed.nodes {
        return Vec::new();
    }
    let mut out = Vec::new();
    // The node's name and id, as the report had them and as the replay has
    // them. Read from the documents rather than from a JSON tree, so a report
    // that is not a document never gets this far. §72.
    let nodes = |doc: &craftkit_model::Document| -> Vec<(String, String)> {
        doc.nodes.iter().map(|n| (n.name.clone(), n.id.get().to_string())).collect()
    };
    let recorded_nodes = nodes(recorded);
    let replayed_nodes = nodes(replayed);
    if recorded_nodes.len() != replayed_nodes.len() {
        out.push(format!(
            "the replay has {} node(s), the report has {}",
            replayed_nodes.len(),
            recorded_nodes.len()
        ));
    }
    for (index, (want, got)) in recorded_nodes.iter().zip(replayed_nodes.iter()).enumerate() {
        if want.0 != got.0 {
            out.push(format!(
                "node {} is `{}` in the report and `{}` in the replay",
                index + 1,
                want.0,
                got.0
            ));
        }
        if want.1 != got.1 {
            out.push(format!(
                "node {} has id {} in the report and id {} in the replay",
                index + 1,
                want.1,
                got.1
            ));
        }
    }
    if recorded.meta != replayed.meta {
        out.push("the two designs have different settings".to_string());
    }
    if out.is_empty() {
        out.push("the replayed design differs from the recorded one".to_string());
    }
    out
}

/// Reject anything that is not a plain report id.
///
/// The id comes from a command line and is joined to a path. `..` in there would
/// otherwise let `craftkit-cli crashes show ../../etc` read outside the folder.
fn sanitize(id: &str) -> String {
    let cleaned: String =
        id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
    if cleaned.is_empty() { "unknown".to_string() } else { cleaned }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A fresh folder per call, named after the test so a failing run leaves
    /// evidence behind rather than deleting it.
    ///
    /// Deliberately *not* the environment variable: `set_var` is process-wide
    /// and would make every test in the binary race this one. That is exactly
    /// why [`write_to`] and its siblings take the folder as a parameter.
    ///
    /// The rule is the tree's, and it lives in one place: §212 gave the
    /// temporary folder a home in `craftkit_core::temp`, because the rule had
    /// been written in sixteen places at once. §211 fixed it here first.
    fn temp_dir(tag: &str) -> PathBuf {
        craftkit_core::temp::dir(tag).expect("a temporary folder")
    }

    /// The mode of a path, where the platform has modes.
    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).expect("metadata").permissions().mode() & 0o777
    }

    /// A report written to a **file** is the same report the crash folder
    /// holds, and the tree's one reader reads it.
    ///
    /// §200: the tree could read a report and could not write one without a
    /// window — the only writer anywhere in the tree was the desktop
    /// binary's panic hook. So the file door is new, and it has to be the
    /// **same** report: one serializer, so a file and a folder hold the same
    /// bytes, and one reader, so the tree does not have two answers about
    /// one report.
    #[test]
    fn the_file_door_and_the_folder_door_write_the_same_bytes() {
        let dir = temp_dir("file-door");
        let session = Session::sample_recorded();
        let report = build(&session, "node.add", "written on request");

        let file = dir.join("report.json");
        write_report_to(&file, &report).expect("the report is written to a file");

        // The same report, filed in the crash folder.
        let folder = dir.join("folder");
        let filed = write_to(&folder, &report).expect("the report is filed");
        let filed_bytes = std::fs::read(filed.join(REPORT_FILE)).expect("the filed report");
        let file_bytes = std::fs::read(&file).expect("the written report");
        assert_eq!(
            file_bytes.len(),
            filed_bytes.len(),
            "the file and the folder hold different bytes: {} against {}",
            file_bytes.len(),
            filed_bytes.len()
        );
        assert!(file_bytes == filed_bytes, "the file and the folder hold the same bytes and different ones");

        // And the tree's one reader reads the file.
        let read = read_report(&file).expect("the report is readable");
        assert_eq!(read.design, report.design, "the design did not survive the round trip");
        assert_eq!(read.base, report.base, "the base did not survive the round trip");
        assert_eq!(read.replay, report.replay, "the replay did not survive the round trip");
        assert_eq!(read.id, report.id, "the id did not survive the round trip");
    }

    /// The sample session with its record kept has **something to replay**.
    ///
    /// §200: the sample session wipes its trace and its log, because a
    /// session that opens with three entries in its log is a lie about what
    /// the user did. A crash report is the opposite — it is exactly a record
    /// of what ran — so the report needs the same session with the record
    /// still in it. Without it, the only report the tree could write
    /// headlessly had an **empty replay** and a base equal to its own
    /// design: a report that replayed nothing and "reproduced" itself.
    #[test]
    fn the_sample_session_with_its_record_kept_has_something_to_replay() {
        let session = Session::sample_recorded();
        assert_eq!(session.trace().len(), 3, "the sample ran three commands: {:?}", session.trace());
        assert!(session.trace_base().nodes.is_empty(), "the commands did not run on an empty document");
        assert_eq!(session.document().nodes.len(), 3, "the sample has three nodes");

        // And the session the sample *opens* has none of it: the two are one
        // session, kept two ways.
        let opened = Session::sample();
        assert!(opened.trace().is_empty(), "the sample session opens with nothing recorded");
        assert_eq!(opened.document().nodes.len(), 3, "the opened sample has three nodes");
    }

    /// A report whose step has parameters that are not an object is refused.
    ///
    /// §202: the report is **input** — it arrives from another machine — and
    /// its steps are instructions. The engine's parameter reader treats a
    /// non-object as an empty view, so the step ran with parameters nobody had
    /// written.
    #[test]
    fn a_report_whose_step_has_broken_parameters_is_refused() {
        let dir = temp_dir("bad-params");
        let session = Session::sample_recorded();
        let mut report = build(&session, "node.add", "written on request");
        report.replay[0].params = json!([1, 2, 3]);
        let path = dir.join("report.json");
        write_report_to(&path, &report).expect("the report is written");

        let err = read_report(&path).expect_err("a report with broken parameters is refused");
        assert!(err.contains("`params` that is not an object"), "{err}");
        assert!(err.contains("step 1"), "{err}");
    }

    /// A report whose **design** the model refuses is refused, by name.
    ///
    /// §183: the reader checked the report's *base* and not its *design* — and
    /// the design is the report's **target**: what the replay has to end up
    /// with. So a report whose design had a frame rate of zero was read,
    /// shown and replayed, while the same document in a design file was
    /// refused. The tree has one promise about what a document is, and every
    /// reader keeps it.
    ///
    /// The reason names the design, because the report has two documents and
    /// the base was already checked.
    #[test]
    fn a_report_whose_design_the_model_refuses_is_refused() {
        let dir = temp_dir("bad-design");
        let s = Session::sample();
        let mut report = build(&s, "node.add", "boom");
        report.design.meta.fps = 0;
        write_to(&dir, &report).expect("the report is written");

        let err = show_in(&dir, &report.id).expect_err("a design the model refuses is refused");
        assert!(err.contains("the report's design is not usable"), "{err}");

        // And the replay door says the same thing, rather than replaying onto
        // a design nothing can use and reporting a verdict about it.
        let err = reproduce_in(&dir, &report.id).expect_err("the replay refuses it too");
        assert!(err.contains("the report's design is not usable"), "{err}");
    }

    /// A crash report of a design the model accepts can be read back.
    ///
    /// §142. The report's files were written with `serde_json::to_string_pretty`
    /// — the tree's layout *without* the bound on the indentation — so a
    /// report of a design the model accepts was 73.5 MiB of text, and the
    /// crash reader refuses anything past 64 MiB. The app wrote crash reports
    /// it could not read back, and the report is the one thing a crash must
    /// produce.
    #[test]
    fn a_crash_report_of_a_big_design_can_be_read_back() {
        let dir = temp_dir("big-design");
        let mut s = Session::sample();
        s.replace_document(a_big_design());
        let report = build(&s, "prop.set", "boom");
        let written = write_to(&dir, &report).expect("the report is written");

        // The report is inside what the reader takes.
        let size = std::fs::metadata(written.join("report.json")).expect("metadata").len();
        let limit = u64::try_from(MAX_REPORT_BYTES).expect("the limit fits in a u64");
        assert!(size <= limit, "the report is {size} bytes, and the reader stops at {MAX_REPORT_BYTES}");

        // And it reads back as the report it was.
        //
        // The id counter is not part of the comparison: it is derived from
        // the ids in use, and the tree says so itself — §72 — so a document is
        // its settings and its nodes.
        let back = show_in(&dir, &report.id).expect("the report reads back");
        assert_eq!(back.design.meta, report.design.meta, "the settings came back changed");
        assert_eq!(back.design.nodes, report.design.nodes, "the nodes came back changed");
        assert_eq!(back.command, report.command, "the command came back changed");
    }

    /// The report's cap covers the worst case over the platforms the tree runs on.
    ///
    /// §231: a report carries the design it was made from, and the design's file
    /// is the design's memory times an expansion that is a fact about the
    /// machine. Measured: the report of a design the model accepts is 43.4 MiB
    /// on a 64-bit host and **65.7 MiB** on a 32-bit ARM target — and the reader
    /// stopped at 64 MiB, so on the device the app wrote crash reports it could
    /// not read back.
    ///
    /// The number is written down, because a constant cannot be a measurement.
    #[test]
    fn the_reports_cap_covers_the_worst_case_over_the_platforms() {
        // The measured worst case, in ten-thousandths: 2.1524 bytes of report
        // for every byte of estimate, on a 32-bit ARM target.
        const MEASURED_WORST_X_10_000: usize = 21_524;
        // Asserted with locals so clippy does not fold them into a constant
        // comparison — the same way `crates/model/src/lib.rs` asserts its own.
        let (expansion, worst) = (REPORT_EXPANSION, MEASURED_WORST_X_10_000);
        assert!(
            expansion * 10_000 >= worst,
            "the expansion is {expansion}, and the worst case measured over the platforms the \
             tree runs on is 2.1524 — a 64-bit host measures 1.435 and a 32-bit ARM target 2.152"
        );
    }

    /// A design the model accepts, and a big one: 32 properties with 10 000
    /// keyframes on each — 29.3 MiB of memory, inside the model's budget.
    fn a_big_design() -> craftkit_model::Document {
        let mut doc = craftkit_model::Document::empty("Big");
        let id = doc.add_node("n", craftkit_model::NodeKind::Source).expect("a node");
        let mut group = craftkit_model::PropGroup::new("props", "Properties");
        for p in 0..32u64 {
            let mut prop =
                craftkit_model::Property::new(p + 1, format!("p{p}"), craftkit_core::Value::Num(1.0));
            for t in 0..10_000i128 {
                let Ok(key) = craftkit_model::Key::new(
                    craftkit_core::Tick::from_raw(t),
                    craftkit_core::Value::Num(t as f64),
                    craftkit_model::Easing::Linear,
                ) else {
                    break;
                };
                if prop.keys.set(key).is_err() {
                    break;
                }
            }
            group.push(prop);
        }
        doc.get_mut(id).expect("just added").props = group;
        doc
    }

    /// A crash report holds the user's design, and it was written
    /// world-readable: the folder `0775` and the files `0664`, both measured.
    /// §57.
    #[cfg(unix)]
    #[test]
    fn a_crash_report_is_not_readable_by_other_users() {
        let dir = temp_dir("private");
        let s = Session::sample();
        let report = build(&s, "prop.set", "boom");
        let written = write_to(&dir, &report).expect("written");

        assert_eq!(mode(&written), 0o700, "the report's folder is not private");
        let m = mode(&written.join(REPORT_FILE));
        assert_eq!(m, 0o600, "the report is readable by other users: mode was {m:o}");
    }

    /// Every door agrees about a report.
    ///
    /// `replay_in` said the report held one command and `show_in` said it
    /// held nine, because the folder held two copies of the steps and each
    /// door read one of them. The report is the only copy now, so the doors
    /// cannot disagree. §185.
    #[test]
    fn every_door_agrees_about_how_many_commands_a_report_holds() {
        let dir = temp_dir("one-answer");
        let mut s = Session::new("one answer");
        for name in ["A", "B", "C"] {
            s.execute("node.add", &json!({ "name": name, "kind": "cue" })).expect("ok");
        }
        let report = build(&s, "node.add", "boom");
        let written = write_to(&dir, &report).expect("written");
        // A second copy of the steps, saying something else. A report arrives
        // from another machine, and it may carry anything.
        std::fs::write(written.join("replay.json"), "[]").expect("written");

        let shown = show_in(&dir, &report.id).expect("the report reads");
        let replayed = replay_in(&dir, &report.id).expect("the steps read");
        assert_eq!(replayed.len(), shown.replay.len(), "the doors disagree about the number of commands");
        assert_eq!(replayed, shown.replay, "the doors disagree about which commands");
    }

    /// A folder that is not a report is not offered as one.
    ///
    /// The crash directory held a folder for every report, and the lister
    /// offered each one — including a folder that was still being written, and
    /// one that arrived from another machine with nothing in it. The reader
    /// then said "could not read the line: No such file or directory", which
    /// names a file that was never there and says nothing about the folder.
    /// §185.
    #[test]
    fn a_folder_that_is_not_a_report_is_not_listed_and_says_so() {
        let dir = temp_dir("not-a-report");
        let empty = dir.join("20250101-000000-dead");
        std::fs::create_dir_all(&empty).expect("dir");
        assert!(list_in(&dir).is_empty(), "an empty folder was offered as a report");

        let err = show_in(&dir, "20250101-000000-dead").expect_err("there is no report");
        assert!(err.contains("there is no crash report"), "{err}");
        assert!(err.contains(REPORT_FILE), "{err}");

        // And a folder that is still being written is not one either.
        let staging = dir.join("20250101-000001-cafe.partial");
        std::fs::create_dir_all(&staging).expect("dir");
        std::fs::write(staging.join(REPORT_FILE), "{}").expect("write");
        assert!(list_in(&dir).is_empty(), "a folder being written into was offered as a report");
    }

    /// A report that cannot be written leaves no folder behind.
    ///
    /// The report is written into a folder beside the one it will live in, and
    /// that folder is renamed into place. So a crash — or a full disk —
    /// leaves the crash directory as it was, rather than a folder that is not
    /// a report. §185.
    #[test]
    fn a_report_that_cannot_be_written_leaves_no_folder_behind() {
        let dir = temp_dir("cannot-write");
        let s = Session::sample();
        let report = build(&s, "prop.set", "boom");
        // A *file* where the staging folder goes: it cannot be made, so the
        // report cannot be written.
        let staging = dir.join(format!("{}{STAGING_SUFFIX}", report.id));
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(&staging, b"not a folder").expect("the blocker");
        assert!(write_to(&dir, &report).is_err(), "the write should have failed");
        // And nothing is left behind but the blocker: the report's folder is
        // not there, so a folder that is not a report is not there either.
        assert!(!dir.join(&report.id).exists(), "a folder was left behind that is not a report");
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![format!("{}{STAGING_SUFFIX}", report.id)],
            "the crash directory holds {left:?}"
        );
    }

    /// A folder that was already there belongs to whoever made it.
    ///
    /// `CRAFTKIT_CRASH_DIR` can name one that other people use too, and
    /// changing its permissions behind its owner's back is §55's defect.
    #[cfg(unix)]
    #[test]
    fn a_crash_folder_that_was_already_there_keeps_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("shared");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let s = Session::sample();
        let report = build(&s, "prop.set", "boom");
        let written = write_to(&dir, &report).expect("written");

        assert_eq!(mode(&dir), 0o755, "the tool changed a folder it did not create");
        // And the report inside it is still private.
        assert_eq!(mode(&written), 0o700, "the report's folder is not private");
        assert_eq!(mode(&written.join(REPORT_FILE)), 0o600, "the report is readable by other users");
    }

    /// A folder the tool creates is made private.
    #[cfg(unix)]
    #[test]
    fn a_crash_folder_the_tool_creates_is_private() {
        let dir = temp_dir("made").join("crashes");
        let _ = std::fs::remove_dir_all(&dir);
        let s = Session::sample();
        let report = build(&s, "prop.set", "boom");
        write_to(&dir, &report).expect("written");
        assert_eq!(mode(&dir), 0o700, "the tool's own crash folder is not private");
    }

    /// The crash actions are one rule, and every front end asks it the same
    /// way.
    ///
    /// §113: the control channel and the MCP server each had their own copy of
    /// the three branches — list, show, replay — and the copies could drift.
    /// The dispatcher is in the engine, and it says the same things to both.
    #[test]
    fn the_crash_actions_are_one_rule() {
        let dir = temp_dir("actions");
        let session = Session::sample();
        let report = build(&session, "prop.set", "boom");
        let id = report.id.clone();
        write_to(&dir, &report).expect("written");

        // list
        let listed = action_in(&dir, "list", None, "Sample plan").expect("listed");
        let ids = listed.get("ids").and_then(Json::as_array).expect("ids");
        assert_eq!(ids.len(), 1, "the folder holds one report: {listed}");
        assert_eq!(ids[0].as_str(), Some(id.as_str()));

        // show
        let shown = action_in(&dir, "show", Some(&id), "Sample plan").expect("shown");
        assert_eq!(shown.get("id").and_then(Json::as_str), Some(id.as_str()));

        // replay
        let replayed = action_in(&dir, "replay", Some(&id), "Sample plan").expect("replayed");
        assert_eq!(replayed.get("reproduced").and_then(Json::as_bool), Some(true), "{replayed}");
        assert_eq!(replayed.get("from").and_then(Json::as_str), Some("Sample plan"));

        // The three failures, and their kinds.
        let missing = action_in(&dir, "show", None, "Sample plan").expect_err("refused");
        assert_eq!(missing.kind(), "bad-parameter");
        assert_eq!(missing, ActionError::MissingId);

        let unknown = action_in(&dir, "delete", None, "Sample plan").expect_err("refused");
        assert_eq!(unknown.kind(), "bad-parameter");
        assert!(unknown.message().contains("not a crash action"), "{}", unknown.message());

        let absent =
            action_in(&dir, "show", Some("19700101-000000-0000"), "Sample plan").expect_err("refused");
        assert_eq!(absent.kind(), "not-found");
    }

    /// The id is the format this type's own documentation promises, and it
    /// says when the crash happened.
    #[test]
    fn the_id_is_the_documented_format() {
        // 1 704 164 800 is 2024-02-29 00:00:00 UTC — a leap day, because the
        // whole point of a date is that it knows about them.
        assert_eq!(new_id(1_709_164_800, 0), "20240229-000000-0000");
        assert_eq!(new_id(1_234_567_890, 0), "20090213-233130-0000");
        assert_eq!(new_id(0, 0), "19700101-000000-0000");
    }

    /// The date arithmetic, against dates anybody can check.
    #[test]
    fn the_date_is_right_for_a_known_day() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
        assert_eq!(civil(1_234_567_890), (2009, 2, 13, 23, 31, 30));
        assert_eq!(civil(1_709_164_800), (2024, 2, 29, 0, 0, 0));
        assert_eq!(civil(4_102_444_800), (2100, 1, 1, 0, 0, 0));
        // The day before a leap day, and the day after: the two that a
        // month-length arithmetic gets wrong.
        assert_eq!(civil(1_709_164_800 - 86_400).2, 28);
        assert_eq!(civil(1_709_164_800 + 86_400).2, 1);
    }

    /// The ids sort into the order the crashes happened.
    ///
    /// The old id restarted every 1 157 days, so a crash one second before the
    /// restart sorted *after* one a second after it. §58.
    #[test]
    fn the_ids_sort_in_the_order_the_crashes_happened() {
        let mut ids = vec![
            new_id(1_800_000_001, 0), // just after the old restart
            new_id(1_799_999_999, 0), // just before it
            new_id(1_000_000_000, 0),
        ];
        ids.sort_unstable();
        assert_eq!(ids, vec![new_id(1_000_000_000, 0), new_id(1_799_999_999, 0), new_id(1_800_000_001, 0),]);
    }

    /// Two crashes in the same second must not destroy each other.
    ///
    /// A crash report is the only evidence a crash happened, and it was
    /// overwritten by the next crash that landed on the same id. §58.
    #[test]
    fn a_second_crash_does_not_destroy_the_first() {
        let dir = temp_dir("collision");
        let s = Session::sample();

        let mut first = build(&s, "node.add", "the FIRST crash");
        first.id = "20250907-093012-7f3a".to_string();
        let mut second = build(&s, "node.add", "the SECOND crash");
        second.id = first.id.clone();

        let first_dir = write_to(&dir, &first).expect("the first report");
        let second_dir = write_to(&dir, &second).expect("the second report");

        assert_ne!(first_dir, second_dir, "the second crash was filed on top of the first");
        let kept = std::fs::read_to_string(first_dir.join("report.json")).expect("the first report");
        assert!(kept.contains("the FIRST crash"), "the first report was destroyed");
        assert!(list_in(&dir).len() >= 2, "both reports should be there: {:?}", list_in(&dir));
    }

    /// The report's own id names the folder it is in.
    #[test]
    fn the_report_names_the_folder_it_is_in() {
        let dir = temp_dir("naming");
        let s = Session::sample();
        let mut first = build(&s, "node.add", "the first");
        first.id = "20250907-093012-7f3a".to_string();
        let mut second = build(&s, "node.add", "the second");
        second.id = first.id.clone();

        let second_dir = write_to(&dir, &second).expect("the second report");
        let body = std::fs::read_to_string(second_dir.join("report.json")).expect("the report");
        let id: serde_json::Value = serde_json::from_str(&body).expect("json");
        let name = second_dir.file_name().expect("name").to_string_lossy().to_string();
        assert_eq!(id["id"], serde_json::json!(name), "the report and its folder disagree");
    }

    #[test]
    fn a_report_round_trips_through_the_folder() {
        let dir = temp_dir("roundtrip");
        let mut s = Session::sample();
        s.execute("node.add", &json!({ "name": "Extra", "kind": "filter" })).expect("ok");
        let report = build(&s, "prop.set", "deliberate");
        let written = write_to(&dir, &report).expect("written");
        assert!(written.starts_with(&dir));

        let back = show_in(&dir, &report.id).expect("read back");
        assert_eq!(back.command, "prop.set");
        assert_eq!(back.message, "deliberate");
        assert_eq!(back.replay.len(), 1);
        assert_eq!(back.replay[0].command, "node.add");
        assert_eq!(back.replay[0].params["name"], json!("Extra"));

        let steps = replay_in(&dir, &report.id).expect("replay");
        assert_eq!(steps, back.replay);
        assert!(list_in(&dir).contains(&report.id));
    }

    /// A report in a file is the report the folder holds.
    ///
    /// A report is **input**: the recipe asks for `report.json` from whoever
    /// had the crash, so the file arrives from another machine — with no
    /// crash folder around it and no id to look it up by. Every door in the
    /// tree read a report by id, out of a folder, so an arriving report had
    /// **no door at all**. §195.
    ///
    /// The file door is the *same reader* as the folder door, so the two
    /// cannot answer differently about the same report: one reader, one
    /// answer. §185.
    #[test]
    fn a_report_in_a_file_is_the_report_the_folder_holds() {
        let dir = temp_dir("arriving");
        let mut s = Session::sample();
        s.execute("node.add", &json!({ "name": "Extra", "kind": "filter" })).expect("ok");
        let report = build(&s, "prop.set", "deliberate");
        let written = write_to(&dir, &report).expect("written");
        let file = written.join(REPORT_FILE);

        // The file door reads it, with no folder and no id.
        let from_file = read_report(&file).expect("the file door reads the report");
        let from_folder = show_in(&dir, &report.id).expect("the folder door reads the report");
        assert_eq!(from_file.id, from_folder.id);
        assert_eq!(from_file.command, from_folder.command);
        assert_eq!(from_file.replay, from_folder.replay);
        assert_eq!(from_file.design.nodes.len(), from_folder.design.nodes.len());

        // And the replay door agrees with the folder's replay door.
        let replayed = reproduce_file(&file).expect("the file door replays the report");
        let filed = reproduce_in(&dir, &report.id).expect("the folder door replays the report");
        assert_eq!(replayed.steps, filed.steps);
        assert_eq!(replayed.reproduced, filed.reproduced);
        assert!(replayed.reproduced, "the replay did not reproduce the design: {:?}", replayed.differences);
    }

    /// The file door keeps the promise the folder door keeps.
    ///
    /// A report is input, and the reader checks its documents the way a
    /// design file is checked — the design as well as the base, because the
    /// design is the report's **target**, what the replay has to end up
    /// with. §183. A file that arrived from another machine is not a smaller
    /// risk than a folder this machine wrote.
    #[test]
    fn an_arriving_report_whose_design_the_model_refuses_is_refused() {
        let dir = temp_dir("arriving-bad");
        let s = Session::sample();
        let mut report = build(&s, "node.add", "boom");
        report.design.meta.fps = 0;
        let written = write_to(&dir, &report).expect("the report is written");
        let file = written.join(REPORT_FILE);

        let err = read_report(&file).expect_err("a design the model refuses is refused");
        assert!(err.contains("the report's design is not usable"), "{err}");

        // And the replay door says the same thing, rather than replaying onto
        // a design nothing can use and reporting a verdict about it.
        let err = reproduce_file(&file).expect_err("the replay refuses it too");
        assert!(err.contains("the report's design is not usable"), "{err}");
    }

    /// A file that is not a report is refused, and the reason names the file.
    ///
    /// The file door is a door for **untrusted input**: it is given a path by
    /// a person, and the path may point at anything at all. §195.
    #[test]
    fn a_file_that_is_not_a_report_is_refused_by_name() {
        let dir = temp_dir("not-a-report");
        let path = dir.join("something-else.json");
        std::fs::write(&path, "{\"hello\": true}").expect("the file is written");
        let err = read_report(&path).expect_err("not a report");
        assert!(err.contains("is not a readable report"), "{err}");
        assert!(err.contains("something-else.json"), "{err}");
    }

    /// The design is in the report, and there is no second copy of it.
    ///
    /// The folder used to hold `design.json` as well — a copy of the design
    /// that **nothing read**, and the copy the recipe told a person to open.
    /// A copy that nothing reads is not evidence: it can disagree with the
    /// report, and the tree notices neither. §185.
    #[test]
    fn the_design_is_in_the_report_and_nowhere_else() {
        let dir = temp_dir("design");
        let s = Session::sample();
        let report = build(&s, "", "boom");
        let written = write_to(&dir, &report).expect("written");
        let body = std::fs::read_to_string(written.join(REPORT_FILE)).expect("the report");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("json");
        assert_eq!(parsed["design"]["meta"]["name"], json!("Sample plan"));
        // And the folder holds nothing else.
        let mut names: Vec<String> = std::fs::read_dir(&written)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec![REPORT_FILE.to_string()], "the folder holds {names:?}");
    }

    #[test]
    fn the_replay_rebuilds_the_document_that_crashed() {
        // The report carries the base the commands ran against, so the replay
        // starts **there** — not in an empty session. The old form of this
        // test replayed into a fresh session and was right only because the
        // session it used had never loaded a file. §47.
        let dir = temp_dir("replay");
        let mut s = Session::new("replay me");
        s.execute("node.add", &json!({ "name": "A", "kind": "source", "amount": 2 })).expect("ok");
        s.execute("node.add", &json!({ "name": "B", "kind": "filter", "amount": 3 })).expect("ok");
        s.execute("prop.set", &json!({ "node": 2, "path": "amount", "value": 7 })).expect("ok");
        let report = build(&s, "prop.set", "boom");
        write_to(&dir, &report).expect("written");

        let mut fresh = Session::new("replay me");
        fresh.load_document(
            checked_document(report.base.clone(), "the report's base design")
                .expect("the base is a document"),
        );
        for step in &report.replay {
            fresh.execute(&step.command, &step.params).expect("replayed");
        }
        assert_eq!(fresh.document(), s.document());
        assert_eq!(
            fresh.execute("plan.evaluate", &json!({})).expect("plan").data,
            s.execute("plan.evaluate", &json!({})).expect("plan").data
        );
    }

    /// A session that opened a document must replay **onto that document**.
    ///
    /// The fourth defect: the trace was not reset when a document was loaded,
    /// so the replay started from an empty session and quietly lost
    /// everything the file had in it — one node where the report had five,
    /// with nothing said about the difference. §47.
    #[test]
    fn a_session_that_opened_a_document_replays_onto_it() {
        let dir = temp_dir("opened");
        // A document that arrived from outside, with four nodes in it.
        let mut file = Session::sample();
        file.execute("node.add", &json!({ "name": "From the file", "kind": "cue" })).expect("ok");
        let file_doc = file.document().clone();

        let mut original = Session::new("fresh");
        original.load_document(file_doc);
        original.execute("node.add", &json!({ "name": "After the open", "kind": "cue" })).expect("ok");
        assert_eq!(original.document().nodes.len(), 5);

        let report = build(&original, "node.add", "boom");
        write_to(&dir, &report).expect("written");
        let outcome = reproduce_in(&dir, &report.id).expect("reproduced");
        assert!(outcome.reproduced, "the replay lost the loaded document: {:?}", outcome.differences);
        assert_eq!(outcome.session["nodes"].as_array().map(|n| n.len()), Some(5));
    }

    /// A report file larger than the limit is refused.
    ///
    /// A report is input: the recipe asks for `report.json` from whoever had
    /// the crash, so the file arrives from another machine. The design reader
    /// caps its own files; this one did not, and a 104 857 690-byte report was
    /// read in 105 ms without a word. §48.
    #[test]
    fn a_report_larger_than_the_limit_is_refused() {
        let dir = temp_dir("too-big");
        let id = "20250907-000000-0048";
        std::fs::create_dir_all(dir.join(id)).expect("dir");
        // A sparse file: the size is over the limit and the blocks are not
        // written, so the test does not spend 64 MiB of disk to prove a bound.
        for name in ["report.json", "replay.json"] {
            let file = std::fs::File::create(dir.join(id).join(name)).expect("create");
            file.set_len(MAX_REPORT_BYTES as u64 + 1).expect("set_len");
        }

        let err = show_in(&dir, id).expect_err("too big");
        assert!(err.contains("limit"), "{err}");
        let err = replay_in(&dir, id).expect_err("too big");
        assert!(err.contains("limit"), "{err}");
    }

    /// A report with more steps than a replay will run is refused.
    ///
    /// The cost of a replay is quadratic in the number of steps, so this is
    /// checked **before** the first step runs rather than after. §48.
    ///
    /// The bound is the tree's own step bound, shared with the script door:
    /// a report this tree wrote can always be replayed, and a script cannot
    /// ask for more work than a replay may do. §181.
    #[test]
    fn a_replay_with_too_many_steps_is_refused_before_it_runs() {
        let dir = temp_dir("too-many-steps");
        let s = Session::new("t");
        let mut report = build(&s, "node.add", "boom");
        // Read-only steps, so that a *missing* bound would cost seconds here
        // rather than the hours it would cost with steps that build a
        // document — the point of the test is the refusal, not the work.
        report.replay = (0..=crate::MAX_COMMAND_STEPS)
            .map(|_| ReplayStep { command: "plan.evaluate".into(), params: json!({}) })
            .collect();
        write_to(&dir, &report).expect("written");

        let err = reproduce_in(&dir, &report.id).expect_err("too many steps");
        assert!(err.contains("at most"), "{err}");
    }

    /// A crash replay does not put the playhead outside the design.
    ///
    /// §135: the replay seeks with `Session::seek_to` — a road to the playhead
    /// that does not go through the registry, and the registry clamps. A
    /// report records the playhead **at crash time** and the design **at the
    /// start of the trace**, and the two disagree whenever the range changed
    /// during the trace: which is what a report written across a `doc.range`
    /// command holds. The replay then started outside the design it was
    /// replaying, and the plan was evaluated there.
    #[test]
    fn a_crash_replay_does_not_put_the_playhead_outside_the_design() {
        let dir = temp_dir("replay-range");
        let mut s = Session::sample();
        // The design the trace starts from: one second long.
        let mut doc = s.document().clone();
        doc.meta.start = craftkit_core::Tick::from_seconds_f64(0.0);
        doc.meta.end = craftkit_core::Tick::from_seconds_f64(1.0);
        s.load_document(doc);
        let mut report = build(&s, "prop.set", "boom");
        // The playhead where the session was standing when it crashed: five
        // seconds in, outside the design the trace started from.
        report.at = i64::try_from(craftkit_core::Tick::from_seconds_f64(5.0).raw()).expect("the tick fits");
        write_to(&dir, &report).expect("written");

        let outcome = reproduce_in(&dir, &report.id).expect("the replay runs");
        let at = outcome.session["at"].as_i64().expect("the playhead is a number");
        let start = outcome.session["start"].as_i64().expect("the start is a number");
        let end = outcome.session["end"].as_i64().expect("the end is a number");
        assert!(
            at >= start && at <= end,
            "the replay put the playhead at {at}, outside the design's range {start}..{end}"
        );
    }

    /// A report whose base is a corrupt document is refused.
    ///
    /// The design reader refuses a document with two nodes sharing an id —
    /// *"two objects share an id"* — and the crash reader accepted one. A step
    /// naming that id then edited the **first** of the two. §48.
    #[test]
    fn a_report_with_a_corrupt_base_is_refused() {
        let dir = temp_dir("corrupt-base");
        let s = Session::new("t");
        let mut report = build(&s, "node.add", "boom");
        let mut base_doc = Session::sample().document().clone();
        base_doc.nodes[1].id = base_doc.nodes[0].id;
        report.base = base_doc;
        write_to(&dir, &report).expect("written");

        let err = reproduce_in(&dir, &report.id).expect_err("a corrupt base");
        assert!(err.contains("not usable"), "{err}");
    }

    /// A long parameter must survive the trace **exactly**.
    ///
    /// This is the defect in its smallest form. The trace used to be a line of
    /// text, cut off after a couple of hundred characters, so a long name
    /// became invalid JSON — and the replay parsed it, failed, and ran the
    /// command with an *empty* parameter object. The result was a different
    /// document, produced without a word of warning. §47.
    #[test]
    fn the_trace_keeps_the_parameters_exactly() {
        let mut s = Session::new("long");
        let name = "x".repeat(250);
        s.execute("node.add", &json!({ "name": name, "kind": "cue" })).expect("ok");
        let report = build(&s, "node.add", "boom");
        assert_eq!(report.replay.len(), 1);
        assert_eq!(report.replay[0].params["name"], json!(name.as_str()));
        assert_eq!(report.replay[0].params["kind"], json!("cue"));
    }

    /// A report says which document its commands start from.
    #[test]
    fn a_report_records_the_document_its_trace_starts_from() {
        let mut s = Session::new("base");
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        let report = build(&s, "node.add", "boom");
        let base = report.base.clone();
        assert!(base.nodes.is_empty(), "the base is the document before the first command");
        assert_eq!(report.design.nodes.len(), 1);
    }

    /// The replay says whether it reproduced the document.
    #[test]
    fn a_replay_says_whether_it_reproduced_the_document() {
        let dir = temp_dir("reproduce");
        let mut s = Session::new("reproduce me");
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        s.execute("node.add", &json!({ "name": "B", "kind": "sink" })).expect("ok");
        let report = build(&s, "node.add", "boom");
        write_to(&dir, &report).expect("written");

        let outcome = reproduce_in(&dir, &report.id).expect("reproduced");
        assert_eq!(outcome.steps, 2);
        assert!(outcome.failures.is_empty(), "{:?}", outcome.failures);
        assert!(outcome.reproduced, "{:?}", outcome.differences);
        assert!(outcome.differences.is_empty());
    }

    /// A replay that lands somewhere else says so.
    ///
    /// The old behaviour was to print "N step(s) replayed" and stop. A reader
    /// had no way to tell a reproduction from a different document that
    /// happened to be produced by the same commands. §47.
    #[test]
    fn a_replay_that_produces_a_different_document_says_so() {
        let dir = temp_dir("diverge");
        let mut s = Session::new("diverge");
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        let mut report = build(&s, "node.add", "boom");
        // The report now claims a document the recorded commands do not build.
        report.design = craftkit_model::Document::empty("Something else");
        write_to(&dir, &report).expect("written");

        let outcome = reproduce_in(&dir, &report.id).expect("reproduced");
        assert!(!outcome.reproduced, "a different document must not be reported as reproduced");
        assert!(!outcome.differences.is_empty(), "a divergence must be described");
    }

    /// A long session cannot keep every command — and says how many it lost.
    #[test]
    fn a_long_session_reports_how_many_commands_it_dropped() {
        let mut s = Session::new("long");
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        // A read-only command still belongs in the trace: it is part of what
        // the session did, and it is cheap enough to run ten thousand times.
        for _ in 0..10_000 {
            let _ = s.execute("plan.evaluate", &json!({}));
        }
        let report = build(&s, "plan.evaluate", "boom");
        assert!(report.dropped > 0, "10 001 commands do not fit in the trace");
        assert_eq!(report.replay.len() + report.dropped, 10_001);
    }

    /// A corrupt replay file is an error, not an empty step.
    ///
    /// The old parser turned anything it could not read into `{}` — an empty
    /// parameter object — and ran the command with it. A file that cannot be
    /// read has to say so. §47.
    #[test]
    fn a_corrupt_replay_file_is_an_error_and_not_an_empty_step() {
        let dir = temp_dir("corrupt-replay");
        let id = "20250907-000000-0001";
        std::fs::create_dir_all(dir.join(id)).expect("dir");
        std::fs::write(dir.join(id).join(REPORT_FILE), b"{not json").expect("write");
        let err = replay_in(&dir, id).expect_err("corrupt");
        assert!(err.contains(REPORT_FILE), "{err}");
    }

    /// A hand-edited replay must not panic the reproduction.
    ///
    /// Two kinds of hand-editing, two answers, and neither panics.
    ///
    /// A step whose **parameters** are broken is refused by the reader — §202
    /// — so the report is not reproduced at all. A step whose parameters are
    /// fine but whose **command** the tree does not have is reproduced, and
    /// the replay says the step failed. Before §202 the first case was read
    /// and reproduced, and the reader let a report through that the script
    /// door would have refused.
    #[test]
    fn a_hand_edited_replay_does_not_panic_the_reproduction() {
        // The step with broken parameters: the reader refuses the report.
        let broken = temp_dir("hand-edited-broken");
        let mut s = Session::new("edited");
        s.execute("node.add", &json!({ "name": "A", "kind": "cue" })).expect("ok");
        let mut report = build(&s, "node.add", "boom");
        report.replay.push(ReplayStep { command: "not.a.command".into(), params: json!(null) });
        write_to(&broken, &report).expect("written");
        let err = reproduce_in(&broken, &report.id).expect_err("refused");
        assert!(err.contains("`params` that is not an object"), "{err}");

        // And the steps the tree can refuse on its own: the report is read,
        // and the replay says which steps failed.
        let unknown = temp_dir("hand-edited-unknown");
        let mut report = build(&s, "node.add", "boom");
        report.replay.push(ReplayStep { command: "not.a.command".into(), params: json!({}) });
        report.replay.push(ReplayStep { command: "node.add".into(), params: json!({ "oops": true }) });
        write_to(&unknown, &report).expect("written");
        let outcome = reproduce_in(&unknown, &report.id).expect("no panic");
        assert_eq!(outcome.failures.len(), 2, "{:?}", outcome.failures);
    }

    #[test]
    fn an_id_cannot_escape_the_crash_folder() {
        assert_eq!(sanitize("../../etc/passwd"), "etcpasswd");
        assert_eq!(sanitize(""), "unknown");
        assert_eq!(sanitize("20250907-093012-7f3a"), "20250907-093012-7f3a");
        assert_eq!(sanitize("a/../b"), "ab");
        assert_eq!(sanitize("a b\tc"), "abc");
    }

    #[test]
    fn an_escaping_id_writes_inside_the_folder_not_outside_it() {
        let dir = temp_dir("escape");
        let s = Session::sample();
        let report = build(&s, "", "boom");
        let written = write_to(&dir, &CrashReport { id: "../../pwned".into(), ..report }).expect("written");
        assert!(
            written.starts_with(&dir),
            "wrote to {} which is outside {}",
            written.display(),
            dir.display()
        );
    }

    #[test]
    fn reading_an_unknown_report_is_an_error_not_a_panic() {
        let dir = temp_dir("missing");
        let err = show_in(&dir, "no-such-report").unwrap_err();
        assert!(err.contains("there is no crash report"), "{err}");
        assert!(list_in(&dir).is_empty());
        assert!(list_in(Path::new("/nonexistent/craftkit")).is_empty());
    }

    #[test]
    fn a_corrupt_report_is_an_error_that_names_the_file() {
        let dir = temp_dir("corrupt");
        let id = "20250907-000000-0000";
        std::fs::create_dir_all(dir.join(id)).expect("dir");
        std::fs::write(dir.join(id).join("report.json"), b"{not json").expect("write");
        let err = show_in(&dir, id).unwrap_err();
        assert!(err.contains("report.json"), "{err}");
    }

    #[test]
    fn two_reports_in_the_same_second_get_different_ids() {
        let s = Session::sample();
        let a = build(&s, "x", "a");
        let b = build(&s, "x", "b");
        assert_ne!(a.id, b.id, "a second crash must not overwrite the first");
    }

    #[test]
    fn recording_a_crash_never_panics_even_with_an_unwritable_folder() {
        // A *file* where the folder should be: every write below must fail and
        // be reported, because this code runs because something else broke.
        let dir = temp_dir("unwritable");
        let blocked = dir.join("blocked");
        std::fs::write(&blocked, b"not a directory").expect("write blocker");
        let s = Session::sample();
        assert!(record_in(&blocked, &s, "x", "deliberate").is_none());
    }

    /// A crash report is **one file** in a folder of its own.
    ///
    /// The folder used to hold three: the report, a copy of the design and a
    /// copy of the steps. One copy was read by nothing, the other by a door
    /// nothing called, and a report that arrived with the copies disagreeing
    /// had two answers. §185.
    #[test]
    fn recording_a_crash_writes_one_file() {
        let dir = temp_dir("one-file");
        let s = Session::sample();
        let written = record_in(&dir, &s, "x", "deliberate").expect("written");
        assert!(written.join(REPORT_FILE).is_file(), "the report is missing");
        let mut names: Vec<String> = std::fs::read_dir(&written)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, vec![REPORT_FILE.to_string()], "the folder holds {names:?}");
        // No half-written temporary file survives, and no staging folder is
        // left beside the report.
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp") || n.ends_with(".partial"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn an_empty_session_produces_an_empty_replay() {
        let s = Session::new("fresh");
        let report = build(&s, "", "nothing happened");
        assert!(report.replay.is_empty());
    }

    // ---------------------------------------------------------------------
    // The reader
    // ---------------------------------------------------------------------

    /// A report whose design is not a document is refused by the reader.
    ///
    /// A report is **input**: it arrives from another machine. The design used
    /// to be read as a JSON tree, and a tree holds anything — so a file whose
    /// "design" was a list of numbers was a perfectly good report, right up
    /// until something tried to use it. Read as a document, the format itself
    /// says no. §72.
    #[test]
    fn a_report_whose_design_is_not_a_document_is_refused() {
        let dir = temp_dir("not-a-document");
        let id = "20250907-000000-0072";
        std::fs::create_dir_all(dir.join(id)).expect("dir");
        let body = concat!(
            r#"{"id":"20250907-000000-0072","unix":0,"command":"node.add","message":"boom","#,
            r#""design":{"nodes":[1,2,3]},"#,
            r#""base":{"meta":{"name":"b","fps":24,"start":0,"end":120},"nodes":[]},"#,
            r#""replay":[]}"#
        );
        std::fs::write(dir.join(id).join("report.json"), body).expect("write");

        let err = show_in(&dir, id).expect_err("a design that is not a document");
        assert!(err.contains("not a readable report"), "{err}");
    }

    /// A report the old tree wrote still reads.
    ///
    /// The report's design used to be a `serde_json::Value`, and a value's map
    /// is sorted — so the file the old tree wrote had its keys in
    /// alphabetical order rather than the model's own. The **shape** is the
    /// same, and a shape is what a report is: the file format did not change
    /// when the reader's types did, and a report written before §72 has to
    /// still be readable afterwards, or a bug report collected on one version
    /// cannot be read on the next.
    #[test]
    fn a_report_the_old_tree_wrote_still_reads() {
        let dir = temp_dir("old-shape");
        let mut s = Session::new("old");
        s.execute("node.add", &json!({ "name": "Level", "kind": "source" })).expect("ok");
        let report = build(&s, "node.add", "boom");

        // The old shape: the whole report through a value, so every map in it
        // comes out sorted.
        let text = serde_json::to_string_pretty(&report).expect("json");
        let as_value: serde_json::Value = serde_json::from_str(&text).expect("json");
        let old_text = serde_json::to_string_pretty(&as_value).expect("json");
        assert_ne!(old_text, text, "the sorted shape should differ from the model's own order");

        let id = "20250907-000000-0072";
        std::fs::create_dir_all(dir.join(id)).expect("dir");
        std::fs::write(dir.join(id).join("report.json"), old_text).expect("write");

        let back = show_in(&dir, id).expect("an old report still reads");
        assert_eq!(back.design.nodes.len(), 1, "the design did not survive");
        assert_eq!(back.design.nodes[0].name, "Level");
        assert_eq!(back.base.nodes.len(), 0);
    }

    /// How many bytes the process holds, according to the kernel.
    ///
    /// `/proc/self/status`'s `VmHWM` is the **peak**: the most resident pages
    /// this process has had at once. That is what a reader's cost is — the
    /// memory it needed at its worst moment, not the memory it happened to be
    /// holding when the measurement was taken.
    ///
    /// Linux only, and the reason is not laziness: a counting global
    /// allocator would say the same thing on every platform, but this crate
    /// forbids `unsafe`, and `unsafe impl GlobalAlloc` is unsafe, so one
    /// cannot be written here. §66 found the same wall.
    #[cfg(target_os = "linux")]
    fn peak_resident_bytes() -> u64 {
        let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
            return 0;
        };
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmHWM:") {
                let kb = rest.trim().trim_end_matches(" kB").trim().parse::<u64>().unwrap_or(0);
                return kb * 1024;
            }
        }
        0
    }

    /// The name the harness gives the probe, for `--exact`.
    #[cfg(target_os = "linux")]
    const PROBE_TEST: &str = "crash::tests::the_reader_probe_measures_what_the_report_cost";

    /// What the probe prints, and the parent reads.
    #[cfg(target_os = "linux")]
    const PEAK_MARKER: &str = "CRAFTKIT_ENGINE_PEAK=";

    /// What the probe prints about the outcome.
    #[cfg(target_os = "linux")]
    const REFUSED_MARKER: &str = "CRAFTKIT_ENGINE_REFUSED=";

    /// What tells the probe which folder holds the report.
    #[cfg(target_os = "linux")]
    const PROBE_DIR_ENV: &str = "CRAFTKIT_ENGINE_PROBE_DIR";

    /// The id of the hostile report the probe reads.
    #[cfg(target_os = "linux")]
    const PROBE_ID: &str = "20260101-000000-cost";

    /// How many numbers the hostile report holds where the nodes should be.
    ///
    /// Sixteen million numbers is thirty-two megabytes of text. Read as a
    /// tree it is five hundred and forty megabytes of memory; read as a
    /// document it is refused at the first number, and the whole cost is the
    /// text itself.
    #[cfg(target_os = "linux")]
    const PROBE_NUMBERS: usize = 16_000_000;

    /// Write the hostile report: a design that is a long list of numbers.
    #[cfg(target_os = "linux")]
    fn write_hostile_report(dir: &Path) -> usize {
        let folder = dir.join(PROBE_ID);
        std::fs::create_dir_all(&folder).expect("folder");
        let mut body = String::with_capacity(PROBE_NUMBERS * 2 + 256);
        body.push_str(concat!(
            r#"{"id":"20260101-000000-cost","unix":0,"command":"node.add","message":"boom","#,
            r#""design":{"nodes":["#
        ));
        for i in 0..PROBE_NUMBERS {
            if i > 0 {
                body.push(',');
            }
            body.push('1');
        }
        body.push_str(concat!(
            r#"]},"#,
            r#""base":{"meta":{"name":"b","fps":24,"start":0,"end":120},"nodes":[]},"#,
            r#""replay":[]}"#
        ));
        let path = folder.join("report.json");
        std::fs::write(&path, &body).expect("write");
        body.len()
    }

    /// Measure what reading the hostile report cost.
    ///
    /// Started as a *child process* by
    /// [`reading_a_report_costs_no_more_than_its_text_and_one_document`],
    /// because the peak belongs to a process that has done nothing else. Run
    /// on its own, with no folder in the environment, it does nothing.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_reader_probe_measures_what_the_report_cost() {
        let Ok(dir) = std::env::var(PROBE_DIR_ENV) else {
            return;
        };
        let result = show_in(Path::new(&dir), PROBE_ID);
        println!("{PEAK_MARKER}{}", peak_resident_bytes());
        println!("{REFUSED_MARKER}{}", result.is_err());
    }

    /// Reading a report must cost no more than its text and one document.
    ///
    /// This is the defect in its smallest form. The report's design used to be
    /// read as a JSON tree, and a tree costs
    /// [`craftkit_core::JSON_EXPANSION`] — thirty-two — times the text it was
    /// parsed from: a 66 000 101-byte report cost 1 260 292 KiB of resident
    /// memory, and 2.37 s. Read as a document, the same report costs 67 796
    /// KiB and 0.04 s.
    ///
    /// The measurement is of the **peak**, in a child process that has done
    /// nothing else, because that is what a reader's cost is. Linux only:
    /// see [`peak_resident_bytes`].
    #[cfg(target_os = "linux")]
    #[test]
    fn reading_a_report_costs_no_more_than_its_text_and_one_document() {
        let dir = temp_dir("reader-cost");
        let text_bytes = write_hostile_report(&dir);

        let exe = std::env::current_exe().expect("the test binary knows its own path");
        let output = std::process::Command::new(&exe)
            .args(["--exact", PROBE_TEST, "--nocapture", "--test-threads=1"])
            .env(PROBE_DIR_ENV, &dir)
            .output()
            .expect("the child process starts");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let field = |marker: &str| -> Option<String> {
            stdout.lines().find_map(|line| line.split_once(marker)).map(|(_, value)| value.trim().to_string())
        };
        let peak = field(PEAK_MARKER)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_else(|| panic!("the child did not report a measurement. Its output was:\n{stdout}"));
        let refused = field(REFUSED_MARKER).as_deref() == Some("true");
        // The report is thirty-two megabytes of generated text: it is written
        // again by every run, and a folder per process would fill a disk.
        let _ = std::fs::remove_dir_all(&dir);
        assert!(refused, "the reader accepted a design that is not a document");

        let budget = report_memory_budget(text_bytes);
        assert!(
            peak <= budget as u64,
            "reading a {text_bytes}-byte report peaked at {peak} bytes of memory; \
             the budget is {budget} bytes (the text and one document)"
        );
    }
}
