//! The ecosystem: every contribution, checked together, once.
//!
//! # Why one check and not five
//!
//! Each registry checks its own ids. But some mistakes are only visible when
//! the registries are read **together**:
//!
//! - a tool names a command that is not there;
//! - a panel names a command that is not there;
//! - a contribution is in a layer that was not linked in;
//! - a layer requires a layer that is not there, or requires itself, or
//!   the requirements have a cycle.
//!
//! None of these belongs to one registry, so none of them can be checked by
//! one registry. They are checked here, **when the ecosystem is built** —
//! which is before anybody has drawn anything.
//!
//! # What it refuses to do
//!
//! It does not fix anything. A missing command is a refusal, not a warning:
//! a program that starts with a tool that cannot run is a program that will
//! fail later, in front of a person, instead of now, in front of a log.

use crate::command::CommandSpec;
use crate::contribution::{Contribution, Registry, RegistryError};
use crate::document::NodeKindSpec;
use crate::seams::{LayerSpec, PanelSpec, ToolSpec};
use crate::session::Session;

/// Why the ecosystem could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EcosystemError {
    /// A registry is wrong on its own.
    Registry(RegistryError),
    /// A contribution is in a layer that is not linked in.
    MissingLayer { id: String, layer: String },
    /// A layer requires a layer that is not linked in.
    MissingRequirement { layer: String, requires: String },
    /// A layer requires itself.
    SelfRequirement(String),
    /// The layers require each other in a circle.
    Cycle(Vec<String>),
    /// A tool names a command that is not there.
    MissingCommand { tool: String, command: String },
    /// A panel names a command that is not there.
    MissingPanelCommand { panel: String, command: String },
}

impl std::fmt::Display for EcosystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EcosystemError::Registry(error) => write!(f, "{error}"),
            EcosystemError::MissingLayer { id, layer } => {
                write!(f, "`{id}` is in the `{layer}` layer, which is not linked in")
            }
            EcosystemError::MissingRequirement { layer, requires } => {
                write!(f, "the `{layer}` layer requires `{requires}`, which is not linked in")
            }
            EcosystemError::SelfRequirement(layer) => {
                write!(f, "the `{layer}` layer requires itself")
            }
            EcosystemError::Cycle(layers) => {
                write!(f, "the layers require each other in a circle: {}", layers.join(" → "))
            }
            EcosystemError::MissingCommand { tool, command } => {
                write!(f, "the tool `{tool}` runs `{command}`, which is not linked in")
            }
            EcosystemError::MissingPanelCommand { panel, command } => {
                write!(f, "the panel `{panel}` shows `{command}`, which is not linked in")
            }
        }
    }
}

impl std::error::Error for EcosystemError {}

impl From<RegistryError> for EcosystemError {
    fn from(error: RegistryError) -> Self {
        EcosystemError::Registry(error)
    }
}

/// Everything the program was linked with.
///
/// One value, holding every registry, and the checks that only make sense
/// across them.
#[derive(Debug, Clone)]
pub struct Ecosystem {
    /// The layers.
    pub layers: Registry<LayerSpec>,
    /// The commands.
    pub commands: Registry<CommandSpec>,
    /// The kinds of node.
    pub kinds: Registry<NodeKindSpec>,
    /// The tools.
    pub tools: Registry<ToolSpec>,
    /// The panels.
    pub panels: Registry<PanelSpec>,
}

impl Ecosystem {
    /// An empty ecosystem.
    pub fn new() -> Self {
        Ecosystem {
            layers: Registry::new(),
            commands: Registry::new(),
            kinds: Registry::new(),
            tools: Registry::new(),
            panels: Registry::new(),
        }
    }

    /// The ecosystem of everything the binary was linked with.
    ///
    /// This is where a plugin arrives: not by being named, but by being
    /// there.
    pub fn from_inventory() -> Result<Self, EcosystemError> {
        let ecosystem = Ecosystem {
            layers: Registry::from_inventory()?,
            commands: Registry::from_inventory()?,
            kinds: Registry::from_inventory()?,
            tools: Registry::from_inventory()?,
            panels: Registry::from_inventory()?,
        };
        ecosystem.validate()?;
        Ok(ecosystem)
    }

    /// Check everything that only makes sense across the registries.
    pub fn validate(&self) -> Result<(), EcosystemError> {
        self.validate_layers()?;
        self.validate_layers_of_contributions()?;
        self.validate_references()?;
        Ok(())
    }

    /// Every requirement names a layer that exists, and there is no cycle.
    fn validate_layers(&self) -> Result<(), EcosystemError> {
        for layer in self.layers.all() {
            for requires in layer.requires {
                if *requires == layer.id {
                    return Err(EcosystemError::SelfRequirement(layer.id.to_string()));
                }
                if self.layers.get(requires).is_none() {
                    return Err(EcosystemError::MissingRequirement {
                        layer: layer.id.to_string(),
                        requires: (*requires).to_string(),
                    });
                }
            }
        }
        // The cycle check: a walk from every layer.
        let mut done: Vec<&'static str> = Vec::new();
        for layer in self.layers.all() {
            let mut path: Vec<&'static str> = Vec::new();
            self.visit(layer.id, &mut path, &mut done)?;
        }
        Ok(())
    }

    fn visit(
        &self,
        id: &'static str,
        path: &mut Vec<&'static str>,
        done: &mut Vec<&'static str>,
    ) -> Result<(), EcosystemError> {
        if done.contains(&id) {
            return Ok(());
        }
        if path.contains(&id) {
            let mut cycle = path.clone();
            cycle.push(id);
            return Err(EcosystemError::Cycle(cycle.iter().map(|part| (*part).to_string()).collect()));
        }
        path.push(id);
        if let Some(layer) = self.layers.get(id) {
            for requires in layer.requires {
                self.visit(requires, path, done)?;
            }
        }
        path.pop();
        done.push(id);
        Ok(())
    }

