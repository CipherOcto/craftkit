//! # craftkit-web
//!
//! craftkit in a browser: the same engine, the same commands, the same shell,
//! compiled to `wasm32-unknown-unknown`.
//!
//! ## What is different in a browser, and what is not
//!
//! **Not different:** the document, the evaluator, the command registry, the
//! shell. This binary adds no rule and no button that the desktop build does not
//! have. If it could, the two would drift.
//!
//! **Different:** there is no filesystem. A browser build has `localStorage`, a
//! handful of megabytes, cleared by the user, per origin. So:
//!
//! - [`STASH_KEY`] holds **one** plan, as the same `.craftkit` text the desktop
//!   writes to a file. Not a different format — the *same* [`craftkit_io`] text,
//!   so a plan can move between a browser and a desktop unchanged.
//! - Stashing is **explicit**. Nothing is written without the button, because
//!   autosaving a document the user cannot see into storage they cannot audit is
//!   worse than not autosaving.
//! - Losing the stash loses the plan, and the panel says so in the open.
//!
//! ## Running it
//!
//! ```text
//! cargo xtask wasm        # or: cargo build --target wasm32-unknown-unknown -p craftkit-web
//! wasm-bindgen --target web --out-dir apps/craftkit-web/dist \
//!     target/wasm32-unknown-unknown/release/craftkit-web.wasm
//! ```
//!
//! Serve `apps/craftkit-web/` over HTTP. `file://` will not do: a canvas needs a
//! real origin, and `localStorage` needs one too.
//!
//! ## The host build
//!
//! This crate also compiles natively, where there is no browser and no stash.
//! That is deliberate: it lets `cargo test -p craftkit-web` exercise the
//! restore/stash logic — the part that can actually be wrong — without a
//! headless browser, and it lets a host run of the binary explain itself instead
//! of failing obscurely.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod stash;

use stash::{STASH_KEY, Stash};

use craftkit_engine::Session;

/// The eframe application.
///
/// Holds the session directly: there is no control channel in a browser tab, so
/// there is no second thread and no lock.
struct Application {
    session: Session,
    stash: Box<dyn Stash>,
    /// Kept between frames by the host; see [`craftkit_ui::frame`].
    ///
    /// §105: this field existed and was *assigned* the shell's state every
    /// frame, which is the same as not keeping it at all: the shell's state
    /// was replaced by a fresh one each frame, so the selection never
    /// survived a redraw. The state must be handed **in** as well as out.
    state: craftkit_ui::ShellState,
    /// What this browser holds, and how the panel last fared.
    storage: Storage,
    /// What the panel last drew.
    ///
    /// A panel that says something should be able to say what it said: the
    /// storage panel is the only part of this app whose output no test could
    /// read, and a line nobody can read is a line nobody can check. §112.
    said: Vec<PanelLine>,
}

/// What the browser's storage holds, and what the panel last said about it.
#[derive(Debug, Clone, PartialEq)]
struct Storage {
    /// Whether this browser holds a plan.
    ///
    /// §112: this flag existed and was **written three times and read never**,
    /// and the panel's own line about saving — the one that says whether there
    /// is anything to come back to — could never be shown, because the note it
    /// would have sat beside is never empty. It is here because the panel has
    /// to answer "is there a plan saved here?", and because a flag nobody
    /// reads is a flag nobody checks.
    saved: bool,
    /// The note about the last thing the panel did.
    note: String,
    /// Whether that last thing failed.
    ///
    /// The panel's **own** outcome, not the shell's. The panel coloured its
    /// note by the shell's error flag, which says the last *command* failed —
    /// so a save the browser refused was drawn in the ordinary colour, and an
    /// unrelated command's failure coloured the save note. A panel that shows
    /// another panel's error is a panel that cannot be read. §112.
    failed: bool,
}

/// One line of the storage panel.
#[derive(Debug, Clone, PartialEq)]
struct PanelLine {
    /// What it says.
    text: String,
    /// Whether it is drawn as a warning.
    warning: bool,
}

