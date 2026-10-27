//! The tree's own keyboard.
//!
//! # Why the tree has one
//!
//! **The platform's keyboard opens and its text never arrives.** Measured on
//! a phone, with the tree's own log and the platform's own report: the
//! soft keyboard appears when a field takes the focus, and every key tapped
//! on it leaves the field exactly as it was. The cause is in the
//! dependencies, and it is not the tree's to fix in the tree's own code:
//! `winit`'s Android backend has **no IME events at all** — the call that
//! shows the keyboard only shows and hides it — and the platform serves a
//! `BaseInputConnection` that the tree's backend never talks to.
//!
//! The other way to the same text is **Java**: an `InputConnection` shim in
//! the package, a `dex` beside the library, and a second named exception for
//! the tree's one unsafe rule. The tree's rules say the package carries no
//! Java, so that road is closed too.
//!
//! So the tree draws its own keyboard, in Rust, with no new dependency, and
//! it works on every platform the tree runs on.
//!
//! # What it is
//!
//! A keyboard is a **row of buttons** and a **buffer**. The buttons are
//! drawn by [`draw`], and the buffer is the field's own: a key press does
//! not own any text, it hands the key to the field that has the focus.
//!
//! The keys are the QWERTY order, because that is what a person who has ever
//! typed expects, and a keyboard in another order is a keyboard that has to
//! be learned. There is a second layer for the digits and the signs, because
//! a keyboard that cannot type a number cannot fill in a distance.

use crate::theme::Tokens;

/// A key on the tree's own keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A letter or a digit: the character it types.
    Char(char),
    /// The space bar.
    Space,
    /// The backspace: it takes the last character away.
    Backspace,
    /// The shift: the next letter is a capital.
    Shift,
    /// The key that switches to the digits and the signs.
    Numbers,
    /// The key that switches back to the letters.
    Letters,
    /// The key that puts the keyboard away.
    Done,
}

impl Key {
    /// What the key says on its face.
    ///
    /// The shift is a **state of the keyboard**, not of the key: the letter
    /// says `T` while the shift is down and `t` when it is not.
    pub fn label(&self, shifted: bool) -> String {
        match self {
            Key::Char(c) => {
                if shifted {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                }
            }
            Key::Space => " ".to_string(),
            Key::Backspace => "⌫".to_string(),
            Key::Shift => "⇧".to_string(),
            Key::Numbers => "123".to_string(),
            Key::Letters => "abc".to_string(),
            Key::Done => "done".to_string(),
        }
    }
}

/// Which layer of the keyboard is showing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    /// The letters.
    #[default]
    Letters,
    /// The digits and the signs.
    Numbers,
}

/// The rows of the keyboard, from the top.
///
/// **A function of the layer, and not a table**, so that a test can ask
/// what the keyboard says without a screen, a window or a GPU — the same
/// rule the tree's other pure functions follow.
pub fn rows(layer: Layer) -> Vec<Vec<Key>> {
    match layer {
        Layer::Letters => vec![
            letters("qwertyuiop"),
            letters("asdfghjkl"),
            row(&[Key::Shift], "zxcvbnm", &[Key::Backspace]),
            vec![Key::Numbers, Key::Space, Key::Done],
        ],
        Layer::Numbers => vec![
            letters("1234567890"),
            letters("-=+.,()[]{}"),
            vec![Key::Backspace],
            vec![Key::Letters, Key::Space, Key::Done],
        ],
    }
}

/// One row: what is on it before the letters, and what is on it after.
fn row(before: &[Key], letters_text: &str, after: &[Key]) -> Vec<Key> {
    let mut keys = before.to_vec();
    keys.extend(letters(letters_text));
    keys.extend_from_slice(after);
    keys
}

/// The keys of one row of letters.
fn letters(text: &str) -> Vec<Key> {
    text.chars().map(Key::Char).collect()
}

/// The character a key types, if it types one.
///
/// The shift is the keyboard's state, so the key itself does not know: the
/// caller asks with the shift it has.
pub fn types(key: Key, shifted: bool) -> Option<char> {
    match key {
        Key::Char(c) => {
            if shifted {
                c.to_uppercase().next()
            } else {
                Some(c)
            }
        }
        Key::Space => Some(' '),
        _ => None,
    }
}

/// Draw the keyboard, and say which key was pressed.
///
/// The keyboard is drawn in the theme's colours, like everything else the
/// shell draws: `egui`'s own defaults are a second palette, and a keyboard
/// in a different palette is a keyboard that does not look like the window
/// it is in. §118.
///
/// The **shift** is a state of the keyboard and is passed in and out: the
/// keyboard does not own it, because the shell's state is what survives a
/// frame.
pub fn draw(ui: &mut egui::Ui, tokens: &Tokens, layer: &mut Layer, shifted: &mut bool) -> Option<Key> {
    let mut pressed = None;
    // The width the keyboard has to fit in.
    //
    // **Measured on the phone**: the first version of the keyboard gave every
    // key a fixed width, and ten keys of that width do not fit on a 720-pixel
    // screen — the last key of every row was **off the screen**, and a key
    // nobody can reach is a key the keyboard does not have. §246.
    let available = ui.available_width();
    // The space between two keys is part of the row's width, so it comes off
    // the keys: a row of ten keys and nine gaps is wider than ten keys.
    let spacing = ui.spacing().item_spacing.x;
    for row in rows(*layer) {
        // The row's keys share the row's width, in proportion to what they
        // are: the space bar is twice a letter, and the row still fills the
        // screen.
        let total: f32 = row.iter().map(|key| weight(*key)).sum();
        let unit = if total > 0.0 { available / total } else { available };
        ui.horizontal(|ui| {
            for key in row {
                let label = key.label(*shifted);
                let text = if matches!(key, Key::Char(_) if *shifted) {
                    egui::RichText::new(label).strong().color(tokens.text)
                } else {
                    egui::RichText::new(label).color(tokens.text)
                };
                let width = (unit * weight(key) - spacing).max(12.0);
                let response = ui.add(egui::Button::new(text).min_size(egui::vec2(width, 0.0)));
                if response.clicked() {
                    pressed = Some(key);
                }
            }
        });
    }
    pressed
}

