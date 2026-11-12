//! The **switches** of a layer.
//!
//! A layer has switches beside its properties: the eye that says it is
//! drawn, the speaker that says it is heard, the solo that says it is
//! alone, the lock that says it may not be edited, and the shy that
//! says it steps aside.
//!
//! The tree's finding is that a switch is a **fact about the layer**
//! and the **rules** that read the facts are of their own:
//!
//! - **A solo silences the others.** While any layer is soloed, the
//!   other layers are not drawn and not heard — the tree's renderer
//!   draws the soloed layers alone.
//! - **A shy layer steps aside when the composition says so.** The
//!   tree's composition has a *hide shy layers* switch, and a shy
//!   layer in such a composition is not picked and says nothing of
//!   its own keys.
//! - **A locked layer is not edited.** The tree's commands leave a
//!   locked layer alone, and refuse when every layer of a selection
//!   is locked.
//!
//! So the switches are stored and the rules are **derived**: the
//! evaluator says which layers are drawn, which are heard and which
//! may be edited, and it says it from the switches as they stand.
//!
//! The tree's switches that are **not** booleans — the quality and
//! the frame blend — are not here: a switch that is a word and not a
//! yes or no is a different thing, and there are enough of the others.

use craftkit_derive::{Command, Evaluator};

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, Document, EngineError, Node, NodeRef, Outcome, Plan, PlanItem, Session, Text};

/// The **switches** of a layer.
///
/// The tree's own list, and the tree's own defaults: the eye and the
/// speaker are on, the effects are on, and the rest are off.
#[derive(Debug, Clone, PartialEq)]
pub struct Switches {
    /// Whether the layer is **drawn**.
    pub video: bool,
    /// Whether the layer is **heard**.
    pub audio: bool,
    /// Whether the layer is **alone**: while any layer is soloed, the
    /// others are not drawn and not heard.
    pub solo: bool,
    /// Whether the layer is **locked**: it is not edited.
    pub locked: bool,
    /// Whether the layer is **shy**: it steps aside when the
    /// composition hides the shy layers.
    pub shy: bool,
    /// Whether the layer's **transformations are collapsed**.
    pub collapse: bool,
    /// Whether the layer's **effects** run.
    pub effects: bool,
    /// Whether the layer is **blurred** by the motion blur.
    pub motion_blur: bool,
    /// Whether the layer is an **adjustment** layer.
    pub adjustment: bool,
    /// Whether the layer is in **three dimensions**.
    pub three_d: bool,
    /// Whether the layer is a **guide**.
    pub guide: bool,
    /// Whether the layer **keeps** its transparency.
    pub preserve_transparency: bool,
}

impl Default for Switches {
    fn default() -> Self {
        Switches {
            video: true,
            audio: true,
            solo: false,
            locked: false,
            shy: false,
            collapse: false,
            effects: true,
            motion_blur: false,
            adjustment: false,
            three_d: false,
            guide: false,
            preserve_transparency: false,
        }
    }
}

