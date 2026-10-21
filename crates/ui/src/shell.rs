//! The shell: the panels, drawn around the engine's state.
//!
//! # Every action is a command
//!
//! Look for `self.run(` in this file. There is no other way a panel changes
//! anything. A button does not call `Document::remove`; it calls
//! [`craftkit_engine::Session::execute`] with `node.remove`. That is the entire
//! point of the shell, and it is why the control channel and the MCP server
//! drive exactly what a person drives.
//!
//! # What a panel may not do
//!
//! It may read state and format it. It may not cache a plan value, and it may
//! not decide whether a value is valid. The moment a panel needs to know the
//! answer to a question the evaluator also answers, that question belongs in the
//! evaluator and the panel should show its answer.

use egui::{Context, Layout, Rect, RichText, ScrollArea, Ui, Vec2};

use craftkit_core::NodeId;
use craftkit_engine::{Area, Session, all};

use crate::automation::{Automation, AutomationId, Widget};
use crate::icons;
use crate::theme::Tokens;

/// What the shell remembers between frames.
///
/// Selection, the transient status line, and the text of the "key at the
/// playhead" fields — the things a person typed and the app must not forget
/// because it redrew. The document lives in the [`Session`]; a second copy
/// here would be a second thing to go stale.
///
/// §105: this was "selection and the status line", and the desktop app drew
/// every frame with a *new* shell, so the selection was gone before the next
/// frame and the properties panel could never open.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ShellState {
    /// The selected node, if any.
    pub selected: Option<NodeId>,
    /// The last thing that happened, shown in the status bar.
    pub status: String,
    /// Whether the last command failed, which is what colours the status bar.
    pub status_is_error: bool,
    /// The path typed into the "key at the playhead" field.
    ///
    /// It is not in the document, because it is a path *to* something in the
    /// document and not part of it yet. §105.
    pub key_path: String,
    /// The value typed into the "key at the playhead" field.
    pub key_value: String,
}

/// The shell.
///
/// Holds a borrow of the session for as long as it lives, so a panel physically
/// cannot mutate the document without going through [`Shell::run`].
pub struct Shell<'a> {
    session: &'a mut Session,
    pub state: ShellState,
    /// Rebuilt each frame and dropped after; see [`Automation`].
    pub automation: Automation,
}

impl<'a> Shell<'a> {
    pub fn new(session: &'a mut Session) -> Self {
        Self::with_state(session, ShellState::default())
    }

    /// A shell that starts from a known state.
    ///
    /// A shell draws *a* screen, and which screen depends on its state: the
    /// properties panel draws the selected node's editors, and draws
    /// *"Select a node."* when there is none. A caller that renders the shell
    /// without saying what it is rendering has silently chosen the empty one.
    /// §106.
    pub fn with_state(session: &'a mut Session, state: ShellState) -> Self {
        Self { session, state, automation: Automation::new() }
    }

    /// The session, for a host that needs to look at it after the frame.
    pub fn session(&self) -> &Session {
        self.session
    }

    /// Run one command and record what happened.
    ///
    /// The single funnel. Every panel goes through it, so a failure reads the
    /// same wherever it came from and `is_error` is trustworthy.
    pub fn run(&mut self, command: &str, params: serde_json::Value) {
        match self.session.execute(command, &params) {
            Ok(outcome) => {
                self.state.status = outcome.message.clone();
                self.state.status_is_error = false;
                log::info!("{command}: {}", outcome.message);
            }
            Err(e) => {
                self.state.status = e.to_string();
                self.state.status_is_error = true;
                log::warn!("{command} refused: {e}");
            }
        }
    }

