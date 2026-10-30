//! `craftkit-plugin-fx` — the document model of a **compositing
//! application**, as v2 nodes and payloads.
//!
//! # What is ported
//!
//! A composition is a stack of **layers**, and every layer has a **tree
//! of properties**: a group holds groups and properties, a property has a
//! value, and the value may be **animated** over time or computed by an
//! **expression**.
//!
//! What is ported is that model: comps, layers, property trees, and the
//! **paths** that address into them — `transform/position`,
//! `effects/#1/blurriness`, `contents/rect#2/size`.
//!
//! # The mapping
//!
//! | the application | v2 |
//! |---|---|
//! | a composition | a **node** of the kind `fx.comp` |
//! | a layer | a **node** of the kind `fx.layer`, in the comp |
//! | a layer's property tree | the node's **payload** |
//! | a property's value, its keyframes, its expression | the **payload** |
//! | the path that addresses a property | the **plugin's own** — see below |
//!
//! # The three things the port found
//!
//! - **A path is not a path of node ids.** The core addresses a node by
//!   its id, and an application addresses a property by *name and
//!   order*: "the third `effects` group, its `blurriness` property". That
//!   path survives a rename and a reorder, and it is **the plugin's**:
//!   the core does not know what a match id is, and a path type in the
//!   core would have to know.
//! - **A property tree is not a tree of nodes.** A layer has fifty
//!   properties and a comp has thirty layers: a thousand nodes for one
//!   picture, and the node graph would be unreadable. So the tree goes in
//!   the **payload**, and the paths address into it. The payload is the
//!   plugin's, and this is what the payload is for.
//! - **The value at a time is derived.** A property's value at frame
//!   forty-eight is the interpolation of its keyframes, and it changes
//!   when a keyframe moves. So the **evaluator** says it, and nobody
//!   writes it down.
//!
//! And one that is not a finding but a fact: the **expressions** are the
//! plugin's own language, and they live in the payload beside the
//! keyframes they sit on.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

/// The compositing layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "fx", name = "Compositing", requires("core"))]
pub struct FxLayer;

/// A **composition**: a stack of layers, and the settings around them.
#[derive(NodeKind)]
#[node_kind(kind = "fx.comp", label = "Composition", layer = "fx", evaluates)]
pub struct CompKind;

impl CompKind {
    /// A comp is not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a comp: never.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a comp **means**: how big it is, and how long.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "width": node.data.get("width").cloned().unwrap_or(serde_json::Value::Null),
            "height": node.data.get("height").cloned().unwrap_or(serde_json::Value::Null),
            "duration": node.data.get("duration").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// A **layer**: a picture in the stack, with its property tree.
#[derive(NodeKind)]
#[node_kind(kind = "fx.layer", label = "Layer", layer = "fx", evaluates)]
pub struct LayerKind;

impl LayerKind {
    /// A layer is not drawn.
    fn paint(_node: &Node, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a layer: never.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a layer **means**: how many properties it has, and how many
    /// of them are animated.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        let props = node.data.get("props").cloned().unwrap_or(serde_json::Value::Null);
        Some(serde_json::json!({
            "properties": count_properties(&props),
            "animated": count_animated(&props),
        }))
    }
}

/// Put a **composition** in the document.
#[derive(Command)]
#[command(id = "fx.comp.add", area = "fx", layer = "fx", icon = "plus", mutates, undoable)]
pub struct AddComp {
    /// How wide it is, in pixels.
    pub width: Number,
    /// How tall it is, in pixels.
    pub height: Number,
    /// How long it is, in frames.
    pub duration: Number,
}

impl AddComp {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let width = frames(self.width, "fx.comp.add")?;
        let height = frames(self.height, "fx.comp.add")?;
        let duration = frames(self.duration, "fx.comp.add")?;
        let data = serde_json::json!({
            "width": width,
            "height": height,
            "duration": duration,
        });
        let id = session.document_mut()?.add_node_with("comp".to_string(), "fx.comp", None, data);
        Ok(Outcome::changed(format!("added a {width} by {height} composition"))
            .with_data(serde_json::json!({ "id": id })))
    }
}

