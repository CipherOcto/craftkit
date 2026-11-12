//! The application's own entry point.
//!
//! Everything interesting is in the library: this is the part a person sees.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{RecordingCanvas, Session};
use craftkit_studio::ecosystem;

/// Print a part of the menu, and what is in it.
fn print_menu(part: &craftkit_sdk::MenuNode, depth: usize) {
    let indent = "   ".repeat(depth);
    println!("{indent}{}", part.name);
    for command in &part.commands {
        println!("{indent}  {} — {}", command.id, command.label);
    }
    for child in &part.children {
        print_menu(child, depth + 1);
    }
}

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
    println!("   evaluators: {}", ecosystem.evaluators.ids().join(", "));

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
    // The **kit** draws: a node whose kind is not linked in is not
    // drawn, a node that is hidden is not drawn, and the handle of the
    // selected node is drawn on top.
    session.paint(&mut canvas);
    for drawn in &canvas.drawn {
        println!("   {drawn}");
    }

    println!("\n── hitting the document, through the kinds");
    let point = craftkit_sdk::Point::new(4.0, 5.0);
    // The **deepest** hit wins: a thing in a group is in front of the
    // group that holds it.
    let document = session.document();
    let hit = document
        .walk()
        .filter(|node| {
            session.kinds().get(&node.kind).map(|kind| (kind.hit)(node, document, point)).unwrap_or(false)
        })
        .last();
    match hit {
        Some(node) => println!("   the point is inside `{}`", node.name),
        None => println!("   the point is in nothing"),
    }

    println!("\n── the plan: what the document means, as the kinds say it");
    for item in session.evaluate().items {
        println!("   {} {} {}", item.kind, item.node, item.value);
    }

    println!("\n── the menu, as a tree");
    for part in ecosystem.projection().menu_tree(&["command", "effect"]) {
        print_menu(&part, 1);
    }

    println!("\n── the session's own state");
    // The tool in the hand, and a part of a node that is selected: both
    // are the session's state and not the document's.
    match session.execute("tool.hold", &json!({ "tool": "tool.rect" })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }
    let rectangle = session.document().node_by_name("rectangle 5").map(|node| node.id);
    if let Some(id) = rectangle {
        match session.execute("select.span", &json!({ "node": id, "from": 1.0, "to": 4.0 })) {
            Ok(outcome) => println!("   {}", outcome.message),
            Err(error) => println!("   refused: {error}"),
        }
    }
    // A second node, so that a relation has two ends.
    let _ = session.execute("node.add", &json!({ "name": "second", "kind": "geometry.rect" }));
    // A relation: a thing attached to another thing, and not in it.
    match session.execute("link.add", &json!({ "from": 1, "to": 2, "kind": "symbol" })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }
    // A file the document needs.
    match session.execute("doc.asset.add", &json!({ "path": "fonts/text.ttf", "kind": "font" })) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }
    println!("   selected: {:?}", session.selection());
    println!("   in the hand: {:?}", session.tool());
    println!("   relations: {:?}", session.document().relations);
    println!("   needs: {:?}", session.document().assets);

    println!("\n── a job: work that may take a while");
    // The pool is for work that may take seconds and may be cancelled.
    // The lane (§ the SDK's `realtime`) is for work that must finish.
    let pool = craftkit_sdk::JobPool::new(2);
    let nodes = session.document().nodes.len();
    pool.spawn(
        Box::new(craftkit_sdk::jobs::FnJob::new("measure", move |_cancel, progress| {
            progress.set(50);
            Ok(format!("{nodes} node(s)").into_bytes())
        })),
        0,
    );
    for _ in 0..100_000 {
        let results = pool.drain();
        if let Some(result) = results.first() {
            println!("   {} -> {}", result.name, String::from_utf8_lossy(&result.outcome.outcome_bytes()));
            break;
        }
        std::thread::yield_now();
    }

    println!("\n── the journal: what the session was asked to do");
    for entry in session.journal() {
        println!("   {} {}", entry.id, entry.params);
    }

    println!("\n── the camera: one set of controls");
    // The camera is the session's state, the way the selection and the
    // tool are. It has its own caps, and the undo does not move it.
    session.camera_mut().set_zoom(2.0);
    session.camera_mut().look_at(craftkit_sdk::Point::new(3.0, 4.0));
    println!(
        "   zoom {}, and the middle of the screen is at {:?}",
        session.camera().zoom(),
        session.camera().offset()
    );

    println!("\n── the history: a tree, not a line");
    // An edit, undone, and then done differently: the work that was
    // undone is still there, on its own branch.
    let _ = session.execute("node.add", &json!({ "name": "branch", "kind": "geometry.rect" }));
    match session.undo() {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
    }
    let _ = session.execute("node.add", &json!({ "name": "other branch", "kind": "geometry.rect" }));
    let history = session.history();
    let point = history.parent().and_then(|parent| history.node(parent));
    println!(
        "   {} branch(es) at the point where the work was undone",
        point.map(|node| node.children.len()).unwrap_or(0)
    );
    match session.execute("history.tree", &json!({})) {
        Ok(outcome) => println!("   {}", outcome.message),
        Err(error) => println!("   refused: {error}"),
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