    /// Draw one frame.
    pub fn show(&mut self, ctx: &Context, tokens: &Tokens) {
        // The registry is **this frame's**: the names are rebuilt and the
        // rectangles are the ones drawn just now. §99.
        self.automation.clear();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui, tokens));
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| self.status_bar(ui, tokens));
        egui::SidePanel::left("nodes").show(ctx, |ui| self.node_list(ui, tokens));
        egui::SidePanel::right("properties").show(ctx, |ui| self.properties(ui, tokens));
        egui::CentralPanel::default().show(ctx, |ui| self.plan_and_timeline(ui, tokens));
    }

    fn toolbar(&mut self, ui: &mut Ui, tokens: &Tokens) {
        ui.horizontal_wrapped(|ui| {
            for area in Area::ALL {
                if area == Area::Debug {
                    ui.separator();
                }
                ui.label(RichText::new(area.label()).color(tokens.text_dim).small());
                for spec in all().iter().filter(|s| s.area == area) {
                    let id = AutomationId::toolbar(spec.id);
                    let enabled = (spec.enabled)(self.session);
                    if self.button(ui, tokens, id, spec.icon.0, spec.label, enabled) {
                        self.run(spec.id, serde_json::json!({}));
                    }
                }
            }
        });
    }

    /// One toolbar button, publishing its rectangle for automation.
    fn button(
        &mut self,
        ui: &mut Ui,
        tokens: &Tokens,
        id: AutomationId,
        icon: &str,
        label: &str,
        enabled: bool,
    ) -> bool {
        let colour = if enabled { tokens.text } else { tokens.text_dim };
        let response = ui.add_enabled(enabled, egui::Button::new(RichText::new(label).color(colour)));
        let rect = response.rect;
        icons::icon(ui, icon, 14.0, colour);
        self.automation.publish(id, rect, enabled);
        response.clicked()
    }

    fn node_list(&mut self, ui: &mut Ui, tokens: &Tokens) {
        ui.heading("Nodes");
        ui.separator();

        // Everything the rows need is read out **before** any of them is drawn.
        // Drawing needs `&mut` to the automation registry and the selection,
        // and iterating the document needs `&` to the document; collecting first
        // is what lets both borrows exist.
        let plan = self.session.plan();
        let rows: Vec<(NodeId, String, String, bool, f64)> = self
            .session
            .document()
            .nodes
            .iter()
            .map(|n| {
                let level = plan.and_then(|p| p.state(n.id)).map(|s| s.level_out).unwrap_or(0.0);
                (n.id, n.name.clone(), n.kind.label().to_string(), n.muted, level)
            })
            .collect();

        let mut selected = self.state.selected;
        let mut automation = std::mem::take(&mut self.automation);
        // `Cell` because egui's layout closures capture by move, and these two
        // have to outlive the frame's closures to be acted on afterwards.
        let to_mute = std::cell::Cell::new(None);
        let to_remove = std::cell::Cell::new(None);

        ScrollArea::vertical().show(ui, |ui| {
            for (id, name, kind, muted, level) in &rows {
                let id = *id;
                let is_selected = selected == Some(id);
                ui.horizontal(|ui| {
                    let colour = if is_selected { tokens.accent } else { tokens.text };
                    let response = ui.selectable_label(is_selected, RichText::new(name).color(colour));
                    let row = response.rect;
                    let clicked = response.clicked();
                    // The **track**, not the fill: the fill is as wide as the
                    // level, so at level zero it is a rectangle with no area at
                    // all — a name an agent can look up and never click. §99.
                    let (level_track, _) = draw_level_bar(ui, tokens, *level);
                    automation.publish(AutomationId::node_row(id), row, true);
                    automation.publish(AutomationId::level_bar(id), level_track, true);
                    if clicked {
                        selected = Some(id);
                    }
                    // The buttons are widgets too: a person can click them,
                    // so an agent has to be able to name them. They used to
                    // have no name at all, so "what is at this point?" said
                    // there was nothing where a button was. §100.
                    let mute = ui.small_button("mute");
                    automation.publish(AutomationId::node_mute(id), mute.rect, true);
                    if mute.clicked() {
                        to_mute.set(Some(id));
                    }
                    let remove = ui.small_button("del");
                    automation.publish(AutomationId::node_remove(id), remove.rect, true);
                    if remove.clicked() {
                        to_remove.set(Some(id));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(kind).color(tokens.text_dim).small());
                    if *muted {
                        ui.label(RichText::new("muted").color(tokens.warning).small());
                    }
                });
            }
        });

        self.automation = automation;
        self.state.selected = selected;

        // Deferred: a command here removes a row we were iterating.
        if let Some(id) = to_mute.get() {
            self.run("node.mute", serde_json::json!({ "node": id.get() }));
        }
        if let Some(id) = to_remove.get() {
            self.run("node.remove", serde_json::json!({ "node": id.get() }));
            if self.state.selected == Some(id) {
                self.state.selected = None;
            }
        }
    }

    fn properties(&mut self, ui: &mut Ui, tokens: &Tokens) {
        ui.heading("Properties");
        ui.separator();
        let Some(id) = self.state.selected else {
            ui.label(RichText::new("Select a node.").color(tokens.text_dim));
            return;
        };
        let Some(node) = self.session.node(id) else {
            ui.label(RichText::new("That node is gone.").color(tokens.warning));
            return;
        };

        ui.label(RichText::new(format!("{} · {}", node.name, node.kind.label())).color(tokens.accent));
        ui.add_space(4.0);

        let mut edits: Vec<(String, serde_json::Value)> = Vec::new();
        let mut actions: Vec<(&'static str, serde_json::Value)> = Vec::new();

        // The model's own flatten, so the panel cannot invent a second view of
        // the property tree.
        let rows: Vec<(String, craftkit_model::Property)> =
            node.props.flatten().into_iter().map(|(path, prop)| (path.to_string(), prop.clone())).collect();

        for (path, prop) in rows {
            let widget = AutomationId::property(id, &path);
            match &prop.value {
                craftkit_core::Value::Bool(b) => {
                    let mut value = *b;
                    let response = ui.checkbox(&mut value, path.clone());
                    self.automation.publish(widget, response.rect, true);
                    if response.changed() {
                        edits.push((path.clone(), serde_json::Value::Bool(value)));
                    }
                }
                craftkit_core::Value::Num(n) => {
                    let mut value = *n;
                    let response = ui.add(
                        egui::Slider::new(&mut value, -100.0..=100.0)
                            .text(path.clone())
                            .clamping(egui::SliderClamping::Never),
                    );
                    self.automation.publish(widget, response.rect, true);
                    if response.changed() {
                        edits.push((path.clone(), serde_json::Value::from(value)));
                    }
                }
                craftkit_core::Value::Color(c) => {
                    let mut rgb = [c.r, c.g, c.b];
                    let response = ui.color_edit_button_srgb(&mut rgb);
                    self.automation.publish(widget, response.rect, true);
                    if response.changed() {
                        edits.push((
                            path.clone(),
                            serde_json::Value::from(format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])),
                        ));
                    }
                }
                other => {
                    let mut text = other.display();
                    let response = ui.add(egui::TextEdit::singleline(&mut text).hint_text(path.clone()));
                    self.automation.publish(widget, response.rect, true);
                    if response.changed() {
                        edits.push((path.clone(), serde_json::Value::from(text)));
                    }
                }
            }

            if prop.is_animated() {
                ui.label(
                    RichText::new(format!("{} keyframe(s)", prop.keys.len())).color(tokens.text_dim).small(),
                );
                let key = AutomationId::clear_keys(id, &path);
                let response = ui.small_button("clear keys");
                self.automation.publish(key, response.rect, true);
                if response.clicked() {
                    actions
                        .push(("prop.reset", serde_json::json!({ "node": id.get(), "path": path.clone() })));
                }
            }
        }

        ui.separator();
        ui.label(RichText::new("Key at the playhead").color(tokens.text_dim));
        // The text lives in the shell's state, not in a local: a local is
        // rebuilt every frame, and the person typing would watch every
        // keystroke disappear. §105.
        let path_field =
            ui.add(egui::TextEdit::singleline(&mut self.state.key_path).hint_text("path, e.g. amount"));
        self.automation.publish(AutomationId::key_path(), path_field.rect, true);
        // The value field is here while there is a path to put a value on.
        // It used to appear only on the frame the path changed, which meant
        // it was gone before anything could be typed into it. §105.
        if !self.state.key_path.is_empty() {
            let value_field =
                ui.add(egui::TextEdit::singleline(&mut self.state.key_value).hint_text("value"));
            self.automation.publish(AutomationId::key_value(), value_field.rect, true);
            if value_field.changed() {
                actions.push((
                    "key.set",
                    serde_json::json!({
                        "node": id.get(),
                        "path": self.state.key_path.clone(),
                        "value": self.state.key_value.clone(),
                        "time": self.session.at().seconds_f64(),
                    }),
                ));
            }
        }

        for (path, value) in edits {
            self.run("prop.set", serde_json::json!({ "node": id.get(), "path": path, "value": value }));
        }
        for (command, params) in actions {
            self.run(command, params);
        }
    }

    fn plan_and_timeline(&mut self, ui: &mut Ui, tokens: &Tokens) {
        let meta = &self.session.document().meta;
        // The frame rate is the design's own, and the panel uses it: the
        // frames below are counted in it. A rate of zero would divide by
        // zero, and the model refuses one — the reader refuses a file that
        // carries one, the command refuses to set one, and the engine's own
        // commands use the rate the same way. §89.
        let rate = meta.frame_rate();
        // The frames the design **has**: its range, not its duration. §115:
        // a timeline built on the duration draws the handle in the wrong
        // place and cannot reach the design's last frames.
        let frames = meta.frame_range();

        ui.horizontal(|ui| {
            ui.label(RichText::new(meta.name.clone()).strong().color(tokens.text));
            ui.label(
                RichText::new(format!("{} fps · {:.2}s", meta.fps, meta.duration().seconds_f64()))
                    .color(tokens.text_dim),
            );
        });
        // The design's own note, if it has one. §110: it was in the file and
        // in the model and in nothing else, so a note could only be read by
        // opening the file that held it.
        //
        // The window shows a glimpse — a note is a paragraph and the panel is
        // not a page — and the state carries the whole of it.
        if !meta.comment.is_empty() {
            ui.label(RichText::new(note_preview(&meta.comment)).color(tokens.text_dim).small().weak());
        }
        ui.separator();

        // The playhead is a place in the design, so the slider is the design's
        // range of frames. The value is clamped into it: the engine keeps the
        // playhead inside the range, and a slider that was handed a value
        // outside it would draw a handle that lies. §115.
        let mut frame = self.session.at().frames_i64(rate).clamp(*frames.start(), *frames.end());
        // The value is not shown: egui would draw it as a number field beside
        // the rail, and that field is a widget of its own — one the registry
        // would not name, and one that edits the playhead behind the
        // command's back. The readout below says the same thing without
        // being a second control. §115.
        let response = ui.add(egui::Slider::new(&mut frame, frames.clone()).show_value(false));
        // The playhead is a widget, and it is the timeline's one control.
        //
        // The slider is on a line of its own: sharing a line with the readout
        // below would make the published rectangle cover the readout too, and
        // a rectangle that covers two widgets is a rectangle that answers for
        // the wrong one. §115.
        self.automation.publish(AutomationId::playhead(), response.rect, true);
        if response.changed() {
            self.run("playback.seek", serde_json::json!({ "frame": frame }));
        }
        // Where the playhead is, in both of the numbers a design is measured
        // in. §115: the slider's own formatter never ran — the value was
        // hidden — so the timeline said nothing at all about where the
        // playhead was.
        let at = self.session.at();
        ui.label(
            RichText::new(playhead_readout(at.frames_i64(rate), at.seconds_f64()))
                .color(tokens.text_dim)
                .small(),
        );

        let Some(plan) = self.session.plan() else {
            ui.label(RichText::new("No plan yet.").color(tokens.warning));
            return;
        };

        if !plan.diagnostics.is_empty() {
            ui.separator();
            ui.label(RichText::new("Problems").color(tokens.warning));
            ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                for d in &plan.diagnostics.items {
                    let colour = if d.severity == craftkit_eval::Severity::Error {
                        tokens.error
                    } else {
                        tokens.warning
                    };
                    ui.label(RichText::new(d.render()).color(colour).small());
                }
                if plan.diagnostics.dropped > 0 {
                    ui.label(
                        RichText::new(format!("{} more not listed", plan.diagnostics.dropped))
                            .color(tokens.text_dim)
                            .small(),
                    );
                }
            });
        }

        ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(RichText::new(plan.summary()).color(tokens.text_dim).small());
        });
    }

    fn status_bar(&mut self, ui: &mut Ui, tokens: &Tokens) {
        let message = self.state.status.clone();
        let is_error = self.state.status_is_error;
        let dirty = self.session.is_dirty();
        let nodes = self.session.document().nodes.len();

        ui.horizontal(|ui| {
            if message.is_empty() {
                ui.label(RichText::new("Ready.").color(tokens.text_dim));
            } else {
                let colour = if is_error { tokens.error } else { tokens.text_dim };
                ui.label(RichText::new(&message).color(colour));
            }
            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                ui.label(
                    RichText::new(if dirty { "unsaved" } else { "saved" })
                        .color(if dirty { tokens.warning } else { tokens.text_dim })
                        .small(),
                );
                ui.label(RichText::new(format!("{nodes} nodes")).color(tokens.text_dim).small());
            });
        });
    }
}