impl Switches {
    /// The switches as they are written.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "video": self.video,
            "audio": self.audio,
            "solo": self.solo,
            "locked": self.locked,
            "shy": self.shy,
            "collapse": self.collapse,
            "effects": self.effects,
            "motion_blur": self.motion_blur,
            "adjustment": self.adjustment,
            "three_d": self.three_d,
            "guide": self.guide,
            "preserve_transparency": self.preserve_transparency,
        })
    }

    /// The switches as they are read.
    pub fn from_json(value: &serde_json::Value) -> Switches {
        let read = |name: &str| value.get(name).and_then(|flag| flag.as_bool());
        let mut switches = Switches::default();
        if let Some(flag) = read("video") {
            switches.video = flag;
        }
        if let Some(flag) = read("audio") {
            switches.audio = flag;
        }
        if let Some(flag) = read("solo") {
            switches.solo = flag;
        }
        if let Some(flag) = read("locked") {
            switches.locked = flag;
        }
        if let Some(flag) = read("shy") {
            switches.shy = flag;
        }
        if let Some(flag) = read("collapse") {
            switches.collapse = flag;
        }
        if let Some(flag) = read("effects") {
            switches.effects = flag;
        }
        if let Some(flag) = read("motion_blur") {
            switches.motion_blur = flag;
        }
        if let Some(flag) = read("adjustment") {
            switches.adjustment = flag;
        }
        if let Some(flag) = read("three_d") {
            switches.three_d = flag;
        }
        if let Some(flag) = read("guide") {
            switches.guide = flag;
        }
        if let Some(flag) = read("preserve_transparency") {
            switches.preserve_transparency = flag;
        }
        switches
    }

    /// The **switch** with this name, if there is one.
    ///
    /// The names are the tree's own: the eye is the video switch, the
    /// lock is the locked switch, and the three dimensions are the
    /// three-dimensional one.
    pub fn get(&self, name: &str) -> Option<bool> {
        Some(match name {
            "video" | "eye" => self.video,
            "audio" => self.audio,
            "solo" => self.solo,
            "lock" | "locked" => self.locked,
            "shy" => self.shy,
            "collapse" => self.collapse,
            "effects" | "fx" => self.effects,
            "motion_blur" | "motionBlur" => self.motion_blur,
            "adjustment" => self.adjustment,
            "three_d" | "threeD" | "3d" | "3D" => self.three_d,
            "guide" => self.guide,
            "preserve_transparency" | "preserveTransparency" => self.preserve_transparency,
            _ => return None,
        })
    }

    /// Put the **switch** with this name on or off.
    ///
    /// The names are the tree's own, and an unknown name is refused.
    pub fn set(&mut self, name: &str, value: bool) -> Result<(), String> {
        match name {
            "video" | "eye" => self.video = value,
            "audio" => self.audio = value,
            "solo" => self.solo = value,
            "lock" | "locked" => self.locked = value,
            "shy" => self.shy = value,
            "collapse" => self.collapse = value,
            "effects" | "fx" => self.effects = value,
            "motion_blur" | "motionBlur" => self.motion_blur = value,
            "adjustment" => self.adjustment = value,
            "three_d" | "threeD" | "3d" | "3D" => self.three_d = value,
            "guide" => self.guide = value,
            "preserve_transparency" | "preserveTransparency" => self.preserve_transparency = value,
            _ => return Err(format!("there is no switch `{name}`")),
        }
        Ok(())
    }
}

/// The **names** of the switches, for the messages that refuse one.
pub const SWITCH_NAMES: [&str; 12] = [
    "video",
    "audio",
    "solo",
    "lock",
    "shy",
    "collapse",
    "effects",
    "motion_blur",
    "adjustment",
    "three_d",
    "guide",
    "preserve_transparency",
];

/// The switches of a **layer**, as the document holds them.
pub fn switches_of(node: &Node) -> Switches {
    match node.data.get("switches") {
        Some(value) => Switches::from_json(value),
        None => Switches::default(),
    }
}

/// Put a **switch** on a layer, or off.
///
/// A switch with no value is **toggled**: the tree's command toggles
/// a switch that is asked for without a value, and a switch that is
/// asked for with a value is set.
///
/// A name that is not a switch is **refused**: the tree's command
/// refuses an unknown switch, and a switch that does not exist cannot
/// be turned on.
#[derive(Command)]
#[command(id = "fx.layer.switch", area = "fx", layer = "fx", icon = "eye", mutates, undoable)]
pub struct SetSwitch {
    /// The layer the switch is on.
    pub layer: NodeRef,
    /// The switch: `video`, `audio`, `solo`, `lock`, `shy`, …
    pub switch: Text,
    /// Whether it is on. Without it, the switch is toggled.
    pub value: Option<Bool>,
}

