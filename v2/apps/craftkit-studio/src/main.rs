//! The application's own entry point.
//!
//! Everything interesting is in the library: this is the part a person sees.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{RecordingCanvas, Session};
use craftkit_studio::ecosystem;

fn main() {
    let ecosystem = match ecosystem() {
        Ok(ecosystem) => ecosystem,
        Err(error) => {
            eprintln!("the ecosystem could not be built: {error}");
            std::process::exit(1);
        }
    };

    println!("── the ecosystem");
    println!("   layers:  {}", ecosystem.layers.ids().join(", "));
    println!("   kinds:  {}", ecosystem.kinds.ids().join(", "));
    println!("   tools:  {}", ecosystem.tools.ids().join(", "));
    println!("   panels: {}", ecosystem.panels.ids().join(", "));
    println!("   formats: {}", ecosystem.formats.ids().join(", "));

    println!("\n── the manifest (what an agent is told)");
    for line in ecosystem.projection().manifest() {
        println!("   {line}");
    }

    println!("\n── the menu, by capability");
    for entry in ecosystem.projection().menu(&["command", "effect"]) {
        println!("   {:<8} {:<22} {:<40} {}", entry.area, entry.id, entry.label, entry.doc);
    }

    let mut session = ecosystem.session();

    println!("\n── a command from a plugin");
    match session.execute("hello.greet", &json!({ "name": "world", "loudly": true })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }

    println!("\n── a kind of node, added by a plugin");
    match session.execute("geometry.rectangle", &json!({ "x": 3, "y": 4, "width": 5, "height": 6 })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }

    println!("\n── drawing the document, through the kinds");
    let mut canvas = RecordingCanvas::default();
    for node in session.document().nodes() {
        // A node whose kind is not linked in is **not drawn**: the core
        // does not know which kinds those are, and it asks the registry.
        let Some(kind) = session.kinds().get(&node.kind) else { continue };
        (kind.paint)(node, &mut canvas);
    }
    for drawn in &canvas.drawn {
        println!("   {drawn}");
    }

    println!("\n── hitting the document, through the kinds");
    let point = craftkit_sdk::Point::new(4.0, 5.0);
    let hit = session
        .document()
        .nodes()
        .find(|node| session.kinds().get(&node.kind).map(|kind| (kind.hit)(node, point)).unwrap_or(false));
    match hit {
        Some(node) => println!("   the point is inside `{}`", node.name),
        None => println!("   the point is in nothing"),
    }

    println!("\n── a command made while the program runs");
    let mut commands = ecosystem.commands.clone();
    match commands.register(craftkit_studio::commands::runtime_echo()) {
        Ok(()) => {
            let mut session = Session::new(commands, ecosystem.kinds.clone());
            match session.execute("runtime.echo", &json!({ "text": "made at run time" })) {
                Ok(outcome) => println!("   {}", outcome.message),
                Err(error) => println!("   refused: {error}"),
            }
        }
        Err(error) => println!("   refused: {error}"),
    }
}
