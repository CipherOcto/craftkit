//! The **colour** of a drawing: a colour in one **space**, and the
//! same colour in another.
//!
//! The tree's own: a colour space is described by its **primaries**,
//! its **white point** and its **tone curve** — and a colour is
//! carried from one space to another by undoing the tone curve of the
//! first, multiplying by the matrix of its primaries, adapting the
//! white with **Bradford**, and doing the same in reverse for the
//! second.
//!
//! # The rules
//!
//! - the **spaces** are the tree's own:
//!   - `srgb` — the sRGB IEC 61966-2.1 primaries, D65 and the sRGB
//!     transfer function;
//!   - `display-p3` — the DCI-P3 primaries, D65 and the sRGB transfer
//!     function;
//!   - `adobe-rgb-compat` — the Adobe RGB (1998) primaries, D65 and
//!     the gamma of 563/256;
//!   - `prophoto-compat` — the ROMM RGB primaries, D50 and the gamma
//!     of 1.8;
//!   - `linear-srgb` — the sRGB primaries with a linear transfer
//!     function;
//!   - `rec2020` — the Rec. ITU-R BT.2020 primaries, D65 and the
//!     Rec. 709 transfer function;
//!   - `gray-gamma-2.2` — a gray with the gamma of 2.2;
//!   - `sgray` — a gray with the sRGB transfer function, so that a
//!     gray `v` looks like the RGB `(v, v, v)` of sRGB.
//! - a **space** that is not one of them is **refused**;
//! - a **gray** space holds one value, and the gray of a colour is
//!   the **luminance** of it — the tree's own rule, and the rule
//!   here;
//! - a colour that falls **outside** the space it is put into is
//!   **refused**: a channel that is not a channel of that space is
//!   not a colour of it, and the tree clips it while this tree does
//!   not;
//! - the **gamut** of a colour in a space is measured the tree's own
//!   way: the colour is sent to the space and **clipped** there, and
//!   the two ways of looking at it are compared in **CIE L\*a\*b\*** —
//!   and a colour is **out of gamut** when the difference is more than
//!   the tree's own threshold of **four**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Number, Outcome, Text};

/// The name of the command of the conversion, in the messages.
const COMMAND: &str = "vector.colour.convert";

/// The name of the command of the gamut, in the messages.
const GAMUT_COMMAND: &str = "vector.colour.gamut";

/// The **slack** of the range of a channel: a number that is a
/// rounding error is not outside a space.
const SLACK: f64 = 1e-9;

/// The **threshold** of the gamut measure: the tree's own.
///
/// It is not a public number: it is the tree's own, and it is named
/// in the README of the tree.
const THRESHOLD: f64 = 4.0;

/// A **matrix**: three rows of three numbers.
type Mat3 = [[f64; 3]; 3];

/// The **white point** of the PCS: D50.
const D50: [f64; 3] = [0.9642, 1.0, 0.8249];

/// The chromaticity of **D65**.
const D65_XY: [f64; 2] = [0.3127, 0.3290];

/// The **Bradford** cone response matrix (Lam 1985).
const BRADFORD: Mat3 = [[0.8951, 0.2664, -0.1614], [-0.7502, 1.7135, 0.0367], [0.0389, -0.0685, 1.0296]];

/// The **inverse** of the Bradford matrix.
const BRADFORD_INV: Mat3 = match invert(&BRADFORD) {
    Some(matrix) => matrix,
    None => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
};

/// Multiply two **matrices**.
const fn mul(a: &Mat3, b: &Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    let mut row = 0;
    while row < 3 {
        let mut column = 0;
        while column < 3 {
            let mut sum = 0.0;
            let mut index = 0;
            while index < 3 {
                sum += a[row][index] * b[index][column];
                index += 1;
            }
            out[row][column] = sum;
            column += 1;
        }
        row += 1;
    }
    out
}

/// Apply a **matrix** to a vector.
const fn apply(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    let mut out = [0.0; 3];
    let mut row = 0;
    while row < 3 {
        out[row] = matrix[row][0] * vector[0] + matrix[row][1] * vector[1] + matrix[row][2] * vector[2];
        row += 1;
    }
    out
}

/// The **determinant** of a matrix.
const fn det(matrix: &Mat3) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

