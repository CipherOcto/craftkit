//! `craftkit-scratch` — **the scratch a test works in**.
//!
//! # What this is
//!
//! A test that reads a file needs a file, and a file that is written
//! into the system's temporary directory and then left there is litter:
//! a full run of the tree's tests leaves tens of thousands of them
//! behind, and nobody notices until the disk is full.
//!
//! This crate hands out a path and **takes it back**:
//!
//! - [`file`] is a path for a file the caller writes;
//! - [`dir`] is a directory, made and ready to be written into.
//!
//! Both are **unique** — the process id and a counter — so two tests, or
//! two crates, never fight over one path. And both are **removed**: the
//! path is remembered by the thread that asked for it, and when that
//! thread ends the file, or the whole directory, is taken away with it.
//!
//! # A file keeps its own name
//!
//! The scratch is a **directory**, and the file is inside it. That is
//! not tidiness: a program that is being tested often takes its name
//! from the file it was given — a picture is named after its file — and
//! a scratch that changed the name would change what the program does.
//! So the name is the caller's, and only the **directory** around it
//! carries the uniqueness.
//!
//! # Why it is a library and not a plugin
//!
//! Because it is a path and a removal: it contributes no command, no
//! kind and no format, and the linker does not wire it in. It is a
//! **dev** dependency of the crates whose tests use it, and nothing
//! that ships links it.
//!
//! # Why the thread and not the process
//!
//! A test harness runs each test on a thread of its own, and a thread's
//! own storage is dropped when the thread ends. That is the moment the
//! scratch should go: the test has finished, and nothing else is going
//! to read it. A process-wide registry would have to wait for the
//! process to end, and a process that is killed — a test that panics
//! takes its harness with it — would take the litter with it too.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    /// What this thread has asked for, and what it will hand back.
    static SCRATCH: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
}

/// The counter that makes every path unique.
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A path this thread asked for, and the kind of removal it wants.
enum Entry {
    /// A directory: removed with everything in it.
    Directory(PathBuf),
}

impl Drop for Entry {
    /// Take the path back.
    ///
    /// A removal that fails is not an error the test can do anything
    /// about — the test has already finished, and the path may already
    /// be gone — so the failure is let pass.
    fn drop(&mut self) {
        match self {
            Entry::Directory(path) => {
                let _ = std::fs::remove_dir_all(path);
            }
        }
    }
}

/// A path for a **file** the caller writes.
///
/// The file is not made — the caller writes it, with whatever it
/// holds — and the directory it is in is removed when the calling
/// thread ends, which takes the file with it.
///
/// The name is the caller's own: the file is called what the caller
/// called it, and only the directory around it carries the
/// uniqueness, so that a program which takes its name from the file it
/// was given is not misled.
pub fn file(name: &str) -> io::Result<PathBuf> {
    let directory = make("")?;
    Ok(directory.join(safe(name)))
}

/// A **directory**, made and ready to be written into.
///
/// The directory is removed — with everything in it — when the calling
/// thread ends. The directory is unique: two calls are two
/// directories, so two tests never share one.
pub fn dir(name: &str) -> io::Result<PathBuf> {
    make(name)
}

/// Make a directory of this thread's own, and remember it.
fn make(label: &str) -> io::Result<PathBuf> {
    let path = unique(label);
    std::fs::create_dir_all(&path)?;
    remember(Entry::Directory(path.clone()));
    Ok(path)
}

/// Remember a path, so that this thread hands it back when it ends.
fn remember(entry: Entry) {
    SCRATCH.with(|scratch| {
        if let Ok(mut scratch) = scratch.try_borrow_mut() {
            scratch.push(entry);
        }
    });
}

/// A path that no other path is.
///
/// The process id keeps two runs apart, and the counter keeps two
/// calls in one run apart. The name is the last part, so that a person
/// reading the temporary directory can tell what a file was for.
fn unique(label: &str) -> PathBuf {
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let label = safe(label);
    let name = if label.is_empty() {
        format!("craftkit-scratch-{}-{count}", std::process::id())
    } else {
        format!("craftkit-scratch-{}-{count}-{label}", std::process::id())
    };
    std::env::temp_dir().join(name)
}

/// A name that is safe in a file name.
///
/// The name is a **label**: nothing in it may make a directory, and a
/// name that cannot be a path cannot reach out of the scratch.
fn safe(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
        .collect()
}
