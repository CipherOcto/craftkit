//! The AU host, against a **real** unit.
//!
//! The unit under test is `craftkit-au-fixture`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds the six symbols the format
//! names with `dlsym`, and calls through them.

use std::path::PathBuf;

use craftkit_au::AuPlugin;

/// The shared library the tests open.
///
/// It is a **dev dependency** of the host, so cargo builds it before
/// the tests run, and it is a `cdylib`, so what is built is a shared
/// library and not a Rust library.
///
/// The path is derived from the test binary's own path: cargo puts the
/// binary in `target/<profile>/deps` and the shared library next to it
/// in `target/<profile>`.
fn library_path() -> PathBuf {
    let binary = std::env::current_exe().expect("the test binary knows where it is");
    let directory =
        binary.parent().and_then(|parent| parent.parent()).expect("the target directory is there");
    directory.join("libcraftkit_au_fixture.so")
}

/// Open the unit.
fn open() -> AuPlugin {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    AuPlugin::open(&path).unwrap_or_else(|error| panic!("the unit opens: {error}"))
}

/// Open the unit, make an instance of it, and hand it to the caller.
///
/// The instance borrows the bundle, so the bundle is opened here and
/// closed when this returns.
fn with_instance(body: impl FnOnce(&craftkit_au::AuInstance<'_>)) {
    let plugin = open();
    let made = plugin.instance().expect("the instance is made");
    body(&made);
}

/// The host opens a bundle and finds the symbols the format names.
#[test]
fn the_host_opens_a_bundle_and_finds_the_symbols() {
    let plugin = open();
    assert_eq!(plugin.path(), library_path().to_string_lossy(), "the bundle is where it was");
}

/// An **instance** is made, through the tree's own door.
#[test]
fn an_instance_is_made() {
    with_instance(|made| made.initialize().expect("the unit initialises"));
}

/// The unit says the **rate** it works at, and takes a new one.
#[test]
fn the_unit_says_its_rate() {
    with_instance(|made| {
        let rate = made.sample_rate().expect("the rate is read");
        assert_eq!(rate, 48_000.0, "the unit starts at forty-eight thousand");
        made.set_sample_rate(96_000.0).expect("the rate is set");
        let rate = made.sample_rate().expect("the rate is read");
        assert_eq!(rate, 96_000.0, "and the rate took");
    });
}

/// A rate that is not a rate is **refused**.
#[test]
fn a_rate_that_is_not_a_rate_is_refused() {
    with_instance(|made| {
        assert!(made.set_sample_rate(0.0).is_err(), "zero is not a rate");
        assert!(made.set_sample_rate(-1.0).is_err(), "and neither is a negative one");
    });
}

/// The unit says the **shape** of the audio it takes.
#[test]
fn the_unit_says_the_shape_of_the_audio() {
    with_instance(|made| {
        let format = made.stream_format().expect("the shape is read");
        assert_eq!(format.sample_rate, 48_000.0, "the rate is the unit's");
        assert_eq!(format.channels_per_frame, 1, "and there is one channel");
        assert_eq!(format.bits_per_channel, 32, "of thirty-two bits");
    });
}

/// The unit says how many frames it takes in **one** slice.
#[test]
fn the_unit_says_its_slice() {
    with_instance(|made| {
        let frames = made.maximum_frames().expect("the slice is read");
        assert_eq!(frames, 1024, "a thousand and twenty-four frames");
    });
}

/// The **audio** goes through the unit.
#[test]
fn the_audio_goes_through_the_unit() {
    with_instance(|made| {
        made.initialize().expect("the unit initialises");
        let input = [0.25f32, 0.5, -0.25, 1.0];
        let mut output = [0.0f32; 4];
        made.render(&input, &mut output).expect("the block is rendered");
        assert!((output[0] - 0.25).abs() < 0.0001, "the first sample is what went in");
        assert!((output[3] - 1.0).abs() < 0.0001, "and the last");
    });
}

/// A unit that is not set up does not **render**.
#[test]
fn a_unit_that_is_not_set_up_does_not_render() {
    with_instance(|made| {
        let input = [0.25f32; 4];
        let mut output = [0.0f32; 4];
        assert!(made.render(&input, &mut output).is_err(), "the unit refused the block");
    });
}

/// A bundle that is not a unit is **refused**.
#[test]
fn a_bundle_that_is_not_a_unit_is_refused() {
    let path = std::env::temp_dir().join("craftkit-v2-au-not-a-unit.so");
    std::fs::write(&path, b"this is not a shared library").expect("the file is written");
    let path = path.to_string_lossy().to_string();
    assert!(AuPlugin::open(&path).is_err(), "the bundle did not open");
}
