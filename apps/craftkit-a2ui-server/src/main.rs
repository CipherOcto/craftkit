//! `craftkit-a2ui-server` — the a2ui door, as a program.
//!
//! Everything interesting is in the library. This is the part an agent
//! talks to: JSON-RPC over the standard input and output.
//!
//! **The catalogue is written to a file**, and the path is written to
//! **stderr** — never to stdout, which carries the JSON-RPC stream.

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

    craftkit_a2ui_server::serve(server);
}
