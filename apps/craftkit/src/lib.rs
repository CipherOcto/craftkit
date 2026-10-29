//! # craftkit
//!
//! The application: a window, a document, and a control channel.
//!
//! **Two entry points, one application.** The desktop binary
//! (`src/main.rs`) parses the flags and calls [`run`]; the Android package
//! (`apps/craftkit-android`) is what the platform's `NativeActivity` calls,
//! and it calls [`run_android`]. The window, the session and the shell are
//! the same code on both.
//!
//! ## What this application owns
//!
//! - **The window.** `eframe` runs the loop; `craftkit_ui` draws the shell.
//! - **The session.** One [`craftkit_engine::Session`], shared between the UI and
//!   the control channel behind a mutex.
//! - **The files.** Open and save go through `craftkit-io`.
//!
//! It owns **no behaviour**. Every command it can perform is registered in the
//! engine; this file adds wiring, a panic hook and a few flags, and not one
//! more rule than that.
//!
//! ## Flags
//!
//! ```text
//! craftkit [--sample] [--control PORT] [FILE]
//! ```
//!
//! `--sample` opens the example document, which is what makes a fresh checkout
//! show something. `--control PORT` opens the loopback control channel; the token
//! is written to `~/.local/share/craftkit/control/PORT.token` unless
//! `CRAFTKIT_CONTROL_TOKEN` says otherwise.
//!
//! ## Panics
//!
//! A panic hook records a crash report before the process dies, because the
//! moment to find out *what the user was doing* is while the session still
//! knows. Everything else is handled by the engine's own boundary, which means
//! most panics never reach this hook at all.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The text the platform's own keyboard commits, behind the gate. §257.
pub mod ime;

use craftkit_control::ServerOptions;
use craftkit_engine::Session;

/// Everything the flags said.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Flags {
    sample: bool,
    control: Option<u16>,
    /// Whether the control channel asks for **no token**. §289.
    control_no_token: bool,
    file: Option<PathBuf>,
    headless_help: bool,
}

impl Flags {
    /// The flags the Android package starts with: the sample design.
    ///
    /// There is no command line on a phone — the launcher starts the
    /// application — so it opens the example, which is what the desktop
    /// application opens with `--sample`.
    pub fn sample() -> Self {
        Self { sample: true, ..Self::default() }
    }

    /// The flags the Android package starts with, and the control channel.
    ///
    /// A phone has no command line, so the port comes from the tree's own
    /// folder: a file called `control.port` in it, with the port in it. The
    /// folder is the tree's own, on the phone, and the file is written by
    /// whoever wants to drive the phone.
    pub fn sample_with_control(port: u16) -> Self {
        Self { sample: true, control: Some(port), ..Self::default() }
    }
}

/// The usage the desktop binary prints.
pub const USAGE: &str = "\
craftkit — a parametric plan editor

USAGE:
    craftkit [OPTIONS] [FILE]

ARGS:
    <FILE>            A .craftkit design to open.

OPTIONS:
    --sample          Open the example document.
    --control <PORT>  Serve the loopback control channel on <PORT>.
                      The token is written to
                      ~/.local/share/craftkit/control/<PORT>.token
                      unless CRAFTKIT_CONTROL_TOKEN is set.
    --control-no-token
                      Serve the control channel without a token: every
                      process on this machine may drive the application.
                      Needs --control.
    -h, --help        Print this message.
    -V, --version     Print the version.

The application is driven by commands, not by this binary: `craftkit-cli
commands` lists everything it can do, and the control channel exposes the same
list over JSON lines.
";

/// The Android app handle, when there is one.
///
/// Android hands the application a handle to the activity, and `eframe` needs
/// it to build the event loop. On the desktop there is no such thing, and the
/// type says so.
#[cfg(target_os = "android")]
type AndroidApp = winit::platform::android::activity::AndroidApp;

/// On the desktop there is no Android app handle. §236.
#[cfg(not(target_os = "android"))]
type AndroidApp = ();

/// Runs the application: the window, the session, and the channel.
///
/// The flags are parsed by the entry point, so that the Android one — which
/// has no flags — and the desktop one agree about what this does.
pub fn run(flags: Flags) -> std::process::ExitCode {
    run_with(flags, None)
}

/// Runs the application on Android, with the platform's handle. §236.
#[cfg(target_os = "android")]
pub fn run_android(flags: Flags, app: AndroidApp) -> std::process::ExitCode {
    // The window is laid out **in screen**, and the platform is asked to
    // report the rectangle that its status bar and navigation bar do not
    // cover. §247.
    //
    // The two flags are one thing: `LAYOUT_INSET_DECOR` is the flag that
    // makes the window manager report the inset rectangle, and it only
    // reports it when the window is laid out in the screen. §239 tried to fix
    // the landscape overlap by taking `LAYOUT_IN_SCREEN` away, which fixed
    // portrait and left landscape broken, and it could not see the second
    // flag because the platform answered an empty rectangle.
    //
    // The rectangle is the platform's, and the tree's job is to lay out
    // inside it.
    app.set_window_flags(
        winit::platform::android::activity::WindowManagerFlags::LAYOUT_IN_SCREEN
            | winit::platform::android::activity::WindowManagerFlags::LAYOUT_INSET_DECOR,
        winit::platform::android::activity::WindowManagerFlags::empty(),
    );
    run_with(flags, Some(app))
}

/// Where the tree's files go, on a phone.
///
/// **The tree has a rule and the application supplies the answer.** The rule
/// is in `craftkit_engine::crash`: the reports go to `$XDG_DATA_HOME/craftkit`,
/// then `$HOME/.local/share/craftkit`, then the temporary directory. A phone
/// has **none** of the first two, and the temporary directory on Android is
/// `/tmp`, which does not exist — so a crash report on a phone would go
/// nowhere, and the tree's own evidence would be lost.
///
/// The answer is the application's own directory, which is private to the
/// application and is readable with `adb shell run-as`. The rule stays the
/// tree's: the application passes the folder, because the tree already takes
/// it as a parameter (`record_in`, `install_crash_hook_in`), and a
/// process-global environment variable would race every other test in the
/// binary. §240.
#[cfg(target_os = "android")]
fn phone_folder(app: &AndroidApp) -> Option<std::path::PathBuf> {
    app.internal_data_path().map(|p| p.join(craftkit_foundation::paths::APP_NAME))
}

#[cfg(not(target_os = "android"))]
fn phone_folder(_app: &AndroidApp) -> Option<std::path::PathBuf> {
    None
}

