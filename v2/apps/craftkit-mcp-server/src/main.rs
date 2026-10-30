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

use std::io::{BufRead, Write};

use craftkit_mcp::Server;

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
    for line in stdin.lock().lines() {
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