impl Application {
    fn new(stash: Box<dyn Stash>) -> Self {
        let (session, storage) = restore(stash.get(STASH_KEY).as_deref());
        Self { session, stash, state: Default::default(), storage, said: Vec::new() }
    }

    /// Draw one frame: the shell, then this host's own storage panel.
    fn frame(&mut self, ctx: &eframe::egui::Context) {
        let tokens = craftkit_ui::Tokens::dark();
        // The state goes in as well as out: the shell is built for the frame
        // and dropped, so a host that does not hand its state back loses the
        // selection before the next frame. §105.
        let _automation = craftkit_ui::frame(ctx, &mut self.session, &tokens, &mut self.state);
        self.storage_panel(ctx, &tokens);
    }

    /// The one thing a browser has that a window does not.
    ///
    /// Drawn as a bottom panel *after* the shell has taken its own, so it takes
    /// the space that is left rather than fighting the shell for it.
    fn storage_panel(&mut self, ctx: &eframe::egui::Context, tokens: &craftkit_ui::Tokens) {
        eframe::egui::TopBottomPanel::bottom("browser-storage").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save in this browser").clicked() {
                    self.save();
                }
                if ui.button("Restore from this browser").clicked() {
                    self.restore_from_browser();
                }
                if ui.button("Copy the plan as text").clicked() {
                    self.copy(ctx);
                }
                if ui.button("Forget").clicked() {
                    self.forget();
                }
            });
            let lines = storage_lines(&self.storage);
            self.said = lines.clone();
            for line in &lines {
                ui.label(eframe::egui::RichText::new(&line.text).small().color(if line.warning {
                    tokens.warning
                } else {
                    tokens.text_dim
                }));
            }
        });
    }

    /// Save the plan into this browser.
    ///
    /// A method rather than code in the button's closure, so that what the
    /// panel does can be tested without a pointer: the buttons are the one
    /// part of this app that a test cannot press. §112.
    fn save(&mut self) {
        match stash_text(&self.session) {
            Ok(text) => match self.stash.set(STASH_KEY, &text) {
                Ok(()) => {
                    self.storage.saved = true;
                    self.storage.note = saved_note(&text);
                    self.storage.failed = false;
                }
                Err(e) => {
                    self.storage.note = e;
                    self.storage.failed = true;
                }
            },
            Err(e) => {
                self.storage.note = e;
                self.storage.failed = true;
            }
        }
    }

    /// Reopen the plan this browser holds.
    fn restore_from_browser(&mut self) {
        let (session, storage) = restore(self.stash.get(STASH_KEY).as_deref());
        self.session = session;
        self.storage.saved = storage.saved;
        self.storage.note = storage.note;
        self.storage.failed = storage.failed;
    }

    /// Copy the plan as text.
    fn copy(&mut self, ctx: &eframe::egui::Context) {
        match stash_text(&self.session) {
            Ok(text) => {
                ctx.copy_text(text.clone());
                self.storage.note = format!("copied {} bytes to the clipboard", text.len());
                self.storage.failed = false;
            }
            Err(e) => {
                self.storage.note = e;
                self.storage.failed = true;
            }
        }
    }

    /// Forget the saved plan.
    fn forget(&mut self) {
        self.stash.remove(STASH_KEY);
        self.storage.saved = false;
        self.storage.note = "the saved plan is gone; what is on screen is not".to_string();
        self.storage.failed = false;
    }
}

/// The lines the storage panel draws.
///
/// A function rather than code in the panel's closure, so that what the panel
/// says can be read without a browser and without a pointer. §112.
///
/// Two lines: what this browser holds, and what just happened. The first is
/// always there — the question "is there a plan saved here?" is the one a user
/// opens the page to answer, and the note answers it only right after the
/// action that changed it.
fn storage_lines(storage: &Storage) -> Vec<PanelLine> {
    let mut lines = vec![PanelLine {
        text: if storage.saved {
            "This browser holds one plan, and it is saved.".to_string()
        } else {
            "This browser holds one plan slot, and nothing is saved in it yet.".to_string()
        },
        warning: false,
    }];
    if !storage.note.is_empty() {
        lines.push(PanelLine { text: storage.note.clone(), warning: storage.failed });
    }
    lines
}

