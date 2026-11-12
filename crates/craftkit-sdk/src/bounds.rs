//! The bounds of the kit, published so that the prose can be checked
//! against them.
//!
//! # Why this is here
//!
//! A bound is a promise: *the document may hold ten thousand nodes, and no
//! more*. The number is in the code and the promise is in the README, and
//! the two were kept in step by a checker that **re-parsed the code** —
//! which meant the checker read the code's text, worked the number out
//! from the text, and compared it with the number the prose named. Two
//! parsers, one of them the tree's, and a mistake in either was invisible.
//!
//! So the code **hands the value over** instead. A bound is published here
//! as a value, and the value is the **constant itself**:
//!
//! ```ignore
//! Bound { name: "MAX_NODES", crate_name: "craftkit-sdk", value: MAX_NODES }
//! ```
//!
//! The compiler ties that `value` to the constant: if the constant changes,
//! the published value changes with it, and nothing can be stale. The prose
//! is then checked against **this**, and the check does not read the code
//! at all.
//!
//! # What a bound is not
//!
//! A bound is **not** a contribution in the house sense of one: it has no
//! area, it is not listed under anything, and nobody groups it. It is in
//! the registry because the registry is the tree's way of saying *this is
//! what the program was built with* — and a bound is exactly that: a
//! number the program was built with.
//!
//! # The one rule
//!
//! **Two crates may not publish one name.** The registry refuses a
//! duplicate id, and a bound's name is its id: two crates with a
//! `MAX_DEPTH` and different numbers would leave the prose with no way to
//! say which is which. The refusal happens when the registry is built, and
//! it names both.
//!
//! So a bound that another crate also declares has to be given a name of
//! its own — `MAX_EQUATION_DEPTH` rather than `MAX_DEPTH` — and the prose
//! follows it.

use crate::contribution::{Contribution, Registration};

/// A bound of the kit: a number the kit promises not to exceed.
///
/// The **value** is the number, and the **name** is the constant's own
/// name, so that a reader who looks the name up finds the promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bound {
    /// The bound's name: the constant's own name, as it is written in the
    /// code.
    pub name: &'static str,
    /// The crate that declares it.
    ///
    /// A bound's name is unique in the registry, so this says which crate
    /// a bound came from when a reader is looking at a list of them.
    pub crate_name: &'static str,
    /// The number the bound is.
    pub value: usize,
}

impl Contribution for Bound {
    fn id(&self) -> &'static str {
        self.name
    }

    fn capability(&self) -> &'static str {
        "bound"
    }

    fn layer(&self) -> &'static str {
        "core"
    }

    /// A bound's name is a **constant's** name, not a dotted id.
    ///
    /// The house's rule about ids is about the things a person clicks and
    /// an agent calls; a bound is neither, and `MAX_DOCUMENT_BYTES` is
    /// the name the code and the prose both use.
    fn malformed_id(_id: &str) -> Option<String> {
        None
    }
}

inventory::submit! {
    // The most bytes a document may hold.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_DOCUMENT_BYTES", crate_name: "craftkit-sdk", value: crate::document::MAX_DOCUMENT_BYTES }
    }
}

inventory::submit! {
    // The most bytes the history of a session may hold.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_HISTORY_BYTES", crate_name: "craftkit-sdk", value: crate::history::MAX_HISTORY_BYTES }
    }
}

inventory::submit! {
    // The most bytes the documents a session holds may take.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_SESSION_BYTES", crate_name: "craftkit-sdk", value: crate::session::MAX_SESSION_BYTES }
    }
}

inventory::submit! {
    // The deepest a document's tree may be.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_DEPTH", crate_name: "craftkit-sdk", value: crate::document::MAX_DEPTH }
    }
}

inventory::submit! {
    // The most nodes a document may hold.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_NODES", crate_name: "craftkit-sdk", value: crate::document::MAX_NODES }
    }
}

inventory::submit! {
    // The most relations a document may hold.
    crate::contribution::Registration {
        item: &Bound { name: "MAX_RELATIONS", crate_name: "craftkit-sdk", value: crate::document::MAX_RELATIONS }
    }
}

inventory::collect!(Registration<Bound>);
