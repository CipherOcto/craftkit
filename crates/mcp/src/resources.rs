//! MCP resources: the state a client can read without calling a tool.
//!
//! A tool **acts**; a resource **is**. The server had tools and nothing else,
//! so a client that read the protocol's other half — `resources/list`,
//! `resources/read` — was told the method did not exist. Measured on the tree
//! before this module: both answered `-32601`, *"is not a method"*, and
//! `initialize` advertised the `tools` capability alone.
//!
//! The family serves both halves of the protocol: **VC** 39 lines of it,
//! **S** 7, **F** 7, **P** 5, **SC** 2. §176.
//!
//! ## One list
//!
//! `resources/list` advertises from [`RESOURCES`] and `resources/read`
//! dispatches through the same list, so a resource cannot be advertised
//! without being readable, or readable without being advertised. §171's rule,
//! applied to the half of the protocol that was missing.
//!
//! ## The content is the tool's content
//!
//! `craftkit://design` holds what `craftkit_status` holds because it is
//! **built by the same call**, and `craftkit://commands` holds what
//! `craftkit_commands` holds for the same reason. A resource that answered
//! with something else would be a second answer to one question, and this
//! tree's rule is one answer per question.
//!
//! ## Nothing here acts
//!
//! A resource reads. It changes no state, leaves no undo entry, and writes
//! no file — so a client that only reads resources cannot damage a design.
//! That is why resources are allowed here at all: they are the one part of
//! the MCP surface that is not a command, because they are not a behaviour.

use std::sync::Mutex;

use craftkit_core::NodeId;
use craftkit_engine::Session;
use serde_json::{Value as Json, json};

/// One resource the server serves.
pub struct Resource {
    /// The URI a client reads it by.
    pub uri: &'static str,
    /// The name a client shows.
    pub name: &'static str,
    /// What it holds, in one line.
    pub description: &'static str,
    /// The media type of what a read returns.
    pub mime: &'static str,
    /// What it holds, read now.
    pub read: fn(&Mutex<Session>) -> Result<Json, ReadError>,
}

/// Every resource, in `resources/list` order.
///
/// **One list.** The advertisement and the dispatch both read it, so they
/// cannot disagree about which resources exist. §176.
pub static RESOURCES: &[Resource] = &[
    Resource {
        uri: "craftkit://design",
        name: "Active design",
        description: "The open design: its name and note, its frame rate and range, its nodes, \
                      the playhead, whether it has unsaved changes, how deep the undo stack is, \
                      the last commands that ran and the plan. The same content as the \
                      craftkit_status tool.",
        mime: "application/json",
        read: design,
    },
    Resource {
        uri: "craftkit://commands",
        name: "Command catalogue",
        description: "Every command with its parameters documented, for when a tool call is \
                      refused and the client wants to see what the command actually takes. The \
                      same content as the craftkit_commands tool.",
        mime: "application/json",
        read: commands,
    },
];

/// One addressable template: the URI with `{var}` in it, and what the
/// variable holds.
pub struct Template {
    /// The URI, with the variable in braces.
    pub uri: &'static str,
    /// The name a client shows.
    pub name: &'static str,
    /// What it holds, in one line.
    pub description: &'static str,
    /// The media type of what a read returns.
    pub mime: &'static str,
    /// What it holds, read now, given the variable's value.
    pub read: fn(&Mutex<Session>, &str) -> Result<Json, ReadError>,
}

/// Every template, in `resources/templates/list` order.
///
/// **One list**, like [`RESOURCES`]. §176.
pub static TEMPLATES: &[Template] = &[Template {
    uri: "craftkit://node/{id}",
    name: "node",
    description: "One node of the open design: its id, name, kind, whether it is muted and \
                      its properties. The id is the one the design prints, `n12`.",
    mime: "application/json",
    read: node,
}];

/// Whether the server serves a resource at all.
///
/// The `resources` capability is advertised exactly when this is true, so a
/// client that reads the capability knows whether to ask. §176.
pub fn serves_any() -> bool {
    !RESOURCES.is_empty() || !TEMPLATES.is_empty()
}

