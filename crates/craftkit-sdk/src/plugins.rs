//! The **plugin libraries** a session has open.
//!
//! A plugin that is not part of the program is a **shared library**
//! (see [`craftkit_ffi`]). A session may have some of them open, and it
//! is the session that holds them: they are **session state**, like the
//! camera and the tasks, and the undo does not touch them.
//!
//! # Why the session holds them
//!
//! A library is a handle to a file on disk, and a handle is worth
//! keeping: a plugin may hold state, and opening it again is work. So
//! the session keeps the libraries it has opened, and a command can ask
//! one of them what it has.
//!
//! The libraries are **not** in the document. A document that named the
//! libraries it needed would be a document that only worked on the
//! machine that had them, and the bundle is the wrong container for
//! them: they are programs, not files the document owns.

use std::sync::Arc;

use craftkit_ffi::Library;

/// A library that is open.
#[derive(Debug, Clone)]
pub struct OpenLibrary {
    /// Where it was opened from.
    pub path: String,
    /// The library itself.
    pub library: Arc<Library>,
}

/// What a plugin says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    /// Its name.
    pub name: String,
    /// Its parameters, as JSON.
    pub params: String,
}

/// The libraries a session has open.
///
/// The numbers are the session's own: the first library opened is
/// library 1.
#[derive(Debug, Clone, Default)]
pub struct PluginLibraries {
    /// The open libraries.
    open: Vec<OpenLibrary>,
}

impl PluginLibraries {
    /// A session with no libraries open.
    pub fn new() -> PluginLibraries {
        PluginLibraries { open: Vec::new() }
    }

    /// Open a library, and give back its number.
    ///
    /// A library that does not open is an error and nothing is kept.
    pub fn open(&mut self, path: &str) -> Result<u64, String> {
        let library = Library::open(path).map_err(|error| error.message)?;
        self.open.push(OpenLibrary { path: path.to_string(), library: Arc::new(library) });
        Ok(self.open.len() as u64)
    }

    /// The library with that number.
    pub fn get(&self, id: u64) -> Option<&OpenLibrary> {
        if id == 0 {
            return None;
        }
        self.open.get((id - 1) as usize)
    }

    /// The plugins inside that library.
    pub fn plugins(&self, id: u64) -> Result<Vec<PluginInfo>, String> {
        let open = self.get(id).ok_or_else(|| format!("there is no library {id}"))?;
        let plugins = open.library.plugins().map_err(|error| error.message)?;
        let mut found = Vec::new();
        for index in 0..plugins.count() {
            let name = plugins.name(index).map_err(|error| error.message)?;
            let params = plugins.params(index).map_err(|error| error.message)?;
            found.push(PluginInfo { name, params });
        }
        Ok(found)
    }

    /// Close a library.
    ///
    /// Closing it twice is refused, and so is closing one that was never
    /// opened.
    pub fn close(&mut self, id: u64) -> Result<(), String> {
        if id == 0 || id as usize > self.open.len() {
            return Err(format!("there is no library {id}"));
        }
        self.open.remove((id - 1) as usize);
        Ok(())
    }

    /// How many libraries are open.
    pub fn len(&self) -> usize {
        self.open.len()
    }

    /// Whether any library is open.
    pub fn is_empty(&self) -> bool {
        self.open.is_empty()
    }
}
