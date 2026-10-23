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

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

pub mod resources;
pub mod rpc;
pub mod server;
pub mod tools;

pub use resources::{RESOURCES, ReadError as ResourceReadError, TEMPLATES};
pub use rpc::{Error as RpcError, Request as RpcRequest, Response as RpcResponse};
pub use server::{handle_line, serve};
pub use tools::{Tool, all, command_for, tool_for, tool_name, tools_json};

/// The protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// What this server calls itself.
pub const SERVER_NAME: &str = "craftkit";