fn run_with(flags: Flags, app: Option<AndroidApp>) -> std::process::ExitCode {
    // The tree's files, on a phone: the application's own directory. `None`
    // on the desktop, where the tree's own rule finds the right place.
    let tree_folder = app.as_ref().and_then(phone_folder);
    // The platform's handle, kept so the safe area can be asked for every
    // frame. §247.
    //
    // The handle is a handle, and a handle is cheap to keep: `eframe` takes
    // one by value and the application keeps another for the insets.
    #[cfg(target_os = "android")]
    let platform: Option<AndroidApp> = app.as_ref().map(|a| a.clone());
    #[cfg(not(target_os = "android"))]
    let _platform: Option<AndroidApp> = None;
    install_logger_to(tree_folder.as_ref().map(|p| p.join("craftkit.log")));
    // The crash reports, in the tree's own shape: the folder, and `crashes`
    // inside it — the same name `report_dir` gives it on the desktop.
    let crash_dir = tree_folder.as_ref().map(|p| p.join("crashes"));
    let session = Arc::new(Mutex::new(open(&flags)));
    install_crash_hook_in(&session, crash_dir.clone());

    // The window's egui context does not exist until eframe runs, but the
    // control channel starts first and outlives any one frame — so the two
    // sides meet here rather than in either one's lifetime.
    let ctx_slot: ContextSlot = Arc::new(Mutex::new(None));

    // The platform's own keyboard, behind the gate: the text it commits
    // arrives on the platform's own thread, and the frame that takes the
    // text out has to be asked for — a window draws when something asks it
    // to, and the platform's text is not a thing the window knows about.
    // §257.
    {
        let slot = Arc::clone(&ctx_slot);
        ime::set_repainter(Box::new(move || request_repaint(&slot)));
    }

    // The shell's memory is shared by the window and the control channel, so
    // that the screen the channel describes is the screen the window is
    // showing. §106.
    // The shell's state, and the one fact about this platform that the
    // shell cannot know: **does this platform have a keyboard of its own?**
    //
    // The desktop has one, and the tree's fields take the keys from it. A
    // phone's soft keyboard opens and delivers nothing (§236), so the tree
    // draws a keyboard of its own there (§246) — and the application is the
    // one that knows which platform it is on, the same way it is the one
    // that knows where the tree's files go (§240).
    let state: SharedState = Arc::new(Mutex::new(craftkit_ui::shell::ShellState {
        // The tree's own keyboard, where the platform has none — and where
        // the platform's own keyboard is behind the gate and cannot be
        // used. §246, §257.
        own_keyboard: cfg!(all(target_os = "android", not(feature = "platform-ime"))),
        ..craftkit_ui::shell::ShellState::default()
    }));

    // Started *before* the window, so a port clash is reported before anything
    // is drawn, and held in a local so it lives exactly as long as the window
    // and not a moment longer.
    // Where the token file goes. On the phone it is the tree's own folder,
    // so the token can be read back with the same `run-as` the crash reports
    // are read back with (§240, §249). On the desktop it is the usual
    // place.
    #[cfg(target_os = "android")]
    let token_dir: Option<std::path::PathBuf> = tree_folder.clone();
    #[cfg(not(target_os = "android"))]
    let token_dir: Option<std::path::PathBuf> = None;

    let control = match flags.control {
        None => None,
        Some(port) => {
            // The token is the default; `--control-no-token` is the other
            // mode, and it does not even generate one. §289.
            let auth = if flags.control_no_token {
                craftkit_control::Auth::None
            } else {
                match craftkit_control::Token::resolve() {
                    Ok(token) => craftkit_control::Auth::Token(token),
                    Err(e) => {
                        eprintln!("control channel: {e}");
                        return std::process::ExitCode::FAILURE;
                    }
                }
            };
            match start_control(port, &session, &ctx_slot, &state, auth, token_dir.as_deref()) {
                Ok((server, _path)) => Some(server),
                Err(e) => {
                    eprintln!("control channel: {e}");
                    return std::process::ExitCode::FAILURE;
                }
            }
        }
    };

    let viewport = eframe::egui::ViewportBuilder::default()
        .with_title(craftkit_foundation::paths::APP_NAME)
        .with_inner_size([1180.0, 720.0]);

    // Android builds the event loop from the activity's handle, and `eframe`
    // refuses to start without it — *"NativeOptions is missing required
    // android_app"*. On the desktop there is no handle, and the field does not
    // exist. §236.
    #[cfg(target_os = "android")]
    let options = eframe::NativeOptions { viewport, android_app: app, ..Default::default() };
    #[cfg(not(target_os = "android"))]
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    // On the desktop there is no handle to hand over.
    #[cfg(not(target_os = "android"))]
    let _ = app;

    let run = eframe::run_native(
        craftkit_foundation::paths::APP_NAME,
        options,
        Box::new(|_cc| {
            Ok(Box::new(Application {
                session,
                ctx_slot,
                state,
                crash_dir,
                #[cfg(target_os = "android")]
                platform: platform.clone(),
            }))
        }),
    );
    match run {
        Ok(()) => {
            // The window has closed, so the channel has nothing left to serve.
            drop(control);
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            // No display, or no GPU. The window is the point of this binary, but
            // it is not the only thing it does: the control channel reaches
            // every command, and `ui.render` draws a real frame into a fresh
            // egui context — no window, no GPU. So a failed window is a loud
            // message and not an exit, and the channel keeps serving.
            //
            // That matters on a build machine, which is exactly where an agent
            // tends to be, and it means `craftkit` without a display is a
            // usable headless host rather than a refusal.
            eprintln!("craftkit could not open a window: {e}");
            match control {
                None => {
                    eprintln!(
                        "There is no control channel either, so there is nothing left to do.\n\
                         On a machine with no display use `craftkit-cli`, which needs neither a window nor a channel."
                    );
                    std::process::ExitCode::FAILURE
                }
                Some(mut server) => {
                    eprintln!(
                        "The control channel is still serving on 127.0.0.1:{} — `engine.execute` and `ui.render` both work.\n\
                         Ctrl-C to stop.",
                        server.port()
                    );
                    server.join();
                    std::process::ExitCode::SUCCESS
                }
            }
        }
    }
}

/// Parse the flags. `Ok(None)` means "printed help and stop".
/// The flags the desktop entry point parsed.
///
/// `None` means the flags asked for something to be printed and the program
/// to stop, which is `--help` and `--version`.
pub fn parse_flags(args: &[String]) -> Result<Option<Flags>, String> {
    let mut flags = Flags::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("craftkit {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--sample" => flags.sample = true,
            "--control-no-token" => flags.control_no_token = true,
            "--control" => {
                let port = args.get(i + 1).ok_or_else(|| "`--control` needs a port".to_string())?;
                flags.control =
                    Some(port.parse::<u16>().map_err(|_| format!("`{port}` is not a port number"))?);
                i += 1;
            }
            other if other.starts_with('-') => {
                return Err(format!("`{other}` is not a flag I know"));
            }
            other => flags.file = Some(PathBuf::from(other)),
        }
        i += 1;
    }
    // The flag that turns the token off is about the channel, so it needs the
    // channel. §289.
    if flags.control_no_token && flags.control.is_none() {
        return Err("`--control-no-token` needs `--control`".to_string());
    }
    Ok(Some(flags))
}

/// The session the window opens with.
fn open(flags: &Flags) -> Session {
    if let Some(path) = &flags.file {
        return match craftkit_io::load(path) {
            Ok(doc) => {
                let mut session = Session::new(doc.meta.name.clone());
                session.load_document(doc);
                // The design came from this file, and "Save" writes it back
                // there. The session cannot know that on its own: it was
                // handed a document, not a path. §288.
                session.set_source_path(path.clone());
                session
            }
            Err(e) => {
                // A file that will not open must not stop the application from
                // starting: an empty document plus a loud message is something
                // a person can act on, and a refusal to launch is not.
                eprintln!("could not open {}: {e}", path.display());
                Session::sample()
            }
        };
    }
    if flags.sample {
        return Session::sample();
    }
    Session::new("Untitled")
}

/// Where the window's egui context is published for the control channel.
///
/// `eframe::Context` is a handle, so handing one to the channel's thread is
/// how a command that ran over TCP reaches the window's frame loop. `None`
/// until the first frame: the channel is deliberately started before the window,
/// so for the first moments there is nothing to repaint and the slot stays
/// empty rather than blocking.
type ContextSlot = Arc<Mutex<Option<eframe::egui::Context>>>;

/// Ask the window to redraw, if there is one.
///
/// # Why this exists
///
/// eframe paints when something happens: an input event, or a repaint request.
/// A command that arrives over the control channel is none of those — it
/// mutates the session on a socket thread, behind a mutex the frame loop never
/// sees. Without this call the window keeps showing the last frame it painted,
/// so an agent that drives the app and then looks at the window is looking at a
/// picture of the past, with no error anywhere to say so.
///
/// Ignored when no context has been published yet, which is the window's first
/// few frames and the whole lifetime of a process whose window failed to open.
///
/// # Why there is no unit test for the call itself
///
/// egui offers no observable that separates "a repaint was requested" from
/// "a repaint is pending" on a context that is not attached to a real window:
/// `has_requested_repaint` is already true after the first headless pass
/// (`outstanding` is re-armed at the end of every pass), and the wake-up
/// callback only fires when the delay is *lowered*, which requires the idle
/// state only a real integration loop reaches. Both were measured, not assumed.
///
/// So the check that matters is `scripts/live-window-check.sh`, which runs this
/// binary on a real display, drives it over the control channel and compares
/// the window's own pixels before and after. A unit test asserting a flag would
/// have looked like evidence and proved nothing — which is the failure mode
/// `KNOWN-GAPS.md` §1 is about.
fn request_repaint(slot: &ContextSlot) {
    let Ok(guard) = slot.lock() else { return };
    if let Some(ctx) = guard.as_ref() {
        ctx.request_repaint();
    }
}

/// Open the loopback control channel.
///
/// `token_dir` is where the token file goes; `None` is the usual place. It is a
/// parameter so that a test can keep the file inside a directory it owns.
fn start_control(
    port: u16,
    session: &Arc<Mutex<Session>>,
    ctx_slot: &ContextSlot,
    state: &SharedState,
    auth: craftkit_control::Auth,
    token_dir: Option<&Path>,
) -> Result<(craftkit_control::Server, Option<PathBuf>), String> {
    // Cloned rather than borrowed: the handler outlives this function, and the
    // slot outlives it in turn because `main` holds it for the window's life.
    let slot = Arc::clone(ctx_slot);
    // The shell's memory, shared with the window: the channel reports the
    // screen the window is showing, not a screen of its own. §106.
    let ui_state = Arc::clone(state);
    // The two modes are two constructors, so the mode is chosen in one place
    // and cannot be half-set. §289.
    let mut options = match &auth {
        craftkit_control::Auth::Token(token) => ServerOptions::new(port, token.clone()),
        craftkit_control::Auth::None => ServerOptions::unauthenticated(port),
    }
    .with_ui(move |session, request| {
        let guard = ui_state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        ui_request(session, request, &guard)
    })
    .with_after(move || request_repaint(&slot));
    if let Some(dir) = token_dir {
        options = options.with_token_dir(dir);
    }
    // The channel is bound **before** the token is published. The token file
    // is the promise that this port answers to this token, and a second
    // instance — which cannot bind — used to take that promise away from the
    // instance that could keep it, locking a running app out with "a bad
    // token". §54.
    let (server, path) = craftkit_control::serve_published(Arc::clone(session), options)?;
    println!("control channel listening on 127.0.0.1:{}", server.port());
    match &path {
        Some(path) => {
            println!("token in {}", path.display());
            println!("try: {{\"id\":1,\"token\":\"…\",\"method\":\"engine.describe\"}}");
        }
        // No token, so no file and no token in the example: the channel is
        // loopback, and this machine is the one it opens. §289.
        None => {
            println!("no token: every process on this machine may drive this application");
            println!("try: {{\"id\":1,\"method\":\"engine.describe\"}}");
        }
    }
    Ok((server, path))
}

