//! The **tests of a hypothesis**: a t-test, a chi-square test, an
//! F-test and a z-test.
//!
//! A test asks a question about **two sets of numbers**: are they the
//! same, are they the same as what was expected, do they spread the
//! same way, and is the mean of one of them a certain number.
//!
//! - the **t-test** compares two sets: the same records measured two
//!   ways, or two groups measured one way;
//! - the **chi-square test** compares what was observed with what was
//!   expected;
//! - the **F-test** compares the **spreads** of two sets;
//! - the **z-test** asks whether the mean of a set is a certain
//!   number.
//!
//! # What the port found
//!
//! - **The tree's t-test has three kinds** and the kind says how the
//!   two sets are related: the first pairs them cell by cell, the
//!   second compares two groups of a known size, and the third
//!   compares two groups whose sizes are not known. Here the same, and
//!   the same arithmetic for each.
//! - **The tree's paired t-test refuses two sets of different sizes**
//!   with `#N/A` and here the same: a pair is a pair of places, and
//!   there is no pair without its other half.
//! - **The tree's chi-square test reads the shape of the ranges**:
//!   the degrees of freedom are the product of the rows less one and
//!   the columns less one when the table is a table, and the number
//!   of the cells less one when it is a row. Here the same, and the
//!   shape of the range is read and not guessed.
//! - **The tree's F-test is two-tailed** and its answer is twice the
//!   smaller of the two tails, and never more than one. Here the same.

use crate::formula::Value;

/// Whether a **name** is the name of a test of a hypothesis.
pub(crate) fn is_hypothesis_function(name: &str) -> bool {
    matches!(
        name,
        "T.TEST"
            | "TTEST"
            | "CHISQ.TEST"
            | "CHITEST"
            | "F.TEST"
            | "FTEST"
            | "Z.TEST"
            | "ZTEST"
            | "CONFIDENCE"
            | "CONFIDENCE.NORM"
            | "CONFIDENCE.T"
    )
}

/// The **numbers** of a set of values; an error is the error.
fn numbers_of(values: &[Value]) -> Result<Vec<f64>, String> {
    let mut numbers = Vec::new();
    for value in values {
        match value {
            Value::Number(number) => numbers.push(*number),
            Value::Error(message) => return Err(message.clone()),
            _ => {}
        }
    }
    Ok(numbers)
}

/// The **mean** of a set of numbers.
fn mean(numbers: &[f64]) -> Result<f64, String> {
    if numbers.is_empty() {
        return Err("#DIV/0!".to_string());
    }
    Ok(numbers.iter().sum::<f64>() / numbers.len() as f64)
}

/// The **variance** of a set of numbers, the sample's.
fn variance_of(numbers: &[f64]) -> Result<f64, String> {
    if numbers.len() < 2 {
        return Err("#DIV/0!".to_string());
    }
    let mean = mean(numbers)?;
    let squares: f64 = numbers.iter().map(|number| (number - mean) * (number - mean)).sum();
    Ok(squares / (numbers.len() - 1) as f64)
}

/// The **t-test** of two sets.
pub(crate) fn t_test(left: &[Value], right: &[Value], tails: f64, kind: f64) -> Result<f64, String> {
    let tails = tails.trunc();
    let kind = kind.trunc();
    if !(tails == 1.0 || tails == 2.0) || !(1.0..=3.0).contains(&kind) {
        return Err("#NUM!".to_string());
    }
    let (t, degrees) = if kind == 1.0 {
        let (first, second) = crate::statistics::pairs_of(left, right)?;
        let differences: Vec<f64> = first.iter().zip(&second).map(|(one, other)| one - other).collect();
        let count = differences.len() as f64;
        let deviation = variance_of(&differences)?.sqrt();
        if deviation == 0.0 {
            return Err("#DIV/0!".to_string());
        }
        ((mean(&differences)? / (deviation / count.sqrt())), count - 1.0)
    } else {
        let first = numbers_of(left)?;
        let second = numbers_of(right)?;
        let (count_one, count_two) = (first.len() as f64, second.len() as f64);
        let (variance_one, variance_two) = (variance_of(&first)?, variance_of(&second)?);
        let difference = mean(&first)? - mean(&second)?;
        if kind == 2.0 {
            let degrees = count_one + count_two - 2.0;
            let pooled = ((count_one - 1.0) * variance_one + (count_two - 1.0) * variance_two) / degrees;
            let error = (pooled * (1.0 / count_one + 1.0 / count_two)).sqrt();
            if error == 0.0 {
                return Err("#DIV/0!".to_string());
            }
            (difference / error, degrees)
        } else {
            let (one, two) = (variance_one / count_one, variance_two / count_two);
            let error = (one + two).sqrt();
            if error == 0.0 {
                return Err("#DIV/0!".to_string());
            }
            let degrees =
                (one + two).powi(2) / (one * one / (count_one - 1.0) + two * two / (count_two - 1.0));
            (difference / error, degrees)
        }
    };
    Ok(tails * crate::distributions::t_upper(t.abs(), degrees))
}

