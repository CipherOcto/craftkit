//! The launch token.
//!
//! # Why one
//!
//! The channel is bound to loopback, which stops the network. It does not stop
//! the *machine*: on a shared build box, any local process can connect to any
//! port. A 128-bit token that only the launching process knows turns "can reach
//! the port" into "was told the token", which is the difference between a
//! nuisance and a hole.
//!
//! # Where it comes from
//!
//! Either `CRAFTKIT_CONTROL_TOKEN` — which is how a test or a script pins it —
//! or 32 bytes from the operating system, written to a `0600` file — where the
//! platform has POSIX permissions, and with the process's own default where it
//! does not — so the launching user can read it and nobody else can.
//!
//! **"From the operating system" is a claim about every platform**, and it
//! used to be true on only one of them: on Windows and on wasm the bytes came
//! from the **clock** and a stack address, which is a reading rather than a
//! source and can be looked up. §220.
//!
//! # Comparison
//!
//! Constant-time. Not because an attacker on loopback can usefully time a
//! 128-bit comparison, but because the alternative is one `==` that someone
//! eventually copies into a path where it matters.
//!
//! Constant-time is not a licence to be inexact, though, and the two were
//! confused here for a while: the length difference was folded into a `u8`
//! accumulator, so every length difference that was a multiple of 256 was lost
//! and the token followed by 256 NUL bytes was accepted as the token. The
//! accumulator is a `usize` now, and §63 has the measurement.

use std::path::{Path, PathBuf};

/// The bytes in a generated token.
pub const BYTES: usize = 32;

/// A launch token.
#[derive(Clone)]
pub struct Token(String);

impl std::fmt::Debug for Token {
    /// Never printed. A token in a log is a token in a paste.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(<redacted>)")
    }
}

impl Token {
    /// From the environment, if it is set to something usable.
    ///
    /// An empty or absurdly long value is treated as unset rather than accepted:
    /// a launch script that exports an empty variable should fall back to a
    /// generated token, not open the door with nothing.
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var(crate::TOKEN_ENV).ok()?;
        Self::from_text(&raw)
    }

    /// A token from a string, if it is long enough to be worth having.
    pub fn from_text(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if trimmed.len() < 16 || trimmed.len() > 4096 {
            return None;
        }
        Some(Token(trimmed.to_string()))
    }

    /// A fresh token from the operating system's randomness.
    pub fn generate() -> Result<Self, String> {
        let mut bytes = [0u8; BYTES];
        fill_random(&mut bytes)?;
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Token(hex))
    }

    /// The token or a generated one.
    pub fn resolve() -> Result<Self, String> {
        Self::from_env().map_or_else(Self::generate, Ok)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the presented token is this token.
    ///
    /// Constant-time over the longer of the two, so the comparison time does not
    /// leak the prefix that matched.
    ///
    /// **The accumulator is a `usize`, not a `u8`, and the type is the whole
    /// point.** The length difference is a count of bytes and can be far
    /// larger than eight bits; folded into a `u8` it was truncated, and every
    /// difference that was a multiple of 256 was lost. A token followed by 256
    /// NUL bytes was therefore accepted as the token — by the real control
    /// channel, not only by this function. §63.
    ///
    /// A comparison that is careful about *timing* has no business being
    /// careless about *equality*: the constant-time part is what a reader
    /// checks, and the exactness is what the code is for.
    pub fn matches(&self, presented: &str) -> bool {
        let a = self.0.as_bytes();
        let b = presented.as_bytes();
        // `a.len() ^ b.len()` is zero only when the lengths are equal, and the
        // `|=` below can never clear a bit it did not set, so a length
        // difference of any size survives to the end.
        let mut diff = a.len() ^ b.len();
        let n = a.len().max(b.len());
        for i in 0..n {
            // Wrapping so an index past the shorter slice contributes zero
            // rather than panicking — and the length is already folded into
            // `diff` above.
            diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
        }
        diff == 0
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.matches(&other.0)
    }
}

impl Eq for Token {}

