//! MCP resources: the state a client can read without calling a tool.
//!
//! A tool **acts**; a resource **is**. This server had tools and
//! nothing else, so a client that read the protocol's other half —
//! `resources/list`, `resources/read` — was told the method did not
//! exist, and `initialize` advertised the `tools` capability alone.
//!
//! ## One list
//!
//! `resources/list` advertises from [`RESOURCES`] and `resources/read`
//! dispatches through the same list, so a resource cannot be
//! advertised without being readable, or readable without being
//! advertised.
//!
//! ## The content is the command's content
//!
//! `craftkit://commands` holds the command catalogue — every command
//! with its label, its area and its typed parameters — because those
//! are the facts the tools are built from. A resource that answered
//! with something else would be a second answer to one question, and
//! this tree's rule is one answer per question.
//!
//! ## Nothing here acts
//!
//! A resource reads. It changes no state, leaves no undo entry, and
//! writes no file — so a client that only reads resources cannot
//! damage a design. That is why resources are allowed here at all:
//! they are the one part of the MCP surface that is not a command,
//! because they are not a behaviour.

use craftkit_sdk::Session;
use craftkit_sdk::serde_json::{Value as Json, json};

/// The **code** a missing resource is answered with.
///
/// The MCP's own code: a URI that names nothing is not a bad request
/// and not a missing method, and the server says so with its own code
/// rather than making a client read prose to tell the two apart.
pub const RESOURCE_NOT_FOUND: i64 = -32002;

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
    pub read: fn(&Session) -> Result<Json, ReadError>,
}

/// Every resource, in `resources/list` order.
///
/// **One list.** The advertisement and the dispatch both read it, so
/// they cannot disagree about which resources exist.
pub static RESOURCES: &[Resource] = &[
    Resource {
        uri: "craftkit://commands",
        name: "Command catalogue",
        description: "Every command with its label, its area and its typed parameters, for when \
                      a tool call is refused and the client wants to see what the command \
                      actually takes.",
        mime: "application/json",
        read: commands,
    },
    Resource {
        uri: "craftkit://design",
        name: "The design",
        description: "The design the session holds: its nodes with their kinds and their names, \
                      and the relations between them.",
        mime: "application/json",
        read: design,
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
    pub read: fn(&Session, &str) -> Result<Json, ReadError>,
}

/// Every template, in `resources/templates/list` order.
///
/// **One list**, like [`RESOURCES`].
pub static TEMPLATES: &[Template] = &[Template {
    uri: "craftkit://node/{id}",
    name: "node",
    description: "One node of the design: its id, its name, its kind and its payload.",
    mime: "application/json",
    read: node,
}];

/// Why a resource could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The URI names no resource or template this server serves.
    NotFound(String),
}

impl ReadError {
    /// The **message** the refusal says.
    pub fn message(&self) -> String {
        match self {
            ReadError::NotFound(missing) => {
                format!("no resource at `{missing}`; call resources/list")
            }
        }
    }

    /// The **code** the refusal is answered with.
    pub fn code(&self) -> i64 {
        match self {
            ReadError::NotFound(_) => RESOURCE_NOT_FOUND,
        }
    }
}

/// The **command catalogue**: every command with its label, its area
/// and its typed parameters.
fn commands(session: &Session) -> Result<Json, ReadError> {
    let mut list: Vec<Json> = session
        .registry()
        .all()
        .iter()
        .map(|spec| {
            json!({
                "id": spec.id,
                "label": spec.label,
                "area": spec.area,
                "doc": spec.doc,
                "params": spec.params.iter().map(|param| json!({
                    "name": param.name,
                    "kind": param.kind.json_name(),
                    "help": param.help,
                    "required": param.required,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    list.sort_by(|one, other| {
        one.get("id")
            .and_then(|id| id.as_str())
            .unwrap_or_default()
            .cmp(other.get("id").and_then(|id| id.as_str()).unwrap_or_default())
    });
    Ok(json!({ "commands": list }))
}

/// The **design** the session holds: its nodes and its relations.
fn design(session: &Session) -> Result<Json, ReadError> {
    let document = session.document();
    let mut nodes: Vec<Json> =
        document.nodes().map(|node| json!({ "id": node.id, "name": node.name, "kind": node.kind })).collect();
    nodes.sort_by_key(|node| node.get("id").and_then(|id| id.as_u64()).unwrap_or_default());
    let mut relations: Vec<Json> = document
        .relations
        .iter()
        .map(|relation| json!({ "from": relation.from, "to": relation.to, "kind": relation.kind }))
        .collect();
    relations.sort_by_key(|relation| {
        (
            relation.get("from").and_then(|from| from.as_u64()).unwrap_or_default(),
            relation.get("to").and_then(|to| to.as_u64()).unwrap_or_default(),
        )
    });
    Ok(json!({ "nodes": nodes, "relations": relations }))
}

/// One **node** of the design, by its id.
fn node(session: &Session, id: &str) -> Result<Json, ReadError> {
    let Some(id) = id.trim().parse::<u64>().ok() else {
        return Err(ReadError::NotFound(id.to_string()));
    };
    let Some(node) = session.document().node(id) else {
        return Err(ReadError::NotFound(id.to_string()));
    };
    Ok(json!({
        "id": node.id,
        "name": node.name,
        "kind": node.kind,
        "data": node.data,
    }))
}

/// The resources, as `resources/list` answers.
pub fn list_json() -> Json {
    Json::Array(
        RESOURCES
            .iter()
            .map(|resource| {
                json!({
                    "uri": resource.uri,
                    "name": resource.name,
                    "description": resource.description,
                    "mimeType": resource.mime,
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
            .map(|template| {
                json!({
                    "uriTemplate": template.uri,
                    "name": template.name,
                    "description": template.description,
                    "mimeType": template.mime,
                })
            })
            .collect(),
    )
}

/// The **value** a URI gives a template's variable, if the URI is that
/// template's.
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

/// Read a resource by URI.
///
/// The dispatch's half of the two lists. A URI is matched against the
/// fixed resources first and then against the templates, so a template
/// can never shadow a fixed resource.
pub fn read(uri: &str, session: &Session) -> Result<(&'static str, Json), ReadError> {
    if let Some(resource) = RESOURCES.iter().find(|resource| resource.uri == uri) {
        return (resource.read)(session).map(|body| (resource.mime, body));
    }
    for template in TEMPLATES {
        if let Some(value) = variable(template.uri, uri) {
            return (template.read)(session, value).map(|body| (template.mime, body));
        }
    }
    Err(ReadError::NotFound(uri.to_string()))
}
