//! The tree's one walk: the files under a root, and none of the build output.
//!
//! Every gate that has to look at the tree's files needs the same walk, and
//! two walks are two rules about what a gate sees. §169.

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
/// one path should not fail a gate about the whole repository.
pub fn walk(
    root: &Path,
    dir: &Path,
    found: &mut Vec<PathBuf>,
    want: &dyn Fn(&Path) -> bool,
) -> Result<(), String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if IGNORED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            walk(root, &path, found, want)?;
        } else if want(&path)
            && let Ok(relative) = path.strip_prefix(root)
        {
            found.push(relative.to_path_buf());
        }
    }
    Ok(())
}
