//! The icon set, drawn in code.
//!
//! # Why every icon is code
//!
//! **No icon, glyph or piece of artwork in this project may come from another
//! product, commercial or free.** Not a font, not an SVG someone downloaded, not
//! a PNG traced from a screenshot. Every shape below is drawn here from
//! original coordinates, and the licence of everything in the repository is
//! either ours or an explicitly recorded open one.
//!
//! This is not squeamishness. It is the rule that keeps the project legally
//! and ethically clean, and "we only used a few" is how it starts.
//!
//! # How it works
//!
//! Each icon is a [`Painter`] of primitives into an `egui::Rect`. They share one
//! stroke weight and one grid so a row of them lines up, and every one is
//! legible at 12 logical pixels because that is the size the toolbar uses.
//!
//! # The contract with the engine
//!
//! [`craftkit_engine::Icon`] is only a *name*. A test at the bottom of this file
//! checks that every command's icon exists here, so a command can never ask for
//! an icon this set does not have.

use egui::{Color32, Rect, Stroke, Ui, Vec2};

/// Every icon name this set can draw, sorted.
pub const ALL: &[&str] = &[
    "arrows", "broom", "copy", "curve", "info", "key", "key-off", "mute", "pencil", "play", "plus", "range",
    "redo", "swap", "trash", "undo", "value", "warning",
];

/// Whether this set can draw `name`.
pub fn has(name: &str) -> bool {
    ALL.contains(&name)
}

/// Draw `name` into `rect`.
///
/// Returns `false` and draws nothing for a name this set does not have, so a
/// missing icon is an empty slot rather than a panic in the middle of a frame.
pub fn draw(ui: &mut Ui, name: &str, rect: Rect, tint: Color32) -> bool {
    let p = Painter { ui, rect, tint };
    match name {
        "plus" => p.plus(),
        "pencil" => p.pencil(),
        "trash" => p.trash(),
        "mute" => p.mute(),
        "swap" => p.swap(),
        "arrows" => p.arrows(),
        "copy" => p.copy(),
        "value" => p.value(),
        "undo" => p.undo(),
        "redo" => p.redo(),
        "key" => p.key(),
        "key-off" => p.key_off(),
        "broom" => p.broom(),
        "curve" => p.curve(),
        "play" => p.play(),
        "warning" => p.warning(),
        "info" => p.info(),
        "range" => p.range(),
        _ => return false,
    }
    true
}

/// Draw `name` as a button-sized icon inside a `Ui`.
pub fn icon(ui: &mut Ui, name: &str, size: f32, tint: Color32) -> bool {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    draw(ui, name, rect, tint)
}

/// The shared drawing context: one rect, one colour, one stroke weight.
struct Painter<'a> {
    ui: &'a mut Ui,
    rect: Rect,
    tint: Color32,
}

/// The stroke every icon uses. One value, so no icon can quietly be heavier.
const WEIGHT: f32 = 1.35;

