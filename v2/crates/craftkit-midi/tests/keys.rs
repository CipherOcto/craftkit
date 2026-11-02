//! The **names** of the keys: what a key signature is called.
//!
//! A key signature in a MIDI file is a number of sharps and a mode, and
//! a person reads it as a name — "G major", "E flat minor".

use craftkit_midi::names::{key_from_name, key_name};

/// A key is named by the **sharps** it carries and its **mode**.
#[test]
fn a_key_is_named_by_its_sharps_and_its_mode() {
    assert_eq!(key_name(0, false), Some("C major".to_string()));
    assert_eq!(key_name(1, false), Some("G major".to_string()));
    assert_eq!(key_name(-1, false), Some("F major".to_string()));
    assert_eq!(key_name(-7, false), Some("Cb major".to_string()));
    assert_eq!(key_name(0, true), Some("A minor".to_string()));
    assert_eq!(key_name(3, true), Some("F# minor".to_string()));
    assert_eq!(key_name(-3, true), Some("C minor".to_string()));
}

/// A number of sharps **outside** the fifteen a signature may carry
/// has no name, and none is made up for it.
#[test]
fn a_signature_outside_the_table_has_no_name() {
    assert_eq!(key_name(8, false), None);
    assert_eq!(key_name(-8, false), None);
    assert_eq!(key_name(8, true), None);
}

/// A name is read back into the key it stands for.
#[test]
fn a_name_says_which_key_it_is() {
    assert_eq!(key_from_name("C major"), Some((0, false)));
    assert_eq!(key_from_name("G major"), Some((1, false)));
    assert_eq!(key_from_name("Bb minor"), Some((-5, true)));
    assert_eq!(key_from_name("E minor"), Some((1, true)));
}

/// The name is read the way a person writes it: the case does not
/// matter, and the mode may be shortened.
#[test]
fn a_name_is_read_the_way_a_person_writes_it() {
    assert_eq!(key_from_name("g major"), Some((1, false)));
    assert_eq!(key_from_name("  D MAJOR  "), Some((2, false)));
    assert_eq!(key_from_name("F# maj"), Some((6, false)));
    assert_eq!(key_from_name("C# min"), Some((4, true)));
}

/// A name that is not a key of the table has **no** key, and a name
/// with no mode has none either: "C" does not say whether the music is
/// in C major or in C minor.
#[test]
fn a_name_that_is_not_a_key_is_refused() {
    assert_eq!(key_from_name("H major"), None);
    assert_eq!(key_from_name("C sharp"), None);
    assert_eq!(key_from_name("C"), None);
    assert_eq!(key_from_name(""), None);
}
