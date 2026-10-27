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

use std::collections::HashMap;

use egui::{Context, Layout, Rect, RichText, ScrollArea, Ui, Vec2};

use craftkit_core::NodeId;
use craftkit_engine::{Area, ParamDoc, Session, all};

use crate::automation::{Automation, AutomationId, Widget};
use crate::icons;
use crate::theme::Tokens;

/// What the shell remembers between frames.
///
/// Selection, the transient status line, the text of the "key at the
/// playhead" fields, and the form a command's parameters are collected in —
/// the things a person typed and the app must not forget because it
/// redrew. The document lives in the [`Session`]; a second copy here would
/// be a second thing to go stale.
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
    /// The form a command's parameters are collected in, if one is open.
    ///
    /// It is here and not in the document because it is not part of the
    /// design: it is what the person is typing into, and a form rebuilt
    /// every frame would lose every keystroke. §118.
    pub prompt: Option<Prompt>,
}

/// A form: the parameters of one command, waiting to be filled in.
///
/// §118: the toolbar used to run every command with an empty parameter
/// object, so seventeen of its twenty-one buttons could only be refused.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Prompt {
    /// The command the form is for.
    pub command: String,
    /// One field per parameter the command asks for.
    pub fields: Vec<PromptField>,
    /// What went wrong the last time the person pressed OK.
    pub error: Option<String>,
}

