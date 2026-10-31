//! The window itself: the drawing, and nothing else.
//!
//! Everything the window knows how to do is in [`craftkit_window`] —
//! the registry, the document, the plan, and the one way to run a
//! command. This file draws them and hands the values back.

use egui::Widget;

use craftkit_sdk::Doc;
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_window::Window;

/// A **drag** on the picture.
///
/// A drag that starts on a node **moves** the node, and a drag that
/// starts on the background **moves the camera**. That is the rule
/// every drawing program uses, and it is one rule and not two: what is
/// under the pointer says which of the two it is.
struct Drag {
    /// The node being moved, if there is one.
    node: Option<u64>,
    /// The node's own place, when a node is being moved.
    place: Option<craftkit_sdk::Rect>,
    /// Where the drag started, in the world.
    from: craftkit_sdk::Point,
}

/// The window's own state, and the values being edited.
struct App {
    /// The kit, as the window sees it.
    window: Window,
    /// The command being filled in.
    open: Option<String>,
    /// The values of the form, by name.
    ///
    /// They are the window's own while they are being edited, and they
    /// are handed back the moment they change: the form in the window
    /// is the one the command is run with.
    values: Vec<(String, Json)>,
    /// The picture of the document, and the texture it is shown as.
    picture: Option<egui::TextureHandle>,
    /// The drag that is going on, if one is.
    drag: Option<Drag>,
    /// What the picture was drawn from.
    ///
    /// The picture is drawn again when the document, the camera or the
    /// command count has changed — and not on every frame, because
    /// drawing a document is work and a window that draws it sixty
    /// times a second is a window that is always busy.
    drawn: Option<(usize, u64, f64, f64, f64)>,
}

impl App {
    /// A window over the kit.
    fn new() -> Result<App, Box<dyn std::error::Error + Send + Sync>> {
        let window =
            Window::new().map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)?;
        Ok(App { window, open: None, values: Vec::new(), picture: None, drawn: None, drag: None })
    }

    /// **Open** a command: the form of what it wants.
    fn open(&mut self, id: &str) {
        if !self.window.open(id) {
            return;
        }
        self.open = Some(id.to_string());
        self.values = self
            .window
            .form()
            .map(|form| form.fields.iter().map(|field| (field.name.clone(), field.value.clone())).collect())
            .unwrap_or_default();
    }

    /// The value of one field, as it is being edited.
    fn value(&self, name: &str) -> Json {
        self.values
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.clone())
            .unwrap_or(Json::Null)
    }

    /// **Set** one field, and tell the window.
    fn set(&mut self, name: &str, value: Json) {
        if let Some(entry) = self.values.iter_mut().find(|(field, _)| field == name) {
            entry.1 = value.clone();
        }
        self.window.set(name, value);
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };
    let _ = eframe::run_native("craftkit", options, Box::new(|_cc| Ok(Box::new(App::new()?))));
}

/// The **commands**, by area, on the left.
///
/// Every command that was linked in is here, and every one of them is
/// a button: the window has no list of its own.
fn commands(app: &mut App, ui: &mut egui::Ui) {
    // The commands are **copied** out of the registry before they are
    // drawn: the window draws them and then runs one, and the two
    // cannot borrow the same registry at once.
    let areas: Vec<String> = app.window.areas().iter().map(|area| area.to_string()).collect();
    let commands: Vec<(String, String, String, String)> = app
        .window
        .commands()
        .iter()
        .map(|command| {
            (
                command.area.to_string(),
                command.id.to_string(),
                command.label.to_string(),
                command.doc.to_string(),
            )
        })
        .collect();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for area in &areas {
            let mut commands: Vec<&(String, String, String, String)> =
                commands.iter().filter(|command| command.0 == *area).collect();
            commands.sort_by(|left, right| left.1.cmp(&right.1));
            egui::CollapsingHeader::new(area).default_open(false).show(ui, |ui| {
                for command in commands {
                    let label = format!("{} — {}", command.2, command.1);
                    if ui.button(label).clicked() {
                        app.open(&command.1);
                    }
                    if !command.3.is_empty() {
                        ui.label(&command.3).on_hover_text(&command.3);
                    }
                }
            });
        }
    });
}