/// Answer a `ui.*` request.
///
/// # Why it draws a fresh frame rather than reading the window
///
/// Reading the window's pixels needs a graphics context, and this runs on a
/// socket thread that has none. It would not be better either: the framebuffer
/// is one particular moment, while this draws the *current* state into a fresh
/// egui context — so what an agent is shown is what the session says now, not
/// whatever the window last painted.
///
/// The consequence is that `ui.render` works with no window at all, which is
/// what makes this binary usable on a build machine. The consequence of *that*
/// is that `ui.screenshot` must not pretend to be the window's pixels, so it
/// says so in its own reply.
///
/// # The state it draws with
///
/// `state` is the shell's memory, and it is the **window's** memory: the
/// window's frame loop and this handler share one value, so the screen the
/// channel describes is the screen the window is showing. It used to draw a
/// shell with nothing selected — which is a screen the window cannot be in,
/// because with nothing selected the properties panel is empty and publishes
/// no editor, so half the window's widgets had no name to be found under.
/// §106.
fn ui_request(
    session: &mut Session,
    request: &craftkit_control::Request,
    state: &craftkit_ui::shell::ShellState,
) -> craftkit_control::Response {
    use craftkit_control::Response;
    use serde_json::Value as Json;

    // The method name decides, so `ui.click` clicks. This used to read the action
    // out of `params` and default it to `render`, which meant the one thing every
    // agent would type from the protocol docs drew a picture instead.
    let action = match request.ui_action() {
        Ok(action) => action,
        Err(response) => return response,
    };
    let (default_width, default_height) =
        (craftkit_ui::raster::DEFAULT_SIZE[0], craftkit_ui::raster::DEFAULT_SIZE[1]);
    let width = size(request.params.get("width")).unwrap_or(default_width);
    let height = size(request.params.get("height")).unwrap_or(default_height);

    // The state to draw with: the window's, unless the caller named a node to
    // select. §106.
    let state = match selected_state(session, state, request.params.get("selected")) {
        Ok(state) => state,
        Err(message) => return Response::error(&request.id, "bad-parameter", message),
    };
    let selected = state.selected.map(|id| id.get());

    match action {
        craftkit_control::UiAction::Render | craftkit_control::UiAction::Screenshot => {
            let message = if action == craftkit_control::UiAction::Screenshot {
                format!(
                    "rendered {}x{} from the current session — a fresh render of the shell, not the window's framebuffer, which a socket thread cannot read",
                    width, height
                )
            } else {
                format!("rendered {width}x{height}")
            };
            match craftkit_ui::raster::render(session, width, height, &state) {
                Ok(frame) => Response::ok(
                    &request.id,
                    message,
                    Json::Object(
                        [
                            ("png_base64".to_string(), Json::String(base64(&frame.png))),
                            ("width".to_string(), Json::from(frame.width)),
                            ("height".to_string(), Json::from(frame.height)),
                            ("widgets".to_string(), Json::from(frame.automation.names())),
                            ("rects".to_string(), rects(&frame.automation)),
                            ("disabled".to_string(), Json::from(frame.automation.disabled_names())),
                            ("selected".to_string(), selected.map(Json::from).unwrap_or(Json::Null)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
                Err(e) => Response::error(&request.id, "bad-parameter", e),
            }
        }
        craftkit_control::UiAction::Click => {
            // By name, or by point — but not both. The names say what is on
            // screen; a name with no position behind it is a name an agent
            // cannot act on, so the name is enough. §104.
            let by_name = request.params.get("name").and_then(|v| v.as_str());
            let by_point = (number(request.params.get("x")), number(request.params.get("y")));
            if by_name.is_some() && by_point.0.is_some() {
                return Response::error(
                    &request.id,
                    "bad-parameter",
                    "send a `name` or an `x` and `y`, not both",
                );
            }
            let clicked = match (by_name, by_point) {
                (Some(name), _) => craftkit_ui::raster::click_name(session, name, width, height, &state),
                (None, (Some(x), Some(y))) => {
                    craftkit_ui::raster::click(session, x, y, width, height, &state)
                }
                (None, _) => {
                    return Response::error(
                        &request.id,
                        "bad-parameter",
                        "`x` and `y` are required, and must be finite numbers; or give the `name` of a widget",
                    );
                }
            };
            match clicked {
                Ok(click) => {
                    let hit = click.hit.clone();
                    Response::ok(
                        &request.id,
                        click_message(hit.as_deref()),
                        Json::Object(
                            [
                                ("hit".to_string(), hit.map(Json::from).unwrap_or(Json::Null)),
                                ("x".to_string(), Json::from(click.point[0])),
                                ("y".to_string(), Json::from(click.point[1])),
                                ("status".to_string(), Json::from(click.state.status)),
                                ("selected".to_string(), selected.map(Json::from).unwrap_or(Json::Null)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    )
                }
                Err(e) => Response::error(&request.id, "bad-parameter", e),
            }
        }
        craftkit_control::UiAction::Widgets => {
            match craftkit_ui::raster::render(session, width, height, &state) {
                Ok(frame) => Response::ok(
                    &request.id,
                    "the widgets on screen, and where they are",
                    Json::Object(
                        [
                            ("widgets".to_string(), Json::from(frame.automation.names())),
                            ("rects".to_string(), rects(&frame.automation)),
                            ("disabled".to_string(), Json::from(frame.automation.disabled_names())),
                            ("width".to_string(), Json::from(frame.width)),
                            ("height".to_string(), Json::from(frame.height)),
                            ("selected".to_string(), selected.map(Json::from).unwrap_or(Json::Null)),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                ),
                Err(e) => Response::error(&request.id, "bad-parameter", e),
            }
        }
    }
}

/// The shell's state to draw with.
///
/// The window's own state, unless the caller named a node to select: an agent
/// that wants to see the properties panel has to be able to put the screen
/// into the state it wants to look at, and without a selection the panel is
/// empty. The named node must exist — a request that names a node that is not
/// there is refused rather than answered with a screen of nothing.
///
/// # Errors
///
/// The `selected` is not a node id, or it is a node id that is not in the
/// document. Both name what was asked for.
fn selected_state(
    session: &Session,
    live: &craftkit_ui::shell::ShellState,
    selected: Option<&serde_json::Value>,
) -> Result<craftkit_ui::shell::ShellState, String> {
    let Some(value) = selected else {
        return Ok(live.clone());
    };
    let id = value.as_u64().ok_or_else(|| format!("`selected` must be a node id; got {value}"))?;
    let node = craftkit_core::NodeId(id);
    if session.node(node).is_none() {
        return Err(format!("there is no node with id {id}"));
    }
    let mut state = live.clone();
    state.selected = Some(node);
    Ok(state)
}

/// Every published widget's rectangle, as a map from its name.
///
/// The names say *what* is on screen; this says *where*. An agent that has
/// been given a name and cannot ask for its position can only guess pixels,
/// and a guess is not a click. §104.
fn rects(automation: &craftkit_ui::automation::Automation) -> serde_json::Value {
    use serde_json::Value as Json;
    Json::Object(
        automation
            .rects()
            .into_iter()
            .map(|(name, rect)| {
                (
                    name.to_string(),
                    Json::Array(vec![
                        Json::from(rect.min.x),
                        Json::from(rect.min.y),
                        Json::from(rect.width()),
                        Json::from(rect.height()),
                    ]),
                )
            })
            .collect(),
    )
}

/// What a click says.
///
/// A click is a hit test and nothing else — see `KNOWN-GAPS.md` §17 — so the
/// reply must not read as though something happened. It used to say
/// ``clicked `toolbar.node.duplicate` ``, which an agent has every reason to
/// read as "the button was pressed": three of those replies in a row, and the
/// document was byte-identical afterwards. The name is still returned, in `hit`,
/// for the caller that wants it; the sentence now says what happened.
fn click_message(hit: Option<&str>) -> String {
    match hit {
        Some(name) => format!(
            "`{name}` is at that point; a click reports where it landed and does not activate \
             anything — run it with `engine.execute`"
        ),
        None => "nothing is at that point".to_string(),
    }
}

/// A requested size, checked rather than trusted.
///
/// `usize::MAX` is a perfectly good JSON number, so this bounds the value before
/// it reaches the renderer rather than trusting the caller to have been
/// reasonable.
fn size(value: Option<&serde_json::Value>) -> Option<usize> {
    let raw = value?.as_u64()?;
    usize::try_from(raw).ok().filter(|n| *n <= craftkit_ui::raster::MAX_SIZE)
}

/// A requested coordinate, checked rather than trusted.
fn number(value: Option<&serde_json::Value>) -> Option<f32> {
    let raw = value?.as_f64()?;
    if raw.is_finite() { Some(raw as f32) } else { None }
}

/// Standard base64, because JSON has no bytes and a PNG is not text.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(triple >> 18) as usize & 63] as char);
        out.push(ALPHABET[(triple >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(triple >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[triple as usize & 63] as char } else { '=' });
    }
    out
}

/// Record a crash report before the process dies.
///
/// # Why this exists
///
/// Everything inside `Session::execute` runs under `catch_unwind`, so the common
/// case never reaches here: a command that panics is caught, the document is
/// restored, and the user sees an error.
///
/// This is for the panics that escape anyway — a panic on a thread that is not
/// the render thread, a poisoned lock turned into an unwrap somewhere we do
/// not own. **A panic in the drawing code does not reach here**: the render
/// path holds the session's lock, and this hook runs on the panicking thread,
/// so it could not write a report even if it tried. That case is caught by
/// [`draw_guarded`], which is the one place that can still see the session.
/// §76.
///
/// The moment to write the report is *while the session still knows what the
/// user was doing*. After the process dies that knowledge is gone, and a crash
/// report that says only "it crashed" is not worth the disk it occupies.
///
/// # What it must not do
///
/// - **It must not panic.** A panic inside a panic hook aborts the process
///   immediately, so the original panic is lost along with the report. Every
///   fallible step here is on its own and gives up quietly.
/// - **It must not block for long.** The session may be locked by the thread
///   that panicked. `try_lock` is used rather than `lock` for exactly that
///   reason: a hook that waits for a lock held by a dead thread is a hang that
///   looks like a freeze.
///
/// A report that could not be written is a log line, not a failure: the crash has
/// already happened and there is nothing this can do about it.
/// The tree's panic hook, writing to an explicit folder.
///
/// `None` is the real crash folder. The parameter exists so the *production*
/// hook can be tested without an environment variable: a process-global
/// `CRAFTKIT_CRASH_DIR` set by one test would be read by another, and a test
/// that cannot run alone is a test that fails in a full suite.
fn install_crash_hook_in(session: &Arc<Mutex<Session>>, dir: Option<std::path::PathBuf>) {
    // Only one hook, and it is not the default one — the default still prints,
    // which is what makes the panic visible in a terminal at all.
    let previous = std::panic::take_hook();
    let session = Arc::clone(session);
    std::panic::set_hook(Box::new(move |info| {
        let message = panic_message(info);
        let command = last_command(&session);

        // `try_lock` on purpose; see the note above.
        match session.try_lock() {
            Ok(guard) => match &dir {
                Some(dir) => {
                    craftkit_engine::crash::record_in(dir, &guard, &command, &message);
                }
                None => {
                    craftkit_engine::crash::record(&guard, &command, &message);
                }
            },
            Err(_) => {
                log::error!(
                    "a crash happened ({message}) and the session was locked, \
                     so the hook could not write a report"
                );
            }
        }

        previous(info);
    }));
}

/// Draw a frame, and survive a panic in the drawing code.
///
/// **The panic hook cannot do this.** The hook runs on the panicking thread,
/// and the render path holds the session's lock for the length of a frame — so
/// the hook's `try_lock` fails, and it writes nothing. Measured: a panic
/// while the session was locked wrote **no report at all**, and a panic in a
/// widget is exactly that. The hook's own comment used to name this case as
/// the one it existed for. §76.
///
/// This is the one place that can still see the document the user was working
/// on, so this is where the report is written. And the application survives:
/// a panic in a widget is a bug, not a reason to lose the work.
fn draw_guarded(
    session: &mut Session,
    dir: Option<&std::path::Path>,
    draw: impl FnOnce(&mut Session),
) -> Result<(), String> {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| draw(session)));
    let Err(payload) = outcome else { return Ok(()) };
    // `as_ref()`, not `&payload`: coercing `&Box<dyn Any>` to a trait object
    // points at the **box**, and the downcast inside the box never matches.
    let message = message_from_payload(payload.as_ref());
    let command = last_command_of(session);
    match dir {
        Some(dir) => {
            craftkit_engine::crash::record_in(dir, session, &command, &message);
        }
        None => {
            craftkit_engine::crash::record(session, &command, &message);
        }
    }
    Err(message)
}

/// The panic message, in a form a person can read.
fn panic_message(info: &std::panic::PanicHookInfo<'_>) -> String {
    message_from_payload(info.payload())
}

/// The message out of a panic payload.
///
/// Split from [`panic_message`] because `PanicHookInfo` cannot be constructed
/// outside a real panic — its constructor is unstable — so the only way to test
/// the awkward payloads is to hand this function one directly.
fn message_from_payload(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        return (*text).to_string();
    }
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    "a panic with no message".to_string()
}

/// The command that was running, from the session's own log.
///
/// The session's log rather than the engine's panic path, because by the time a
/// panic hook runs the engine may not have written anything: the panic may have
/// happened outside a command entirely.
fn last_command(session: &Arc<Mutex<Session>>) -> String {
    session
        .try_lock()
        .ok()
        .map(|guard| last_command_of(&guard))
        .unwrap_or_else(|| "none recorded".to_string())
}

/// [`last_command`] for a session that is already in hand.
///
/// The render path holds the lock, so it can ask the session directly — and
/// the crash report it writes should name the command that was running, which
/// is the one thing that says what the user was doing. §76.
fn last_command_of(session: &Session) -> String {
    session.log().last_command().map(str::to_string).unwrap_or_else(|| "none recorded".to_string())
}

/// A minimal stderr logger.
///
/// Hand-rolled rather than pulled in: a logging framework is a dependency
/// surface for something this small, and the whole of it is twenty lines.
/// The tree's logger, writing to a file as well as to the standard error.
///
/// **A phone has no standard error.** A `NativeActivity`'s stdout and stderr
/// go nowhere, so the tree's log on a phone was a log nobody could read. With
/// a file, the log is in the application's own directory, and `adb shell
/// run-as` reads it.
///
/// The file is opened once and appended to. A file that cannot be opened is
/// **not** an error: the standard error is still there, and a logger that
/// refused to start because of a file would take the application down.
///
/// **And the file is bounded.** The log was the one file the tree wrote with
/// no limit on it: it grew for as long as the application ran, and it grew
/// **across** runs, because the file was appended to and never rotated. §242.
fn install_logger_to(path: Option<std::path::PathBuf>) {
    struct Sink {
        file: Option<std::sync::Mutex<LogFile>>,
    }
    impl log::Log for Sink {
        fn enabled(&self, _: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            if !self.enabled(record.metadata()) {
                return;
            }
            let line = format!("[{}] {}\n", record.level(), record.args());
            eprint!("{line}");
            if let Some(file) = &self.file
                && let Ok(mut file) = file.lock()
            {
                file.write_line(&line);
            }
        }

        fn flush(&self) {}
    }

    // The logger is a `static`, because `set_logger` wants a `&'static`, and
    // a `File` cannot be built in one — so the sink lives in a one-shot cell
    // and is filled in once, before the logger is installed.
    static LOGGER: std::sync::OnceLock<Sink> = std::sync::OnceLock::new();

    let file = path.and_then(LogFile::open);

    let sink = LOGGER.get_or_init(|| Sink { file: file.map(std::sync::Mutex::new) });
    // `set_logger` fails only if something already claimed the global logger,
    // which is a test harness rather than a reason to abort.
    let _ = log::set_logger(sink);
    log::set_max_level(log::LevelFilter::Info);
}

/// The largest the log's file may grow before the sink starts a new one.
///
/// The tree bounds its other files — the undo stack by bytes (§52), the
/// command trace by bytes, the command log by count — and the log was the one
/// file it wrote with **no bound at all**. §242.
const MAX_LOG_BYTES: u64 = 1 << 20;

/// The log's file: the one being appended to, how big it has grown, and the
/// name of the file before it.
///
/// The log is bounded by **rotation**, not by truncation. The file that has
/// reached its limit is moved aside under a second name and the log starts a
/// new one; a truncated log would lose the lines at the **start** — the ones
/// that say what the application was doing before it went wrong.
///
/// **Two files is the whole history.** The file before the current one is
/// removed when the log rotates again, so the tree keeps the last two files'
/// worth of the log — about two megabytes — and no more. That is the trade
/// this makes: a log that is bounded keeps the **recent** past and drops the
/// rest, where an unbounded log keeps everything and takes the folder with
/// it.
struct LogFile {
    /// The file the log is being appended to.
    path: PathBuf,
    /// The file the log was in before the last rotation.
    previous: PathBuf,
    /// The open file, or `None` when the log cannot be written to a file at
    /// all — the standard error still has it.
    file: Option<std::fs::File>,
    /// How many bytes the file has grown to.
    bytes: u64,
}

impl LogFile {
    /// Open the log's file, appending to whatever is already there.
    ///
    /// A file that cannot be opened is **not** an error: the standard error is
    /// still there, and a logger that refused to start because of a file
    /// would take the application down.
    fn open(path: PathBuf) -> Option<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = match open_append(&path) {
            Ok(file) => file,
            Err(e) => {
                eprintln!("the log file could not be opened ({e}); the log goes to the standard error");
                return None;
            }
        };
        // The file may already hold a run's worth of lines: it is appended
        // to across runs, so its size is asked for rather than assumed.
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Some(LogFile { previous: previous_of(&path), path, file: Some(file), bytes })
    }

    /// Write one line, and rotate the file if it has reached its limit.
    fn write_line(&mut self, line: &str) {
        if self.file.is_none() {
            return;
        }
        if self.bytes + line.len() as u64 > MAX_LOG_BYTES {
            self.rotate();
        }
        let Some(file) = self.file.as_mut() else { return };
        use std::io::Write;
        if file.write_all(line.as_bytes()).is_ok() {
            self.bytes += line.len() as u64;
        }
    }

    /// Move the full file aside and start a new one.
    ///
    /// A rotation that fails leaves the file that was there alone and the log
    /// goes on growing: the bound is a bound, and a log that stopped being
    /// written would be worse than a log that is too big.
    fn rotate(&mut self) {
        // The file before this one goes away: two files is the bound, and
        // three would not be. The log is the tree's own file and the tree's
        // own folder, so removing it is the tree's business.
        let _ = std::fs::remove_file(&self.previous);
        if std::fs::rename(&self.path, &self.previous).is_err() {
            return;
        }
        match open_append(&self.path) {
            Ok(file) => {
                self.file = Some(file);
                self.bytes = 0;
            }
            Err(_) => {
                // The file the log was in is now the previous one, and the new
                // one cannot be opened — so the log goes to the standard error
                // and to the file it was in.
                self.file = None;
                self.bytes = 0;
            }
        }
    }
}

/// The name of the file that holds the log before the current one.
///
/// The suffix is a `.1` beside the log's own name, so the two files are the
/// same file in two ages rather than two things with names of their own.
fn previous_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".1");
    PathBuf::from(name)
}

/// Open a file to append to, creating it if it is not there.
///
/// The call is **one line** on purpose: the tree's gate allows
/// `OpenOptions::new` in a file that may only append, and it looks for
/// `append(true)` on the same line. A call spread over three lines would
/// look like a file that creates without appending. §240.
///
/// **And the file is private.** The tree's rule is that what the program
/// writes is private (§55, §57), and the log is a file the program writes:
/// it holds the commands the user ran and the names of their nodes. A file
/// created with the umask is a file somebody else on the machine can read —
/// on this account the umask is `0002`, so the log would be `0664`. The
/// mode is asked for **at creation**, where the umask can only clear bits
/// the mode does not grant, and not afterwards — a file that is readable for
/// the length of a `chmod` is a file that was readable. §244.
#[cfg(unix)]
fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new().create(true).append(true).mode(0o600).open(path)
}

/// Open a file to append to, on a system that has no modes to ask for.
///
/// Everywhere else the file is created with the process's own default, which
/// is the best available. §244.
#[cfg(not(unix))]
fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new().create(true).append(true).open(path)
}

/// The eframe application.
///
/// Holds the session behind a mutex because the control channel runs on its own
/// thread and needs the same document. The UI only ever borrows it for the
/// length of a frame.
///
/// `ctx_slot` is the other half of that sharing: the channel is started before
/// the window, so it cannot be handed a context directly.
struct Application {
    session: Arc<Mutex<Session>>,
    ctx_slot: ContextSlot,
    /// Where the tree's crash reports go, when the platform says.
    ///
    /// `None` is the tree's own rule: the platform's data directory, or the
    /// user's. On a phone the rule finds nothing (§240), so the application
    /// says where — and this is the value that says it.
    crash_dir: Option<std::path::PathBuf>,
    /// What the shell remembers between frames.
    ///
    /// §105: the window draws with a fresh shell every frame, and the shell
    /// is where the selection and the status line live. Without this the
    /// selection was gone before the next frame, and the properties panel
    /// could never open.
    ///
    /// §106: it is shared with the control channel, so the screen the channel
    /// reports is the screen the window is showing.
    state: SharedState,
    /// The platform's handle, when there is one. §247.
    ///
    /// The application is the one that knows the platform, the same way it is
    /// the one that knows where the tree's files go (§240): the tree asks,
    /// and the application answers.
    #[cfg(target_os = "android")]
    platform: Option<AndroidApp>,
}

/// The shell's memory, shared by the window and the control channel.
///
/// A mutex because the two live on different threads, and the *same* mutex
/// because the window takes the session before the state and the channel's
/// handler is called with the session already held — so the two always take
/// them in the same order, and neither can be waiting for the other.
type SharedState = Arc<Mutex<craftkit_ui::shell::ShellState>>;

/// The safe area, on a phone. §247.
///
/// The platform says where the application may draw: the window is laid out
/// in the screen, and the platform reports the rectangle that its status bar
/// and navigation bar do not cover. The rectangle is in **pixels**; `egui`
/// works in **points**, and the tree's own module does that arithmetic — the
/// application only asks the platform and hands the answer over.
#[cfg(target_os = "android")]
impl Application {
    fn fill_safe_area(&self, ctx: &eframe::egui::Context, raw_input: &mut eframe::egui::RawInput) {
        let Some(handle) = self.platform.as_ref() else { return };
        let rect = handle.content_rect();
        let Some(viewport) = raw_input.screen_rect else { return };
        // The scale factor: the platform's rectangle is in pixels and `egui`
        // works in points. The window's own answer is the first one to ask
        // for, and the context's is the fallback — the context's is the
        // frame before, and the first frame has none.
        let ppp = raw_input
            .viewports
            .get(&raw_input.viewport_id)
            .and_then(|view| view.native_pixels_per_point)
            .unwrap_or_else(|| ctx.input(|i| i.pixels_per_point));
        let platform = craftkit_ui::insets::PlatformRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };
        if let Some(insets) = craftkit_ui::insets::insets(platform, viewport, ppp) {
            raw_input.safe_area_insets = Some(insets);
        }
    }
}

