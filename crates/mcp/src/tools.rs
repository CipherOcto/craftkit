//! Turning the command registry into MCP tools.
//!
//! # One table, two shapes
//!
//! A tool's JSON schema is generated from the same [`craftkit_engine::ParamDoc`]
//! the CLI help and the docs are generated from. That is the whole point: a
//! parameter an agent cannot see in the schema is a parameter it will guess at,
//! and a guess that is silently ignored is worse than a refusal.
//!
//! # Types
//!
//! craftkit's [`Doc`] types map onto JSON Schema types. The two that do not map
//! directly — `node-ref` and `time` — become strings, because a JSON Schema
//! that says "number" for something that also accepts `"12f"` is a schema that
//! lies.

use std::sync::Mutex;

use craftkit_engine::{Doc, Session, command};
use serde_json::{Map, Value as Json, json};

use crate::rpc::{self, Response};

/// One MCP tool.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Json,
}

impl Tool {
    /// The JSON MCP sends in `tools/list`.
    pub fn to_json(&self) -> Json {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        })
    }
}

/// A tool that is not a command.
///
/// It carries **what it is** — the name and the description a client reads —
/// and **what it does** — the handler. One record, so the advertisement and
/// the dispatch cannot disagree about which tools exist. §171.
pub struct BuiltIn {
    /// The name the client calls it by.
    pub name: &'static str,
    /// What it does, in one line.
    pub description: &'static str,
    /// The schema its arguments have.
    pub schema: fn() -> Json,
    /// What it does when it is called.
    pub call: fn(id: &Json, arguments: &Json, session: &Mutex<Session>) -> Response,
}

/// The tools that are not a command.
///
/// Three of them, and they are about the *server* rather than the document:
/// the running session, the catalogue, and the crash reports this machine
/// has written.
///
/// **One list.** `tools/list` advertises from it and `tools/call` dispatches
/// through it, so a tool cannot be advertised without being callable, or
/// callable without being advertised. §171.
///
/// The comment that used to sit here said *"`session.state` and
/// `diagnostics.list` are commands already, so they appear from the registry"*.
/// `diagnostics.list` is a command and does appear from the registry.
/// `session.state` is **not** a command — it is the control channel's method,
/// and the registry has no such id — so it did not appear, and the sentence
/// was wrong about the one thing it was explaining. §113.
pub const BUILT_INS: &[BuiltIn] = &[
    BuiltIn {
        name: "craftkit_status",
        description: "The running session: the document's name, its note, its frame rate and \
                      its range, its nodes, the playhead, whether the document has unsaved \
                      changes, how deep the undo stack is and how much memory it holds, the \
                      last commands that ran, and the plan. Call this first.",
        schema: no_arguments,
        call: status,
    },
    BuiltIn {
        name: "craftkit_commands",
        description: "The full command catalogue with every parameter documented, for when \
                      a command refuses and you want to see what the command actually takes.",
        schema: no_arguments,
        call: commands,
    },
    BuiltIn {
        name: "craftkit_crashes",
        description: "The crash reports this machine has written. `action` is list, show or \
                      replay; `show` and `replay` need the report's `id`. A report holds the \
                      design that crashed and the commands that ran, and `replay` runs those \
                      commands again and says whether the design comes back the same.",
        schema: crash_arguments,
        call: crashes,
    },
];

/// The built-in a name is, if it is one.
///
/// The dispatch's half of [`BUILT_INS`]. §171.
pub fn built_in(name: &str) -> Option<&'static BuiltIn> {
    BUILT_INS.iter().find(|built_in| built_in.name == name)
}

/// The built-in tools, as the client sees them.
fn built_in_tools() -> Vec<Tool> {
    BUILT_INS
        .iter()
        .map(|built_in| Tool {
            name: built_in.name.to_string(),
            description: built_in.description.to_string(),
            input_schema: (built_in.schema)(),
        })
        .collect()
}