/// The **document**, in the middle: the tree of nodes, and the picture
/// of it.
fn tree(app: &App, ui: &mut egui::Ui) {
    ui.heading("the document");
    let rows = app.window.tree();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for row in rows {
            ui.label(format!("{}  {}  [{}]", row.id, row.name, row.kind));
        }
    });
}

/// The **picture** of the document, drawn through the kinds.
fn picture(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    let camera = app.window.session().camera();
    let offset = camera.offset();
    let drawn = (app.window.tree().len(), app.window.runs(), camera.zoom(), offset.x, offset.y);
    if app.drawn != Some(drawn) {
        let size = ui.available_size();
        let width = size.x.max(1.0) as usize;
        let height = size.y.max(1.0) as usize;
        app.picture = match app.window.picture(width, height) {
            Ok(picture) => {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [picture.width, picture.height],
                    &picture.pixels,
                );
                Some(ctx.load_texture("the document", image, egui::TextureOptions::LINEAR))
            }
            Err(_) => None,
        };
        app.drawn = Some(drawn);
    }
    let Some(texture) = app.picture.clone() else {
        ui.label("the document cannot be drawn");
        return;
    };
    let size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
    let response = ui.image((texture.id(), size));
    interact(app, ui, &response);
}

/// The **mouse** on the picture: drag to move it, the wheel to zoom it,
/// and a click to pick what is under it.
///
/// The camera is the **session's**, and the mapping from a point of the
/// screen to a point of the world is the **camera's** too — so what is
/// drawn and what is clicked are the same thing.
fn interact(app: &mut App, ui: &egui::Ui, response: &egui::Response) {
    let camera = *app.window.session().camera();
    // A drag that starts on a node moves the node, and a drag that
    // starts on the background moves the camera.
    if response.drag_started() {
        let pointer = response.interact_pointer_pos().unwrap_or(response.rect.center());
        let centre = response.rect.center();
        let world = camera
            .to_world(craftkit_sdk::Point::new((pointer.x - centre.x) as f64, (centre.y - pointer.y) as f64));
        let picked = app.window.pick(world);
        let place =
            picked.and_then(|node| app.window.session().document().node(node).and_then(|node| node.rect));
        // A node with **no place** is not moved: there is nowhere to
        // move it to.
        let node = if place.is_some() { picked } else { None };
        app.drag = Some(Drag { node, place, from: world });
    }
    if response.dragged() && app.drag.as_ref().map(|drag| drag.node.is_none()).unwrap_or(true) {
        let delta = response.drag_delta();
        let zoom = camera.zoom();
        app.window.pan(-delta.x as f64 / zoom, delta.y as f64 / zoom);
    }
    // The node moves when the mouse is let go: every move is a command,
    // and every command is an undo step.
    if response.drag_stopped()
        && let Some(drag) = app.drag.take()
        && let (Some(node), Some(place)) = (drag.node, drag.place)
    {
        let pointer = response.interact_pointer_pos().unwrap_or(response.rect.center());
        let centre = response.rect.center();
        let world = camera
            .to_world(craftkit_sdk::Point::new((pointer.x - centre.x) as f64, (centre.y - pointer.y) as f64));
        let across = world.x - drag.from.x;
        let up = world.y - drag.from.y;
        let _ = app.window.move_node(node, place.x + across, place.y + up);
    }
    // The wheel zooms, and the camera clamps it.
    if response.hovered() {
        let scroll = ui.input(|input| input.smooth_scroll_delta);
        if scroll.y != 0.0 {
            app.window.zoom_by(1.0 + scroll.y as f64 * 0.1);
        }
    }
    // A click picks the node under it, and selects it through the
    // registry.
    if response.clicked()
        && let Some(position) = response.interact_pointer_pos()
    {
        let centre = response.rect.center();
        let across = position.x - centre.x;
        let down = centre.y - position.y;
        let world = camera.to_world(craftkit_sdk::Point::new(across as f64, down as f64));
        if let Some(node) = app.window.pick(world) {
            let _ = app.window.select_node(node);
        }
    }
}

