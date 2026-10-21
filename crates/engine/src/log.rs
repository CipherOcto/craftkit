//! The command log, and the text helper the crash path needs.

/// How many log entries are kept.
///
/// Bounded, because a long session can run for days and the log exists to
/// explain the last few minutes, not to be a file.
pub const MAX_ENTRIES: usize = 256;

/// One line of the log.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LogEntry {
    pub seq: u64,
    pub command: String,
    pub ok: bool,
    pub message: String,
}

/// A bounded ring of what has been run.
#[derive(Debug)]
pub struct LogBook {
    entries: Vec<LogEntry>,
    seq: u64,
    /// How many commands have failed since the session opened.
    ///
    /// Counted here rather than derived from `entries`, because the entries are
    /// a bounded ring of the last [`MAX_ENTRIES`] commands: a count taken over
    /// the ring is a count of the last 256 commands, and it was reported as
    /// "since the session opened". §69.
    failures: usize,
}

impl Default for LogBook {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBook {
    pub fn new() -> Self {
        Self { entries: Vec::new(), seq: 0, failures: 0 }
    }

    pub fn record(&mut self, command: &str, ok: bool, message: &str) {
        self.seq = self.seq.saturating_add(1);
        if !ok {
            self.failures = self.failures.saturating_add(1);
        }
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(LogEntry {
            seq: self.seq,
            command: command.to_string(),
            ok,
            message: truncate(message, 400),
        });
    }

    /// The most recent command's id, if anything has run.
    ///
    /// For the crash hook: "what was the user doing when this happened?" is
    /// answerable from the log even when the panic happened outside a command
    /// entirely, which is exactly the case a command-based trace cannot cover.
    pub fn last_command(&self) -> Option<&str> {
        self.entries().iter().rev().map(|e| e.command.as_str()).next()
    }

    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// The last `n` entries, newest last.
    pub fn tail(&self, n: usize) -> Vec<LogEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].to_vec()
    }

    /// Forget everything, and start numbering from one again.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.seq = 0;
        self.failures = 0;
    }

    /// How many commands have failed since the session opened.
    ///
    /// Every failure, not only the ones still in the ring: the entries are
    /// bounded and this count is not. §69.
    pub fn failures(&self) -> usize {
        self.failures
    }

    /// How many commands have run since the session opened.
    ///
    /// Every command, not only the ones still in the ring. §69.
    pub fn total(&self) -> u64 {
        self.seq
    }
}

/// A readable message from a panic payload of any shape.
///
/// `panic_any(42)` is legal, so a formatter that only downcasts to `&str` is a
/// crash waiting to happen inside the crash handler.
pub fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        truncate(s, 400)
    } else if let Some(s) = payload.downcast_ref::<String>() {
        truncate(s, 400)
    } else {
        "no message".to_string()
    }
}

/// Truncate on a character boundary, never mid-code-point.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_numbered_from_one_and_grow() {
        let mut b = LogBook::new();
        b.record("a", true, "ok");
        b.record("b", false, "no");
        let e = b.entries();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].seq, 1);
        assert_eq!(e[1].seq, 2);
        assert_eq!(b.failures(), 1);
    }

    #[test]
    fn the_book_is_bounded() {
        let mut b = LogBook::new();
        for _ in 0..MAX_ENTRIES + 5 {
            b.record("a", true, "ok");
        }
        assert_eq!(b.entries().len(), MAX_ENTRIES);
    }

    /// The failures that fall out of the ring are still failures.
    ///
    /// §69: the entries are a bounded ring of the last [`MAX_ENTRIES`]
    /// commands, and the failure count was taken over the ring — while the
    /// doc said *"since the session opened"*.
    #[test]
    fn the_failure_count_is_every_failure_and_not_only_the_ones_in_the_ring() {
        let mut b = LogBook::new();
        for _ in 0..MAX_ENTRIES + 44 {
            b.record("a", false, "no");
        }
        assert_eq!(b.entries().len(), MAX_ENTRIES, "the ring is bounded");
        assert_eq!(
            b.failures(),
            MAX_ENTRIES + 44,
            "the failures that fell out of the ring are still failures"
        );
    }

    /// And the same for the commands themselves.
    #[test]
    fn the_total_is_every_command_and_not_only_the_ones_in_the_ring() {
        let mut b = LogBook::new();
        for _ in 0..MAX_ENTRIES + 44 {
            b.record("a", true, "ok");
        }
        assert_eq!(b.entries().len(), MAX_ENTRIES, "the ring is bounded");
        assert_eq!(b.total(), (MAX_ENTRIES + 44) as u64, "the total is not bounded");
    }

    /// Clearing forgets the failures too — the count is "since the session
    /// opened", and clearing is the session opening again.
    #[test]
    fn clearing_forgets_the_failures_too() {
        let mut b = LogBook::new();
        b.record("a", false, "no");
        b.record("a", true, "ok");
        assert_eq!(b.failures(), 1);
        b.clear();
        assert_eq!(b.failures(), 0);
        assert_eq!(b.total(), 0);
    }

    #[test]
    fn the_tail_returns_the_most_recent_entries() {
        let mut b = LogBook::new();
        for i in 0..10 {
            b.record(&format!("c{i}"), true, "ok");
        }
        let t = b.tail(3);
        assert_eq!(t.len(), 3);
        assert_eq!(t[2].command, "c9");
    }

    #[test]
    fn a_long_message_is_truncated_on_a_character_boundary() {
        let msg = "é".repeat(500);
        let got = truncate(&msg, 10);
        assert_eq!(got.chars().count(), 11, "10 characters plus the ellipsis");
        assert!(got.ends_with('…'));
    }

    #[test]
    fn a_short_message_is_left_alone() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn every_panic_payload_shape_yields_a_usable_detail() {
        fn boxed(v: impl std::any::Any + Send + 'static) -> Box<dyn std::any::Any + Send> {
            Box::new(v)
        }
        assert_eq!(panic_detail(&boxed("boom")), "boom");
        assert_eq!(panic_detail(&boxed(String::from("boom"))), "boom");
        assert_eq!(panic_detail(&boxed(42u32)), "no message");
        assert_eq!(panic_detail(&boxed(())), "no message");
    }
}