/// The **inverse** of a matrix, when it has one.
const fn invert(matrix: &Mat3) -> Option<Mat3> {
    let determinant = det(matrix);
    if determinant.abs() < 1e-12 {
        return None;
    }
    let inverse = 1.0 / determinant;
    Some([
        [
            (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1]) * inverse,
            (matrix[0][2] * matrix[2][1] - matrix[0][1] * matrix[2][2]) * inverse,
            (matrix[0][1] * matrix[1][2] - matrix[0][2] * matrix[1][1]) * inverse,
        ],
        [
            (matrix[1][2] * matrix[2][0] - matrix[1][0] * matrix[2][2]) * inverse,
            (matrix[0][0] * matrix[2][2] - matrix[0][2] * matrix[2][0]) * inverse,
            (matrix[0][2] * matrix[1][0] - matrix[0][0] * matrix[1][2]) * inverse,
        ],
        [
            (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]) * inverse,
            (matrix[0][1] * matrix[2][0] - matrix[0][0] * matrix[2][1]) * inverse,
            (matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0]) * inverse,
        ],
    ])
}

/// The **XYZ** of a chromaticity `(x, y)`.
fn xy_to_xyz(xy: [f64; 2]) -> [f64; 3] {
    let [x, y] = xy;
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// The **Bradford** adaptation matrix from the white `src` to the
/// white `dst`, both XYZ.
fn bradford(src: [f64; 3], dst: [f64; 3]) -> Mat3 {
    let cone_src = apply(&BRADFORD, src);
    let cone_dst = apply(&BRADFORD, dst);
    let scale = [
        [cone_dst[0] / cone_src[0], 0.0, 0.0],
        [0.0, cone_dst[1] / cone_src[1], 0.0],
        [0.0, 0.0, cone_dst[2] / cone_src[2]],
    ];
    mul(&BRADFORD_INV, &mul(&scale, &BRADFORD))
}

/// The **matrix** from RGB to XYZ, relative to the `white`, for the
/// primaries given as chromaticities — and nothing when the
/// primaries are not independent.
fn rgb_to_xyz_matrix(r: [f64; 2], g: [f64; 2], b: [f64; 2], white: [f64; 3]) -> Option<Mat3> {
    let (xr, xg, xb) = (xy_to_xyz(r), xy_to_xyz(g), xy_to_xyz(b));
    let matrix = [[xr[0], xg[0], xb[0]], [xr[1], xg[1], xb[1]], [xr[2], xg[2], xb[2]]];
    let inverse = invert(&matrix)?;
    let scale = apply(&inverse, white);
    Some([
        [matrix[0][0] * scale[0], matrix[0][1] * scale[1], matrix[0][2] * scale[2]],
        [matrix[1][0] * scale[0], matrix[1][1] * scale[1], matrix[1][2] * scale[2]],
        [matrix[2][0] * scale[0], matrix[2][1] * scale[1], matrix[2][2] * scale[2]],
    ])
}

/// The **tone curve** of a space.
#[derive(Clone, Copy)]
enum Curve {
    /// The sRGB transfer function (IEC 61966-2.1).
    Srgb,
    /// `y = x^g`.
    Gamma(f64),
    /// `y = x`.
    Linear,
}

impl Curve {
    /// The curve at `x`.
    fn eval(self, x: f64) -> f64 {
        match self {
            Curve::Srgb => {
                if x <= 0.04045 {
                    x / 12.92
                } else {
                    ((x + 0.055) / 1.055).powf(2.4)
                }
            }
            Curve::Gamma(gamma) => {
                if x < 0.0 {
                    -(-x).powf(gamma)
                } else {
                    x.powf(gamma)
                }
            }
            Curve::Linear => x,
        }
    }

    /// The curve **undone**, at `y`.
    fn eval_inverse(self, y: f64) -> f64 {
        match self {
            Curve::Srgb => {
                if y <= 0.0031308 {
                    12.92 * y
                } else {
                    1.055 * y.powf(1.0 / 2.4) - 0.055
                }
            }
            Curve::Gamma(gamma) => {
                if y < 0.0 {
                    -(-y).powf(1.0 / gamma)
                } else {
                    y.powf(1.0 / gamma)
                }
            }
            Curve::Linear => y,
        }
    }
}

/// A **space** of colour.
struct Space {
    /// The name the tree gives it.
    name: &'static str,
    /// The primaries, as chromaticities; a gray space has none.
    primaries: Option<[[f64; 2]; 3]>,
    /// The white point, as XYZ.
    white: [f64; 3],
    /// The tone curve of the space.
    curve: Curve,
}

/// The **spaces** the tree knows: the name of each and its own.
pub const SPACES: [&str; 8] = [
    "srgb",
    "display-p3",
    "adobe-rgb-compat",
    "prophoto-compat",
    "linear-srgb",
    "rec2020",
    "gray-gamma-2.2",
    "sgray",
];

/// The **white point** of D65, as XYZ.
fn d65() -> [f64; 3] {
    xy_to_xyz(D65_XY)
}

/// The space of the **name**, when the tree knows it.
fn space_of(name: &str) -> Option<Space> {
    let wanted = name.trim().to_ascii_lowercase();
    let space = match wanted.as_str() {
        "srgb" => Space {
            name: "srgb",
            primaries: Some([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]]),
            white: d65(),
            curve: Curve::Srgb,
        },
        "display-p3" | "p3" => Space {
            name: "display-p3",
            primaries: Some([[0.680, 0.320], [0.265, 0.690], [0.150, 0.060]]),
            white: d65(),
            curve: Curve::Srgb,
        },
        "adobe-rgb-compat" | "adobe-rgb" => Space {
            name: "adobe-rgb-compat",
            primaries: Some([[0.64, 0.33], [0.21, 0.71], [0.15, 0.06]]),
            white: d65(),
            curve: Curve::Gamma(563.0 / 256.0),
        },
        "prophoto-compat" | "prophoto" | "romm-rgb" => Space {
            name: "prophoto-compat",
            primaries: Some([[0.7347, 0.2653], [0.1596, 0.8404], [0.0366, 0.0001]]),
            white: D50,
            curve: Curve::Gamma(1.8),
        },
        "linear-srgb" => Space {
            name: "linear-srgb",
            primaries: Some([[0.64, 0.33], [0.30, 0.60], [0.15, 0.06]]),
            white: d65(),
            curve: Curve::Linear,
        },
        "rec2020" => Space {
            name: "rec2020",
            primaries: Some([[0.708, 0.292], [0.170, 0.797], [0.131, 0.046]]),
            white: d65(),
            curve: Curve::Srgb,
        },
        "gray-gamma-2.2" | "gray" => {
            Space { name: "gray-gamma-2.2", primaries: None, white: D50, curve: Curve::Gamma(2.2) }
        }
        "sgray" => Space { name: "sgray", primaries: None, white: D50, curve: Curve::Srgb },
        _ => return None,
    };
    Some(space)
}

