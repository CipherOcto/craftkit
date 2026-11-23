//! The **component tree** — a2ui's own shape, checked and read.
//!
//! # What a tree is
//!
//! a2ui's components are a **flat list with ids**: every component has
//! an `id`, and a container names its children **by id** — the
//! catalogue says so itself (*"Children cannot be defined inline, they
//! must be referred to by ID"*).
//!
//! So the tree is read in three steps:
//!
//! 1. the **list** is checked — every entry is a component, every
//!    component has an id, and no two have the same one;
//! 2. the **root** is found — a2ui says one of the components has the
//!    id `root`, and it is the root of the tree; and
//! 3. the **links** are followed — a container's children are named by
//!    id, and an id that names nothing is a name that is not known, and
//!    a name that is not known is **refused**.
//!
//! # What is refused
//!
//! - a payload that is not a list of components;
//! - a component with no `component`, or one the layer does not draw;
//! - a component with no `id`, or with an id another has;
//! - a tree with no root, or with two;
//! - a child id that names no component;
//! - a component that no container holds — a tree with a part that is
//!   not in it; and
//! - a **template** child list — a2ui's own second branch, which the
//!   layer refuses (see the layer's `schema`).
//!
//! Nothing here is dropped silently: a component the tree does not use
//! is a mistake, and a mistake is refused.

use craftkit_a2ui::layout::{Align, Child, Direction, Justify, arrange};
use craftkit_sdk::document::Rect;
use craftkit_sdk::serde_json::Value as Json;

/// The id the **root** of a tree has.
///
/// a2ui says it: *"One of the components in one of the components lists
/// MUST have an 'id' of 'root' to serve as the root of the component
/// tree."*
pub const ROOT_ID: &str = "root";

/// The components that hold a **list** of children.
const LISTS: &[&str] = &["Row", "Column", "List"];

/// The components that hold **one** child.
const SINGLES: &[&str] = &["Card", "Button"];

/// One component of a tree.
#[derive(Debug, Clone)]
pub struct Component {
    /// The component's **id**: how a container names it.
    pub id: String,
    /// The **a2ui name** of the component: `Text`.
    pub kind: String,
    /// The component **as it was sent**: the props live in here.
    pub json: Json,
}

/// A **tree** of components: the flat list, checked.
#[derive(Debug, Clone)]
pub struct Tree {
    components: Vec<Component>,
}

impl Tree {
    /// Read a tree from a payload.
    ///
    /// The payload is a2ui's own `ComponentsList`: a list of components,
    /// with at least one in it.
    pub fn parse(payload: &Json) -> Result<Tree, String> {
        let Some(items) = payload.as_array() else {
            return Err("the component tree is not a list of components".to_string());
        };
        if items.is_empty() {
            return Err("the component tree is empty".to_string());
        }
        let mut components: Vec<Component> = Vec::with_capacity(items.len());
        for item in items {
            let Some(object) = item.as_object() else {
                return Err("a component is not a tree of its own".to_string());
            };
            let kind = object
                .get("component")
                .and_then(Json::as_str)
                .ok_or_else(|| "a component names no kind".to_string())?
                .to_string();
            if !craftkit_a2ui::defs::is_supported(&kind) {
                return Err(match craftkit_a2ui::defs::refusal_of(&kind) {
                    Some(reason) => format!("the component `{kind}` is refused: {reason}"),
                    None => format!("the component `{kind}` is not one this layer draws"),
                });
            }
            let id = object
                .get("id")
                .and_then(Json::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| format!("the component `{kind}` has no id"))?
                .to_string();
            if components.iter().any(|component| component.id == id) {
                return Err(format!("the id `{id}` is used by two components"));
            }
            components.push(Component { id, kind, json: item.clone() });
        }
        for component in &components {
            check_props(component)?;
        }
        let tree = Tree { components };
        tree.check_links()?;
        Ok(tree)
    }

    /// The **root** of the tree.
    pub fn root(&self) -> &Component {
        // The check has refused a tree with no root, so there is one.
        self.components
            .iter()
            .find(|component| component.id == ROOT_ID)
            .unwrap_or_else(|| &self.components[0])
    }

