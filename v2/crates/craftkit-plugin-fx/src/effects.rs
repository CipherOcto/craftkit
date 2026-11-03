//! The **effects** of a layer.
//!
//! An effect is a named set of **parameters** that a layer may carry:
//! a blur with a radius, a glow with a radius and an intensity. The
//! tree's effect is a `EffectSpec` — an id, a name, a category and its
//! typed parameters — and an **instance** of one on a layer is a
//! property group, so that every parameter is animatable and
//! addressable like any other property.
//!
//! Here the **catalogue** is the kit's own and it is small: this tree
//! has no renderer, and a catalogue of effects that nothing draws
//! would be a list of names. What is here is what an effect **is** —
//! the parameters it takes, their kinds, their bounds and their
//! defaults — and an instance is a group in the layer's property tree
//! that says which effect it is and holds its parameters at their
//! defaults.
//!
//! And an instance is made by **a command**: `fx.effect.add` puts one
//! on a layer, `fx.effect.remove` takes one away, and `fx.effect.list`
//! says what there is to put on. A group under `effects` that no
//! command made is not an effect, and `fx.prop.set` refuses to make
//! one.
//!
//! # What the port found
//!
//! - **The tree's instance has a uid and v2's has its order.** The
//!   tree's instance is a property group with a uid of its own, so
//!   moving it in the stack does not move anything that addresses it.
//!   Here the instance is addressed by its **number** in the stack —
//!   `effects/#2/radius` — because that is the way the property paths
//!   address it, and so a move in the stack is a move of what the
//!   paths say. The tree's reorder of the stack is not here yet, and
//!   when it comes it will say what it does to the paths.
//! - **The tree's catalogue is hundreds of effects and v2's is six.**
//!   The tree renders them, and this tree does not render anything:
//!   the `fx.layer` kind says so in its own words ("a layer is not
//!   drawn"). So what is ported is the **shape** of an effect — an id,
//!   a name, a category, typed parameters with defaults — and a small
//!   catalogue that shows the shape.
//!
//! # What it does not do
//!
//! - **The tree's bypass of one instance is not here.** The tree may
//!   switch one effect off and leave its values in place. Here the
//!   layer's own `effects` switch says whether the effects run at all
//!   (the module `switches`), and one instance has no switch of its
//!   own.
//! - **The tree's reorder, duplicate and reset are not here.** They
//!   are commands on an instance, and they come when the instance is
//!   worth having.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The **kind** of a parameter, and what it holds.
#[derive(Clone, Copy)]
pub enum ParamKind {
    /// A number between two bounds.
    Number {
        /// The smallest it may be.
        min: f64,
        /// The largest it may be.
        max: f64,
        /// What it is when it is new.
        default: f64,
    },
    /// A yes or a no.
    Boolean {
        /// What it is when it is new.
        default: bool,
    },
    /// One of a list of numbers.
    Popup {
        /// The options, each with the number it stands for.
        options: &'static [(&'static str, f64)],
        /// Which option it is when it is new.
        default: usize,
    },
}

impl ParamKind {
    /// The **name** of the kind, for the answer.
    pub fn name(&self) -> &'static str {
        match self {
            ParamKind::Number { .. } => "number",
            ParamKind::Boolean { .. } => "boolean",
            ParamKind::Popup { .. } => "popup",
        }
    }

    /// The **value** a parameter of this kind has when it is new.
    pub fn default_value(&self) -> serde_json::Value {
        match self {
            ParamKind::Number { default, .. } => serde_json::json!(*default),
            ParamKind::Boolean { default } => serde_json::json!(*default),
            ParamKind::Popup { options, default } => {
                let value = options.get(*default).map(|(_, value)| *value).unwrap_or(0.0);
                serde_json::json!(value)
            }
        }
    }
}

/// A **parameter** of an effect.
pub struct ParamSpec {
    /// What it is called, in a path.
    pub id: &'static str,
    /// What it is called, for a person.
    pub name: &'static str,
    /// What kind of thing it is.
    pub kind: ParamKind,
}

