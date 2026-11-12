//! The anchor: the one name an application gives a plugin crate.
//!
//! # Why this exists
//!
//! A plugin registers itself by putting a `#[used]` static in its **own**
//! object code, and the linker decides whether that object code is in the
//! program. On most targets the linker keeps it. On
//! `wasm32-unknown-unknown` it does not, and it does not say so: the program
//! starts, the registry is empty, and nothing is wrong anywhere.
//!
//! It was measured, in this tree, on `wasm32-unknown-unknown`:
//!
//! | what the application names | commands in the registry |
//! |---|---|
//! | nothing (`use plugin as _;` only) | **0** |
//! | one item of the plugin | 3 |
//!
//! So an application names **one** thing per plugin crate — the plugin's
//! **anchor** — and the anchor names the plugin's items. The name is a
//! relocation, and a relocation is something a linker cannot drop.
//!
//! # What an anchor is
//!
//! A function pointer per item. The pointer is to a closure that returns the
//! item's address, so the anchor's own initializer carries a relocation to
//! every item it covers, and the items' objects are extracted with it.
//!
//! It is a function and not a raw pointer because a raw pointer is not
//! `Sync`, and a static has to be.
//!
//! # Who writes it
//!
//! The derive macro writes the [`Anchored`] implementation, and the plugin
//! writes the anchor itself, once, at the root of its crate:
//!
//! ```ignore
//! craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);
//! ```
//!
//! The list is the plugin's own items, and the compiler checks it: a type
//! that does not implement [`Anchored`] is a compile error, not a silently
//! dropped plugin.

/// The type of a plugin's anchor.
///
/// The application writes this type in the file it generates, so it is
/// spelled out here once and used by both sides.
pub type PluginAnchor = &'static [fn() -> usize];

/// An item that a plugin's anchor can name.
///
/// The derive macro implements this for every item it writes, so a plugin
/// lists its own types and the compiler checks the list.
pub trait Anchored {
    /// A function that returns the address of the item.
    ///
    /// The address is what makes the item's object code stay in the program:
    /// a reference to it is a relocation, and the linker has to resolve it.
    const ANCHOR: fn() -> usize;
}

/// Write a plugin's anchor.
///
/// Called once, at the root of a plugin crate, with the plugin's own types:
///
/// ```ignore
/// craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);
/// ```
///
/// The macro writes one public static, `CRAFTKIT_PLUGIN_ANCHOR`, and the
/// application names it. A type that does not implement [`Anchored`] — one
/// the derive macro did not write — is a compile error.
#[macro_export]
macro_rules! plugin_anchor {
    ($($ty:ty),* $(,)?) => {
        /// The plugin's anchor: the one name that keeps its items linked in.
        #[doc(hidden)]
        pub static CRAFTKIT_PLUGIN_ANCHOR: $crate::PluginAnchor = &[
            $(<$ty as $crate::Anchored>::ANCHOR),*
        ];
    };
}