impl SetSwitch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.layer.switch".to_string();
        let layer = self.layer.0;
        let name = self.switch.0.clone();
        let Some(node) = session.document().node(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        if node.kind != "fx.layer" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
            });
        }
        let mut switches = switches_of(node);
        let current = switches.get(&name).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no switch `{name}`: the switches are {}", SWITCH_NAMES.join(", ")),
        })?;
        let value = self.value.map(|value| value.0).unwrap_or(!current);
        switches.set(&name, value).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let Some(node) = session.document_mut()?.node_mut(layer) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no layer `{layer}`"),
            });
        };
        node.data["switches"] = switches.to_json();
        Ok(Outcome::changed(format!("the switch `{name}` of the layer `{layer}` is {value}"))
            .with_data(serde_json::json!({ "layer": layer, "switch": name, "value": value })))
    }
}

/// Put the composition's **hide shy layers** switch on or off.
///
/// The tree's composition has one switch that is about the layers
/// rather than about the composition, and it is the one that says the
/// shy layers step aside: they are not picked and their keys are not
/// stepped to.
#[derive(Command)]
#[command(id = "fx.comp.switch", area = "fx", layer = "fx", icon = "eye", mutates, undoable)]
pub struct SetCompSwitch {
    /// The composition.
    pub comp: NodeRef,
    /// The switch: `hide_shy`.
    pub switch: Text,
    /// Whether it is on. Without it, the switch is toggled.
    pub value: Option<Bool>,
}

impl SetCompSwitch {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.comp.switch".to_string();
        let comp = self.comp.0;
        let name = self.switch.0.clone();
        let Some(node) = session.document().node(comp) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no composition `{comp}`"),
            });
        };
        if node.kind != "fx.comp" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{comp}` is a `{}` and not a composition", node.kind),
            });
        }
        let current = match name.as_str() {
            "hide_shy" | "hideShy" | "shy" => {
                node.data.get("hide_shy").and_then(|hide| hide.as_bool()).unwrap_or(false)
            }
            _ => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no switch `{name}`: the switch of a composition is hide_shy"),
                });
            }
        };
        let value = self.value.map(|value| value.0).unwrap_or(!current);
        let Some(node) = session.document_mut()?.node_mut(comp) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no composition `{comp}`"),
            });
        };
        node.data["hide_shy"] = serde_json::Value::Bool(value);
        Ok(Outcome::changed(format!("the switch `{name}` of the composition `{comp}` is {value}"))
            .with_data(serde_json::json!({ "comp": comp, "switch": name, "value": value })))
    }
}

/// The evaluator that says what the switches **do**.
///
/// What is drawn, what is heard and what may be edited are **derived**
/// from the switches: they change when a switch is turned, and nobody
/// writes them down.
#[derive(Evaluator)]
#[evaluator(id = "fx.switches", label = "Switches", layer = "fx")]
pub struct SwitchesEvaluator;

impl SwitchesEvaluator {
    /// Say what each layer's switches do, in its composition.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for comp in document.walk().filter(|node| node.kind == "fx.comp") {
            let hide_shy = comp.data.get("hide_shy").and_then(|hide| hide.as_bool()).unwrap_or(false);
            let layers: Vec<&Node> =
                comp.children.iter().filter(|child| child.kind == "fx.layer").map(|child| &**child).collect();
            // A **solo** silences the others: while any layer is soloed,
            // the tree's renderer draws the soloed layers alone.
            let any_solo = layers.iter().any(|layer| switches_of(layer).solo);
            for layer in &layers {
                let switches = switches_of(layer);
                // A **shy** layer in a composition that hides the shy
                // layers is not picked, and is not drawn by itself.
                let shy = hide_shy && switches.shy;
                let drawn = switches.video && !shy && (!any_solo || switches.solo);
                let heard = switches.audio && (!any_solo || switches.solo);
                let editable = !switches.locked && !shy;
                // The **effects** that run: the ones that are enabled
                // and the layer's own effects switch is on.
                let effects = crate::effects::running_effects(layer, switches.effects);
                plan.items.push(PlanItem {
                    node: layer.id,
                    kind: "fx.switches".to_string(),
                    value: serde_json::json!({
                        "switches": switches.to_json(),
                        "drawn": drawn,
                        "heard": heard,
                        "editable": editable,
                        "any_solo": any_solo,
                        "effects": effects,
                        "hide_shy": hide_shy,
                    }),
                });
            }
        }
        plan
    }
}