/// The **matrix** of a space: its device values to the PCS.
fn to_pcs(space: &Space) -> Option<Mat3> {
    let primaries = space.primaries?;
    let matrix = rgb_to_xyz_matrix(primaries[0], primaries[1], primaries[2], space.white)?;
    let adapt = bradford(space.white, D50);
    Some(mul(&adapt, &matrix))
}

/// The **PCS** (XYZ D50) of a colour in a space.
fn pcs_of(space: &Space, rgb: [f64; 3]) -> [f64; 3] {
    let linear = [space.curve.eval(rgb[0]), space.curve.eval(rgb[1]), space.curve.eval(rgb[2])];
    match to_pcs(space) {
        Some(matrix) => apply(&matrix, linear),
        // A **gray** space: the PCS is the white of the PCS, scaled by
        // the gray.
        None => [D50[0] * linear[0], D50[1] * linear[0], D50[2] * linear[0]],
    }
}

/// The **linear** values of a colour in a space, from the PCS.
fn linear_of(space: &Space, pcs: [f64; 3]) -> [f64; 3] {
    match to_pcs(space) {
        Some(matrix) => match invert(&matrix) {
            Some(inverse) => apply(&inverse, pcs),
            None => [0.0; 3],
        },
        // A **gray** space: the gray is the luminance of the colour.
        None => {
            let gray = pcs[1] / D50[1];
            [gray, gray, gray]
        }
    }
}

/// The colour of a **space**, carried from one space to another.
fn convert(from: &Space, to: &Space, rgb: [f64; 3]) -> [f64; 3] {
    let pcs = pcs_of(from, rgb);
    let linear = linear_of(to, pcs);
    [to.curve.eval_inverse(linear[0]), to.curve.eval_inverse(linear[1]), to.curve.eval_inverse(linear[2])]
}

