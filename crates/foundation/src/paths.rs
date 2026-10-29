//! Where the application's own data lives.
//!
//! **One home for the name and one home for the folder.** §283: the
//! control channel's token file and the engine's crash reports each built
//! the *same* folder themselves — the same three branches, the same name,
//! the same code — in two crates that never met. So a fork that renamed
//! the folder in one of them put the token in one place and the crash
//! reports in another, and the tree's rule that the app's data lives in
//! one place was a rule twice over.
//!
//! The shape is the platform's: `XDG_DATA_HOME` ✓, the home directory's
//! `.local/share` ✓, and the temporary directory as the last resort ✓ —
//! somewhere writable is better than giving up ✓.

use std::path::PathBuf;

/// The application's own name.
///
/// **The one home of that name** in the tree's own code ✓ — the folder its
/// data lives in ✓, the folder its tests write to ✓ — so a fork renames
/// the application in one place ✓.
pub const APP_NAME: &str = "craftkit";

/// The folder the application's data lives in.
///
/// The same folder for everything the application writes outside the
/// user's own folders ✓ — the control channel's token ✓, the engine's
/// crash reports ✓ — so an operator finds them all in one place ✓.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME")
        && !dir.trim().is_empty()
    {
        return PathBuf::from(dir).join(APP_NAME);
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        return PathBuf::from(home).join(".local").join("share").join(APP_NAME);
    }
    // A container with no home and no XDG: somewhere writable is better than
    // giving up, and the caller is told the path in the log either way.
    std::env::temp_dir().join(APP_NAME)
}

/// The folder the application's **tests** write to.
///
/// The name's folder with `-tests` on the end ✓, so a test's leftovers are
/// never confused with the application's own data ✓.
pub fn test_dir_name() -> String {
    format!("{APP_NAME}-tests")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The name is the name, and the folder carries it.
    ///
    /// The defect §283 found was two crates building the same folder with
    /// two different spellings of the application's name.
    #[test]
    fn the_data_folder_carries_the_applications_name() {
        assert_eq!(APP_NAME, "craftkit");
        let dir = data_dir();
        let last = dir.file_name().and_then(|n| n.to_str()).expect("a folder name");
        assert_eq!(last, APP_NAME, "the folder is not the application's: {dir:?}");
    }

    /// The test folder is the application's folder with `-tests` on the end.
    ///
    /// So a test's leftovers are never mistaken for the application's data.
    #[test]
    fn the_test_folder_is_not_the_applications_folder() {
        let tests = test_dir_name();
        assert!(tests.starts_with(APP_NAME), "{tests}");
        assert_ne!(tests, APP_NAME, "the test folder is the application's folder");
    }
}