/// The **chi-square test** of what was observed and what was expected.
///
/// The shape of the table says how many degrees of freedom the test
/// has: a table of rows and columns has one for every row but one and
/// every column but one, and a row has one for every cell but one.
pub(crate) fn chisq_test(
    observed: &[Value],
    expected: &[Value],
    rows: usize,
    columns: usize,
) -> Result<f64, String> {
    if observed.len() != expected.len() {
        return Err("#N/A".to_string());
    }
    let mut total = 0.0;
    for (one, other) in observed.iter().zip(expected) {
        match (one, other) {
            (Value::Error(message), _) | (_, Value::Error(message)) => return Err(message.clone()),
            (Value::Number(one), Value::Number(other)) => {
                if *other == 0.0 {
                    return Err("#DIV/0!".to_string());
                }
                total += (one - other).powi(2) / other;
            }
            _ => {}
        }
    }
    let degrees = if rows > 1 && columns > 1 {
        ((rows - 1) * (columns - 1)) as f64
    } else {
        observed.len().saturating_sub(1) as f64
    };
    if degrees < 1.0 {
        return Err("#N/A".to_string());
    }
    Ok(crate::special::reg_gamma_q(degrees / 2.0, total / 2.0))
}

/// The **F-test** of two sets: do they spread the same way?
pub(crate) fn f_test(left: &[Value], right: &[Value]) -> Result<f64, String> {
    let first = numbers_of(left)?;
    let second = numbers_of(right)?;
    let (variance_one, variance_two) = (variance_of(&first)?, variance_of(&second)?);
    if variance_one == 0.0 || variance_two == 0.0 {
        return Err("#DIV/0!".to_string());
    }
    let ratio = variance_one / variance_two;
    let (first, second) = ((first.len() - 1) as f64, (second.len() - 1) as f64);
    let cumulative = crate::distributions::f_cdf(ratio, first, second);
    Ok((2.0 * cumulative.min(1.0 - cumulative)).min(1.0))
}

/// The **z-test** of a set and a number.
pub(crate) fn z_test(values: &[Value], number: f64, deviation: Option<f64>) -> Result<f64, String> {
    let numbers = numbers_of(values)?;
    if numbers.is_empty() {
        return Err("#N/A".to_string());
    }
    let (deviation, given) = match deviation {
        Some(deviation) => (deviation, true),
        None => (variance_of(&numbers)?.sqrt(), false),
    };
    if deviation <= 0.0 {
        return Err(if given { "#NUM!" } else { "#DIV/0!" }.to_string());
    }
    let z = (mean(&numbers)? - number) / (deviation / (numbers.len() as f64).sqrt());
    Ok(1.0 - crate::special::norm_cdf(z))
}

/// The **confidence** of a mean: how wide the mean's uncertainty is.
///
/// The confidence of the normal distribution is the normal quantile
/// of the tail, and the confidence of Student's t is the t quantile
/// of the tail with the degrees of freedom of the size less one.
pub(crate) fn confidence_norm(alpha: f64, deviation: f64, size: f64) -> Result<f64, String> {
    if !(alpha > 0.0 && alpha < 1.0) || deviation <= 0.0 || size < 1.0 {
        return Err("#NUM!".to_string());
    }
    Ok(crate::special::norm_inv(1.0 - alpha / 2.0) * deviation / size.sqrt())
}

/// The **confidence** of a mean, with Student's t.
pub(crate) fn confidence_t(alpha: f64, deviation: f64, size: f64) -> Result<f64, String> {
    if !(alpha > 0.0 && alpha < 1.0) || deviation <= 0.0 || size < 1.0 {
        return Err("#NUM!".to_string());
    }
    if size == 1.0 {
        return Err("#DIV/0!".to_string());
    }
    Ok(crate::distributions::t_inv_upper(alpha / 2.0, size - 1.0) * deviation / size.sqrt())
}
