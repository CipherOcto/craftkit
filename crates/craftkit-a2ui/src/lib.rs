//! `craftkit-a2ui` — the a2ui protocol, as a layer above the SDK.
//!
//! # What this crate is
//!
//! The **a2ui compatibility layer**: the protocol's types, the data
//! model, the binding resolution, the box layout, and the catalogue the
//! door serves.
//!
//! # Where it sits
//!
//! **L3** — above the SDK (**L2**) and below the plugins (**L4**). It
//! depends on the SDK and on nothing else, and the plugins depend on it.
//!
//! # What it does **not** do
//!
//! - It does **not** know what craftkit is, beyond the SDK's own types.
//!   The **SDK knows nothing about a2ui**: the compatibility runs the
//!   other way round.
//! - It does **not** write the document. A write into craftkit is a
//!   **command**, and the commands are the plugin's.
//! - It does **not** copy a2ui. The shapes are **re-derived** from a2ui's
//!   public specification and written in craftkit's own words, with
//!   **local** `$ref`s — so the catalogue is **one self-contained file**
//!   and no a2ui file enters the tree.
//!
//! # The modules
//!
//! | the module | what it is |
//! |---|---|
//! | [`model`] | the **data model** — a tree of values, addressed by path |
//! | [`bindings`] | the **bindings** — a2ui's dynamic values, resolved |
//! | [`layout`] | the **box layout** — the arrangement of children |
//! | [`functions`] | the **function table** — what the layer does, and what it expects |
//! | [`defs`] | the **property-name table** — the layer's names |
//! | [`schema`] | the **shapes** — the layer's re-derived schemas |
//! | [`catalog`] | the **a2ui catalogue** — the projection of the entries |
//!
//! # The two artefacts and the three checks
//!
//! The **table** (`defs`) owns the **names**, the **schema text**
//! (`schema`) owns the **shapes**, and the **row** (in the plugin) owns
//! the **resolved** side. And there is a check for each pair, so that no
//! direction is uncovered:
//!
//! | the check | when | what it catches |
//! |---|---|---|
//! | the **row** against the **table** | **compile time** — a `const` assertion | a prop the row declares and the table does not hold, **and** a prop the table holds and the row does not declare |
//! | the **table** against the **schema text** | a test | a prop the table holds and the schema text does not |
//! | the **row** against the **published** schema | a test | a prop the row declares and the published schema does not carry |

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod bindings;
pub mod catalog;
pub mod defs;
pub mod functions;
pub mod layout;
pub mod model;
pub mod schema;

/// The crate's **own** JSON, so that a caller need not depend on
/// `serde_json` to talk to it.
pub use craftkit_sdk::serde_json;
