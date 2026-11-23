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
    assert_eq!(declaration["required"], json!(["@call"]));
    assert!(declaration["returnType"].is_string(), "and it says what it returns");
    assert_eq!(declaration["unevaluatedProperties"], json!(false));
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
