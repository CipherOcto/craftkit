//! The seams a vector editor needs: tools, panels and layers.
//!
//! # A tool
//!
//! A tool is a **gesture that runs a command**. It is not a command: it has
//! no parameters of its own, it has an icon, and it says which command it
//! runs. The distinction matters because the front end asks "what are the
//! tools?" and "what are the commands?" and the answers are different sets.
//!
//! # A panel
//!
//! A panel is a **window's worth of commands**. It is data: an id, a label
//! and the commands it shows. The ecosystem checks that every command it
//! names exists, so a panel cannot point at a command that is not there.
//!
//! # A layer
//!
//! A layer is a group of contributions that depend on each other. The
//! ecosystem checks the graph when the program starts: every requirement has
//! to name a layer that is linked in, and the graph has to be acyclic.
//!
//! This is the answer to a tree whose layers are a `const` in a build tool:
//! a `const` can say what the tree knows at compile time, and it cannot say
//! anything about a crate that was linked in later.

use crate::contribution::Contribution;

/// A tool: a gesture that runs a command.
#[derive(Clone, Copy, Debug)]
pub struct ToolSpec {
    /// The tool's id: `<area>.<thing>`.
    pub id: &'static str,
    /// What it is called.
    pub label: &'static str,
    /// The layer that owns it.
    pub layer: &'static str,
    /// The icon's name.
    pub icon: &'static str,
    /// The chord it is advertised with.
    pub shortcut: Option<&'static str>,
    /// The command it runs.
    pub command: &'static str,
}

// The link collects tools, panels and layers here.
inventory::collect!(crate::contribution::Registration<ToolSpec>);
inventory::collect!(crate::contribution::Registration<PanelSpec>);
inventory::collect!(crate::contribution::Registration<LayerSpec>);

impl Contribution for ToolSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        "tool"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

/// A panel: a window's worth of commands.
#[derive(Clone, Copy, Debug)]
pub struct PanelSpec {
    /// The panel's id: `<area>.<thing>`.
    pub id: &'static str,
    /// What it is called.
    pub label: &'static str,
    /// The layer that owns it.
    pub layer: &'static str,
    /// The commands it shows, in order.
    pub commands: &'static [&'static str],
}

impl Contribution for PanelSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        "panel"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

/// The layer every other layer may depend on.
///
/// The bottom of the graph. It is here rather than in the application
/// because it is always true: everything can depend on the core.
pub static CORE_LAYER: LayerSpec = LayerSpec { id: "core", name: "Core", requires: &[] };

inventory::submit! {
    crate::contribution::Registration { item: &CORE_LAYER }
}

/// A layer: a group of contributions that depend on each other.
#[derive(Clone, Copy, Debug)]
pub struct LayerSpec {
    /// The layer's id.
    pub id: &'static str,
    /// Its name.
    pub name: &'static str,
    /// The layers it needs. Empty means it is at the bottom.
    pub requires: &'static [&'static str],
}

impl Contribution for LayerSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        "layer"
    }

    fn layer(&self) -> &'static str {
        self.id
    }

    /// A layer is named by one word, not by an area and a thing.
    fn malformed_id(id: &str) -> Option<String> {
        crate::contribution::malformed_word_id(id)
    }
}
