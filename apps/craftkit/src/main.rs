//! The desktop entry point.
//!
//! The application itself is the library next door: this binary reads the
//! flags and hands them over. The Android entry point is in the library too,
//! and the two agree about what the application does.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flags = match craftkit::parse_flags(&args) {
        Ok(Some(flags)) => flags,
        Ok(None) => return std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}\n\n{}", craftkit::USAGE);
            return std::process::ExitCode::from(2);
        }
    };
    craftkit::run(flags)
}
