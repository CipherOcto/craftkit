//! The scratch is handed out and handed back: the two ends of it.
//!
//! The end that matters is the **handing back**: a path this thread
//! asked for is gone when the thread ends, and that is what keeps the
//! tree's tests from filling the disk.
//!
//! The other end is the **name**: a file is called what the caller
//! called it, because a program under test often takes its name from
//! the file it was given — a picture is named after its file — and a
//! scratch that changed the name would change what the program does.

use std::path::PathBuf;

/// Run `body` on a thread of its own, and give back what it left.
///
/// A thread's own storage is dropped when the thread ends, and the
/// scratch is remembered there — so the path is gone by the time the
/// join returns.
fn on_its_own_thread<T, F>(body: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(body).join().expect("the thread finished")
}

/// A path is in the temporary directory, and it is the caller's to keep.
#[test]
fn a_file_is_in_the_temporary_directory() {
    let path = craftkit_scratch::file("a-picture.png").expect("the scratch is made");
    assert!(
        path.starts_with(std::env::temp_dir()),
        "the path is not in the temporary directory: {}",
        path.display()
    );
}

/// A file keeps the name the caller gave it.
#[test]
fn a_file_keeps_its_name() {
    let path = craftkit_scratch::file("a-picture.png").expect("the scratch is made");
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    assert_eq!(name, "a-picture.png", "the file was renamed");
}

/// Two calls are two paths: the scratch is never shared.
#[test]
fn two_files_are_two_paths() {
    let one = craftkit_scratch::file("the-same-name").expect("the scratch is made");
    let two = craftkit_scratch::file("the-same-name").expect("the scratch is made");
    assert_ne!(one, two, "two calls handed out the same path");
}

/// The name is a label and not a path: nothing in it can make a
/// directory.
#[test]
fn a_name_cannot_reach_out_of_the_scratch() {
    let path = craftkit_scratch::file("a/name/with/slashes").expect("the scratch is made");
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    assert!(!name.contains('/'), "the name holds a separator: {name}");
    assert!(path.parent().is_some_and(|p| p.starts_with(std::env::temp_dir())));
}

/// A file is gone when the thread that wrote it ends.
#[test]
fn a_file_is_removed_when_the_thread_ends() {
    let path: PathBuf = on_its_own_thread(|| {
        let path = craftkit_scratch::file("written").expect("the scratch is made");
        std::fs::write(&path, b"the bytes").expect("the file is written");
        assert!(path.exists(), "the file is not there");
        path
    });
    assert!(!path.exists(), "the file was left behind: {}", path.display());
}

/// A directory is made, and it is there to be written into.
#[test]
fn a_directory_is_made() {
    let path = craftkit_scratch::dir("a-directory").expect("the scratch is made");
    assert!(path.is_dir(), "the directory is not there: {}", path.display());
}

/// A directory is gone — with everything in it — when the thread ends.
#[test]
fn a_directory_is_removed_when_the_thread_ends() {
    let path: PathBuf = on_its_own_thread(|| {
        let path = craftkit_scratch::dir("filled").expect("the scratch is made");
        let inside = path.join("inside.txt");
        std::fs::write(&inside, b"the bytes").expect("the file is written");
        assert!(inside.exists(), "the file inside is not there");
        path
    });
    assert!(!path.exists(), "the directory was left behind: {}", path.display());
}
