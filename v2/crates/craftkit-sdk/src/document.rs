//! The document: what a command changes.
//!
//! Small on purpose. v2 is not a modelling project — it is about **where the
//! behaviour comes from**. The document here is a stand-in with the two
//! things a command needs to be interesting: something to name, and a list
//! of nodes that can be muted.

/// A node in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Its id.
    pub id: u64,
    /// Its name.
    pub name: String,
    /// Whether it is muted.
    pub muted: bool,
}

/// The document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The document's name.
    pub name: String,
    /// Its nodes.
    pub nodes: Vec<Node>,
}

impl Document {
    /// An empty document with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Document { name: name.into(), nodes: Vec::new() }
    }

    /// The next free id.
    pub fn next_id(&self) -> u64 {
        self.nodes.iter().map(|node| node.id).max().unwrap_or(0) + 1
    }

    /// Add a node, and give back its id.
    pub fn add_node(&mut self, name: impl Into<String>) -> u64 {
        let id = self.next_id();
        self.nodes.push(Node { id, name: name.into(), muted: false });
        id
    }

    /// The node with an id.
    pub fn node(&self, id: u64) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    /// The node with an id, to change.
    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    /// The node with a name.
    pub fn node_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.name == name)
    }
}
