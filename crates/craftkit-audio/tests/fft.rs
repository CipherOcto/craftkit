//! The **transform**: the spectrum of a frame of sound.
//!
//! The kit's crates carry no dependencies, so the fast Fourier
//! transform is here in its own right — and the tree's onset detector
//! needs one.

use craftkit_audio::{is_power_of_two, power};

/// A **sine** of that frequency, in that many samples.
fn a_sine(frequency: f64, rate: u32, samples: usize) -> Vec<f32> {
    (0..samples)
        .map(|index| (2.0 * std::f64::consts::PI * frequency * index as f64 / rate as f64).sin() as f32)
        .collect()
}

/// The power of a frame is **not** nothing: a frame of sound has
/// sound in it.
#[test]
fn a_frame_of_sound_has_power() {
    let frame = a_sine(1000.0, 48_000, 1024);
    let powers = power(&frame);
    assert_eq!(powers.len(), 1024 / 2 + 1);
    assert!(powers.iter().sum::<f32>() > 0.0, "the frame has power");
}

/// The power of a frame of **silence** is nothing.
#[test]
fn a_frame_of_silence_has_no_power() {
    let powers = power(&vec![0.0_f32; 1024]);
    assert!(powers.iter().all(|power| *power == 0.0), "the frame is silent");
}

/// The power of a **sine** is at the sine's own frequency: the bin
/// that says a thousand hertz is the loudest one, and the bins around
/// it are the ones the sine leaks into.
#[test]
fn a_sine_has_its_power_at_its_own_frequency() {
    let rate = 48_000_u32;
    let size = 1024_usize;
    let frame = a_sine(1000.0, rate, size);
    let powers = power(&frame);
    // A thousand hertz is bin twenty-one of a thousand and twenty-four
    // at forty-eight kilohertz.
    let bin = (1000.0 * size as f64 / rate as f64).round() as usize;
    let loudest = powers
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(bin, _)| bin)
        .expect("there is a loudest bin");
    assert_eq!(loudest, bin, "the loudest bin is the sine's own");
    assert!(powers[bin] > powers[bin - 4], "and it is louder than its neighbour");
}

/// A length that is **not** a power of two is refused: there is no
/// answer and not a wrong one.
#[test]
fn a_length_that_is_not_a_power_of_two_is_refused() {
    assert!(power(&vec![0.0_f32; 1000]).is_empty(), "the power is refused");
    assert!(is_power_of_two(1024));
    assert!(is_power_of_two(1));
    assert!(!is_power_of_two(1000));
    assert!(!is_power_of_two(0));
}
