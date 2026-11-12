//! The **operations** on the notes: moving them, and changing how hard
//! and how long they are.

use craftkit_midi::{
    DurationOp, Note, QuantizeOptions, VelocityOp, change_duration, change_velocity, quantize,
    split_by_pitch, transpose,
};

/// A note of that pitch, starting at that tick, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A quantise onto a grid of that many ticks, moving every note all
/// the way.
fn onto(grid_ticks: i64) -> QuantizeOptions {
    QuantizeOptions { grid_ticks, strength: 1.0, ..QuantizeOptions::default() }
}

/// A note is put on the **nearest line** of the grid.
#[test]
fn a_note_is_put_on_the_nearest_line() {
    let mut notes = vec![note(60, 110, 100)];
    let moved = quantize(&mut notes, &onto(100));
    assert_eq!(moved, 1, "the note moved");
    assert_eq!(notes[0].start, 100, "the nearest line is the one at a hundred");
    assert_eq!(notes[0].length, 100, "and the note keeps its length");
}

/// A note that is **already** on a line stays where it is.
#[test]
fn a_note_on_a_line_stays_where_it_is() {
    let mut notes = vec![note(60, 200, 100)];
    let moved = quantize(&mut notes, &onto(100));
    assert_eq!(moved, 1, "the note was on the grid");
    assert_eq!(notes[0].start, 200, "and it did not move");
}

/// A grid of **no ticks** is nothing to quantise to, and the notes are
/// left alone.
#[test]
fn a_grid_of_no_ticks_leaves_the_notes_alone() {
    let mut notes = vec![note(60, 110, 100)];
    let moved = quantize(&mut notes, &onto(0));
    assert_eq!(moved, 0);
    assert_eq!(notes[0].start, 110);
}

/// The **strength** is how far a note moves towards the line: half is
/// half way.
#[test]
fn the_strength_is_how_far_a_note_moves() {
    let mut notes = vec![note(60, 110, 100)];
    let options = QuantizeOptions { grid_ticks: 100, strength: 0.5, ..QuantizeOptions::default() };
    quantize(&mut notes, &options);
    assert_eq!(notes[0].start, 105, "half way to the line");
}

/// A transpose moves the **pitch**.
#[test]
fn a_transpose_moves_the_pitch() {
    let mut notes = vec![note(60, 0, 100), note(64, 100, 100)];
    transpose(&mut notes, 12).expect("the notes are transposed");
    assert_eq!(notes[0].pitch, 72);
    assert_eq!(notes[1].pitch, 76);
}

/// A transpose that would take a note **out of the range** is refused,
/// and the notes are not changed at all.
#[test]
fn a_transpose_out_of_the_range_is_refused() {
    let mut notes = vec![note(60, 0, 100), note(120, 100, 100)];
    let refused = transpose(&mut notes, 12);
    assert!(refused.is_err(), "the note of a hundred and twenty would leave the range");
    assert_eq!(notes[0].pitch, 60, "and no note was moved");
    assert_eq!(notes[1].pitch, 120);
}

/// The **velocity** is how hard a note is struck, and it is set, or
/// added to, or scaled.
#[test]
fn the_velocity_is_set_added_and_scaled() {
    let mut notes = vec![note(60, 0, 100)];
    change_velocity(&mut notes, &VelocityOp::Set(40)).expect("the velocity is set");
    assert_eq!(notes[0].velocity, 40);
    change_velocity(&mut notes, &VelocityOp::Add(10)).expect("the velocity is changed");
    assert_eq!(notes[0].velocity, 50);
    change_velocity(&mut notes, &VelocityOp::Scale(2.0)).expect("the velocity is scaled");
    assert_eq!(notes[0].velocity, 100);
}

/// A velocity outside the range is **refused**, and the notes are not
/// changed.
#[test]
fn a_velocity_outside_the_range_is_refused() {
    let mut notes = vec![note(60, 0, 100)];
    let refused = change_velocity(&mut notes, &VelocityOp::Set(200));
    assert!(refused.is_err(), "a note cannot be struck at two hundred");
    assert_eq!(notes[0].velocity, 100, "and the note was not changed");
    let refused = change_velocity(&mut notes, &VelocityOp::Set(0));
    assert!(refused.is_err(), "a velocity of zero is not a velocity");
}