    /// The component with an **id**, if the tree has one.
    pub fn by_id(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|component| component.id == id)
    }

    /// Every component of the tree.
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// The **children** of a component, in the order it names them.
    ///
    /// A component that holds no children has none, and that is not an
    /// error: a `Text` holds nothing.
    pub fn children_of(&self, component: &Component) -> Result<Vec<&Component>, String> {
        if LISTS.contains(&component.kind.as_str()) {
            let children = component
                .json
                .get("children")
                .ok_or_else(|| format!("the {} `{}` names no children", component.kind, component.id))?;
            if children.is_object() {
                return Err(format!(
                    "the children of the {} `{}` are a template, and the layer refuses a template",
                    component.kind, component.id
                ));
            }
            let Some(ids) = children.as_array() else {
                return Err(format!(
                    "the children of the {} `{}` are not a list of ids",
                    component.kind, component.id
                ));
            };
            let mut out: Vec<&Component> = Vec::with_capacity(ids.len());
            for id in ids {
                let Some(id) = id.as_str() else {
                    return Err(format!(
                        "the children of the {} `{}` are not a list of ids",
                        component.kind, component.id
                    ));
                };
                let child = self
                    .by_id(id)
                    .ok_or_else(|| format!("the child `{id}` is not a component of this tree"))?;
                out.push(child);
            }
            return Ok(out);
        }
        if SINGLES.contains(&component.kind.as_str()) {
            let id = component
                .json
                .get("child")
                .and_then(Json::as_str)
                .ok_or_else(|| format!("the {} `{}` names no child", component.kind, component.id))?;
            let child =
                self.by_id(id).ok_or_else(|| format!("the child `{id}` is not a component of this tree"))?;
            return Ok(vec![child]);
        }
        Ok(Vec::new())
    }

    /// The **weight** of a component: how much of the row it takes.
    ///
    /// A component with no weight takes an equal share — the box is
    /// divided, and a child that asked for nothing does not get
    /// nothing.
    pub fn weight_of(&self, component: &Component) -> Result<f64, String> {
        let Some(weight) = component.json.get("weight") else { return Ok(1.0) };
        let number = match weight {
            Json::Number(number) => number.as_f64(),
            Json::String(text) => text.parse::<f64>().ok(),
            _ => None,
        };
        match number {
            Some(number) if number.is_finite() && number > 0.0 => Ok(number),
            _ => Err(format!("the weight of the component `{}` is not a weight", component.id)),
        }
    }

    /// The **direction** a container lays its children out in.
    pub fn direction_of(&self, component: &Component) -> Result<Direction, String> {
        match component.kind.as_str() {
            "Row" => Ok(Direction::Horizontal),
            "Column" => Ok(Direction::Vertical),
            "List" => {
                let name = component.json.get("direction").and_then(Json::as_str).unwrap_or("vertical");
                Direction::of(name).ok_or_else(|| {
                    format!("the direction `{name}` of the list `{}` is not a direction", component.id)
                })
            }
            _ => Ok(Direction::Vertical),
        }
    }

    /// The **justification** a container packs its children with.
    pub fn justify_of(&self, component: &Component) -> Result<Justify, String> {
        let name = component.json.get("justify").and_then(Json::as_str).unwrap_or("");
        Justify::of(name).ok_or_else(|| {
            format!(
                "the justification `{name}` of the {} `{}` is not a justification",
                component.kind, component.id
            )
        })
    }

    /// The **alignment** a container aligns its children with.
    pub fn align_of(&self, component: &Component) -> Result<Align, String> {
        let name = component.json.get("align").and_then(Json::as_str).unwrap_or("");
        Align::of(name).ok_or_else(|| {
            format!("the alignment `{name}` of the {} `{}` is not an alignment", component.kind, component.id)
        })
    }

    /// The **links** of the tree: every child is there, every component
    /// is in the tree, and the tree is a tree.
    ///
    /// The last one is the one that matters most: a component that is
    /// reached **twice** is a **cycle**, and a cycle is not a tree that
    /// can be drawn — it is a loop that never ends. So the walk from
    /// the root carries the components it has seen, and a component
    /// that is seen again is refused.
    ///
    /// Without this the root could be held by its own descendant, and
    /// the walk that draws the tree would go round for ever.
    fn check_links(&self) -> Result<(), String> {
        if !self.components.iter().any(|component| component.id == ROOT_ID) {
            return Err(format!("the tree has no component with the id `{ROOT_ID}`"));
        }
        // Every component that is not the root is somebody's child.
        let mut held: Vec<&str> = Vec::new();
        for component in &self.components {
            for child in self.children_of(component)? {
                if held.contains(&child.id.as_str()) {
                    return Err(format!("the component `{}` is held by two parents", child.id));
                }
                held.push(&child.id);
            }
        }
        for component in &self.components {
            if component.id == ROOT_ID {
                continue;
            }
            if !held.contains(&component.id.as_str()) {
                return Err(format!("the component `{}` is in no parent", component.id));
            }
        }
        // And the tree is a **tree**: the walk from the root reaches
        // every component once and once only.
        let mut seen: Vec<String> = Vec::new();
        let root = self.root();
        self.walk(root, &mut seen)?;
        Ok(())
    }

    /// Walk the tree from a component, refusing a **cycle**.
    ///
    /// The components seen are carried along, and a component that is
    /// seen twice is a cycle — and a cycle is refused, because a tree
    /// that is not a tree cannot be drawn and must not be walked.
    fn walk(&self, component: &Component, seen: &mut Vec<String>) -> Result<(), String> {
        if seen.iter().any(|id| id == &component.id) {
            return Err(format!(
                "the component `{}` is in a cycle: a tree is a tree and not a loop",
                component.id
            ));
        }
        seen.push(component.id.clone());
        for child in self.children_of(component)? {
            self.walk(child, seen)?;
        }
        Ok(())
    }
}