/// The schema of a tool that takes nothing.
fn no_arguments() -> Json {
    object_schema(&[])
}

/// The schema of the crash reports' tool.
fn crash_arguments() -> Json {
    object_schema(&[
        (
            "action".to_string(),
            json!({
                "type": "string",
                "enum": ["list", "show", "replay"],
                "description": "list the reports, show one, or replay one.",
            }),
        ),
        (
            "id".to_string(),
            json!({
                "type": "string",
                "description": "The report's id, from the list. Required for show and replay.",
            }),
        ),
    ])
}

/// The running session.
fn status(id: &Json, _arguments: &Json, session: &Mutex<Session>) -> Response {
    let Ok(guard) = session.lock() else {
        return rpc::tool_error(id, "poisoned", "the session is unavailable", None);
    };
    rpc::tool_ok(id, guard.snapshot_json())
}

/// The command catalogue.
fn commands(id: &Json, _arguments: &Json, _session: &Mutex<Session>) -> Response {
    rpc::tool_ok(id, craftkit_engine::catalogue())
}

/// The crash reports this machine has written.
///
/// The tree's answer to "my application crashed", and the agent holding the
/// tool is the one most likely to be holding it because of a crash. §113.
fn crashes(id: &Json, arguments: &Json, session: &Mutex<Session>) -> Response {
    let action = arguments.get("action").and_then(Json::as_str).unwrap_or("list");
    let report_id = arguments.get("id").and_then(Json::as_str);
    let from = match session.lock() {
        Ok(guard) => guard.document().meta.name.clone(),
        Err(_) => {
            return rpc::tool_error(id, "poisoned", "the session is unavailable", None);
        }
    };
    let dir = craftkit_engine::crash::report_dir();
    match craftkit_engine::crash::action_in(&dir, action, report_id, &from) {
        Ok(data) => rpc::tool_ok(id, data),
        Err(e) => rpc::tool_error(id, e.kind(), &e.message(), None),
    }
}

/// Every tool, built-ins first and then the commands in registry order.
pub fn all() -> Vec<Tool> {
    let mut tools = built_in_tools();
    tools.extend(command::all().iter().map(|spec| tool_for(spec)));
    tools
}

pub fn tools_json() -> Json {
    Json::Array(all().iter().map(Tool::to_json).collect())
}

/// The tool name a command is exposed under.
///
/// `node.add` becomes `craftkit_node_add`. Namespacing matters here more than in
/// a CLI: a client's tool list is a flat namespace shared with every other
/// server, and `add` on its own would be a collision waiting to happen.
pub fn tool_name(id: &str) -> String {
    format!("{}_{}", crate::SERVER_NAME, id.replace('.', "_"))
}

/// The command id behind a tool name, if it is one.
pub fn command_for(name: &str) -> Option<&'static str> {
    let prefix = format!("{}_", crate::SERVER_NAME);
    if !name.starts_with(&prefix) {
        return None;
    }
    // The full id is reconstructed from the command table rather than by
    // string surgery: `prop_set` and `prop.reset` are one command, and
    // guessing which dots were separators is exactly the kind of guess that
    // goes wrong. §171.
    command::all().iter().find(|c| tool_name(c.id) == name).map(|c| c.id)
}

/// The MCP tool for a command.
pub fn tool_for(spec: &command::CommandSpec) -> Tool {
    let mut description = format!("{} — {}", spec.label, spec.doc);
    if !spec.params.is_empty() {
        let parts: Vec<String> = spec.params.iter().map(|p| format!("{} ({})", p.name, p.help)).collect();
        description.push_str("\n\nParameters: ");
        description.push_str(&parts.join("; "));
    }
    if let Some(shortcut) = spec.shortcut {
        // The tree binds no keys, so the chord is a specification rather than
        // a binding. Saying "Keyboard: Ctrl+Z" would tell an agent to send a
        // person to a key that does nothing. §83.
        description.push_str(&format!("\n\nIntended keyboard: {shortcut} (this build binds no keys)"));
    }
    Tool { name: tool_name(spec.id), description, input_schema: schema_for(spec.params) }
}

