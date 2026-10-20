//! Undo and redo, by snapshot.
//!
//! # Why snapshots and not commands
//!
//! The usual alternative is to record "the inverse of what I just did", which
//! is cheaper and is what most editors do. It is also the thing that goes
//! wrong: an inverse that is not exactly the inverse is how undo stacks start
//! losing work, and the bug only shows up after a crash, which is the worst
//! possible moment to find it.
//!
//! A snapshot cannot be subtly wrong. The cost is a clone per mutating command
//! on a document of at most `MAX_NODES` nodes — bounded, predictable, and
//! profiled rather than guessed. If it ever shows up, the fix is to raise
//! [`MAX_ENTRIES`] and add a size cap, not to start inverting commands by hand.
//!
//! **That last paragraph was wrong, and §52 is why.** A count of entries is
//! not a bound on memory: the document's own size was never bounded in bytes,
//! so 128 snapshots of a 12 MiB document cost 1.5 GiB — and undoing them did
//! not give the memory back, because the snapshots moved to the redo stack.
//! The count is still here (a session can run for days), but the *budget* is
//! what bounds the memory now.
//!
//! **And a budget is only a budget if it counts what is really held.** §68:
//! the budget counts the memory the undo and redo stacks hold *together*, and
//! `push` throws the redo stack away whenever a new edit arrives — the undone
//! branch is unreachable. The throwaway did not give its memory back to the
//! budget, so every edit after an undo added a phantom, and the budget
//! trimmed the undo stack against it: on a full-size document, undo stopped
//! working after twenty-one edit/undo cycles, while the budget claimed 66 MB
//! for stacks holding nothing at all.

//! **And a budget has to be enforced on every path that holds memory.** §84:
//! `push` checked the budget and the undo path did not. An undo moves the
//! *current* document into the redo stack, and the current document can be
//! larger than the snapshot it replaced — so the two stacks could hold more
//! than the budget allows. Measured: 67 936 429 bytes against a 67 108 864
//! byte budget. The undo path trims now, oldest step first, which is the
//! same rule `push` follows.
use craftkit_model::Document;

/// How many undo steps are kept.
///
/// Bounded because a session can run for days. This is a bound on **steps**;
/// [`MAX_HISTORY_BYTES`] is the bound on the memory they cost.
pub const MAX_ENTRIES: usize = 128;

/// How much memory the undo and redo stacks together may hold.
///
/// The budget is in bytes, because bytes are what the memory is. When the
/// document is small this is never reached — 128 small snapshots are far
/// below it — and when the document is large the stack keeps fewer steps
/// rather than more memory. §52.
pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;

/// One step.
#[derive(Debug, Clone)]
struct Entry {
    label: String,
    doc: Document,
    /// How much memory the document in this step holds.
    ///
    /// Measured when the step was pushed, so dropping the oldest step is a
    /// subtraction and not a walk of the whole document.
    bytes: usize,
}