/// What a **prop** must be.
///
/// The props are a2ui's own, and the catalogue says what each one is.
/// The plugin checks them **when the tree is applied**, so that a
/// payload that does not fit is **refused** — and not drawn as nothing.
#[derive(Debug, Clone, Copy)]
enum Prop {
    /// A dynamic value: a literal, a binding or a call.
    Dynamic,
    /// A dynamic **boolean**: a boolean literal, a binding or a call.
    DynamicBool,
    /// An **icon's name**: a path, a shape, a binding or a call.
    IconName,
    /// A dynamic value that must be a **binding**.
    ///
    /// The layer narrows `TextField`'s value to a binding: a literal has
    /// nowhere to live, and the tree does not hold a second copy of it.
    Binding,
    /// A text, and the **names** it may be.
    OneOf(&'static [&'static str]),
    /// A number: the weight.
    Number,
    /// The **children** of a container.
    Children,
    /// The **one child** of a container.
    Child,
    /// An **action**: an event, or a function call.
    Action,
}

/// The props of every component the layer draws.
///
/// The names are the layer's table (`craftkit_a2ui::defs`), and this is
/// what each one must be. A property a component does not have is
/// **refused**: the published schema says `additionalProperties: false`,
/// and a schema the tree does not keep is a schema that lies.
const PROPS: &[(&str, &[(&str, Prop)])] = &[
    (
        "Text",
        &[("text", Prop::Dynamic), ("variant", Prop::OneOf(&["caption", "body"])), ("weight", Prop::Number)],
    ),
    (
        "Image",
        &[
            ("url", Prop::Dynamic),
            ("description", Prop::Dynamic),
            ("fit", Prop::OneOf(&["contain", "cover", "fill", "none", "scaleDown"])),
            (
                "variant",
                Prop::OneOf(&["icon", "avatar", "smallFeature", "mediumFeature", "largeFeature", "header"]),
            ),
            ("weight", Prop::Number),
        ],
    ),
    ("Icon", &[("name", Prop::IconName), ("weight", Prop::Number)]),
    (
        "Row",
        &[
            ("children", Prop::Children),
            (
                "justify",
                Prop::OneOf(&[
                    "center",
                    "end",
                    "spaceAround",
                    "spaceBetween",
                    "spaceEvenly",
                    "start",
                    "stretch",
                ]),
            ),
            ("align", Prop::OneOf(&["start", "center", "end", "stretch"])),
            ("weight", Prop::Number),
        ],
    ),
    (
        "Column",
        &[
            ("children", Prop::Children),
            (
                "justify",
                Prop::OneOf(&[
                    "center",
                    "end",
                    "spaceAround",
                    "spaceBetween",
                    "spaceEvenly",
                    "start",
                    "stretch",
                ]),
            ),
            ("align", Prop::OneOf(&["start", "center", "end", "stretch"])),
            ("weight", Prop::Number),
        ],
    ),
    (
        "List",
        &[
            ("children", Prop::Children),
            ("direction", Prop::OneOf(&["vertical", "horizontal"])),
            ("align", Prop::OneOf(&["start", "center", "end", "stretch"])),
            ("weight", Prop::Number),
        ],
    ),
    ("Card", &[("child", Prop::Child), ("weight", Prop::Number)]),
    ("Divider", &[("axis", Prop::OneOf(&["horizontal", "vertical"])), ("weight", Prop::Number)]),
    (
        "Button",
        &[
            ("child", Prop::Child),
            ("variant", Prop::OneOf(&["default", "primary", "borderless"])),
            ("action", Prop::Action),
            ("weight", Prop::Number),
        ],
    ),
    ("CheckBox", &[("label", Prop::Dynamic), ("value", Prop::DynamicBool), ("weight", Prop::Number)]),
    (
        "TextField",
        &[
            ("label", Prop::Dynamic),
            ("value", Prop::Binding),
            ("placeholder", Prop::Dynamic),
            ("variant", Prop::OneOf(&["longText", "number", "shortText", "obscured"])),
            ("weight", Prop::Number),
        ],
    ),
];

