//! The commands for the **plugin libraries**: shared libraries a session
//! has open.
//!
//! The door itself is in [`craftkit_ffi`]; this is the part of it that a
//! person, an agent and a script reach, so that the registry is the one
//! way in — as it is for everything else.

use craftkit_derive::Command;

use crate::{EngineError, Number, Outcome, Session, Text};

/// The number of a library.
///
/// It is a number and not a node reference: a library is not in the
/// document, so it has no node.
fn library_id(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a library is numbered with a whole number".to_string(),
        });
    }
    Ok(value.0 as u64)
}

/// Open a **plugin library**: a shared library with a plugin table in it.
#[derive(Command)]
#[command(id = "plugin.library.open", area = "plugin", icon = "plus", menu("Plugin"))]
pub struct OpenLibrary {
    /// Where the library is.
    pub path: Text,
}

impl OpenLibrary {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: "plugin.library.open".to_string(),
                message: "a library has a path".to_string(),
            });
        }
        let id = session
            .plugins_mut()
            .open(&path)
            .map_err(|message| EngineError::Failed { id: "plugin.library.open".to_string(), message })?;
        Ok(Outcome::changed(format!("opened the library `{path}` as library {id}"))
            .with_data(crate::serde_json::json!({ "library": id, "path": path })))
    }
}

/// The **plugins** in the open libraries.
#[derive(Command)]
#[command(id = "plugin.library.list", area = "plugin", icon = "list", menu("Plugin"))]
pub struct ListLibraries {
    /// Only this library. Every open library, if it is not given.
    pub library: Option<Number>,
}

impl ListLibraries {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let wanted = match &self.library {
            Some(library) => Some(library_id(*library, "plugin.library.list")?),
            None => None,
        };
        let libraries = session.plugins();
        let mut found = Vec::new();
        for id in 1..=libraries.len() as u64 {
            if wanted.is_some_and(|wanted| wanted != id) {
                continue;
            }
            let Some(open) = libraries.get(id) else { continue };
            let plugins = libraries
                .plugins(id)
                .map_err(|message| EngineError::Failed { id: "plugin.library.list".to_string(), message })?;
            found.push(crate::serde_json::json!({
                "library": id,
                "path": open.path,
                "plugins": plugins
                    .into_iter()
                    .map(|plugin| crate::serde_json::json!({
                        "name": plugin.name,
                        "params": plugin.params,
                    }))
                    .collect::<Vec<_>>(),
            }));
        }
        let count = found.len();
        Ok(Outcome::new(format!("{count} librar(ies)"))
            .with_data(crate::serde_json::json!({ "libraries": found })))
    }
}

/// Close a **plugin library**.
#[derive(Command)]
#[command(id = "plugin.library.close", area = "plugin", icon = "minus", menu("Plugin"))]
pub struct CloseLibrary {
    /// The library.
    pub library: Number,
}

impl CloseLibrary {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = library_id(self.library, "plugin.library.close")?;
        session
            .plugins_mut()
            .close(id)
            .map_err(|message| EngineError::Failed { id: "plugin.library.close".to_string(), message })?;
        Ok(Outcome::changed(format!("closed the library {id}")))
    }
}
