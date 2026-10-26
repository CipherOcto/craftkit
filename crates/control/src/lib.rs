//! # craftkit-control
//!
//! The control channel: a token-authenticated, loopback-only, JSON-lines
//! socket that lets something outside the process drive the application.
//!
//! ## Why this exists
//!
//! An agent that can only run a CLI cannot *see* what it did. It can run
//! `craftkit-cli run script.json` and then `snapshot`, but it cannot click a
//! button, take a screenshot of the result, or check what the problems panel
//! says. That gap is why "the agent has a CLI" is not the same as "the
//! application is agent-ready".
//!
//! ## The protocol
//!
//! One JSON object per line, in and out. Never a length prefix, never a
//! framing header — a line is the frame, so a human can attach with `nc` and a
//! transcript is readable.
//!
//! ```text
//! → {"id":1,"token":"…","method":"engine.execute","params":{"command":"node.add","params":{…}}}
//! ← {"id":1,"ok":true,"message":"added filter `Echo`","data":{"id":4}}
//! ← {"id":1,"ok":false,"kind":"bad-parameter","message":"…","command":"node.add"}
//! ```
//!
//! ## The rules
//!
//! 1. **Loopback only.** The listener binds `127.0.0.1`. A control channel that
//!    accepts a remote connection is a remote code execution surface wearing a
//!    friendly name.
//! 2. **A token per launch.** Written to a `0600` file — where the platform
//!    has POSIX permissions, and with the process's own default where it does
//!    not — or supplied in
//!    `CRAFTKIT_CONTROL_TOKEN`. A connection that does not present it is
//!    closed without a reply, so a port scanner learns nothing.
//! 3. **Nothing reaches past the engine.** Every method maps onto something the
//!    engine already does. There is no method that opens a file, spawns a
//!    process, or evaluates arbitrary code.
//! 4. **Every reply carries the `id`.** Including errors, including a request
//!    whose `id` was unparsable — in which case the id is `null` and the caller
//!    can still tell what happened.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod protocol;
pub mod server;
pub mod token;

pub use protocol::{Method, ProtocolError, Request, Response, UiAction};
pub use server::{Server, ServerOptions, dispatch, publish_token, serve, serve_published, serve_with};
pub use token::{Token, token_path, token_path_in, write_token};

/// The environment variable that supplies the token instead of generating one.
pub const TOKEN_ENV: &str = "CRAFTKIT_CONTROL_TOKEN";

/// Where the token for `port` is written when the launch supplied none.
pub const TOKEN_DIR: &str = "control";
