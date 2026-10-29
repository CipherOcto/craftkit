//! # craftkit-mcp
//!
//! An MCP server over stdio: the same commands, in the shape an agent expects.
//!
//! ## What it is for
//!
//! The control channel is a socket, for scripts and for driving a running
//! window. An MCP server is a subprocess speaking JSON-RPC, for an agent that
//! has a tool-calling loop and wants craftkit's capabilities *as tools*.
//!
//! The important thing is that **it exposes no new behaviour**. Every tool that
//! acts maps onto a [`craftkit_engine`] command, and a tool's schema is
//! generated from the same `ParamDoc` table the CLI help and the docs use. A
//! capability that exists only over MCP is a capability the guard, the undo
//! snapshot and the crash reporter do not know about — which is exactly the
//! failure the whole "everything is a command" rule exists to prevent.
//!
//! Three tools are not commands, and they are about the *server* rather than
//! the document: the running session (`craftkit_status`), the catalogue
//! (`craftkit_commands`), and the crash reports this machine has written
//! (`craftkit_crashes`). They read what the engine already knows and change
//! nothing — and the crash reports are the tree's answer to "my application
//! crashed", which is a question an agent asks more often than any other.
//! §113.
//!
//! ## Transport
//!
//! JSON-RPC 2.0 over stdio, one message per line. Line framing rather than
//! `Content-Length` headers: it is what the current MCP stdio transport uses,
//! and it keeps `nc` usable as a debugging tool.
//!
//! ## Resources
//!
//! The server also serves the protocol's other half: `resources/list`,
//! `resources/read` and `resources/templates/list`. A tool **acts**; a
//! resource **is**. The resources here are the open design and the command
//! catalogue — read by the same calls the `craftkit_status` and
//! `craftkit_commands` tools make, so the two cannot drift apart. §176.
//!
//! ## Prompts
//!
//! The protocol's third part: a prompt **says**. A prompt is a templated
//! message that tells an agent what to do, and the server offers three —
//! add a node, set a property, key a property. Each one names a command and
//! is built from the registry's own words for it, so a prompt cannot
//! describe a command differently from the tree does. §178.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod prompts;
pub mod resources;
pub mod rpc;
pub mod server;
pub mod tools;

pub use prompts::{PROMPTS, PromptError};
pub use resources::{RESOURCES, ReadError as ResourceReadError, TEMPLATES};
pub use rpc::{Error as RpcError, Request as RpcRequest, Response as RpcResponse};
pub use server::{handle_line, serve};
pub use tools::{Tool, all, command_for, tool_for, tool_name, tools_json};

/// Every protocol revision this server can speak, **newest first**.
///
/// **One list.** The handshake answers the client's revision when it is in
/// this list, and the server's own — the first entry — when it is not. There
/// is no second copy of the answer anywhere. §179.
///
/// The list says what the server can *speak*, which is a promise about the
/// whole protocol and not only about the methods craftkit happens to
/// implement:
///
/// | The revision | What it changed | craftkit |
/// |---|---|---|
/// | `2025-06-18` | structured tool output, elicitation, resource links — and **batching removed** | **yes** — the tools already answer with `structuredContent` |
/// | `2025-03-26` | the `completions` capability, batching, tool annotations | **no** — it added batching, and this server does not read a batch |
/// | `2024-11-05` | the first | **yes** |
///
/// The middle revision is left out on purpose. Claiming it would promise a
/// server that reads a JSON-RPC batch, and this one does not: a request that
/// is an array is refused as "not a JSON-RPC object". Saying so is better
/// than saying it and being wrong.
pub const SUPPORTED_VERSIONS: [&str; 2] = ["2025-06-18", "2024-11-05"];

/// The revision this server speaks when the client asks for one it does not
/// have: the newest it supports.
///
/// **The first entry of the list**, so the answer is drawn from the list and
/// cannot drift from it. §179.
pub const PROTOCOL_VERSION: &str = SUPPORTED_VERSIONS[0];

/// The revision to answer the handshake with.
///
/// **The client's, if this server speaks it; the server's own otherwise.**
/// The specification says exactly this: a server that supports the
/// requested revision answers with it, and a server that does not answers
/// with a revision it does support. §179.
pub fn negotiate(requested: Option<&str>) -> &str {
    match requested {
        Some(version) if SUPPORTED_VERSIONS.contains(&version) => version,
        _ => PROTOCOL_VERSION,
    }
}

/// What this server calls itself.
///
/// **The application's own name** ✓ — [`craftkit_foundation::paths::APP_NAME`]
/// ✓ — because the server publishes the application's name ✓, and a fork
/// that renames the application renames the server with it ✓. §284.
///
/// The name appears in what the server publishes: the tool names, the
/// resource URIs, and the descriptions that name a tool. §282: the tool
/// names were **derived** from [`SERVER_NAME`] and the resource URIs were
/// **literals**, so a fork that renamed the server renamed half of what it
/// publishes and left the rest craftkit's.
///
/// So everything is built from the one literal ✓ — the tool names with
/// [`crate::tools::tool_name`] ✓, and the URIs with `concat!` ✓, which
/// needs a literal ✓ and a `const` is not one ✓.
pub const SERVER_NAME: &str = craftkit_foundation::app_name!();
