//! The tree's one walk: the files under a root, and none of the build output.
//!
//! Every gate that has to look at the tree's files needs the same walk, and
//! two walks are two rules about what a gate sees.
//!
//! # Where this came from
//!
//! This file is a **copy that was folded back together**, and it says so
//! because a copy that forgets where it came from is a copy that drifts.
//!
//! The assets gate had the walk **twice** — `walk` and `collect`, near
//! identical — and the lists gate had it **three** times: `collect_manifests`,
//! `collect_markdown` and `collect_rust`. §168 folded the assets gate's two
//! into this one and wrote that *"there is now one, in `xtask/src/tree.rs`,
//! and both gates use it"*. **That was false when it was written**: the same
//! commit left the lists gate's three behind.
//!
//! And they had already drifted, which is the whole argument for one walk:
//!
//! | The walk | Ignores |
//! |---|---|
//! | this one | `target`, `.git`, `node_modules`, `dist`, and every dot-directory |
//! | `collect_manifests` | `target`, `node_modules`, and every dot-directory — **not `dist`** |
//! | `collect_markdown`, `collect_rust` | `target`, `node_modules`, `dist`, and every dot-directory |
//!
//! A gate that walks the tree and a gate that walks the tree differently do
//! not see the same tree, and the difference is a file nobody looked at. §170
//! folded the three in and added the gate that keeps it that way.

use std::path::{Path, PathBuf};

/// Paths never inspected: build output and version control.
pub const IGNORED_DIRS: &[&str] = &["target", ".git", "node_modules", "dist"];

/// Every file under `root` that `want` accepts, as paths relative to `root`.
///
/// Hand-written rather than done with a glob crate: it is twenty lines, and it
/// lets the tests run on a temporary directory with no filesystem assumptions
/// beyond "a directory is a directory".
///
/// A directory that cannot be read is skipped, not fatal: a permission error on
/// one path should not fail a gate about the whole repository. So this cannot
/// fail, and it does not pretend that it can.
pub fn walk(root: &Path, dir: &Path, found: &mut Vec<PathBuf>, want: &dyn Fn(&Path) -> bool) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if IGNORED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            walk(root, &path, found, want);
        } else if want(&path)
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_path_buf());
        }
    }
}
