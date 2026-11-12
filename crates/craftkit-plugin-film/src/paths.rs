//! The **path** of a media file.
//!
//! A file that names another file may name it in three ways, and the
//! tree's own rules say what each one means:
//!
//! - a **file URL** — `file:///a/b`, `file://localhost/a/b`,
//!   `file:///C:/x` — is a path in disguise: the scheme is taken off,
//!   the host `localhost` is dropped, and the rest is decoded;
//! - an **absolute path** is a path already, and it stays as it is;
//! - a **relative path** is relative to the **folder of the file that
//!   names it**, and it is joined to it: `.` is nothing and `..` is
//!   one folder up.
//!
//! # What it does not do
//!
//! - **The file is not opened**: the path is worked out, and nothing
//!   looks for it.

/// Whether a path is **absolute**: it begins at the root, or it is a
/// Windows path with a drive.
pub fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || path.starts_with("\\\\") || is_windows_absolute(path)
}

/// Whether a path is a **Windows absolute** path: a drive and a
/// separator.
fn is_windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\')
}

/// The **bytes** of a percent-encoding, decoded.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] == b'%' && at + 2 < bytes.len() {
            let pair = std::str::from_utf8(&bytes[at + 1..at + 3])
                .ok()
                .and_then(|pair| u8::from_str_radix(pair, 16).ok());
            if let Some(value) = pair {
                out.push(value);
                at += 3;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// A **file URL** as a path. A text that is not a file URL is
/// returned unchanged.
pub fn file_url_to_path(url: &str) -> String {
    let lower = url.get(..7).map(|head| head.to_ascii_lowercase());
    if lower.as_deref() != Some("file://") {
        return url.to_string();
    }
    let rest = &url[7..];
    // The host `localhost` is not a host: it is the machine itself.
    let is_localhost = rest.get(..9).is_some_and(|head| head.eq_ignore_ascii_case("localhost"))
        && rest.get(9..).is_some_and(|after| after.is_empty() || after.starts_with('/'));
    let rest = if is_localhost { &rest[9..] } else { rest };
    let decoded = percent_decode(rest);
    // `file:///C:/x` is the path `C:/x`.
    if decoded.len() >= 3 && decoded.starts_with('/') && is_windows_absolute(&decoded[1..]) {
        return decoded[1..].to_string();
    }
    if !decoded.starts_with('/') {
        // `file://server/share` is a network path.
        return format!("//{decoded}");
    }
    decoded
}

/// A **relative path** joined to a **folder**.
fn join(base: &str, relative: &str) -> String {
    let separator = if base.contains('\\') && !base.contains('/') { '\\' } else { '/' };
    let mut parts: Vec<&str> = base.split(['/', '\\']).collect();
    if parts.last() == Some(&"") && parts.len() > 1 {
        parts.pop();
    }
    for segment in relative.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.len() > 1 {
                    parts.pop();
                }
            }
            other => parts.push(other),
        }
    }
    parts.join(&separator.to_string())
}

/// The **path** a reference names, resolved against a **folder**.
///
/// The tree's own rules: a file URL is a path in disguise, an absolute
/// path stays, and a relative path is joined to the folder.
pub fn resolve_path(reference: &str, base: Option<&str>) -> String {
    let path = file_url_to_path(reference.trim());
    if is_absolute(&path) {
        return path;
    }
    match base {
        Some(base) if !base.is_empty() => join(base, &path),
        _ => path,
    }
}

/// The **folder** a file is in: the path of the file without its own
/// name.
pub fn folder_of(path: &str) -> Option<String> {
    let parent = std::path::Path::new(path).parent()?;
    let folder = parent.to_string_lossy().to_string();
    if folder.is_empty() { None } else { Some(folder) }
}