/// Fill `bytes` from the operating system.
///
/// On a Unix target this is `/dev/urandom`, which is what every other program
/// on the machine uses.
///
/// Everywhere else it is the standard library's own random keys, and those
/// come from the operating system too: **measured** in the standard library's
/// source, on Windows its `fill_bytes` calls `ProcessPrng` — the system's own
/// generator, documented as always succeeding — and its random keys are
/// filled from it.
///
/// The clock is **not** a source of randomness, and it used to be one here.
/// It is a *reading*, and a reading can be looked up: a secret made of the
/// clock and a stack address has the clock's window as its search space, not
/// the system's generator. §220.
///
/// On `wasm32-unknown-unknown` the standard library has no generator of its
/// own — its `fill_bytes` there panics, and its random keys are the addresses
/// of two allocations — and the browser page does not use this crate anyway,
/// so the secret is whatever the standard library can offer. It says so.
#[cfg(unix)]
fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
    use std::io::Read;
    // Read only what is asked for: `/dev/urandom` is endless, so `fs::read`
    // on it would try to read all of it.
    let mut source = std::fs::File::open("/dev/urandom").map_err(|e| e.to_string())?;
    source.read_exact(bytes).map_err(|e| format!("no entropy source: {e}"))
}

/// Fill `bytes` from the standard library's random keys.
///
/// The keys are the operating system's, and the hasher is keyed with them, so
/// the bytes are the system's own output run through a keyed function. The
/// counter is what makes two calls differ: the keys are per thread, and a
/// second call with the same input would otherwise be the same bytes.
#[cfg(not(unix))]
fn fill_random(bytes: &mut [u8]) -> Result<(), String> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};

    // How many secrets have been made in this process.
    static SECRETS: AtomicU64 = AtomicU64::new(0);

    let mut out = Vec::with_capacity(bytes.len() + 8);
    while out.len() < bytes.len() {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(SECRETS.fetch_add(1, Ordering::Relaxed));
        out.extend_from_slice(&hasher.finish().to_le_bytes());
    }
    bytes.copy_from_slice(&out[..bytes.len()]);
    Ok(())
}

/// Where the token for `port` lives.
///
/// `CRAFTKIT_CONTROL_DIR` overrides it. That is for a **launch script** — a
/// program that starts craftkit with the environment already set — and not
/// for a test: setting an environment variable is `unsafe` in edition 2024,
/// and this crate forbids `unsafe`. A test that wants the token file
/// somewhere it can clean up asks for it through
/// [`crate::ServerOptions::with_token_dir`] instead.
pub fn token_path(port: u16) -> PathBuf {
    if let Some(dir) = std::env::var_os("CRAFTKIT_CONTROL_DIR") {
        return PathBuf::from(dir).join(format!("{port}.token"));
    }
    base_dir().join(crate::TOKEN_DIR).join(format!("{port}.token"))
}

/// Where the token for `port` lives, in a directory of the caller's choosing.
///
/// The same shape as [`token_path`], without the environment.
pub fn token_path_in(dir: &Path, port: u16) -> PathBuf {
    dir.join(format!("{port}.token"))
}

/// The folder the token file lives in.
///
/// The application's own data folder ✓ — **one home** ✓, shared with the
/// engine's crash reports ✓, so an operator finds them all in one place ✓.
/// §283: this function was written here and written again in the engine,
/// and a fork that renamed one of them renamed half the application's data.
fn base_dir() -> PathBuf {
    craftkit_foundation::paths::data_dir()
}

