//! The **model** — the surface's data model, and the one place that
//! holds it.
//!
//! # Why a lock
//!
//! The model is **shared**: the commands write it, and the painters read
//! it. craftkit has no interior-mutability type of its own that fits, so
//! the model sits behind a [`Mutex`] — and a painter **copies** what it
//! needs and lets the lock go, so that a draw never holds a lock.
//!
//! # Why it is here and not in the layer
//!
//! The **layer** owns the model's *type* (`craftkit_a2ui::model::Model`)
//! and its rules; the **plugin** owns the *instance*, because the plugin
//! is where the commands are and the commands are what write it.

use std::sync::{Mutex, OnceLock};

use craftkit_a2ui::model::Model;
use craftkit_sdk::serde_json::Value as Json;

/// The model, once and for all.
fn the_model() -> &'static Mutex<Model> {
    static MODEL: OnceLock<Mutex<Model>> = OnceLock::new();
    MODEL.get_or_init(|| Mutex::new(Model::new()))
}

/// The model, as a **snapshot**.
///
/// A painter reads the model through this, so that it never holds a lock
/// while it draws.
pub fn model() -> Json {
    match the_model().lock() {
        Ok(model) => model.root().clone(),
        // A **poisoned** lock means a thread panicked while it held the
        // model. The tree's rule is that a crash loses the work, not the
        // program, and a poisoned model is a model that is not trusted:
        // it reads as empty, and the next write replaces it.
        Err(_) => Json::Object(Default::default()),
    }
}

/// The model itself, for a command that writes it.
///
/// The lock is poisoned by a panic in another thread, and a poisoned
/// model is **replaced** — the tree's rule is that a crash loses the
/// work, not the program.
pub fn with_model<T>(write: impl FnOnce(&mut Model) -> T) -> T {
    let mut model = match the_model().lock() {
        Ok(model) => model,
        Err(poisoned) => poisoned.into_inner(),
    };
    write(&mut model)
}

/// The model's **own** instance, for the layer's readers.
pub fn resolve<T>(read: impl FnOnce(&Model) -> T) -> T {
    let model = match the_model().lock() {
        Ok(model) => model,
        Err(poisoned) => poisoned.into_inner(),
    };
    read(&model)
}
