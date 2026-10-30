//! `craftkit-numeric` — the **typed value**: a length and an angle.
//!
//! # Why this crate exists
//!
//! Every number in a document is a JSON number, and the core has no unit:
//! it does not know whether `10` is ten millimetres, ten seconds or ten
//! degrees. §39 asked what a typed value would look like when it arrived,
//! and the answer was that the **arithmetic** belongs to a library and
//! the **thing it operates on** belongs to a plugin.
//!
//! So this crate is the arithmetic, and it is a **library**: it
//! contributes nothing to the registry, the linker does not wire it in,
//! and a plugin that wants it depends on it the way it depends on the
//! SDK.
//!
//! # The two types
//!
//! - a **[`Length`]** is in **millimetres**, because the tree's unit is
//!   the millimetre and Z is up;
//! - an **[`Angle`]** is in **radians**, because the tree's other unit is
//!   the radian.
//!
//! Both are **newtypes over `f64`**, so a millimetre can never be passed
//! where a radian is wanted: the mistake is a compile error, and not a
//! number that is wrong by a factor of 57.
//!
//! # The input is hostile
//!
//! A length that comes from outside — a command parameter, a file, a
//! network — is not a length until it has been **checked**. The
//! constructors refuse what cannot be a length: a number that is not
//! finite, and a number outside the range the model will hold.
//!
//! # What is deliberately not here
//!
//! There is no plugin, no command, no kind and no panel. The types are
//! here; what they are *for* is a plugin's business.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// The furthest length the model will hold: a billion millimetres,
/// which is a thousand kilometres.
pub const MAX_MILLIMETRES: f64 = 1_000_000_000.0;

/// Why a number was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum NumericError {
    /// The number was not finite: it was infinite, or it was not a
    /// number at all.
    NotFinite {
        /// What was being measured.
        what: String,
    },
    /// The number was finite and outside the range the model holds.
    OutOfRange {
        /// What was being measured.
        what: String,
        /// The number that was refused.
        value: f64,
        /// The limit it broke.
        limit: f64,
    },
    /// The unit was not one this model knows.
    UnknownUnit {
        /// The unit that was named.
        unit: String,
    },
}

impl std::fmt::Display for NumericError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumericError::NotFinite { what } => write!(f, "{what} is not a finite number"),
            NumericError::OutOfRange { what, value, limit } => {
                write!(f, "{what} is {value}, and the model holds at most {limit}")
            }
            NumericError::UnknownUnit { unit } => write!(f, "`{unit}` is not a unit this model knows"),
        }
    }
}

impl std::error::Error for NumericError {}

/// A **length**, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Length {
    millimetres: f64,
}

impl Length {
    /// A length of nothing.
    pub const fn zero() -> Length {
        Length { millimetres: 0.0 }
    }

    /// A length in **millimetres**.
    ///
    /// The number is checked: it has to be finite and inside the range
    /// the model holds.
    pub fn millimetres(value: f64) -> Result<Length, NumericError> {
        if !value.is_finite() {
            return Err(NumericError::NotFinite { what: "a length".to_string() });
        }
        if value.abs() > MAX_MILLIMETRES {
            return Err(NumericError::OutOfRange {
                what: "a length".to_string(),
                value,
                limit: MAX_MILLIMETRES,
            });
        }
        Ok(Length { millimetres: value })
    }

    /// A length in **metres**.
    pub fn metres(value: f64) -> Result<Length, NumericError> {
        Length::millimetres(value * 1000.0)
    }

    /// A length in **centimetres**.
    pub fn centimetres(value: f64) -> Result<Length, NumericError> {
        Length::millimetres(value * 10.0)
    }

    /// A length in **inches**.
    pub fn inches(value: f64) -> Result<Length, NumericError> {
        Length::millimetres(value * 25.4)
    }

    /// A length in **feet**.
    pub fn feet(value: f64) -> Result<Length, NumericError> {
        Length::inches(value * 12.0)
    }

    /// A length in **points**: a sixtieth of an inch, which is what a
    /// page is measured in.
    pub fn points(value: f64) -> Result<Length, NumericError> {
        Length::inches(value / 72.0)
    }

    /// The length in millimetres.
    pub fn as_millimetres(self) -> f64 {
        self.millimetres
    }

    /// The length in metres.
    pub fn as_metres(self) -> f64 {
        self.millimetres / 1000.0
    }

    /// The length in centimetres.
    pub fn as_centimetres(self) -> f64 {
        self.millimetres / 10.0
    }

    /// The length in inches.
    pub fn as_inches(self) -> f64 {
        self.millimetres / 25.4
    }

