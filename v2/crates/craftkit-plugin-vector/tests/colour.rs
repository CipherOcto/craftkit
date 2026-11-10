//! The **colour** of a drawing: a colour in one **space**, and the
//! same colour in another.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_vector::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the vector battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// The **colour** that a conversion gives.
fn convert(session: &mut Session, colour: &str, from: &str, to: &str) -> Vec<f64> {
    let outcome = session
        .execute("vector.colour.convert", &json!({ "colour": colour, "from": from, "to": to }))
        .expect("the colour is converted");
    outcome.data["colour"]
        .as_array()
        .expect("the colour is there")
        .iter()
        .filter_map(|channel| channel.as_f64())
        .collect()
}

/// Whether the colour is **inside** the gamut of a space.
fn in_gamut(session: &mut Session, colour: &str, from: &str, to: &str) -> bool {
    let outcome = session
        .execute("vector.colour.gamut", &json!({ "colour": colour, "from": from, "to": to }))
        .expect("the gamut is measured");
    outcome.data["in_gamut"].as_bool().expect("the verdict is there")
}

/// **The white of sRGB is the white of every space.**
#[test]
fn the_white_of_srgb_is_the_white_of_every_space() {
    let mut session = session();
    for space in ["display-p3", "linear-srgb", "rec2020", "adobe-rgb-compat"] {
        let there = convert(&mut session, "#ffffff", "srgb", space);
        assert!((there[0] - 1.0).abs() < 0.001, "the white is white in `{space}`: {there:?}");
        assert!((there[1] - 1.0).abs() < 0.001, "the white is white in `{space}`: {there:?}");
        assert!((there[2] - 1.0).abs() < 0.001, "the white is white in `{space}`: {there:?}");
    }
}

/// **The colour of sRGB is the colour of P3, in the numbers of P3.**
#[test]
fn the_colour_of_srgb_is_the_colour_of_p3_in_the_numbers_of_p3() {
    let mut session = session();
    // The orange of sRGB, in the primaries of P3.
    let orange = convert(&mut session, "#ff8000", "srgb", "display-p3");
    assert!((orange[0] - 0.936).abs() < 0.002, "the red is the red of P3: {orange:?}");
    assert!((orange[1] - 0.529).abs() < 0.002, "the green is the green of P3: {orange:?}");
    assert!((orange[2] - 0.199).abs() < 0.002, "and the blue is the blue of P3: {orange:?}");
    // The red of sRGB, in the primaries of P3.
    let red = convert(&mut session, "#ff0000", "srgb", "display-p3");
    assert!((red[0] - 0.917).abs() < 0.002, "the red is the red of P3: {red:?}");
    assert!((red[1] - 0.200).abs() < 0.002, "the green is the green of P3: {red:?}");
    assert!((red[2] - 0.139).abs() < 0.002, "and the blue is the blue of P3: {red:?}");
}

/// **A gray stays a gray.**
#[test]
fn a_gray_stays_a_gray() {
    let mut session = session();
    let there = convert(&mut session, "#808080", "srgb", "display-p3");
    assert!((there[0] - there[1]).abs() < 0.001, "the gray is a gray: {there:?}");
    assert!((there[1] - there[2]).abs() < 0.001, "the gray is a gray: {there:?}");
    assert!((there[0] - 0.502).abs() < 0.01, "and it is the gray it was: {there:?}");
}

/// **The linear space undoes the tone curve.**
#[test]
fn the_linear_space_undoes_the_tone_curve() {
    let mut session = session();
    let red = convert(&mut session, "#ff0000", "srgb", "linear-srgb");
    assert!((red[0] - 1.0).abs() < 0.001, "the red is one: {red:?}");
    assert!(red[1].abs() < 0.001, "and the green is nothing: {red:?}");
    let half = convert(&mut session, "#808080", "srgb", "linear-srgb");
    assert!((half[0] - 0.216).abs() < 0.01, "the half is the curve undone: {half:?}");
}

/// **The gray of a colour is its luminance.**
#[test]
fn the_gray_of_a_colour_is_its_luminance() {
    let mut session = session();
    let white = convert(&mut session, "#ffffff", "srgb", "sgray");
    assert!((white[0] - 1.0).abs() < 0.001, "the white is white: {white:?}");
    let black = convert(&mut session, "#000000", "srgb", "sgray");
    assert!(black[0].abs() < 0.001, "the black is black: {black:?}");
    // The gray of the red is the luminance of the red, measured
    // against the white of the PCS — and the red of sRGB is brighter
    // than a third of the light.
    let red = convert(&mut session, "#ff0000", "srgb", "gray-gamma-2.2");
    assert!((red[0] - 0.505).abs() < 0.01, "the red is as bright as it looks: {red:?}");
    assert!((red[0] - red[1]).abs() < 0.001, "and the gray is a gray: {red:?}");
    let sgray = convert(&mut session, "#ff0000", "srgb", "sgray");
    assert!((sgray[0] - 0.509).abs() < 0.01, "and the sGray of it is the tree's: {sgray:?}");
}

/// **A colour outside the space is refused.**
#[test]
fn a_colour_outside_the_space_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("vector.colour.convert", &json!({ "colour": "#ff0000", "from": "display-p3", "to": "srgb" }))
        .expect_err("the colour is outside sRGB");
    let message = refusal.to_string();
    assert!(message.contains("outside"), "the refusal says so: {message}");
}

/// **The gamut of a space is measured the tree's own way.**
#[test]
fn the_gamut_of_a_space_is_measured() {
    let mut session = session();
    assert!(in_gamut(&mut session, "#ff0000", "srgb", "display-p3"), "the red of sRGB is inside P3");
    assert!(!in_gamut(&mut session, "#ff0000", "display-p3", "srgb"), "the red of P3 is outside sRGB");
    assert!(in_gamut(&mut session, "#808080", "srgb", "display-p3"), "the gray is inside every space");
}

/// **A space that is not one of the tree's own is refused.**
#[test]
fn a_space_that_is_not_one_of_the_trees_own_is_refused() {
    let mut session = session();
    let refusal = session
        .execute("vector.colour.convert", &json!({ "colour": "#ff0000", "from": "srgb", "to": "cmyk" }))
        .expect_err("the space is refused");
    let message = refusal.to_string();
    assert!(message.contains("space"), "the refusal says so: {message}");
}
