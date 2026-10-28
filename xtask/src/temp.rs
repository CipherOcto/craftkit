//! A temporary folder of its own, for a test that writes files.
//!
//! §213. This is the **core crate's** `temp` module, restated for the tree
//! that cannot see the core.
//!
//! The xtask depends on **nothing of the tree's own** — a checker that cannot
//! be built when the tree does not compile is a checker that is silent exactly
//! when it is needed — so it cannot call `craftkit_core::temp::dir`. The rule
//! is the same rule, and it is written here **once** instead of in
//! **eighty-three** places.
//!
//! **Measured, what it cost to leave the copies.** The xtask built a
//! temporary folder in eighty-four places, and seventy-five of them did not
//! empty the folder they found — so a run that reused a process id inherited
//! the last run's files. The gate `the_walk_has_one_home` counts the
//! recursive walks in the tree's own source, and a leftover file in a
//! fabricated tree is one more walk:
//!
//! ```text
//! "xtask/src/leftover.rs:2: `a_leftover_walk` walks the tree, ..."
//!   left: 2
//!  right: 1
//! ```
//!
//! The failure is **sticky**: the folder is removed at the end of the test,
//! and a test that fails never gets there.
//!
//! **A copy that forgets where it came from is a copy that drifts**, so the
//! record is in this file and not only in the history. The core's home is
//! `crates/foundation/src/temp.rs`, and it is the same shape: a **counter** in the
//! name, so every call gets a folder of its own, and an existing folder
//! **emptied** first, so no call inherits one.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// A temporary folder of its own, named after the tag.
///
/// Every call gets a folder of its own, so two calls never share one — even
/// when they pass the same tag.
///
/// And a folder that is already there is **emptied** first: the process id in
/// the name is ours, and a process id belongs to one process at a time, so the
/// folder it names belongs to a process that is gone. That is how "leave the
/// evidence behind" and "start from a clean folder" are both true: the folder
/// is left behind when the run ends and emptied when the next run that owns
/// the name begins.
///
/// The folder is **not** removed when the run ends. A failing test leaves
/// what it wrote behind, on purpose, so that a person can look at it.
pub fn dir(tag: &str) -> Result<PathBuf, String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    dir_numbered(tag, n)
}

/// The folder for a given number.
///
/// The number is part of the name, and the folder is emptied first, so that
/// every call gets a folder of its own and no call inherits one. §213.
fn dir_numbered(tag: &str, n: u64) -> Result<PathBuf, String> {
    let name = format!("{tag}-{}-{n}", std::process::id());
    let dir = std::env::temp_dir().join("craftkit-xtask-tests").join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("could not clear {}: {e}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not make {}: {e}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every call gets a folder of its own, and a folder is never inherited.
    ///
    /// The core's home has this test, and so does this one: the two homes are
    /// two copies of a rule, and a copy is proved by its own test rather than
    /// by a comparison with the original. §213.
    #[test]
    fn every_call_gets_a_folder_of_its_own() {
        let one = dir("same-tag").expect("a folder");
        let two = dir("same-tag").expect("a folder");
        assert_ne!(one, two, "two calls must not share a folder");

        // And a folder that is already there is fresh, not the last run's.
        // The number is named here, so the test does not depend on how many
        // other tests ran first.
        let known = dir_numbered("inherit", 0).expect("a folder");
        std::fs::write(known.join("left-behind.txt"), b"old").expect("write");
        let again = dir_numbered("inherit", 0).expect("a folder");
        assert!(
            !again.join("left-behind.txt").exists(),
            "a folder that is already there must not inherit the files in it"
        );
    }
}