/// The names of a2ui's own **components**: the discriminator and the
/// id are not props.
const OWN: &[&str] = &["component", "id"];

/// The props a component **must** be given.
///
/// The published schema says which they are, and the tree keeps what
/// it publishes: a component that is missing one is **refused**, and
/// not drawn as nothing. The list here is the row's own — and the
/// plugin's own test says that it agrees with the published schema.
const REQUIRED: &[(&str, &[&str])] = &[
    ("Text", &["text"]),
    ("Image", &["url"]),
    ("Icon", &["name"]),
    ("Row", &["children"]),
    ("Column", &["children"]),
    ("List", &["children"]),
    ("Card", &["child"]),
    ("Button", &["child"]),
    ("Divider", &[]),
    ("CheckBox", &["label"]),
    ("TextField", &["label"]),
];

/// Check the **props** of a component.
///
/// Every prop is checked against what the catalogue says it is, and a
/// prop the component does not have is refused.
fn check_props(component: &Component) -> Result<(), String> {
    let Some((_, props)) = PROPS.iter().find(|(kind, _)| *kind == component.kind) else {
        return Err(format!("the component `{}` is not one this layer draws", component.kind));
    };
    let Some(object) = component.json.as_object() else {
        return Err(format!("the component `{}` is not a tree of its own", component.id));
    };
    // The props it **must** be given: the tree keeps what it publishes.
    let required = REQUIRED
        .iter()
        .find(|(kind, _)| *kind == component.kind)
        .map(|(_, required)| *required)
        .unwrap_or(&[]);
    for prop in required {
        if !object.contains_key(*prop) {
            return Err(format!(
                "the component `{}` has no `{prop}`, and the published schema says it must",
                component.id
            ));
        }
    }
    for (name, value) in object {
        if OWN.contains(&name.as_str()) {
            continue;
        }
        let Some((_, prop)) = props.iter().find(|(prop, _)| prop == name) else {
            return Err(format!("the component `{}` has no property `{name}`", component.kind));
        };
        check_prop(component, name, *prop, value)?;
    }
    Ok(())
}