/// Write a token to `path`, readable only by its owner.
///
/// # Two promises, and how each is kept
///
/// **The file is private.** The secret is written into a file that was created
/// private — `0600` from the moment it existed, where the platform has POSIX
/// permissions, and with the process's own default where it does not — and is
/// moved into place afterwards. It is never written into a file that another user could read and
/// is only afterwards locked down: `std::fs::write` creates its file with the
/// process's umask, which measured `0664` on this account, so writing first and
/// restricting second leaves the secret on disk in a world-readable file for as
/// long as the second call takes.
///
/// **The directory is left alone unless we made it.** The tool's own directory
/// is created `0700`. A directory that was *already there* — one the user
/// pointed at — keeps the permissions its owner gave it. Restricting it was
/// how the old code kept the file private; it is also how the old code changed
/// a shared directory from `0755` to `0700` behind the user's back, and
/// dropped a `setgid` bit on the way. §55.
///
/// The file's own mode is what keeps the secret, so the directory's mode does
/// not have to.
pub fn write_token(path: &Path, token: &Token) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        // Only a directory this call creates is this tool's to make private.
        // One that was already there belongs to whoever made it.
        let ours = !parent.exists();
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        if ours {
            restrict(parent, true).map_err(|e| format!("could not protect {}: {e}", parent.display()))?;
        }
    }
    // The tree's **one** writer (§215): into a temporary name beside the
    // token, created private from the moment it exists, and then moved into
    // place.
    craftkit_foundation::atomic::write_file(
        path,
        token.as_str().as_bytes(),
        craftkit_foundation::atomic::Visibility::OwnerOnly,
    )
    .map_err(|e| format!("could not write {}: {e}", path.display()))
}

