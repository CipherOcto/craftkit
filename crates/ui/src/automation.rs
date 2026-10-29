//! Addressing widgets by name, so an agent can find one.
//!
//! # The problem this solves
//!
//! "Click the button that deletes the selected node" is not a thing an agent
//! can say to a pixel buffer. What it *can* do is drive the same commands the
//! buttons do — but that misses the cases where the point is to exercise the
//! **UI path**: proving a button works, that a panel renders, that a keyboard
//! shortcut fires.
//!
//! # The contract
//!
//! Every widget that matters carries an [`AutomationId`] — a stable dotted name
//! like `toolbar.node.add` or `nodes.row.4`. The name is derived from the
//! command id where there is one, so a command and its button can never drift
//! apart, and the registry knows every name that exists.
//!
//! Two rules make this useful rather than decorative:
//!
//! 1. **A name is stable.** It is derived from ids and positions, never from a
//!    hash of a label or a closure.
//! 2. **A name is checked.** [`Automation::known`] rejects a name nobody
//!    publishes, so an agent's screenshot check fails loudly rather than
//!    silently clicking nothing.
//!
//! And one that keeps the process alive:
//!
//! 3. **A name is owned.** It is a [`String`], not a `&'static str`. The names
//!    are built *every frame* — the toolbar's, every node row's, every
//!    property editor's — so a name that cannot be given back is a process
//!    that grows for as long as the window is open. §66.
//!
//! And one that keeps a lookup honest:
//!
//! 4. **A name is one widget.** Two widgets wanting one name is a collision:
//!    the first is published and the second is **not in the registry at all**,
//!    so a lookup for it succeeds and answers with the other widget — in
//!    another panel, doing something else. The collision is recorded, so it
//!    can be refused instead of swallowed. §99.
//!
//! And one that keeps the registry and the screen in step:
//!
//! 5. **A name is a widget that is there.** Every widget the shell draws
//!    that somebody might click carries a name, so "what is at this point?"
//!    has an answer for the button under the pointer — and a name never
//!    promises a widget that is not drawn, because a name that promises one
//!    sends an agent looking for something that is not there. §100.

use egui::{Id, Rect};
use std::collections::HashSet;

/// A stable widget name.
///
/// The name is **owned**, not leaked.
///
/// It used to be a `&'static str` made by `Box::leak`, because a `&'static
/// str` is `Copy` and the registry's lookups were written against one. But
/// the names are built *every frame* — the toolbar's, every node row's, every
/// property editor's — so leaking one per widget per frame leaked the
/// process's memory for as long as the window was open. Measured: 1 077 bytes
/// a frame, which is 221 MB an hour at sixty frames a second. §66.
///
/// The name is still **stable**: it is derived from ids and positions, never
/// from a hash of a label or a closure, and two builds of the same name
/// compare equal. `egui`'s widget identity hashes the name's *contents*, so
/// a widget keeps its identity across frames even though the string behind
/// it is a fresh allocation each time.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AutomationId(pub String);

impl AutomationId {
    /// The name for the button that runs `command_id`.
    ///
    /// `node.add` → `toolbar.node.add`. The prefix says where it lives, so two
    /// buttons that run the same command in different panels do not collide.
    pub fn toolbar(command_id: &str) -> Self {
        AutomationId(format!("toolbar.{command_id}"))
    }