/// Check one **prop** against what it must be.
fn check_prop(component: &Component, name: &str, prop: Prop, value: &Json) -> Result<(), String> {
    let refused = |what: &str| format!("the `{name}` of the component `{}` is {what}", component.id);
    match prop {
        Prop::Dynamic | Prop::DynamicBool | Prop::IconName => {
            let object = match value {
                Json::Object(object) => object,
                Json::String(text) if !matches!(prop, Prop::DynamicBool) => {
                    // An icon's name is a **path** and not a name: the
                    // fifty-nine built-in names are a narrowing, and a
                    // name the tree cannot draw is drawn as nothing.
                    if matches!(prop, Prop::IconName) && craftkit_sdk::svgpath::read_path(text).is_err() {
                        return Err(refused("a name that is not a path"));
                    }
                    return Ok(());
                }
                Json::Bool(_) if matches!(prop, Prop::DynamicBool) => return Ok(()),
                _ => return Err(refused("not a dynamic value")),
            };
            // An icon's name may be a **shape**: an object with an
            // `svgPath` in it, which is what a2ui's catalogue
            // publishes — and the shape must be a path, because a name
            // the tree cannot draw is a name that would be drawn as
            // nothing.
            if matches!(prop, Prop::IconName) && object.contains_key("svgPath") {
                let path = object.get("svgPath").and_then(Json::as_str).unwrap_or_default();
                if craftkit_sdk::svgpath::read_path(path).is_err() {
                    return Err(refused("a shape that is not a path"));
                }
                return Ok(());
            }
            let has_path = object.contains_key("@path");
            let has_call = object.contains_key("@call");
            match (has_path, has_call) {
                (true, false) => {
                    let path = object.get("@path").and_then(Json::as_str).unwrap_or_default();
                    if !craftkit_a2ui::model::is_path(path) {
                        return Err(refused("a binding to a path that is not a path"));
                    }
                    Ok(())
                }
                (false, true) => {
                    let call = object.get("@call").and_then(Json::as_str).unwrap_or_default();
                    if craftkit_a2ui::functions::by_name(call).is_none() {
                        return Err(refused("a call to a function that is not in the table"));
                    }
                    Ok(())
                }
                _ => Err(refused("neither a binding nor a call")),
            }
        }
        Prop::Binding => {
            let Some(object) = value.as_object() else { return Err(refused("not a binding")) };
            let Some(path) = object.get("@path").and_then(Json::as_str) else {
                return Err(refused("not a binding"));
            };
            if !craftkit_a2ui::model::is_path(path) {
                return Err(refused("a binding to a path that is not a path"));
            }
            Ok(())
        }
        Prop::OneOf(names) => {
            let Some(text) = value.as_str() else { return Err(refused("not a text")) };
            if !names.contains(&text) {
                return Err(refused(&format!("a `{text}` that is not one of its names")));
            }
            Ok(())
        }
        Prop::Number => match value {
            Json::Number(number) => match number.as_f64() {
                // A weight of **nothing** is a child that is not drawn:
                // and a child that is not drawn is a child the tree
                // does not show and does not say.
                Some(number) if number.is_finite() && number > 0.0 => Ok(()),
                _ => Err(refused("not a weight")),
            },
            Json::String(text) => match text.parse::<f64>() {
                Ok(number) if number.is_finite() && number > 0.0 => Ok(()),
                _ => Err(refused("not a weight")),
            },
            _ => Err(refused("not a weight")),
        },
        Prop::Children => {
            if value.is_object() {
                return Err(refused("a template, and the layer refuses a template"));
            }
            let Some(items) = value.as_array() else { return Err(refused("not a list of ids")) };
            for item in items {
                if item.as_str().is_none() {
                    return Err(refused("not a list of ids"));
                }
            }
            Ok(())
        }
        Prop::Child => {
            if value.as_str().is_none() {
                return Err(refused("not the id of a child"));
            }
            Ok(())
        }
        Prop::Action => {
            let Some(object) = value.as_object() else { return Err(refused("not an action")) };
            let event = object.get("event");
            let call = object.get("functionCall");
            match (event, call) {
                (Some(event), None) => {
                    // An **event** names what happened.
                    let name = event
                        .get("name")
                        .and_then(Json::as_str)
                        .filter(|name| !name.is_empty())
                        .ok_or_else(|| refused("an event that names nothing"))?;
                    let _ = name;
                    Ok(())
                }
                (None, Some(call)) => {
                    // A **call** names a function in the layer's table.
                    let name = call
                        .get("@call")
                        .and_then(Json::as_str)
                        .filter(|name| !name.is_empty())
                        .ok_or_else(|| refused("a call that names no function"))?;
                    if craftkit_a2ui::functions::by_name(name).is_none() {
                        return Err(refused("a call to a function that is in neither table"));
                    }
                    Ok(())
                }
                _ => Err(refused("neither an event nor a function call")),
            }
        }
    }
}

/// The **kind** a component is drawn as.
///
/// The a2ui name is `Text`, and a craftkit kind is dotted and lower
/// case: so the kind is `a2ui.text`.
pub fn component_kind(a2ui_name: &str) -> String {
    format!("a2ui.{}", a2ui_name.to_lowercase())
}