/// Put a **layer** in a composition.
#[derive(Command)]
#[command(id = "fx.layer.add", area = "fx", layer = "fx", icon = "plus", mutates, undoable)]
pub struct AddLayer {
    /// The composition it goes in.
    pub comp: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl AddLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let comp = self.comp.0;
        if session.document().node(comp).is_none() {
            return Err(EngineError::Failed {
                id: "fx.layer.add".to_string(),
                message: format!("there is no composition `{comp}`"),
            });
        }
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "fx.layer.add".to_string(),
                message: "a layer has a name".to_string(),
            });
        }
        let data = serde_json::json!({ "props": serde_json::json!({}) });
        let id =
            session.document_mut()?.add_node_with_under(Some(comp), name.clone(), "fx.layer", None, data);
        Ok(Outcome::changed(format!("added the layer `{name}`"))
            .with_data(serde_json::json!({ "id": id, "name": name })))
    }
}

/// Set a property, by its path.
///
/// The path is the plugin's own: `transform/position`, `effects/#1/blurriness`.
#[derive(Command)]
#[command(id = "fx.prop.set", area = "fx", layer = "fx", icon = "pencil", mutates, undoable)]
pub struct SetProp {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
    /// What it is now.
    pub value: serde_json::Value,
}

impl SetProp {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let segments = parse_path(&path);
        if segments.is_empty() {
            return Err(EngineError::Failed {
                id: "fx.prop.set".to_string(),
                message: "a path has something in it".to_string(),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: "fx.prop.set".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        set_at(&mut props, &segments, self.value.clone())
            .map_err(|message| EngineError::Failed { id: "fx.prop.set".to_string(), message })?;
        node.data["props"] = props;
        Ok(Outcome::changed(format!("`{path}` is set"))
            .with_data(serde_json::json!({ "layer": layer, "path": path })))
    }
}

/// Read a property, by its path.
#[derive(Command)]
#[command(id = "fx.prop.get", area = "fx", layer = "fx", icon = "list")]
pub struct GetProp {
    /// The layer the property is on.
    pub layer: NodeRef,
    /// The path of the property.
    pub path: Text,
}

impl GetProp {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let layer = self.layer.0;
        let path = self.path.0.clone();
        let segments = parse_path(&path);
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: "fx.prop.get".to_string(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        let props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        let value = get_at(&props, &segments).cloned();
        let message = match &value {
            Some(value) => format!("`{path}` is {value}"),
            None => format!("there is no `{path}`"),
        };
        Ok(Outcome::new(message)
            .with_data(serde_json::json!({ "layer": layer, "path": path, "value": value })))
    }
}

/// The evaluator that counts the **properties**.
///
/// How many a layer has, and how many of them are **animated**: a value
/// that is an object with keyframes on it. Both are derived — they change
/// when a property is added or a keyframe moves.
#[derive(Evaluator)]
#[evaluator(id = "fx.props", label = "Properties", layer = "fx")]
pub struct PropsEvaluator;

impl PropsEvaluator {
    /// Count the properties of every layer.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if node.kind != "fx.layer" {
                continue;
            }
            let props = node.data.get("props").cloned().unwrap_or(serde_json::Value::Null);
            plan.items.push(PlanItem {
                node: node.id,
                kind: "fx.props".to_string(),
                value: serde_json::json!({
                    "properties": count_properties(&props),
                    "animated": count_animated(&props),
                }),
            });
        }
        plan
    }
}

/// The panel that shows the composition.
#[derive(Panel)]
#[panel(id = "panel.fx", label = "Composition", layer = "fx", commands("fx.comp.add", "fx.layer.add"))]
pub struct FxPanel;

// --------------------------------------------------------------- the paths --

/// One step of a path.
///
/// A path addresses a property by **name and order**, not by id: the
/// third `effects` group, its `blurriness` property. That survives a
/// rename and a reorder, which is why it is the way an application
/// addresses its own properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    /// The child with this match id, the *n*-th one.
    Match(String, usize),
    /// The *n*-th child.
    Index(usize),
}

