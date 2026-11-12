//! The tests of the kit's own plugins.
//!
//! A plugin is a program over a buffer of numbers, so the tests are the
//! numbers: a gain is a gain, a low pass is smooth, a delay is later,
//! and a name that is not in the catalogue is a refusal with a name.

use craftkit_plugins::{Error, NAMES, Params, Plugin, make};
use std::collections::HashMap;

/// The parameters, from a list of pairs.
fn params(pairs: &[(&str, f64)]) -> Params {
    let mut values: HashMap<String, f64> = HashMap::new();
    for (name, value) in pairs {
        values.insert(name.to_string(), *value);
    }
    Params::from_map(values)
}

/// A plugin of that name, with those parameters.
fn a_plugin(name: &str, pairs: &[(&str, f64)]) -> Box<dyn Plugin> {
    make(name, &params(pairs)).expect("the plugin is made")
}

#[test]
fn the_catalogue_has_the_kits_own_plugins() {
    assert!(NAMES.contains(&"gain"), "the catalogue has a gain");
    assert!(NAMES.contains(&"lowpass"), "the catalogue has a low pass");
    assert!(NAMES.contains(&"delay"), "the catalogue has a delay");
    for name in NAMES {
        let plugin = make(name, &Params::default()).expect("the plugin is made");
        assert_eq!(plugin.name(), *name, "the plugin says what it is");
    }
}

/// A name that is not in the catalogue is refused, **with its name**.
#[test]
fn a_name_that_is_not_in_the_catalogue_is_refused() {
    let error = make("reverb", &Params::default()).expect_err("there is no reverb");
    assert_eq!(error, Error::Unknown { name: "reverb".to_string() });
    assert!(error.to_string().contains("reverb"), "{error}");
}

/// A **gain** is a gain: six decibels down is about half.
#[test]
fn a_gain_is_a_gain() {
    let mut plugin = a_plugin("gain", &[("gain_db", -6.0)]);
    let mut samples = vec![1.0_f32, 1.0];
    plugin.process(&mut samples, 1, 48_000);
    assert!((samples[0] - 0.5012).abs() < 0.001, "the signal is quieter: {}", samples[0]);
}

/// A gain with no parameter is a gain of **nothing**: the signal is as
/// it is.
#[test]
fn a_gain_with_no_parameter_changes_nothing() {
    let mut plugin = a_plugin("gain", &[]);
    let mut samples = vec![0.5_f32];
    plugin.process(&mut samples, 1, 48_000);
    assert_eq!(samples[0], 0.5);
}

/// A **low pass** smooths the signal: a step becomes a ramp.
#[test]
fn a_low_pass_smooths_the_signal() {
    let mut plugin = a_plugin("lowpass", &[("cutoff_hz", 200.0)]);
    let mut samples = vec![1.0_f32; 16];
    plugin.process(&mut samples, 1, 48_000);
    assert!(samples[0] < 1.0, "the first sample is not the whole step");
    assert!(samples[0] < samples[15], "and the signal rises: {} and {}", samples[0], samples[15]);
}

/// A **delay** is later: the signal comes back after so many samples.
#[test]
fn a_delay_is_later() {
    let mut plugin = a_plugin("delay", &[("delay_ms", 1.0)]);
    let mut samples = vec![1.0_f32; 96];
    plugin.process(&mut samples, 1, 48_000);
    assert_eq!(samples[47], 0.0, "the delay is silent before it");
    assert_eq!(samples[48], 1.0, "and the signal is back after it");
}

/// A parameter that is not a size is refused: a cutoff of nothing is
/// not a cutoff.
#[test]
fn a_parameter_that_is_not_a_size_is_refused() {
    let error = make("lowpass", &params(&[("cutoff_hz", 0.0)])).expect_err("that is not a cutoff");
    assert_eq!(error, Error::BadParameter { name: "cutoff_hz".to_string(), value: 0.0 });
    let error = make("delay", &params(&[("delay_ms", -5.0)])).expect_err("that is not a delay");
    assert_eq!(error, Error::BadParameter { name: "delay_ms".to_string(), value: -5.0 });
}

/// A parameter that is **not a number** is left out: the plugin is made
/// with what it has.
#[test]
fn a_parameter_that_is_not_a_number_is_left_out() {
    let mut values: HashMap<String, f64> = HashMap::new();
    values.insert("gain_db".to_string(), f64::NAN);
    let params = Params::from_map(values);
    let mut plugin = make("gain", &params).expect("the plugin is made");
    let mut samples = vec![0.5_f32];
    plugin.process(&mut samples, 1, 48_000);
    assert_eq!(samples[0], 0.5, "the gain is the one that is given");
}