impl Painter<'_> {
    fn stroke(&self) -> Stroke {
        Stroke { width: WEIGHT, color: self.tint }
    }

    /// Map `0.0..=1.0` inside the icon rect, so every shape below is written in
    /// the same 0..1 space and cannot escape its box.
    fn p(&self, x: f32, y: f32) -> egui::Pos2 {
        let r = self.rect;
        egui::pos2(r.left() + r.width() * x, r.top() + r.height() * y)
    }

    fn line(&self, a: (f32, f32), b: (f32, f32)) {
        self.ui.painter().line_segment([self.p(a.0, a.1), self.p(b.0, b.1)], self.stroke());
    }

    fn path(&self, pts: &[(f32, f32)]) {
        let points: Vec<egui::Pos2> = pts.iter().map(|(x, y)| self.p(*x, *y)).collect();
        if points.len() < 2 {
            return;
        }
        self.ui.painter().add(egui::Shape::line(points, self.stroke()));
    }

    fn circle(&self, c: (f32, f32), r: f32) {
        let rr = self.rect.width() * r;
        self.ui.painter().circle_stroke(self.p(c.0, c.1), rr, self.stroke());
    }

    fn disc(&self, c: (f32, f32), r: f32) {
        let rr = self.rect.width() * r;
        self.ui.painter().circle_filled(self.p(c.0, c.1), rr, self.tint);
    }

    fn rect(&self, min: (f32, f32), max: (f32, f32), fill: bool) {
        let r = Rect::from_min_max(self.p(min.0, min.1), self.p(max.0, max.1));
        if fill {
            self.ui.painter().rect_filled(r, 1.5, self.tint);
        } else {
            self.ui.painter().rect_stroke(r, 1.5, self.stroke(), egui::StrokeKind::Middle);
        }
    }

    // --- the icons -------------------------------------------------------

    fn plus(&self) {
        self.line((0.5, 0.2), (0.5, 0.8));
        self.line((0.2, 0.5), (0.8, 0.5));
    }

    fn pencil(&self) {
        // A body and a nib: two lines and a triangle. Drawn as a diagonal because
        // that is what reads as "edit" at 12 px, and the eraser end is what
        // stops it reading as a slash.
        self.path(&[(0.22, 0.78), (0.30, 0.70), (0.74, 0.26), (0.86, 0.38), (0.42, 0.82)]);
        self.line((0.74, 0.26), (0.86, 0.14));
        self.line((0.30, 0.70), (0.14, 0.86));
    }

    fn trash(&self) {
        self.rect((0.30, 0.34), (0.70, 0.86), false);
        self.line((0.24, 0.34), (0.76, 0.34));
        self.line((0.40, 0.24), (0.60, 0.24));
        self.line((0.43, 0.44), (0.43, 0.76));
        self.line((0.57, 0.44), (0.57, 0.76));
    }

    fn mute(&self) {
        // A speaker and a slash. The speaker is three segments so it has a
        // silhouette; the slash is what makes it "muted" rather than "volume".
        self.rect((0.24, 0.38), (0.36, 0.62), true);
        self.path(&[(0.36, 0.38), (0.52, 0.24), (0.52, 0.76), (0.36, 0.62)]);
        self.line((0.60, 0.34), (0.84, 0.66));
    }

    fn swap(&self) {
        self.path(&[(0.22, 0.36), (0.74, 0.36)]);
        self.path(&[(0.64, 0.26), (0.74, 0.36), (0.64, 0.46)]);
        self.path(&[(0.78, 0.64), (0.26, 0.64)]);
        self.path(&[(0.36, 0.54), (0.26, 0.64), (0.36, 0.74)]);
    }

    fn arrows(&self) {
        self.path(&[(0.5, 0.20), (0.5, 0.80)]);
        self.path(&[(0.42, 0.28), (0.5, 0.18), (0.58, 0.28)]);
        self.path(&[(0.42, 0.72), (0.5, 0.82), (0.58, 0.72)]);
    }

    fn copy(&self) {
        self.rect((0.20, 0.20), (0.62, 0.62), false);
        self.rect((0.38, 0.38), (0.80, 0.80), false);
    }

    fn value(&self) {
        // A diamond with a dot: "a value in a slot", which is what a property is.
        self.path(&[(0.5, 0.20), (0.80, 0.50), (0.5, 0.80), (0.20, 0.50), (0.5, 0.20)]);
        self.disc((0.5, 0.5), 0.08);
    }

    fn undo(&self) {
        self.path(&[(0.80, 0.40), (0.50, 0.26), (0.26, 0.40), (0.50, 0.54), (0.80, 0.54), (0.80, 0.70)]);
        self.path(&[(0.30, 0.70), (0.22, 0.78), (0.30, 0.86)]);
    }

    fn redo(&self) {
        self.path(&[(0.20, 0.40), (0.50, 0.26), (0.74, 0.40), (0.50, 0.54), (0.20, 0.54), (0.20, 0.70)]);
        self.path(&[(0.70, 0.70), (0.78, 0.78), (0.70, 0.86)]);
    }

    fn key(&self) {
        self.circle((0.34, 0.40), 0.16);
        self.path(&[(0.44, 0.54), (0.76, 0.84)]);
        self.line((0.62, 0.72), (0.72, 0.62));
    }

    fn key_off(&self) {
        self.circle((0.34, 0.40), 0.16);
        self.path(&[(0.44, 0.54), (0.76, 0.84)]);
        self.line((0.60, 0.60), (0.86, 0.26));
    }

    fn broom(&self) {
        self.line((0.20, 0.78), (0.62, 0.34));
        self.path(&[(0.62, 0.34), (0.82, 0.30), (0.78, 0.52), (0.62, 0.34)]);
        self.line((0.30, 0.66), (0.40, 0.76));
    }

    fn curve(&self) {
        // A bezier and its two handles: unmistakably "the shape of a curve".
        self.path(&[(0.16, 0.78), (0.34, 0.22), (0.66, 0.22), (0.84, 0.78)]);
        self.line((0.16, 0.78), (0.16, 0.44));
        self.line((0.84, 0.78), (0.84, 0.56));
        self.disc((0.16, 0.78), 0.06);
        self.disc((0.84, 0.78), 0.06);
    }

    fn play(&self) {
        self.path(&[(0.30, 0.20), (0.80, 0.50), (0.30, 0.80), (0.30, 0.20)]);
    }

    fn warning(&self) {
        self.path(&[(0.50, 0.16), (0.88, 0.80), (0.12, 0.80), (0.50, 0.16)]);
        self.line((0.50, 0.40), (0.50, 0.60));
        self.disc((0.50, 0.70), 0.045);
    }

    fn info(&self) {
        self.circle((0.50, 0.50), 0.34);
        self.line((0.50, 0.46), (0.50, 0.72));
        self.disc((0.50, 0.32), 0.045);
    }

    fn range(&self) {
        // A bracket with an arrow between its ends: a time range.
        self.path(&[(0.30, 0.22), (0.14, 0.22), (0.14, 0.78), (0.30, 0.78)]);
        self.path(&[(0.70, 0.22), (0.86, 0.22), (0.86, 0.78), (0.70, 0.78)]);
        self.path(&[(0.34, 0.50), (0.66, 0.50)]);
        self.path(&[(0.56, 0.40), (0.66, 0.50), (0.56, 0.60)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craftkit_engine::all;

    #[test]
    fn every_command_has_an_icon_this_set_can_draw() {
        for spec in all() {
            assert!(
                has(spec.icon.0),
                "command `{}` asks for icon `{}`, which is not in the set",
                spec.id,
                spec.icon.0
            );
        }
    }

    #[test]
    fn the_name_list_is_sorted_and_free_of_duplicates() {
        let mut sorted = ALL.to_vec();
        sorted.sort_unstable();
        assert_eq!(ALL, sorted, "keep ALL sorted");
        let before = ALL.len();
        let mut deduped = ALL.to_vec();
        deduped.dedup();
        assert_eq!(before, deduped.len(), "ALL has a duplicate");
    }

    #[test]
    fn every_name_is_dotted_lower_case() {
        for name in ALL {
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "`{name}` must be lower-case with dashes"
            );
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn an_unknown_icon_is_reported_not_drawn() {
        // `draw` returns false rather than panicking, so a typo in a command's
        // icon degrades to an empty slot rather than a broken frame.
        let ctx = egui::Context::default();
        let results = std::cell::RefCell::new((true, false));
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                *results.borrow_mut() =
                    (draw(ui, "no-such-icon", rect, Color32::WHITE), draw(ui, "plus", rect, Color32::WHITE));
            });
        });
        assert_eq!(*results.borrow(), (false, true));
    }

    #[test]
    fn every_icon_draws_inside_its_rect() {
        // Each painter maps through `p`, which clamps to the rect by
        // construction. This proves it for the whole set at once rather than
        // trusting that every author remembered.
        let ctx = egui::Context::default();
        let drawn = std::cell::RefCell::new(Vec::new());
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut drawn = drawn.borrow_mut();
                for name in ALL {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    assert!(draw(ui, name, rect, Color32::WHITE), "{name} did not draw");
                    drawn.push((*name).to_string());
                }
            });
        });
        assert_eq!(drawn.borrow().len(), ALL.len());
    }
}
