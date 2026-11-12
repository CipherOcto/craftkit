//! `craftkit-bridge` — the second front end, as a program.
//!
//! Everything interesting is in the library. This is the part an agent
//! talks to: JSON-RPC over the standard input and output.

use craftkit_bridge::Bridge;
use craftkit_sdk::Ecosystem;

fn main() {
    let ecosystem = match Ecosystem::from_inventory() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("the ecosystem could not be built: {error}");
            std::process::exit(1);
        }
    };
    craftkit_bridge::serve(Bridge::new(ecosystem));
}
