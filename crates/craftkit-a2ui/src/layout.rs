//! The **box layout** — the one thing craftkit does not have.
//!
//! # What this is
//!
//! `Row`, `Column` and `List` need to **arrange** their children. craftkit
//! has a line breaker and a typeface's metrics, and **no** flex, no box
//! arrangement and no stack — so this is the minimum arrangement the
//! layer needs: **measure the children and stack them**.
//!
//! # Where it lives, and why
//!
//! **Here**, in `craftkit-a2ui` (L3), and not in the plugin. The reason is
//! the plugin rule: a plugin may not depend on another plugin, so a
//! layout in a plugin could never be shared with a second one — and L3 is
//! exactly "the things above the SDK that are not plugins".
//!
//! # The trigger for making it more general
//!
//! The moment a **second renderer** needs to arrange the same children.
//! That is written down here so that the decision is not made again by
//! accident.
//!
//! # It is derived, never stored
//!
//! The arrangement is worked out **when the surface is applied**, and it is
//! **derived from the tree** every time the tree is applied. **No layout
//! is stored** — the rectangles are the children's *places*, set by the
//! command that creates them, exactly as every other node's place in
//! craftkit is set by the command that creates it.
//!
//! A child whose place the layout cannot work out is a node with **no
//! place** — which the house rule allows, and which no painter has to
//! draw.

use craftkit_sdk::document::Rect;

/// Which way a row lays its children out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Across, from left to right.
    Horizontal,
    /// Down, from top to bottom.
    Vertical,
}

impl Direction {
    /// The direction an a2ui `List` says.
    ///
    /// A list's `direction` is a name — `vertical` or `horizontal` — and a
    /// name that is not one of the two is **refused**: the house rule is
    /// that an unknown name is refused, and the catalogue says which names
    /// there are.
    pub fn of(name: &str) -> Option<Direction> {
        match name {
            "vertical" => Some(Direction::Vertical),
            "horizontal" => Some(Direction::Horizontal),
            _ => None,
        }
    }
}

/// How the children are **packed** along the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    /// Packed at the start.
    Start,
    /// Packed in the middle.
    Center,
    /// Packed at the end.
    End,
    /// Pushed to the edges.
    SpaceBetween,
    /// Spread with space around each.
    SpaceAround,
    /// Spread with even space.
    SpaceEvenly,
    /// Stretched to fill.
    Stretch,
}

impl Justify {
    /// The justification an a2ui component says.
    ///
    /// An empty name is `Start` — a2ui's own default — and a name that is
    /// not one of the seven is **refused**.
    pub fn of(name: &str) -> Option<Justify> {
        match name {
            "" | "start" => Some(Justify::Start),
            "center" => Some(Justify::Center),
            "end" => Some(Justify::End),
            "spaceBetween" => Some(Justify::SpaceBetween),
            "spaceAround" => Some(Justify::SpaceAround),
            "spaceEvenly" => Some(Justify::SpaceEvenly),
            "stretch" => Some(Justify::Stretch),
            _ => None,
        }
    }
}

/// How the children are **aligned** on the cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// At the start.
    Start,
    /// In the middle.
    Center,
    /// At the end.
    End,
    /// Stretched to fill.
    Stretch,
}

impl Align {
    /// The alignment an a2ui component says.
    ///
    /// An empty name is `Stretch` — a2ui's own default — and a name that
    /// is not one of the four is **refused**.
    pub fn of(name: &str) -> Option<Align> {
        match name {
            "" | "stretch" => Some(Align::Stretch),
            "start" => Some(Align::Start),
            "center" => Some(Align::Center),
            "end" => Some(Align::End),
            _ => None,
        }
    }
}

/// One child of a layout: what it asks for, and what it is given.
#[derive(Debug, Clone)]
pub struct Child {
    /// The child's **weight**: how much of the row it takes.
    ///
    /// A child with no weight takes its own size, and the rest of the
    /// space is left over.
    pub weight: Option<f64>,
    /// The child's own size, along the main axis.
    pub size: f64,
    /// The child's own size, across the main axis.
    pub cross: f64,
}

