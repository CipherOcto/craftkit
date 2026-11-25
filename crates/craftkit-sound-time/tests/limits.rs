//! The **limits of the time model**: the numbers it will not hold.
//!
//! A tick comes from outside — a command parameter, a file, an agent —
//! so it is checked before it is used, and a limit that no test names is
//! a limit nothing proves holds.

use craftkit_sound_time::{MAX_POSITION, TempoMap};

/// A **tempo** set beyond `MAX_POSITION` is refused.
///
/// The map is anchored on ticks, and a tick past the bound is a number
/// the model does not hold: the refusal names the bound it broke.
#[test]
fn a_tempo_beyond_the_limit_is_refused() {
    let mut map = TempoMap::new();
    let tick = MAX_POSITION + 1;
    let error = map.set_tempo(tick, 120.0).expect_err("that tick is past the bound");
    assert!(error.contains(&MAX_POSITION.to_string()), "the error names the bound: {error}");
}

/// A **meter** set beyond `MAX_POSITION` is refused.
#[test]
fn a_meter_beyond_the_limit_is_refused() {
    let mut map = TempoMap::new();
    let tick = MAX_POSITION + 1;
    let error = map.set_meter(tick, 4, 4).expect_err("that tick is past the bound");
    assert!(error.contains(&MAX_POSITION.to_string()), "the error names the bound: {error}");
}

/// A **tempo** set **at** `MAX_POSITION` is kept.
///
/// This is the other half of the tests above: a cap that refuses
/// everything is not a cap, it is a wall.
#[test]
fn a_tempo_at_the_limit_is_kept() {
    let mut map = TempoMap::new();
    map.set_tempo(MAX_POSITION, 90.0).expect("that tick is within the bound");
    assert_eq!(map.tempo_at_tick(MAX_POSITION), 90.0, "the tempo is in force there");
}

/// A **meter** set **at** `MAX_POSITION` is kept.
#[test]
fn a_meter_at_the_limit_is_kept() {
    let mut map = TempoMap::new();
    map.set_meter(MAX_POSITION, 3, 4).expect("that tick is within the bound");
    assert_eq!(map.meter_at_tick(MAX_POSITION), (3, 4), "the meter is in force there");
}
