//! One home for writing a file the tree owns.
//!
//! §215. The tree has a rule for this — *"write atomically: into a temporary
//! name, then rename, so a crash halfway through cannot leave a half-written
//! file that looks like a real one"* — and the rule was written **three** times,
//! in three crates, with three different temporary names, while **two** of the
//! tree's own writers did not follow it at all.
//!
//! This is the tree's **one** way to write a file, for the same reason the
//! temporary folder has one home (§212) and the readers have one (§149): a rule
//! written three times is three chances to forget it, and this one was written
//! three times and missed twice.
//!
//! # What the rule is
//!
//! - **A temporary name first.** The bytes go into a file beside the target,
//!   named after the target, the process and a counter — so two writers never
//!   share one name, and a run that reused a process id does not inherit the
//!   last run's file.
//! - **Then a rename.** The finished file is moved onto the target in one
//!   step. A reader sees the old file or the new one, never half of the new
//!   one.
//! - **And the temporary goes away.** If anything fails, the temporary file is
//!   removed, so a failed write leaves the folder as it was.
//!
//! # What it costs, measured
//!
//! A write that fails partway is not a rare thing: a disk that fills, a quota
//! that is reached, a file-size limit. Measured with a 1 KiB file-size limit
//! and a 4 MiB payload, the write dies where it is stopped:
//!
//! ```text
//! direct:  the target held 1024 bytes of the new content where 11 bytes of
//!          the old content had been — the old file was destroyed and a
//!          partial file was left in its place, looking like a real one.
//! through a temporary name: the target still held its 11 old bytes; the
//!          partial content was in the temporary file, which is a file nobody
//!          was reading.
//! ```
//!
//! So the rule is not tidiness: it is the difference between losing the file
//! that was there and keeping it.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// How visible the file this call creates is.
///
/// The tree's convention (§55, §57, and the tenth item of the suggested order):
/// **what this program writes is private**, and a directory the program did not
/// create is not the program's to change.
///
/// The one file that does not follow it is the design the user saves where
/// they choose — *that* one is deliberately left alone, because a document the
/// user put somewhere is the user's business, not the program's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Only its owner may read it: the file is created `0600`, from the moment
    /// it exists — where the platform has POSIX permissions, and with the
    /// process's own default where it does not.
    ///
    /// Not created and restricted afterwards: `std::fs::write` creates its file
    /// with the process's umask, which measured `0664` on this account, so
    /// writing first and restricting second leaves the file readable by
    /// everybody for the length of one `chmod`.
    OwnerOnly,
    /// The process's own default: the file is created with the umask, the way
    /// any file the user creates would be.
    AsCreated,
}

/// The ways a write can fail.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The bytes could not be written, or the finished file could not be moved
    /// into place.
    ///
    /// The path is the file the call was working on: the temporary file while
    /// the bytes are going in, the target while the file is being moved.
    #[error("could not write {path}: {message}")]
    Io { path: PathBuf, message: String },
    /// The path has no file name, so there is no name to write beside.
    #[error("{path} has no file name to write beside")]
    NoName { path: PathBuf },
    /// The file could not be made private, and the bytes were not written into
    /// it.
    ///
    /// A file that another user may read is a file we refuse to write rather
    /// than one we write and then repair.
    #[error("{path} is readable by other users (mode {mode:04o})")]
    NotPrivate { path: PathBuf, mode: u32 },
}

impl WriteError {
    /// The file the call was working on.
    pub fn path(&self) -> &Path {
        match self {
            WriteError::Io { path, .. }
            | WriteError::NoName { path }
            | WriteError::NotPrivate { path, .. } => path,
        }
    }
}

/// Whether a file's mode says that nobody but its owner may read it.
///
/// The check is a **function of the mode**, so it can be asked about a mode
/// without a file: a test can say what a mode means without arranging for a
/// filesystem that hands out that mode.
#[must_use]
pub fn mode_is_private(mode: u32) -> bool {
    mode & 0o077 == 0
}

/// Write `bytes` to `path`, atomically.
///
/// The bytes go into a temporary file beside the target, and the finished file
/// is renamed onto the target. The target is therefore **either the file that
/// was there or the new file, and never half of the new one** — and if the
/// write fails, the target is left exactly as it was.
///
/// The file is created with the visibility the caller asks for: `OwnerOnly`
/// for a file that holds a secret or a design the tree copied, `AsCreated` for
/// a file the user asked to be put somewhere of their choosing.
///
/// A missing parent directory is **not** created: the caller knows whether the
/// place it writes to is one it owns.
pub fn write_file(path: &Path, bytes: &[u8], visibility: Visibility) -> Result<(), WriteError> {
    let temp = temp_path(path)?;
    if let Err(e) = write_temp(&temp, bytes, visibility) {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    if let Err(e) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(WriteError::Io { path: path.to_path_buf(), message: e.to_string() });
    }
    Ok(())
}