/// Why a resource could not be read.
///
/// The distinction matters: a URI that names nothing is **not** a bad
/// request and **not** a missing method, and the server says so with its own
/// code rather than making a client read prose to tell the two apart.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// The URI names no resource or template this server serves.
    #[error("no resource at `{0}`; call resources/list")]
    NotFound(String),
    /// The session is unavailable.
    #[error("the session is unavailable")]
    Session,
}

/// The resources, as `resources/list` answers.
pub fn list_json() -> Json {
    Json::Array(
        RESOURCES
            .iter()
            .map(|r| {
                json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mimeType": r.mime,
                })
            })
            .collect(),
    )
}

/// The templates, as `resources/templates/list` answers.
pub fn templates_json() -> Json {
    Json::Array(
        TEMPLATES
            .iter()
            .map(|t| {
                json!({
                    "uriTemplate": t.uri,
                    "name": t.name,
                    "description": t.description,
                    "mimeType": t.mime,
                })
            })
            .collect(),
    )
}

/// Read a resource by URI.
///
/// The dispatch's half of the two lists. A URI is matched against the fixed
/// resources first and then against the templates, so a template can never
/// shadow a fixed resource.
pub fn read(uri: &str, session: &Mutex<Session>) -> Result<(&'static str, Json), ReadError> {
    if let Some(resource) = RESOURCES.iter().find(|r| r.uri == uri) {
        return (resource.read)(session).map(|json| (resource.mime, json));
    }
    for template in TEMPLATES {
        if let Some(value) = variable(template.uri, uri) {
            return (template.read)(session, value).map(|json| (template.mime, json));
        }
    }
    Err(ReadError::NotFound(uri.to_string()))
}

/// The value a URI gives a template's variable, if the URI is that template's.
///
/// The variable is one path segment: a value that carries a `/` names
/// something else, and a URI that ends at the prefix names nothing.
fn variable<'a>(template: &str, uri: &'a str) -> Option<&'a str> {
    let (prefix, rest) = template.split_once('{')?;
    let name = rest.strip_suffix('}')?;
    if name.is_empty() {
        return None;
    }
    let value = uri.strip_prefix(prefix)?;
    if value.is_empty() || value.contains('/') {
        return None;
    }
    Some(value)
}

/// The open design.
///
/// The same call the `craftkit_status` tool makes, so the two cannot differ.
fn design(session: &Mutex<Session>) -> Result<Json, ReadError> {
    let guard = session.lock().map_err(|_| ReadError::Session)?;
    Ok(guard.snapshot_json())
}

/// The command catalogue.
///
/// The same call the `craftkit_commands` tool makes.
fn commands(_session: &Mutex<Session>) -> Result<Json, ReadError> {
    Ok(craftkit_engine::catalogue())
}

/// One node of the open design.
fn node(session: &Mutex<Session>, value: &str) -> Result<Json, ReadError> {
    let Some(id) = node_id(value) else {
        return Err(ReadError::NotFound(format!("craftkit://node/{value}")));
    };
    let guard = session.lock().map_err(|_| ReadError::Session)?;
    let Some(node) = guard.node(id) else {
        return Err(ReadError::NotFound(format!("craftkit://node/{value}")));
    };
    // The properties are flattened the way the plan flattens them — by the
    // model's own `flatten`, so the path a client reads is the path a
    // command writes. A property list in a shape of its own would be a second
    // spelling of the same thing. §176.
    let at = guard.at();
    let properties: Vec<Json> = node
        .props
        .flatten()
        .into_iter()
        .map(|(path, prop)| {
            json!({
                "path": path.to_string(),
                "id": prop.id,
                "value": serde_json::to_value(prop.value_at(at)).unwrap_or(Json::Null),
                "animated": prop.is_animated(),
            })
        })
        .collect();
    Ok(json!({
        "id": node.id.get(),
        "name": node.name,
        "kind": node.kind.label(),
        "muted": node.muted,
        "properties": properties,
    }))
}