impl eframe::App for Application {
    /// The safe area: the platform's insets, in the points `egui` uses. §247.
    ///
    /// The platform draws its status bar and navigation bar over the window,
    /// and the clock lands on the toolbar's first row (§239). `egui` has the
    /// mechanism for this — the **safe area** — and the tree's job is to fill
    /// it in with what the platform says, so that every panel is laid out
    /// inside the area nothing covers.
    ///
    /// The desktop has no insets, and the hook does nothing there.
    fn raw_input_hook(&mut self, _ctx: &eframe::egui::Context, _raw_input: &mut eframe::egui::RawInput) {
        #[cfg(target_os = "android")]
        self.fill_safe_area(_ctx, _raw_input);
    }

    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Published before the session is touched, and every frame rather than
        // once, because a window can be replaced and a stale context would ask
        // for repaints on a viewport that no longer exists.
        if let Ok(mut slot) = self.ctx_slot.lock()
            && slot.as_ref().map(|held| held.clone() != *ctx).unwrap_or(true)
        {
            *slot = Some(ctx.clone());
        }

        // The platform's own keyboard, behind the gate: the text it
        // committed goes into this frame, as if it had been typed. §257.
        //
        // It is taken out **before** the frame is drawn, so the text lands in
        // the field that has the focus in this same frame.
        for text in ime::take() {
            // The text is logged, because on a phone the platform's keyboard
            // is the only way in and the log is the only place a person can
            // see that it arrived. §257.
            log::info!("the platform's keyboard committed {} characters", text.chars().count());
            ctx.input_mut(|input| input.events.push(eframe::egui::Event::Text(text)));
        }