/// Whether a **kind** is a component's kind.
///
/// The surface's own kind is not one, and neither is a mirror's: the
/// surface's children are the tree, and a mirror is a value.
pub fn is_component_kind(kind: &str) -> bool {
    PROPS.iter().any(|(name, _)| component_kind(name) == kind)
}

/// The **height** of a line of text, in the world's units.
///
/// The tree's own text is drawn at twelve units, and a caption at
/// eight: so a line of text is as tall as the text in it.
const TEXT_HEIGHT: f64 = 12.0;

/// The **cross size** a child asks for, across the axis it is stacked
/// on.
///
/// A child has no size of its own in this tree, except for the two
/// whose size the tree knows: a **text** is as tall as the text in it,
/// and a **divider** is a line. So a child asks for the whole cross
/// axis — and a text in a row asks for its own height, and the
/// alignment then has something to align.
///
/// The size is never larger than the box: the box is the room there
/// is, and a child that asks for more room than there is takes what
/// there is.
fn cross_of(direction: Direction, kind: &str, cross: f64) -> f64 {
    match (direction, kind) {
        (Direction::Horizontal, "Text") => TEXT_HEIGHT.min(cross),
        (Direction::Horizontal, "Divider") => 1.0_f64.min(cross),
        _ => cross,
    }
}

/// The props a component **must** be given, by their a2ui names.
///
/// The list the tree refuses a component without — and the plugin's own
/// test says that it agrees with the published schema.
pub fn required_of(component: &str) -> Vec<String> {
    REQUIRED
        .iter()
        .find(|(kind, _)| *kind == component)
        .map(|(_, required)| required.iter().map(|prop| prop.to_string()).collect())
        .unwrap_or_default()
}

