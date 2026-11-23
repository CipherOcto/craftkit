//! The **function table** — what the layer does, and what it expects.

use craftkit_a2ui::functions;
use craftkit_a2ui::serde_json::json;

/// The **five** local functions are the layer's own commands.
#[test]
fn the_five_local_functions_are_the_layers_own_commands() {
    assert_eq!(functions::LOCAL.len(), 5);
    for function in functions::LOCAL {
        let command = functions::command_of(function.name).expect("the command is there");
        assert_ne!(command, function.name, "the key and the value are never the same string");
        assert!(command.starts_with("a2ui."), "and the command is craftkit's: {command}");
    }
}

/// The agent-side functions are **fourteen**, and the layer implements
/// none of them.
#[test]
fn the_agent_side_functions_are_the_ones_the_layer_expects() {
    assert_eq!(functions::AGENT_SIDE.len(), 14);
    for function in functions::AGENT_SIDE {
        assert_eq!(function.kind, functions::Kind::Agent);
        assert!(functions::command_of(function.name).is_none(), "the layer does not implement it");
    }
}

/// A name that is in neither is **refused**.
#[test]
fn a_name_that_is_in_neither_is_refused() {
    assert!(functions::by_name("surfaceApply").is_some(), "a local one is there");
    assert!(functions::by_name("required").is_some(), "an agent-side one is there");
    assert!(functions::by_name("somethingElse").is_none(), "and an unknown one is not");
}

/// A **reserved** name is recognised, and it is refused.
#[test]
fn a_reserved_name_is_recognised() {
    assert!(functions::is_reserved("@index"));
    assert!(!functions::is_reserved("surfaceApply"));
    assert!(functions::by_name("@index").is_none(), "and the layer has no reserved function");
}

/// The declaration carries the `@call` envelope a2ui requires.
#[test]
fn the_declaration_carries_the_envelope() {
    let function = functions::by_name("surfaceApply").expect("the function is there");
    let declaration = functions::declaration(&function);
    assert_eq!(declaration["properties"]["@call"], json!({ "const": "surfaceApply" }));
    assert_eq!(declaration["required"], json!(["@call", "args"]));
    assert!(declaration["returnType"].is_string(), "and it says what it returns");
    // The keyword that refuses an unknown property belongs to a
    // **schema**: the `args` are a schema and carry it, and the
    // declaration — an instance — carries none.
    assert_eq!(declaration["properties"]["args"]["unevaluatedProperties"], json!(false));
    assert!(declaration.get("unevaluatedProperties").is_none(), "and the instance carries none");
}

/// Every declared function says **who may call it**.
///
/// A function that does not say is the renderer's alone, and the layer
/// says: the local ones are the renderer's and the agent's, and the
/// agent-side ones are the agent's.
#[test]
fn every_function_says_who_may_call_it() {
    for function in craftkit_a2ui::functions::all() {
        // The plan's own table: **both** kinds are `rendererOnly` — the
        // value that lets the renderer dispatch the call and forbids the
        // agent from calling it back.
        let allowed = "rendererOnly";
        assert_eq!(function.allowed_callers, allowed, "{} says who may call it", function.name);
        let declared = craftkit_a2ui::functions::declaration(&function);
        assert_eq!(declared["allowedCallers"], json!(allowed), "{} declares it", function.name);
    }
}

/// The **arguments** of a function are written down, in the a2ui
/// spelling and in craftkit's.
///
/// An agent that follows the catalogue can then call a function: the
/// catalogue says which arguments it takes, and the table says which
/// craftkit parameter each one is.
#[test]
fn the_arguments_are_written_down() {
    let args = functions::arguments_of("surfaceCreate").expect("the function has arguments");
    assert_eq!(args, &[("surfaceId", "surface_id")]);
    let args = functions::arguments_of("surfaceApply").expect("the function has arguments");
    assert_eq!(args, &[("surfaceId", "surface_id"), ("components", "components")]);
    // And the **declaration** says them.
    let declared = functions::declaration(&functions::LOCAL[1]);
    assert_eq!(declared["properties"]["args"]["properties"]["surfaceId"], json!({}));
    assert_eq!(declared["properties"]["args"]["properties"]["components"], json!({}));
}

/// Every **declaration** is a catalogue entry a2ui's own schema
/// accepts.
///
/// The keyword that refuses an unknown property belongs to a **schema**,
/// and a declaration is an **instance** — so the keyword is inside the
/// `args`, which are a schema, and nowhere else. An instance that
/// carries it is an entry the catalogue's own validator refuses.
#[test]
fn every_declaration_is_a_catalogue_entry_a2ui_accepts() {
    for function in functions::all() {
        let declaration = functions::declaration(&function);
        let name = function.name;
        // The **instance** carries no keyword.
        assert!(
            declaration.get("unevaluatedProperties").is_none(),
            "{name} carries a schema keyword in the instance"
        );
        // The **args** are a schema, and carry it.
        let args = &declaration["properties"]["args"];
        assert_eq!(args["unevaluatedProperties"], json!(false), "{name}");
        // The envelope is there: the `@call` const, and the required
        // list holding it.
        assert_eq!(declaration["properties"]["@call"]["const"], json!(name), "{name}");
        let required = declaration["required"].as_array().expect("there is a required list");
        assert!(required.contains(&json!("@call")), "{name}: {required:?}");
        // And every argument the table names is a property, and every
        // required argument is one of them.
        let properties = args["properties"].as_object().expect("there are properties");
        for (argument, _) in function.args {
            assert!(properties.contains_key(*argument), "{name} does not name `{argument}`");
        }
        let args_required = args.get("required").and_then(craftkit_a2ui::serde_json::Value::as_array);
        for argument in function.required {
            assert!(properties.contains_key(*argument), "{name} requires `{argument}`");
            if let Some(list) = args_required {
                assert!(list.contains(&json!(argument)), "{name} requires `{argument}`: {list:?}");
            }
        }
    }
}