#[cfg(unix)]
fn restrict(path: &Path, dir: bool) -> Result<(), std::io::Error> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if dir { 0o700 } else { 0o600 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn restrict(_path: &Path, _dir: bool) -> Result<(), std::io::Error> {
    // No POSIX permissions here. The file is created with the process's own
    // default ACL, which is the best available.
    Ok(())
}

/// Read a token back from `path`.
pub fn read_token(path: &Path) -> Result<Token, String> {
    let body =
        std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Token::from_text(&body).ok_or_else(|| format!("{} does not hold a usable token", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_token_is_long_and_differs_each_time() {
        let a = Token::generate().expect("entropy");
        let b = Token::generate().expect("entropy");
        assert_eq!(a.as_str().len(), BYTES * 2);
        assert_ne!(a.as_str(), b.as_str(), "two tokens must not collide");
        assert!(a.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn a_token_never_prints_itself() {
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        let shown = format!("{t:?}");
        assert!(!shown.contains("0123456789abcdef"), "{shown}");
        assert!(shown.contains("redacted"), "{shown}");
    }

    #[test]
    fn a_token_matches_only_itself() {
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        assert!(t.matches("0123456789abcdef0123"));
        assert!(!t.matches("0123456789abcdef0124"));
        assert!(!t.matches("0123456789abcdef012"), "a prefix is not a token");
        assert!(!t.matches(""));
    }

    /// A token with padding on the end is not the token.
    ///
    /// The comparison folded the length difference into a `u8` accumulator, so
    /// every length difference that was a multiple of 256 was lost: the token
    /// followed by 256 NUL bytes was accepted as the token — by the real
    /// control channel, which is where the measurement in §63 was taken.
    #[test]
    fn a_token_with_trailing_nuls_is_not_a_token() {
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        for pad in [1usize, 255, 256, 257, 512] {
            let presented = format!("0123456789abcdef0123{}", "\0".repeat(pad));
            assert!(!t.matches(&presented), "{pad} NUL bytes on the end was accepted as the token");
        }
    }

    /// The same thing in the other direction: the token is the long one.
    ///
    /// A missing byte and a NUL byte are the same value to the loop, so the
    /// length is what has to catch this, and it has to catch it in a unit that
    /// can hold it.
    #[test]
    fn a_token_is_not_accepted_because_its_tail_is_nuls() {
        let t = Token::from_text(&format!("0123456789abcdef0123{}", "\0".repeat(256))).expect("valid");
        assert!(!t.matches("0123456789abcdef0123"), "the shorter half was accepted");
        assert!(!t.matches(""));
    }

    #[test]
    fn a_short_or_enormous_token_from_the_environment_is_refused() {
        assert!(Token::from_text("short").is_none());
        assert!(Token::from_text("").is_none());
        assert!(Token::from_text(&"x".repeat(4097)).is_none());
        assert!(Token::from_text(&"x".repeat(4096)).is_some());
    }

    #[test]
    fn surrounding_whitespace_in_a_token_is_ignored() {
        let t = Token::from_text("  0123456789abcdef0123\n").expect("valid");
        assert_eq!(t.as_str(), "0123456789abcdef0123");
    }

    /// A temporary folder of its own.
    ///
    /// The rule is the tree's, and it lives in one place: §212 gave the
    /// temporary folder a home in `craftkit_foundation::temp`.
    fn temp_dir(tag: &str) -> PathBuf {
        craftkit_foundation::temp::dir(tag).expect("a temporary folder")
    }

    #[test]
    fn a_token_round_trips_through_a_file() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");
        assert_eq!(read_token(&path).expect("read").as_str(), t.as_str());
    }

    #[cfg(unix)]
    #[test]
    fn the_token_file_is_not_world_readable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("perms");
        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");
        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "mode was {mode:o}");
    }

    /// A directory the tool did not create is not the tool's to change.
    ///
    /// The old code restricted the parent of the token file unconditionally, so
    /// pointing `CRAFTKIT_CONTROL_DIR` at a shared directory turned it from
    /// `0755` into `0700` — silently, and with the `setgid` bit dropped on the
    /// way — while the application carried on as though nothing had happened.
    /// §55.
    #[cfg(unix)]
    #[test]
    fn a_directory_that_was_already_there_keeps_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("shared");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");

        let mode = std::fs::metadata(&dir).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "the tool changed a directory it did not create: mode was {mode:o}");
    }

    /// The secret is kept private by the file's own mode, not the directory's.
    ///
    /// So a shared directory can stay shared and the token can still be
    /// unreadable by anyone else.
    #[cfg(unix)]
    #[test]
    fn the_token_file_is_private_in_a_shared_directory() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("shared-file");
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o777)).expect("chmod");

        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");

        let dir_mode = std::fs::metadata(&dir).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o777, "the tool changed a directory it did not create: {dir_mode:o}");
        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "the token file is not private: mode was {mode:o}");
        assert_eq!(read_token(&path).expect("read").as_str(), t.as_str());
    }

    /// The tool's own directory is still made private.
    #[cfg(unix)]
    #[test]
    fn a_directory_the_tool_creates_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("made").join("control");
        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");
        let mode = std::fs::metadata(&dir).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o700, "the tool's own directory is not private: mode was {mode:o}");
    }

    /// The token file is replaced, not appended to.
    ///
    /// The file is written into a temporary name and moved into place, so a
    /// token file left over from a previous launch is replaced rather than
    /// having the new token written after the old one.
    #[test]
    fn a_token_file_left_over_from_a_previous_launch_is_replaced() {
        let dir = temp_dir("replace");
        let path = dir.join("1234.token");
        std::fs::write(&path, "x".repeat(4096)).expect("write");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), t.as_str());
    }

    /// No temporary file is left behind.
    #[test]
    fn writing_a_token_leaves_no_temporary_file_behind() {
        let dir = temp_dir("tidy");
        let path = dir.join("1234.token");
        let t = Token::from_text("0123456789abcdef0123").expect("valid");
        write_token(&path, &t).expect("write");
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n != "1234.token")
            .collect();
        left.sort();
        assert!(left.is_empty(), "left behind: {left:?}");
    }

    #[test]
    fn reading_a_file_that_holds_nothing_useful_is_an_error() {
        let dir = temp_dir("bad");
        let path = dir.join("short.token");
        std::fs::write(&path, "nope").expect("write");
        let err = read_token(&path).unwrap_err();
        assert!(err.contains("usable token"), "{err}");
    }

    #[test]
    fn reading_a_missing_file_names_it() {
        let path = PathBuf::from("/nonexistent/craftkit/9999.token");
        assert!(read_token(&path).unwrap_err().contains("9999.token"));
    }

    /// The token file lives in the application's own data folder.
    ///
    /// §283: this crate built that folder itself and the engine's crash
    /// reporter built it again, so a fork that renamed one of them put the
    /// token in one place and the crash reports in another.
    #[test]
    fn the_token_file_lives_in_the_applications_data_folder() {
        let path = token_path(1234);
        let data = craftkit_foundation::paths::data_dir();
        assert!(path.starts_with(&data), "the token is at {path:?} and the data folder is {data:?}");
        assert_eq!(path.parent().and_then(|p| p.file_name()), Some(std::ffi::OsStr::new("control")));
    }
}
