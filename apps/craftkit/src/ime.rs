//! The text the platform's own keyboard commits, and the way into the frame.
//!
//! The platform's IME is a **gate**: it is off by default, and the tree's
//! own keyboard is the default (§246). Behind the gate, the platform commits
//! text through an input connection of the tree's own — and the commit
//! arrives on the platform's thread, not on the frame's.
//!
//! So the text is put in a queue here, and taken out on the frame that
//! draws. **This is the whole of the bridge on this side, and it is safe**:
//! the unsafe half is in the package, where the tree's one named exception
//! already lives (§236, §257).
//!
//! The queue is bounded: a commit that nobody reads is a commit nobody
//! typed, and the platform may commit faster than the tree draws.

use std::sync::Mutex;

/// The one that asks the platform for its keyboard.
///
/// The tree's frame says whether a field has the focus, and **the platform's
/// half** is the one that can ask the platform: the view the platform's
/// keyboard talks to is Java's (§257). So the application registers who to
/// ask, and the package registers the Java half.
type Asker = Box<dyn Fn(bool) + Send + Sync>;

/// Who asks the platform for its keyboard.
static ASKER: Mutex<Option<Asker>> = Mutex::new(None);

/// Say who asks the platform for its keyboard.
///
/// The package registers the Java half here, at startup (§257).
pub fn set_asker(asker: Asker) {
    let Ok(mut slot) = ASKER.lock() else { return };
    *slot = Some(asker);
}

/// The last thing the tree asked for.
///
/// The tree asks on **every frame** — the frame is what knows whether a field
/// has the focus — and the ask is a call across the bridge and into the
/// platform, which is not something to do sixty times a second for the same
/// answer. So the ask is made when the answer **changes**, and not before.
static LAST_ASKED: Mutex<Option<bool>> = Mutex::new(None);

/// Ask the platform for its keyboard, or tell it that it is not wanted.
///
/// The ask is made only when it changes: the tree asks on every frame, and
/// the platform's half of the bridge is a call into another language (§257).
pub fn want_keyboard(wanted: bool) {
    {
        let Ok(mut last) = LAST_ASKED.lock() else { return };
        if *last == Some(wanted) {
            return;
        }
        *last = Some(wanted);
    }
    let Ok(slot) = ASKER.lock() else { return };
    let Some(asker) = slot.as_ref() else { return };
    asker(wanted);
}

/// How many committed texts may wait for the next frame.
pub const MAX_PENDING: usize = 64;

/// The one that asks for a frame.
///
/// The commit arrives on the platform's thread, and the tree's frame takes
/// the text out on the next frame — but a window only draws when something
/// asks it to. **A commit that waits for a frame that never comes is a
/// keystroke nobody sees**: measured, the platform's text arrived and the
/// tree never drew it, because nothing had asked for a frame (§257).
type Repainter = Box<dyn Fn() + Send + Sync>;

/// Who asks for a frame when the platform commits something.
static REPAINTER: Mutex<Option<Repainter>> = Mutex::new(None);

/// Say who asks for a frame when the platform commits something.
///
/// The application registers it at startup: it is the one that holds the
/// window (§257).
pub fn set_repainter(repainter: Repainter) {
    let Ok(mut slot) = REPAINTER.lock() else { return };
    *slot = Some(repainter);
}

/// Ask for a frame, if anyone is there to ask.
fn repaint() {
    let Ok(slot) = REPAINTER.lock() else { return };
    let Some(repainter) = slot.as_ref() else { return };
    repainter();
}

/// The text the platform committed, waiting for the next frame.
static COMMITTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Put a committed text in the queue.
///
/// The platform's half of the bridge calls this. A poisoned lock means a
/// frame panicked while holding it; the text is dropped rather than
/// panicking again — a lost keystroke is not worth a crash.
pub fn commit(text: String) {
    {
        let Ok(mut queue) = COMMITTED.lock() else { return };
        if queue.len() >= MAX_PENDING {
            // The oldest is dropped: a queue that grows is a memory that grows,
            // and the newest text is the one a person is looking at.
            queue.remove(0);
        }
        queue.push(text);
    }
    // The frame that takes the text has to be asked for: the window draws
    // when something asks it to, and the platform's text is not a thing the
    // window knows about. §257.
    repaint();
}

/// Take everything the platform committed since the last frame.
pub fn take() -> Vec<String> {
    COMMITTED.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default()
}

/// How many texts are waiting. For the tests, and for the log.
pub fn pending() -> usize {
    COMMITTED.lock().map(|queue| queue.len()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_commit_waits_for_the_frame() {
        commit("a".to_string());
        commit("b".to_string());
        assert_eq!(pending(), 2, "both texts are waiting");
        assert_eq!(take(), vec!["a".to_string(), "b".to_string()], "in the order they came");
        assert_eq!(pending(), 0, "the queue is empty again");
    }

    /// The queue is bounded: a commit that nobody reads must not grow for
    /// ever.
    #[test]
    fn the_queue_is_bounded() {
        for _ in 0..(MAX_PENDING + 10) {
            commit("x".to_string());
        }
        assert!(pending() <= MAX_PENDING, "the queue grew past its bound: {}", pending());
        let taken = take();
        assert_eq!(taken.len(), MAX_PENDING, "and the oldest are the ones that went");
    }

    /// The asks are counted, so the test can see them.
    static ASKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// The frames are counted, so the test can see them.
    static REPAINTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    /// A commit asks for the frame that will show it.
    ///
    /// A window draws when something asks it to, and the platform's text is
    /// not a thing the window knows about: without this the text waits for a
    /// frame that may never come, and the person sees nothing (§257).
    #[test]
    fn a_commit_asks_for_a_frame() {
        use std::sync::atomic::Ordering;
        set_repainter(Box::new(|| {
            REPAINTS.fetch_add(1, Ordering::SeqCst);
        }));
        let before = REPAINTS.load(Ordering::SeqCst);
        commit("x".to_string());
        assert!(
            REPAINTS.load(Ordering::SeqCst) > before,
            "the frame that takes the text was never asked for"
        );
        take();
    }

    /// The tree asks on every frame, and the ask is a call across the
    /// bridge: the same answer must not be sent again and again.
    #[test]
    fn the_platform_is_asked_only_when_the_answer_changes() {
        use std::sync::atomic::Ordering;
        set_asker(Box::new(|_wanted| {
            ASKS.fetch_add(1, Ordering::SeqCst);
        }));
        // The first ask sets the answer, so it goes across.
        want_keyboard(false);
        let after_first = ASKS.load(Ordering::SeqCst);
        // The second is the same answer, so it does not.
        want_keyboard(false);
        assert_eq!(
            ASKS.load(Ordering::SeqCst),
            after_first,
            "the platform was asked again for an answer it already had"
        );
        // And a different answer goes across.
        want_keyboard(true);
        assert_eq!(
            ASKS.load(Ordering::SeqCst),
            after_first + 1,
            "the platform was not asked for the answer it did not have"
        );
    }
}
