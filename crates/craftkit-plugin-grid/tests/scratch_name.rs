use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

fn session() -> Session {
    let ecosystem = Ecosystem::from_inventory().expect("the ecosystem is built");
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

#[test]
fn the_dotted_name() {
    let mut session = session();
    let outcome = session.execute("grid.workbook.add", &json!({ "name": "b" })).expect("w");
    let workbook = outcome.data["id"].as_u64().unwrap();
    let outcome =
        session.execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "One" })).expect("s");
    let sheet = outcome.data["id"].as_u64().unwrap();
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("c");
    let out = session.execute(
        "grid.name.set",
        &json!({ "workbook": workbook, "name": "My.Sales", "refers_to": "One!A1" }),
    );
    println!("name set: {:?}", out.map(|o| o.data.clone()));
    let out =
        session.execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=My.Sales" }));
    match out {
        Ok(o) => println!("formula set: {:?}", o.data),
        Err(e) => println!("formula refused: {e}"),
    }
}