/// A small bar showing a level, coloured by magnitude.
///
/// Free rather than a method so it can be called from inside the closure the
/// node row is drawn in, without the row holding a borrow of the shell.
fn draw_level_bar(ui: &mut Ui, tokens: &Tokens, level: f64) -> (Rect, Rect) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(64.0, 10.0), egui::Sense::hover());
    let width = rect.width() * Tokens::ramp_fraction(level);
    ui.painter().rect_filled(rect, 2.0, tokens.meter_low);
    let fill = Rect::from_min_size(rect.min, Vec2::new(width, rect.height()));
    ui.painter().rect_filled(fill, 2.0, tokens.meter(level));
    (rect, fill)
}

/// Draw one frame, carrying the shell's state from the last one.
///
/// The host keeps the [`ShellState`] and the [`Automation`] registry; it must
/// not keep the [`Shell`], whose borrow ends here.
///
/// The state is the selection, the status line and the text of the "key at
/// the playhead" fields — the things the shell remembers between frames. A
/// host that draws with a fresh shell every frame must **keep** it and hand it
/// back in, or the selection is gone before the next frame and the
/// properties panel can never open. §105.
pub fn frame(ctx: &Context, session: &mut Session, tokens: &Tokens, state: &mut ShellState) -> Automation {
    let mut shell = Shell::new(session);
    // Copied in and copied out rather than moved, so that a panic in the
    // drawing code leaves the host's state as it was: the desktop app catches
    // that panic, and the person who caused it should not also lose their
    // selection. The copy is two short strings and a flag — nothing next to
    // the milliseconds a frame costs elsewhere (§103).
    shell.state = state.clone();
    shell.show(ctx, tokens);
    *state = shell.state.clone();
    shell.automation
}

