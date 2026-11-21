//! `cargo xtask survey` — the **session's** half of what an arc costs.
//!
//! # The question this answers
//!
//! The tree measures its own half: `cargo xtask census` says how big an arc
//! is, how many files it touches and how long it takes from one arc to the
//! next. The **other** half is the session that did the work — the tool
//! calls, the round trips and the machine time — and that half lives in the
//! **runtime's own record**: every message, every tool call, and the
//! **measured** duration of every command that was tracked.
//!
//! Both surveys so far measured that half by hand, with a script written for
//! the occasion. A measurement that is a script in somebody's `/tmp` is a
//! measurement that is repeated by hand, and the next one is done slightly
//! differently.
//!
//! ```text
//! cargo xtask survey
//! cargo xtask survey --since 2026-11-12
//! cargo xtask survey --database /path/to/runtime-state.sqlite --workspace /path/to/tree
//! ```
//!
//! # The rules
//!
//! - **The durations are measured, not estimated.** The runtime starts every
//!   shell command as a tracked background task and records when it ended; a
//!   command with no tracked duration is **left out** and counted as such.
//! - **The model time is a remainder.** A tool call costs the session the
//!   time until the *next* call; the measured execution is subtracted and
//!   the rest is one bucket — generation, round trip and idle.
//! - **The record is read, never written.** The database is opened
//!   read-only, and the command says so when it cannot.
//! - **A machine without the `sqlite3` command is told so**, and gets no
//!   half-answer.

use std::collections::HashMap;
use std::path::Path;

/// One **tool call** of the session.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    /// When the call was made, in milliseconds since the epoch.
    pub when: i64,
    /// The tool: `bash`, `read`, `edit`, `task`, …
    pub tool: String,
    /// The command, for the calls that run one.
    pub command: String,
    /// The background task the call was tracked as, when it was.
    pub task: Option<String>,
    /// The **measured** duration of the command, in seconds.
    pub seconds: Option<f64>,
}

/// The **session**: the calls, and what they cost.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Session {
    /// Every call, in the order the session made them.
    pub calls: Vec<Call>,
    /// The calls whose duration was measured.
    pub measured: usize,
    /// The commands whose duration was **not** measured.
    pub unmeasured: usize,
}

impl Session {
    /// The **machine time**: the measured durations, added up, in hours.
    pub fn machine_hours(&self) -> f64 {
        self.calls.iter().filter_map(|call| call.seconds).sum::<f64>() / 3600.0
    }

    /// The **median gap** between two calls, in seconds.
    pub fn median_gap(&self) -> f64 {
        let mut times: Vec<i64> = self.calls.iter().map(|call| call.when).collect();
        times.sort_unstable();
        let mut gaps: Vec<f64> = times.windows(2).map(|pair| (pair[1] - pair[0]) as f64 / 1000.0).collect();
        if gaps.is_empty() {
            return 0.0;
        }
        gaps.sort_by(|a, b| a.total_cmp(b));
        let middle = gaps.len() / 2;
        if gaps.len().is_multiple_of(2) { (gaps[middle - 1] + gaps[middle]) / 2.0 } else { gaps[middle] }
    }

    /// The calls that **search** the tree: a shell command that greps.
    pub fn greps(&self) -> usize {
        self.calls.iter().filter(|call| is_a_grep(&call.command)).count()
    }

    /// The calls that ran **the gate**.
    pub fn gates(&self) -> usize {
        self.calls
            .iter()
            .filter(|call| call.command.contains("xtask ci") || call.command.contains("xtask verify"))
            .count()
    }

    /// The calls that ran **the tests**.
    pub fn tests(&self) -> usize {
        self.calls.iter().filter(|call| call.command.contains("cargo test")).count()
    }

    /// The calls that **spawned a sub-agent**.
    pub fn subagents(&self) -> usize {
        self.calls.iter().filter(|call| call.tool == "task").count()
    }
}

/// Whether a command **searches**.
pub fn is_a_grep(command: &str) -> bool {
    command
        .split([' ', ';', '&', '|', '\n'])
        .any(|word| word == "rg" || word == "grep" || word == "ag" || word == "ack" || word == "fd")
}

/// Read the **session** out of the runtime's record.
///
/// The database is read with the `sqlite3` command of the machine, and it
/// is read **only**: the command that runs it says `-readonly`.
pub fn read(database: &Path, workspace: &str, since: Option<i64>) -> Result<Session, String> {
    if !database.is_file() {
        return Err(format!("there is no session record at {}", database.display()));
    }
    let durations = the_durations(database)?;
    let calls = the_calls(database, workspace, since)?;
    let mut session = Session::default();
    for mut call in calls {
        call.seconds = call.task.as_ref().and_then(|task| durations.get(task).copied());
        if call.seconds.is_some() {
            session.measured += 1;
        } else {
            session.unmeasured += 1;
        }
        session.calls.push(call);
    }
    Ok(session)
}

