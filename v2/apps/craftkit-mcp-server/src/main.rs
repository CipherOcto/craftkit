//! `craftkit-mcp-server` — the MCP server, over stdio.
//!
//! It is the same program in every other way: the studio's ecosystem, all
//! six batteries linked in, and one session. The only difference is that
//! this one talks **MCP** on its standard output instead of drawing a
//! window.
//!
//! ```text
//! cargo run -p craftkit-mcp-server
//! ```
//!
//! Every command in the registry is a tool of its own, so an agent that
//! asks for the tool list gets the command list.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::io::Write;

use craftkit_mcp::Server;

/// The most bytes a line of the protocol may hold.
///
/// A request is a JSON object and an answer is a JSON object, and a
/// tool's arguments may carry a document — so this is bigger than the
/// plugin's, and it is this program's number.
const MAX_LINE_BYTES: craftkit_sdk::lines::LineLimit = 4 * 1024 * 1024;

fn main() {
    let ecosystem = match craftkit_studio::ecosystem() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("craftkit-mcp-server: the ecosystem did not build: {error}");
            std::process::exit(1);
        }
    };
    let commands = ecosystem.commands.all().len();
    let mut server = Server::new(ecosystem.session());
    eprintln!("craftkit-mcp-server: {commands} commands, one tool each");

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in craftkit_sdk::lines::lines_bounded(stdin.lock(), MAX_LINE_BYTES) {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Some(answer) = server.handle_line(&line) else { continue };
        // The protocol is on the standard output, and **only** the
        // protocol: a log line here would break the client.
        if writeln!(stdout, "{answer}").is_err() {
            break;
        }
        let _ = stdout.flush();
    }
}