/// Parse a path.
///
/// `transform/position`, `effects/#1/blurriness`, `contents/rect#2/size`.
pub fn parse_path(path: &str) -> Vec<Seg> {
    let mut segments = Vec::new();
    for part in path.split(['/', '.']) {
        if part.is_empty() {
            continue;
        }
        if let Some(index) = part.strip_prefix('#').and_then(|index| index.parse::<usize>().ok()) {
            segments.push(Seg::Index(index));
            continue;
        }
        // `rect#2` is the second child with that name; `rect` is the
        // first.
        let named = part.rsplit_once('#').and_then(|(name, occurrence)| {
            occurrence.parse::<usize>().ok().map(|occurrence| (name.to_string(), occurrence.max(1)))
        });
        match named {
            Some((name, occurrence)) => segments.push(Seg::Match(name, occurrence)),
            None => segments.push(Seg::Match(part.to_string(), 1)),
        }
    }
    segments
}

/// The value at a path, if there is one.
pub fn get_at<'a>(value: &'a serde_json::Value, path: &[Seg]) -> Option<&'a serde_json::Value> {
    let (first, rest) = path.split_first()?;
    let child = match first {
        Seg::Match(name, occurrence) => {
            let child = value.get(name.as_str())?;
            if *occurrence == 1 { child } else { child.as_array()?.get(occurrence.checked_sub(1)?)? }
        }
        Seg::Index(index) => value.as_array()?.get(index.checked_sub(1)?)?,
    };
    if rest.is_empty() { Some(child) } else { get_at(child, rest) }
}

/// Put a value at a path.
///
/// The path **creates what it needs**: a layer that has no `transform`
/// group gets one, because that is what setting a property on a layer
/// means. What it does not do is invent a *list*: `effects/#5` is
/// refused when there are two effects, because there is no fifth one to
/// put a value in.
pub fn set_at(value: &mut serde_json::Value, path: &[Seg], new: serde_json::Value) -> Result<(), String> {
    let (first, rest) = path.split_first().ok_or_else(|| "a path has something in it".to_string())?;
    match first {
        Seg::Match(name, occurrence) => {
            let object = value.as_object_mut().ok_or_else(|| "there is no group here".to_string())?;
            if *occurrence > 1 {
                // The n-th child with this name: the name is a list.
                let child =
                    object.get_mut(name.as_str()).ok_or_else(|| format!("there is no `{name}` here"))?;
                let array =
                    child.as_array_mut().ok_or_else(|| format!("`{name}` is not a list of children"))?;
                let slot = array
                    .get_mut(
                        occurrence
                            .checked_sub(1)
                            .ok_or_else(|| "a child is numbered from one".to_string())?,
                    )
                    .ok_or_else(|| format!("there is no `{name}` number {occurrence} here"))?;
                return put(slot, rest, new);
            }
            if rest.is_empty() {
                object.insert(name.clone(), new);
                return Ok(());
            }
            let child = object.entry(name.clone()).or_insert_with(|| empty_group(rest));
            put(child, rest, new)
        }
        Seg::Index(index) => {
            let array = value.as_array_mut().ok_or_else(|| "this is not a list of children".to_string())?;
            let slot = array
                .get_mut(index.checked_sub(1).ok_or_else(|| "a child is numbered from one".to_string())?)
                .ok_or_else(|| format!("there is no child number {index} here"))?;
            put(slot, rest, new)
        }
    }
}

/// Put a value at the rest of a path.
fn put(value: &mut serde_json::Value, rest: &[Seg], new: serde_json::Value) -> Result<(), String> {
    if rest.is_empty() {
        *value = new;
        return Ok(());
    }
    set_at(value, rest, new)
}

/// The group a path needs: an object, or a list of one when the next step
/// is a number.
fn empty_group(rest: &[Seg]) -> serde_json::Value {
    match rest.first() {
        Some(Seg::Index(_)) => serde_json::json!([{}]),
        _ => serde_json::json!({}),
    }
}

/// How many properties a tree has.
///
/// The shape: an **object** is a group, a **leaf** is a property's value,
/// an object with a `value` in it is a property that has keyframes, and
/// an **array** is either a list of groups or a list of values — a
/// position is one property, not two.
fn count_properties(value: &serde_json::Value) -> usize {
    let Some(object) = value.as_object() else { return 0 };
    let mut count = 0;
    for child in object.values() {
        match child {
            serde_json::Value::Object(group) if group.contains_key("value") => count += 1,
            serde_json::Value::Object(_) => count += count_properties(child),
            serde_json::Value::Array(array) => {
                let all_groups = !array.is_empty() && array.iter().all(|item| item.is_object());
                if all_groups {
                    for item in array {
                        count += count_properties(item);
                    }
                } else {
                    count += 1;
                }
            }
            _ => count += 1,
        }
    }
    count
}