/// Every node kind, read from the model rather than written out here.
///
/// A hand-copied list would be one more place for a new kind to be forgotten.
fn node_kind_names() -> Json {
    Json::Array(craftkit_model::NodeKind::all().iter().map(|k| Json::from(k.label())).collect())
}

/// The JSON Schema for a command's parameters.
fn schema_for(params: &[command::ParamDoc]) -> Json {
    let properties: Vec<(String, Json)> =
        params.iter().map(|p| (p.name.to_string(), property_for(p.kind, p.help))).collect();
    let required: Vec<&str> = params.iter().filter(|p| p.required).map(|p| p.name).collect();
    let mut schema = object_schema(&properties);
    if !required.is_empty() {
        schema["required"] = Json::Array(required.into_iter().map(Json::from).collect());
    }
    // A parameter that may be given instead of another one is **one of the
    // two**, and the schema has to say so: a client that reads `required`
    // alone would refuse the second spelling — which the command accepts,
    // and which the tree's own probe uses. §116.
    //
    // The pairs are combined with `allOf`, because two pairs are two
    // separate "one of"s and an `anyOf` over all four names would let a
    // call through with neither of one of them.
    let pairs: Vec<Json> = params
        .iter()
        .filter_map(|p| p.alternative.map(|alternative| (p.name, alternative)))
        .map(
            |(name, alternative)| json!({ "anyOf": [{ "required": [name] }, { "required": [alternative] }] }),
        )
        .collect();
    if !pairs.is_empty() {
        schema["allOf"] = Json::Array(pairs);
    }
    schema
}

fn object_schema(properties: &[(String, Json)]) -> Json {
    let mut map = Map::new();
    map.insert("type".to_string(), json!("object"));
    if properties.is_empty() {
        map.insert("properties".to_string(), json!({}));
    } else {
        let mut props = Map::new();
        for (name, shape) in properties {
            props.insert(name.clone(), shape.clone());
        }
        map.insert("properties".to_string(), Json::Object(props));
    }
    map.insert("additionalProperties".to_string(), json!(false));
    Json::Object(map)
}

