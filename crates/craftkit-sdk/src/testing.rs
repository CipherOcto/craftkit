//! The shape of a test of a command, written by a macro.
//!
//! # Why this is here
//!
//! Thirty-seven per cent of every arc's lines are tests, and much of that is
//! the same four lines over and over: build a document, execute a command,
//! assert the outcome, name the failure. The shape is not the interesting
//! part of a test — **the rule is** — so the shape is written once, here, and
//! an arc writes the assertion that is specific to its rule.
//!
//! # The shape
//!
//! ```ignore
//! assert_command!(
//!     id = "grid.workbook.add",
//!     params = { "name": "Book" },
//!     |session| { /* set the document up; the session is yours to change */ },
//!     |outcome| { /* assert what the command did */ },
//! );
//! ```
//!
//! The first closure runs **before** the command and holds the session, so
//! that the document can be set up. The second runs **after** it and holds
//! the command's [`Outcome`].
//!
//! Two things the macro does not do:
//!
//! - it does **not** have a second way to run a command. The command runs
//!   through [`Session::execute`], which is the one way a command runs;
//! - it does **not** swallow a refusal. A command that is refused fails the
//!   test with the **refusal's own words**, because the refusal is what the
//!   reader of the test needs to see.
//!
//! The macro leaves the session behind, so a test can look at the document
//! the command left:
//!
//! ```ignore
//! let session = assert_command!(
//!     id = "grid.workbook.add",
//!     params = { "name": "Book" },
//!     |session| { … },
//!     |outcome| { assert!(outcome.changed); },
//! );
//! assert_eq!(session.document().nodes.len(), 1);
//! ```
//!
//! # The setup closure, and the one way
//!
//! The session hands out the document only to a command — that is the tree's
//! rule, and a test does not get an exception to it. So a test that needs a
//! document sets it up by **running a command**: the setup closure runs
//! another command through the same session, and the command under test sees
//! what that one left.

use crate::command::CommandSpec;
use crate::contribution::{Registry, RegistryError};
use crate::document::NodeKindSpec;
use crate::session::Session;

pub use crate::assert_command;

/// A session over every contribution the binary was linked with.
///
/// The registry is built from the **link**, so a test that declares its
/// commands — with the derive, or by submitting them to the inventory by
/// hand — finds them in this session, and a test that links no command finds
/// none.
///
/// The error is the registry's own: two contributions with one id is a
/// refusal, and this says so rather than picking one of them.
pub fn session() -> Result<Session, RegistryError> {
    let commands = Registry::<CommandSpec>::from_inventory()?;
    let kinds = Registry::<NodeKindSpec>::from_inventory()?;
    Ok(Session::new(commands, kinds))
}

/// Run the setup closure, with the session it is to change.
///
/// The closure's parameter is typed **here**, so that the reader of a test
/// does not have to write the type down: `|session| { … }` is enough.
pub fn setup<R>(setup: impl FnOnce(&mut Session) -> R, session: &mut Session) -> R {
    setup(session)
}

/// Run the assertion closure, with the outcome it is to say something about.
///
/// As with [`setup`], the parameter is typed here.
pub fn assert_outcome<R>(
    assertion: impl FnOnce(&crate::error::Outcome) -> R,
    outcome: &crate::error::Outcome,
) -> R {
    assertion(outcome)
}

/// Assert what a command does.
///
/// The macro writes the shape of a test of a command: it builds the session,
/// runs the setup, executes the command through [`Session::execute`], and
/// hands the outcome to the assertion. An arc writes the assertion that is
/// specific to its rule.
///
/// ```ignore
/// assert_command!(
///     id = "grid.workbook.add",
///     params = { "name": "Book" },
///     |session| { /* set the document up */ },
///     |outcome| { /* assert what the command did */ },
/// );
/// ```
///
/// The command is refused rather than swallowed: a refusal fails the test
/// with the refusal's own words.
///
/// The value of the macro is the **session**, so that a test can look at the
/// document the command left.
#[macro_export]
macro_rules! assert_command {
    (
        id = $id:literal,
        params = $params:tt,
        $setup:expr,
        $assert:expr $(,)?
    ) => {{
        let mut session = $crate::testing::session()
            .unwrap_or_else(|error| panic!("the registry could not be built: {error}"));
        let _ = $crate::testing::setup($setup, &mut session);
        let outcome = session
            .execute($id, &$crate::serde_json::json!($params))
            .unwrap_or_else(|error| panic!("the command `{}` was refused: {error}", $id));
        let _ = $crate::testing::assert_outcome($assert, &outcome);
        session
    }};
}
