//! `craftkit-a2ui-server` — the a2ui door, as a program.
//!
//! Everything interesting is in the library. This is the part an agent
//! talks to: JSON-RPC over the standard input and output.
//!
//! **The catalogue is written to a file**, and the path is written to
//! **stderr** — never to stdout, which carries the JSON-RPC stream.
//!
//! And the **control channel**: the door serves its own session on it,
//! so that a client can see what the program is showing. The port is
//! asked for on the command line — `--control 4711` — and the **token**
//! is asked for and not invented.

use craftkit_a2ui_server::Server;
use craftkit_sdk::Ecosystem;

fn main() {
    let ecosystem = match Ecosystem::from_inventory() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("the ecosystem could not be built: {error}");
            std::process::exit(1);
        }
    };
    let server = Server::new(ecosystem);

    // The **catalogue file**: the door writes it, and says where — on
    // stderr, because stdout carries the JSON-RPC stream.
    let path = std::env::temp_dir().join("craftkit-a2ui-catalog.json");
    if let Err(error) = server.write_catalogue(&path) {
        eprintln!("the catalogue was not written: {error}");
    } else {
        eprintln!("the a2ui catalogue is at {}", path.display());
    }

    // The **control channel**, over the door's own session.
    if let Some(port) = the_port() {
        open_the_channel(&server, port);
    }

    craftkit_a2ui_server::serve(server);
}

/// The port the channel is asked for, if it is asked for.
fn the_port() -> Option<u16> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--control" {
            return args.next().and_then(|port| port.parse().ok());
        }
    }
    None
}

/// Open the **control channel** over the door's own session.
///
/// The channel serves the door's document and not a copy of it: a
/// command that is run over the channel changes what the door is
/// drawing. The **token** is asked for and not invented — a channel
/// with a token that a program can guess is a channel with no token.
fn open_the_channel(server: &craftkit_a2ui_server::Server, port: u16) {
    let Ok(token) = std::env::var("CRAFTKIT_CONTROL_TOKEN") else {
        eprintln!("the control channel needs CRAFTKIT_CONTROL_TOKEN, and it was not given");
        return;
    };
    let render = std::sync::Arc::new(std::sync::Mutex::new(craftkit_a2ui_server::render::Headless::new(
        server.shared_session(),
        server.outbound(),
    )));
    let options = craftkit_control::Options {
        token,
        address: std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        announce: None,
        render: Some(render),
    };
    match craftkit_control::Server::serve_shared(server.shared_session(), options) {
        Ok(server) => {
            eprintln!("the control channel is listening on {}", server.address());
        }
        Err(error) => {
            eprintln!("the control channel could not be opened: {error}");
        }
    }
}
