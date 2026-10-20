//! Where a browser build keeps a plan between visits.
//!
//! This is the whole of the browser-specific surface: three operations over one
//! string. Keeping it that small is what lets the logic be tested on a host,
//! where there is no `localStorage` at all.

/// The one key the plan lives under.
///
/// Fixed, not generated: a plan has no session, and a key that changed per tab
/// would make "come back to my plan" impossible. The `/` keeps it out of the way
/// of anything else on the origin.
pub const STASH_KEY: &str = "craftkit/plan";

/// The element `index.html` provides for `eframe` to draw into.
///
/// Browser-only, so a host build does not carry a constant that means nothing
/// to it.
#[cfg(target_arch = "wasm32")]
pub const CANVAS_ID: &str = "craftkit-canvas";

/// Somewhere to keep a plan as text.
///
/// A trait rather than a direct `localStorage` call for two reasons: the panel
/// that uses it can then be exercised on the host, and the host's version can
/// *fail* instead of pretending, which is what keeps the panel honest.
pub trait Stash {
    /// The text under `key`, if there is any.
    fn get(&self, key: &str) -> Option<String>;
    /// Store `value` under `key`, or say why not.
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    /// Forget `key`.
    fn remove(&self, key: &str);
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

/// The real thing: the origin's `localStorage`.
///
/// `None` here is not a crash — storage is disabled in a private window, blocked
/// by a policy, or absent in a sandboxed frame. Every call reports that rather
/// than pretending to have saved something.
#[cfg(target_arch = "wasm32")]
pub struct Browser;

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}

#[cfg(target_arch = "wasm32")]
impl Stash for Browser {
    fn get(&self, key: &str) -> Option<String> {
        match storage() {
            Some(store) => match store.get_item(key) {
                Ok(text) => text,
                Err(e) => {
                    log::warn!("localStorage refused a read of {key}: {e:?}");
                    None
                }
            },
            None => {
                log::warn!("this browser has no localStorage; nothing to restore");
                None
            }
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let store = storage().ok_or_else(|| "this browser has no localStorage to save into".to_string())?;
        store
            .set_item(key, value)
            .map_err(|e| format!("the browser refused to save ({e:?}); the plan is only on screen"))
    }

    fn remove(&self, key: &str) {
        match storage() {
            Some(store) => {
                if let Err(e) = store.remove_item(key) {
                    log::warn!("localStorage refused to forget {key}: {e:?}");
                }
            }
            None => log::warn!("this browser has no localStorage; there was nothing to forget"),
        }
    }
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

/// The host has no stash.
///
/// A unit type that reports failure for every operation, so a host run of the
/// panel code — or a future headless use of it — cannot quietly lose a document
/// by writing it nowhere.
///
/// Host-only: the browser has [`Browser`], and a wasm build carrying a type
/// that can only fail would be dead weight.
#[cfg(not(target_arch = "wasm32"))]
pub struct Nowhere;

#[cfg(not(target_arch = "wasm32"))]
impl Stash for Nowhere {
    fn get(&self, _key: &str) -> Option<String> {
        None
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), String> {
        Err("this build has nowhere to save; a browser build does".to_string())
    }

    fn remove(&self, _key: &str) {}
}

/// An in-process stash, for tests.
///
/// `Rc<RefCell<..>>` rather than a plain map so a test can hold one half and hand
/// the other to the code under test. Tests run on one thread.
#[cfg(test)]
pub struct Memory(pub std::rc::Rc<std::cell::RefCell<std::collections::BTreeMap<String, String>>>);

#[cfg(test)]
impl Stash for Memory {
    fn get(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.borrow_mut().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn remove(&self, key: &str) {
        self.0.borrow_mut().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_survives_a_write_a_read_and_a_forget() {
        let map = std::rc::Rc::new(std::cell::RefCell::new(std::collections::BTreeMap::new()));
        let store = Memory(map);
        assert!(store.get(STASH_KEY).is_none());
        store.set(STASH_KEY, "hello").expect("writes");
        assert_eq!(store.get(STASH_KEY).as_deref(), Some("hello"));
        store.remove(STASH_KEY);
        assert!(store.get(STASH_KEY).is_none());
    }

    #[test]
    fn the_host_stash_says_it_has_nowhere_to_save() {
        // A silent success here is the worst possible bug: the user clicks
        // Save, sees no error, and loses the plan.
        let err = Nowhere.set(STASH_KEY, "x").expect_err("reports");
        assert!(err.contains("nowhere"), "{err}");
        assert!(Nowhere.get(STASH_KEY).is_none());
    }

    #[test]
    fn forgetting_something_that_is_not_there_is_not_an_error() {
        let store = Nowhere;
        store.remove(STASH_KEY);
    }
}
