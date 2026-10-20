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
/// Selection and the transient status line, and nothing else. The document lives
/// in the [`Session`]; a second copy here would be a second thing to go stale.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ShellState {
    /// The selected node, if any.
    pub selected: Option<NodeId>,
    /// The last thing that happened, shown in the status bar.
    pub status: String,
    /// Whether the last command failed, which is what colours the status bar.
    pub status_is_error: bool,
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
        Self { session, state: ShellState::default(), automation: Automation::new() }
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
                    let (_, level_bar) = draw_level_bar(ui, tokens, *level);
                    automation.publish(AutomationId::node_row(id), row, true);
                    automation.publish(AutomationId::property(id, "$level"), level_bar, true);
                    if clicked {
                        selected = Some(id);
                    }
                    if ui.small_button("mute").clicked() {
                        to_mute.set(Some(id));
                    }
                    if ui.small_button("del").clicked() {
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
                let key = AutomationId::keyframe(id, &path);
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
        let mut path_text = String::new();
        let mut value_text = String::new();
        let path_changed =
            ui.add(egui::TextEdit::singleline(&mut path_text).hint_text("path, e.g. amount")).changed();
        if path_changed {
            let value_changed =
                ui.add(egui::TextEdit::singleline(&mut value_text).hint_text("value")).changed();
            if value_changed {
                actions.push((
                    "key.set",
                    serde_json::json!({
                        "node": id.get(),
                        "path": path_text,
                        "value": value_text,
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
        let rate = meta.frame_rate();
        let total = meta.duration().frames_i64(rate).max(1);
        // The frame rate is checked by the model when the document is read and
        // when a command changes it, but a zero here would divide by zero, so
        // the panel does not trust it. §89.
        let fps = f64::from(meta.fps).max(1.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(meta.name.clone()).strong().color(tokens.text));
            ui.label(
                RichText::new(format!("{} fps · {:.2}s", meta.fps, meta.duration().seconds_f64()))
                    .color(tokens.text_dim),
            );
        });
        ui.separator();

        let mut frame = self.session.at().frames_i64(rate);
        let response = ui.add(
            egui::Slider::new(&mut frame, 0..=total)
                .custom_formatter(move |f, _| format!("{:.2}s", f / fps))
                .show_value(false),
        );
        if response.changed() {
            self.run("playback.seek", serde_json::json!({ "frame": frame }));
        }

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

/// Draw one frame and hand the finished shell back.
///
/// The host keeps the [`ShellState`] and the [`Automation`] registry; it must
/// not keep the [`Shell`], whose borrow ends here.
pub fn frame(ctx: &Context, session: &mut Session, tokens: &Tokens) -> (ShellState, Automation) {
    let mut shell = Shell::new(session);
    shell.show(ctx, tokens);
    (shell.state, shell.automation)
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
                registry.known(AutomationId::property(id, "$level").as_str()) != Widget::Missing,
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
}
