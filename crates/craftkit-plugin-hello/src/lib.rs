//! `craftkit-plugin-hello` — a plugin.
//!
//! # What a plugin is
//!
//! A crate that **adds commands** to an application. It depends on the SDK
//! and the macro, and on nothing else — in particular, not on the
//! application. The application gets the commands by *having this crate
//! linked in*, and it does not have to name a single symbol of it.
//!
//! # What this plugin adds
//!
//! - `hello.greet` — says hello to somebody;
//! - `hello.count` — counts the document's nodes.
//!
//! Both are declared the way any command is declared: a struct, an
//! attribute, a doc comment and a `run` method. There is no registration
//! code here, no list to add to, and no call into the application.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::Command;
use craftkit_sdk::{Bool, EngineError, Outcome, Session, Text};

/// Say hello to somebody.
///
/// The name is a parameter, and the parameter is a **field**: the macro reads
/// it, the declaration says it is required, and the body is handed a value
/// rather than a bag of values.
#[derive(Command)]
#[command(id = "hello.greet", area = "hello", icon = "hello")]
pub struct HelloGreet {
    /// Who to greet.
    pub name: Text,
    /// Whether to shout.
    pub loudly: Option<Bool>,
}

impl HelloGreet {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let name = &self.name.0;
        let greeting = match self.loudly {
            Some(Bool(true)) => format!("HELLO, {name}!"),
            _ => format!("Hello, {name}."),
        };
        Ok(Outcome::new(greeting))
    }
}

/// Count the nodes in the document.
///
/// A command with **no parameters**: the struct has no fields, so the
/// declaration has no parameters, and a caller that passes one is refused.
#[derive(Command)]
#[command(id = "hello.count", area = "hello", icon = "count")]
pub struct HelloCount {}

impl HelloCount {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let nodes = session.document().nodes.len();
        Ok(Outcome::new(format!("{nodes} node(s)"))
            .with_data(craftkit_sdk::serde_json::json!({ "nodes": nodes })))
    }
}

// The plugin's anchor.
//
// One name that keeps both commands in the program, on every target —
// including `wasm32-unknown-unknown`, whose linker drops a crate that
// nothing names. See the SDK's `anchor` module.
craftkit_sdk::plugin_anchor!(HelloGreet, HelloCount);

#[cfg(test)]
mod tests {
    use super::*;

    /// The anchor names the plugin's items.
    ///
    /// The anchor is the one name the application gives this crate, and the
    /// address it carries is the relocation that keeps the commands in the
    /// program — on `wasm32-unknown-unknown`, where a crate nothing names is
    /// dropped whole.
    #[test]
    fn the_anchor_names_both_commands() {
        assert_eq!(CRAFTKIT_PLUGIN_ANCHOR.len(), 2);
        let addresses: Vec<usize> = CRAFTKIT_PLUGIN_ANCHOR.iter().map(|anchor| anchor()).collect();
        assert!(
            addresses.contains(&(core::ptr::from_ref(&HELLO_GREET) as usize)),
            "the anchor does not name the greeting command"
        );
        assert!(
            addresses.contains(&(core::ptr::from_ref(&HELLO_COUNT) as usize)),
            "the anchor does not name the counting command"
        );
    }
}
