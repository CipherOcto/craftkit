//! `craftkit-hello-server` — a plugin that is a **separate program**.
//!
//! # What this is
//!
//! The **third door**. The other two are a crate the application depends
//! on, and a script. This one is a program that is *started while the
//! application runs*:
//!
//! - it says hello with a **catalogue**: the commands it has, with their
//!   ids, their labels and their parameters;
//! - it then answers **requests**, one line of JSON at a time, and each
//!   answer may carry the document the command would have left behind.
//!
//! # Why it is a program and not a crate
//!
//! Because the point is that the host does **not** link it. The host
//! starts it, reads what it says, and makes its commands into the host's
//! own commands — over a wire, with the same door, the same metadata and
//! the same refusals.
//!
//! # What it is not
//!
//! It is not checked by the compiler **by the host**. The catalogue is
//! data, and a catalogue with a mistake in it is refused when the plugin
//! is loaded, not when it is compiled. That is the price of this door, and
//! it is the reason the other two exist.

use craftkit_sdk::Ecosystem;

// The plugin whose commands are served. The reference keeps the crate in
// the binary: without it the linker drops it, and the crate's
// registrations go with it. This is the same trick the applications use,
// and it is the same reason.
use craftkit_plugin_hello as _;

fn main() {
    let ecosystem = match Ecosystem::from_inventory() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("the ecosystem could not be built: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = craftkit_sdk::plugin::serve(&ecosystem) {
        eprintln!("the plugin's wire broke: {error}");
        std::process::exit(1);
    }
}