/// CIE XYZ to **L\*a\*b\***, relative to the `white`.
fn xyz_to_lab(xyz: [f64; 3], white: [f64; 3]) -> [f64; 3] {
    const EPS: f64 = 216.0 / 24389.0;
    const KAPPA: f64 = 24389.0 / 27.0;
    let f = |t: f64| if t > EPS { t.cbrt() } else { (KAPPA * t + 16.0) / 116.0 };
    let fx = f(xyz[0] / white[0]);
    let fy = f(xyz[1] / white[1]);
    let fz = f(xyz[2] / white[2]);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// The **difference** of two colours, the tree's own way.
///
/// The colour is sent to the space and **clipped** there, and the two
/// ways of looking at it are compared in L\*a\*b\* — the lightness on
/// the scale of a hundred and the two colours on the scale of two
/// hundred and fifty-five.
fn difference(from: &Space, to: &Space, rgb: [f64; 3]) -> f64 {
    let pcs = pcs_of(from, rgb);
    let direct = xyz_to_lab(pcs, D50);
    let linear = linear_of(to, pcs);
    let encoded = [
        to.curve.eval_inverse(linear[0]),
        to.curve.eval_inverse(linear[1]),
        to.curve.eval_inverse(linear[2]),
    ];
    let clipped = [encoded[0].clamp(0.0, 1.0), encoded[1].clamp(0.0, 1.0), encoded[2].clamp(0.0, 1.0)];
    let via = xyz_to_lab(pcs_of(to, clipped), D50);
    let d = |index: usize, scale: f64| (direct[index] - via[index]) * scale;
    (d(0, 100.0).powi(2) + d(1, 255.0).powi(2) + d(2, 255.0).powi(2)).sqrt()
}

/// The **colour** of a caller: a hex text, and the three channels of
/// it.
fn colour_of(text: &str, id: &str) -> Result<[f64; 3], EngineError> {
    let parsed = craftkit_sdk::document::Colour::from_hex(text.trim())
        .map_err(|message| EngineError::Failed { id: id.to_string(), message })?;
    Ok([f64::from(parsed.0[0]), f64::from(parsed.0[1]), f64::from(parsed.0[2])])
}

commands! {
    /// **Convert** a colour from one space into another.
    ///
    /// The tree's own: the tone curve of the first space is undone, the
    /// primaries of the first space are multiplied out, the white is
    /// adapted with Bradford, and the second space is done in reverse.
    vector.colour.convert {
        area: "vector",
        layer: "vector",
        icon: "list",
        params: {
            /// The colour: a hex text, `#rrggbb`.

            colour: Text,
            /// The space the colour is in.

            from: Text,
            /// The space the colour is put into.

            to: Text,
        },
        run: {

            let _ = session;
            let from = space_of(&self.from.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a space of colour: it is one of {}",
                    self.from.0.trim(),
                    SPACES.join(", ")
                ),
            })?;
            let to = space_of(&self.to.0).ok_or_else(|| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "`{}` is not a space of colour: it is one of {}",
                    self.to.0.trim(),
                    SPACES.join(", ")
                ),
            })?;
            let colour = colour_of(&self.colour.0, COMMAND)?;
            let converted = convert(&from, &to, colour);
            // A colour that is **outside** the space it is put into is
            // refused: the tree clips it, and this tree does not.
            if converted.iter().any(|channel| !channel.is_finite() || *channel < -SLACK || *channel > 1.0 + SLACK)
            {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the colour is outside the space `{}`: {converted:?}", to.name),
                });
            }
            Ok(Outcome::new(format!("the colour in the space `{}`", to.name)).with_data(json!({
                "colour": converted,
                "space": to.name,
                "from": from.name,
                "difference": difference(&from, &to, colour),
            })))

        }
    }
    /// **Say** whether a colour is inside the **gamut** of a space.
    ///
    /// The tree's own: the colour is sent to the space and clipped there,
    /// and the two ways of looking at it are compared in L\*a\*b\* — and a
    /// colour is out of gamut when the difference is more than the
    /// threshold.
    vector.colour.gamut {
        area: "vector",
        layer: "vector",
        icon: "list",
        params: {
            /// The colour: a hex text, `#rrggbb`.

            colour: Text,
            /// The space the colour is in.

            from: Text,
            /// The space whose gamut is tested.

            to: Text,
            /// The threshold of the measure: the tree's own is four.

            threshold: Option<Number>,
        },
        run: {

            let _ = session;
            let from = space_of(&self.from.0).ok_or_else(|| EngineError::Failed {
                id: GAMUT_COMMAND.to_string(),
                message: format!(
                    "`{}` is not a space of colour: it is one of {}",
                    self.from.0.trim(),
                    SPACES.join(", ")
                ),
            })?;
            let to = space_of(&self.to.0).ok_or_else(|| EngineError::Failed {
                id: GAMUT_COMMAND.to_string(),
                message: format!(
                    "`{}` is not a space of colour: it is one of {}",
                    self.to.0.trim(),
                    SPACES.join(", ")
                ),
            })?;
            let colour = colour_of(&self.colour.0, GAMUT_COMMAND)?;
            let threshold = match self.threshold {
                Some(threshold) => {
                    let threshold = threshold.0;
                    if !threshold.is_finite() || threshold < 0.0 {
                        return Err(EngineError::Failed {
                            id: GAMUT_COMMAND.to_string(),
                            message: "the threshold of the gamut is a number that is not negative".to_string(),
                        });
                    }
                    threshold
                }
                None => THRESHOLD,
            };
            let difference = difference(&from, &to, colour);
            let in_gamut = difference <= threshold;
            Ok(Outcome::new(format!(
                "the colour is {} the gamut of `{}`",
                if in_gamut { "inside" } else { "outside" },
                to.name
            ))
            .with_data(json!({
                "colour": colour,
                "from": from.name,
                "to": to.name,
                "difference": difference,
                "threshold": threshold,
                "in_gamut": in_gamut,
            })))

        }
    }
}