/// A glimpse of the design's note, for the window.
///
/// The note is a paragraph and the panel is not a page, so the window shows
/// the first 200 characters and says that it is cut. The whole note is in the
/// state. §110.
pub fn note_preview(note: &str) -> String {
    const NOTE_PREVIEW_CHARS: usize = 200;
    if note.chars().count() <= NOTE_PREVIEW_CHARS {
        return note.to_string();
    }
    let mut out: String = note.chars().take(NOTE_PREVIEW_CHARS).collect();
    out.push('…');
    out
}

/// Where the playhead is, in the two numbers a design is measured in.
///
/// The frame number and the time. They are different numbers, and the
/// design's rate is what turns one into the other — so showing one without
/// the other hides the drift between them. §89: the tick rate does not
/// divide by every rate a design may have, and the drift is only visible if
/// both numbers are on the screen.
///
/// §115: the frame number was shown **nowhere**. The slider's value was
/// hidden, and its formatter — the thing that would have written the time —
/// never ran, because egui only calls the formatter for the value it draws.
pub fn playhead_readout(frame: i64, seconds: f64) -> String {
    format!("frame {frame} · {seconds:.2}s")
}

/// Whether a widget is present and clickable at `name`.
pub fn clickable(registry: &Automation, name: &str) -> bool {
    matches!(registry.known(name), Widget::Found { enabled: true, .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draw two frames and hand back what the shell published.
    ///
    /// The shell is built **outside** `Context::run` and borrowed by the
    /// closure, rather than built inside the closure: that way the borrow of the
    /// session lasts for the whole test and the published registry outlives the
    /// frame it came from.
    fn draw(session: &mut Session) -> (ShellState, Automation) {
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(session);
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| shell.show(ctx, &tokens));
        }
        (shell.state, shell.automation)
    }

    #[test]
    fn a_level_maps_into_the_bar_fraction_through_the_theme() {
        // One definition, shared with the colour ramp: two ways to turn a level
        // into a fraction is two ways for them to disagree about a NaN.
        assert_eq!(Tokens::ramp_fraction(0.0), 0.0);
        assert_eq!(Tokens::ramp_fraction(craftkit_core::MAX_MAGNITUDE), 1.0);
        assert_eq!(Tokens::ramp_fraction(-craftkit_core::MAX_MAGNITUDE), 1.0);
        assert_eq!(Tokens::ramp_fraction(f64::NAN), 0.0);
    }

    #[test]
    fn a_command_that_works_sets_the_status_and_not_the_error_flag() {
        let mut session = Session::sample();
        let mut shell = Shell::new(&mut session);
        shell.run("node.add", serde_json::json!({ "name": "From the shell", "kind": "cue" }));
        assert!(!shell.state.status_is_error, "{}", shell.state.status);
        assert!(shell.state.status.contains("From the shell"), "{}", shell.state.status);
        assert_eq!(shell.session().document().nodes.len(), 4);
    }

    #[test]
    fn a_command_that_is_refused_sets_the_error_flag_and_changes_nothing() {
        let mut session = Session::sample();
        let before = session.document().nodes.len();
        let mut shell = Shell::new(&mut session);
        shell.run("node.add", serde_json::json!({ "name": "X", "kind": "not-a-kind" }));
        assert!(shell.state.status_is_error);
        assert!(shell.state.status.contains("not-a-kind"), "{}", shell.state.status);
        assert_eq!(shell.session().document().nodes.len(), before);
    }

    #[test]
    fn the_shell_is_the_only_thing_that_runs_commands() {
        let mut session = Session::sample();
        let mut shell = Shell::new(&mut session);
        shell.run("node.add", serde_json::json!({ "name": "A", "kind": "cue" }));
        shell.run("history.undo", serde_json::json!({}));
        assert_eq!(shell.session().document().nodes.len(), 3);
        assert!(shell.state.status.contains("undid"), "{}", shell.state.status);
    }

    #[test]
    fn every_registered_command_has_a_button() {
        let mut session = Session::sample();
        let (_state, registry) = draw(&mut session);
        for spec in all() {
            let button = AutomationId::toolbar(spec.id);
            let id = button.as_str();
            assert!(registry.known(id) != Widget::Missing, "`{id}` is registered but has no button");
        }
    }

    #[test]
    fn every_node_has_a_row_and_a_level_bar() {
        let mut session = Session::sample();
        let ids: Vec<NodeId> = session.document().nodes.iter().map(|n| n.id).collect();
        let (_state, registry) = draw(&mut session);
        for id in ids {
            assert!(registry.known(AutomationId::node_row(id).as_str()) != Widget::Missing);
            assert!(
                registry.known(AutomationId::level_bar(id).as_str()) != Widget::Missing,
                "node {id} has no level bar"
            );
        }
    }

    #[test]
    fn a_disabled_command_publishes_a_button_that_cannot_be_clicked() {
        // `history.undo` on a fresh session is disabled. Automation must say so
        // rather than offering a point that would do nothing.
        let mut session = Session::new("fresh");
        let (_state, registry) = draw(&mut session);
        let button = AutomationId::toolbar("history.undo");
        let id = button.as_str();
        assert!(!clickable(&registry, id), "{id} should not be clickable");
    }

    #[test]
    fn an_enabled_command_is_clickable() {
        let mut session = Session::sample();
        let (_state, registry) = draw(&mut session);
        assert!(clickable(&registry, AutomationId::toolbar("node.add").as_str()));
    }

    #[test]
    fn a_property_editor_appears_once_something_is_selected() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;

        let (mut state, registry) = draw(&mut session);
        assert!(state.selected.is_none());
        assert_eq!(registry.known(AutomationId::property(first, "amount").as_str()), Widget::Missing);

        state.selected = Some(first);
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(&mut session);
        shell.state = state;
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| shell.show(ctx, &tokens));
        }
        assert_ne!(shell.automation.known(AutomationId::property(first, "amount").as_str()), Widget::Missing);
    }

    /// Draw the shell with a node selected, and hand back the registry.
    ///
    /// The properties panel only draws when something is selected, so the
    /// tests that look at a property's editor have to select one.
    fn draw_selected(session: &mut Session, id: NodeId) -> Automation {
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(session);
        shell.state.selected = Some(id);
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| shell.show(ctx, &tokens));
        }
        shell.automation
    }

    /// Every property of the selected node has an editor in the registry.
    ///
    /// §99: the node list published the level bar under
    /// `node.<id>.prop.$level` — a **property's** name. A property may be
    /// called `$level`, and when it was, the property's editor and the level
    /// bar wanted the same name. The first name published in a frame wins and
    /// the node list is drawn before the properties panel, so the property's
    /// editor was **never published at all**: an agent asking for
    /// `node.1.prop.$level` was answered with the level bar's rectangle, in
    /// the wrong panel, and the property had no editor to click.
    ///
    /// The count alone cannot see it — the level bar was wearing the
    /// property's name, so the numbers matched — which is why the frame is
    /// also checked for collisions, in
    /// [`a_frame_publishes_no_name_twice`].
    #[test]
    fn every_property_of_the_selected_node_has_an_editor() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let created = session.execute(
            "prop.set",
            &serde_json::json!({ "node": first.get(), "path": "$level", "value": "0.5" }),
        );
        assert!(created.is_ok(), "a property may be called `$level`: {created:?}");

        let registry = draw_selected(&mut session, first);
        let paths: Vec<String> = session
            .node(first)
            .map(|n| n.props.flatten().into_iter().map(|(path, _)| path.to_string()).collect())
            .unwrap_or_default();
        assert!(paths.contains(&"$level".to_string()), "the probe property is missing: {paths:?}");
        for path in &paths {
            let name = AutomationId::property(first, path);
            assert_ne!(
                registry.known(name.as_str()),
                Widget::Missing,
                "the property `{path}` has no editor in the registry"
            );
        }
    }

    /// A frame publishes no name twice.
    ///
    /// §99: two widgets wanting one name is a collision, and the loser is not
    /// in the registry — so a lookup for it succeeds and answers with the
    /// *other* widget. The level bar wanted `node.<id>.prop.$level`, which
    /// is a property's name, and a property called `$level` wanted it too.
    ///
    /// This is the general shape of the bug: it is not about `$level` but
    /// about two widgets sharing a name, so the check is over the whole
    /// frame and not over one property.
    #[test]
    fn a_frame_publishes_no_name_twice() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let created = session.execute(
            "prop.set",
            &serde_json::json!({ "node": first.get(), "path": "$level", "value": "0.5" }),
        );
        assert!(created.is_ok(), "{created:?}");
        let registry = draw_selected(&mut session, first);
        assert_eq!(
            registry.collisions(),
            &[] as &[AutomationId],
            "two widgets want the same name: {:?}",
            registry.collisions()
        );
    }

    /// The level bar and a property are two widgets, and they have two names.
    ///
    /// §99: they shared one name, and the property's editor lost.
    #[test]
    fn a_property_named_after_the_level_bar_keeps_its_own_editor() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let created = session.execute(
            "prop.set",
            &serde_json::json!({ "node": first.get(), "path": "$level", "value": "0.5" }),
        );
        assert!(created.is_ok(), "{created:?}");

        let registry = draw_selected(&mut session, first);
        let property = AutomationId::property(first, "$level");
        let bar = AutomationId::level_bar(first);
        let property_rect = registry.rect(property.as_str());
        let bar_rect = registry.rect(bar.as_str());
        assert!(property_rect.is_some(), "the property's editor is not published");
        assert!(bar_rect.is_some(), "the level bar is not published");
        assert_ne!(
            property_rect, bar_rect,
            "the level bar and the property named `$level` are the same widget"
        );
    }

    /// A widget an agent can look up must have an area to click.
    ///
    /// §99: the level bar published the **fill** — the coloured part, whose
    /// width is the level. At level zero that is a rectangle with no width at
    /// all, so the name resolved to a point no click could ever land on.
    /// Measured before the fix: `node.1.prop.$level` resolved to
    /// `[[54.3 61.0] - [54.3 71.0]]` — zero wide.
    #[test]
    fn a_level_bar_publishes_a_rectangle_with_an_area() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let registry = draw_selected(&mut session, first);
        let bar = AutomationId::level_bar(first);
        let rect = registry.rect(bar.as_str()).unwrap_or_else(|| panic!("`{bar}` is not published"));
        assert!(rect.width() > 0.0, "the level bar's rectangle has no width: {rect:?}");
        assert!(rect.height() > 0.0, "the level bar's rectangle has no height: {rect:?}");
    }

    /// The registry is one frame's worth of widgets.
    ///
    /// §99: it was never cleared, so it grew for as long as the shell lived
    /// and "the first publication wins" meant "the first **frame** wins" — a
    /// widget that had moved kept the rectangle it had when it was first
    /// drawn, and a widget that had gone kept its rectangle too.
    ///
    /// Measured before the fix: with a node selected, the registry held 31
    /// widgets; the selection was then cleared, the properties panel drew
    /// nothing, and the registry still held all 31 — `node.1.prop.amount`
    /// was still "found", at a rectangle where no editor was.
    #[test]
    fn the_registry_holds_the_frame_that_was_drawn() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(&mut session);

        shell.state.selected = Some(first);
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| shell.show(ctx, &tokens));
        }
        let editor = AutomationId::property(first, "amount");
        assert_ne!(
            shell.automation.known(editor.as_str()),
            Widget::Missing,
            "the editor was not published in the first place"
        );

        // The selection goes: the properties panel draws nothing now.
        shell.state.selected = None;
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| shell.show(ctx, &tokens));
        }
        assert_eq!(
            shell.automation.known(editor.as_str()),
            Widget::Missing,
            "the editor is gone from the screen but the registry still has it"
        );
    }

    #[test]
    fn every_node_row_has_its_buttons_named() {
        let mut session = Session::sample();
        let ids: Vec<NodeId> = session.document().nodes.iter().map(|n| n.id).collect();
        let registry = draw_selected(&mut session, ids[0]);
        for id in ids {
            for name in [AutomationId::node_mute(id), AutomationId::node_remove(id)] {
                let rect =
                    registry.rect(name.as_str()).unwrap_or_else(|| panic!("`{name}` is not published"));
                assert!(rect.width() > 0.0, "`{name}` has no width: {rect:?}");
                let centre = rect.center();
                assert_eq!(
                    registry.at(centre.x, centre.y).map(|hit| hit.name),
                    Some(name.as_str()),
                    "`{name}` is published but the registry answers for something else there"
                );
            }
        }
    }

    /// The timeline has a name.
    ///
    /// §100: the playhead slider was drawn and published nowhere. It is the
    /// timeline's one control, and a control an agent cannot name is a
    /// control it cannot move.
    #[test]
    fn the_timeline_has_a_name() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let registry = draw_selected(&mut session, first);
        let name = AutomationId::playhead();
        let rect = registry.rect(name.as_str()).unwrap_or_else(|| panic!("`{name}` is not published"));
        assert!(rect.width() > 0.0, "`{name}` has no width: {rect:?}");
    }

    /// The "key at the playhead" fields have names.
    ///
    /// §100: the path and value fields were drawn and published nowhere, so
    /// a field an agent could not name was a field it could not fill.
    #[test]
    fn the_key_fields_have_names() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let registry = draw_selected(&mut session, first);
        for name in [AutomationId::key_path(), AutomationId::key_value()] {
            // The value field only appears once the path has been typed into,
            // so only the path is on screen in a fresh frame.
            if name == AutomationId::key_value() {
                continue;
            }
            let rect = registry.rect(name.as_str()).unwrap_or_else(|| panic!("`{name}` is not published"));
            assert!(rect.width() > 0.0, "`{name}` has no width: {rect:?}");
        }
    }

    /// Every name the registry publishes answers at its own centre.
    ///
    /// §100: this is the general shape of the bug — the registry and the
    /// screen can disagree. A name whose widget is somewhere else, or is
    /// covered by another, is a name that points at the wrong thing.
    #[test]
    fn every_published_name_answers_at_its_own_centre() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        // An animated property, so the "clear keys" button is drawn too.
        let keyed = session.execute(
            "key.set",
            &serde_json::json!({ "node": first.get(), "path": "amount", "value": "0.5", "time": 48 }),
        );
        assert!(keyed.is_ok(), "{keyed:?}");
        let registry = draw_selected(&mut session, first);
        assert!(!registry.names().is_empty());
        for name in registry.names() {
            let Some(rect) = registry.rect(name) else {
                continue;
            };
            let centre = rect.center();
            assert_eq!(
                registry.at(centre.x, centre.y).map(|hit| hit.name),
                Some(name),
                "the registry publishes `{name}` but answers for something else at its centre"
            );
        }
    }

    /// A name never promises a widget that is not drawn.
    ///
    /// §100: the "clear keys" button was published as `node.<id>.key.<path>`,
    /// which is the name of a **keyframe row** — a widget the shell does not
    /// draw. An agent that found the name would look for a row, and the
    /// button that answers for it says nothing of the kind.
    #[test]
    fn a_name_never_promises_a_widget_that_is_not_drawn() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let keyed = session.execute(
            "key.set",
            &serde_json::json!({ "node": first.get(), "path": "amount", "value": "0.5", "time": 48 }),
        );
        assert!(keyed.is_ok(), "{keyed:?}");
        let registry = draw_selected(&mut session, first);

        // The button is there, under a name that says what it is.
        let clear = AutomationId::clear_keys(first, "amount");
        assert_ne!(registry.known(clear.as_str()), Widget::Missing, "the clear-keys button is not published");
        // And the name of the row that does not exist is nowhere.
        let row = format!("node.{}.key.amount", first.get());
        assert_eq!(
            registry.known(&row),
            Widget::Missing,
            "`{row}` promises a keyframe row, and there is no such widget"
        );
    }

    /// One frame through [`frame`], with the given input.
    ///
    /// The state is carried the way a host carries it: in and out, so that a
    /// frame can see what the last one remembered. §105.
    fn one_frame(
        ctx: &Context,
        session: &mut Session,
        tokens: &Tokens,
        state: &mut ShellState,
        input: egui::RawInput,
    ) -> Automation {
        let mut out = None;
        let _ = ctx.run(input, |ctx| {
            out = Some(frame(ctx, session, tokens, state));
        });
        out.expect("the frame ran")
    }

    /// The state survives from one frame to the next.
    ///
    /// §105: the desktop app draws with `frame()`, which builds a shell for
    /// the frame and drops it — so the shell's state (the selection, the
    /// status line) was gone before the next frame, and the properties panel
    /// could never open.
    #[test]
    fn a_selection_made_in_one_frame_is_there_in_the_next() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        // Where the first row is: draw a frame and look.
        let mut state = ShellState::default();
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let rect = automation.rect("nodes.row.1").expect("the first row has a rectangle");
        // Frame one: press on it.
        let press = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        one_frame(&ctx, &mut session, &tokens, &mut state, press);
        // And release: a click is a press and a release.
        let release = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: rect.center(),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        };
        one_frame(&ctx, &mut session, &tokens, &mut state, release);
        assert!(state.selected.is_some(), "the click did not select the node");
        // Frame three: nothing happens, and the selection must still be there.
        one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        assert!(state.selected.is_some(), "the selection was lost between frames");
    }

    /// A path typed into the "key at the playhead" field is still there in
    /// the next frame.
    ///
    /// §105: the field's text was a local, rebuilt every frame, so every
    /// keystroke was thrown away — and the value field, which only appeared
    /// on the frame the path changed, was gone before anything could be typed
    /// into it.
    #[test]
    fn a_path_typed_into_the_key_field_is_still_there_in_the_next_frame() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(first), ..ShellState::default() };
        // Where the path field is.
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let rect =
            automation.rect(AutomationId::key_path().as_str()).expect("the path field has a rectangle");
        // Click it, so that it has the focus a typed character goes to.
        for pressed in [true, false] {
            let click = egui::RawInput {
                events: vec![egui::Event::PointerButton {
                    pos: rect.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            };
            one_frame(&ctx, &mut session, &tokens, &mut state, click);
        }
        // And type a path.
        let typed = egui::RawInput { events: vec![egui::Event::Text("amount".into())], ..Default::default() };
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, typed);
        assert!(
            automation.rect(AutomationId::key_value().as_str()).is_some(),
            "the value field did not appear once there was a path"
        );
        // The next frame: the path is still typed.
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        assert!(
            automation.rect(AutomationId::key_value().as_str()).is_some(),
            "the typed path was thrown away when the frame was redrawn"
        );
    }

    /// Press at a point and release it there.
    ///
    /// A click is a press and a release, and egui takes its input once per
    /// frame: the two halves have to be two frames.
    fn press_at(
        ctx: &Context,
        session: &mut Session,
        tokens: &Tokens,
        state: &mut ShellState,
        x: f32,
        y: f32,
    ) {
        for pressed in [true, false] {
            let input = egui::RawInput {
                events: vec![egui::Event::PointerButton {
                    pos: egui::pos2(x, y),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            };
            one_frame(ctx, session, tokens, state, input);
        }
    }

    /// The pixels of a frame in a rectangle.
    ///
    /// The frame is a PNG in memory and the rectangle is in the same points
    /// the frame was drawn in, so the two line up one to one.
    fn crop(png: &[u8], x: f32, y: f32, w: f32, h: f32) -> Vec<u8> {
        let image = image::load_from_memory(png).expect("the frame is a png").to_rgba8();
        let mut out = Vec::new();
        for row in y as u32..(y + h) as u32 {
            for column in x as u32..(x + w) as u32 {
                out.extend_from_slice(&image.get_pixel(column, row).0);
            }
        }
        out
    }

    /// The timeline can reach both ends of the design.
    ///
    /// §115: the slider's range was the design's **duration** — how many
    /// frames the design is long — and the value it held was an absolute
    /// frame. A design that starts one second in has frames 24..=120 at 24 fps
    /// and a duration of 96 frames, so the slider could not show the design's
    /// first frame and could not reach its last: pressing the far right of it
    /// stopped at frame 96, twenty-four frames short of the end.
    #[test]
    fn the_timeline_can_reach_both_ends_of_the_design() {
        let mut session = Session::sample();
        session
            .execute("doc.range", &serde_json::json!({ "start": 1.0, "end": 5.0 }))
            .expect("the range can be set");
        let rate = session.document().meta.frame_rate();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();
        // Two frames: the first lays the panels out, the second places them.
        one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let rect = automation.rect(AutomationId::playhead().as_str()).expect("the playhead has a rectangle");

        press_at(&ctx, &mut session, &tokens, &mut state, rect.right() - 0.5, rect.center().y);
        assert_eq!(
            session.at().frames_i64(rate),
            session.document().meta.end.frames_i64(rate),
            "the far right of the timeline is not the design's last frame"
        );

        press_at(&ctx, &mut session, &tokens, &mut state, rect.left() + 0.5, rect.center().y);
        assert_eq!(
            session.at().frames_i64(rate),
            session.document().meta.start.frames_i64(rate),
            "the far left of the timeline is not the design's first frame"
        );
    }

    /// The timeline says where the playhead is.
    ///
    /// §115: the slider's own formatter never ran — egui only calls the
    /// formatter for the value it draws, and the value was hidden — so the
    /// timeline showed no time and no frame number at all.
    ///
    /// The check is on the pixels **below** the slider, because that region
    /// holds the readout and nothing else: the handle is above it and the
    /// plan's summary is at the bottom of the panel. A comparison of the
    /// whole frames would pass even if the readout were never drawn, because
    /// the handle moves too — §110's lesson, in a new place.
    #[test]
    fn the_timeline_says_where_the_playhead_is() {
        let mut session = Session::sample();
        session
            .execute("doc.range", &serde_json::json!({ "start": 1.0, "end": 5.0 }))
            .expect("the range can be set");
        let state = ShellState::default();

        session.execute("playback.seek", &serde_json::json!({ "frame": 24 })).expect("the playhead can move");
        let first = crate::raster::render(&mut session, 600, 400, &state).expect("the frame renders");

        session
            .execute("playback.seek", &serde_json::json!({ "frame": 120 }))
            .expect("the playhead can move");
        let second = crate::raster::render(&mut session, 600, 400, &state).expect("the frame renders");

        let rect =
            first.automation.rect(AutomationId::playhead().as_str()).expect("the playhead has a rectangle");
        let below =
            |frame: &crate::raster::Frame| crop(&frame.png, rect.left(), rect.bottom() + 4.0, 160.0, 14.0);
        assert_ne!(
            below(&first),
            below(&second),
            "the timeline says the same thing about two playheads a hundred frames apart"
        );
    }

    /// The readout says the frame and the time.
    ///
    /// Two numbers, because they are two numbers: the frame is the design's
    /// grid and the time is the wall clock, and the rate is what turns one
    /// into the other. §89: the drift between them is only visible if both
    /// are shown.
    #[test]
    fn the_playhead_readout_says_the_frame_and_the_time() {
        assert_eq!(playhead_readout(24, 1.0), "frame 24 · 1.00s");
        assert_eq!(playhead_readout(0, 0.0), "frame 0 · 0.00s");
    }

    #[test]
    fn the_shell_survives_an_empty_document() {
        let mut session = Session::new("nothing here");
        let (state, registry) = draw(&mut session);
        assert!(!state.status_is_error, "{}", state.status);
        assert!(registry.names().contains(&"toolbar.node.add"));
    }

    #[test]
    fn drawing_does_not_change_the_document() {
        let mut session = Session::sample();
        let before = session.document().clone();
        let _ = draw(&mut session);
        assert_eq!(session.document(), &before, "a frame must be side-effect free");
    }

    /// The note is a glimpse, and it says that it is a glimpse.
    ///
    /// §110: a note is a paragraph and the panel is not a page. The window
    /// shows the first 200 characters and marks the cut; the whole note is in
    /// the state.
    #[test]
    fn the_note_preview_is_a_glimpse_of_the_note() {
        assert_eq!(note_preview(""), "");
        assert_eq!(note_preview("short"), "short");

        let long = "n".repeat(500);
        let preview = note_preview(&long);
        assert!(preview.ends_with('…'), "a cut note says that it was cut: {preview}");
        assert_eq!(preview.chars().count(), 201, "the preview is 200 characters and the mark");
        assert!(long.starts_with(preview.trim_end_matches('…')), "the preview is not the note's head");
    }

    /// A frame with a note is not the frame without one.
    ///
    /// §110: the note was in the model and in the file and **nowhere else** —
    /// not in the window. The check is on the pixels, because a note that is
    /// computed and never drawn would pass every test that looked at the
    /// model.
    ///
    /// The two frames differ in **one** thing: the text of the note. That is
    /// on purpose. A frame with a note and a frame without one also differ in
    /// the status bar — the second is "unsaved" and the first is "saved" —
    /// so that comparison would pass even if the note were never drawn, and
    /// the mutation that stops the note being drawn was **missed** until this
    /// test was written the other way round.
    #[test]
    fn a_frame_with_a_note_is_not_the_frame_without_one() {
        let mut session = Session::sample();
        let state = ShellState::default();

        session
            .execute("doc.note", &serde_json::json!({ "text": "why the design is the way it is" }))
            .expect("the note can be set");
        let with = crate::raster::render(&mut session, 600, 400, &state).expect("the frame renders");

        session
            .execute("doc.note", &serde_json::json!({ "text": "a different note" }))
            .expect("the note can be set");
        let other = crate::raster::render(&mut session, 600, 400, &state).expect("the frame renders");

        assert_ne!(
            with.png, other.png,
            "two designs whose notes differ have the same frame: the note is not on the screen"
        );
    }
}