    /// Every contribution is in a layer that is linked in.
    fn validate_layers_of_contributions(&self) -> Result<(), EcosystemError> {
        check_layers(&self.commands, &self.layers)?;
        check_layers(&self.kinds, &self.layers)?;
        check_layers(&self.tools, &self.layers)?;
        check_layers(&self.panels, &self.layers)?;
        Ok(())
    }

    /// Every command a tool or a panel names is there.
    fn validate_references(&self) -> Result<(), EcosystemError> {
        for tool in self.tools.all() {
            if self.commands.get(tool.command).is_none() {
                return Err(EcosystemError::MissingCommand {
                    tool: tool.id.to_string(),
                    command: tool.command.to_string(),
                });
            }
        }
        for panel in self.panels.all() {
            for command in panel.commands {
                if self.commands.get(command).is_none() {
                    return Err(EcosystemError::MissingPanelCommand {
                        panel: panel.id.to_string(),
                        command: (*command).to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// A session over this ecosystem.
    pub fn session(&self) -> Session {
        Session::new(self.commands.clone(), self.kinds.clone())
    }

    /// The views of this ecosystem.
    pub fn projection(&self) -> Projection<'_> {
        Projection { ecosystem: self }
    }
}

impl Default for Ecosystem {
    fn default() -> Self {
        Ecosystem::new()
    }
}

/// Every contribution in a registry is in a layer that exists.
///
/// One generic function for every kind of contribution: that is the point of
/// the one registry.
fn check_layers<T: Contribution>(
    registry: &Registry<T>,
    layers: &Registry<LayerSpec>,
) -> Result<(), EcosystemError> {
    for item in registry.all() {
        let layer = item.layer();
        if layers.get(layer).is_none() {
            return Err(EcosystemError::MissingLayer { id: item.id().to_string(), layer: layer.to_string() });
        }
    }
    Ok(())
}

/// The views of an ecosystem: what a front end shows and what an agent is
/// told.
///
/// **One source.** A menu, a toolbar, a schema, a chord map and a document
/// are five views of the same registry, and this is where they are made, so
/// that a command added by a plugin appears in all five without anybody
/// writing it down five times.
#[derive(Debug, Clone, Copy)]
pub struct Projection<'a> {
    ecosystem: &'a Ecosystem,
}

/// One line of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuEntry {
    /// The area it is under.
    pub area: &'static str,
    /// The command's id.
    pub id: &'static str,
    /// What it says.
    pub label: &'static str,
    /// One line about what it does.
    pub doc: &'static str,
    /// What it is for.
    pub capability: &'static str,
    /// Its chord, if it has one.
    pub shortcut: Option<&'static str>,
}

impl<'a> Projection<'a> {
    /// The commands, grouped into a menu by area.
    ///
    /// Only the capabilities asked for: a toolbar wants `tool` and
    /// `command`, and a file menu wants `exporter` and `importer`.
    pub fn menu(&self, capabilities: &[&str]) -> Vec<MenuEntry> {
        let mut entries: Vec<MenuEntry> = Vec::new();
        for command in self.ecosystem.commands.all() {
            if !capabilities.contains(&command.capability) {
                continue;
            }
            entries.push(MenuEntry {
                area: command.area,
                id: command.id,
                label: command.label,
                doc: command.doc,
                capability: command.capability,
                shortcut: command.shortcut,
            });
        }
        entries.sort_by(|left, right| left.area.cmp(right.area).then_with(|| left.id.cmp(right.id)));
        entries
    }

    /// The tools, in order.
    pub fn tools(&self) -> Vec<&ToolSpec> {
        self.ecosystem.tools.all().iter().collect()
    }

    /// The panels.
    pub fn panels(&self) -> Vec<&PanelSpec> {
        self.ecosystem.panels.all().iter().collect()
    }

    /// The node kinds.
    pub fn kinds(&self) -> Vec<&NodeKindSpec> {
        self.ecosystem.kinds.all().iter().collect()
    }

    /// The schema of a command: its parameters, as an agent is told them.
    pub fn schema(&self, id: &str) -> Option<serde_json::Value> {
        let command = self.ecosystem.commands.get(id)?;
        let parameters: Vec<serde_json::Value> = command
            .params
            .iter()
            .map(|param| {
                serde_json::json!({
                    "name": param.name,
                    "kind": param.kind.json_name(),
                    "required": param.required,
                    "help": param.help,
                    "alternative": param.alternative,
                })
            })
            .collect();
        Some(serde_json::json!({
            "id": command.id,
            "label": command.label,
            "area": command.area,
            "capability": command.capability,
            "doc": command.doc,
            "parameters": parameters,
        }))
    }

    /// Every contribution, one line each: the manifest.
    ///
    /// This is what an agent is told about the program it is driving.
    pub fn manifest(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for command in self.ecosystem.commands.all() {
            lines.push(format!(
                "command {} ({}) in {} — {}",
                command.id, command.capability, command.layer, command.doc
            ));
        }
        for kind in self.ecosystem.kinds.all() {
            lines.push(format!("node kind {} in {} — {}", kind.kind, kind.layer, kind.label));
        }
        for tool in self.ecosystem.tools.all() {
            lines.push(format!("tool {} in {} — runs {}", tool.id, tool.layer, tool.command));
        }
        for panel in self.ecosystem.panels.all() {
            lines.push(format!(
                "panel {} in {} — {} command(s)",
                panel.id,
                panel.layer,
                panel.commands.len()
            ));
        }
        lines.sort();
        lines
    }
}