/// An **effect**: what it is and what it takes.
pub struct EffectSpec {
    /// What it is called, in a path.
    pub id: &'static str,
    /// What it is called, for a person.
    pub name: &'static str,
    /// Where it sits in the tree's own order of kinds.
    pub category: &'static str,
    /// The parameters it takes.
    pub params: &'static [ParamSpec],
}

/// The **catalogue**: the effects there are.
///
/// This is the kit's own and it is deliberately small. A tree that
/// renders its effects has hundreds of them; this tree renders
/// nothing, and what it has is the shape of one.
const CATALOGUE: &[EffectSpec] = &[
    EffectSpec {
        id: "blur",
        name: "Blur",
        category: "blur",
        params: &[ParamSpec {
            id: "radius",
            name: "Radius",
            kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
        }],
    },
    EffectSpec {
        id: "glow",
        name: "Glow",
        category: "stylize",
        params: &[
            ParamSpec {
                id: "radius",
                name: "Radius",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
            },
            ParamSpec {
                id: "intensity",
                name: "Intensity",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
            },
        ],
    },
    EffectSpec {
        id: "levels",
        name: "Levels",
        category: "color",
        params: &[
            ParamSpec {
                id: "input_low",
                name: "Input Low",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
            },
            ParamSpec {
                id: "input_high",
                name: "Input High",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 100.0 },
            },
            ParamSpec {
                id: "output_low",
                name: "Output Low",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
            },
            ParamSpec {
                id: "output_high",
                name: "Output High",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 100.0 },
            },
        ],
    },
    EffectSpec {
        id: "noise",
        name: "Noise",
        category: "stylize",
        params: &[
            ParamSpec {
                id: "amount",
                name: "Amount",
                kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
            },
            ParamSpec { id: "monochrome", name: "Monochrome", kind: ParamKind::Boolean { default: false } },
        ],
    },
    EffectSpec {
        id: "posterize",
        name: "Posterize",
        category: "stylize",
        params: &[ParamSpec {
            id: "steps",
            name: "Steps",
            kind: ParamKind::Popup {
                options: &[("2", 2.0), ("4", 4.0), ("8", 8.0), ("16", 16.0)],
                default: 0,
            },
        }],
    },
    EffectSpec {
        id: "sepia",
        name: "Sepia",
        category: "color",
        params: &[ParamSpec {
            id: "amount",
            name: "Amount",
            kind: ParamKind::Number { min: 0.0, max: 100.0, default: 0.0 },
        }],
    },
];

/// The **effect** an id says, if there is one.
///
/// The name is the kit's own, and the case does not matter.
pub fn spec_of(id: &str) -> Option<&'static EffectSpec> {
    let wanted = id.trim();
    CATALOGUE.iter().find(|spec| spec.id.eq_ignore_ascii_case(wanted))
}

/// The **names** of the effects there are.
fn names() -> String {
    CATALOGUE.iter().map(|spec| spec.id).collect::<Vec<_>>().join(", ")
}

/// Say what an **effect** is.
#[derive(Command)]
#[command(id = "fx.effect.list", area = "fx", layer = "fx", icon = "list")]
pub struct ListEffects {}

impl ListEffects {
    fn run(&self, _session: &mut Session) -> Result<Outcome, EngineError> {
        let effects: Vec<serde_json::Value> = CATALOGUE
            .iter()
            .map(|spec| {
                serde_json::json!({
                    "id": spec.id,
                    "name": spec.name,
                    "category": spec.category,
                    "params": spec.params.iter().map(|param| {
                        let mut value = serde_json::json!({
                            "id": param.id,
                            "name": param.name,
                            "kind": param.kind.name(),
                            "default": param.kind.default_value(),
                        });
                        match param.kind {
                            ParamKind::Number { min, max, .. } => {
                                value["min"] = serde_json::json!(min);
                                value["max"] = serde_json::json!(max);
                            }
                            ParamKind::Boolean { .. } => {}
                            ParamKind::Popup { options, .. } => {
                                value["options"] = serde_json::json!(options
                                    .iter()
                                    .map(|(label, number)| serde_json::json!({
                                        "label": label,
                                        "value": number,
                                    }))
                                    .collect::<Vec<_>>());
                            }
                        }
                        value
                    }).collect::<Vec<_>>(),
                })
            })
            .collect();
        Ok(Outcome::new(format!("there are {} effects", CATALOGUE.len()))
            .with_data(serde_json::json!({ "effects": effects })))
    }
}