    /// The length in feet.
    pub fn as_feet(self) -> f64 {
        self.as_inches() / 12.0
    }

    /// The length in points.
    pub fn as_points(self) -> f64 {
        self.as_inches() * 72.0
    }

    /// The length of nothing.
    pub fn is_zero(self) -> bool {
        self.millimetres == 0.0
    }

    /// The length, as a number, for a payload.
    pub fn to_number(self) -> f64 {
        self.millimetres
    }

    /// A length read back from a payload.
    pub fn from_number(value: f64) -> Result<Length, NumericError> {
        Length::millimetres(value)
    }

    /// The sum of two lengths.
    ///
    /// It is **checked**: a sum that leaves the range the model holds is
    /// refused rather than wrapped.
    pub fn checked_add(self, other: Length) -> Result<Length, NumericError> {
        Length::millimetres(self.millimetres + other.millimetres)
    }

    /// The difference of two lengths.
    ///
    /// It is **checked**, like the sum.
    pub fn checked_sub(self, other: Length) -> Result<Length, NumericError> {
        Length::millimetres(self.millimetres - other.millimetres)
    }

    /// The length, times a number.
    pub fn scale(self, factor: f64) -> Result<Length, NumericError> {
        Length::millimetres(self.millimetres * factor)
    }

    /// The length, the other way up.
    pub fn abs(self) -> Length {
        Length { millimetres: self.millimetres.abs() }
    }

    /// The shorter of two lengths.
    pub fn min(self, other: Length) -> Length {
        if self.millimetres <= other.millimetres { self } else { other }
    }

    /// The longer of two lengths.
    pub fn max(self, other: Length) -> Length {
        if self.millimetres >= other.millimetres { self } else { other }
    }
}

/// An **angle**, in radians.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Angle {
    radians: f64,
}

impl Angle {
    /// An angle of nothing.
    pub const fn zero() -> Angle {
        Angle { radians: 0.0 }
    }

    /// A full turn, in radians.
    pub const fn turn() -> Angle {
        Angle { radians: std::f64::consts::TAU }
    }

    /// An angle in **radians**.
    pub fn radians(value: f64) -> Result<Angle, NumericError> {
        if !value.is_finite() {
            return Err(NumericError::NotFinite { what: "an angle".to_string() });
        }
        Ok(Angle { radians: value })
    }

    /// An angle in **degrees**.
    pub fn degrees(value: f64) -> Result<Angle, NumericError> {
        Angle::radians(value * std::f64::consts::PI / 180.0)
    }

    /// The angle in radians.
    pub fn as_radians(self) -> f64 {
        self.radians
    }

    /// The angle in degrees.
    pub fn as_degrees(self) -> f64 {
        self.radians * 180.0 / std::f64::consts::PI
    }

    /// The angle, as a number, for a payload.
    pub fn to_number(self) -> f64 {
        self.radians
    }

    /// An angle read back from a payload.
    pub fn from_number(value: f64) -> Result<Angle, NumericError> {
        Angle::radians(value)
    }

    /// The angle turned into the range a turn covers: from nothing up to
    /// a full turn.
    pub fn normalized(self) -> Angle {
        let turn = std::f64::consts::TAU;
        let mut radians = self.radians % turn;
        if radians < 0.0 {
            radians += turn;
        }
        Angle { radians }
    }

    /// The sum of two angles.
    pub fn checked_add(self, other: Angle) -> Result<Angle, NumericError> {
        Angle::radians(self.radians + other.radians)
    }

    /// The difference of two angles.
    pub fn checked_sub(self, other: Angle) -> Result<Angle, NumericError> {
        Angle::radians(self.radians - other.radians)
    }
}

/// Read a length in the unit that is named.
///
/// The units are the ones the family uses: millimetres, centimetres,
/// metres, inches, feet and points.
pub fn length_in(unit: &str, value: f64) -> Result<Length, NumericError> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "mm" | "millimetre" | "millimetres" | "millimeter" | "millimeters" => Length::millimetres(value),
        "cm" | "centimetre" | "centimetres" | "centimeter" | "centimeters" => Length::centimetres(value),
        "m" | "metre" | "metres" | "meter" | "meters" => Length::metres(value),
        "in" | "inch" | "inches" => Length::inches(value),
        "ft" | "foot" | "feet" => Length::feet(value),
        "pt" | "point" | "points" => Length::points(value),
        other => Err(NumericError::UnknownUnit { unit: other.to_string() }),
    }
}

