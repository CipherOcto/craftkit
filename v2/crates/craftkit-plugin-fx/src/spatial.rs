//! The **path** of a spatial property.
//!
//! A spatial property is a property whose value is a **point**: a pair
//! or a triple of numbers. The tree's spatial property does not move
//! along a line from one keyframe to the next — it follows a **path**
//! in the plane, and the time maps to the **length** along that path.
//!
//! The path is a cubic: the two keyframes are its ends, and its two
//! control points are the ends plus the **tangents** of the keyframes.
//! The tangents are the auto ones — as After Effects computes them, a
//! sixth of the way from the keyframe before to the keyframe after —
//! and a keyframe that has no keyframe on one side uses the one it has.
//!
//! The path's length is worked out with a **table**: the curve is cut
//! into cells, the length of each cell is added up, and a point at a
//! given length is found by walking the table and refining with Newton
//! steps.

/// The number of **cells** the arc-length table has.
const ARC_N: usize = 96;

/// The 5-point Gauss-Legendre nodes and weights on -1..1.
const GAUSS: [(f64, f64); 5] = [
    (0.0, 0.568_888_888_888_888_9),
    (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
    (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
    (0.906_179_845_938_664, 0.236_926_885_056_189_1),
];

/// A cubic in the plane, with the **table** of its length.
pub struct Seg {
    /// The four points of the cubic.
    p: [[f64; 3]; 4],
    /// The length up to `u = i / ARC_N`.
    table: Vec<f64>,
}

impl Seg {
    /// The cubic through four points, and the table of its length.
    pub fn new(p: [[f64; 3]; 4]) -> Seg {
        let mut table = Vec::with_capacity(ARC_N + 1);
        table.push(0.0);
        let mut acc = 0.0;
        for i in 1..=ARC_N {
            acc += Self::arc(&p, (i - 1) as f64 / ARC_N as f64, i as f64 / ARC_N as f64);
            table.push(acc);
        }
        Seg { p, table }
    }

    /// The point of the cubic at `u`.
    fn point(p: &[[f64; 3]; 4], u: f64) -> [f64; 3] {
        [
            crate::keys::bez(p[0][0], p[1][0], p[2][0], p[3][0], u),
            crate::keys::bez(p[0][1], p[1][1], p[2][1], p[3][1], u),
            crate::keys::bez(p[0][2], p[1][2], p[2][2], p[3][2], u),
        ]
    }

    /// How fast the curve is moving at `u`: the length of its tangent.
    fn tangent_len(p: &[[f64; 3]; 4], u: f64) -> f64 {
        (0..3).map(|k| crate::keys::bez_d(p[0][k], p[1][k], p[2][k], p[3][k], u).powi(2)).sum::<f64>().sqrt()
    }

    /// The length of the curve between `a` and `b`.
    fn arc(p: &[[f64; 3]; 4], a: f64, b: f64) -> f64 {
        let (m, h) = ((a + b) * 0.5, (b - a) * 0.5);
        GAUSS.iter().map(|(x, w)| w * Self::tangent_len(p, m + h * x)).sum::<f64>() * h
    }

    /// The **length** of the whole curve.
    pub fn length(&self) -> f64 {
        *self.table.last().unwrap_or(&0.0)
    }

    /// The point at the length `s` along the curve.
    ///
    /// The table cell's linear guess is refined with Newton steps on
    /// the true length, so the position is smooth along a curved path
    /// and not piecewise linear.
    pub fn at_length(&self, s: f64) -> [f64; 3] {
        let len = self.length();
        // A path whose length overflowed or is not a number has no
        // table to walk.
        if !(len > 0.0 && len.is_finite()) {
            return self.p[0];
        }
        let s = s.clamp(0.0, len);
        let i = self.table.partition_point(|&v| v < s).clamp(1, ARC_N);
        let (Some(&a), Some(&b)) = (self.table.get(i - 1), self.table.get(i)) else {
            return self.p[0];
        };
        let (u0, u1) = ((i - 1) as f64 / ARC_N as f64, i as f64 / ARC_N as f64);
        let mut u = u0 + if b > a { (s - a) / (b - a) } else { 0.0 } * (u1 - u0);
        for _ in 0..4 {
            let d = Self::tangent_len(&self.p, u);
            if d <= 1e-12 {
                break;
            }
            let e = a + Self::arc(&self.p, u0, u) - s;
            u = (u - e / d).clamp(u0, u1);
            if e.abs() <= 1e-12 * len.max(1.0) {
                break;
            }
        }
        Self::point(&self.p, u)
    }
}

/// The **auto** spatial tangents of the keyframe at `i`.
///
/// The tangents are the ones After Effects computes: the out tangent
/// is a sixth of the way from the keyframe before to the keyframe
/// after, and the in tangent is its mirror. The times play no part.
///
/// A keyframe at the end of the list uses the one neighbour it has, so
/// the path leaves the first keyframe and enters the last one. And a
/// side facing a keyframe at the same place gets no tangent at all, so
/// the segment between two keyframes at the same place has no length.
pub fn auto_tangents(values: &[[f64; 3]], i: usize) -> ([f64; 3], [f64; 3]) {
    if i >= values.len() || values.len() < 2 {
        return ([0.0; 3], [0.0; 3]);
    }
    let a = values[i.saturating_sub(1)];
    let c = values[(i + 1).min(values.len() - 1)];
    let tout = [(c[0] - a[0]) / 6.0, (c[1] - a[1]) / 6.0, (c[2] - a[2]) / 6.0];
    let p = values[i];
    let tin = if i > 0 && a == p { [0.0; 3] } else { [-tout[0], -tout[1], -tout[2]] };
    let tout = if i + 1 < values.len() && c == p { [0.0; 3] } else { tout };
    (tin, tout)
}

/// Whether a control point lies **on the chord** — so that a cubic
/// with such handles traces the straight line.
pub fn on_chord(a: [f64; 3], c: [f64; 3], b: [f64; 3]) -> bool {
    let d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let dd = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    if dd <= 1e-24 {
        return false;
    }
    let t = (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]) / dd;
    let off = [v[0] - d[0] * t, v[1] - d[1] * t, v[2] - d[2] * t];
    (0.0..=1.0).contains(&t) && off[0] * off[0] + off[1] * off[1] + off[2] * off[2] <= dd * 1e-18
}

/// The **progress** along a segment, from the eases of its two sides.
///
/// The speeds are the speeds along the path: the tree's ease says how
/// fast the value is moving, and the path is longer than the straight
/// line, so the speed along the path is smaller.
pub fn ease_progress(out_speed_n: f64, out_inf: f64, in_speed_n: f64, in_inf: f64, x: f64) -> f64 {
    // The time handles may cross: the tree keeps both influences.
    let (i0, i1) = (out_inf.clamp(0.0, 1.0), in_inf.clamp(0.0, 1.0));
    let u = crate::keys::solve_u(0.0, i0, 1.0 - i1, 1.0, x);
    crate::keys::bez(0.0, out_speed_n * i0, 1.0 - in_speed_n * i1, 1.0, u)
}

/// The **distance** between two points.
pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}