/// How much of a row a key takes.
///
/// The space bar is twice a letter and the "done" key is one and a half:
/// a row of keys that are all the same width has no room for the two keys
/// that are not letters.
fn weight(key: Key) -> f32 {
    match key {
        Key::Space => 2.0,
        Key::Done => 1.5,
        _ => 1.0,
    }
}

/// What a key press types, and what it leaves the shift as.
///
/// The shift types **one** letter, the way every keyboard does: it is a
/// shift, not a caps lock. §246.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressed {
    /// The character the key types, if it types one.
    pub character: Option<char>,
    /// Whether the shift is still on afterwards.
    pub shift: bool,
}

/// What a key press does, in one place.
///
/// The keyboard's own state and the character it types are decided **here**,
/// so that the shell does not have to know that a shift is a shift and not a
/// caps lock.
pub fn press(key: Key, shifted: bool) -> Pressed {
    match key {
        Key::Char(c) => {
            let character = if shifted { c.to_uppercase().next().unwrap_or(c) } else { c };
            Pressed { character: Some(character), shift: false }
        }
        Key::Space => Pressed { character: Some(' '), shift: shifted },
        Key::Backspace | Key::Shift | Key::Numbers | Key::Letters | Key::Done => {
            Pressed { character: None, shift: shifted }
        }
    }
}

/// What a key press does to a buffer.
///
/// The buffer is the field's own, and the tree does not own it: this is the
/// one place that says what a key means, so that the meaning is in one
/// place and not in every field.
pub fn apply(buffer: &mut String, key: Key) {
    match key {
        Key::Backspace => {
            buffer.pop();
        }
        Key::Char(c) => buffer.push(c),
        Key::Space => buffer.push(' '),
        Key::Done | Key::Shift | Key::Numbers | Key::Letters => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keyboard is the QWERTY order.
    ///
    /// A keyboard in another order is a keyboard that has to be learned, and
    /// the order is the one every keyboard in the world has.
    #[test]
    fn the_letters_are_in_the_qwerty_order() {
        let rows = rows(Layer::Letters);
        let first: String = rows[0].iter().filter_map(|k| types(*k, false)).collect();
        assert_eq!(first, "qwertyuiop");
        let second: String = rows[1].iter().filter_map(|k| types(*k, false)).collect();
        assert_eq!(second, "asdfghjkl");
        let third: String = rows[2].iter().filter_map(|k| types(*k, false)).collect();
        assert_eq!(third, "zxcvbnm");
    }

    /// The digits are on the keyboard.
    ///
    /// A keyboard that cannot type a number cannot fill in a distance, and
    /// the tree's commands ask for numbers.
    #[test]
    fn the_digits_are_on_the_keyboard() {
        let rows = rows(Layer::Numbers);
        let digits: String = rows[0].iter().filter_map(|k| types(*k, false)).collect();
        assert_eq!(digits, "1234567890");
    }

    /// The shift capitalises the next letter, and only the letters.
    #[test]
    fn the_shift_capitalises_the_letters() {
        assert_eq!(types(Key::Char('t'), true), Some('T'));
        assert_eq!(types(Key::Char('t'), false), Some('t'));
        assert_eq!(types(Key::Char('7'), true), Some('7'));
    }

    /// A key press changes the buffer the way a key press does.
    #[test]
    fn a_key_press_changes_the_buffer() {
        let mut buffer = String::new();
        apply(&mut buffer, Key::Char('a'));
        apply(&mut buffer, Key::Char('b'));
        assert_eq!(buffer, "ab");
        apply(&mut buffer, Key::Backspace);
        assert_eq!(buffer, "a");
        apply(&mut buffer, Key::Space);
        assert_eq!(buffer, "a ");
        // And the keys that are not characters change nothing.
        apply(&mut buffer, Key::Shift);
        apply(&mut buffer, Key::Done);
        assert_eq!(buffer, "a ");
    }

    /// The shift types one letter, and no more.
    ///
    /// A shift that stays on is a caps lock, and a phone with a caps lock
    /// has no shift at all. §246.
    #[test]
    fn the_shift_types_one_letter_and_no_more() {
        let pressed = press(Key::Char('t'), true);
        assert_eq!(pressed.character, Some('T'));
        assert!(!pressed.shift, "the shift is still on after the letter it types");
        // And without the shift, the letter is a small one and the shift
        // stays as it was.
        let pressed = press(Key::Char('t'), false);
        assert_eq!(pressed.character, Some('t'));
        assert!(!pressed.shift);
        // The keys that are not letters leave the shift alone.
        let pressed = press(Key::Backspace, true);
        assert_eq!(pressed.character, None);
        assert!(pressed.shift, "the backspace turned the shift off");
    }
}