/// Put an **effect** on a layer.
///
/// The instance holds the parameters of the effect at their defaults,
/// and it is a group in the layer's property tree, so that every
/// parameter is addressable like any other property.
#[derive(Command)]
#[command(id = "fx.effect.add", area = "fx", layer = "fx", icon = "plus", mutates, undoable)]
pub struct AddEffect {
    /// The layer the effect goes on.
    pub layer: NodeRef,
    /// Which effect it is: `blur`, `glow`, `levels`, `noise`,
    /// `posterize` or `sepia`.
    pub effect: Text,
}

impl AddEffect {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.effect.add".to_string();
        let layer = self.layer.0;
        let wanted = self.effect.0.clone();
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{layer}`"),
            });
        };
        if node.kind != "fx.layer" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
            });
        }
        let Some(spec) = spec_of(&wanted) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no effect `{}`: the effects are {}", wanted.trim(), names()),
            });
        };
        // The **instance**: the effect's own name and its parameters at
        // their defaults.
        let mut instance = serde_json::json!({ "effect": spec.id });
        for param in spec.params {
            instance[param.id] = param.kind.default_value();
        }
        let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        if !props.is_object() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the properties of the layer `{layer}` are not a group"),
            });
        }
        let mut effects = match props.get("effects") {
            Some(effects) if effects.is_array() => effects.as_array().cloned().unwrap_or_default(),
            Some(_) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the effects of the layer `{layer}` are not a list"),
                });
            }
            None => Vec::new(),
        };
        effects.push(instance.clone());
        let index = effects.len();
        props["effects"] = serde_json::json!(effects);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            node.data["props"] = props;
        }
        Ok(Outcome::changed(format!("the effect `{}` is added to the layer `{layer}`", spec.id)).with_data(
            serde_json::json!({
                "layer": layer,
                "effect": spec.id,
                "index": index,
                "instance": instance,
            }),
        ))
    }
}

/// Take an **effect** away from a layer.
///
/// The effect is named by its **number** in the stack, counting from
/// one, because that is how the property paths address it.
#[derive(Command)]
#[command(id = "fx.effect.remove", area = "fx", layer = "fx", icon = "trash", mutates, undoable)]
pub struct RemoveEffect {
    /// The layer the effect is on.
    pub layer: NodeRef,
    /// Which effect it is, counting from one.
    pub index: Number,
}

impl RemoveEffect {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.effect.remove".to_string();
        let layer = self.layer.0;
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{layer}`"),
            });
        };
        if node.kind != "fx.layer" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
            });
        }
        let index = self.index.0;
        if !index.is_finite() || index < 1.0 || index.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("an effect is counted from one: {index} is not one"),
            });
        }
        let index = index as usize;
        let mut props = node.data.get("props").cloned().unwrap_or(serde_json::json!({}));
        let mut effects = match props.get("effects") {
            Some(effects) if effects.is_array() => effects.as_array().cloned().unwrap_or_default(),
            Some(_) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the effects of the layer `{layer}` are not a list"),
                });
            }
            None => Vec::new(),
        };
        if index > effects.len() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no effect number {index} on the layer `{layer}`"),
            });
        }
        let removed = effects.remove(index - 1);
        props["effects"] = serde_json::json!(effects);
        if let Some(node) = session.document_mut()?.node_mut(layer) {
            node.data["props"] = props;
        }
        Ok(Outcome::changed(format!("the effect number {index} is taken away from the layer `{layer}`"))
            .with_data(serde_json::json!({
                "layer": layer,
                "index": index,
                "removed": removed,
            })))
    }
}