/// What the panel says after a successful save.
///
/// A **count**, never the plan. This was `format!("saved {text} bytes under …")`
/// — one interpolation short of `text.len()` — which put the whole document in
/// the note. A 7 KB plan then filled the bottom panel, pushed the toolbar off
/// the top of the window, and made "saved … bytes" a sentence with seven
/// thousand characters in it.
///
/// Found by clicking the button in a real browser, which is the only way to see
/// it: the note was never wrong in a test, because no test read it.
fn saved_note(text: &str) -> String {
    format!("saved {} bytes under “{STASH_KEY}”", text.len())
}

/// Reopen a stashed plan, or open the sample when there is nothing to reopen.
///
/// Never returns an error. A browser stash is user data that can be truncated
/// by a quota, hand-edited in devtools, or left over from an older format — and
/// none of those are reasons to start with a blank window and no explanation.
/// The fallback is stated in the note instead.
///
/// It also says whether the browser holds a plan, because that is a question
/// the panel has to answer on every frame and not only after an action.
/// §112.
fn restore(stashed: Option<&str>) -> (Session, Storage) {
    let Some(text) = stashed else {
        return (
            Session::sample(),
            Storage {
                saved: false,
                note: "opened the sample plan; nothing was saved here yet".to_string(),
                failed: false,
            },
        );
    };
    match craftkit_io::load_text(text) {
        Ok(doc) => {
            let name = doc.meta.name.clone();
            let mut session = Session::new(name.clone());
            session.load_document(doc);
            (
                session,
                Storage {
                    // The bytes are there, whether or not they were what the
                    // user thought they were.
                    saved: true,
                    note: format!("reopened “{name}” from this browser"),
                    failed: false,
                },
            )
        }
        Err(e) => (
            Session::sample(),
            Storage {
                saved: true,
                note: format!("what this browser holds would not load ({e}); opened the sample instead"),
                failed: true,
            },
        ),
    }
}