/// The **plan**, on the right: what the document means.
fn plan(app: &App, ui: &mut egui::Ui) {
    ui.heading("the plan");
    let rows = app.window.plan();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for row in rows {
            ui.label(format!("{}  {}  {}", row.node, row.kind, row.value));
        }
    });
}

/// The **form**, at the bottom: what the command wants.
fn form(app: &mut App, ui: &mut egui::Ui) {
    let Some(id) = app.open.clone() else { return };
    let Some(form) = app.window.form().cloned() else { return };
    ui.horizontal(|ui| {
        ui.heading(format!("{} — {}", form.label, id));
        let nodes: Vec<(u64, String)> = app.window.tree().into_iter().map(|row| (row.id, row.name)).collect();
        let kinds: Vec<String> =
            app.window.session().kinds().ids().iter().map(|kind| kind.to_string()).collect();
        for field in &form.fields {
            ui.label(format!("{}:", field.name));
            match field.kind {
                Doc::Number => {
                    let mut value = app.value(&field.name).as_f64().unwrap_or(0.0);
                    let edit = egui::DragValue::new(&mut value).speed(0.1).ui(ui);
                    if edit.changed() {
                        app.set(&field.name, json!(value));
                    }
                }
                Doc::Text => {
                    let mut value = app.value(&field.name).as_str().unwrap_or("").to_string();
                    let edit = ui.text_edit_singleline(&mut value);
                    if edit.changed() {
                        app.set(&field.name, json!(value));
                    }
                }
                Doc::Bool => {
                    let mut value = app.value(&field.name).as_bool().unwrap_or(false);
                    if ui.checkbox(&mut value, "").changed() {
                        app.set(&field.name, json!(value));
                    }
                }
                Doc::NodeRef => {
                    let current = app.value(&field.name).as_u64().unwrap_or(0);
                    let names: Vec<String> =
                        nodes.iter().map(|(id, name)| format!("{id} — {name}")).collect();
                    let selected = nodes.iter().position(|(id, _)| *id == current);
                    egui::ComboBox::from_label("")
                        .selected_text(
                            selected
                                .map(|index| names[index].clone())
                                .unwrap_or_else(|| "nothing".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for (index, name) in names.iter().enumerate() {
                                if ui.selectable_label(Some(index) == selected, name).clicked() {
                                    app.set(&field.name, json!(nodes[index].0));
                                }
                            }
                        });
                }
                Doc::NodeKind => {
                    let current = app.value(&field.name).as_str().unwrap_or("").to_string();
                    let selected = kinds.iter().position(|kind| *kind == current);
                    egui::ComboBox::from_label("")
                        .selected_text(
                            selected
                                .map(|index| kinds[index].clone())
                                .unwrap_or_else(|| "nothing".to_string()),
                        )
                        .show_ui(ui, |ui| {
                            for (index, kind) in kinds.iter().enumerate() {
                                if ui.selectable_label(Some(index) == selected, kind).clicked() {
                                    app.set(&field.name, json!(kind));
                                }
                            }
                        });
                }
                Doc::Json => {
                    ui.label(app.value(&field.name).to_string());
                }
            }
            if !field.help.is_empty() {
                ui.label(&field.help).on_hover_text(&field.help);
            }
        }
        if ui.button("run").clicked() {
            let _ = app.window.run();
            app.open = None;
            app.values.clear();
        }
    });
}

/// The **log**: what the window has to say.
fn log(app: &App, ui: &mut egui::Ui) {
    for line in app.window.log() {
        ui.label(line);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("craftkit");
            ui.label("the registry, the document and the plan — and nothing else");
        });
        egui::SidePanel::left("commands").default_width(320.0).show(ctx, |ui| {
            ui.heading("the commands");
            commands(self, ui);
        });
        egui::SidePanel::right("plan").default_width(360.0).show(ctx, |ui| {
            plan(self, ui);
        });
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            form(self, ui);
            log(self, ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            picture(self, ctx, ui);
            tree(self, ui);
        });
    }
}