fn property_for(kind: Doc, help: &str) -> Json {
    let mut map = Map::new();
    map.insert("description".to_string(), json!(help));
    match kind {
        Doc::Number => {
            map.insert("type".to_string(), json!("number"));
        }
        Doc::Bool => {
            map.insert("type".to_string(), json!("boolean"));
        }
        // Everything else is text on the wire. `time` is addressed by
        // seconds, frames or ticks, `easing` by preset name — none of which
        // a JSON Schema number can express honestly.
        Doc::Text | Doc::Color => {
            map.insert("type".to_string(), json!("string"));
            if matches!(kind, Doc::Color) {
                // The colour's shape is the core's own: the tree publishes
                // the rule the parser enforces, not a copy of it. §254.
                map.insert("pattern".to_string(), json!(craftkit_core::Color::PATTERN));
            }
        }
        Doc::NodeRef => {
            // A **whole number**, and nothing else. §116: the schema used to
            // say `["number", "string"]` — "a node id, as a number or as the
            // node's name" — and the reader refuses a name in this parameter
            // ("`Level` is a name, not an id; names are resolved by
            // `node_name`"). A name belongs in `node_name`, which is the
            // parameter's alternative and is in the schema beside it.
            map.insert("type".to_string(), json!("number"));
            map.insert(
                "description".to_string(),
                json!(format!("{help} A whole number, or use `node_name`.")),
            );
        }
        Doc::NodeKind => {
            map.insert("type".to_string(), json!("string"));
            map.insert("enum".to_string(), node_kind_names());
        }
        Doc::Time => {
            map.insert("type".to_string(), json!(["number", "string"]));
            map.insert(
                "description".to_string(),
                json!(format!("{help} Seconds as a number, or \"12f\" for frames, \"120t\" for ticks.")),
            );
        }
        Doc::PropertyPath => {
            map.insert("type".to_string(), json!("string"));
            map.insert("examples".to_string(), json!(["amount", "trigger/lead", "effects/#2/amount", "@12"]));
        }
        Doc::Easing => {
            // **A string, and not an enum.** The tree publishes an easing's
            // own spelling, and a bezier that is not one of the presets is
            // published with its control points — `bezier(0.5,0.25)` — so an
            // enum of the five preset names would refuse a value the tree
            // itself writes. §267.
            map.insert("type".to_string(), json!("string"));
            map.insert(
                "description".to_string(),
                json!(format!(
                    "{help} linear, hold, easeIn, easeOut or easeInOut, or a bezier with its \
                     control points, bezier(outX,outY)."
                )),
            );
        }
    }
    Json::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_model::NodeKind;

    /// The colour the schema publishes is the colour the core reads.
    ///
    /// §254: the schema carried its own copy of the colour's shape, and the
    /// core's parser was looser than the copy — `"#+f+f+f"` parsed as a
    /// colour and the pattern rejected it. The pattern is now the core's
    /// own, so the two cannot drift.
    #[test]
    fn the_colour_pattern_the_schema_publishes_is_the_core_own() {
        assert_eq!(craftkit_core::Color::PATTERN, "^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$");
        let params = [command::ParamDoc::req("tint", craftkit_engine::Doc::Color, "the colour")];
        let schema = schema_for(&params);
        assert_eq!(
            schema["properties"]["tint"]["pattern"].as_str(),
            Some(craftkit_core::Color::PATTERN),
            "the schema publishes its own copy of the colour's shape"
        );
    }

    #[test]
    fn every_command_becomes_exactly_one_tool() {
        let tools = all();
        // The built-ins are the three that are not a command: the running
        // session, the catalogue, and the crash reports. §113. The count is
        // the table's, not a number written here. §171.
        assert_eq!(tools.len(), command::all().len() + BUILT_INS.len());
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "two tools share a name");
    }

    /// Every tool name a description mentions is a tool this server
    /// advertises.
    ///
    /// The `craftkit_commands` tool's description said "for when
    /// **`craftkit_execute`** refuses" — and the server advertises no such
    /// tool. The commands are advertised one by one (`craftkit_node_add`,
    /// `craftkit_prop_set`, …) and the registry has no `execute` command, so
    /// an agent that read the description, looked for the tool and did not
    /// find it, could conclude the server cannot run commands at all. §198.
    ///
    /// A description is the one thing a client reads before it calls
    /// anything, so a name in it is a promise that the name exists — and the
    /// list of names is right here, so the check is one list against itself.
    #[test]
    fn every_tool_name_in_a_description_is_a_tool_the_server_advertises() {
        let tools = all();
        for tool in &tools {
            for mentioned in tool_names_in(&tool.description) {
                assert!(
                    tools.iter().any(|other| other.name == mentioned),
                    "`{}` mentions `{}`, which this server does not advertise",
                    tool.name,
                    mentioned
                );
            }
        }
    }

    /// The tool names a description mentions.
    ///
    /// A word that starts with the server's own prefix: `craftkit_` and then
    /// letters and underscores. The description is prose, so the words are
    /// split on everything that is not part of a name.
    fn tool_names_in(description: &str) -> Vec<String> {
        let prefix = format!("{}_", crate::SERVER_NAME);
        description
            .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
            .filter(|word| word.starts_with(&prefix) && word.len() > prefix.len())
            .map(str::to_string)
            .collect()
    }

    /// Every built-in is advertised.
    ///
    /// The other direction: a tool the server can call but does not
    /// advertise is a tool no client finds. §171.
    #[test]
    fn every_built_in_is_advertised() {
        let advertised = all();
        for built_in in BUILT_INS {
            assert!(
                advertised.iter().any(|tool| tool.name == built_in.name),
                "{} is callable but not advertised",
                built_in.name
            );
        }
    }

    /// The crash reports are a tool.
    ///
    /// §113: the tree writes a crash report with the design that crashed and
    /// the commands that ran, and the CLI and the control channel could both
    /// read them — but the MCP server, whose reader is an agent, had no way to
    /// see one.
    #[test]
    fn the_crash_reports_are_a_tool() {
        let tools = all();
        let tool = tools.iter().find(|t| t.name == "craftkit_crashes").expect("a crash tool");
        let schema = tool.to_json();
        let text = schema.to_string();
        assert!(text.contains("action"), "the tool does not take an action: {text}");
        assert!(text.contains("id"), "the tool does not take an id: {text}");
        assert!(
            text.contains("list") && text.contains("show") && text.contains("replay"),
            "the tool does not say which actions it has: {text}"
        );
    }

    #[test]
    fn tool_names_are_namespaced_and_round_trip() {
        assert_eq!(tool_name("node.add"), "craftkit_node_add");
        for spec in command::all() {
            let name = tool_name(spec.id);
            assert!(name.starts_with("craftkit_"), "{name}");
            assert_eq!(command_for(&name), Some(spec.id), "{name} did not resolve back");
        }
    }

    #[test]
    fn a_name_that_is_not_a_tool_resolves_to_nothing() {
        assert_eq!(command_for("craftkit_node_nope"), None);
        assert_eq!(command_for("node.add"), None);
        assert_eq!(command_for(""), None);
    }

    #[test]
    fn a_required_parameter_is_listed_in_required() {
        let json = tools_json();
        let arr = json.as_array().expect("array");
        let set = arr.iter().find(|t| t["name"] == json!("craftkit_prop_set")).expect("prop.set is exposed");
        let required = set["inputSchema"]["required"].as_array().expect("required");
        assert!(required.contains(&json!("path")), "{set}");
        assert!(required.contains(&json!("value")), "{set}");
        // An optional parameter must not be in `required`, or a client will
        // refuse to call the tool without it.
        assert!(!required.contains(&json!("animated")), "{set}");
    }

    /// A parameter with an alternative is not required by itself — the pair
    /// is "one of the two", and the schema has to say so.
    ///
    /// §116: the schema listed `node` in `required`, so a client was told
    /// it could not use `node_name` — which the command accepts, and which
    /// the tree's own probe uses for `playback.seek`.
    #[test]
    fn a_parameter_with_an_alternative_is_one_of_the_two() {
        let json = tools_json();
        let arr = json.as_array().expect("array");

        let set = arr.iter().find(|t| t["name"] == json!("craftkit_prop_set")).expect("prop.set is exposed");
        let required = set["inputSchema"]["required"].as_array().expect("required");
        assert!(!required.contains(&json!("node")), "the id is one of a pair, not required: {set}");
        let pairs = set["inputSchema"]["allOf"].as_array().expect("the pair");
        assert_eq!(pairs.len(), 1, "{set}");
        let any_of = pairs[0]["anyOf"].as_array().expect("one of");
        assert_eq!(any_of.len(), 2, "{set}");
        assert!(any_of[0]["required"].as_array().expect("required").contains(&json!("node")), "{set}");
        assert!(any_of[1]["required"].as_array().expect("required").contains(&json!("node_name")), "{set}");

        // And the same for the other pair in the tree.
        let seek =
            arr.iter().find(|t| t["name"] == json!("craftkit_playback_seek")).expect("the seek is exposed");
        let pairs = seek["inputSchema"]["allOf"].as_array().expect("the pair");
        let any_of = pairs[0]["anyOf"].as_array().expect("one of");
        assert!(any_of[0]["required"].as_array().expect("required").contains(&json!("seconds")), "{seek}");
        assert!(any_of[1]["required"].as_array().expect("required").contains(&json!("frame")), "{seek}");
    }

    /// The node parameter is a whole number, and not a name.
    ///
    /// §116: the schema said `["number", "string"]` — "a node id, as a
    /// number or as the node's name" — and the reader refuses a name in
    /// this parameter (*"`Level` is a name, not an id"*).
    #[test]
    fn the_node_parameter_is_a_whole_number() {
        let json = tools_json();
        let arr = json.as_array().expect("array");
        let set = arr.iter().find(|t| t["name"] == json!("craftkit_prop_set")).expect("prop.set is exposed");
        assert_eq!(set["inputSchema"]["properties"]["node"]["type"], json!("number"), "{set}");
        assert_eq!(set["inputSchema"]["properties"]["node_name"]["type"], json!("string"), "{set}");
    }

    #[test]
    fn a_command_with_no_parameters_has_no_required_key() {
        let json = tools_json();
        let arr = json.as_array().expect("array");
        let undo = arr
            .iter()
            .find(|t| t["name"] == json!("craftkit_history_undo"))
            .expect("history.undo is exposed");
        assert!(undo["inputSchema"].get("required").is_none(), "{undo}");
        assert_eq!(undo["inputSchema"]["type"], json!("object"));
        assert_eq!(undo["inputSchema"]["additionalProperties"], json!(false));
    }

    #[test]
    fn unknown_parameters_are_refused_by_the_schema_too() {
        // The engine refuses them anyway; saying so in the schema means an
        // agent finds out before it sends anything.
        let json = tools_json();
        let arr = json.as_array().expect("array");
        for t in arr {
            assert_eq!(t["inputSchema"]["additionalProperties"], json!(false), "{}", t["name"]);
        }
    }

    #[test]
    fn every_documented_parameter_has_a_description() {
        for spec in command::all() {
            let tool = tool_for(spec);
            let props = tool.input_schema["properties"].as_object().expect("properties");
            for p in spec.params {
                let shape =
                    props.get(p.name).unwrap_or_else(|| panic!("{} has no schema for {}", tool.name, p.name));
                let desc = shape["description"].as_str().unwrap_or_default();
                assert!(!desc.is_empty(), "{}.{} has no description", tool.name, p.name);
            }
        }
    }

    #[test]
    fn the_node_kind_parameter_lists_the_real_kinds() {
        let spec = command::get("node.add").expect("registered");
        let tool = tool_for(spec);
        let kinds = tool.input_schema["properties"]["kind"]["enum"].as_array().expect("enum");
        for k in NodeKind::all() {
            assert!(kinds.contains(&json!(k.label())), "{k:?} is missing from the schema");
        }
    }

    #[test]
    fn every_description_says_what_the_command_does() {
        for tool in all() {
            assert!(tool.description.len() > 20, "{}: {:?}", tool.name, tool.description);
        }
    }

    /// A tool description does not advertise a key the build does not bind.
    ///
    /// The tree binds no keys — there is no shortcut parser and no key
    /// handling in the UI — so a chord in the registry is a specification. The
    /// old description said "Keyboard: Ctrl+Z", which an agent reads as
    /// "send the person here", and the person would press a key that does
    /// nothing. §83.
    #[test]
    fn a_tool_description_does_not_claim_a_key_the_build_does_not_bind() {
        for spec in command::all() {
            let tool = tool_for(spec);
            let Some(chord) = spec.shortcut else { continue };
            assert!(
                tool.description.contains(&format!("Intended keyboard: {chord}")),
                "{}: the chord is not in the description: {:?}",
                spec.id,
                tool.description
            );
            assert!(
                tool.description.contains("binds no keys"),
                "{}: the description claims a binding the build does not have: {:?}",
                spec.id,
                tool.description
            );
        }
    }
}