/// One field of a [`Prompt`].
#[derive(Debug, Clone, PartialEq)]
pub struct PromptField {
    /// The command's own declaration of this parameter.
    ///
    /// The form holds the declaration rather than a copy of it, so a field
    /// cannot drift from the parameter it fills — and the conversion of what
    /// was typed is the engine's, because the engine is what reads it.
    pub param: ParamDoc,
    /// What the person has typed.
    pub text: String,
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
        let _ = self.try_run(command, params);
    }

    /// Run one command and say whether it worked.
    ///
    /// The form needs to know: a command that was refused keeps the form
    /// open, with the refusal in it. §118.
    fn try_run(
        &mut self,
        command: &str,
        params: serde_json::Value,
    ) -> Result<craftkit_engine::Outcome, craftkit_engine::EngineError> {
        match self.session.execute(command, &params) {
            Ok(outcome) => {
                self.state.status = outcome.message.clone();
                self.state.status_is_error = false;
                log::info!("{command}: {}", outcome.message);
                Ok(outcome)
            }
            Err(e) => {
                self.state.status = e.to_string();
                self.state.status_is_error = true;
                log::warn!("{command} refused: {e}");
                Err(e)
            }
        }
    }

    /// Start a command from the interface.
    ///
    /// This is the interactive counterpart of [`Session::execute`], and the
    /// only path that may ask a question. A command that needs parameters
    /// opens a **form** for them; a command that needs none runs at once.
    ///
    /// The shell knows one thing the engine cannot see: which node the
    /// person is looking at. A command that addresses a node is given it,
    /// so "Mute" is one click with a node selected and two without.
    ///
    /// §118: the toolbar used to call `run` with an empty parameter object,
    /// so seventeen of its twenty-one buttons could only be refused.
    pub fn start(&mut self, command: &str) {
        let Some(spec) = craftkit_engine::get(command) else {
            self.state.status = format!("there is no command named `{command}`");
            self.state.status_is_error = true;
            return;
        };
        let mut fields: Vec<PromptField> = spec
            .ask_params()
            .into_iter()
            .map(|param| PromptField { param: *param, text: String::new() })
            .collect();
        // The node the person is looking at, for a command that addresses one.
        if let Some(id) = self.state.selected
            && let Some(field) = fields.iter_mut().find(|f| f.param.name == "node")
        {
            field.text = id.get().to_string();
        }
        // Everything the command asks for is in hand: run it. A command that
        // asks for nothing has an empty list here, and runs too.
        if fields.iter().all(|f| !f.text.trim().is_empty()) {
            match self.params_for(&fields) {
                Ok(params) => self.run(command, params),
                Err(e) => {
                    self.state.prompt = Some(Prompt { command: command.to_string(), fields, error: Some(e) });
                }
            }
            return;
        }
        self.state.prompt = Some(Prompt { command: command.to_string(), fields, error: None });
    }

    /// The JSON a set of filled-in fields is worth.
    ///
    /// The conversion is the engine's, because the engine is what reads the
    /// result: one place decides what a number, a name or a time means.
    /// §118.
    fn params_for(&self, fields: &[PromptField]) -> Result<serde_json::Value, String> {
        let mut map = serde_json::Map::new();
        for field in fields {
            if field.text.trim().is_empty() {
                if field.param.needed() {
                    return Err(format!("`{}` is required", field.param.name));
                }
                continue;
            }
            let typed = craftkit_engine::params::text_to_param(&field.param, &field.text)?;
            map.insert(typed.name.to_string(), typed.value);
        }
        Ok(serde_json::Value::Object(map))
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
        // The form is drawn last, so it is on top of the panels it is about.
        self.prompt_window(ctx, tokens);
    }

    /// The form, if one is open.
    ///
    /// It is a floating window rather than a panel: the panels are the
    /// design, and the form is a question about it. §118.
    fn prompt_window(&mut self, ctx: &Context, tokens: &Tokens) {
        let Some(mut prompt) = self.state.prompt.take() else { return };
        let mut fields = std::mem::take(&mut prompt.fields);
        let command = prompt.command.clone();
        let label = craftkit_engine::get(&command).map(|s| s.label).unwrap_or("Command");
        let mut automation = std::mem::take(&mut self.automation);
        let mut submit = false;
        let mut cancel = false;

        egui::Window::new("")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                // The form's colours come from the theme, like everything
                // else the shell draws. `egui`'s own defaults are a second
                // palette, and a dialog in a different palette is a dialog
                // that does not look like the window it is in. §118.
                ui.visuals_mut().window_fill = tokens.surface;
                // The width is spelled as an `f32`, because the newest Rust
                // refuses to fall back to one: a float literal that reaches
                // an `f32` through a trait bound that `f32` does not satisfy
                // is a future error, and the workflow builds with warnings as
                // errors. §217.
                ui.visuals_mut().window_stroke = egui::Stroke::new(1.0_f32, tokens.border);
                ui.visuals_mut().override_text_color = Some(tokens.text);
                ui.visuals_mut().weak_text_color = Some(tokens.text_dim);
                // The title is drawn here rather than by the window, so it is
                // the theme's colour and not `egui`'s dimmed one.
                ui.label(RichText::new(label).strong().color(tokens.text));
                for field in fields.iter_mut() {
                    // A parameter with an alternative is one parameter in two
                    // spellings, and the field takes either — so the label
                    // names both. §120.
                    let label = match field.param.alternative {
                        Some(other) => format!("{} or {other}", field.param.name),
                        None => field.param.name.to_string(),
                    };
                    ui.label(RichText::new(&label).color(tokens.text));
                    ui.label(RichText::new(field.param.help).color(tokens.text_dim).small().weak());
                    let response = ui.add(egui::TextEdit::singleline(&mut field.text).hint_text(&label));
                    automation.publish(
                        AutomationId::prompt_field(&command, field.param.name),
                        response.rect,
                        true,
                    );
                }
                if let Some(error) = &prompt.error {
                    ui.label(RichText::new(error).color(tokens.error));
                }
                ui.horizontal(|ui| {
                    let ok = ui.button("OK");
                    automation.publish(AutomationId::prompt_ok(), ok.rect, true);
                    if ok.clicked() {
                        submit = true;
                    }
                    let no = ui.button("Cancel");
                    automation.publish(AutomationId::prompt_cancel(), no.rect, true);
                    if no.clicked() {
                        cancel = true;
                    }
                });
            });

        self.automation = automation;
        prompt.fields = fields;

        if cancel {
            self.state.prompt = None;
            return;
        }
        if !submit {
            self.state.prompt = Some(prompt);
            return;
        }
        match self.params_for(&prompt.fields) {
            Ok(params) => match self.try_run(&prompt.command, params) {
                Ok(_) => self.state.prompt = None,
                Err(e) => {
                    prompt.error = Some(e.to_string());
                    self.state.prompt = Some(prompt);
                }
            },
            Err(e) => {
                prompt.error = Some(e);
                self.state.prompt = Some(prompt);
            }
        }
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
                        // `start`, not `run`: a command that needs
                        // parameters is asked for them, and a command that
                        // needs none runs at once. §118.
                        self.start(spec.id);
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
        // Everything the rows need is read out **before** any of them is drawn.
        // Drawing needs `&mut` to the automation registry and the selection,
        // and iterating the document needs `&` to the document; collecting first
        // is what lets both borrows exist.
        //
        // §127: the collection is a pure function of the document and the
        // plan, so it can be measured **on its own** — which is how the cost
        // of asking the plan for every node's level was found.
        let rows = node_rows(self.session.document(), self.session.plan());
        self.node_rows_panel(ui, tokens, &rows);
    }

    /// Draw the node list's rows.
    ///
    /// Split from the collection so that a gate can time the two apart: the
    /// panel's cost is the rows' **drawing** plus the rows' **collecting**,
    /// and only the second was quadratic. §127.
    fn node_rows_panel(&mut self, ui: &mut Ui, tokens: &Tokens, rows: &[NodeRow]) {
        ui.heading("Nodes");
        ui.separator();

        let mut selected = self.state.selected;
        let mut automation = std::mem::take(&mut self.automation);
        // `Cell` because egui's layout closures capture by move, and these two
        // have to outlive the frame's closures to be acted on afterwards.
        let to_mute = std::cell::Cell::new(None);
        let to_remove = std::cell::Cell::new(None);

        // `[false, true]`: do not shrink across, shrink down.
        //
        // egui's scroll area shrinks to the width of its content unless it is
        // told not to, and a scroll area narrower than the panel answers the
        // wheel only over that narrower strip. The properties panel was told
        // in §126; the node list was not, so the wheel did nothing over the
        // panel's right-hand margin, and the scroll bar was not at the panel's
        // edge. §128.
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            for row in rows {
                let id = row.id;
                let is_selected = selected == Some(id);
                ui.horizontal(|ui| {
                    let colour = if is_selected { tokens.accent } else { tokens.text };
                    let response = ui.selectable_label(is_selected, RichText::new(&row.name).color(colour));
                    let row_rect = response.rect;
                    let clicked = response.clicked();
                    // The **track**, not the fill: the fill is as wide as the
                    // level, so at level zero it is a rectangle with no area at
                    // all — a name an agent can look up and never click. §99.
                    let (level_track, _) = draw_level_bar(ui, tokens, row.level);
                    automation.publish(AutomationId::node_row(id), row_rect, true);
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
                    ui.label(RichText::new(&row.kind).color(tokens.text_dim).small());
                    if row.muted {
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

        // The key editor comes **before** the list of properties.
        //
        // §126: the list lives in a scroll area, and a scroll area takes the
        // rest of the panel — so a control drawn *after* it would be laid out
        // below the panel's bottom edge, where nobody can reach it and
        // nobody can see it. The editor is therefore above the list, where it
        // stays put while the list scrolls.
        ui.separator();
        Shell::key_editor(&mut self.state, &mut self.automation, self.session, ui, tokens, id, &mut actions);
        ui.separator();

        // The model's own flatten, so the panel cannot invent a second view of
        // the property tree.
        //
        // The rows **borrow** the tree. They used to **copy** it — every
        // property of the node, with every keyframe's value, on every frame —
        // and the panel draws four things out of them: the path, the value,
        // the number of keyframes and the first eight times. §123.
        let rows = node.props.flatten();

        // The rows live in a **scroll area**.
        //
        // §126: a node can carry more properties than the panel is tall, and
        // the ones that did not fit were below the panel's bottom edge with
        // no way to reach them: the panel had no scroll area at all, so the
        // node list's scroll bar was the only one on the screen and the
        // properties were not on it.
        //
        // The scroll area **fills the panel's width** (`auto_shrink` is
        // `[false, true]`: do not shrink across, shrink down), so the scroll
        // bar is at the panel's edge and the wheel answers anywhere across
        // the list — and not only over the editor of one property.
        //
        // The rows are moved **into** the closure, so the borrow of the
        // session ends with the closure, and the commands below can still
        // run.
        let mut automation = std::mem::take(&mut self.automation);
        ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
            for (path, prop) in rows {
                let path = path.to_string();
                let widget = AutomationId::property(id, &path);
                // A locked property is **greyed**: its editor is drawn disabled and
                // cannot be edited, and the registry says so — §107's rule, applied
                // to a lock. §124: the model said a locked property was greyed in
                // the UI, and the UI had never heard of `locked`.
                let enabled = !prop.locked;
                match &prop.value {
                    craftkit_core::Value::Bool(b) => {
                        let mut value = *b;
                        let response = ui.add_enabled(enabled, egui::Checkbox::new(&mut value, path.clone()));
                        automation.publish(widget, response.rect, enabled);
                        if response.changed() {
                            edits.push((path.clone(), serde_json::Value::Bool(value)));
                        }
                    }
                    craftkit_core::Value::Num(n) => {
                        let mut value = *n;
                        let response = ui.add_enabled(
                            enabled,
                            egui::Slider::new(&mut value, -100.0..=100.0)
                                .text(path.clone())
                                .clamping(egui::SliderClamping::Never),
                        );
                        automation.publish(widget, response.rect, enabled);
                        if response.changed() {
                            edits.push((path.clone(), serde_json::Value::from(value)));
                        }
                    }
                    craftkit_core::Value::Color(c) => {
                        let mut rgb = [c.r, c.g, c.b];
                        // The colour button is a function of the `Ui`, not a
                        // widget, so it cannot be wrapped by `add_enabled` — it is
                        // drawn in a scope that is disabled instead.
                        let response =
                            ui.add_enabled_ui(enabled, |ui| ui.color_edit_button_srgb(&mut rgb)).inner;
                        automation.publish(widget, response.rect, enabled);
                        if response.changed() {
                            edits.push((
                                path.clone(),
                                serde_json::Value::from(format!(
                                    "#{:02x}{:02x}{:02x}",
                                    rgb[0], rgb[1], rgb[2]
                                )),
                            ));
                        }
                    }
                    other => {
                        let mut text = other.display();
                        let response = ui.add_enabled(
                            enabled,
                            egui::TextEdit::singleline(&mut text).hint_text(path.clone()),
                        );
                        automation.publish(widget, response.rect, enabled);
                        if response.changed() {
                            edits.push((path.clone(), serde_json::Value::from(text)));
                        }
                    }
                }

                if prop.is_animated() {
                    ui.label(
                        RichText::new(format!("{} keyframe(s)", prop.keys.len()))
                            .color(tokens.text_dim)
                            .small(),
                    );
                    // **When** the keyframes are. §117: the panel said how many
                    // there were and not when any of them was, and the times were
                    // in the model, in the file and in three commands' parameters
                    // — on no screen.
                    ui.label(RichText::new(keyframe_times(prop.keys.keys())).color(tokens.text_dim).small());
                    let key = AutomationId::clear_keys(id, &path);
                    // The button cannot work on a locked property — `prop.reset`
                    // refuses one — so it is greyed and says so. §124.
                    let response = ui.add_enabled(enabled, egui::Button::new("clear keys").small());
                    automation.publish(key, response.rect, enabled);
                    if response.clicked() {
                        actions.push((
                            "prop.reset",
                            serde_json::json!({ "node": id.get(), "path": path.clone() }),
                        ));
                    }
                }
            }
        });
        self.automation = automation;

        for (path, value) in edits {
            self.run("prop.set", serde_json::json!({ "node": id.get(), "path": path, "value": value }));
        }
        for (command, params) in actions {
            self.run(command, params);
        }
    }

    /// The "key at the playhead" editor.
    ///
    /// It adds a keyframe to the property named by the path field, at the
    /// playhead, with the value in the value field.
    ///
    /// §126: this is drawn **above** the list of properties, because the list
    /// lives in a scroll area and a scroll area takes the rest of the panel.
    /// A control drawn after it would be laid out below the panel's bottom
    /// edge — visible to nobody, and reachable by nobody.
    ///
    /// It takes the pieces it needs rather than the shell: the panel holds a
    /// borrow of the session (the node it is drawing) while it draws, so it
    /// cannot lend out `&mut self`.
    fn key_editor(
        state: &mut ShellState,
        automation: &mut Automation,
        session: &Session,
        ui: &mut Ui,
        tokens: &Tokens,
        id: NodeId,
        actions: &mut Vec<(&'static str, serde_json::Value)>,
    ) {
        ui.label(RichText::new("Key at the playhead").color(tokens.text_dim));
        // The text lives in the shell's state, not in a local: a local is
        // rebuilt every frame, and the person typing would watch every
        // keystroke disappear. §105.
        let path_field =
            ui.add(egui::TextEdit::singleline(&mut state.key_path).hint_text("path, e.g. amount"));
        automation.publish(AutomationId::key_path(), path_field.rect, true);
        // The value field is here while there is a path to put a value on.
        // It used to appear only on the frame the path changed, which meant
        // it was gone before anything could be typed into it. §105.
        if !state.key_path.is_empty() {
            let value_field = ui.add(egui::TextEdit::singleline(&mut state.key_value).hint_text("value"));
            automation.publish(AutomationId::key_value(), value_field.rect, true);
            if value_field.changed() {
                actions.push((
                    "key.set",
                    serde_json::json!({
                        "node": id.get(),
                        "path": state.key_path.clone(),
                        "value": state.key_value.clone(),
                        "time": session.at().seconds_f64(),
                    }),
                ));
            }
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
        // The rail spans the panel. §121: egui's default slider is a hundred
        // pixels wide, and the timeline was one — a stub in a panel seven
        // hundred pixels across, with the design's frames crammed into a
        // seventh of it and no room at all for the marks that say where the
        // keys are.
        let rail_width = ui.available_width();
        let previous_width = ui.spacing_mut().slider_width;
        ui.spacing_mut().slider_width = rail_width;
        let response = ui.add(
            egui::Slider::new(&mut frame, frames.clone())
                .show_value(false)
                // The handle's shape decides how far inside the rail its
                // centre can travel, and the marks are placed where it
                // travels. Pinning the shape here keeps the two in step: a
                // theme that changed it would move the marks off the frames
                // they name. §121.
                .handle_shape(egui::style::HandleShape::Rect { aspect_ratio: HANDLE_ASPECT }),
        );
        ui.spacing_mut().slider_width = previous_width;
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
        // The design's keyframes, marked on the rail. §121: the timeline
        // showed the range and the playhead and nothing else, so the only
        // part of a design that *moves* was on no screen at all — the
        // properties panel lists the times for the selected node as text
        // (§117), and the other nodes' keys were nowhere.
        let marks = keyframe_marks(self.session.document(), self.state.selected, rate);
        let travel = handle_travel(response.rect);
        for mark in &marks {
            // A key outside the design's range has no place on the rail:
            // there is no frame to put it at. §119's diagnostics name it.
            if mark.frame < *frames.start() || mark.frame > *frames.end() {
                continue;
            }
            let x = mark_x(travel, mark.frame, *frames.start(), *frames.end());
            let colour = if mark.selected { tokens.accent } else { tokens.text_dim };
            paint_mark(ui.painter(), x, response.rect.center().y, colour);
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
            // The scroll area **fills the panel's width** (`auto_shrink` is
            // off across): a scroll area that shrinks to its content is
            // narrower than the panel, and the strip between the content and
            // the panel's edge is dead to the wheel. §126 fixed it in the
            // properties panel and §128 in the node list; this panel had the
            // same defect and nobody looked, because the content is a line of
            // text and the strip is most of the panel. §199.
            ScrollArea::vertical().max_height(140.0).auto_shrink([false, true]).show(ui, |ui| {
                for (index, d) in plan.diagnostics.items.iter().enumerate() {
                    let colour = if d.severity == craftkit_eval::Severity::Error {
                        tokens.error
                    } else {
                        tokens.warning
                    };
                    let response = ui.label(RichText::new(d.render()).color(colour).small());
                    // A problem the plan found is a line a person reads, so an
                    // agent has to be able to name it the same way. §199.
                    self.automation.publish(AutomationId::problem(index), response.rect, true);
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

/// One row of the node list.
///
/// The rows are collected **before** any of them is drawn — a panel reads the
/// document and the plan, and never holds a borrow of either while it draws
/// (§123's rule) — so what a row needs has to be owned.
pub struct NodeRow {
    pub id: NodeId,
    pub name: String,
    pub kind: String,
    pub muted: bool,
    /// The level leaving this node, as the plan resolved it.
    pub level: f64,
}

/// Collect the node list's rows.
///
/// The plan answers *"what is this node's level?"* by **scanning** its own
/// nodes: [`craftkit_eval::Plan::state`] is a `find`. Asking it about every
/// node in turn is therefore a scan per node — quadratic in the number of
/// nodes, for a list that draws a few dozen rows on the screen.
///
/// Measured, §127: at the model's own limit of ten thousand nodes the
/// collection took **43.7 ms**, and with the levels found once it takes
/// **1.2 ms**.
///
/// The levels are therefore found **once**, into a map, and every row then
/// costs one lookup. The rows still come from the **document**, not from the
/// plan: the list shows the document's nodes, and a plan that has fallen
/// behind the document must not hide a node or invent one.
pub fn node_rows(document: &craftkit_model::Document, plan: Option<&craftkit_eval::Plan>) -> Vec<NodeRow> {
    // The plan holds one state per node, in the document's order, with the
    // ids unique — so the map has exactly one entry per node.
    let levels: HashMap<NodeId, f64> = match plan {
        Some(p) => p.nodes.iter().map(|s| (s.id, s.level_out)).collect(),
        None => HashMap::new(),
    };
    document
        .nodes
        .iter()
        .map(|n| NodeRow {
            id: n.id,
            name: n.name.clone(),
            kind: n.kind.label().to_string(),
            muted: n.muted,
            level: levels.get(&n.id).copied().unwrap_or(0.0),
        })
        .collect()
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

/// The handle's width as a fraction of its radius.
///
/// `egui`'s default for a slider's handle. It is pinned here because the
/// keyframe marks are placed where the handle travels and the handle's
/// travel depends on its shape: a theme that changed the shape would move
/// the marks off the frames they name. §121.
const HANDLE_ASPECT: f32 = 0.75;

/// `egui`'s rule for a handle's radius: the rail's height over this.
///
/// The same reason as [`HANDLE_ASPECT`]: the marks and the handle have to
/// agree on where the handle can be, or a mark would point at a frame the
/// playhead could not reach.
///
/// The number is `egui`'s rule, read from its source — and the test that
/// clicks a mark and checks where the playhead lands is what says whether it
/// is still true. §121.
const HANDLE_RADIUS_DIVISOR: f32 = 2.5;

/// Where the playhead's handle travels along the rail.
///
/// `egui` keeps the handle inside the rail's ends, so the handle's centre at
/// the design's first frame is not at the rail's left edge but a radius in.
/// The marks use the same travel, or a mark would sit where the handle
/// cannot go. §121.
fn handle_travel(rect: Rect) -> egui::Rangef {
    let radius = rect.height() / HANDLE_RADIUS_DIVISOR * HANDLE_ASPECT;
    rect.x_range().shrink(radius)
}

/// Where a frame is on the rail.
///
/// The frame's share of the design's range, along the handle's travel.
///
/// The share is clamped into the design's range, so this can never answer
/// with a place off the end of the rail. A key outside the design's range
/// has no frame to put it at — §119's diagnostics name such a key — and the
/// caller decides whether to draw it at all. §121.
fn mark_x(travel: egui::Rangef, frame: i64, start: i64, end: i64) -> f32 {
    if end <= start {
        return travel.center();
    }
    let share = ((frame - start) as f64 / (end - start) as f64).clamp(0.0, 1.0);
    travel.min + (travel.max - travel.min) * share as f32
}

/// One keyframe, as a place on the timeline.
struct KeyMark {
    /// The frame the key is at.
    frame: i64,
    /// Whether the key belongs to the selected node.
    selected: bool,
}

/// The design's keyframes, as places on the timeline.
///
/// Every node's keys, not only the selected one's: the timeline is the
/// design's, and the properties panel speaks for one node at a time. §121.
///
/// The tree is walked **without** the model's listing: the listing builds a
/// path for every property — a copy of the tree — and the timeline redraws on
/// every frame. §122.
fn keyframe_marks(
    document: &craftkit_model::Document,
    selected: Option<NodeId>,
    rate: craftkit_core::FrameRate,
) -> Vec<KeyMark> {
    let mut marks = Vec::new();
    for node in &document.nodes {
        collect_keys(&node.props, selected == Some(node.id), rate, &mut marks);
    }
    marks
}

/// The keyframes of one property tree, without a path for each.
///
/// The model's own listing is the right answer when a path is wanted, and the
/// timeline does not want one: it wants the *instants*. §122.
fn collect_keys(
    group: &craftkit_model::PropGroup,
    selected: bool,
    rate: craftkit_core::FrameRate,
    out: &mut Vec<KeyMark>,
) {
    for node in &group.nodes {
        match node {
            craftkit_model::PropNode::Prop(property) => {
                for key in property.keys.keys() {
                    out.push(KeyMark { frame: key.time.frames_i64(rate), selected });
                }
            }
            craftkit_model::PropNode::Group(group) => collect_keys(group, selected, rate, out),
        }
    }
}

/// Paint one keyframe's mark on the rail.
///
/// A short bar across the rail: a frame's worth of "something is keyed
/// here", in the accent colour for the selected node's keys and the dim
/// colour for the rest. §121.
fn paint_mark(painter: &egui::Painter, x: f32, y: f32, colour: egui::Color32) {
    let half = 1.5;
    let tall = 5.0;
    painter.rect_filled(
        Rect::from_center_size(egui::pos2(x, y), Vec2::new(half * 2.0, tall * 2.0)),
        0.0,
        colour,
    );
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

/// When a property's keyframes are, as one line of text.
///
/// The times in seconds, in the order the model keeps them. §117: the panel
/// said how many keyframes a property had and not **when** any of them was,
/// and the times were in the model, in the file and in three commands'
/// parameters — on no screen.
///
/// The line is bounded and the cut is marked, so nobody reads it as the whole
/// list: the same rule as the note's preview (§110).
pub fn keyframe_times(keys: &[craftkit_model::Key]) -> String {
    const TIMES_IN_A_LINE: usize = 8;
    let times: Vec<String> =
        keys.iter().take(TIMES_IN_A_LINE).map(|k| format!("{:.3}s", k.time.seconds_f64())).collect();
    let mut line = times.join(", ");
    if keys.len() > TIMES_IN_A_LINE {
        line.push_str(", …");
    }
    line
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

    /// A frame with one node whose only property is locked — or not.
    ///
    /// §124: the model says a locked property is greyed in the UI, and the UI
    /// had never heard of `locked`. The two frames differ in one thing, so a
    /// difference in the pixels is the lock and nothing else.
    fn frame_with_a_locked_property(locked: bool) -> (Vec<u8>, Automation, NodeId) {
        let mut doc = craftkit_model::Document::empty("probe");
        let id = doc.add_node("One", craftkit_model::NodeKind::Source).expect("the node");
        {
            let node = doc.get_mut(id).expect("the node");
            let property = craftkit_model::Property::new(
                1,
                "amount",
                craftkit_core::Value::num("amount", 0.5).expect("a number"),
            );
            node.props.push(if locked { property.locked() } else { property });
        }
        let id = doc.nodes[0].id;
        let mut session = Session::new("probe");
        session.load_document(doc);
        let state = ShellState { selected: Some(id), ..ShellState::default() };
        let frame = crate::raster::render(&mut session, 900, 600, &state).expect("the frame renders");
        (frame.png, frame.automation, id)
    }

    /// A locked property's editor is published as disabled.
    ///
    /// §124: the registry carries an `enabled` flag for every widget it holds
    /// (§107), and it said *enabled* for a property nobody may change — so an
    /// agent asking "can this be clicked?" was told it could.
    #[test]
    fn a_locked_property_s_editor_is_published_as_disabled() {
        let (_, registry, id) = frame_with_a_locked_property(true);
        let name = AutomationId::property(id, "amount");
        match registry.known(name.as_str()) {
            Widget::Found { enabled: false, .. } => {}
            other => panic!("the editor of a locked property is published as {other:?}"),
        }
        // And an ordinary property's editor is still published as enabled, so
        // the flag says something about the lock and not about the registry.
        let (_, registry, id) = frame_with_a_locked_property(false);
        let name = AutomationId::property(id, "amount");
        match registry.known(name.as_str()) {
            Widget::Found { enabled: true, .. } => {}
            other => panic!("the editor of an ordinary property is published as {other:?}"),
        }
    }

    /// A locked property is drawn greyed.
    ///
    /// §124: the model says *"A locked property is greyed in the UI"*, and the
    /// sentence was false — the panel drew a locked property exactly like an
    /// ordinary one, in full colour, with an editor that could be dragged.
    ///
    /// The comparison is between two frames that differ in one thing: the same
    /// node, the same property, the same value, one of them locked.
    #[test]
    fn a_locked_property_is_drawn_greyed() {
        let (locked_png, locked, id) = frame_with_a_locked_property(true);
        let (free_png, free, _) = frame_with_a_locked_property(false);
        // The rectangle comes from the full lookup, not from `rect()`: that one
        // answers only for a widget that can be clicked, and a locked
        // property's editor cannot be (§107).
        let rect_of =
            |registry: &Automation| match registry.known(AutomationId::property(id, "amount").as_str()) {
                Widget::Found { rect, .. } => rect,
                other => panic!("the property has no rectangle: {other:?}"),
            };
        let rect = rect_of(&locked);
        let same_rect = rect_of(&free);
        assert_eq!(rect, same_rect, "the two editors are not in the same place");
        let band = |png: &[u8]| crop(png, rect.left(), rect.top(), rect.width(), rect.height());
        assert_ne!(
            band(&locked_png),
            band(&free_png),
            "a locked property is drawn exactly like an ordinary one"
        );
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

    /// The panel says when the keyframes are.
    ///
    /// §117: the panel said how many keyframes a property had and not **when**
    /// any of them was, and the times were in the model, in the file and in
    /// three commands' parameters — on no screen.
    ///
    /// The comparison is on the pixels **between** the property's editor and
    /// the "clear keys" button: that band holds the count and the times, and
    /// the count is the same in both frames — the two designs have two
    /// keyframes each, at 1 s and at 2 s.
    #[test]
    fn the_panel_says_when_the_keyframes_are() {
        let (first_png, first, id) = frame_with_keys(&[1.0, 2.0]);
        let (second_png, _second, _) = frame_with_keys(&[1.0, 3.0]);
        let property =
            first.rect(AutomationId::property(id, "amount").as_str()).expect("the property has a rectangle");
        let clear =
            first.rect(AutomationId::clear_keys(id, "amount").as_str()).expect("the button has a rectangle");
        let band = |png: &[u8]| {
            crop(png, property.left(), property.bottom(), property.width(), clear.top() - property.bottom())
        };
        assert_ne!(
            band(&first_png),
            band(&second_png),
            "the panel says the same thing about two sets of keyframes"
        );
    }

    /// A design with keys on a property, drawn with that property selected.
    ///
    /// The times are the keyframes' times in seconds. §117.
    fn frame_with_keys(times: &[f64]) -> (Vec<u8>, Automation, NodeId) {
        let mut session = Session::sample();
        let id = session.document().nodes[0].id;
        for (index, time) in times.iter().enumerate() {
            session
                .execute(
                    "key.set",
                    &serde_json::json!({
                        "node": id.get(),
                        "path": "amount",
                        "value": index.to_string(),
                        "time": time,
                    }),
                )
                .expect("the key can be set");
        }
        let state = ShellState { selected: Some(id), ..ShellState::default() };
        let frame = crate::raster::render(&mut session, 900, 600, &state).expect("the frame renders");
        (frame.png, frame.automation, id)
    }

    /// A node with one property and `keys` keyframes on it.
    ///
    /// Each keyframe's value is `value_bytes` of text: a **text** value is
    /// the worst case for a copy, because every one of them is a separate
    /// allocation, and a number is the best case, because copying it copies
    /// nothing but the bits. §123.
    fn node_with_keyframes(keys: usize, value_bytes: usize) -> (craftkit_model::Document, NodeId) {
        let mut doc = craftkit_model::Document::empty("probe");
        let id = doc.add_node("One", craftkit_model::NodeKind::Source).expect("the node");
        let node = doc.get_mut(id).expect("the node");
        let mut property = craftkit_model::Property::new(
            1,
            "amount",
            craftkit_core::Value::num("amount", 0.5).expect("a number"),
        );
        let text = "x".repeat(value_bytes);
        let mut list = craftkit_model::Keyframes::new();
        for k in 0..keys {
            let time = craftkit_core::Tick::from_raw(k as i128 * 1_000_000_000);
            let value = craftkit_core::Value::text("amount", text.clone()).expect("a text");
            let key = craftkit_model::Key::new(time, value, craftkit_model::Easing::Linear).expect("a key");
            list.set(key).expect("the key is set");
        }
        property.keys = list;
        node.props.push(property);
        let id = doc.nodes[0].id;
        (doc, id)
    }

    /// How long the properties panel takes, on its own.
    ///
    /// The panel is drawn by itself, in its own layout, with the node
    /// selected — so what is measured is the panel's work and not the
    /// window's. §123.
    ///
    /// The **fastest** of several rounds, because a preempted round says
    /// nothing about the code. §101's shape.
    fn panel_alone(document: craftkit_model::Document, id: NodeId) -> f64 {
        let mut session = Session::new("probe");
        session.load_document(document);
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(&mut session);
        shell.state.selected = Some(id);
        // The first frame lays out; the later ones place.
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| shell.properties(ui, &tokens));
        });
        let mut best = f64::MAX;
        for _ in 0..9 {
            let start = std::time::Instant::now();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| shell.properties(ui, &tokens));
            });
            best = best.min(start.elapsed().as_secs_f64());
        }
        best
    }

    /// The properties panel does not copy the keyframes it does not draw.
    ///
    /// §123: the panel used to **copy** the selected node's whole property
    /// tree on every frame — every property, with every keyframe's value —
    /// and it draws four things out of it: the path, the value, the number
    /// of keyframes and the first eight times.
    ///
    /// Measured: a property with ten thousand text-valued keyframes cost the
    /// panel 484 µs a frame, and a property with one cost 20 µs. The
    /// difference was the copy.
    ///
    /// The test is **relative**, so it does not depend on how fast the
    /// machine is: ten thousand keyframes must not cost eight times what one
    /// costs. §101's shape — and §233's fix to it: the number is the **middle**
    /// round's ratio, because the ratio of two independent measurements is a
    /// statement about the scheduler on a busy machine.
    #[test]
    fn the_properties_panel_does_not_copy_the_keyframes_it_does_not_draw() {
        // Nine rounds, and each round builds the two designs it measures.
        const ROUNDS: usize = 9;
        let mut round = || {
            let one = node_with_keyframes(1, 16);
            let many = node_with_keyframes(10_000, 16);
            let fast = panel_alone(one.0, one.1);
            let slow = panel_alone(many.0, many.1);
            (fast, slow)
        };
        let ratio = craftkit_core::measure::quietest_ratio(ROUNDS, &mut round);
        assert!(
            ratio < 8.0,
            "the properties panel took {ratio:.1}x as long with 10000 keyframes as with 1: \
             it is copying the keyframes it does not draw"
        );
    }

    /// The properties panel does not copy the keyframe **values** either.
    ///
    /// §123: the copy took the values with it, and a value can be as large
    /// as the model allows — 64 KiB of text. A property with two hundred
    /// such keyframes made the panel copy twelve megabytes a frame to draw
    /// a number, a count and eight times.
    ///
    /// The shape is the same as the gate above, on the other axis: the
    /// **size** of what is copied, not the number of things.
    #[test]
    fn the_properties_panel_does_not_copy_the_keyframe_values() {
        const ROUNDS: usize = 9;
        let mut round = || {
            let small = node_with_keyframes(200, 1);
            let big = node_with_keyframes(200, 64 * 1024);
            let fast = panel_alone(small.0, small.1);
            let slow = panel_alone(big.0, big.1);
            (fast, slow)
        };
        let ratio = craftkit_core::measure::quietest_ratio(ROUNDS, &mut round);
        assert!(
            ratio < 8.0,
            "the properties panel took {ratio:.1}x as long with 64 KiB keyframe values as with 1 byte: \
             it is copying the keyframe values it does not draw"
        );
    }

    /// The panel's editors are the model's own paths — one for each property.
    ///
    /// §123: the panel used to copy the node's whole property tree on every
    /// frame. The copy is gone, and the paths it draws are still the model's
    /// own: the panel cannot invent a second view of the tree, and it does
    /// not publish an editor for anything the model does not hold.
    #[test]
    fn the_panel_publishes_one_editor_for_each_property_and_no_other() {
        let mut session = Session::sample();
        let id = session.document().nodes[0].id;
        let created = session.execute(
            "prop.set",
            &serde_json::json!({ "node": id.get(), "path": "custom/thing", "value": 7 }),
        );
        assert!(created.is_ok(), "{created:?}");

        let registry = draw_selected(&mut session, id);
        let prefix = format!("node.{}.prop.", id.get());
        let published: Vec<String> =
            registry.names().into_iter().filter(|n| n.starts_with(&prefix)).map(|n| n.to_string()).collect();
        let paths: Vec<String> = session
            .node(id)
            .map(|n| n.props.flatten().into_iter().map(|(path, _)| path.to_string()).collect())
            .unwrap_or_default();
        assert_eq!(
            published.len(),
            paths.len(),
            "the panel published {published:?} for the model's {paths:?}"
        );
        for path in &paths {
            let name = format!("{prefix}{path}");
            assert!(published.contains(&name), "the panel published no editor for `{path}`: {published:?}");
        }
    }

    /// A node with `count` plain properties, named `p0`, `p1`, …
    ///
    /// §126: the properties panel has to cope with a node that carries more
    /// properties than the panel is tall.
    fn node_with_properties(count: usize) -> (craftkit_model::Document, NodeId) {
        let mut doc = craftkit_model::Document::empty("probe");
        let id = doc.add_node("One", craftkit_model::NodeKind::Source).expect("the node");
        {
            let node = doc.get_mut(id).expect("the node");
            for p in 0..count {
                let value = craftkit_core::Value::num("v", 0.5).expect("a number");
                node.props.push(craftkit_model::Property::new(p as u64 + 1, format!("p{p}"), value));
            }
        }
        let id = doc.nodes[0].id;
        (doc, id)
    }

    /// The window the panel is drawn in, for the scroll tests.
    fn screen() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 600.0))
    }

    /// A wheel event over `at`, and the frame that carries it.
    ///
    /// The wheel is a **point** delta of seven points, which egui applies whole
    /// in the frame it arrives: a larger delta is smoothed over several
    /// frames, and a line delta is smoothed too, so neither says "this frame
    /// moved seven points".
    fn wheel_at(at: egui::Pos2) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(screen()),
            events: vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::Vec2::new(0.0, -7.0),
                    modifiers: Default::default(),
                },
            ],
            ..Default::default()
        }
    }

    /// The properties panel scrolls: a property that does not fit comes into
    /// view.
    ///
    /// §126: the panel had **no scroll area at all**, so a node with more
    /// properties than the panel is tall put the rest of them below the
    /// panel's bottom edge — where nobody could see them and nobody could
    /// reach them. The registry still named every one of them, so an agent
    /// was told about five thousand editors and could get to none of them.
    ///
    /// The wheel only answers when the pointer is over the scroll area, and
    /// egui's scroll area shrinks to its content unless it is told not to —
    /// so the pointer here is over the **first editor**, which is inside the
    /// list whatever the list's width is.
    #[test]
    fn the_properties_panel_scrolls_to_the_properties_that_do_not_fit() {
        let (doc, id) = node_with_properties(40);
        let mut session = Session::new("probe");
        session.load_document(doc);
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(id), ..Default::default() };

        let plain = egui::RawInput { screen_rect: Some(screen()), ..Default::default() };
        let before = one_frame(&ctx, &mut session, &tokens, &mut state, plain);
        let first =
            before.rect(&format!("node.{}.prop.p0", id.get())).expect("the first property has an editor");
        let last = format!("node.{}.prop.p39", id.get());
        let start = before.rect(&last).expect("the last property has an editor");
        assert!(
            start.top() > screen().height(),
            "the last property is at y={:.0}, which is on the screen: \
             the test needs a list that does not fit",
            start.top()
        );

        // Turn the wheel until the last property is on the screen.
        let mut frames = 0;
        loop {
            frames += 1;
            assert!(
                frames < 400,
                "the list did not scroll in {frames} frames: the last property is still at y={:.0}",
                start.top()
            );
            let after = one_frame(&ctx, &mut session, &tokens, &mut state, wheel_at(first.center()));
            let now = after.rect(&last).expect("the last property still has an editor");
            if now.top() < screen().height() {
                break;
            }
        }
    }

    /// The wheel scrolls the list from anywhere across the panel.
    ///
    /// §126: egui's scroll area shrinks to the width of its content unless it
    /// is told not to, and a scroll area narrower than the panel answers the
    /// wheel only over that narrower strip. The panel's own scroll bar was
    /// the only scroll bar on the screen, and it had to be over the *content*
    /// for the wheel to reach it.
    #[test]
    fn the_wheel_scrolls_the_list_from_across_the_panel() {
        let (doc, id) = node_with_properties(40);
        let mut session = Session::new("probe");
        session.load_document(doc);
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(id), ..Default::default() };

        let plain = egui::RawInput { screen_rect: Some(screen()), ..Default::default() };
        let before = one_frame(&ctx, &mut session, &tokens, &mut state, plain);
        let first_name = format!("node.{}.prop.p0", id.get());
        let first = before.rect(&first_name).expect("the first property has an editor");

        // The pointer is at the panel's right-hand edge, over the panel's
        // scroll bar and over no editor at all.
        let edge = egui::pos2(screen().width() - 12.0, first.center().y);
        let mut after = before.clone();
        for _ in 0..20 {
            after = one_frame(&ctx, &mut session, &tokens, &mut state, wheel_at(edge));
        }
        let moved = after.rect(&first_name).expect("the first property still has an editor").top();
        assert!(
            moved < first.top() - 50.0,
            "the list did not scroll from the panel's edge: the first property is still at y={:.0}",
            moved
        );
    }

    /// The key editor stays on screen when the node has many properties.
    ///
    /// §126: a scroll area takes the rest of the panel, so a control drawn
    /// **after** it is laid out below the panel's bottom edge — where nobody
    /// can see it and nobody can reach it. The key editor is therefore drawn
    /// above the list.
    #[test]
    fn the_key_editor_stays_on_screen_when_the_node_has_many_properties() {
        let (doc, id) = node_with_properties(200);
        let mut session = Session::new("probe");
        session.load_document(doc);
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(id), ..Default::default() };

        let plain = egui::RawInput { screen_rect: Some(screen()), ..Default::default() };
        let registry = one_frame(&ctx, &mut session, &tokens, &mut state, plain);
        let field = registry.rect("properties.key.path").expect("the key field has a rectangle");
        assert!(
            field.top() >= 0.0 && field.bottom() <= screen().height(),
            "the key editor is at y={:.0}..{:.0}, outside the window: \
             it is drawn after the list, and the list takes the panel",
            field.top(),
            field.bottom()
        );
    }

    /// A keyframe inside a group is marked on the rail.
    ///
    /// §122: the marks are found by walking the property tree, and a group
    /// is a different shape from a property — a walk that only looked at the
    /// properties directly under a node would miss every key inside a group.
    #[test]
    fn a_keyframe_inside_a_group_is_marked_on_the_rail() {
        let mut session = Session::sample();
        let id = session.document().nodes[0].id;
        let set = session.execute(
            "prop.set",
            &serde_json::json!({ "node": id.get(), "path": "trigger/lead", "value": 0.5 }),
        );
        assert!(set.is_ok(), "{set:?}");
        let keyed = session.execute(
            "key.set",
            &serde_json::json!({ "node": id.get(), "path": "trigger/lead", "value": 0.5, "time": 2.0 }),
        );
        assert!(keyed.is_ok(), "{keyed:?}");

        let state = ShellState { selected: Some(id), ..ShellState::default() };
        let frame = crate::raster::render(&mut session, 900, 600, &state).expect("the frame renders");
        let rect =
            frame.automation.rect(AutomationId::playhead().as_str()).expect("the playhead has a rectangle");
        assert!(
            mark_column(&frame.png, rect, Tokens::dark().accent).is_some(),
            "the keyframe inside the group is not marked on the rail"
        );
    }

    /// The timeline marks the keyframes.
    ///
    /// §121: the timeline showed the range and the playhead and nothing
    /// else, so the only part of a design that *moves* was on no screen at
    /// all. The panel listed the times for the selected node as text (§117),
    /// and the other nodes' keys were nowhere.
    ///
    /// The comparison is on the **rail**: the playhead is at the same frame
    /// in both frames, so the handle is in the same place and any difference
    /// in the rail is a difference in the marks.
    #[test]
    fn the_timeline_marks_the_keyframes() {
        let (first_png, first, _) = frame_with_keys(&[1.0]);
        let (second_png, second, _) = frame_with_keys(&[3.0]);
        let rail = |png: &[u8], registry: &Automation| {
            let rect =
                registry.rect(AutomationId::playhead().as_str()).expect("the playhead has a rectangle");
            crop(png, rect.left(), rect.top(), rect.width(), rect.height())
        };
        assert_ne!(
            rail(&first_png, &first),
            rail(&second_png, &second),
            "the timeline marks the same place for two keyframes a hundred frames apart"
        );
    }

    /// A keyframe's mark is where the playhead goes when it is clicked.
    ///
    /// §121: the mark has to be *at* the frame it names, and the only way to
    /// know that is to click it. The mark is found in the picture — asking
    /// the drawing code where it drew the mark would prove only that the
    /// drawing code agrees with itself.
    #[test]
    fn a_keyframe_mark_is_where_the_playhead_goes_when_it_is_clicked() {
        let mut session = Session::sample();
        let id = session.document().nodes[0].id;
        session
            .execute(
                "key.set",
                &serde_json::json!({ "node": id.get(), "path": "amount", "value": 0.5, "time": 2.0 }),
            )
            .expect("the key can be set");
        let rate = session.document().meta.frame_rate();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(id), ..ShellState::default() };

        let frame = crate::raster::render(&mut session, 900, 600, &state).expect("the frame renders");
        let rect =
            frame.automation.rect(AutomationId::playhead().as_str()).expect("the playhead has a rectangle");
        let column =
            mark_column(&frame.png, rect, tokens.accent).expect("the keyframe is not marked on the rail");

        // The same picture, drawn the same size, so the click lands where the
        // mark was.
        let ctx = Context::default();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(900.0, 600.0));
        let sized = egui::RawInput { screen_rect: Some(screen), ..Default::default() };
        one_frame(&ctx, &mut session, &tokens, &mut state, sized.clone());
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, sized);
        assert_eq!(
            automation.rect(AutomationId::playhead().as_str()),
            Some(rect),
            "the picture and the click are not the same layout"
        );

        press_at(&ctx, &mut session, &tokens, &mut state, column as f32 + 0.5, rect.center().y);
        assert_eq!(
            session.at().frames_i64(rate),
            48,
            "clicking the mark did not put the playhead on the frame the mark named"
        );
    }

    /// The rail follows the panel.
    ///
    /// §121: egui's default slider is a hundred pixels wide, and the
    /// timeline's rail was one — a stub in a panel seven hundred pixels
    /// across, with the design's frames crammed into a seventh of it.
    #[test]
    fn the_timeline_rail_follows_the_panel() {
        let mut session = Session::sample();
        let narrow = rail_width(&mut session, 900, 600);
        let wide = rail_width(&mut session, 1200, 600);
        assert!(narrow > 400.0, "the rail is still a stub: {narrow}px wide in a 900px frame");
        assert!(wide > narrow + 100.0, "the rail does not follow the panel: {narrow}px -> {wide}px");
    }

    /// A key outside the design's range has no mark on the rail.
    ///
    /// §121: there is no frame to put such a key at, and a mark at the end
    /// of the rail would say the key is there. §119's diagnostics name it
    /// instead.
    #[test]
    fn a_key_outside_the_design_has_no_mark_on_the_rail() {
        let mut outside = Session::sample();
        let id = outside.document().nodes[0].id;
        outside
            .execute(
                "key.set",
                &serde_json::json!({ "node": id.get(), "path": "amount", "value": 0.5, "time": 30.0 }),
            )
            .expect("the key can be set");
        let mut plain = Session::sample();

        let state = ShellState::default();
        let keyed = crate::raster::render(&mut outside, 900, 600, &state).expect("the frame renders");
        let bare = crate::raster::render(&mut plain, 900, 600, &state).expect("the frame renders");
        let rail = |frame: &crate::raster::Frame| {
            let rect = frame
                .automation
                .rect(AutomationId::playhead().as_str())
                .expect("the playhead has a rectangle");
            crop(&frame.png, rect.left(), rect.top(), rect.width(), rect.height())
        };
        assert_eq!(
            rail(&keyed),
            rail(&bare),
            "a key outside the design is drawn on the rail as if it were inside it"
        );
    }

    /// The width of the timeline's rail, in a frame of the given size.
    fn rail_width(session: &mut Session, width: usize, height: usize) -> f32 {
        let frame =
            crate::raster::render(session, width, height, &ShellState::default()).expect("the frame renders");
        frame
            .automation
            .rect(AutomationId::playhead().as_str())
            .expect("the playhead has a rectangle")
            .width()
    }

    /// The column of a keyframe's mark on the rail, found in the picture.
    ///
    /// The mark is painted in the accent colour when its node is selected,
    /// and the accent colour is nowhere else on the rail. Looking for it in
    /// the pixels is the point: a test that asked the drawing code where it
    /// drew the mark would prove only that the drawing code agrees with
    /// itself.
    fn mark_column(png: &[u8], rect: Rect, colour: egui::Color32) -> Option<usize> {
        let image = image::load_from_memory(png).expect("the frame is a png").to_rgba8();
        let [r, g, b, _] = colour.to_array();
        let near = |got: [u8; 4]| {
            (got[0] as i32 - r as i32).abs() < 24
                && (got[1] as i32 - g as i32).abs() < 24
                && (got[2] as i32 - b as i32).abs() < 24
        };
        for x in rect.left() as u32..=rect.right() as u32 {
            for y in rect.top() as u32..=rect.bottom() as u32 {
                if near(image.get_pixel(x, y).0) {
                    return Some(x as usize);
                }
            }
        }
        None
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

    /// The form is on the screen when it is open.
    ///
    /// A form that is open and not drawn is a form the person cannot answer,
    /// and every other test here would still pass: the state says there is a
    /// prompt, and the picture says there is not. §118.
    #[test]
    fn a_frame_with_a_form_is_not_the_frame_without_one() {
        let mut session = Session::sample();
        let spec = craftkit_engine::get("doc.rename").expect("the command");
        let asked = spec.ask_params();
        let param = **asked.first().expect("the name");

        let open = ShellState {
            prompt: Some(Prompt {
                command: "doc.rename".to_string(),
                fields: vec![PromptField { param, text: "Renamed by the form".to_string() }],
                error: None,
            }),
            ..Default::default()
        };
        let with = crate::raster::render(&mut session, 600, 400, &open).expect("the frame renders");
        let without =
            crate::raster::render(&mut session, 600, 400, &ShellState::default()).expect("the frame renders");

        assert_ne!(with.png, without.png, "the form is not on the screen");
    }

    /// Click a named widget and say where it was.
    ///
    /// Two frames: the first lays the panels out and the second places them,
    /// so the rectangle is the one the person would click.
    fn click_named(
        ctx: &Context,
        session: &mut Session,
        tokens: &Tokens,
        state: &mut ShellState,
        name: &str,
    ) {
        one_frame(ctx, session, tokens, state, egui::RawInput::default());
        let automation = one_frame(ctx, session, tokens, state, egui::RawInput::default());
        let rect =
            automation.rect(name).unwrap_or_else(|| panic!("`{name}` is on the screen and can be clicked"));
        press_at(ctx, session, tokens, state, rect.center().x, rect.center().y);
    }

    /// Type into the field of a form.
    ///
    /// The text goes in as egui's own text event, so the widget that has the
    /// focus is the one that receives it — the same path a keyboard takes.
    fn type_into(
        ctx: &Context,
        session: &mut Session,
        tokens: &Tokens,
        state: &mut ShellState,
        name: &str,
        text: &str,
    ) {
        one_frame(ctx, session, tokens, state, egui::RawInput::default());
        let automation = one_frame(ctx, session, tokens, state, egui::RawInput::default());
        let rect =
            automation.rect(name).unwrap_or_else(|| panic!("`{name}` is on the screen and can be clicked"));
        // The click gives the field the focus.
        press_at(ctx, session, tokens, state, rect.center().x, rect.center().y);
        let input =
            egui::RawInput { events: vec![egui::Event::Text(text.to_string())], ..Default::default() };
        one_frame(ctx, session, tokens, state, input);
    }

    /// A field takes what the soft keyboard commits.
    ///
    /// Android's keyboard does not send key presses. The platform composes the
    /// text in the app's own `InputConnection` and **commits** it, and the host
    /// turns that into egui's own IME event — `Preedit` while the person is
    /// still composing, `Commit` when they are done.
    ///
    /// The tree's fields are egui text fields, so the question is whether they
    /// take that event, and **they do**: the field holds exactly the committed
    /// text, and the preedit that came before it does not double up. Measured.
    /// §234.
    ///
    /// **What this is not**: a soft keyboard. It is the tree's half of one —
    /// the field takes what a keyboard commits — and it runs on the device,
    /// because it is egui and not a window.
    #[test]
    fn a_field_takes_what_the_soft_keyboard_commits() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();

        // Open the form, and give its field the focus.
        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("doc.rename").as_str());
        one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
        let name = AutomationId::prompt_field("doc.rename", "name");
        let rect = automation.rect(name.as_str()).expect("the field is on the screen");
        press_at(&ctx, &mut session, &tokens, &mut state, rect.center().x, rect.center().y);

        // What a keyboard commits: the text being composed, and then the text.
        let input = egui::RawInput {
            events: vec![
                egui::Event::Ime(egui::ImeEvent::Preedit("Via".to_string())),
                egui::Event::Ime(egui::ImeEvent::Commit("Via the IME".to_string())),
            ],
            ..Default::default()
        };
        one_frame(&ctx, &mut session, &tokens, &mut state, input);

        let fields: Vec<String> = state
            .prompt
            .as_ref()
            .map(|p| p.fields.iter().map(|f| f.text.clone()).collect())
            .unwrap_or_default();
        assert_eq!(
            fields,
            vec!["Via the IME".to_string()],
            "the field did not take what the keyboard committed"
        );
    }

    /// A button that needs parameters opens a form instead of being refused.
    ///
    /// §118: the toolbar ran every command with an empty parameter object,
    /// so pressing "Rename plan" said *"`name` this parameter is
    /// required"* and changed nothing.
    #[test]
    fn a_button_that_needs_parameters_opens_a_form() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();

        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("doc.rename").as_str());

        let prompt = state.prompt.as_ref().expect("the form is open");
        assert_eq!(prompt.command, "doc.rename");
        assert_eq!(prompt.fields.len(), 1, "the form asks for the name and nothing else");
        assert_eq!(prompt.fields[0].param.name, "name");
        assert!(!state.status_is_error, "the form is not a refusal: {}", state.status);
        assert_eq!(session.document().meta.name, "Sample plan", "nothing was renamed");
    }

    /// The form runs the command when it is filled in.
    ///
    /// The whole point of the form: the parameters the person typed are the
    /// parameters the command runs with, and the command is the engine's.
    #[test]
    fn the_form_runs_the_command_when_it_is_filled_in() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();

        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("doc.rename").as_str());
        type_into(
            &ctx,
            &mut session,
            &tokens,
            &mut state,
            AutomationId::prompt_field("doc.rename", "name").as_str(),
            "Renamed by the form",
        );
        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::prompt_ok().as_str());

        assert_eq!(session.document().meta.name, "Renamed by the form");
        assert!(state.prompt.is_none(), "the form closed when the command ran");
        assert!(!state.status_is_error, "{}", state.status);
    }

    /// A command whose parameters the shell knows runs at once.
    ///
    /// The shell knows which node the person is looking at, so "Mute" is
    /// one click with a node selected — and two without, because then the
    /// form has to ask which node.
    #[test]
    fn a_button_whose_parameters_the_shell_knows_runs_the_command() {
        let mut session = Session::sample();
        let first = session.document().nodes[0].id;
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState { selected: Some(first), ..Default::default() };

        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("node.mute").as_str());

        assert!(state.prompt.is_none(), "the shell knew the node, so no form was needed");
        assert!(session.document().get(first).expect("the node is there").muted);
        assert!(!state.status_is_error, "{}", state.status);
    }

    /// A command that needs a node asks for one when nothing is selected.
    #[test]
    fn a_button_that_needs_a_node_asks_for_one() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();

        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("node.mute").as_str());

        let prompt = state.prompt.as_ref().expect("the form is open");
        assert_eq!(prompt.command, "node.mute");
        assert_eq!(prompt.fields.len(), 1);
        assert_eq!(prompt.fields[0].param.name, "node");
        assert!(prompt.fields[0].text.is_empty(), "nothing is selected, so nothing to offer");
    }

    /// The form says what is wrong with what was typed.
    ///
    /// A refusal the form cannot show is a refusal the person has to guess
    /// about. §118.
    #[test]
    fn the_form_says_what_is_wrong_with_what_was_typed() {
        let mut session = Session::sample();
        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut state = ShellState::default();

        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::toolbar("doc.range").as_str());
        type_into(
            &ctx,
            &mut session,
            &tokens,
            &mut state,
            AutomationId::prompt_field("doc.range", "fps").as_str(),
            "lots",
        );
        click_named(&ctx, &mut session, &tokens, &mut state, AutomationId::prompt_ok().as_str());

        let prompt = state.prompt.as_ref().expect("the form stays open");
        let error = prompt.error.as_deref().expect("the form says what is wrong");
        assert!(error.contains("not a number"), "{error}");
    }

    /// No toolbar button is refused for wanting a parameter.
    ///
    /// This is the whole of §118, as a test: every command in the registry
    /// has a button, and pressing a button either runs its command or opens
    /// the form that asks for what it needs. Never a refusal.
    #[test]
    fn no_toolbar_button_is_refused_for_wanting_a_parameter() {
        for spec in all() {
            let mut session = Session::sample();
            let ctx = Context::default();
            let tokens = Tokens::dark();
            let mut state = ShellState::default();

            one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
            let automation = one_frame(&ctx, &mut session, &tokens, &mut state, egui::RawInput::default());
            let id = AutomationId::toolbar(spec.id);
            // A disabled button has no rectangle, and cannot be pressed.
            // §107.
            let Some(rect) = automation.rect(id.as_str()) else { continue };
            press_at(&ctx, &mut session, &tokens, &mut state, rect.center().x, rect.center().y);

            assert!(
                !state.status.contains("rejected its parameters"),
                "the toolbar refused `{}` for wanting a parameter: {}",
                spec.id,
                state.status
            );
        }
    }

    // --- §128: the scroll areas that shrink to their content ----------------

    /// The wheel scrolls the node list from anywhere across the panel.
    ///
    /// §128: egui's scroll area shrinks to the width of its content unless
    /// it is told not to, and a scroll area narrower than the panel answers
    /// the wheel only over that narrower strip. The properties panel was told
    /// in §126; the node list never was, so the wheel did nothing over the
    /// panel's right-hand margin.
    ///
    /// The panel is drawn here with a width the test knows, because the point
    /// of the test is the **strip between the content and the panel's edge**,
    /// and a test that cannot say where the panel ends cannot say whether the
    /// pointer was over the list or over the margin.
    #[test]
    fn the_wheel_scrolls_the_node_list_from_across_the_panel() {
        const PANEL: f32 = 240.0;

        let mut doc = craftkit_model::Document::empty("probe");
        for n in 0..40 {
            doc.add_node(format!("node {n}"), craftkit_model::NodeKind::Source).expect("the node");
        }
        let mut session = Session::new("probe");
        session.load_document(doc);
        let first = session.document().nodes[0].id;

        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(&mut session);

        // The width the panel gives the list, measured as the list draws it.
        let mut available = 0.0f32;
        let draw = |shell: &mut Shell, input: egui::RawInput, available: &mut f32| {
            shell.automation.clear();
            let _ = ctx.run(input, |ctx| {
                egui::SidePanel::left("nodes").exact_width(PANEL).show(ctx, |ui| {
                    *available = ui.available_width();
                    shell.node_list(ui, &tokens);
                });
            });
            shell.automation.clone()
        };

        let plain = egui::RawInput { screen_rect: Some(screen()), ..Default::default() };
        let before = draw(&mut shell, plain, &mut available);
        let name = AutomationId::node_row(first);
        let start = before.rect(name.as_str()).expect("the first row has a rectangle");

        // How wide is the content? The widest thing the list draws.
        let mut content = 0.0f32;
        for published in before.names() {
            if let Some(rect) = before.rect(published) {
                content = content.max(rect.right());
            }
        }
        assert!(
            content < available - 4.0,
            "the content is {content:.1} points wide and the panel gives {available:.1}: \
             there is no margin for the test to aim at"
        );

        // The pointer is over the panel's right-hand margin: over the panel,
        // and over no row at all.
        let edge = egui::pos2(available - 2.0, start.center().y);
        let mut after = before.clone();
        for _ in 0..20 {
            after = draw(&mut shell, wheel_at(edge), &mut available);
        }
        let moved = after.rect(name.as_str()).expect("the first row still has a rectangle").top();
        assert!(
            moved < start.top() - 50.0,
            "the list did not scroll from the panel's edge: the first row is still at y={moved:.0}, \
             and the content ends at x={content:.1} while the panel gives {available:.1}"
        );
    }

    /// The wheel scrolls the problems panel from anywhere across the panel.
    ///
    /// §126 fixed the properties panel's scroll area and §128 the node
    /// list's: both shrank to their content, and the strip between the
    /// content and the panel's edge was dead to the wheel. The problems panel
    /// had the same defect and nothing looked — because the content is a line
    /// of text and the strip is **most** of the panel.
    ///
    /// Measured without the fix, in a headless frame, with forty problems in
    /// the panel:
    ///
    /// | | |
    /// |---|---|
    /// | the panel's width | 884 points |
    /// | the problems' content reaches | x = 244.1 |
    /// | the wheel at x = 882 — over the panel, over no problem | **the list did not move** (y = 99.0 → 99.0) |
    /// | the wheel at x = 40 — over the content | the list **scrolled** (y = 99.0 → −34.0) |
    ///
    /// So six hundred and forty of the panel's eight hundred and eighty-four
    /// points were dead to the wheel. §199.
    #[test]
    fn the_wheel_scrolls_the_problems_panel_from_across_the_panel() {
        let mut doc = craftkit_model::Document::empty("probe");
        for n in 0..40 {
            doc.add_node(format!("node {n}"), craftkit_model::NodeKind::Source).expect("the node");
        }
        let mut session = Session::new("probe");
        session.load_document(doc);
        // A node with no `amount` earns a problem, so forty nodes are forty
        // problems — and the panel's cap is a hundred and forty points, so
        // the list has to scroll.
        assert_eq!(session.plan().expect("a plan").diagnostics.items.len(), 40);

        let ctx = Context::default();
        let tokens = Tokens::dark();
        let mut shell = Shell::new(&mut session);

        let mut available = 0.0f32;
        let draw = |shell: &mut Shell, input: egui::RawInput, available: &mut f32| {
            shell.automation.clear();
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    *available = ui.available_width();
                    shell.plan_and_timeline(ui, &tokens);
                });
            });
            shell.automation.clone()
        };

        let plain = egui::RawInput { screen_rect: Some(screen()), ..Default::default() };
        let before = draw(&mut shell, plain, &mut available);
        let name = AutomationId::problem(0);
        let first = before.rect(name.as_str()).expect("the first problem has a rectangle");

        // The pointer is over the panel's right-hand margin: over the panel,
        // and over no problem at all.
        let edge = egui::pos2(available - 2.0, first.center().y);
        let mut after = before.clone();
        for _ in 0..20 {
            after = draw(&mut shell, wheel_at(edge), &mut available);
        }
        let moved = after.rect(name.as_str()).expect("the first problem still has a rectangle").top();
        assert!(
            moved < first.top() - 50.0,
            "the problems panel did not scroll from the panel's edge: the first problem is still at \
             y={moved:.0}, and the panel gives {available:.1} points"
        );
    }

    // --- §127: the node list's rows ---------------------------------------

    /// A document of `count` nodes, all of them sources.
    fn many_nodes(count: usize) -> craftkit_model::Document {
        let mut doc = craftkit_model::Document::empty("probe");
        for n in 0..count {
            doc.add_node(format!("node {n}"), craftkit_model::NodeKind::Source).expect("the node");
        }
        doc
    }

    /// The rows collected the **slow** way: the plan is asked about every node.
    ///
    /// This is the reference the gate measures the real collection against. It
    /// is deliberately the way the panel used to do it — one scan of the plan
    /// per node — so the gate compares the production code against the thing
    /// it replaced, on the same machine, in the same run.
    fn rows_by_scanning(
        document: &craftkit_model::Document,
        plan: Option<&craftkit_eval::Plan>,
    ) -> Vec<NodeRow> {
        document
            .nodes
            .iter()
            .map(|n| {
                let level = plan.and_then(|p| p.state(n.id)).map(|s| s.level_out).unwrap_or(0.0);
                NodeRow {
                    id: n.id,
                    name: n.name.clone(),
                    kind: n.kind.label().to_string(),
                    muted: n.muted,
                    level,
                }
            })
            .collect()
    }

    /// The cheapest of `rounds` runs of `work`, in seconds.
    ///
    /// The **minimum**, not the mean: the question is how fast the code *can*
    /// go, and a round that was interrupted by the machine is a round that
    /// says nothing about the code.
    fn fastest(rounds: usize, mut work: impl FnMut()) -> f64 {
        let mut best = f64::MAX;
        for _ in 0..rounds {
            let start = std::time::Instant::now();
            work();
            let each = start.elapsed().as_secs_f64();
            if each < best {
                best = each;
            }
        }
        best
    }

    /// The node list carries the level the plan resolved.
    ///
    /// §127: the rows used to ask the plan for each node's level, and the
    /// plan finds a node by scanning. The fix asks **once**. Either way the
    /// answer has to be the plan's answer: a row that said level zero for
    /// every node would look right on screen — a bar with nothing in it — and
    /// would be wrong.
    #[test]
    fn the_node_list_carries_the_level_the_plan_resolved() {
        let mut session = Session::new("probe");
        session.load_document(many_nodes(64));
        let plan = session.plan().expect("the plan has been computed");

        let rows = node_rows(session.document(), Some(plan));
        assert_eq!(rows.len(), session.document().nodes.len());
        for (row, state) in rows.iter().zip(plan.nodes.iter()) {
            assert_eq!(row.id, state.id);
            assert_eq!(row.level, state.level_out, "node {} has the wrong level", row.id.get());
        }
        // A level is not zero by accident: the sources start at one.
        assert!(rows.iter().all(|r| r.level > 0.0), "every source should carry a level");

        // With no plan at all there is nothing to resolve, and a row says so
        // with a level of zero rather than inventing one.
        let without = node_rows(session.document(), None);
        assert!(without.iter().all(|r| r.level == 0.0));
    }

    /// The node list does not scan the plan once per node.
    ///
    /// §127: the plan answers *"what is this node's level?"* by scanning its
    /// own nodes, so asking it about every node was a scan per node —
    /// quadratic. At the model's own limit of ten thousand nodes that was
    /// **43.7 ms** of collecting, and the panel paid it every frame.
    ///
    /// The gate measures the production collection against the way it was
    /// done before, **in the same run, on the same machine**: the fast way
    /// has to be at least four times cheaper than the scan. Measured: 1.2 ms
    /// against 43.7 ms — thirty-six times.
    ///
    /// §134: the threshold was **eight**, and eight was a number read off a
    /// quiet machine. The fast way takes about a millisecond and the scan
    /// takes forty, so a moment when the machine is busy is *all* of the fast
    /// measurement and a twentieth of the slow one — and the ratio fell to
    /// **7.3 times**, below the threshold, on a machine that was not wrong.
    /// The fast way is therefore measured in bulk, so that both sides of the
    /// comparison span about the same half-second of the machine's time, and
    /// the threshold is **four**: the scan this gate replaces is *identical* to
    /// the fast way — a ratio of one — so four is four times the margin.
    #[test]
    fn the_node_list_does_not_scan_the_plan_once_per_node() {
        let mut session = Session::new("probe");
        session.load_document(many_nodes(10_000));
        assert_eq!(session.document().nodes.len(), 10_000, "the model's own limit");

        // The fast way is measured in bulk. One call is about a millisecond,
        // and a nine-millisecond hiccup in the machine is all of it; thirty-two
        // calls are about as long as one scan, so the two sides of the
        // comparison are measured over the same kind of time.
        const CALLS: usize = 32;
        let collected = fastest(9, || {
            for _ in 0..CALLS {
                std::hint::black_box(node_rows(session.document(), session.plan()));
            }
        }) / CALLS as f64;
        let scanned = fastest(9, || {
            std::hint::black_box(rows_by_scanning(session.document(), session.plan()));
        });

        assert!(
            collected * 4.0 < scanned,
            "collecting the rows of ten thousand nodes took {collected:.6}s, and asking the plan \
             about every node took {scanned:.6}s: the rows are still collected a scan at a time"
        );
    }

    /// The node list draws the rows the collector found.
    ///
    /// §127: the panel's rows are collected by [`node_rows`], and this is the
    /// gate that ties the two together — the panel draws exactly the rows
    /// that were collected, and no others. Without it the collector could be
    /// correct and fast while the panel drew rows of its own, and nothing
    /// would say so.
    #[test]
    fn the_node_list_draws_the_rows_the_collector_found() {
        let mut session = Session::new("probe");
        session.load_document(many_nodes(12));
        let rows = node_rows(session.document(), session.plan());

        let (_state, registry) = draw(&mut session);
        for row in &rows {
            let name = AutomationId::node_row(row.id);
            assert!(
                registry.known(name.as_str()) != Widget::Missing,
                "node {} was collected but has no row on the screen",
                row.id.get()
            );
        }
        let published = registry
            .names()
            .into_iter()
            .filter(|n| n.starts_with("nodes.row.") && !n.ends_with(".mute") && !n.ends_with(".del"))
            .count();
        assert_eq!(published, rows.len(), "the panel drew {published} rows for {} nodes", rows.len());
    }
}
