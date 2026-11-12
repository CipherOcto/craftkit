//! The **transform**: the spectrum of a frame of sound.
//!
//! A frame of sound is a list of numbers, and the **spectrum** is the
//! same frame told in another way: how much of it is at each
//! frequency. The transform that does it is a fast Fourier transform,
//! and it is here because the tree's onset detector needs one and this
//! kit's crates carry **no dependencies at all**.
//!
//! # How it works
//!
//! The transform is the classic one: the samples are put in a bit-
//! reversed order and then folded together in stages, each stage
//! halving the distance between the samples that are added up. The
//! work is `n log n` and the arithmetic is the same at every stage.
//!
//! The numbers are **f64**: a transform of a thousand numbers is not
//! a lot of arithmetic, and the extra digits are worth more than the
//! half of the time they cost.
//!
//! # What it refuses
//!
//! A transform is only defined for a number of samples that is a
//! **power of two**, and a length that is not one is **refused**:
//! there is no answer and not a wrong one.

use std::f64::consts::PI;

/// A complex number, in two parts.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Complex {
    /// The real part.
    re: f64,
    /// The imaginary part.
    im: f64,
}

impl Complex {
    /// The sum of two numbers.
    fn add(self, other: Complex) -> Complex {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }

    /// The difference of two numbers.
    fn sub(self, other: Complex) -> Complex {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }

    /// The product of two numbers.
    fn mul(self, other: Complex) -> Complex {
        Complex { re: self.re * other.re - self.im * other.im, im: self.re * other.im + self.im * other.re }
    }
}

/// Whether a number is a **power of two**.
///
/// One is a power of two: it is two to the power of nothing.
pub fn is_power_of_two(n: usize) -> bool {
    n != 0 && n & (n - 1) == 0
}

/// The **transform** of a frame: its spectrum, in complex numbers.
///
/// The length must be a **power of two**, and a length that is not one
/// gives an empty spectrum: the transform is refused and not fudged.
fn transform(samples: &[f64]) -> Vec<Complex> {
    let n = samples.len();
    if !is_power_of_two(n) {
        return Vec::new();
    }
    let mut values: Vec<Complex> = samples.iter().map(|sample| Complex { re: *sample, im: 0.0 }).collect();
    if n < 2 {
        return values;
    }
    // The **bit-reversed order**: the samples are put where the folding
    // expects them, so that the distance between the two that are added
    // up halves at every stage.
    let mut j = 0_usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            values.swap(i, j);
        }
    }
    // The **folds**: each stage pairs the samples that are half a stage
    // apart and adds them up two ways.
    let mut width = 2_usize;
    while width <= n {
        let angle = -2.0 * PI / width as f64;
        let step = Complex { re: angle.cos(), im: angle.sin() };
        let half = width / 2;
        let mut start = 0_usize;
        while start < n {
            let mut rotation = Complex { re: 1.0, im: 0.0 };
            for k in 0..half {
                let even = values[start + k];
                let odd = values[start + k + half].mul(rotation);
                values[start + k] = even.add(odd);
                values[start + k + half] = even.sub(odd);
                rotation = rotation.mul(step);
            }
            start += width;
        }
        width *= 2;
    }
    values
}

/// The **power** of each frequency of a frame: how much of the frame
/// is at that frequency.
///
/// The power is the square of the size of the complex number, and it is
/// the **raw** size: a frame of a thousand samples has a power a
/// thousand times that of a frame of one, and the caller scales it to
/// the size it wants.
///
/// The length must be a **power of two**, and a length that is not one
/// gives no power at all: the transform is refused and not fudged.
pub fn power(samples: &[f32]) -> Vec<f32> {
    let n = samples.len();
    if !is_power_of_two(n) {
        return Vec::new();
    }
    let real: Vec<f64> = samples.iter().map(|sample| f64::from(*sample)).collect();
    let spectrum = transform(&real);
    spectrum.iter().take(n / 2 + 1).map(|value| (value.re * value.re + value.im * value.im) as f32).collect()
}
