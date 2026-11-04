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

/// The **gamma function**.
pub(crate) fn gamma(x: f64) -> f64 {
    if !x.is_finite() || (x <= 0.0 && x == x.floor()) {
        return f64::NAN;
    }
    if x == x.floor() && x <= 171.0 {
        let mut product = 1.0f64;
        let mut index = 2.0;
        while index < x {
            product *= index;
            index += 1.0;
        }
        return product;
    }
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    if x > 171.7 {
        return f64::INFINITY;
    }
    ln_gamma(x).exp()
}

/// The **logarithm of the beta function**.
pub(crate) fn ln_beta(a: f64, b: f64) -> f64 {
    ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
}

/// The regularised lower incomplete gamma.
pub(crate) fn reg_gamma_p(a: f64, x: f64) -> f64 {
    if a.is_nan() || a <= 0.0 || x.is_nan() || x < 0.0 || !a.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x.is_infinite() {
        return 1.0;
    }
    if x < a + 1.0 { gamma_series(a, x) } else { 1.0 - gamma_cf(a, x) }
}

/// The regularised upper incomplete gamma.
pub(crate) fn reg_gamma_q(a: f64, x: f64) -> f64 {
    if a.is_nan() || a <= 0.0 || x.is_nan() || x < 0.0 || !a.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 1.0;
    }
    if x.is_infinite() {
        return 0.0;
    }
    if x < a + 1.0 { 1.0 - gamma_series(a, x) } else { gamma_cf(a, x) }
}

/// The **series** of the incomplete gamma.
fn gamma_series(a: f64, x: f64) -> f64 {
    let mut term_a = a;
    let mut sum = 1.0 / a;
    let mut term = sum;
    for _ in 0..10_000 {
        term_a += 1.0;
        term *= x / term_a;
        sum += term;
        if term.abs() < sum.abs() * 1e-17 {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// The **continued fraction** of the incomplete gamma.
fn gamma_cf(a: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / TINY;
    let mut d = 1.0 / b;
    let mut h = d;
    for index in 1..10_000 {
        let index = index as f64;
        let numerator = -(index * (index - a));
        b += 2.0;
        d = numerator * d + b;
        if d.abs() < TINY {
            d = TINY;
        }
        c = b + numerator / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let change = d * c;
        h *= change;
        if (change - 1.0).abs() < 1e-17 {
            break;
        }
    }
    (-x + a * x.ln() - ln_gamma(a)).exp() * h
}

/// The **error function**.
pub(crate) fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x.abs() < 0.5 {
        // The series of the error function.
        let mut sum = x;
        let mut term = x;
        let square = x * x;
        for index in 1..60 {
            term *= -square / index as f64;
            let add = term / (2 * index + 1) as f64;
            sum += add;
            if add.abs() < 1e-18 * sum.abs() {
                break;
            }
        }
        return sum * 2.0 / PI.sqrt();
    }
    let probability = reg_gamma_p(0.5, x * x);
    if x < 0.0 { -probability } else { probability }
}

/// The **complementary error function**, accurate in the tails.
pub(crate) fn erfc(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x.abs() < 0.5 {
        return 1.0 - erf(x);
    }
    if x > 0.0 { reg_gamma_q(0.5, x * x) } else { 1.0 + reg_gamma_p(0.5, x * x) }
}

/// The regularised incomplete beta.
pub(crate) fn reg_inc_beta(x: f64, a: f64, b: f64) -> f64 {
    if a.is_nan() || a <= 0.0 || b.is_nan() || b <= 0.0 || !(0.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    let log = -ln_beta(a, b) + a * x.ln() + b * (1.0 - x).ln();
    let factor = log.exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        factor * beta_cf(a, b, x) / a
    } else {
        1.0 - factor * beta_cf(b, a, 1.0 - x) / b
    }
}

/// The **continued fraction** of the incomplete beta.
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let sum = a + b;
    let a_plus = a + 1.0;
    let a_minus = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - sum * x / a_plus;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for index in 1..10_000 {
        let index = index as f64;
        let twice = 2.0 * index;
        let first = index * (b - index) * x / ((a_minus + twice) * (a + twice));
        d = 1.0 + first * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + first / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let second = -(a + index) * (sum + index) * x / ((a + twice) * (a_plus + twice));
        d = 1.0 + second * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + second / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let change = d * c;
        h *= change;
        if (change - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

/// The **density** of the standard normal.
pub(crate) fn norm_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * PI).sqrt()
}

/// The **cumulative** of the standard normal.
pub(crate) fn norm_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

/// The **inverse** of the standard normal cumulative: a number outside
/// the open interval of zero and one has no answer.
pub(crate) fn norm_inv(p: f64) -> f64 {
    if !(p > 0.0 && p < 1.0) {
        return f64::NAN;
    }
    // Acklam's rational approximation, and then one Halley step.
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] =
        [7.784_695_709_041_462e-3, 3.224_671_290_700_398e-1, 2.445_134_137_142_996, 3.754_408_661_907_416];
    const LOW: f64 = 0.02425;
    let mut x = if p < LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - LOW {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    for _ in 0..2 {
        let error = norm_cdf(x) - p;
        let u = error * (2.0 * PI).sqrt() * (x * x / 2.0).exp();
        let next = x - u / (1.0 + x * u / 2.0);
        if !next.is_finite() {
            break;
        }
        x = next;
    }
    x
}

/// The **root** of a monotone function, by bisection.
pub(crate) fn invert(function: impl Fn(f64) -> f64, target: f64, low: f64, high: f64, highest: f64) -> f64 {
    let (mut low, mut high) = (low, high);
    let mut low_value = function(low) - target;
    let mut high_value = function(high) - target;
    let mut guard = 0;
    while low_value * high_value > 0.0 && high < highest && guard < 200 {
        low = high;
        high = (high * 2.0).min(highest);
        high_value = function(high) - target;
        guard += 1;
    }
    low_value = function(low) - target;
    if low_value == 0.0 {
        return low;
    }
    if high_value == 0.0 {
        return high;
    }
    if low_value * high_value > 0.0 || low_value.is_nan() || high_value.is_nan() {
        return f64::NAN;
    }
    let low_is_negative = low_value < 0.0;
    for _ in 0..300 {
        let middle = 0.5 * (low + high);
        if middle <= low || middle >= high {
            break;
        }
        let middle_value = function(middle) - target;
        if middle_value == 0.0 {
            return middle;
        }
        if (middle_value < 0.0) == low_is_negative {
            low = middle;
        } else {
            high = middle;
        }
        if (high - low).abs() <= 1e-15 * middle.abs().max(1e-300) {
            break;
        }
    }
    0.5 * (low + high)
}
