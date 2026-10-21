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
    /// True once a stash has been written, so the panel can say "saved" rather
    /// than making the user wonder whether it has ever saved anything.
    saved: bool,
    /// The last thing the storage panel did, shown under the buttons.
    note: String,
}

impl Application {
    fn new(stash: Box<dyn Stash>) -> Self {
        let (session, note) = restore(stash.get(STASH_KEY).as_deref());
        Self { session, stash, state: Default::default(), saved: false, note }
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
                    self.note = match stash_text(&self.session) {
                        Ok(text) => match self.stash.set(STASH_KEY, &text) {
                            Ok(()) => {
                                self.saved = true;
                                saved_note(&text)
                            }
                            Err(e) => e,
                        },
                        Err(e) => e,
                    };
                }
                if ui.button("Restore from this browser").clicked() {
                    self.note = match self.stash.get(STASH_KEY) {
                        None => "there is nothing saved here yet".to_string(),
                        Some(text) => {
                            let (session, note) = restore(Some(&text));
                            self.session = session;
                            note
                        }
                    };
                }
                if ui.button("Copy the plan as text").clicked() {
                    self.note = match stash_text(&self.session) {
                        Ok(text) => {
                            ctx.copy_text(text.clone());
                            format!("copied {} bytes to the clipboard", text.len())
                        }
                        Err(e) => e,
                    };
                }
                if ui.button("Forget").clicked() {
                    self.stash.remove(STASH_KEY);
                    self.saved = false;
                    self.note = "the saved plan is gone; what is on screen is not".to_string();
                }
            });
            ui.label(
                eframe::egui::RichText::new(if self.note.is_empty() {
                    "This browser holds one plan. Saving is explicit.".to_string()
                } else {
                    self.note.clone()
                })
                .small()
                .color(if self.state.status_is_error && !self.note.is_empty() {
                    tokens.warning
                } else {
                    tokens.text_dim
                }),
            );
        });
    }
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
fn restore(stashed: Option<&str>) -> (Session, String) {
    let Some(text) = stashed else {
        return (Session::sample(), "opened the sample plan; nothing was saved here yet".into());
    };
    match craftkit_io::load_text(text) {
        Ok(doc) => {
            let name = doc.meta.name.clone();
            let mut session = Session::new(name.clone());
            session.load_document(doc);
            (session, format!("reopened “{name}” from this browser"))
        }
        Err(e) => {
            (Session::sample(), format!("the saved plan would not load ({e}); opened the sample instead"))
        }
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
        let (session, note) = restore(None);
        assert_eq!(session.document().nodes.len(), 3, "the sample has three nodes");
        assert!(note.contains("nothing was saved"), "{note}");
    }

    #[test]
    fn a_stashed_plan_reopens_unchanged() {
        let original = Session::sample();
        let text = stash_text(&original).expect("encodes");

        let (session, note) = restore(Some(&text));
        assert_eq!(session.document().nodes.len(), original.document().nodes.len());
        assert!(note.contains("reopened"), "{note}");
        assert!(!session.is_dirty(), "a freshly reopened plan is not unsaved work");
    }

    #[test]
    fn a_broken_stash_falls_back_to_the_sample_and_says_why() {
        let (_, note) = restore(Some("{ this is not a plan"));
        assert!(note.contains("would not load"), "{note}");
        assert!(note.contains("sample"), "{note}");
    }

    #[test]
    fn an_empty_string_is_a_broken_stash_not_an_empty_plan() {
        // Storage can hand back "" for a key it lost. Treating that as "no
        // plan saved" is right, but it must not produce an empty window
        // silently.
        let (session, note) = restore(Some(""));
        assert_eq!(session.document().nodes.len(), 3);
        assert!(note.contains("would not load"), "{note}");
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
        assert!(!app.saved, "a host build cannot claim to have saved anything");
    }

    #[test]
    fn a_stashed_plan_reaches_the_palette_through_the_stash_not_by_luck() {
        let (store, boxed) = memory();
        let text = stash_text(&Session::sample()).expect("encodes");
        store.set(STASH_KEY, &text).expect("stores");

        let mut app = Application::new(boxed);
        draw_frames(&mut app, 1);
        assert_eq!(app.session.document().nodes.len(), 3);
        assert!(!app.note.contains("sample"), "it restored, it did not fall back: {}", app.note);
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