/// Read an angle in the unit that is named.
///
/// The units are radians and degrees.
pub fn angle_in(unit: &str, value: f64) -> Result<Angle, NumericError> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "rad" | "radian" | "radians" => Angle::radians(value),
        "deg" | "degree" | "degrees" => Angle::degrees(value),
        other => Err(NumericError::UnknownUnit { unit: other.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_is_in_millimetres() {
        let length = Length::millimetres(10.0).expect("the length is a length");
        assert_eq!(length.as_millimetres(), 10.0);
        assert_eq!(length.as_metres(), 0.01);
        assert_eq!(length.as_centimetres(), 1.0);
    }

    #[test]
    fn a_length_converts_to_inches_and_points() {
        let length = Length::inches(1.0).expect("the length is a length");
        assert!((length.as_millimetres() - 25.4).abs() < 1e-9, "an inch is 25.4 millimetres");
        assert!((length.as_points() - 72.0).abs() < 1e-9, "and seventy-two points");
    }

    #[test]
    fn a_length_that_is_not_a_number_is_refused() {
        assert!(matches!(Length::millimetres(f64::NAN), Err(NumericError::NotFinite { .. })));
        assert!(matches!(Length::millimetres(f64::INFINITY), Err(NumericError::NotFinite { .. })));
    }

    #[test]
    fn a_length_that_is_too_long_is_refused() {
        let error = Length::millimetres(MAX_MILLIMETRES * 2.0).expect_err("that is too long");
        assert!(matches!(error, NumericError::OutOfRange { .. }));
    }

    #[test]
    fn lengths_add_and_subtract() {
        let one = Length::millimetres(1.0).expect("a length");
        let two = Length::millimetres(2.0).expect("a length");
        assert_eq!(one.checked_add(two).expect("the sum").as_millimetres(), 3.0);
        assert_eq!(two.checked_sub(one).expect("the difference").as_millimetres(), 1.0);
    }

    #[test]
    fn a_sum_that_is_too_long_is_refused() {
        let big = Length::millimetres(MAX_MILLIMETRES).expect("a length");
        let error = big.checked_add(big).expect_err("that is too long");
        assert!(matches!(error, NumericError::OutOfRange { .. }));
    }

    #[test]
    fn a_length_scales() {
        let length = Length::millimetres(2.5).expect("a length");
        assert_eq!(length.scale(4.0).expect("the scaled length").as_millimetres(), 10.0);
    }

    #[test]
    fn the_shorter_and_the_longer() {
        let one = Length::millimetres(1.0).expect("a length");
        let two = Length::millimetres(2.0).expect("a length");
        assert_eq!(one.min(two).as_millimetres(), 1.0);
        assert_eq!(one.max(two).as_millimetres(), 2.0);
    }

    #[test]
    fn an_angle_is_in_radians() {
        let angle = Angle::degrees(180.0).expect("the angle is an angle");
        assert!((angle.as_radians() - std::f64::consts::PI).abs() < 1e-9);
        assert!((angle.as_degrees() - 180.0).abs() < 1e-9);
    }

    #[test]
    fn an_angle_that_is_not_a_number_is_refused() {
        assert!(matches!(Angle::radians(f64::NAN), Err(NumericError::NotFinite { .. })));
    }

    #[test]
    fn an_angle_normalises_into_a_turn() {
        let angle = Angle::degrees(-90.0).expect("the angle is an angle");
        let normalised = angle.normalized();
        assert!((normalised.as_degrees() - 270.0).abs() < 1e-9, "a quarter turn the other way");
        assert!(normalised.as_radians() >= 0.0 && normalised.as_radians() < std::f64::consts::TAU);
    }

    #[test]
    fn angles_add_and_subtract() {
        let quarter = Angle::degrees(90.0).expect("an angle");
        let half = Angle::degrees(180.0).expect("an angle");
        assert!((quarter.checked_add(quarter).expect("the sum").as_degrees() - 180.0).abs() < 1e-9);
        assert!((half.checked_sub(quarter).expect("the difference").as_degrees() - 90.0).abs() < 1e-9);
    }

    #[test]
    fn a_length_is_read_in_the_unit_that_is_named() {
        let length = length_in("cm", 2.5).expect("the length is a length");
        assert_eq!(length.as_millimetres(), 25.0);
        let length = length_in("inches", 1.0).expect("the length is a length");
        assert!((length.as_millimetres() - 25.4).abs() < 1e-9);
    }

    #[test]
    fn a_unit_that_is_not_a_unit_is_refused() {
        let error = length_in("furlongs", 1.0).expect_err("that is not a unit");
        assert!(matches!(error, NumericError::UnknownUnit { .. }));
    }
}