/// The node id a URI variable names, if it names one.
///
/// The tree writes a node's id as `n12` — that is [`NodeId`]'s own spelling,
/// and the one the design prints — so a URI has exactly one way to say it.
/// A bare number is refused rather than guessed at.
fn node_id(value: &str) -> Option<NodeId> {
    let rest = value.strip_prefix('n')?;
    let raw = rest.parse::<u64>().ok()?;
    Some(NodeId(raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn session() -> Mutex<Session> {
        Mutex::new(Session::sample())
    }

    /// The rule the two lists exist for: everything the server advertises
    /// can be read, and everything it can read is advertised.
    ///
    /// The defect this prevents is the one the family had: an advertisement
    /// that is not an implementation. §171, §176.
    #[test]
    fn every_resource_the_list_advertises_can_be_read() {
        let s = session();
        for resource in RESOURCES {
            let (mime, json) = read(resource.uri, &s).expect("an advertised resource reads");
            assert_eq!(mime, resource.mime, "{}", resource.uri);
            assert!(!json.is_null(), "{} holds nothing", resource.uri);
        }
    }

    /// The same rule for the templates: each one reads with a value the open
    /// design actually has.
    #[test]
    fn every_advertised_template_reads_with_a_value_the_design_has() {
        let s = session();
        let id = {
            let guard = s.lock().expect("the session");
            guard.document().nodes.first().expect("the sample has nodes").id.get()
        };
        for template in TEMPLATES {
            let value = match template.uri {
                "craftkit://node/{id}" => format!("n{id}"),
                other => panic!("{other} has no value in this test"),
            };
            let uri = template.uri.replace("{id}", &value);
            let (mime, json) = read(&uri, &s).expect("an advertised template reads");
            assert_eq!(mime, template.mime, "{uri}");
            assert_eq!(json["id"], json!(id), "{uri} read the wrong node");
        }
    }

    /// The content of a resource is the content of the tool that holds the
    /// same state — because it is built by the same call.
    ///
    /// Two answers to one question is how a client ends up acting on the one
    /// that is out of date. §176.
    #[test]
    fn a_resource_holds_what_its_tool_holds() {
        let s = session();
        let (_, design_json) = read("craftkit://design", &s).expect("the design reads");
        let (_, commands_json) = read("craftkit://commands", &s).expect("the catalogue reads");

        let tool = |name: &str| {
            let response = crate::server::handle_line(
                &format!(r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{name}"}}}}"#),
                &s,
            )
            .expect("a reply");
            let line = response.to_line().expect("json");
            let parsed: Json = serde_json::from_str(&line).expect("parses back");
            parsed["result"]["structuredContent"].clone()
        };

        assert_eq!(design_json, tool("craftkit_status"), "the design resource and the status tool differ");
        assert_eq!(
            commands_json,
            tool("craftkit_commands"),
            "the catalogue resource and the commands tool differ"
        );
    }

    /// A URI that names nothing is refused, and it is refused **as a missing
    /// resource** rather than as a missing method: the client asked for
    /// something and something was there to ask for.
    #[test]
    fn a_uri_that_names_nothing_is_refused() {
        let s = session();
        assert_eq!(
            read("craftkit://nope", &s),
            Err(ReadError::NotFound("craftkit://nope".to_string())),
            "an unknown uri was not refused"
        );
        assert!(read("craftkit://node/", &s).is_err(), "an empty variable names nothing");
        assert!(read("craftkit://node/n12/extra", &s).is_err(), "a variable is one segment");
        assert!(read("craftkit://node/12", &s).is_err(), "a bare number is not the tree's spelling");
    }

    #[test]
    fn the_variable_is_one_segment_of_the_uri() {
        assert_eq!(variable("craftkit://node/{id}", "craftkit://node/n12"), Some("n12"));
        assert_eq!(variable("craftkit://node/{id}", "craftkit://node/"), None);
        assert_eq!(variable("craftkit://node/{id}", "craftkit://node/n12/x"), None);
        assert_eq!(variable("craftkit://node/{id}", "craftkit://other/n12"), None);
    }
}
