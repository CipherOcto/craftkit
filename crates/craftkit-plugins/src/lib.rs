//! `craftkit-plugins` — **the kit's own audio plugins**: a catalogue of
//! small programs that change a signal.
//!
//! # What this is
//!
//! A **plugin** is a program. It takes a buffer of samples and changes
//! them, and that is the whole of the contract.
//!
//! This crate is a **catalogue**: a name and a set of parameters make a
//! program, and the program is run over a buffer. The catalogue is
//! small — a gain, a low pass and a delay — and it is the tree's own:
//! every line here is written for this tree, and there is no artwork
//! and no asset of any kind.
//!
//! # Why it is a library and not a plugin
//!
//! Because the word means two things in this kit, and they are two
//! different things. A **plugin** of the *registry* is a crate that
//! contributes kinds and commands; a **plugin** of *this* crate is a
//! program that changes a signal. The registry's plugins are
//! contributed at build time and the linker wires them in (§ the
//! linker); the audio plugins are named by a document and made at
//! run time, and the catalogue is where the names are.
//!
//! # The parameters are hostile
//!
//! A parameter comes from a document, and a document can say anything.
//! So a parameter that is not a number is **left out** and the plugin
//! is made with what it has, and a parameter that is not a size — a
//! gain that is negative, a cutoff that is above the top of the ear —
//! is **refused** and the plugin is not made at all.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::collections::HashMap;

/// A **plugin**: a program that changes a signal.
///
/// A plugin can be **printed**: a caller that has a plugin and does not
/// know what it is says so, and a plugin that cannot be printed cannot
/// say what it is.
pub trait Plugin: std::fmt::Debug {
    /// Its **name**: what an insert calls it.
    fn name(&self) -> &str;
    /// What it does to a buffer of samples.
    ///
    /// The samples are **interleaved**, the channels are how many there
    /// are and the rate is how fast they are played. A plugin that
    /// changes one channel changes the one it changes, and the samples
    /// of the other channels are left alone.
    fn process(&mut self, samples: &mut [f32], channels: usize, rate: u32);
}

/// The **parameters** a plugin is made with.
///
/// The parameters are a map of names to numbers, and the numbers came
/// from a document: a name the plugin does not know is **left out**,
/// and a number that is not a number is **left out** too.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Params {
    values: HashMap<String, f64>,
}

impl Params {
    /// The parameters, from a map of numbers.
    pub fn from_map(values: HashMap<String, f64>) -> Params {
        Params { values }
    }

    /// The value of a parameter, if it has one.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied().filter(|value| value.is_finite())
    }

    /// The value of a parameter, or the one that is given.
    pub fn get_or(&self, name: &str, fallback: f64) -> f64 {
        self.get(name).unwrap_or(fallback)
    }
}

/// Why a plugin was **not** made.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// No plugin of that name is in the catalogue.
    Unknown {
        /// The name that was asked for.
        name: String,
    },
    /// A parameter of the plugin is not a size.
    BadParameter {
        /// The name of the parameter.
        name: String,
        /// The number it said.
        value: f64,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Unknown { name } => write!(f, "there is no plugin called `{name}`"),
            Error::BadParameter { name, value } => {
                write!(f, "the parameter `{name}` of {value} is not a size")
            }
        }
    }
}

impl std::error::Error for Error {}

/// The names of the plugins in the catalogue.
pub const NAMES: &[&str] = &["gain", "lowpass", "delay"];

/// Make a plugin by name.
///
/// A name that is not in the catalogue is a refusal with a name, and a
/// parameter that is not a size is a refusal with the parameter's name.
pub fn make(name: &str, params: &Params) -> Result<Box<dyn Plugin>, Error> {
    match name {
        "gain" => Ok(Box::new(Gain::new(params.get_or("gain_db", 0.0)))),
        "lowpass" => {
            let cutoff = params.get_or("cutoff_hz", 1_000.0);
            if !(20.0..=20_000.0).contains(&cutoff) {
                return Err(Error::BadParameter { name: "cutoff_hz".to_string(), value: cutoff });
            }
            Ok(Box::new(LowPass::new(cutoff)))
        }
        "delay" => {
            let milliseconds = params.get_or("delay_ms", 100.0);
            if !(0.0..=5_000.0).contains(&milliseconds) {
                return Err(Error::BadParameter { name: "delay_ms".to_string(), value: milliseconds });
            }
            Ok(Box::new(Delay::new(milliseconds)))
        }
        other => Err(Error::Unknown { name: other.to_string() }),
    }
}