/// The plan as `.craftkit` text — the same bytes the desktop writes to a file.
fn stash_text(session: &Session) -> Result<String, String> {
    craftkit_io::save_text(session.document()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;

    /// Boot the browser build.
    ///
    /// `eframe` starts the run loop inside this future, so it is spawned rather
    /// than awaited; the `Result` is the interesting part, because a browser
    /// that cannot get a canvas produces a blank page and a console message, and
    /// the console message is the only clue a user will get.
    #[wasm_bindgen::prelude::wasm_bindgen(start)]
    pub fn start() {
        let runner = eframe::WebRunner::new();
        wasm_bindgen_futures::spawn_local(async move {
            let canvas = match canvas().await {
                Ok(canvas) => canvas,
                Err(message) => {
                    log::error!("{message}");
                    return;
                }
            };
            let options = eframe::WebOptions::default();
            let creator: eframe::AppCreator<'static> = Box::new(|_cc| {
                let app: Box<dyn eframe::App> = Box::new(Application::new(Box::new(stash::Browser)));
                Ok(app)
            });
            if let Err(e) = runner.start(canvas, options, creator).await {
                log::error!("craftkit could not start in the browser: {e:?}");
            }
        });
    }

    /// The canvas `index.html` provides.
    async fn canvas() -> Result<web_sys::HtmlCanvasElement, String> {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| "there is no document to draw into".to_string())?;
        let element = document
            .get_element_by_id(stash::CANVAS_ID)
            .ok_or_else(|| format!("index.html has no element with id “{}”", stash::CANVAS_ID))?;
        use wasm_bindgen::JsCast;
        element
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| format!("“{}” is not a <canvas>", stash::CANVAS_ID))
    }

    impl eframe::App for Application {
        fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
            self.frame(ctx);
        }

        fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
            if self.session.is_dirty() {
                log::warn!("leaving with unsaved changes");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Host
// ---------------------------------------------------------------------------

/// The browser build's `main`.
///
/// The work is done by the `start` export below; a `wasm32-unknown-unknown`
/// *binary* still needs a `main` symbol to link, and doing nothing is the
/// honest one. Writing a browser UI here would mean starting it twice.
#[cfg(target_arch = "wasm32")]
fn main() {}

/// Running this on a machine is a mistake worth explaining rather than
/// crashing on: the binary needs a browser's canvas and has no way to get one.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("craftkit-web {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--selftest") {
        return match selftest() {
            Ok(frames) => {
                println!("drew {frames} frames headlessly, no browser involved");
                std::process::ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("selftest failed: {message}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    println!(
        "craftkit-web is the browser build and needs one.\n\
         It is still a normal Rust crate, so `cargo test -p craftkit-web` runs here,\n\
         and `craftkit-web --selftest` draws frames with no browser at all.\n\
         Build it with `cargo xtask wasm`, or use `craftkit` for a desktop window."
    );
    std::process::ExitCode::from(2)
}

/// Draw the shell and the storage panel a few times with no window at all.
///
/// `egui` runs headlessly, so every panel's layout, drawing and error paths are
/// reachable from a build machine and from CI. A browser is the one thing this
/// cannot check, which is why the crate has a `wasm32` build as well.
#[cfg(not(target_arch = "wasm32"))]
fn selftest() -> Result<usize, String> {
    let mut app = Application::new(Box::new(stash::Nowhere));
    const FRAMES: usize = 3;
    draw_frames(&mut app, FRAMES);
    if app.session.document().nodes.is_empty() {
        return Err("the shell drew nothing and left no document".to_string());
    }
    Ok(FRAMES)
}

/// Draw `frames` frames into a context that is never shown.
///
/// `egui` refuses to lay out outside [`eframe::egui::Context::run`] — it needs
/// a frame to measure against — so every headless draw goes through here rather
/// than calling [`Application::frame`] straight.
#[cfg(not(target_arch = "wasm32"))]
fn draw_frames(app: &mut Application, frames: usize) {
    let ctx = eframe::egui::Context::default();
    for _ in 0..frames {
        // The returned `FullOutput` is what the CLI's software rasteriser walks;
        // here it would only be thrown away, and drawing is the point.
        let _ = ctx.run(Default::default(), |ctx| app.frame(ctx));
    }
}

#[cfg(not(target_arch = "wasm32"))]
const USAGE: &str = "\
craftkit-web — craftkit in a browser

USAGE (browser only):
    cargo xtask wasm
    wasm-bindgen --target web --out-dir apps/craftkit-web/dist \\
        target/wasm32-unknown-unknown/release/craftkit-web.wasm
    # then serve apps/craftkit-web/ over HTTP

ON A MACHINE:
    craftkit-web --selftest   Draw frames headlessly, no browser.
    craftkit-web              Explain that this is the browser build (exit 2).

For a desktop window use `craftkit`; for a headless session use `craftkit-cli`.
";

#[cfg(test)]
mod tests {
    use super::*;
    use stash::Memory;

    /// A stash that can be inspected from the outside.
    fn memory() -> (Memory, Box<dyn Stash>) {
        let shared = std::rc::Rc::new(std::cell::RefCell::new(std::collections::BTreeMap::new()));
        (Memory(shared.clone()), Box::new(Memory(shared)) as Box<dyn Stash>)
    }

    #[test]
    fn nothing_stashed_opens_the_sample_and_says_so() {
        let (session, storage) = restore(None);
        assert_eq!(session.document().nodes.len(), 3, "the sample has three nodes");
        assert!(storage.note.contains("nothing was saved"), "{}", storage.note);
        assert!(!storage.saved, "nothing is saved, and the panel says so");
    }

    #[test]
    fn a_stashed_plan_reopens_unchanged() {
        let original = Session::sample();
        let text = stash_text(&original).expect("encodes");

        let (session, storage) = restore(Some(&text));
        assert_eq!(session.document().nodes.len(), original.document().nodes.len());
        assert!(storage.note.contains("reopened"), "{}", storage.note);
        assert!(storage.saved, "the browser holds a plan, and the panel says so");
        assert!(!storage.failed, "the plan loaded");
        assert!(!session.is_dirty(), "a freshly reopened plan is not unsaved work");
    }

    #[test]
    fn a_broken_stash_falls_back_to_the_sample_and_says_why() {
        let (_, storage) = restore(Some("{ this is not a plan"));
        assert!(storage.note.contains("would not load"), "{}", storage.note);
        assert!(storage.note.contains("sample"), "{}", storage.note);
        assert!(storage.failed, "a stash that would not load is a failure, and the panel says so");
    }

    #[test]
    fn an_empty_string_is_a_broken_stash_not_an_empty_plan() {
        // Storage can hand back "" for a key it lost. Treating that as "no
        // plan saved" is right, but it must not produce an empty window
        // silently.
        let (session, storage) = restore(Some(""));
        assert_eq!(session.document().nodes.len(), 3);
        assert!(storage.note.contains("would not load"), "{}", storage.note);
    }

    #[test]
    fn the_stash_survives_a_round_trip_through_the_same_bytes_a_file_would_get() {
        let (store, _) = memory();
        let original = Session::sample();
        let text = stash_text(&original).expect("encodes");
        store.set(STASH_KEY, &text).expect("stores");

        let stored = store.get(STASH_KEY).expect("reads back");
        assert_eq!(stored, text);
        // And it is real .craftkit text, not a browser-only dialect: the tag is
        // the current format version, read back out of the file itself.
        let parsed: serde_json::Value = serde_json::from_str(&stored).expect("parses");
        assert_eq!(parsed["format"], craftkit_io::format::tag(1));
    }

    #[test]
    fn the_key_is_a_single_fixed_slot() {
        // One plan, one key. If this ever becomes a list, the panel's copy and
        // the stale-plan warning are both wrong.
        assert!(!STASH_KEY.contains(' '));
        assert!(STASH_KEY.starts_with("craftkit/"), "{STASH_KEY}");
    }

    #[test]
    fn the_palette_draws_headlessly_with_a_host_stash() {
        // The panel is only reachable in a browser otherwise, and "it compiles"
        // is not the same as "it draws". Three frames, because the first one
        // warms every font and the second is the one that finds a stale rect.
        let mut app = Application::new(Box::new(stash::Nowhere));
        draw_frames(&mut app, 3);
        assert_eq!(app.session.document().nodes.len(), 3);
        assert!(!app.storage.saved, "a host build cannot claim to have saved anything");
    }

    #[test]
    fn a_stashed_plan_reaches_the_palette_through_the_stash_not_by_luck() {
        let (store, boxed) = memory();
        let text = stash_text(&Session::sample()).expect("encodes");
        store.set(STASH_KEY, &text).expect("stores");

        let mut app = Application::new(boxed);
        draw_frames(&mut app, 1);
        assert_eq!(app.session.document().nodes.len(), 3);
        assert!(
            !app.storage.note.contains("sample"),
            "it restored, it did not fall back: {}",
            app.storage.note
        );
    }

    /// The panel says whether this browser holds a plan.
    ///
    /// §112: the flag existed and was written three times and read never, and
    /// the panel's own line about saving could never be shown — the note it
    /// would have sat beside is never empty, so the line was dead code the
    /// moment it was written.
    #[test]
    fn the_panel_says_whether_a_plan_is_saved() {
        let empty = storage_lines(&Storage {
            saved: false,
            note: "opened the sample plan; nothing was saved here yet".to_string(),
            failed: false,
        });
        let saved = storage_lines(&Storage {
            saved: true,
            note: "saved 7 bytes under “craftkit/plan”".to_string(),
            failed: false,
        });
        assert_ne!(empty[0].text, saved[0].text, "the two states say the same thing");
        assert!(empty[0].text.contains("nothing is saved"), "{}", empty[0].text);
        assert!(saved[0].text.contains("it is saved"), "{}", saved[0].text);
        // The line about the browser is not the note about the last action.
        assert_eq!(empty.len(), 2, "the panel draws the slot's line and the note");
        assert_eq!(saved.len(), 2);
    }

    /// A save the browser refused is drawn as a warning.
    ///
    /// §112: the panel coloured its note by the **shell's** error flag, so a
    /// refused save was drawn in the ordinary colour and an unrelated
    /// command's failure coloured the save note.
    #[test]
    fn a_failed_save_is_drawn_as_a_warning_and_a_successful_one_is_not() {
        let refused = storage_lines(&Storage {
            saved: false,
            note: "this browser has no localStorage to save into".to_string(),
            failed: true,
        });
        let worked = storage_lines(&Storage {
            saved: true,
            note: "saved 7 bytes under “craftkit/plan”".to_string(),
            failed: false,
        });
        assert!(refused[1].warning, "a refused save is not a warning: {:?}", refused[1]);
        assert!(!worked[1].warning, "a saved plan is not a warning: {:?}", worked[1]);
    }

    /// The panel's outcome is the panel's own, not the shell's.
    ///
    /// §112: the panel used to colour its note by `state.status_is_error` —
    /// the *shell's* last command. A test that pressed the shell's failure
    /// button and looked at the storage panel would have seen the panel's note
    /// in the warning colour, with nothing wrong with the panel at all.
    #[test]
    fn the_panels_outcome_is_not_the_shells_outcome() {
        let mut app = Application::new(Box::new(stash::Nowhere));
        draw_frames(&mut app, 1);

        // The shell's last command failed.
        app.state.status_is_error = true;
        app.save();
        draw_frames(&mut app, 1);
        let said = &app.said;
        assert!(!said.is_empty(), "the panel said nothing");
        let note = said.last().expect("a note");
        assert!(note.text.contains("nowhere to save"), "the refused save is not on the screen: {said:?}");
        assert!(note.warning, "the panel's own failure is the panel's own: {said:?}");

        // And a save that worked is not a warning either, even though the
        // shell is still showing a failure.
        let (store, boxed) = memory();
        let mut app = Application::new(boxed);
        app.state.status_is_error = true;
        app.save();
        draw_frames(&mut app, 1);
        let note = app.said.last().expect("a note");
        assert!(note.text.contains("saved"), "the save is not on the screen: {:?}", app.said);
        assert!(!note.warning, "a save that worked is not a warning");
        // And the panel now says that this browser holds a plan.
        assert!(app.storage.saved, "the panel saved a plan and does not say so");
        assert!(app.said[0].text.contains("it is saved"), "{}", app.said[0].text);
        // The store really does hold it.
        assert!(store.get(STASH_KEY).is_some(), "the plan was not stored");
    }

    /// The panel can be read.
    ///
    /// §112: the storage panel is the only part of this app whose output no
    /// test could read. A panel that says something should be able to say what
    /// it said.
    #[test]
    fn the_panel_remembers_what_it_said() {
        let mut app = Application::new(Box::new(stash::Nowhere));
        draw_frames(&mut app, 1);
        assert!(!app.said.is_empty(), "the panel drew nothing to read");
        assert!(
            app.said.iter().any(|line| line.text.contains("nothing is saved")),
            "the panel did not say what this browser holds: {:?}",
            app.said
        );
    }
}

#[cfg(test)]
mod stash_note_tests {
    use super::{STASH_KEY, saved_note};

    /// The defect: `{text}` where `{text.len()}` was meant, so the note carried
    /// the whole plan. A test that only checked "the note mentions the plan"
    /// would have passed; this one checks the note is *short*, which is the
    /// property that was broken and the one that keeps the panel a panel.
    #[test]
    fn the_save_note_reports_a_count_and_never_the_plan() {
        let plan = r#"{"format":"craftkit/1","document":{"nodes":[{"id":1,"name":"Level"}]}}"#;
        let note = saved_note(plan);

        assert!(note.contains(&plan.len().to_string()), "the count is missing: {note}");
        assert!(note.contains(STASH_KEY), "the key is missing: {note}");

        // The real assertion: no fragment of the document may appear.
        assert!(!note.contains("Level"), "the note carries the plan: {note}");
        assert!(!note.contains("format"), "the note carries the plan: {note}");
        assert!(note.len() < 80, "a note of {} characters cannot fit in a panel: {note}", note.len());
    }

    #[test]
    fn an_empty_plan_is_still_a_sentence() {
        let note = saved_note("");
        assert!(note.contains("saved 0 bytes"), "{note}");
        assert!(note.contains(STASH_KEY), "{note}");
    }
}
