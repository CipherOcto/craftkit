//! The **special functions** of the numbers.
//!
//! The tree's own numeric helpers, written from the standard textbook
//! formulations: the **Lanczos** approximation for the gamma function,
//! and the series for the error function.

use std::f64::consts::PI;

/// The Lanczos constant.
const LANCZOS_G: f64 = 7.0;

/// The Lanczos coefficients.
const LANCZOS: [f64; 9] = [
    0.999_999_999_999_809_9,
    676.520_368_121_885_1,
    -1_259.139_216_722_402_8,
    771.323_428_777_653_1,
    -176.615_029_162_140_6,
    12.507_343_278_686_905,
    -0.138_571_095_265_720_12,
    9.984_369_578_019_572e-6,
    1.505_632_735_149_311_6e-7,
];

/// The **logarithm of the absolute value of the gamma function**.
///
/// A number that is not a number has no answer, and a negative whole
/// number has none either.
pub(crate) fn ln_gamma(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x <= 0.0 && x == x.floor() {
        return f64::NAN;
    }
    if x < 0.5 {
        // The reflection formula.
        let sine = (PI * x).sin().abs();
        return (PI / sine).ln() - ln_gamma(1.0 - x);
    }
    if x < 15.0 && x == x.floor() {
        // Exact for the small whole numbers.
        let mut product = 1.0f64;
        let mut index = 2.0;
        while index < x {
            product *= index;
            index += 1.0;
        }
        return product.ln();
    }
    let x = x - 1.0;
    let mut sum = LANCZOS[0];
    let t = x + LANCZOS_G + 0.5;
    for (index, coefficient) in LANCZOS.iter().enumerate().skip(1) {
        sum += coefficient / (x + index as f64);
    }
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + sum.ln()
}