/// The undo/redo pair.
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    /// The memory both stacks hold together.
    bytes: usize,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// How many steps are kept.
    pub fn len(&self) -> usize {
        self.undo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.undo.is_empty()
    }

    /// How much memory the undo and redo stacks hold together.
    ///
    /// Reported so that a caller can see what the history costs, and so that
    /// a test can see that the budget is honoured.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// The label of the step undo would revert, for a button tooltip.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }

    /// The label of the step redo would reapply.
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }

    /// Record the state *before* a command ran.
    ///
    /// Pushing clears the redo stack, because a new edit after an undo means
    /// the undone branch is no longer reachable. That is what every editor
    /// does and what every person expects.
    pub fn push(&mut self, doc: Document, label: &str) {
        self.clear_redo();
        if self.undo.len() >= MAX_ENTRIES {
            // Drop the oldest, not the newest: recent steps are the ones a
            // person is likely to want.
            self.drop_oldest_undo();
        }
        let bytes = doc.estimated_bytes();
        // The budget is a budget on memory, so the oldest steps go until this
        // one fits. A document bigger than the whole budget is not kept at
        // all: there is no version of "keep it and use more memory" that is
        // still a budget.
        while self.bytes + bytes > MAX_HISTORY_BYTES {
            if self.undo.is_empty() {
                break;
            }
            self.drop_oldest_undo();
        }
        if self.bytes + bytes > MAX_HISTORY_BYTES {
            return;
        }
        self.bytes += bytes;
        self.undo.push(Entry { label: label.to_string(), doc, bytes });
    }

    /// Take the state to restore, moving it to the redo stack.
    pub fn pop_undo(&mut self, current: Document) -> Option<Document> {
        let entry = self.undo.pop()?;
        self.bytes -= entry.bytes;
        let bytes = current.estimated_bytes();
        self.bytes += bytes;
        self.redo.push(Entry { label: entry.label.clone(), doc: current, bytes });
        // Moving a document can make the two stacks hold more than they did:
        // the current document is bigger than the snapshot it replaced, and
        // the budget is a budget. The oldest undo steps give way — the recent
        // ones are the ones a person is likely to want. §84.
        self.trim_to_budget();
        Some(entry.doc)
    }

    /// Take the state to restore, moving it to the undo stack.
    pub fn pop_redo(&mut self, current: Document) -> Option<Document> {
        let entry = self.redo.pop()?;
        self.bytes -= entry.bytes;
        let bytes = current.estimated_bytes();
        self.bytes += bytes;
        self.undo.push(Entry { label: entry.label.clone(), doc: current, bytes });
        // The same, the other way round. §84.
        self.trim_to_budget();
        Some(entry.doc)
    }

    /// Forget everything. Loading a document does this.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.bytes = 0;
    }

    /// A compact history for the debug panel: labels only, newest last.
    pub fn labels(&self) -> Vec<String> {
        self.undo.iter().map(|e| e.label.clone()).collect()
    }

    /// Drop the oldest undo step and give its memory back.
    fn drop_oldest_undo(&mut self) {
        if self.undo.is_empty() {
            return;
        }
        let entry = self.undo.remove(0);
        self.bytes = self.bytes.saturating_sub(entry.bytes);
    }

    /// Drop the oldest redo step and give its memory back.
    fn drop_oldest_redo(&mut self) {
        if self.redo.is_empty() {
            return;
        }
        let entry = self.redo.remove(0);
        self.bytes = self.bytes.saturating_sub(entry.bytes);
    }

    /// Drop the oldest steps until the two stacks fit the budget.
    ///
    /// The undo steps go first: the redo entry that was just pushed is the
    /// newest state there is, and the oldest undo step is the one a person is
    /// least likely to want — which is the same rule `push` follows.
    ///
    /// §84: `push` enforced the budget and the undo path did not, so the two
    /// stacks could hold more than the budget allows. The measurement: an undo
    /// that moved an 8 MiB document into the redo stack left the history
    /// holding 67 936 429 bytes against a 67 108 864-byte budget.
    fn trim_to_budget(&mut self) {
        while self.bytes > MAX_HISTORY_BYTES {
            if !self.undo.is_empty() {
                self.drop_oldest_undo();
            } else if !self.redo.is_empty() {
                self.drop_oldest_redo();
            } else {
                break;
            }
        }
    }

    /// Forget the redo branch, and give its memory back.
    ///
    /// The budget counts the memory the two stacks hold *together*, so a stack
    /// that is emptied has to be subtracted from it — the same way
    /// [`drop_oldest_undo`] does.
    ///
    /// It used to be `self.redo.clear()`, which freed the memory and left the
    /// budget counting it. The budget then trimmed the undo stack against a
    /// phantom: on a full-size document, undo stopped working after twenty-one
    /// edit/undo cycles, while the budget claimed 66 MB for stacks holding
    /// nothing at all. §68.
    fn clear_redo(&mut self) {
        for entry in self.redo.drain(..) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_model::{Document, NodeKind};

    fn doc_with(n: usize) -> Document {
        let mut d = Document::empty("t");
        for _ in 0..n {
            let _ = d.add_node("n", NodeKind::Source);
        }
        d
    }

    #[test]
    fn an_empty_history_can_do_nothing() {
        let mut h = History::new();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert!(h.pop_undo(doc_with(0)).is_none());
        assert!(h.pop_redo(doc_with(0)).is_none());
    }

    #[test]
    fn undo_returns_the_previous_state_and_offers_a_redo() {
        let mut h = History::new();
        h.push(doc_with(1), "Add node");
        let restored = h.pop_undo(doc_with(2)).expect("a step");
        assert_eq!(restored.nodes.len(), 1);
        assert!(h.can_redo());
        assert_eq!(h.redo_label(), Some("Add node"));
    }

    #[test]
    fn redo_returns_the_newer_state_and_offers_an_undo() {
        let mut h = History::new();
        h.push(doc_with(1), "Add node");
        let _ = h.pop_undo(doc_with(2)).expect("undo");
        let redone = h.pop_redo(doc_with(1)).expect("redo");
        assert_eq!(redone.nodes.len(), 2);
        assert!(h.can_undo());
    }

    #[test]
    fn a_new_edit_after_an_undo_discards_the_redo_branch() {
        let mut h = History::new();
        h.push(doc_with(1), "one");
        let _ = h.pop_undo(doc_with(2));
        assert!(h.can_redo());
        h.push(doc_with(3), "two");
        assert!(!h.can_redo(), "the abandoned branch must not stay reachable");
    }

    #[test]
    fn the_stack_is_bounded_and_drops_the_oldest() {
        let mut h = History::new();
        for i in 0..MAX_ENTRIES + 10 {
            h.push(doc_with(i % 3), &format!("step {i}"));
        }
        assert_eq!(h.len(), MAX_ENTRIES);
        assert_eq!(h.undo_label(), Some(&*format!("step {}", MAX_ENTRIES + 9)));
        // The oldest labels are gone; the newest are still there.
        assert_eq!(h.labels().first().map(String::as_str), Some("step 10"));
    }

    #[test]
    fn clearing_empties_both_directions() {
        let mut h = History::new();
        h.push(doc_with(1), "one");
        let _ = h.pop_undo(doc_with(2));
        h.clear();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert!(h.is_empty());
        assert_eq!(h.bytes(), 0, "clearing must give the memory back");
    }

    /// The history keeps its memory within the budget.
    ///
    /// §52: 128 snapshots of a 12 MiB document cost 1.5 GiB, and the count of
    /// steps was never a bound on memory. The budget is what bounds it now.
    #[test]
    fn the_history_keeps_its_memory_within_a_budget() {
        let mut h = History::new();
        let doc = doc_of_megabytes(2);
        for i in 0..MAX_ENTRIES + 10 {
            h.push(doc.clone(), &format!("step {i}"));
        }
        assert!(
            h.bytes() <= MAX_HISTORY_BYTES,
            "the history held {} bytes, and the budget is {MAX_HISTORY_BYTES}",
            h.bytes()
        );
        assert!(
            h.len() < MAX_ENTRIES,
            "the budget should have bound this before the count did: {} steps",
            h.len()
        );
        // The newest steps are the ones that are kept.
        assert_eq!(h.undo_label(), Some(&*format!("step {}", MAX_ENTRIES + 9)));
    }

    /// Undoing gives the memory back — or rather, moves it, which is the
    /// other half of the bug: the redo stack holds the same snapshots, so
    /// "undo everything" never freed anything. §52.
    #[test]
    fn undoing_does_not_grow_the_history() {
        let mut h = History::new();
        let doc = doc_of_megabytes(2);
        for i in 0..40 {
            h.push(doc.clone(), &format!("step {i}"));
        }
        let before = h.bytes();
        for _ in 0..40 {
            let _ = h.pop_undo(doc.clone());
        }
        assert_eq!(h.bytes(), before, "undoing every step should not change what the history holds");
        assert!(h.bytes() <= MAX_HISTORY_BYTES, "the history held {} bytes", h.bytes());
    }

    /// The budget counts what the stacks hold, not what they used to hold.
    ///
    /// §68: `push` clears the redo stack when a new edit arrives — the undone
    /// branch is unreachable — and the budget kept counting the memory it had
    /// just freed. Every edit after an undo added a phantom, and the budget
    /// trimmed the undo stack against it.
    #[test]
    fn the_budget_counts_the_memory_the_stacks_hold() {
        let mut h = History::new();
        let doc = doc_with(10);
        // What a *stored copy* costs. A clone has no spare capacity, so it
        // measures a little less than the document it was copied from — the
        // budget counts the copies, and this is what a copy costs.
        let one = doc.clone().estimated_bytes();
        for _ in 0..20 {
            h.push(doc.clone(), "edit");
            let _ = h.pop_undo(doc.clone());
            h.push(doc.clone(), "edit");
        }
        assert_eq!(
            h.bytes(),
            h.len() * one,
            "the budget counts memory that was freed: {} bytes for {} steps of {one}",
            h.bytes(),
            h.len()
        );
    }

    /// The discarded branch gives its memory back the moment it is discarded.
    ///
    /// The smallest form of §68: undo a step, edit again, and the budget must
    /// not still be counting the branch that the edit made unreachable.
    #[test]
    fn a_discarded_redo_branch_gives_its_memory_back() {
        let mut h = History::new();
        let doc = doc_with(10);
        h.push(doc.clone(), "one");
        let held = h.bytes();
        let _ = h.pop_undo(doc.clone());
        assert_eq!(h.bytes(), held, "undoing moves the memory, it does not change it");
        h.push(doc.clone(), "two");
        assert!(!h.can_redo(), "the undone branch must not stay reachable");
        assert_eq!(h.bytes(), held, "the budget still counts the branch that was just thrown away");
    }

    /// Undo still works after many edits that throw a redo branch away.
    ///
    /// The symptom of §68: the budget counted every redo branch it had
    /// discarded, so on a big enough document it trimmed the undo stack
    /// against a phantom — and then refused to record anything at all, and
    /// undo stopped working.
    #[test]
    fn undo_still_works_after_many_edits_that_discard_a_redo_branch() {
        let mut h = History::new();
        let doc = doc_of_megabytes(4);
        for _ in 0..25 {
            h.push(doc.clone(), "edit");
            let _ = h.pop_undo(doc.clone());
            h.push(doc.clone(), "edit");
        }
        assert!(h.can_undo(), "undo stopped working: {} step(s) left", h.len());
        assert!(h.len() > 1, "the undo stack was trimmed to {} step(s)", h.len());
    }

    /// Undoing does not take the history over its budget.
    ///
    /// §84: `push` enforces the budget and the undo path did not. An undo
    /// moves the **current** document into the redo stack, and the current
    /// document can be larger than the snapshot it replaced — so the two
    /// stacks could hold more than the budget allows, and the budget's own
    /// report said so.
    #[test]
    fn undoing_does_not_take_the_history_over_its_budget() {
        let mut h = History::new();
        let doc = doc_of_megabytes(2);
        for i in 0..MAX_ENTRIES {
            h.push(doc.clone(), &format!("step {i}"));
        }
        let before = h.bytes();
        assert!(
            before > MAX_HISTORY_BYTES / 2,
            "the stack should be near the budget before the undo: {before} bytes"
        );
        // The current document is bigger than the snapshot it replaces.
        let big = doc_of_megabytes(8);
        let _ = h.pop_undo(big);
        assert!(
            h.bytes() <= MAX_HISTORY_BYTES,
            "the history held {} bytes, and the budget is {MAX_HISTORY_BYTES}",
            h.bytes()
        );
    }

    /// A document of roughly `mebibytes` megabytes, built the cheap way.
    fn doc_of_megabytes(mebibytes: usize) -> Document {
        let mut doc = Document::empty("t");
        let id = doc.add_node("n", craftkit_model::NodeKind::Source).expect("node");
        let mut group = craftkit_model::PropGroup::new("props", "Properties");
        // The model measures a keyframe at about 88 bytes, and there are four
        // properties' worth of them below.
        let keys = (mebibytes * 1024 * 1024) / 88 / 4;
        for p in 0..4u64 {
            let mut prop =
                craftkit_model::Property::new(p + 1, format!("p{p}"), craftkit_core::Value::Num(1.0));
            for t in 0..keys {
                let _ = prop.keys.set(
                    craftkit_model::Key::new(
                        craftkit_core::Tick::from_raw(t as i128),
                        craftkit_core::Value::Num(t as f64),
                        craftkit_model::Easing::Linear,
                    )
                    .expect("key"),
                );
            }
            group.push(prop);
        }
        doc.get_mut(id).expect("just added").props = group;
        doc
    }
}