/// The **places** of a container's children, inside the container's own
/// box.
///
/// The box is divided by **weight**: every child takes a share of the
/// space, in proportion to the weight it asks for, and a child that
/// asks for nothing takes an equal share. The division fills the box,
/// so no child overflows it and none is left out.
///
/// The **direction** says which way the box runs, and the
/// **justification** and the **alignment** are the container's own —
/// the layer refuses a name it does not know.
///
/// And there is one **narrowing**, and it is written down: the box is
/// divided exactly, so there is no spare space for the justification
/// to distribute — a child that grew took it all. The alignment still
/// has something to align: a child has no size of its own here, except
/// for the two whose size the tree knows, and the alignment moves those
/// along the cross axis.
pub fn places(
    tree: &Tree,
    container: &Component,
    box_: Rect,
    children: &[&Component],
) -> Result<Vec<Rect>, String> {
    let direction = tree.direction_of(container)?;
    let justify = tree.justify_of(container)?;
    let align = tree.align_of(container)?;
    let (size, cross) = match direction {
        Direction::Horizontal => (box_.w, box_.h),
        Direction::Vertical => (box_.h, box_.w),
    };
    let mut laid: Vec<Child> = Vec::with_capacity(children.len());
    for child in children {
        let weight = tree.weight_of(child)?;
        laid.push(Child { weight: Some(weight), size: 0.0, cross: cross_of(direction, &child.kind, cross) });
    }
    let arrangement = arrange(direction, justify, align, size, cross, &laid)?;
    // The arrangement is in the box's **own** coordinates, and the box
    // has a place of its own: the places are the box's, moved to where
    // the box is.
    Ok(arrangement
        .rects
        .iter()
        .map(|rect| Rect::new(box_.x + rect.x, box_.y + rect.y, rect.w, rect.h))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_sdk::serde_json::json;

    /// A tree of two components: a root column and a text.
    fn tree() -> Json {
        json!([
            { "component": "Column", "id": "root", "children": ["t1"] },
            { "component": "Text", "id": "t1", "text": "hello" },
        ])
    }

    /// A tree is read.
    #[test]
    fn a_tree_is_read() {
        let tree = Tree::parse(&tree()).expect("the tree is read");
        assert_eq!(tree.components().len(), 2);
        assert_eq!(tree.root().id, ROOT_ID);
        assert_eq!(tree.root().kind, "Column");
    }

    /// A payload that is not a list is **refused**.
    #[test]
    fn a_payload_that_is_not_a_list_is_refused() {
        let error = Tree::parse(&json!({ "component": "Text" })).expect_err("the payload is refused");
        assert!(error.contains("not a list"), "{error}");
    }

    /// A component with no id is **refused**.
    #[test]
    fn a_component_with_no_id_is_refused() {
        let error =
            Tree::parse(&json!([{ "component": "Text", "text": "hello" }])).expect_err("it is refused");
        assert!(error.contains("no id"), "{error}");
    }

    /// Two components with one id are **refused**.
    #[test]
    fn two_components_with_one_id_are_refused() {
        let payload = json!([
            { "component": "Column", "id": "root", "children": ["t1"] },
            { "component": "Text", "id": "t1", "text": "a" },
            { "component": "Text", "id": "t1", "text": "b" },
        ]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("used by two"), "{error}");
    }

    /// A tree with no root is **refused**.
    #[test]
    fn a_tree_with_no_root_is_refused() {
        let payload = json!([{ "component": "Text", "id": "t1", "text": "a" }]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("root"), "{error}");
    }

    /// A child that names nothing is **refused**.
    #[test]
    fn a_child_that_names_nothing_is_refused() {
        let payload = json!([{ "component": "Column", "id": "root", "children": ["nope"] }]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("nope"), "{error}");
    }

    /// A component that no parent holds is **refused**.
    #[test]
    fn a_component_that_no_parent_holds_is_refused() {
        let payload = json!([
            { "component": "Column", "id": "root", "children": [] },
            { "component": "Text", "id": "t1", "text": "a" },
        ]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("no parent"), "{error}");
    }

    /// A component the layer refuses is **refused**, and says why.
    #[test]
    fn a_component_the_layer_refuses_is_refused() {
        let payload = json!([{ "component": "Video", "id": "root" }]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("media"), "{error}");
    }

    /// A **template** child list is **refused**.
    #[test]
    fn a_template_child_list_is_refused() {
        let payload = json!([
            { "component": "Column", "id": "root", "children": { "componentId": "t1", "path": "/x" } },
            { "component": "Text", "id": "t1", "text": "a" },
        ]);
        let error = Tree::parse(&payload).expect_err("it is refused");
        assert!(error.contains("template"), "{error}");
    }

    /// A **malformed weight** is **refused**.
    #[test]
    fn a_malformed_weight_is_refused() {
        let payload = json!([
            { "component": "Column", "id": "root", "children": ["t1"] },
            { "component": "Text", "id": "t1", "text": "a", "weight": "heavy" },
        ]);
        let error = Tree::parse(&payload).expect_err("the weight is refused");
        assert!(error.contains("weight"), "{error}");
    }

    /// A **weight** that is a number is read.
    #[test]
    fn a_weight_that_is_a_number_is_read() {
        let payload = json!([
            { "component": "Column", "id": "root", "children": ["t1"] },
            { "component": "Text", "id": "t1", "text": "a", "weight": 2 },
        ]);
        let tree = Tree::parse(&payload).expect("the tree is read");
        let child = tree.by_id("t1").expect("the child is there");
        assert_eq!(tree.weight_of(child).expect("the weight is read"), 2.0);
    }

    /// The **places** divide the box, and no child overflows it.
    #[test]
    fn the_places_divide_the_box() {
        let tree = Tree::parse(&tree()).expect("the tree is read");
        let root = tree.root();
        let children = tree.children_of(root).expect("the children are there");
        let box_ = Rect::new(10.0, 20.0, 100.0, 50.0);
        let places = places(&tree, root, box_, &children).expect("the places are worked out");
        assert_eq!(places.len(), 1);
        assert_eq!(places[0], Rect::new(10.0, 20.0, 100.0, 50.0));
    }

    /// Two children **share** the box, and neither overflows it.
    #[test]
    fn two_children_share_the_box() {
        let payload = json!([
            { "component": "Row", "id": "root", "children": ["t1", "t2"] },
            { "component": "Text", "id": "t1", "text": "a" },
            { "component": "Text", "id": "t2", "text": "b" },
        ]);
        let tree = Tree::parse(&payload).expect("the tree is read");
        let root = tree.root();
        let children = tree.children_of(root).expect("the children are there");
        let places = places(&tree, root, Rect::new(0.0, 0.0, 100.0, 20.0), &children)
            .expect("the places are worked out");
        assert!((places[0].w - 50.0).abs() < 0.001, "{}", places[0].w);
        assert!((places[1].w - 50.0).abs() < 0.001, "{}", places[1].w);
        assert_eq!(places[1].x, 50.0);
    }
}