/// A **gain**: the level of the signal, in decibels.
///
/// The gain is in decibels because that is how a clip's gain is
/// (§ the clip's payload) and the two are the same kind of number.
#[derive(Debug, Clone, PartialEq)]
pub struct Gain {
    /// The level, linear: one is as it is.
    factor: f32,
}

impl Gain {
    /// A gain of so many decibels.
    pub fn new(gain_db: f64) -> Gain {
        let factor = if gain_db.is_finite() { 10.0_f64.powf(gain_db / 20.0) } else { 1.0 };
        Gain { factor: factor as f32 }
    }
}

impl Plugin for Gain {
    fn name(&self) -> &str {
        "gain"
    }

    fn process(&mut self, samples: &mut [f32], _channels: usize, _rate: u32) {
        for sample in samples.iter_mut() {
            *sample *= self.factor;
        }
    }
}

/// A **low pass**: one pole, and a cutoff in hertz.
///
/// The filter is the simplest there is that changes a signal: a
/// one-pole filter, which is a running average of the signal and the
/// last value of the filter. It is not a good filter and it is honest
/// about what it is.
#[derive(Debug, Clone, PartialEq)]
pub struct LowPass {
    /// How much of the last value is kept: between nothing and one.
    coefficient: f32,
    /// The last value of the filter.
    last: f32,
}

impl LowPass {
    /// A low pass with a cutoff of so many hertz.
    pub fn new(cutoff_hz: f64) -> LowPass {
        LowPass { coefficient: 0.0, last: 0.0 }.with_cutoff(cutoff_hz)
    }

    /// The same filter with another cutoff.
    fn with_cutoff(mut self, cutoff_hz: f64) -> LowPass {
        let cutoff = cutoff_hz.clamp(20.0, 20_000.0);
        // The time constant of a one-pole filter, and the fraction of
        // the last value that is kept.
        let seconds = 1.0 / (2.0 * std::f64::consts::PI * cutoff);
        self.coefficient = (seconds / (seconds + 1.0 / 48_000.0)) as f32;
        self
    }
}

impl Plugin for LowPass {
    fn name(&self) -> &str {
        "lowpass"
    }

    fn process(&mut self, samples: &mut [f32], _channels: usize, _rate: u32) {
        for sample in samples.iter_mut() {
            self.last += (1.0 - self.coefficient) * (*sample - self.last);
            *sample = self.last;
        }
    }
}

/// A **delay**: the signal, later.
///
/// The delay is in **milliseconds**, and it is a whole number of
/// samples, because a delay that is not a whole number of samples is a
/// delay with a fraction of a sample in it and the kit does not do
/// that.
#[derive(Debug, Clone, PartialEq)]
pub struct Delay {
    /// The delay, in milliseconds.
    milliseconds: f64,
    /// The line of the signal.
    line: Vec<f32>,
    /// Where the line is written.
    at: usize,
}

impl Delay {
    /// A delay of so many milliseconds.
    pub fn new(milliseconds: f64) -> Delay {
        Delay { milliseconds, line: Vec::new(), at: 0 }
    }
}

impl Plugin for Delay {
    fn name(&self) -> &str {
        "delay"
    }

    fn process(&mut self, samples: &mut [f32], _channels: usize, rate: u32) {
        let wanted = ((self.milliseconds / 1_000.0) * rate as f64).round() as usize;
        let wanted = wanted.max(1);
        if self.line.len() != wanted {
            self.line = vec![0.0; wanted];
            self.at = 0;
        }
        for sample in samples.iter_mut() {
            let out = self.line[self.at];
            self.line[self.at] = *sample;
            self.at = (self.at + 1) % wanted;
            *sample = out;
        }
    }
}