/// How many properties are **animated**.
fn count_animated(value: &serde_json::Value) -> usize {
    let Some(object) = value.as_object() else { return 0 };
    let mut count = 0;
    for child in object.values() {
        if let Some(group) = child.as_object() {
            if group.get("keys").map(|keys| keys.is_array()).unwrap_or(false) {
                count += 1;
                continue;
            }
            count += count_animated(child);
        } else if let Some(array) = child.as_array() {
            for item in array {
                count += count_animated(item);
            }
        }
    }
    count
}

/// A number of frames or pixels.
fn frames(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a whole number, please".to_string(),
        });
    }
    Ok(value.0 as u64)
}

// The plugin's anchor: one name that keeps every item above in the
// program, on every target.
craftkit_sdk::plugin_anchor!(
    FxLayer,
    CompKind,
    LayerKind,
    AddComp,
    AddLayer,
    SetProp,
    GetProp,
    PropsEvaluator,
    FxPanel
);

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree with a group, a property and a list.
    fn tree() -> serde_json::Value {
        serde_json::json!({
            "transform": { "position": [0, 0] },
            "effects": [ { "blurriness": 5 }, { "blurriness": 9 } ],
            "opacity": 100,
        })
    }

    /// A path is parsed into its steps.
    #[test]
    fn a_path_is_parsed_into_its_steps() {
        assert_eq!(
            parse_path("transform/position"),
            vec![Seg::Match("transform".into(), 1), Seg::Match("position".into(), 1)]
        );
        assert_eq!(
            parse_path("effects/#1/blurriness"),
            vec![Seg::Match("effects".into(), 1), Seg::Index(1), Seg::Match("blurriness".into(), 1)]
        );
        assert_eq!(
            parse_path("contents/rect#2"),
            vec![Seg::Match("contents".into(), 1), Seg::Match("rect".into(), 2)]
        );
    }

    /// A path finds the value.
    #[test]
    fn a_path_finds_the_value() {
        let tree = tree();
        assert_eq!(get_at(&tree, &parse_path("transform/position")), Some(&serde_json::json!([0, 0])));
        assert_eq!(get_at(&tree, &parse_path("opacity")), Some(&serde_json::json!(100)));
        assert_eq!(get_at(&tree, &parse_path("effects/#2/blurriness")), Some(&serde_json::json!(9)));
        assert_eq!(get_at(&tree, &parse_path("nothing/here")), None);
    }

    /// A path sets the value.
    #[test]
    fn a_path_sets_the_value() {
        let mut tree = tree();
        set_at(&mut tree, &parse_path("transform/position"), serde_json::json!([10, 20])).expect("it is set");
        assert_eq!(get_at(&tree, &parse_path("transform/position")), Some(&serde_json::json!([10, 20])));
        set_at(&mut tree, &parse_path("effects/#1/blurriness"), serde_json::json!(7)).expect("it is set");
        assert_eq!(get_at(&tree, &parse_path("effects/#1/blurriness")), Some(&serde_json::json!(7)));
    }

    /// A path that names a child that is not there is refused, and says
    /// which.
    #[test]
    fn a_path_that_names_nothing_is_refused() {
        let mut tree = tree();
        let error = set_at(&mut tree, &parse_path("effects/#5/blurriness"), serde_json::json!(1))
            .expect_err("there is no fifth effect");
        assert!(error.contains("number 5"), "{error}");
    }

    /// The setter creates the group it needs.
    #[test]
    fn the_setter_creates_the_group_it_needs() {
        let mut tree = serde_json::json!({});
        set_at(&mut tree, &parse_path("transform/position"), serde_json::json!([1, 2]))
            .expect("the group is made");
        assert_eq!(get_at(&tree, &parse_path("transform/position")), Some(&serde_json::json!([1, 2])));
    }

    /// The properties are counted, and the animated ones.
    #[test]
    fn the_properties_are_counted() {
        let tree = serde_json::json!({
            "transform": { "position": [0, 0] },
            "opacity": 100,
            "blurriness": { "value": 0, "keys": [] },
        });
        assert_eq!(count_properties(&tree), 3, "position, opacity and blurriness");
        assert_eq!(count_animated(&tree), 1, "the one with keyframes on it");
    }
}