/// The **measured duration** of every tracked command, by its task.
fn the_durations(database: &Path) -> Result<HashMap<String, f64>, String> {
    let query = "select task_id, (ended_at_ms - created_at_ms) / 1000.0 \
                  from local_runtime_background_tasks where ended_at_ms is not null;";
    let out = sqlite(database, query)?;
    let mut durations = HashMap::new();
    for line in out.lines() {
        let Some((task, seconds)) = line.split_once('|') else { continue };
        if let Ok(seconds) = seconds.trim().parse::<f64>() {
            durations.insert(task.trim().to_string(), seconds);
        }
    }
    Ok(durations)
}

/// Every **tool call** of the sessions of a workspace.
fn the_calls(database: &Path, workspace: &str, since: Option<i64>) -> Result<Vec<Call>, String> {
    let query = format!(
        "select json_extract(m.data_json,'$.timestamp'), \
                json_extract(j.value,'$.tool_name'), \
                coalesce(json_extract(json_extract(j.value,'$.tool_call_args'),'$.command'),''), \
                json_extract(json_extract(j.value,'$.tool_call_result_data'),'$.details.task_id') \
         from local_runtime_message_rows m, json_each(m.data_json,'$.tool_calls') j \
         where m.session_id in (select session_id from local_runtime_sessions \
                                where workspace_dir like '%{workspace}%') \
         order by 1;",
        workspace = workspace.replace('\'', "''")
    );
    let out = sqlite(database, &query)?;
    let mut calls = Vec::new();
    for line in out.lines() {
        let mut parts = line.splitn(4, '|');
        let (Some(when), Some(tool), Some(command), Some(task)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(when) = when.trim().parse::<i64>() else { continue };
        if let Some(since) = since
            && when < since
        {
            continue;
        }
        calls.push(Call {
            when,
            tool: tool.trim().to_string(),
            command: command.trim().to_string(),
            task: if task.trim().is_empty() { None } else { Some(task.trim().to_string()) },
            seconds: None,
        });
    }
    Ok(calls)
}

/// Run the `sqlite3` command of the machine.
fn sqlite(database: &Path, query: &str) -> Result<String, String> {
    let output = crate::command(
        "sqlite3",
        &["-readonly", "-separator", "|", &database.to_string_lossy(), query],
        Path::new("."),
    )
    .map_err(|e| format!("the session record could not be read: {e}"))?;
    if !output.ok() {
        return Err(format!("the session record could not be read:\n{}", output.all()));
    }
    Ok(output.stdout)
}

/// The **report** of a session, beside the tree's own numbers.
pub fn report(session: &Session, arcs: usize, days: f64) -> String {
    let mut out = String::new();
    let calls = session.calls.len();
    let per_arc = if arcs == 0 { 0.0 } else { calls as f64 / arcs as f64 };
    out.push_str("   the session\n");
    out.push_str(&format!("     the calls            {calls}\n"));
    out.push_str(&format!("     an arc makes         {per_arc:.1} calls\n"));
    out.push_str(&format!("     the machine time     {:.1} h\n", session.machine_hours()));
    out.push_str(&format!(
        "     the measured share   {} of {} calls\n",
        session.measured,
        session.measured + session.unmeasured
    ));
    out.push_str(&format!("     the median gap       {:.1} s\n", session.median_gap()));
    out.push_str(&format!(
        "     the model and trip   {:.0} h (the calls times the median gap)\n",
        calls as f64 * session.median_gap() / 3600.0
    ));
    out.push_str(&format!("     the greps            {}\n", session.greps()));
    out.push_str(&format!("     the gates            {}\n", session.gates()));
    out.push_str(&format!("     the test runs        {}\n", session.tests()));
    out.push_str(&format!("     the sub-agents       {}\n", session.subagents()));
    out.push_str("   the tree\n");
    out.push_str(&format!("     the arcs             {arcs}\n"));
    if days > 0.0 {
        out.push_str(&format!("     the days             {days:.1}\n"));
        out.push_str(&format!("     an arc a day         {:.1}\n", arcs as f64 / days));
    }
    if arcs > 0 {
        out.push_str(&format!(
            "     the machine an arc   {:.1} min\n",
            session.machine_hours() * 60.0 / arcs as f64
        ));
    }
    out
}

/// Run the command.
pub fn run(root: &Path, args: &[String]) -> Result<crate::Outcome, String> {
    let mut database = default_database();
    let mut workspace = root.to_string_lossy().to_string();
    let mut since: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let mut value = || -> Result<String, String> {
            let value = args.get(index + 1).cloned();
            index += 2;
            value.ok_or_else(|| format!("`{flag}` needs a value"))
        };
        match flag {
            "--database" => database = std::path::PathBuf::from(value()?),
            "--workspace" => workspace = value()?,
            "--since" => since = Some(value()?),
            other => return Err(format!("`{other}` is not a flag of `survey`")),
        }
    }
    let since_at = match &since {
        Some(day) => Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| format!("the clock is wrong: {e}"))?
                .as_millis() as i64
                - days_ago_ms(day)?,
        ),
        None => None,
    };
    let session = read(&database, &workspace, since_at)?;
    let census = crate::census::census(root)?;
    // **The feature arcs** — the commits that carry the code. The census
    // counts every commit that names a section, and the documentation half
    // of each arc is one of those.
    let arcs = census.arcs.iter().filter(|arc| arc.subject.starts_with("feat(")).count();
    // The window the arcs of the census cover: the first of them to the
    // last, in days. Git counts in seconds.
    let days = match (census.arcs.first(), census.arcs.last()) {
        (Some(first), Some(last)) => last.when.saturating_sub(first.when) as f64 / 86_400.0,
        _ => 0.0,
    };
    println!("\n── survey");
    println!("{}", report(&session, arcs, days));
    Ok(crate::Outcome::Ran)
}