/// The **limit** moves a note onto the edge of the limit: a limit says
/// no softer than this and no harder than that, so a note outside it
/// is moved to its edge.
#[test]
fn the_velocity_is_limited() {
    let mut notes = vec![note(60, 0, 100)];
    notes[0].velocity = 20;
    change_velocity(&mut notes, &VelocityOp::Limit { min: 40, max: 100 }).expect("the notes are limited");
    assert_eq!(notes[0].velocity, 40, "the note was softer than the limit");
    let mut notes = vec![note(60, 0, 100)];
    notes[0].velocity = 120;
    change_velocity(&mut notes, &VelocityOp::Limit { min: 40, max: 100 }).expect("the notes are limited");
    assert_eq!(notes[0].velocity, 100, "the note was harder than the limit");
}

/// The bounds of a limit are **swapped** when they are the wrong way
/// round, so a limit of a hundred and ten is a limit of ten and a
/// hundred.
#[test]
fn the_bounds_of_a_limit_are_swapped() {
    let mut notes = vec![note(60, 0, 100)];
    notes[0].velocity = 50;
    change_velocity(&mut notes, &VelocityOp::Limit { min: 100, max: 40 }).expect("the notes are limited");
    assert_eq!(notes[0].velocity, 50, "the note was inside the limit either way");
    notes[0].velocity = 20;
    change_velocity(&mut notes, &VelocityOp::Limit { min: 100, max: 40 }).expect("the notes are limited");
    assert_eq!(notes[0].velocity, 40, "and the note is on the soft edge");
}

/// A limit whose own bounds are **outside the range** is refused: a
/// limit of zero is not a limit of anything.
#[test]
fn a_limit_outside_the_range_is_refused() {
    let mut notes = vec![note(60, 0, 100)];
    let refused = change_velocity(&mut notes, &VelocityOp::Limit { min: 0, max: 100 });
    assert!(refused.is_err(), "a limit of zero is not a limit");
    assert_eq!(notes[0].velocity, 100, "and the note was not changed");
}

/// The **ramp** strikes the notes harder and harder along their span:
/// the earliest note at the first value and the latest at the second.
#[test]
fn the_velocity_is_ramped() {
    let mut notes = vec![note(60, 0, 100), note(62, 500, 100), note(64, 1000, 100)];
    change_velocity(&mut notes, &VelocityOp::Ramp { from: 20, to: 120 }).expect("the notes are ramped");
    assert_eq!(notes[0].velocity, 20, "the earliest note");
    assert_eq!(notes[1].velocity, 70, "the note in the middle");
    assert_eq!(notes[2].velocity, 120, "the latest note");
}

/// A ramp of **one note** strikes it at the first value: there is no
/// span to ramp along.
#[test]
fn a_ramp_of_one_note_strikes_it_at_the_first_value() {
    let mut notes = vec![note(60, 0, 100)];
    change_velocity(&mut notes, &VelocityOp::Ramp { from: 20, to: 120 }).expect("the note is ramped");
    assert_eq!(notes[0].velocity, 20);
}

/// The **length** of the notes is set, added to and scaled, and a note
/// is always at least one tick long.
#[test]
fn the_length_of_the_notes_is_changed() {
    let mut notes = vec![note(60, 0, 100)];
    change_duration(&mut notes, &DurationOp::Set(50));
    assert_eq!(notes[0].length, 50);
    change_duration(&mut notes, &DurationOp::Add(-100));
    assert_eq!(notes[0].length, 1, "a note is at least one tick long");
    change_duration(&mut notes, &DurationOp::Scale(4.0));
    assert_eq!(notes[0].length, 4);
}

/// The **legato** ends every note before the next note starts.
#[test]
fn the_legato_ends_a_note_before_the_next_one() {
    let mut notes = vec![note(60, 0, 500), note(62, 200, 500)];
    change_duration(&mut notes, &DurationOp::Legato { gap: 20 });
    assert_eq!(notes[0].length, 180, "the note ends twenty ticks before the next");
    assert_eq!(notes[1].length, 500, "the last note keeps its length");
}

/// The notes may be **split by their pitch**: the ones in the range and
/// the rest.
#[test]
fn the_notes_are_split_by_their_pitch() {
    let notes = vec![note(60, 0, 100), note(72, 100, 100), note(48, 200, 100)];
    let (inside, outside) = split_by_pitch(&notes, 60, 72);
    assert_eq!(inside.len(), 2, "the two notes in the range");
    assert_eq!(outside.len(), 1, "and the one that is not");
    assert_eq!(outside[0].pitch, 48);
}