    /// The name for the row of node `id`.
    pub fn node_row(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("nodes.row.{}", id.get()))
    }

    /// The name for the mute button in the row of node `id`.
    pub fn node_mute(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("nodes.row.{}.mute", id.get()))
    }

    /// The name for the delete button in the row of node `id`.
    pub fn node_remove(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("nodes.row.{}.del", id.get()))
    }

    /// The name for the playhead slider in the timeline.
    pub fn playhead() -> Self {
        AutomationId("timeline.playhead".to_string())
    }

    /// The name for the `index`th line of the problems panel.
    ///
    /// The problems panel's lines are what a person reads when the plan says
    /// something is wrong, so an agent has to be able to name them the same
    /// way. §199.
    pub fn problem(index: usize) -> Self {
        AutomationId(format!("problems.item.{index}"))
    }

    /// The name for the path field of the "key at the playhead" row.
    pub fn key_path() -> Self {
        AutomationId("properties.key.path".to_string())
    }

    /// The name for the value field of the "key at the playhead" row.
    pub fn key_value() -> Self {
        AutomationId("properties.key.value".to_string())
    }

    /// The name for the editor of `path` on node `id`.
    pub fn property(id: craftkit_core::NodeId, path: &str) -> Self {
        AutomationId(format!("node.{}.prop.{}", id.get(), path))
    }

    /// The name for the level bar of node `id`.
    ///
    /// The level bar is **not a property**, so it does not publish a
    /// property's name. It used to publish `node.<id>.prop.$level`, which is
    /// a property's name spelled the way a property spelled `$level` spells
    /// it — and a property may be called `$level`, so the property's editor
    /// and the level bar wanted the same name, and the property's editor
    /// lost: the first name published in a frame wins, and the node list is
    /// drawn before the properties panel. §99.
    pub fn level_bar(id: craftkit_core::NodeId) -> Self {
        AutomationId(format!("node.{}.level", id.get()))
    }

    /// The name for the "clear keys" button of `path` on node `id`.
    ///
    /// It used to be published as `node.<id>.key.<path>`, which is the name of
    /// a **keyframe row** — a widget the shell does not draw. A name that
    /// promises a widget that is not on screen sends an agent looking for
    /// something it will never find, and the button it does find answers under
    /// a name that says otherwise. §100.
    pub fn clear_keys(id: craftkit_core::NodeId, path: &str) -> Self {
        AutomationId(format!("node.{}.keys.{}.clear", id.get(), path))
    }

    /// The name for the field of `param` in the form for `command`.
    ///
    /// The form is a set of widgets like any other, and an agent that can
    /// name them can fill them in. §118.
    pub fn prompt_field(command: &str, param: &str) -> Self {
        AutomationId(format!("prompt.{command}.{param}"))
    }

    /// The name for the form's "OK" button.
    pub fn prompt_ok() -> Self {
        AutomationId("prompt.ok".to_string())
    }

    /// The name for the form's "Cancel" button.
    pub fn prompt_cancel() -> Self {
        AutomationId("prompt.cancel".to_string())
    }

    /// The `egui` id, so two widgets cannot collide.
    ///
    /// Salted with the crate name so a name shared with some other egui
    /// application in the same process does not collide either.
    pub fn egui_id(&self) -> Id {
        // Hashed by *contents*, so the widget keeps its identity across frames
        // even though the string behind the name is a fresh allocation each
        // time. §66.
        Id::new((craftkit_foundation::paths::APP_NAME, self.0.as_str()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AutomationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What was found at a name.
#[derive(Debug, Clone, PartialEq)]
pub enum Widget {
    /// Nothing is published at that name right now.
    Missing,
    /// Something is there.
    Found { rect: Rect, enabled: bool },
}

/// What is at a point.
///
/// The name, and whether it can be clicked. The two go together because they
/// are one fact about the screen: a button that is greyed out is on the
/// screen, and pressing it does nothing. §107.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit<'a> {
    /// The widget's name.
    pub name: &'a str,
    /// Whether the widget can be clicked.
    pub enabled: bool,
}

/// What the shell published this frame.
///
/// Rebuilt every frame and dropped after, so a widget that disappears does not
/// linger in the registry and let a click land on stale coordinates.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Automation {
    entries: Vec<(AutomationId, Rect, bool)>,
    /// The names already published this frame.
    ///
    /// A set, so the duplicate check is a **lookup** and not a scan of
    /// everything published so far.
    ///
    /// The scan made a frame quadratic in the number of widgets: the node list
    /// publishes two names per node, so a document at the model's own limit of
    /// ten thousand nodes publishes twenty thousand names, and the frame spent
    /// **378 ms** in the registry — every frame, for a document the model
    /// accepts. Measured: 3.4 ms for a thousand nodes, 79 ms for five
    /// thousand, 378 ms for ten. §98.
    seen: HashSet<AutomationId>,
    /// The names two widgets wanted in the same frame.
    ///
    /// A name published twice means two widgets want it, and only the first
    /// is published — so the second widget is **not in the registry at all**,
    /// and an agent that asks for it is answered with the first widget's
    /// rectangle, which is in a different panel and does something else.
    ///
    /// The collision is otherwise invisible: the name is present, the lookup
    /// succeeds, and nothing says that the widget behind it is not the one
    /// that was asked for. The level bar wanted `node.<id>.prop.$level` — a
    /// property's name — and a property called `$level` wanted it too, so the
    /// property's editor was dropped without a word. §99.
    collisions: Vec<AutomationId>,
}

impl Automation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a widget at `rect`.
    pub fn publish(&mut self, id: AutomationId, rect: Rect, enabled: bool) {
        // A second publish at the same name in one frame means two widgets
        // share an id; the first wins, because the first is the one a person
        // would call "the" button.
        //
        // The set answers "have I seen this name?" in one lookup. The copy of
        // the name it keeps is the price, and it is the same order as the
        // allocation the caller has already made for it.
        //
        // The loser is **recorded**. It is not in the registry, so nothing
        // else would say that it was ever there. §99.
        if self.seen.insert(id.clone()) {
            self.entries.push((id, rect, enabled));
        } else {
            self.collisions.push(id);
        }
    }

    /// The names two widgets wanted in the same frame.
    ///
    /// Empty is the healthy answer. A name here is a widget that is **not**
    /// in the registry, under a name that another widget answers for.
    pub fn collisions(&self) -> &[AutomationId] {
        &self.collisions
    }

    /// Forget everything published, for the next frame.
    ///
    /// The registry is **one frame's** worth of widgets. The names are built
    /// every frame and the rectangles belong to the frame they were drawn in,
    /// so a registry that carried the last frame's entries would answer
    /// about a screen that is no longer there: a widget that has moved would
    /// keep its old rectangle, and a widget that has gone would keep its
    /// rectangle too, and an agent's click would land on nothing.
    ///
    /// §99: the registry was never cleared, so it grew for as long as the
    /// shell lived, and "the first one published wins" meant "the first
    /// *frame* wins" — every later frame's publications were recorded as
    /// collisions with the first frame's.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.seen.clear();
        self.collisions.clear();
    }

    /// Every name published this frame, sorted.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.entries.iter().map(|(id, _, _)| id.0.as_str()).collect();
        names.sort_unstable();
        names
    }

    /// Look a name up.
    pub fn known(&self, name: &str) -> Widget {
        match self.entries.iter().find(|(id, _, _)| id.0 == name) {
            Some((_, rect, enabled)) => Widget::Found { rect: *rect, enabled: *enabled },
            None => Widget::Missing,
        }
    }

    /// The topmost widget at a point, if any.
    ///
    /// "Topmost" is the last one published at that point, which is the one drawn
    /// last and therefore the one a person would say they clicked. Publish order
    /// is draw order, and draw order is front to back in egui, so this is the
    /// same answer the window gives.
    ///
    /// This is what makes `ui.click` answerable: an agent that knows a button
    /// exists can ask where it is, and an agent given a point can ask what is
    /// there, without either of them guessing.
    ///
    /// The answer carries **whether the widget can be clicked**, because that is
    /// half of what is at a point: a disabled button is on the screen and
    /// cannot be pressed, and an answer that left that out would send an agent
    /// to press something that does not move. §107.
    pub fn at(&self, x: f32, y: f32) -> Option<Hit<'_>> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.entries
            .iter()
            .rfind(|(_, rect, _)| rect.contains(egui::pos2(x, y)))
            .map(|(id, _, enabled)| Hit { name: id.0.as_str(), enabled: *enabled })
    }

    /// The names of the widgets that are on the screen but cannot be clicked.
    ///
    /// Sorted, so the answer is the same every time. §107.
    pub fn disabled_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> =
            self.entries.iter().filter(|(_, _, enabled)| !*enabled).map(|(id, _, _)| id.0.as_str()).collect();
        names.sort_unstable();
        names
    }

    /// The rectangle at `name`, if it is there and enabled.
    pub fn rect(&self, name: &str) -> Option<Rect> {
        match self.known(name) {
            Widget::Found { rect, enabled: true } => Some(rect),
            _ => None,
        }
    }

    /// The rectangle of every published widget, in publish order.
    ///
    /// The names say what the panel published; this says **where**. An agent
    /// that has been given a name and cannot ask for its position can only
    /// guess pixels, and a guess is not a click. §104.
    ///
    /// A widget that is laid out but not painted — a row of a scroll area
    /// that has been scrolled out of view — is published too, with a
    /// rectangle outside the window. §126: the properties panel published
    /// four thousand nine hundred and ninety-one such rows, and this comment
    /// used to say that the names say what is **on screen**.
    pub fn rects(&self) -> Vec<(&str, Rect)> {
        self.entries.iter().map(|(id, rect, _)| (id.0.as_str(), *rect)).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Build the input for a click at `point`, to be fed to the next frame.
///
/// This is the whole of `ui.click`. It does not find widgets by name — that is
/// [`Automation::rect`] plus the caller's arithmetic — because a click is a
/// click: the input goes to whatever is at that point, whatever that is. An
/// agent that wants "the delete button" resolves the name to a point and
/// clicks the point, exactly as a person would.
///
/// It returns a [`egui::RawInput`] rather than poking a live context because
/// egui takes its input once per frame: an event handed to a context between
/// frames can sit unprocessed until something else happens to redraw. Merging
/// into the next frame's input has no such window.
pub fn click(point: egui::Pos2) -> egui::RawInput {
    let button = |pressed: bool| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    let mut raw = egui::RawInput::default();
    // Both halves, in order: a press with no release leaves a drag that never
    // ends, and the next click reads as a double click.
    raw.events.push(button(true));
    raw.events.push(button(false));
    raw
}

/// Merge `extra`'s events into `base`, leaving `base`'s geometry alone.
///
/// The host's real input (window size, time) must not be clobbered by a
/// synthetic click, so only the event list is combined.
pub fn merge(base: &mut egui::RawInput, mut extra: egui::RawInput) {
    base.events.append(&mut extra.events);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toolbar_name_is_derived_from_the_command_id() {
        assert_eq!(AutomationId::toolbar("node.add").as_str(), "toolbar.node.add");
        assert_eq!(AutomationId::toolbar("history.undo").as_str(), "toolbar.history.undo");
    }

    #[test]
    fn names_for_rows_are_stable_and_distinct() {
        let a = AutomationId::node_row(craftkit_core::NodeId(1));
        let b = AutomationId::node_row(craftkit_core::NodeId(2));
        assert_eq!(a.as_str(), "nodes.row.1");
        assert_ne!(a, b);
        // Built twice, identical: no hash, no counter, no closure address.
        assert_eq!(a, AutomationId::node_row(craftkit_core::NodeId(1)));
    }

    #[test]
    fn a_property_and_a_keyframe_at_the_same_path_do_not_collide() {
        let prop = AutomationId::property(craftkit_core::NodeId(3), "amount");
        let key = AutomationId::clear_keys(craftkit_core::NodeId(3), "amount");
        assert_ne!(prop, key);
    }

    /// A level bar is not a property, so it does not share a property's name.
    ///
    /// §99: the level bar published `node.<id>.prop.$level`, which is how a
    /// property *named* `$level` spells its own name. The two wanted the same
    /// name, and the property's editor lost it — it was never published, so
    /// an agent asking for the property's editor was answered with the level
    /// bar's rectangle, in a different panel.
    #[test]
    fn a_level_bar_and_a_property_never_share_a_name() {
        let bar = AutomationId::level_bar(craftkit_core::NodeId(1));
        assert_eq!(bar.as_str(), "node.1.level");
        for path in ["$level", "level", "amount", "$level#2"] {
            assert_ne!(bar, AutomationId::property(craftkit_core::NodeId(1), path));
        }
        assert_ne!(bar, AutomationId::node_row(craftkit_core::NodeId(1)));
    }

    /// The answer at a point says whether the widget can be clicked.
    ///
    /// §107: the answer was the name and nothing else, so a click on a
    /// greyed-out button said "that button is at that point" — and the button
    /// could not be pressed. The same question asked by name was answered
    /// properly, so the two ways of asking disagreed about one widget.
    #[test]
    fn the_answer_at_a_point_says_whether_it_can_be_clicked() {
        let mut a = Automation::new();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        a.publish(AutomationId::toolbar("history.undo"), rect, false);
        let hit = a.at(5.0, 5.0).expect("there is a widget at that point");
        assert_eq!(hit.name, "toolbar.history.undo");
        assert!(!hit.enabled, "the widget is disabled and the answer does not say so");
        assert_eq!(a.disabled_names(), vec!["toolbar.history.undo"]);
    }

    /// Only the widgets that cannot be clicked are in the list of the dead ones.
    #[test]
    fn only_the_widgets_that_cannot_be_clicked_are_in_the_dead_list() {
        let mut a = Automation::new();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        a.publish(AutomationId::toolbar("node.add"), rect, true);
        a.publish(AutomationId::toolbar("history.undo"), rect, false);
        // The topmost widget at that point is the last one published there.
        let hit = a.at(5.0, 5.0).expect("there is a widget at that point");
        assert_eq!(hit.name, "toolbar.history.undo");
        assert!(!hit.enabled, "the topmost widget there is the disabled one");
        assert_eq!(a.disabled_names(), vec!["toolbar.history.undo"]);
    }

    #[test]
    fn an_empty_registry_reports_missing() {
        let a = Automation::new();
        assert!(a.is_empty());
        assert_eq!(a.known("toolbar.node.add"), Widget::Missing);
        assert!(a.rect("toolbar.node.add").is_none());
    }

    #[test]
    fn a_published_widget_is_found_and_can_be_clicked() {
        let mut a = Automation::new();
        let rect = Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(80.0, 24.0));
        a.publish(AutomationId::toolbar("node.add"), rect, true);
        assert_eq!(a.len(), 1);
        assert_eq!(a.known("toolbar.node.add"), Widget::Found { rect, enabled: true });
        assert_eq!(a.rect("toolbar.node.add"), Some(rect));
        assert_eq!(a.names(), vec!["toolbar.node.add"]);
    }

    #[test]
    fn a_disabled_widget_is_found_but_not_clickable() {
        let mut a = Automation::new();
        a.publish(
            AutomationId::toolbar("history.undo"),
            Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
            false,
        );
        assert!(matches!(a.known("toolbar.history.undo"), Widget::Found { enabled: false, .. }));
        assert!(a.rect("toolbar.history.undo").is_none(), "a disabled widget must not be clicked");
    }

    #[test]
    fn publishing_the_same_name_twice_keeps_the_first() {
        let mut a = Automation::new();
        let first = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        let second = Rect::from_min_size(egui::pos2(50.0, 50.0), egui::vec2(10.0, 10.0));
        a.publish(AutomationId::toolbar("node.add"), first, true);
        a.publish(AutomationId::toolbar("node.add"), second, true);
        assert_eq!(a.len(), 1);
        assert_eq!(a.rect("toolbar.node.add"), Some(first));
    }

    /// A name two widgets want is a collision, and it is recorded.
    ///
    /// §99: the loser of a name is not in the registry at all, so a lookup
    /// for it succeeds and answers with the *other* widget — the level bar's
    /// rectangle for a property's editor, in a different panel. Nothing said
    /// so, until the collision was recorded.
    #[test]
    fn a_name_two_widgets_want_is_recorded_as_a_collision() {
        let mut a = Automation::new();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        a.publish(AutomationId::property(craftkit_core::NodeId(1), "$level"), rect, true);
        a.publish(AutomationId::property(craftkit_core::NodeId(1), "$level"), rect, true);
        assert_eq!(a.len(), 1, "the first still wins");
        assert_eq!(a.collisions().len(), 1, "the loser is recorded");
        assert_eq!(a.collisions()[0], AutomationId::property(craftkit_core::NodeId(1), "$level"));
    }

    /// Publishing a widget must not cost more than the widgets before it.
    ///
    /// §98: the duplicate check scanned every name published so far, so a
    /// frame was **quadratic** in the number of widgets. The node list
    /// publishes two names per node, so a document at the model's own limit of
    /// ten thousand nodes publishes twenty thousand names — and the frame
    /// spent 378 ms in the registry, every frame.
    ///
    /// The test is **relative**, so it does not depend on how fast the machine
    /// is: a second batch of the same size must not cost twice as much as the
    /// first. Linear costs the same; quadratic costs four times as much,
    /// because the second batch scans a list twice as long.
    ///
    /// §101: and each side is measured **several times, keeping the fastest
    /// round**. A round can be preempted — by a sibling test binary, by a
    /// build, by whatever else the machine is doing — and a preempted round
    /// says nothing about the code. Measured under load: two rounds of the
    /// *same* code gave ratios of 0.01 and 72, so the test failed about half
    /// the time on a busy machine. The fastest round is the least disturbed
    /// one, and the fastest round of a quadratic implementation is still
    /// three times its first batch, so the mutation is still caught.
    ///
    /// **§233**: the fastest of each side is not enough, because the two
    /// fastest times come from different rounds and the ratio is then a
    /// statement about the scheduler. The number is the **middle** round's
    /// ratio now, measured by the tree's one home for the shape.
    #[test]
    fn publishing_a_widget_does_not_cost_more_than_the_widgets_before_it() {
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        // How many rounds to measure. Nine is odd, and the whole test still
        // takes about twenty milliseconds.
        const ROUNDS: usize = 9;
        let mut round = || {
            let mut registry = Automation::new();
            let fast = craftkit_foundation::measure::seconds(&mut || {
                for i in 0..5_000 {
                    registry.publish(AutomationId(format!("widget.{i}")), rect, true);
                }
            });
            let slow = craftkit_foundation::measure::seconds(&mut || {
                for i in 5_000..10_000 {
                    registry.publish(AutomationId(format!("widget.{i}")), rect, true);
                }
            });
            (fast, slow)
        };
        let ratio = craftkit_foundation::measure::quietest_ratio(ROUNDS, &mut round);
        assert!(
            ratio < 2.0,
            "the second batch of 5000 widgets took {ratio:.1}x as long as the first: \
             the registry is scanning what it has already published"
        );
    }

    #[test]
    fn every_registered_command_has_a_toolbar_name_that_is_distinct() {
        let mut names: Vec<AutomationId> =
            craftkit_engine::all().iter().map(|spec| AutomationId::toolbar(spec.id)).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(before, names.len(), "two commands share a toolbar name");
    }

    #[test]
    fn a_click_is_a_press_followed_by_a_release() {
        // Both halves matter: a press with no release leaves a drag that never
        // ends, and the next click reads as a double click.
        let raw = click(egui::pos2(5.0, 5.0));
        let halves: Vec<(bool, egui::Pos2)> = raw
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::PointerButton { pressed, pos, .. } => Some((*pressed, *pos)),
                _ => None,
            })
            .collect();
        assert_eq!(halves, vec![(true, egui::pos2(5.0, 5.0)), (false, egui::pos2(5.0, 5.0))]);
    }

    // ---------------------------------------------------------------------
    // The leak
    // ---------------------------------------------------------------------

    /// The name's bytes belong to the name.
    ///
    /// The name used to be a `&'static str` made by `Box::leak`, and a leaked
    /// string is one the process can never give back: the names are built every
    /// frame, so the process grew by a kilobyte a frame — 221 MB an hour at
    /// sixty frames a second. §66.
    ///
    /// This is the cheap half of the guarantee, and it is half a guarantee:
    /// it says the name *owns* its bytes, and it stops compiling if the name
    /// ever becomes a borrowed `&'static str` again. It cannot see a name that
    /// leaks a copy of itself and then owns another one — that is what
    /// [`building_a_widget_name_does_not_leak_memory`] is for.
    #[test]
    fn a_widget_name_owns_its_bytes() {
        let id = AutomationId::toolbar("node.add");
        // Moving the field out: if it were a `&'static str`, this line would
        // not compile, which is the whole point of the assertion.
        let owned: String = id.0;
        assert_eq!(owned, "toolbar.node.add");
    }

    /// How many bytes the process grew, according to the kernel.
    ///
    /// Linux only, and the reason is not laziness: `/proc/self/statm` is the
    /// kernel's own count of this process's resident pages, so it includes
    /// memory the allocator has not handed back — which is what a leak *is*.
    /// A counting global allocator would say the same thing on every
    /// platform, but this crate forbids `unsafe`, and `unsafe impl
    /// GlobalAlloc` is unsafe, so one cannot be written here.
    #[cfg(target_os = "linux")]
    fn resident_bytes() -> u64 {
        // `/proc/self/statm`: size resident shared text lib data dt, in pages.
        let Ok(statm) = std::fs::read_to_string("/proc/self/statm") else {
            return 0;
        };
        let pages = statm.split_whitespace().nth(1).and_then(|field| field.parse::<u64>().ok()).unwrap_or(0);
        pages * 4096
    }

    /// The name the harness gives the probe, for `--exact`.
    #[cfg(target_os = "linux")]
    const PROBE_TEST: &str = "automation::tests::the_widget_name_probe_measures_the_growth_it_caused";

    /// What the probe prints, and the parent reads.
    #[cfg(target_os = "linux")]
    const GROWTH_MARKER: &str = "CRAFTKIT_UI_RSS_GROWTH=";

    /// What tells the probe that it is the one being asked to measure.
    #[cfg(target_os = "linux")]
    const CHILD_MARKER: &str = "CRAFTKIT_UI_RSS_CHILD";

    /// How many names the probe builds.
    #[cfg(target_os = "linux")]
    const PROBE_BUILDS: usize = 200_000;

    /// How many bytes the process may grow while it builds the probe's names.
    ///
    /// The names are sixteen to twenty-four bytes each, so a leak of them
    /// costs six megabytes or more — six times this limit — while a process
    /// that frees them grows by almost nothing, because the allocator reuses
    /// the same block every time round. The limit is not a measurement of one
    /// name; it is a floor under "no growth".
    #[cfg(target_os = "linux")]
    const PROBE_LIMIT_BYTES: u64 = 1024 * 1024;

    /// Measure how much the process grew while it built widget names.
    ///
    /// Started as a *child process* by
    /// [`building_a_widget_name_does_not_leak_memory`], because a leak is only
    /// visible against a baseline, and a baseline is only worth anything in a
    /// process that has done nothing else. Run on its own, with no marker in
    /// the environment, it does nothing.
    #[test]
    #[cfg(target_os = "linux")]
    fn the_widget_name_probe_measures_the_growth_it_caused() {
        if std::env::var_os(CHILD_MARKER).is_none() {
            return;
        }
        // Warm up: a process's first allocations pay for the allocator's own
        // bookkeeping, and that is not a leak.
        for _ in 0..10_000 {
            std::hint::black_box(AutomationId::toolbar("node.add"));
        }
        let before = resident_bytes();
        for _ in 0..PROBE_BUILDS {
            std::hint::black_box(AutomationId::toolbar("node.add"));
            std::hint::black_box(AutomationId::node_row(craftkit_core::NodeId(1)));
            std::hint::black_box(AutomationId::property(craftkit_core::NodeId(1), "amount"));
            std::hint::black_box(AutomationId::clear_keys(craftkit_core::NodeId(1), "amount"));
        }
        let after = resident_bytes();
        println!("{GROWTH_MARKER}{}", after.saturating_sub(before));
    }

    /// Building a widget name must not cost the process any memory.
    ///
    /// The names are built every frame — the toolbar's, every node row's,
    /// every property editor's — so a name that leaks its bytes is a process
    /// that grows for as long as the window is open. Measured before the fix:
    /// 1 077 bytes a frame, 221 MB an hour at sixty frames a second. §66.
    #[test]
    #[cfg(target_os = "linux")]
    fn building_a_widget_name_does_not_leak_memory() {
        let exe = std::env::current_exe().expect("the test binary knows its own path");
        let output = std::process::Command::new(&exe)
            .args(["--exact", PROBE_TEST, "--nocapture", "--test-threads=1"])
            .env(CHILD_MARKER, "1")
            .output()
            .expect("the child process starts");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let growth = stdout
            .lines()
            .find_map(|line| line.split_once(GROWTH_MARKER))
            .and_then(|(_, value)| value.trim().parse::<u64>().ok());
        let growth = growth
            .unwrap_or_else(|| panic!("the child did not report a measurement. Its output was:\n{stdout}"));
        assert!(
            growth < PROBE_LIMIT_BYTES,
            "building {PROBE_BUILDS} widget names grew the process by {growth} bytes; the limit is {PROBE_LIMIT_BYTES}"
        );
    }

    #[test]
    fn merging_a_click_keeps_the_hosts_own_geometry() {
        let mut host = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0))),
            ..Default::default()
        };
        host.events.push(egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });

        let mut merged = host.clone();
        merge(&mut merged, click(egui::pos2(1.0, 1.0)));
        assert_eq!(merged.screen_rect, host.screen_rect, "the click must not resize the window");
        assert_eq!(merged.events.len(), 3, "the host's own event survived: {:?}", merged.events.len());
        assert!(matches!(merged.events[0], egui::Event::Key { .. }));
    }
}
