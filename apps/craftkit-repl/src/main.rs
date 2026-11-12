//! The door's own entry point: the prompt, and the line.
//!
//! The work is in the library; this is the part a person sees.

use std::io::Write;

use craftkit_repl::Reply;

fn main() {
    let ecosystem = match craftkit_studio::ecosystem() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("the ecosystem could not be built: {error}");
            std::process::exit(1);
        }
    };
    let mut repl = craftkit_repl::Repl::new(ecosystem);
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    println!("craftkit — type `help` for the commands, `quit` to leave");
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();
        // The door reads the line, and the door owns the bound: a person
        // may paste anything, and the door reads what it was asked for
        // and stops.
        match repl.read_line(&mut reader) {
            Ok(Some(Reply::Lines(lines))) => {
                for line in lines {
                    println!("{line}");
                }
            }
            Ok(Some(Reply::Quit)) => break,
            Ok(None) => break,
            Err(message) => println!("refused: {message}"),
        }
    }
}