/// The name the bytes are written to before they are moved into place.
///
/// Beside the target, so the move is within one filesystem. Named after the
/// target, the process and a counter, so that two writers never share one
/// name — and so that a process id that comes round again does not inherit the
/// last run's file.
fn temp_path(path: &Path) -> Result<PathBuf, WriteError> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let Some(name) = path.file_name() else {
        return Err(WriteError::NoName { path: path.to_path_buf() });
    };
    let name = name.to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!(".{name}.{}.{n}.tmp", std::process::id())))
}

/// Write the bytes into the temporary file.
///
/// The file is **created** with its mode, not created with the umask and
/// restricted afterwards: the bytes are never in a file another user can read,
/// not even for the length of one `chmod`.
fn write_temp(temp: &Path, bytes: &[u8], visibility: Visibility) -> Result<(), WriteError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    set_mode(&mut options, visibility);
    let mut file = options
        .open(temp)
        .map_err(|e| WriteError::Io { path: temp.to_path_buf(), message: e.to_string() })?;
    // Checked **before** the bytes go in, so a file that is not private is a
    // file we refuse to write rather than one we write and then repair.
    check_private(temp, visibility)?;
    file.write_all(bytes).map_err(|e| WriteError::Io { path: temp.to_path_buf(), message: e.to_string() })?;
    // The bytes are in the file's own buffer until it is closed, so the file
    // is flushed before it is moved into place: a rename that publishes a file
    // with half its bytes still in the buffer publishes half a file.
    file.flush().map_err(|e| WriteError::Io { path: temp.to_path_buf(), message: e.to_string() })?;
    Ok(())
}

/// Whether the file that was created is as private as the caller asked for.
///
/// A filesystem that does not keep permissions — a FAT volume, a network
/// share — hands out a mode the caller did not ask for, and then the bytes
/// are in a file anybody can read. So the mode is **looked at**, not assumed.
#[cfg(unix)]
fn check_private(path: &Path, visibility: Visibility) -> Result<(), WriteError> {
    use std::os::unix::fs::PermissionsExt;
    if visibility != Visibility::OwnerOnly {
        return Ok(());
    }
    let mode = std::fs::metadata(path)
        .map_err(|e| WriteError::Io { path: path.to_path_buf(), message: e.to_string() })?
        .permissions()
        .mode()
        & 0o777;
    if mode_is_private(mode) {
        Ok(())
    } else {
        Err(WriteError::NotPrivate { path: path.to_path_buf(), mode })
    }
}

/// The system has no permissions to look at.
#[cfg(not(unix))]
fn check_private(_path: &Path, _visibility: Visibility) -> Result<(), WriteError> {
    Ok(())
}

/// Give the file the mode the caller asked for.
///
/// On a system with POSIX permissions the mode is set at creation, where the
/// umask can only clear bits the mode does not grant: `0600` grants nothing
/// beyond the owner, so no umask can make it readable by anybody else.
///
/// Everywhere else the file is created with the process's own default, which
/// is the best available.
#[cfg(unix)]
fn set_mode(options: &mut std::fs::OpenOptions, visibility: Visibility) {
    use std::os::unix::fs::OpenOptionsExt;
    let mode = match visibility {
        Visibility::OwnerOnly => 0o600,
        Visibility::AsCreated => 0o666,
    };
    options.mode(mode);
}