/// The **record** of the runtime, when the machine has one.
fn default_database() -> std::path::PathBuf {
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from).unwrap_or_default();
    home.join(".minimax/v2/sqlite/runtime-state.sqlite")
}

/// How many milliseconds ago a day was.
fn days_ago_ms(day: &str) -> Result<i64, String> {
    let parts: Vec<&str> = day.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("`{day}` is not a day: it wants `YYYY-MM-DD`"));
    }
    let year: i64 = parts[0].parse().map_err(|_| format!("`{day}` is not a day"))?;
    let month: i64 = parts[1].parse().map_err(|_| format!("`{day}` is not a day"))?;
    let day: i64 = parts[2].parse().map_err(|_| format!("`{day}` is not a day"))?;
    // The days from the epoch, by the civil calendar of the machine.
    let mut days = 0i64;
    for each in 1970..year {
        days += if is_a_leap_year(each) { 366 } else { 365 };
    }
    for each in 1..month {
        days += days_in_month(year, each);
    }
    days += day - 1;
    Ok(days * 24 * 60 * 60 * 1000)
}

/// Whether a year is a leap year.
fn is_a_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days a month has.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_a_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A call, for the tests.
    fn call(tool: &str, command: &str, when: i64) -> Call {
        Call { when, tool: tool.to_string(), command: command.to_string(), task: None, seconds: None }
    }

    #[test]
    fn a_grep_is_a_search_and_a_build_is_not() {
        assert!(is_a_grep("cd /x && rg --files"));
        assert!(is_a_grep("grep -rn foo crates/"));
        assert!(!is_a_grep("cargo test -p craftkit-xlsx"));
        assert!(!is_a_grep("git log --oneline"));
    }

    #[test]
    fn the_machine_time_is_the_measured_durations() {
        let mut session = Session::default();
        let mut first = call("bash", "cargo test", 0);
        first.seconds = Some(10.0);
        let mut second = call("bash", "cargo build", 1_000);
        second.seconds = Some(20.0);
        session.calls.push(first);
        session.calls.push(second);
        assert!((session.machine_hours() - 30.0 / 3600.0).abs() < 1e-9);
    }

    #[test]
    fn the_median_gap_is_the_middle_one() {
        let mut session = Session::default();
        for index in 0..5 {
            session.calls.push(call("read", "", index * 10_000));
        }
        assert_eq!(session.median_gap(), 10.0);
    }

    #[test]
    fn the_kinds_of_call_are_counted() {
        let mut session = Session::default();
        session.calls.push(call("bash", "rg foo", 0));
        session.calls.push(call("bash", "cargo xtask ci", 1));
        session.calls.push(call("bash", "cargo test -p x", 2));
        session.calls.push(call("task", "", 3));
        assert_eq!(session.greps(), 1);
        assert_eq!(session.gates(), 1);
        assert_eq!(session.tests(), 1);
        assert_eq!(session.subagents(), 1);
    }

    #[test]
    fn a_day_is_a_number_of_milliseconds() {
        // 1970-01-02 is one day after the epoch.
        assert_eq!(days_ago_ms("1970-01-02").expect("the day"), 86_400_000);
        // 2024 is a leap year, and February has 29 days in it.
        assert_eq!(
            days_ago_ms("2024-03-01").expect("the day"),
            days_ago_ms("2024-02-29").expect("the day") + 86_400_000
        );
    }

    #[test]
    fn a_day_that_is_not_a_day_is_refused() {
        let error = days_ago_ms("the eleventh").expect_err("it is refused");
        assert!(error.contains("not a day"), "{error}");
    }
}
