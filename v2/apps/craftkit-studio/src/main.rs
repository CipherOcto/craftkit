//! The application's own entry point.
//!
//! Everything interesting is in the library: this is the part a person sees.

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::json;
use craftkit_studio::catalogue;

fn main() {
    let registry = catalogue();

    println!("── the catalogue");
    for spec in registry.all() {
        let parameters: Vec<String> =
            spec.params.iter().map(|param| format!("{}: {}", param.name, param.kind.json_name())).collect();
        println!(
            "   {:<14} {:<8} {:<40} {}",
            spec.id,
            spec.area,
            spec.doc,
            if parameters.is_empty() { "no parameters".to_string() } else { parameters.join(", ") }
        );
    }
    println!("   areas: {}", registry.areas().join(", "));

    let mut session = Session::new(registry);

    println!("\n── a command from a plugin");
    match session.execute("hello.greet", &json!({ "name": "world", "loudly": true })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }

    println!("\n── a command of this application's own");
    match session.execute("node.add", &json!({ "name": "filter" })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }
    match session.execute("node.mute", &json!({ "node_name": "filter" })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }

    println!("\n── a command made while the program runs");
    let mut registry = session.registry().clone();
    match registry.register(craftkit_studio::commands::runtime_echo()) {
        Ok(()) => {
            let mut session = Session::new(registry);
            match session.execute("runtime.echo", &json!({ "text": "made at run time" })) {
                Ok(outcome) => println!("   {}", outcome.message),
                Err(error) => println!("   refused: {error}"),
            }
        }
        Err(error) => println!("   refused: {error}"),
    }
}