        let tokens = craftkit_ui::Tokens::dark();
        let Ok(mut session) = self.session.lock() else {
            // A poisoned lock means another thread panicked. The document is
            // still there; refusing to draw would lose more than it protects.
            log::error!("the session lock is poisoned; the document may be mid-edit");
            return;
        };
        // A panic in the drawing code is caught here, where the session is
        // still in hand, and the crash report is written from here. §76.
        let outcome = draw_guarded(&mut session, self.crash_dir.as_deref(), |session| {
            // The shell's memory, shared with the control channel: the screen
            // the channel reports is the screen the window is showing. §106.
            //
            // A poisoned lock means a frame panicked while holding it; the
            // state is still there, and losing it would lose the person's
            // selection for no gain.
            let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let _ = craftkit_ui::frame(ctx, session, &tokens, &mut state);
        });
        if let Err(message) = outcome {
            log::error!("the drawing code panicked ({message}); the crash report has been written");
        }

        // The platform's own keyboard, behind the gate: the platform is asked
        // for its keyboard when a field of the tree's has the focus, and told
        // that it is not wanted when the last one loses it. §257.
        //
        // The answer is the shell's own: it knows which field has the focus,
        // and it is the only part of the tree that does.
        //
        // **And in the gated package the platform's keyboard is the
        // keyboard.** The tree draws a keyboard of its own on a phone
        // (§246), and the gate is the package that does not: there the
        // platform's keyboard is what the person types with, so it is asked
        // for as long as the tree is in front of them. Asking only for a
        // focused field would mean a keyboard that appears and disappears
        // with the focus — and, on a phone that will not let the shell
        // inject a touch (§256), one that never appears at all.
        {
            let Ok(state) = self.state.lock() else { return };
            let wanted = state.typing.field.is_some() || !state.own_keyboard;
            ime::want_keyboard(wanted);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let Ok(session) = self.session.lock() else { return };
        if session.is_dirty() {
            log::warn!("exiting with unsaved changes");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The panic hook is **process-global**, so the tests that install it, or
    /// that panic while it is installed, must not run at the same time.
    ///
    /// They did, and the suite was flaky: the hook that ran for one test's
    /// panic was the one another test had installed, with that test's folder,
    /// so the report landed in the wrong directory. §76.
    static HOOK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Take the hook lock, ignoring a poisoned one.
    ///
    /// A test that panics while holding it poisons it, and the next test must
    /// still be able to run.
    fn hook_lock() -> std::sync::MutexGuard<'static, ()> {
        HOOK_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A request with a method and some parameters.
    fn ui_request_params(method: &str, params: serde_json::Value) -> craftkit_control::Request {
        craftkit_control::Request {
            id: serde_json::json!(1),
            token: None,
            method: method.to_string(),
            params,
        }
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    /// A click by name reaches the widget it names.
    ///
    /// §104: the registry publishes the names of the widgets and the click
    /// took pixels, so an agent that knew a name could not act on it — it
    /// had to guess where the name was.
    #[test]
    fn a_click_by_name_reaches_the_widget_it_names() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui.click", serde_json::json!({ "name": "nodes.row.1.del" }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(response.ok, "the click by name was refused: {:?}", response.message);
        let hit = response.data.as_ref().and_then(|d| d.get("hit")).and_then(|v| v.as_str());
        assert_eq!(hit, Some("nodes.row.1.del"), "the click did not land on the widget");
    }

    /// A click with neither a name nor a point says what it wants.
    #[test]
    fn a_click_with_neither_a_name_nor_a_point_is_refused() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui.click", serde_json::json!({}));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(!response.ok, "a click with no target was accepted");
        assert!(response.message.as_deref().unwrap_or("").contains("name"), "{:?}", response.message);
    }

    /// A click with both a name and a point is refused, not guessed at.
    #[test]
    fn a_click_with_both_a_name_and_a_point_is_refused() {
        let mut session = craftkit_engine::Session::sample();
        let request =
            ui_request_params("ui.click", serde_json::json!({ "name": "nodes.row.1.del", "x": 10, "y": 10 }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(!response.ok, "a click with two targets was accepted");
        assert!(response.message.as_deref().unwrap_or("").contains("not both"), "{:?}", response.message);
    }

    /// The widgets reply says where the widgets are.
    ///
    /// §104: the names alone left an agent with nothing to click.
    #[test]
    fn the_widgets_reply_says_where_the_widgets_are() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui", serde_json::json!({ "action": "widgets" }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(response.ok, "the widgets reply was refused: {:?}", response.message);
        let data = response.data.as_ref().expect("the reply carries data");
        let rects = data.get("rects").and_then(|v| v.as_object()).expect("the rectangles");
        let row = rects.get("nodes.row.1").and_then(|v| v.as_array()).expect("the row's rectangle");
        assert_eq!(row.len(), 4, "a rectangle is x, y, width, height: {row:?}");
        assert!(row[2].as_f64().expect("a width") > 0.0, "the row has no width");
        assert!(data.get("width").is_some(), "the reply does not say what size it is");
    }

    /// The widget names in a reply.
    fn widget_names(response: &craftkit_control::Response) -> Vec<String> {
        response
            .data
            .as_ref()
            .and_then(|d| d.get("widgets"))
            .and_then(|v| v.as_array())
            .map(|list| list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    }

    /// The properties panel is on the screen when a node is selected.
    ///
    /// §106: the channel drew the shell with nothing selected, so the
    /// properties panel was empty and published no editor — and the names §100
    /// gave that panel (`properties.key.path`, `properties.key.value`) and
    /// every property editor's name could never be looked up through the
    /// channel at all.
    #[test]
    fn the_property_editors_are_on_the_screen_when_a_node_is_selected() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui", serde_json::json!({ "action": "widgets", "selected": 1 }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(response.ok, "the widgets reply was refused: {:?}", response.message);
        let names = widget_names(&response);
        assert!(
            names.iter().any(|n| n.starts_with("node.1.prop.")),
            "the selected node's property editors are not on the screen: {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "properties.key.path"),
            "the key field is not on the screen: {names:?}"
        );
    }

    /// The channel reports the screen the **window** is showing.
    ///
    /// §106: the channel and the window share one shell state, so what the
    /// channel says is on screen is what the person is looking at. Before
    /// this, the channel drew a shell of its own — always the empty one.
    #[test]
    fn the_widgets_reply_is_the_screen_the_window_is_showing() {
        let mut session = craftkit_engine::Session::sample();
        let live = craftkit_ui::shell::ShellState {
            selected: Some(craftkit_core::NodeId(1)),
            ..craftkit_ui::shell::ShellState::default()
        };
        let request = ui_request_params("ui", serde_json::json!({ "action": "widgets" }));
        let response = ui_request(&mut session, &request, &live);
        assert!(response.ok, "the widgets reply was refused: {:?}", response.message);
        let names = widget_names(&response);
        assert!(
            names.iter().any(|n| n.starts_with("node.1.prop.")),
            "the window's selection is not in the channel's screen: {names:?}"
        );
        let selected = response.data.as_ref().and_then(|d| d.get("selected")).and_then(|v| v.as_u64());
        assert_eq!(selected, Some(1), "the reply does not say which node is selected");
    }

    /// A node that is not there is refused, not answered with an empty panel.
    #[test]
    fn a_selected_node_that_is_not_there_is_refused() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui", serde_json::json!({ "action": "widgets", "selected": 9999 }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(!response.ok, "a selection of a node that is not there was accepted");
        assert!(
            response.message.as_deref().unwrap_or("").contains("no node with id"),
            "{:?}",
            response.message
        );
    }

    /// A `selected` that is not a node id is refused, not ignored.
    #[test]
    fn a_selected_that_is_not_a_node_id_is_refused() {
        let mut session = craftkit_engine::Session::sample();
        let request =
            ui_request_params("ui", serde_json::json!({ "action": "widgets", "selected": "the first one" }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(!response.ok, "a selection that is not a node id was accepted");
        assert!(response.message.as_deref().unwrap_or("").contains("node id"), "{:?}", response.message);
    }

    /// A click on a disabled widget is refused, however it is asked for.
    ///
    /// §107: the click by point said "that button is at that point", and the
    /// button was greyed out; the click by name said "it is on screen but
    /// disabled, so there is nothing to click". One question, two answers.
    #[test]
    fn a_click_on_a_disabled_widget_is_refused_how_it_is_asked_for() {
        let mut session = craftkit_engine::Session::sample();
        let state = craftkit_ui::shell::ShellState::default();

        // Where the disabled Undo button is, from the widgets reply.
        let widgets = ui_request(
            &mut session,
            &ui_request_params("ui", serde_json::json!({ "action": "widgets" })),
            &state,
        );
        let rects = widgets
            .data
            .as_ref()
            .and_then(|d| d.get("rects"))
            .and_then(|v| v.as_object())
            .expect("the rectangles");
        let rect = rects
            .get("toolbar.history.undo")
            .and_then(|v| v.as_array())
            .expect("the undo button's rectangle");
        let x = rect[0].as_f64().expect("an x") + rect[2].as_f64().expect("a width") / 2.0;
        let y = rect[1].as_f64().expect("a y") + rect[3].as_f64().expect("a height") / 2.0;

        let by_point = ui_request(
            &mut session,
            &ui_request_params("ui", serde_json::json!({ "action": "click", "x": x, "y": y })),
            &state,
        );
        let by_name = ui_request(
            &mut session,
            &ui_request_params(
                "ui",
                serde_json::json!({ "action": "click", "name": "toolbar.history.undo" }),
            ),
            &state,
        );
        assert!(!by_point.ok, "a click on a disabled widget was accepted: {:?}", by_point.message);
        assert!(!by_name.ok, "a click on a disabled widget was accepted: {:?}", by_name.message);
        assert_eq!(
            by_point.message, by_name.message,
            "the two ways of asking about one widget gave two answers"
        );
        assert!(by_point.message.as_deref().unwrap_or("").contains("disabled"), "{:?}", by_point.message);
    }

    /// The widgets reply says which of the widgets cannot be clicked.
    ///
    /// §107: the reply listed every widget and where it was, and a greyed-out
    /// button looked exactly like a live one — so the only way to find out was
    /// to try to click it.
    #[test]
    fn the_widgets_reply_says_which_widgets_cannot_be_clicked() {
        let mut session = craftkit_engine::Session::sample();
        let request = ui_request_params("ui", serde_json::json!({ "action": "widgets" }));
        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(response.ok, "the widgets reply was refused: {:?}", response.message);
        let data = response.data.as_ref().expect("the reply carries data");
        let disabled = data.get("disabled").and_then(|v| v.as_array()).expect("the dead widgets");
        let names: Vec<&str> = disabled.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&"toolbar.history.undo"),
            "the undo button cannot be clicked with nothing to undo, and the reply does not say so: {names:?}"
        );
    }

    /// The channel, over a socket, reports the screen the window is showing.    ///
    /// §106: the channel and the window share one shell state, so what the
    /// channel says is on screen is what the person is looking at. Before this,
    /// the channel drew a shell of its own — always the empty one, so the
    /// properties panel was never in the answer.
    ///
    /// This goes through the socket rather than calling the handler directly,
    /// because the thing that can go wrong is the *wiring*: a handler that is
    /// handed a state of its own instead of the shared one.
    #[test]
    fn the_channel_reports_the_windows_screen_over_the_socket() {
        use std::io::{BufRead, BufReader, Write};

        let session = Arc::new(Mutex::new(craftkit_engine::Session::sample()));
        let state: SharedState = Arc::new(Mutex::new(craftkit_ui::shell::ShellState {
            selected: Some(craftkit_core::NodeId(1)),
            ..craftkit_ui::shell::ShellState::default()
        }));
        let ctx_slot: ContextSlot = Arc::new(Mutex::new(None));

        // A directory of its own, so the test does not write into the
        // user's own control folder.
        let dir = craftkit_foundation::temp::dir("control").expect("a directory for the token");

        let token = craftkit_control::Token::from_text("0123456789abcdef0123").expect("a valid token");
        let (server, _path) = start_control(
            0,
            &session,
            &ctx_slot,
            &state,
            craftkit_control::Auth::Token(token.clone()),
            Some(&dir),
        )
        .expect("binds");

        let mut stream = std::net::TcpStream::connect(("127.0.0.1", server.port())).expect("connects");
        // The read is bounded: a test that waits without a bound cannot catch
        // a hang — it *is* the hang. §224.
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
        let request = serde_json::json!({
            "id": 1,
            "token": "0123456789abcdef0123",
            "method": "ui",
            "params": { "action": "widgets" },
        });
        writeln!(stream, "{request}").expect("sends");
        stream.flush().ok();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).expect("a reply");
        let reply: serde_json::Value = serde_json::from_str(&line).expect("a JSON reply");
        assert_eq!(reply.get("ok"), Some(&serde_json::json!(true)), "{reply}");
        let names: Vec<String> = reply
            .get("data")
            .and_then(|d| d.get("widgets"))
            .and_then(|v| v.as_array())
            .map(|list| list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        assert!(
            names.iter().any(|n| n.starts_with("node.1.prop.")),
            "the channel did not report the window's screen: {names:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_flags_means_an_empty_session() {
        let flags = parse_flags(&args(&[])).expect("ok").expect("not help");
        assert!(!flags.sample);
        assert!(flags.control.is_none());
        assert!(flags.file.is_none());
        assert!(open(&flags).document().nodes.is_empty());
        assert!(!open(&flags).is_dirty(), "a fresh session is not unsaved work");
    }

    #[test]
    fn the_sample_flag_opens_the_example() {
        let flags = parse_flags(&args(&["--sample"])).expect("ok").expect("not help");
        assert!(flags.sample);
        assert_eq!(open(&flags).document().nodes.len(), 3);
    }

    /// The flag that turns the token off needs the channel it turns off.
    ///
    /// §289: the flag is about the control channel, so a launch that asks
    /// for no token and no channel is a launch that says nothing.
    #[test]
    fn the_no_token_flag_needs_the_control_flag() {
        let err = parse_flags(&args(&["--control-no-token"])).expect_err("the flag needs a channel");
        assert!(err.contains("--control"), "{err}");
    }

    /// The flag is remembered, and it does not disturb the port.
    #[test]
    fn the_no_token_flag_is_remembered() {
        let flags =
            parse_flags(&args(&["--control", "4711", "--control-no-token"])).expect("ok").expect("not help");
        assert!(flags.control_no_token);
        assert_eq!(flags.control, Some(4711));
    }

    /// The default is the token.
    #[test]
    fn the_default_asks_for_a_token() {
        let flags = parse_flags(&args(&["--control", "4711"])).expect("ok").expect("not help");
        assert!(!flags.control_no_token);
    }

    #[test]
    fn a_file_is_remembered() {
        let flags = parse_flags(&args(&["plan.craftkit"])).expect("ok").expect("not help");
        assert_eq!(flags.file, Some(PathBuf::from("plan.craftkit")));
    }

    /// The defect this exists for: a command over the control channel changed
    /// the document, and the window went on showing the frame before it.
    ///
    /// The window is started *after* the channel, so for the first few frames —
    /// and for the whole life of a process whose window never opened — the slot
    /// is empty. A command arriving then must not panic or block.
    #[test]
    fn a_command_before_the_window_exists_is_not_an_error() {
        let slot: ContextSlot = Arc::new(Mutex::new(None));
        request_repaint(&slot);
    }

    #[test]
    fn the_control_port_is_parsed_and_its_argument_consumed() {
        let flags = parse_flags(&args(&["--control", "4711", "--sample"])).expect("ok").expect("not help");
        assert_eq!(flags.control, Some(4711));
        assert!(flags.sample, "the flag after the port must survive");
    }

    #[test]
    fn help_and_version_stop_before_anything_else() {
        assert!(parse_flags(&args(&["--help"])).expect("ok").is_none());
        assert!(parse_flags(&args(&["-V"])).expect("ok").is_none());
    }

    #[test]
    fn a_panic_hook_writes_a_report_a_person_can_act_on() {
        // The whole point of the hook. A crash report that nobody can read is
        // worse than none, because it displaces the crash you would otherwise
        // have noticed.
        let dir = craftkit_foundation::temp::dir("crash-probe").expect("a directory for the probe");

        let _lock = hook_lock();

        let session = Arc::new(Mutex::new(Session::sample()));
        let _ = session.lock().expect("fresh").execute(
            "node.add",
            &serde_json::json!({
                "name": "Before", "kind": "source"
            }),
        );
        install_crash_hook_in(&session, Some(dir.clone()));

        // A real panic, on a thread, so the hook is the production one.
        let panic_on = Arc::clone(&session);
        std::thread::spawn(move || {
            let _ = panic_on;
            panic!("deliberate probe");
        })
        .join()
        .ok();

        // Only this test's own panic: the panic hook is process-global, so a
        // panic caught in another test can write a report into this folder.
        let ids = craftkit_engine::crash::list_in(&dir);
        let mine: Vec<String> = ids
            .iter()
            .filter(|id| {
                craftkit_engine::crash::show_in(&dir, id)
                    .map(|r| r.message.contains("deliberate probe"))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        assert_eq!(mine.len(), 1, "expected one report in {}", dir.display());
        let report = craftkit_engine::crash::show_in(&dir, &mine[0]).expect("reads back");
        assert!(report.message.contains("deliberate probe"), "{}", report.message);
        assert_eq!(report.command, "node.add", "the report should name what was running");
        assert!(
            !report.replay.is_empty(),
            "a report with no steps cannot be replayed, which is the point of one"
        );
        // The design must be the document as it was *before* the panic, so it
        // can be recovered.
        // The design is stored as the document itself, so a person can open it
        // again. Four nodes: the three the sample has, plus the one added just
        // before the panic — which is the whole reason the hook runs while the
        // session still exists.
        let nodes = &report.design.nodes;
        assert_eq!(nodes.len(), 4, "the design should be the document at the crash");
        assert!(
            nodes.iter().any(|n| n.name == "Before"),
            "the node added just before the panic should be in the report"
        );

        let _ = std::fs::remove_dir_all(&dir);
        // Put the default hook back for the rest of the suite.
        let _ = std::panic::take_hook();
    }

    /// A panic in the drawing code writes a report.
    ///
    /// The panic hook cannot do this: it runs on the panicking thread, and
    /// the render path holds the session's lock for the length of a frame, so
    /// the hook's `try_lock` fails and it writes nothing. Measured: a panic
    /// while the session was locked wrote **no report at all**. §76.
    #[test]
    fn a_panic_in_the_drawing_code_writes_a_report() {
        let dir = craftkit_foundation::temp::dir("crash-draw").expect("a directory for the crash");

        let _lock = hook_lock();

        let mut session = Session::sample();
        let outcome = draw_guarded(&mut session, Some(dir.as_path()), |_| {
            panic!("a panic in the drawing code");
        });

        let message = outcome.expect_err("the panic is caught and reported");
        assert!(message.contains("a panic in the drawing code"), "{message}");
        let ids = craftkit_engine::crash::list_in(&dir);
        assert_eq!(ids.len(), 1, "expected one report in {}", dir.display());
        let report = craftkit_engine::crash::show_in(&dir, &ids[0]).expect("reads back");
        assert!(report.message.contains("a panic in the drawing code"), "{}", report.message);
        assert!(
            !report.design.nodes.is_empty(),
            "the report should carry the document the user was working on"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A frame that does not panic writes no report and says nothing.
    #[test]
    fn a_frame_that_does_not_panic_writes_no_report() {
        let dir = craftkit_foundation::temp::dir("crash-draw-ok").expect("a directory for the crash");

        let mut session = Session::sample();
        let outcome = draw_guarded(&mut session, Some(dir.as_path()), |_session| {});
        assert!(outcome.is_ok(), "{outcome:?}");
        assert!(craftkit_engine::crash::list_in(&dir).is_empty());
    }

    #[test]
    fn the_hook_survives_a_panic_payload_it_cannot_read() {
        // A panic inside the hook aborts the process and loses the original
        // panic, so the hook has to cope with whatever it is handed — including
        // a payload that is neither a `&str` nor a `String`.
        // A literal `panic!("…")` payload is `&'static str`; `panic!("{}", x)`
        // with a formatted String is `String`. Both are the cases that matter.
        let borrowed: &'static str = "a borrowed str";
        assert_eq!(message_from_payload(&borrowed), "a borrowed str");
        let owned: String = String::from("an owned str");
        assert_eq!(message_from_payload(&owned), "an owned str");
        assert_eq!(
            message_from_payload(&()),
            "a panic with no message",
            "an unreadable payload must still produce something to write down"
        );
        assert_eq!(message_from_payload(&42u32), "a panic with no message");
    }

    #[test]
    fn a_missing_or_bad_port_is_an_error_with_the_usage() {
        let err = parse_flags(&args(&["--control"])).expect_err("refused");
        assert!(err.contains("needs a port"), "{err}");
        let err = parse_flags(&args(&["--control", "banana"])).expect_err("refused");
        assert!(err.contains("port number"), "{err}");
    }

    #[test]
    fn an_unknown_flag_is_refused_rather_than_ignored() {
        let err = parse_flags(&args(&["--frobnicate"])).expect_err("refused");
        assert!(err.contains("--frobnicate"), "{err}");
    }

    #[test]
    fn a_file_that_will_not_open_still_gives_a_usable_session() {
        // The point of the test: an unreadable design must not stop the
        // application from starting.
        let flags = parse_flags(&args(&["/nonexistent/plan.craftkit"])).expect("ok").expect("not help");
        let session = open(&flags);
        assert!(!session.document().nodes.is_empty());
    }

    #[test]
    fn the_usage_mentions_every_flag_the_parser_accepts() {
        for flag in ["--sample", "--control", "--help", "--version"] {
            assert!(USAGE.contains(flag), "the usage text does not mention {flag}");
        }
    }

    /// The sentence an agent reads after clicking, which must not claim an action
    /// that did not happen.
    ///
    /// It said ``clicked `toolbar.node.duplicate` ``, three times in a row against
    /// a live app, with the document byte-identical afterwards. `data.hit` is where
    /// the name belongs; the sentence has to say what a click is.
    #[test]
    fn a_click_reply_does_not_claim_to_have_clicked() {
        let hit = click_message(Some("toolbar.node.duplicate"));
        assert!(!hit.starts_with("clicked"), "the sentence claims an action: {hit}");
        // The name is still reported — that is the whole point of a click.
        assert!(hit.contains("toolbar.node.duplicate"), "{hit}");
        // And the way to actually do it is named, rather than left to be guessed.
        assert!(hit.contains("engine.execute"), "{hit}");

        // The empty case was already honest; keep it that way.
        assert_eq!(click_message(None), "nothing is at that point");
    }

    /// `ui.click` reaches the click, through the handler that answers `ui.*`.
    ///
    /// The unit test above is about the sentence; this one is about the request
    /// getting there at all, which is what was broken: the action defaulted to
    /// `render` and the typed method was never read.
    #[test]
    fn a_ui_click_request_reaches_the_click_handler() {
        let mut session = Session::sample();
        // **The point is found on the screen, not written down.** A
        // hard-coded point stops being on a button the moment the toolbar
        // changes — and it stopped being on one when the save commands
        // joined it (§288) — and a click that hits nothing proves nothing
        // about the handler.
        let frame =
            craftkit_ui::raster::render(&mut session, 800, 600, &craftkit_ui::shell::ShellState::default())
                .expect("a frame");
        let button = frame.automation.rect("toolbar.session.stats").expect("the button is on the screen");
        let (x, y) = (button.center().x, button.center().y);

        let mut request = craftkit_control::Request::new(1, craftkit_control::Method::Ui);
        request.method = "ui.click".to_string();
        request.params = serde_json::json!({ "x": x, "y": y });

        let response = ui_request(&mut session, &request, &craftkit_ui::shell::ShellState::default());
        assert!(response.ok, "{:?}", response.message);
        let message = response.message.clone().unwrap_or_default();
        assert!(!message.starts_with("rendered"), "ui.click was rendered instead of clicked: {message}");
        // A click does not activate, so the document is untouched — and the reply
        // must not have a png in it either.
        let data = response.data.unwrap_or(serde_json::Value::Null);
        assert!(data.get("png_base64").is_none(), "{data}");
        assert!(data.get("hit").is_some(), "no hit was reported: {data}");

        // `ui.render` still renders: the two are not the same request.
        let mut render = craftkit_control::Request::new(2, craftkit_control::Method::Ui);
        render.method = "ui.render".to_string();
        render.params = serde_json::json!({ "width": 320, "height": 240 });
        let response = ui_request(&mut session, &render, &craftkit_ui::shell::ShellState::default());
        let render_message = response.message.unwrap_or_default();
        assert!(render_message.starts_with("rendered"), "{render_message}");
        assert!(response.data.and_then(|d| d.get("png_base64").cloned()).is_some(), "no frame");
    }

    /// A folder for the log's files, and the name of the log in it.
    fn log_folder(tag: &str) -> (PathBuf, PathBuf) {
        let dir = craftkit_foundation::temp::dir(tag).expect("a folder");
        let path = dir.join("craftkit.log");
        (dir, path)
    }

    /// The log's file stops growing at the bound.
    ///
    /// §242: the log was the one file the tree wrote with no limit on it,
    /// and the fix has to be a **bound** — a file that has reached the limit
    /// is moved aside and the log starts a new one.
    #[test]
    fn the_log_file_is_bounded() {
        let (_dir, path) = log_folder("log-bound");
        let mut log = LogFile::open(path.clone()).expect("a log file");

        // Enough lines to pass the bound several times over.
        let line = format!("[INFO] {}\n", "x".repeat(1024));
        for i in 0..4096 {
            log.write_line(&format!("{i} {line}"));
        }

        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        assert!(size <= MAX_LOG_BYTES, "the log's file is {size} bytes, past the bound of {MAX_LOG_BYTES}");

        // And the bound is a bound on the folder, not on one file: the lines
        // before the rotation are still there, under the other name.
        let previous = previous_of(&path);
        assert!(previous.exists(), "the log was rotated, and the old file is gone: {previous:?}");
        let kept = std::fs::metadata(&previous).map(|m| m.len()).unwrap_or(0);
        assert!(kept <= MAX_LOG_BYTES, "the previous file is {kept} bytes");
    }

    /// The rotation keeps the lines it wrote.
    ///
    /// A bound that threw the log away would be a bound that solved the
    /// problem by deleting the evidence, and the lines at the start of a log
    /// are the ones that say what the application was doing before it went
    /// wrong. **One** rotation keeps them: the file before the current one is
    /// removed at the next one, so the tree keeps two files' worth.
    #[test]
    fn the_rotation_keeps_the_lines_it_wrote() {
        let (_dir, path) = log_folder("log-keeps");
        let mut log = LogFile::open(path.clone()).expect("a log file");

        log.write_line("[INFO] the first line\n");
        // Enough to reach the bound once, and no further.
        let filler = format!("[INFO] {}\n", "y".repeat(1024));
        for _ in 0..1200 {
            log.write_line(&filler);
        }
        log.write_line("[INFO] the last line\n");

        let current = std::fs::read_to_string(&path).unwrap_or_default();
        let previous = std::fs::read_to_string(previous_of(&path)).unwrap_or_default();
        assert!(current.contains("the last line"), "the newest line is not in the log's file");
        assert!(previous.contains("the first line"), "the oldest line is not in the previous file");
    }

    /// The folder the log lives in is bounded, not just the file.
    ///
    /// The bound is on what the tree keeps, and the tree keeps two files:
    /// the one being written and the one before it. A third file would be a
    /// third file in the folder the tree writes into, and the folder is on a
    /// phone.
    #[test]
    fn the_logs_folder_is_bounded() {
        let (dir, path) = log_folder("log-folder");
        let mut log = LogFile::open(path.clone()).expect("a log file");

        let filler = format!("[INFO] {}\n", "z".repeat(1024));
        for _ in 0..4096 {
            log.write_line(&filler);
        }

        let mut files = 0usize;
        let mut total = 0u64;
        for entry in std::fs::read_dir(dir).expect("the folder").flatten() {
            let path = entry.path();
            if path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("craftkit.log")) {
                files += 1;
                total += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            }
        }
        assert!(files <= 2, "the log's folder holds {files} files");
        assert!(total <= 2 * MAX_LOG_BYTES, "the log's folder holds {total} bytes");
    }

    /// A log that is opened twice keeps what was there.
    ///
    /// The log is appended to **across runs** — the file is in the tree's
    /// folder, and the folder is where the tree's files live — so the size a
    /// new run starts from is the size the file already has, not zero.
    #[test]
    fn a_log_that_is_opened_again_keeps_what_was_there() {
        let (_dir, path) = log_folder("log-again");
        let mut first = LogFile::open(path.clone()).expect("a log file");
        first.write_line("[INFO] from the first run\n");
        drop(first);

        let mut second = LogFile::open(path.clone()).expect("a log file");
        second.write_line("[INFO] from the second run\n");
        drop(second);

        let text = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(text.contains("from the first run"), "{text}");
        assert!(text.contains("from the second run"), "{text}");
    }

    /// The log's file is private.
    ///
    /// §244: the tree's rule is that **what the program writes is private**,
    /// and the log is a file the program writes — it holds the commands the
    /// user ran and the names of their nodes. A file created with the umask
    /// is a file somebody else on the machine can read.
    #[cfg(unix)]
    #[test]
    fn the_logs_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = log_folder("log-private");
        let log = LogFile::open(path.clone()).expect("a log file");
        drop(log);

        let mode = std::fs::metadata(&path).map(|m| m.permissions().mode()).unwrap_or(0);
        assert!(
            craftkit_foundation::atomic::mode_is_private(mode),
            "the log's file is {mode:04o}, and other users may read it"
        );
    }
}