/// The **arrangement** of a set of children: where each one goes.
#[derive(Debug, Clone)]
pub struct Arrangement {
    /// The children's rectangles, in the order they were given.
    pub rects: Vec<Rect>,
}

/// Arrange the children in a box.
///
/// The box is `size` along the main axis and `cross` across it, and the
/// children are stacked along the main axis.
///
/// **The weights are the whole of it.** A child with a weight takes that
/// share of the *space that is left* after the children without a weight
/// have taken theirs; a child without a weight takes its own size. The
/// shares are handed out in order, and a share that cannot be divided is
/// given to the children that asked for it, in order.
///
/// The **cross** axis is simpler: a child that is aligned `Stretch` takes
/// the whole cross size, and a child that is not takes its own, placed
/// at the start, the middle or the end.
///
/// A child whose size is not a number — a `NaN`, say — is **refused**: an
/// input-derived number is hostile, and a box that cannot be measured is
/// not a box.
pub fn arrange(
    direction: Direction,
    justify: Justify,
    align: Align,
    size: f64,
    cross: f64,
    children: &[Child],
) -> Result<Arrangement, String> {
    if !size.is_finite() || !cross.is_finite() {
        return Err("the box is not a number this tree can draw".to_string());
    }
    // A box that is **inside out** is not a box: a negative size is a
    // number, and it is not one this tree can draw. The house rule is
    // that an out-of-range value is refused and not clamped.
    if size < 0.0 || cross < 0.0 {
        return Err("the box has a negative size".to_string());
    }
    for child in children {
        if !child.size.is_finite() || !child.cross.is_finite() {
            return Err("a child's size is not a number this tree can draw".to_string());
        }
        if child.size < 0.0 || child.cross < 0.0 {
            return Err("a child's size is negative".to_string());
        }
        if let Some(weight) = child.weight
            && (!weight.is_finite() || weight < 0.0)
        {
            return Err(format!("the weight {weight} is not a weight"));
        }
    }

    let count = children.len();
    if count == 0 {
        return Ok(Arrangement { rects: Vec::new() });
    }

    // The **fixed** children: the ones that take their own size.
    let fixed: f64 = children
        .iter()
        .map(|child| match child.weight {
            Some(_) => 0.0,
            None => child.size,
        })
        .sum();
    // The **weights**, and their total.
    let weights: Vec<f64> = children.iter().map(|child| child.weight.unwrap_or(0.0)).collect();
    let total_weight: f64 = weights.iter().sum();

    // The **left** for the children that asked for a share of it.
    let left = (size - fixed).max(0.0);
    let mut sizes: Vec<f64> = Vec::with_capacity(count);
    let mut given = 0.0;
    for (index, child) in children.iter().enumerate() {
        let child_size = match child.weight {
            None => child.size,
            Some(_) => {
                if total_weight > 0.0 {
                    let share = left * weights[index] / total_weight;
                    given += share;
                    share
                } else {
                    // Every child asked for a share and none of them can
                    // be told: they all take their own size.
                    child.size
                }
            }
        };
        sizes.push(child_size);
    }
    // The **rounding**: the last child that asked for a share takes what
    // is left, so the children fill the box exactly.
    if total_weight > 0.0 && given > 0.0 {
        let mut last = None;
        for (index, child) in children.iter().enumerate() {
            if child.weight.is_some() {
                last = Some(index);
            }
        }
        if let Some(index) = last {
            sizes[index] = (sizes[index] + (left - given)).max(0.0);
        }
    }

    // The **positions** along the main axis.
    let used: f64 = sizes.iter().sum();
    let spare = (size - used).max(0.0);
    let (mut at, gap) = match justify {
        Justify::Start => (0.0, 0.0),
        Justify::End => (spare, 0.0),
        Justify::Center => (spare / 2.0, 0.0),
        Justify::Stretch => (0.0, 0.0),
        Justify::SpaceBetween => {
            let gaps = count.saturating_sub(1);
            (0.0, if gaps == 0 { 0.0 } else { spare / gaps as f64 })
        }
        Justify::SpaceAround => {
            let gaps = count.saturating_sub(1);
            let each = if gaps == 0 { 0.0 } else { spare / (gaps + 1) as f64 };
            (each / 2.0, each)
        }
        Justify::SpaceEvenly => {
            let gaps = count + 1;
            let each = if gaps == 0 { 0.0 } else { spare / gaps as f64 };
            (each, each)
        }
    };

    let mut rects = Vec::with_capacity(count);
    for (index, child) in children.iter().enumerate() {
        let child_size = sizes[index];
        // A child that does not fit across is **refused**, and not
        // shrunk: a box that is smaller than its child is a mistake,
        // and a shrunk child is a lie.
        let cross_size = match align {
            Align::Stretch => cross,
            _ => {
                if child.cross > cross {
                    return Err(format!("a child is {} across and the box is {cross}", child.cross));
                }
                child.cross
            }
        };
        let cross_at = match align {
            Align::Start | Align::Stretch => 0.0,
            Align::Center => (cross - cross_size) / 2.0,
            Align::End => cross - cross_size,
        };
        let rect = match direction {
            Direction::Horizontal => Rect::new(at, cross_at, child_size, cross_size),
            Direction::Vertical => Rect::new(cross_at, at, cross_size, child_size),
        };
        at += child_size + gap;
        rects.push(rect);
    }
    Ok(Arrangement { rects })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A child that takes its own size.
    fn child(size: f64) -> Child {
        Child { weight: None, size, cross: 10.0 }
    }

    /// A child that asks for a share.
    fn weighted(weight: f64) -> Child {
        Child { weight: Some(weight), size: 0.0, cross: 10.0 }
    }

    /// The children are stacked in order.
    #[test]
    fn the_children_are_stacked_in_order() {
        let arrangement = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Stretch,
            100.0,
            20.0,
            &[child(30.0), child(40.0)],
        )
        .expect("the children are arranged");
        assert_eq!(arrangement.rects.len(), 2);
        assert_eq!(arrangement.rects[0].x, 0.0);
        assert_eq!(arrangement.rects[1].x, 30.0);
        assert_eq!(arrangement.rects[0].h, 20.0, "and they are stretched across");
    }

    /// A **vertical** layout stacks them down.
    #[test]
    fn a_vertical_layout_stacks_them_down() {
        let arrangement = arrange(
            Direction::Vertical,
            Justify::Start,
            Align::Start,
            100.0,
            20.0,
            &[child(30.0), child(40.0)],
        )
        .expect("the children are arranged");
        assert_eq!(arrangement.rects[0].y, 0.0);
        assert_eq!(arrangement.rects[1].y, 30.0);
        assert_eq!(arrangement.rects[0].w, 10.0, "and they are not stretched across");
    }

    /// The **weights** divide what is left.
    #[test]
    fn the_weights_divide_what_is_left() {
        let arrangement = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Stretch,
            100.0,
            20.0,
            &[weighted(1.0), weighted(1.0)],
        )
        .expect("the children are arranged");
        assert!((arrangement.rects[0].w - 50.0).abs() < 0.001, "{}", arrangement.rects[0].w);
        assert!((arrangement.rects[1].w - 50.0).abs() < 0.001, "{}", arrangement.rects[1].w);
    }

    /// A weighted and a fixed child **share** the box.
    #[test]
    fn a_weighted_and_a_fixed_child_share_the_box() {
        let arrangement = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Stretch,
            100.0,
            20.0,
            &[child(40.0), weighted(1.0)],
        )
        .expect("the children are arranged");
        assert!((arrangement.rects[1].w - 60.0).abs() < 0.001, "{}", arrangement.rects[1].w);
    }

    /// **Space between** pushes them to the edges.
    #[test]
    fn space_between_pushes_them_to_the_edges() {
        let arrangement = arrange(
            Direction::Horizontal,
            Justify::SpaceBetween,
            Align::Stretch,
            100.0,
            20.0,
            &[child(20.0), child(20.0)],
        )
        .expect("the children are arranged");
        assert_eq!(arrangement.rects[0].x, 0.0);
        assert!((arrangement.rects[1].x - 80.0).abs() < 0.001, "{}", arrangement.rects[1].x);
    }

    /// **End** pushes them to the end.
    #[test]
    fn end_pushes_them_to_the_end() {
        let arrangement =
            arrange(Direction::Horizontal, Justify::End, Align::Stretch, 100.0, 20.0, &[child(20.0)])
                .expect("the children are arranged");
        assert!((arrangement.rects[0].x - 80.0).abs() < 0.001, "{}", arrangement.rects[0].x);
    }

    /// **Center** packs them in the middle.
    #[test]
    fn center_packs_them_in_the_middle() {
        let arrangement =
            arrange(Direction::Horizontal, Justify::Center, Align::Stretch, 100.0, 20.0, &[child(20.0)])
                .expect("the children are arranged");
        assert!((arrangement.rects[0].x - 40.0).abs() < 0.001, "{}", arrangement.rects[0].x);
    }

    /// **Centered** on the cross axis.
    #[test]
    fn centered_on_the_cross_axis() {
        let arrangement =
            arrange(Direction::Horizontal, Justify::Start, Align::Center, 100.0, 20.0, &[child(20.0)])
                .expect("the children are arranged");
        assert!((arrangement.rects[0].y - 5.0).abs() < 0.001, "{}", arrangement.rects[0].y);
    }

    /// A box that is not a number is **refused**.
    #[test]
    fn a_box_that_is_not_a_number_is_refused() {
        let error =
            arrange(Direction::Horizontal, Justify::Start, Align::Stretch, f64::NAN, 20.0, &[child(20.0)])
                .expect_err("the box is refused");
        assert!(error.contains("not a number"), "{error}");
    }

    /// A child's size that is not a number is **refused**.
    #[test]
    fn a_child_that_is_not_a_number_is_refused() {
        let error =
            arrange(Direction::Horizontal, Justify::Start, Align::Stretch, 100.0, 20.0, &[child(f64::NAN)])
                .expect_err("the child is refused");
        assert!(error.contains("not a number"), "{error}");
    }

    /// A box with a **negative** size is refused.
    #[test]
    fn a_box_with_a_negative_size_is_refused() {
        let error =
            arrange(Direction::Horizontal, Justify::Start, Align::Stretch, -10.0, 20.0, &[child(20.0)])
                .expect_err("the box is refused");
        assert!(error.contains("negative"), "{error}");
    }

    /// A child that does not fit across is refused, and not shrunk.
    #[test]
    fn a_child_that_does_not_fit_is_refused() {
        let error = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Start,
            100.0,
            5.0,
            &[Child { weight: None, size: 20.0, cross: 100.0 }],
        )
        .expect_err("the child is refused");
        assert!(error.contains("across"), "{error}");
    }

    /// A weight that is not a weight is **refused**.
    #[test]
    fn a_weight_that_is_not_a_weight_is_refused() {
        let error = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Stretch,
            100.0,
            20.0,
            &[Child { weight: Some(-1.0), size: 0.0, cross: 10.0 }],
        )
        .expect_err("the weight is refused");
        assert!(error.contains("not a weight"), "{error}");
    }

    /// No children is no arrangement, and it is not an error.
    #[test]
    fn no_children_is_no_arrangement() {
        let arrangement = arrange(Direction::Horizontal, Justify::Start, Align::Stretch, 100.0, 20.0, &[])
            .expect("there is nothing to arrange");
        assert!(arrangement.rects.is_empty());
    }

    /// The children **fill** the box exactly, even with the rounding.
    #[test]
    fn the_children_fill_the_box_exactly() {
        let arrangement = arrange(
            Direction::Horizontal,
            Justify::Start,
            Align::Stretch,
            100.0,
            20.0,
            &[weighted(1.0), weighted(1.0), weighted(1.0)],
        )
        .expect("the children are arranged");
        let total: f64 = arrangement.rects.iter().map(|rect| rect.w).sum();
        assert!((total - 100.0).abs() < 0.001, "the box is filled: {total}");
    }
}
