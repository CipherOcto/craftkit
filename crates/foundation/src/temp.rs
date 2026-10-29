//! A temporary folder of its own, for a test that writes files.
//!
//! §212. The rule was written in **sixteen** places at once, and each of them
//! was a chance to get it wrong: a folder named after a tag and a process id
//! is not a name, so two calls with the same tag shared one folder, and a run
//! that reused the process id inherited the last run's files.
//!
//! This is the tree's **one** way to make a temporary folder, for the same
//! reason the readers are in one place (§149): a rule written three times is
//! three chances to forget it, and this one was written sixteen.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::CoreError;

/// A temporary folder of its own, named after the tag.
///
/// Every call gets a folder of its own, so two calls never share one — even
/// when they pass the same tag.
///
/// And a folder that is already there is **emptied** first: the process id in
/// the name is ours, and a process id belongs to one process at a time, so the
/// folder it names belongs to a process that is gone. That is how "leave the
/// evidence behind" and "start from a clean folder" are both true: the
/// folder is left behind when the run ends and emptied when the next run
/// that owns the name begins.
///
/// The folder is **not** removed when the run ends. A failing test leaves
/// what it wrote behind, on purpose, so that a person can look at it.
pub fn dir(tag: &str) -> Result<PathBuf, CoreError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    dir_numbered(tag, n)
}

/// The folder for a given number.
///
/// The number is part of the name, and the folder is emptied first, so that
/// every call gets a folder of its own and no call inherits one. §212.
fn dir_numbered(tag: &str, n: u64) -> Result<PathBuf, CoreError> {
    let name = format!("{tag}-{}-{n}", std::process::id());
    let dir = std::env::temp_dir().join(crate::paths::test_dir_name()).join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|e| CoreError::BadPath(format!("could not clear {}: {e}", dir.display())))?;
    }
    std::fs::create_dir_all(&dir)
        .map_err(|e| CoreError::BadPath(format!("could not make {}: {e}", dir.display())))?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every call gets a folder of its own, and a folder is never inherited.
    ///
    /// §211 found the defect in the engine's crash tests and §212 gave the
    /// rule one home. The claim is the tree's now, not one crate's.
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