/// The mode is not a thing this system has.
#[cfg(not(unix))]
fn set_mode(_options: &mut std::fs::OpenOptions, _visibility: Visibility) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temp;

    /// A write leaves no temporary file behind.
    ///
    /// The temporary file is a file in the user's folder; a failed write that
    /// leaves one behind is litter, and the tree's own checks look for `.tmp`
    /// files beside the ones it wrote.
    #[test]
    fn a_write_leaves_no_temporary_behind() {
        let dir = temp::dir("atomic").expect("a folder");
        let path = dir.join("thing.txt");
        write_file(&path, b"hello", Visibility::AsCreated).expect("the write");
        let leftovers: Vec<String> = std::fs::read_dir(&dir)
            .expect("the folder")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
        assert_eq!(std::fs::read(&path).expect("the file"), b"hello");
    }

    /// The file that was there survives a write that fails.
    ///
    /// The failure is real: the child process writes four mebibytes with a
    /// one-kibibyte file-size limit, so the write is stopped where the limit
    /// is reached. The child is killed by the signal the limit raises, so it
    /// cannot clean up after itself — which is the point: the target must not
    /// depend on the writer's good manners.
    ///
    /// This is the whole promise of the home, so it is measured rather than
    /// argued: the child writes straight to the target when the home is
    /// broken, and the target loses its contents.
    #[cfg(unix)]
    #[test]
    fn the_file_that_was_there_survives_a_write_that_fails() {
        const CHILD: &str = "CRAFTKIT_ATOMIC_CHILD";
        let exe = std::env::current_exe().expect("the test binary");

        // The child: the parent named the file, and the child writes into it.
        // The child must write into the **parent's** file, so the path comes
        // through the environment and the child makes no folder of its own.
        if let Ok(path) = std::env::var(CHILD) {
            let path = PathBuf::from(path);
            let marker = path.with_file_name("child-started");
            // It says it has started, so the parent knows the write was
            // reached and not something before it.
            std::fs::write(&marker, b"started").expect("the marker");
            // Then it writes far more than the limit allows.
            let bytes = vec![b'x'; 4 * 1024 * 1024];
            let _ = write_file(&path, &bytes, Visibility::AsCreated);
            return;
        }

        let dir = temp::dir("atomic-fail").expect("a folder");
        let path = dir.join("thing.txt");
        let marker = dir.join("child-started");
        std::fs::write(&path, b"OLD CONTENT").expect("the file that was there");

        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg("ulimit -f 1; exec \"$0\" \"$1\"")
            .arg(exe)
            .arg("atomic::tests::the_file_that_was_there_survives_a_write_that_fails")
            .env(CHILD, &path)
            // The child's own output goes to nowhere: the limit is on the
            // size of a **file**, and the harness's output would go to the
            // file the parent was started with — which may already be past
            // the limit, so the child would be killed before it reached the
            // write, and the test would say nothing.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let status = status.expect("the child runs");
        assert!(!status.success(), "the child should not have finished the write: {status:?}");
        assert!(marker.exists(), "the child never reached the write: {status:?}");

        let after = std::fs::read(&path).expect("the file that was there");
        assert_eq!(after, b"OLD CONTENT", "a write that failed took the file that was there with it");
    }

    /// A file the caller asks to be private is private from the moment it
    /// exists.
    ///
    /// The mode is checked on the file that is left behind, which is the one
    /// that matters: the temporary file is renamed onto the target, and the
    /// rename keeps the mode.
    #[cfg(unix)]
    #[test]
    fn a_private_file_is_private_when_it_appears() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp::dir("atomic-private").expect("a folder");
        let path = dir.join("secret.txt");
        write_file(&path, b"the secret", Visibility::OwnerOnly).expect("the write");
        let mode = std::fs::metadata(&path).expect("the file").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the file is {mode:04o}, not 0600");
    }

    /// A file the caller does not ask to be private is created the way the
    /// process creates any other file.
    ///
    /// The comparison is with a file the process made itself, in the same
    /// folder at the same moment, so the umask is the same for both.
    #[cfg(unix)]
    #[test]
    fn a_file_that_is_not_private_is_created_the_way_the_process_creates_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp::dir("atomic-shared").expect("a folder");
        let path = dir.join("plan.craftkit");
        let probe = dir.join("probe");
        std::fs::write(&probe, b"x").expect("the probe");
        write_file(&path, b"the design", Visibility::AsCreated).expect("the write");
        let mode = |p: &Path| std::fs::metadata(p).expect("the file").permissions().mode() & 0o777;
        assert_eq!(mode(&path), mode(&probe), "the file is not created the way the process creates files");
    }

    /// A mode says whether the file is private, and the question is asked of
    /// the mode.
    ///
    /// The check is a function, so it can be asked about a mode without
    /// arranging for a filesystem that hands out that mode — which is the
    /// whole point of the check.
    #[test]
    fn a_mode_says_whether_the_file_is_private() {
        assert!(mode_is_private(0o600), "0600 is the owner's alone");
        assert!(mode_is_private(0o400), "0400 is the owner's alone");
        assert!(mode_is_private(0o700), "0700 is the owner's alone");
        assert!(!mode_is_private(0o640), "0640 is readable by the group");
        assert!(!mode_is_private(0o604), "0604 is readable by others");
        assert!(!mode_is_private(0o666), "0666 is readable by everybody");
    }
}
