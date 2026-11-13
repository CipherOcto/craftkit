//! The AU host, against a **real** unit.
//!
//! The unit under test is `craftkit-au-fixture`: a real `cdylib`,
//! built by cargo, exported by name. Nothing here is a mock: the host
//! opens a file on disk with `dlopen`, finds the six symbols the format
//! names with `dlsym`, and calls through them.
//!
//! The door only exists where a shared library can be opened. Every test
//! that needs a **real** library is therefore unix-only, and the last
//! test in this file is the one that runs everywhere else: it says what
//! the tree does on a target with no shared libraries.

use std::path::PathBuf;

use craftkit_au::AuPlugin;

/// The name of the shared library cargo writes for a fixture.
///
/// Cargo names a shared library differently on each platform, and the tree's
/// tests open a **real** one, so the name has to be right on all three.
fn fixture_name(crate_name: &str) -> String {
    let stem = crate_name.replace('-', "_");
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}

/// The name is the one cargo writes **here**.
///
/// The three platforms disagree about the name of a shared library, and
/// this test cannot run on the other two, so it checks the shape it can
/// see: the one this host is running on.
#[test]
fn the_fixture_name_is_the_one_cargo_writes_here() {
    let name = fixture_name("craftkit-au-fixture");
    if cfg!(target_os = "windows") {
        assert_eq!(name, "craftkit_au_fixture.dll", "windows has no lib prefix");
    } else if cfg!(target_os = "macos") {
        assert_eq!(name, "libcraftkit_au_fixture.dylib", "macos writes a dylib");
    } else {
        assert_eq!(name, "libcraftkit_au_fixture.so", "the other unix writes a so");
    }
}

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
    directory.join(fixture_name("craftkit-au-fixture"))
}

/// Open the unit.
#[cfg(unix)]
fn open() -> AuPlugin {
    let path = library_path();
    let path = path.to_string_lossy().to_string();
    AuPlugin::open(&path).unwrap_or_else(|error| panic!("the unit opens: {error}"))
}

/// Open the unit, make an instance of it, and hand it to the caller.
///
/// The instance borrows the bundle, so the bundle is opened here and
/// closed when this returns.
#[cfg(unix)]
fn with_instance(body: impl FnOnce(&craftkit_au::AuInstance<'_>)) {
    let plugin = open();
    let made = plugin.instance().expect("the instance is made");
    body(&made);
}

/// The host opens a bundle and finds the symbols the format names.
#[cfg(unix)]
#[test]
fn the_host_opens_a_bundle_and_finds_the_symbols() {
    let plugin = open();
    assert_eq!(plugin.path(), library_path().to_string_lossy(), "the bundle is where it was");
}

/// An **instance** is made, through the tree's own door.
#[cfg(unix)]
#[test]
fn an_instance_is_made() {
    with_instance(|made| made.initialize().expect("the unit initialises"));
}

/// The unit says the **rate** it works at, and takes a new one.
#[cfg(unix)]
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
#[cfg(unix)]
#[test]
fn a_rate_that_is_not_a_rate_is_refused() {
    with_instance(|made| {
        assert!(made.set_sample_rate(0.0).is_err(), "zero is not a rate");
        assert!(made.set_sample_rate(-1.0).is_err(), "and neither is a negative one");
    });
}

/// The unit says the **shape** of the audio it takes.
#[cfg(unix)]
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
#[cfg(unix)]
#[test]
fn the_unit_says_its_slice() {
    with_instance(|made| {
        let frames = made.maximum_frames().expect("the slice is read");
        assert_eq!(frames, 1024, "a thousand and twenty-four frames");
    });
}

/// The **audio** goes through the unit.
#[cfg(unix)]
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
#[cfg(unix)]
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
    let path =
        craftkit_scratch::file(&fixture_name("craftkit-v2-au-not-a-unit")).expect("the scratch is made");
    std::fs::write(&path, b"this is not a shared library").expect("the file is written");
    let path = path.to_string_lossy().to_string();
    assert!(AuPlugin::open(&path).is_err(), "the bundle did not open");
}

/// Where there is no shared library, the door **refuses**, and says why.
///
/// This test runs on every target that is not unix, so the door is not
/// untested there: it is tested for what it does, which is refuse.
#[cfg(not(unix))]
#[test]
fn a_shared_library_cannot_be_opened_on_this_target() {
    let path = library_path().to_string_lossy().to_string();
    let error = match AuPlugin::open(&path) {
        Ok(_) => panic!("there is no door on this target"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("a shared library cannot be opened on this target"), "{error}");
}
