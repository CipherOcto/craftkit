//! The **icon** — a shape the agent draws.
//!
//! # What an icon is
//!
//! a2ui's `Icon` has three branches: a **string enum** of fifty-nine
//! built-in names, an **object** with an `svgPath`, and a **binding**.
//!
//! The fifty-nine names are **removed** — a declared narrowing, and the
//! reason is that craftkit has **no icon vocabulary**: drawing
//! fifty-nine glyphs is a large hand-written body, and a vocabulary of
//! names that the tree cannot draw is a vocabulary that lies.
//!
//! So an icon is a **path**: the agent sends the shape, and the tree
//! draws it. The path is read by **the SDK's reader**
//! (`craftkit_sdk::svgpath`), which is the same reader the vector plugin
//! uses — one reader, in one place.

use craftkit_sdk::Path;

/// The path of an icon, if the name is one.
///
/// A name that is **not** a path is **refused** — the caller says so,
/// and the icon is not drawn. The tree does not guess and does not fall
/// back to a placeholder shape.
pub fn path_of(name: &str) -> Option<Path> {
    craftkit_sdk::svgpath::read_path(name).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path is read.
    #[test]
    fn a_path_is_read() {
        let path = path_of("M 0 0 L 10 0").expect("the path is read");
        assert_eq!(path.subpaths.len(), 1);
    }

    /// A name that is **not** a path is refused.
    #[test]
    fn a_name_that_is_not_a_path_is_refused() {
        assert!(path_of("check").is_none(), "a built-in name is not a path");
    }
}
