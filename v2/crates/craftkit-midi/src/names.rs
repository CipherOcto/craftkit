//! The **names** of the notes: what a pitch is called.
//!
//! A MIDI note is a number, and a number is not something a person
//! reads. The tree's MIDI crate carries a table of names — the note
//! names, the drum names and the program names — so that a note can be
//! called "C3" and a drum can be called "Bass Drum 1" and a program
//! can be called "Acoustic Grand Piano".
//!
//! The **note names** follow the tree's own convention: the middle C —
//! the note of pitch sixty — is **C3**, so that the octave of a note
//! is the one a piano names it by. A note of pitch zero is **C-2**,
//! because the octave below the first is the second one down.

/// The twelve pitch classes, in the order they go up.
const PITCH_CLASSES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// The name of a **pitch**.
///
/// The middle C — the pitch of sixty — is "C3", so that the octave of
/// a note is the one a piano names it by, and the pitch of zero is
/// "C-2": the octave below the first is the second one down.
pub fn note_name(pitch: u8) -> String {
    let class = PITCH_CLASSES.get(usize::from(pitch % 12)).copied().unwrap_or("?");
    let octave = i32::from(pitch / 12) - 2;
    format!("{class}{octave}")
}

/// The General MIDI **drum** names, in the order they go up.
///
/// The drums of the General MIDI kit are on the pitches from thirty-five
/// to eighty-one, and this is the table for them.
const GM_DRUMS: [&str; 47] = [
    "Acoustic Bass Drum",
    "Bass Drum 1",
    "Side Stick",
    "Acoustic Snare",
    "Hand Clap",
    "Electric Snare",
    "Low Floor Tom",
    "Closed Hi-Hat",
    "High Floor Tom",
    "Pedal Hi-Hat",
    "Low Tom",
    "Open Hi-Hat",
    "Low-Mid Tom",
    "Hi-Mid Tom",
    "Crash Cymbal 1",
    "High Tom",
    "Ride Cymbal 1",
    "Chinese Cymbal",
    "Ride Bell",
    "Tambourine",
    "Splash Cymbal",
    "Cowbell",
    "Crash Cymbal 2",
    "Vibraslap",
    "Ride Cymbal 2",
    "Hi Bongo",
    "Low Bongo",
    "Mute Hi Conga",
    "Open Hi Conga",
    "Low Conga",
    "High Timbale",
    "Low Timbale",
    "High Agogo",
    "Low Agogo",
    "Cabasa",
    "Maracas",
    "Short Whistle",
    "Long Whistle",
    "Short Guiro",
    "Long Guiro",
    "Claves",
    "Hi Wood Block",
    "Low Wood Block",
    "Mute Cuica",
    "Open Cuica",
    "Mute Triangle",
    "Open Triangle",
];

/// The name of a **drum**, for a pitch on the drum kit.
///
/// The drums are on the pitches from thirty-five to eighty-one, and a
/// pitch outside them has no drum name: **none**, and not a made-up
/// one.
pub fn gm_drum_name(pitch: u8) -> Option<&'static str> {
    let index = usize::from(pitch).checked_sub(35)?;
    GM_DRUMS.get(index).copied()
}

/// The General MIDI **program** names, in the order they go up.
const GM_PROGRAMS: [&str; 128] = [
    "Acoustic Grand Piano",
    "Bright Acoustic Piano",
    "Electric Grand Piano",
    "Honky-tonk Piano",
    "Electric Piano 1",
    "Electric Piano 2",
    "Harpsichord",
    "Clavi",
    "Celesta",
    "Glockenspiel",
    "Music Box",
    "Vibraphone",
    "Marimba",
    "Xylophone",
    "Tubular Bells",
    "Dulcimer",
    "Drawbar Organ",
    "Percussive Organ",
    "Rock Organ",
    "Church Organ",
    "Reed Organ",
    "Accordion",
    "Harmonica",
    "Tango Accordion",
    "Acoustic Guitar (nylon)",
    "Acoustic Guitar (steel)",
    "Electric Guitar (jazz)",
    "Electric Guitar (clean)",
    "Electric Guitar (muted)",
    "Overdriven Guitar",
    "Distortion Guitar",
    "Guitar Harmonics",
    "Acoustic Bass",
    "Electric Bass (finger)",
    "Electric Bass (pick)",
    "Fretless Bass",
    "Slap Bass 1",
    "Slap Bass 2",
    "Synth Bass 1",
    "Synth Bass 2",
    "Violin",
    "Viola",
    "Cello",
    "Contrabass",
    "Tremolo Strings",
    "Pizzicato Strings",
    "Orchestral Harp",
    "Timpani",
    "String Ensemble 1",
    "String Ensemble 2",
    "SynthStrings 1",
    "SynthStrings 2",
    "Choir Aahs",
    "Voice Oohs",
    "Synth Voice",
    "Orchestra Hit",
    "Trumpet",
    "Trombone",
    "Tuba",
    "Muted Trumpet",
    "French Horn",
    "Brass Section",
    "SynthBrass 1",
    "SynthBrass 2",
    "Soprano Sax",
    "Alto Sax",
    "Tenor Sax",
    "Baritone Sax",
    "Oboe",
    "English Horn",
    "Bassoon",
    "Clarinet",
    "Piccolo",
    "Flute",
    "Recorder",
    "Pan Flute",
    "Blown Bottle",
    "Shakuhachi",
    "Whistle",
    "Ocarina",
    "Lead 1 (square)",
    "Lead 2 (sawtooth)",
    "Lead 3 (calliope)",
    "Lead 4 (chiff)",
    "Lead 5 (charang)",
    "Lead 6 (voice)",
    "Lead 7 (fifths)",
    "Lead 8 (bass + lead)",
    "Pad 1 (new age)",
    "Pad 2 (warm)",
    "Pad 3 (polysynth)",
    "Pad 4 (choir)",
    "Pad 5 (bowed)",
    "Pad 6 (metallic)",
    "Pad 7 (halo)",
    "Pad 8 (sweep)",
    "FX 1 (rain)",
    "FX 2 (soundtrack)",
    "FX 3 (crystal)",
    "FX 4 (atmosphere)",
    "FX 5 (brightness)",
    "FX 6 (goblins)",
    "FX 7 (echoes)",
    "FX 8 (sci-fi)",
    "Sitar",
    "Banjo",
    "Shamisen",
    "Koto",
    "Kalimba",
    "Bag pipe",
    "Fiddle",
    "Shanai",
    "Tinkle Bell",
    "Agogo",
    "Steel Drums",
    "Woodblock",
    "Taiko Drum",
    "Melodic Tom",
    "Synth Drum",
    "Reverse Cymbal",
    "Guitar Fret Noise",
    "Breath Noise",
    "Seashore",
    "Bird Tweet",
    "Telephone Ring",
    "Helicopter",
    "Applause",
    "Gunshot",
];

/// The name of a **program**, for a program number.
///
/// A program outside the hundred and twenty-eight of the General MIDI
/// set has no name: "Unknown", and not a made-up one.
pub fn gm_program_name(program: u8) -> &'static str {
    GM_PROGRAMS.get(usize::from(program)).copied().unwrap_or("Unknown")
}

/// The name of a **note**, with the drum it is when it is on the drum
/// kit.
///
/// The drums of the General MIDI kit are on the channel ten, and a
/// note on that channel is a drum: the note name is the drum's name.
pub fn note_or_drum_name(pitch: u8, channel: u8) -> String {
    if channel == 9 {
        if let Some(name) = gm_drum_name(pitch) {
            return name.to_string();
        }
    }
    note_name(pitch)
}
