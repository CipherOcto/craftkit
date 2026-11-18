//! The **conditional formats** of a sheet.
//!
//! A conditional format is a **range** of cells, a **criterion** and a
//! **look**: the cells of the range whose value matches the criterion
//! are written with it. The look is a **number format** and a **fill**,
//! and each of them is a part of its own: a rule may carry the format
//! and no fill, and a rule may carry the fill and no format.
//!
//! The rules **layer**: every rule that matches is tried, and for each
//! part of the look the rule that is **seen first** and says something
//! about it wins — the tree's own rule, where two rules set the same
//! part of a cell the one seen first wins. And the tree's **stop if
//! true**: a rule that matched and says stop ends the walk, and the
//! rules after it are not tried.
//!
//! And the format a cell is **written with** is **derived**: the
//! conditional format that matches it wins over the cell's own format,
//! as it does in the tree and in Excel — the conditional format is a
//! rule about the cell and the cell's own format is a fact about the
//! cell, and the rule is on top.
//!
//! And the criterion is the tree's **one criterion** (§347): the same
//! operation and the same value the filter of a sheet uses, and the
//! four operations that name a cell by **what it holds** — `isEmpty`,
//! `isNotEmpty`, `isError` and `isNotError` — with no value at all.
//!
//! And a rule may be about the range as a whole: the **top** of it —
//! the count highest numbers of it, or the count lowest — is a rule of
//! its own (`grid.conditional.top10`), and so is the **duplicate** of a
//! value in it (`grid.conditional.duplicate`) and the **average** of it
//! (`grid.conditional.average`). And a rule may be a **colour scale**:
//! the fill of a cell is the mix of the two colours that hold it, in
//! the proportion its number is between them
//! (`grid.conditional.colour.scale`). A scale has two colours — of the
//! lowest and of the highest number of the range — or three, and then
//! the middle one stands for the **median** of the numbers.
//!
//! And a rule may be a **data bar**: the bar behind a cell is as long
//! as its number is between the lowest and the highest of the range
//! (`grid.conditional.data.bar`). The bar is **derived**, and the front
//! end draws it.
//!
//! Such a rule needs the **numbers** or the **counts** of the range,
//! and they are worked out **once** for a read and read as often as a
//! cell is read: a rule worked out again for every cell of the range
//! would cost the square of the range.
//!
//! # What the port found
//!
//! - **The tree's rule is a style and this one's is a look.** The
//!   tree's conditional format carries a style — a fill, a font
//!   colour, an icon set — and the style is shared between cells. Here
//!   it carries a **number format** and a **fill**: the format is a
//!   cell's own thing (the tree's format is a style too, and this
//!   tree's format is the cell's own), and the fill is a cell's own
//!   thing too (§821). The tree's font colour and its icon set are
//!   still not here — and §1029 says otherwise for the icon set: it is
//!   here (`grid.conditional.icon.set`), and §1033 for the font
//!   colour: it is here too, and the rule carries it as its own
//!   `colour`.
//! - **The tree's criteria are many and this one's is one.** The
//!   tree's rule may be a comparison, a colour scale, a data bar, an
//!   icon set or an expression. Here a rule is a **comparison** — the
//!   same criterion the filter of a sheet uses (§347) — and the other
//!   kinds are commands of their own when they come — and §1017 says
//!   otherwise: the **expression** is here (`grid.conditional.formula`),
//!   and §1021 says otherwise: the **top ten** is here
//!   (`grid.conditional.top10`), and §1022 says otherwise: the
//!   **duplicate** is here (`grid.conditional.duplicate`), and §1023
//!   says otherwise: the **above-the-average** is here
//!   (`grid.conditional.average`), and §1024 says otherwise: the
//!   **colour scale** is here (`grid.conditional.colour.scale`), and
//!   §1027 says otherwise: the **data bar** is here
//!   (`grid.conditional.data.bar`), and §1029 says otherwise: the
//!   **icon set** is here (`grid.conditional.icon.set`), and the rule
//!   of the time period is not.
//! - **The tree's rules are ordered and the first that matches wins.**
//!   The tree's rules are in a list and it walks them in **priority** —
//!   the rule that names the lower number is seen first. Here they are
//!   in a list too, and the rule that names the **lower priority** is
//!   tried first; a rule that names no priority is tried after the
//!   ones that do, in the order the sheet holds it. The tree renumbers
//!   the priorities when a rule comes or goes; here the priority is
//!   **named** and it stays where it is.
//! - **The tree's rules layer and here they layer too.** The tree
//!   applies every rule that matches and where two of them set the
//!   same part of a cell the one seen first wins; here each part of
//!   the look is the part of the first rule that matched and said
//!   something about it. And the tree's **stop if true** — a rule
//!   that ends the walk when it matched — is here as a **stop**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

use crate::formula::{Area, CellRef, Criterion, FILTER_OPS};

/// The **top** of a range: the count highest values of it, or the count
/// lowest.
///
/// The count is a count of the values, or a **percent** of the numbers
/// of the range.
#[derive(Debug, Clone, PartialEq)]
pub struct Top {
    /// How many of the values: ten of them by default.
    pub count: u32,
    /// Whether the **lowest** values are the ones and not the highest.
    pub bottom: bool,
    /// Whether the count is a **percent** of the numbers of the range.
    pub percent: bool,
}

/// The **duplicate** of a range: the cells whose value is **repeated**
/// in it, or the cells whose value is **not**.
#[derive(Debug, Clone, PartialEq)]
pub struct Duplicate {
    /// Whether the rule is about the values that are **not** repeated.
    pub unique: bool,
}

/// The **average** of a range: the cells whose number is **above** it,
/// or the cells whose number is **below** it.
#[derive(Debug, Clone, PartialEq)]
pub struct Average {
    /// Whether the rule is about the numbers **above** the average and
    /// not the numbers below it.
    pub above: bool,
}

/// The **colour scale** of a range: two colours — the one of the lowest
/// number of the range and the one of the highest — or three, and then
/// the middle one stands for the **middle** of the numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourScale {
    /// The colours of the scale, in order: the lowest, and the highest.
    /// With three, the middle one stands for the middle of the numbers.
    pub colours: Vec<String>,
}

/// The **data bar** of a range: the bar behind a cell, and the colour
/// of it.
#[derive(Debug, Clone, PartialEq)]
pub struct DataBar {
    /// The colour of the bar.
    pub colour: String,
}

/// The **icon set** of a range: the three icons of it, and the two
/// numbers that tell which icon a cell has.
#[derive(Debug, Clone, PartialEq)]
pub struct IconSet {
    /// The **percent** of the numbers that carry the high icon: the
    /// 67th by default.
    pub high: f64,
    /// The **percent** of the numbers that carry the middle icon: the
    /// 33rd by default.
    pub low: f64,
}

/// A **conditional format** of a sheet.
pub struct Conditional {
    /// The range of cells it is about.
    pub range: String,
    /// The criterion a cell's value is judged by, when the rule has
    /// one.
    pub criterion: Option<Criterion>,
    /// The **top** of the range the rule is about, when the rule has
    /// one: the cells whose number is among the highest of the range
    /// are the cells that match.
    pub top: Option<Top>,
    /// The **duplicate** of the range the rule is about, when the rule
    /// has one: the cells whose value is repeated in the range are the
    /// cells that match.
    pub duplicate: Option<Duplicate>,
    /// The **average** of the range the rule is about, when the rule
    /// has one: the cells whose number is above the average are the
    /// cells that match.
    pub average: Option<Average>,
    /// The **colour scale** of the range the rule is about, when the
    /// rule has one: the fill of a cell is the mix of the two colours,
    /// in the proportion its number is between the lowest and the
    /// highest of the range.
    pub colour_scale: Option<ColourScale>,
    /// The **data bar** of the range the rule is about, when the rule
    /// has one: the bar behind a cell is as long as its number is
    /// between the lowest and the highest of the range.
    pub data_bar: Option<DataBar>,
    /// The **icon set** of the range the rule is about, when the rule
    /// has one: the icon of a cell is the one its number falls under.
    pub icon_set: Option<IconSet>,
    /// The **formula** a cell is judged by, when the rule has one: the
    /// cells whose formula is **true** are the cells that match.
    ///
    /// The formula is written as it is in the **top-left cell** of the
    /// range, and it is moved with the cell it is judged for.
    pub formula: Option<String>,
    /// The **format code** the matching cells are written with, when
    /// the rule names one.
    pub format: Option<String>,
    /// The **fill** the matching cells are filled with, when the rule
    /// names one.
    pub fill: Option<String>,
    /// The **colour** the text of the matching cells is written in,
    /// when the rule names one.
    pub colour: Option<String>,
    /// The **stop**: the tree's *stop if true*. A rule that matched
    /// and says stop ends the walk, and the rules after it are not
    /// tried.
    pub stop: Option<bool>,
    /// The **priority** of the rule, when it names one: the lower the
    /// number, the earlier the rule is tried.
    pub priority: Option<u32>,
}

/// The **conditional formats** of a sheet, in order.
pub fn conditionals_of(node: &craftkit_sdk::Node) -> Vec<Conditional> {
    let mut formats = Vec::new();
    let Some(list) = node.data.get("conditionals").and_then(|list| list.as_array()) else {
        return formats;
    };
    for entry in list {
        let Some(range) = entry.get("range").and_then(|range| range.as_str()) else { continue };
        let op = entry.get("op").and_then(|op| op.as_str()).unwrap_or("=").to_string();
        let value = entry.get("value").and_then(|value| value.as_str()).unwrap_or("").to_string();
        // A rule has a **criterion**, a **formula** or a **top**: the
        // cells whose formula is true are the cells that match, and the
        // cells whose number is among the highest of the range are the
        // cells that match a top.
        let formula = entry.get("formula").and_then(|formula| formula.as_str()).map(|f| f.to_string());
        let top = entry.get("top").and_then(|top| {
            if !top.is_object() {
                return None;
            }
            let count = top.get("count").and_then(|count| count.as_u64())?;
            let count = u32::try_from(count).ok()?;
            Some(Top {
                count,
                bottom: top.get("bottom").and_then(|bottom| bottom.as_bool()).unwrap_or(false),
                percent: top.get("percent").and_then(|percent| percent.as_bool()).unwrap_or(false),
            })
        });
        let duplicate = entry.get("duplicate").and_then(|duplicate| {
            if !duplicate.is_object() {
                return None;
            }
            let unique = duplicate.get("unique").and_then(|unique| unique.as_bool()).unwrap_or(false);
            Some(Duplicate { unique })
        });
        let average = entry.get("average").and_then(|average| {
            if !average.is_object() {
                return None;
            }
            let above = average.get("above").and_then(|above| above.as_bool()).unwrap_or(true);
            Some(Average { above })
        });
        let colour_scale = entry.get("colour_scale").and_then(|scale| {
            if !scale.is_object() {
                return None;
            }
            let colours = scale.get("colours")?.as_array()?;
            let colours: Vec<String> = colours
                .iter()
                .map(|colour| colour.as_str().map(|colour| colour.to_string()))
                .collect::<Option<Vec<String>>>()?;
            Some(ColourScale { colours })
        });
        let data_bar = entry.get("data_bar").and_then(|bar| {
            if !bar.is_object() {
                return None;
            }
            let colour = bar.get("colour").and_then(|colour| colour.as_str())?.to_string();
            Some(DataBar { colour })
        });
        let icon_set = entry.get("icon_set").and_then(|icons| {
            if !icons.is_object() {
                return None;
            }
            let high = icons.get("high").and_then(|high| high.as_f64()).unwrap_or(67.0);
            let low = icons.get("low").and_then(|low| low.as_f64()).unwrap_or(33.0);
            Some(IconSet { high, low })
        });
        let criterion = if formula.is_some()
            || top.is_some()
            || duplicate.is_some()
            || average.is_some()
            || colour_scale.is_some()
            || data_bar.is_some()
            || icon_set.is_some()
        {
            None
        } else {
            Some(Criterion { op, value })
        };
        let format = entry.get("format").and_then(|format| format.as_str()).map(|format| format.to_string());
        let fill = entry.get("fill").and_then(|fill| fill.as_str()).map(|fill| fill.to_string());
        let colour = entry.get("colour").and_then(|colour| colour.as_str()).map(|colour| colour.to_string());
        let stop = entry.get("stop").and_then(|stop| stop.as_bool());
        let priority = entry
            .get("priority")
            .and_then(|priority| priority.as_u64())
            .and_then(|priority| u32::try_from(priority).ok());
        formats.push(Conditional {
            range: range.to_string(),
            criterion,
            top,
            duplicate,
            average,
            colour_scale,
            data_bar,
            icon_set,
            formula,
            format,
            fill,
            colour,
            stop,
            priority,
        });
    }
    formats
}

/// The **area** a conditional format's range names.
pub(crate) fn area_of(range: &str) -> Option<Area> {
    let trimmed = range.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = crate::formula::Anchor::parse(first)?;
            let second = crate::formula::Anchor::parse(second)?;
            Area::new(first.cell, second.cell)
        }
        None => {
            let only = crate::formula::Anchor::parse(trimmed)?;
            Area::new(only.cell, only.cell)
        }
    };
    Some(area)
}

/// The **conditional formats** of a sheet, in the order they are
/// tried.
///
/// The rules are tried in **priority**: the rule that names the lower
/// number is tried first, and a rule that names none is tried after
/// the ones that do, in the order the sheet holds them. The tree
/// numbers the rules the same way — the rule that names the lower
/// number is seen first, and where two rules set the same part of a
/// cell the one seen first wins.
pub fn ordered_conditionals(node: &craftkit_sdk::Node) -> Vec<Conditional> {
    let mut conditionals = conditionals_of(node);
    conditionals.sort_by_key(|conditional| conditional.priority.unwrap_or(u32::MAX));
    conditionals
}

/// The **facts** about the ranges the conditional formats of a sheet are
/// about.
///
/// A rule may be about the range as a whole — the highest values of it,
/// say — and such a rule needs the **numbers** of the range. They are
/// worked out **once** and read as often as a cell is read: a rule that
/// is worked out again for every cell of the range costs the square of
/// the range.
#[derive(Debug, Default, Clone)]
pub struct Facts {
    /// The numbers of each range a rule needs, by the range's own text.
    numbers: Vec<(String, Vec<f64>)>,
    /// How many times each value of a range is held in it, by the
    /// range's own text.
    counts: Vec<(String, std::collections::BTreeMap<String, u64>)>,
}

impl Facts {
    /// The facts of a sheet: the numbers and the counts of the ranges
    /// its rules are about, and nothing when no rule needs any.
    pub fn of(book: &crate::formula::Book, node: &craftkit_sdk::Node) -> Facts {
        let mut facts = Facts::default();
        for conditional in conditionals_of(node) {
            if conditional.top.is_none()
                && conditional.duplicate.is_none()
                && conditional.average.is_none()
                && conditional.colour_scale.is_none()
                && conditional.data_bar.is_none()
                && conditional.icon_set.is_none()
            {
                continue;
            }
            let Some(area) = area_of(&conditional.range) else { continue };
            if conditional.top.is_some()
                || conditional.average.is_some()
                || conditional.colour_scale.is_some()
                || conditional.data_bar.is_some()
                || conditional.icon_set.is_some()
            {
                let numbers = numbers_of(book, &node.name, &area, node);
                facts.numbers.push((conditional.range.clone(), numbers));
            }
            if conditional.duplicate.is_some() {
                let counts = counts_of(book, &node.name, &area, node);
                facts.counts.push((conditional.range.clone(), counts));
            }
        }
        facts
    }

    /// The **numbers** of a range, when they are known.
    pub fn numbers_of(&self, range: &str) -> Option<&[f64]> {
        self.numbers.iter().find(|(name, _)| name == range).map(|(_, numbers)| numbers.as_slice())
    }

    /// The **counts** of the values of a range, when they are known.
    pub fn counts_of(&self, range: &str) -> Option<&std::collections::BTreeMap<String, u64>> {
        self.counts.iter().find(|(name, _)| name == range).map(|(_, counts)| counts)
    }
}

/// The **numbers** of the cells of an area that hold one.
///
/// A cell that does not hold a number is not one of them, and a cell
/// that is not there is not either.
fn numbers_of(book: &crate::formula::Book, sheet: &str, area: &Area, node: &craftkit_sdk::Node) -> Vec<f64> {
    let mut numbers = Vec::new();
    for (cell, _) in crate::cells_of(node).iter() {
        if !area.covers(cell) {
            continue;
        }
        let value = book.value_of(sheet, cell);
        let Some(number) = value.as_number() else { continue };
        numbers.push(number);
    }
    // The numbers are kept **in order**, the highest first: a rule that
    // is about the range reads the order and does not sort them again.
    numbers.sort_by(|one, other| other.partial_cmp(one).unwrap_or(std::cmp::Ordering::Equal));
    numbers
}

/// The **key** of a value, for the count of the values of a range.
///
/// The **number** is the number, the **text** is the text without case,
/// and the kinds are kept apart: a number and the text of it are two
/// values and not one.
fn key_of(value: &crate::formula::Value) -> String {
    use crate::formula::Value;
    match value {
        Value::Number(number) => format!("n:{number}"),
        Value::Text(text) => format!("t:{}", text.to_lowercase()),
        Value::Bool(flag) => format!("b:{flag}"),
        Value::Error(message) => format!("e:{}", message.to_lowercase()),
        Value::Empty => "empty".to_string(),
    }
}

/// How many times each **value** of an area is held in it.
///
/// A cell that holds **nothing** is not a value and is not counted: a
/// blank cell is not a duplicate of another blank cell.
fn counts_of(
    book: &crate::formula::Book,
    sheet: &str,
    area: &Area,
    node: &craftkit_sdk::Node,
) -> std::collections::BTreeMap<String, u64> {
    let mut counts = std::collections::BTreeMap::new();
    for (cell, _) in crate::cells_of(node).iter() {
        if !area.covers(cell) {
            continue;
        }
        let value = book.value_of(sheet, cell);
        if matches!(value, crate::formula::Value::Empty) {
            continue;
        }
        *counts.entry(key_of(&value)).or_insert(0) += 1;
    }
    counts
}

/// Whether a cell's **value** is repeated in the range — or, when the
/// rule is about the values that are **not** repeated, whether it is
/// held once and only once.
fn duplicate_matches(
    facts: &Facts,
    range: &str,
    duplicate: &Duplicate,
    value: &crate::formula::Value,
) -> bool {
    if matches!(value, crate::formula::Value::Empty) {
        return false;
    }
    let Some(counts) = facts.counts_of(range) else { return false };
    let count = counts.get(&key_of(value)).copied().unwrap_or(0);
    if duplicate.unique { count == 1 } else { count > 1 }
}

/// The **median** of a list of numbers in order, the highest first: the
/// middle number of them, and the half between the two middle ones when
/// there are an even number of them.
fn median_of(sorted: &[f64]) -> Option<f64> {
    let length = sorted.len();
    if length == 0 {
        return None;
    }
    if length % 2 == 1 {
        return sorted.get(length / 2).copied();
    }
    let upper = sorted.get(length / 2 - 1).copied()?;
    let lower = sorted.get(length / 2).copied()?;
    Some((upper + lower) / 2.0)
}

/// The **fill** a cell of a colour scale is written with: the mix of
/// the colours of the scale, in the proportion its number is between
/// the two of them that hold it.
///
/// A cell that does not hold a number has no fill, and a range whose
/// numbers are all the same has no **span** and writes nothing: a
/// proportion between nothing and nothing is not a number.
fn scale_fill(
    facts: &Facts,
    range: &str,
    scale: &ColourScale,
    value: &crate::formula::Value,
) -> Option<String> {
    let numbers = facts.numbers_of(range)?;
    let highest = numbers.first().copied()?;
    let lowest = numbers.last().copied()?;
    if highest == lowest {
        return None;
    }
    let number = value.as_number()?;
    if number < lowest || number > highest {
        return None;
    }
    // The **stops** of the scale: the number each colour stands for.
    // The lowest and the highest, and the middle of the numbers when
    // the scale has three colours.
    let stops: Vec<(f64, (u8, u8, u8))> = match scale.colours.as_slice() {
        [low, high] => {
            let low = crate::colours::channels_of(low)?;
            let high = crate::colours::channels_of(high)?;
            vec![(lowest, low), (highest, high)]
        }
        [low, middle, high] => {
            let low = crate::colours::channels_of(low)?;
            let middle = crate::colours::channels_of(middle)?;
            let high = crate::colours::channels_of(high)?;
            let median = median_of(numbers)?;
            vec![(lowest, low), (median, middle), (highest, high)]
        }
        _ => return None,
    };
    // The **pair** of stops that holds the number, and the part of the
    // way the number is from the first of them to the second.
    for pair in stops.windows(2) {
        let (low, low_colour) = pair[0];
        let (high, high_colour) = pair[1];
        if number < low || number > high {
            continue;
        }
        let part = if high > low { (number - low) / (high - low) } else { 0.0 };
        return Some(crate::colours::mix_of(low_colour, high_colour, part));
    }
    None
}

/// The **part** of a cell's width that its bar fills: the proportion of
/// the way its number is between the lowest and the highest of the
/// range.
///
/// A cell that does not hold a number has no bar, and a range whose
/// numbers are all the same has no span and no bars.
fn bar_part(facts: &Facts, range: &str, value: &crate::formula::Value) -> Option<f64> {
    let numbers = facts.numbers_of(range)?;
    let highest = numbers.first().copied()?;
    let lowest = numbers.last().copied()?;
    if highest == lowest {
        return None;
    }
    let number = value.as_number()?;
    if number < lowest || number > highest {
        return None;
    }
    Some((number - lowest) / (highest - lowest))
}

/// The **icon** of a cell: the high one, the middle one or the low
/// one, as the tree's own numbers — two, one and zero.
///
/// The icons stand for the **percentiles** of the numbers of the range:
/// the high icon for the numbers at or above the high percentile, the
/// middle icon for the numbers at or above the low percentile, and the
/// low icon for the rest.
fn icon_of(facts: &Facts, range: &str, icons: &IconSet, value: &crate::formula::Value) -> Option<u8> {
    let numbers = facts.numbers_of(range)?;
    let high = crate::formula::percentile_of(numbers, icons.high)?;
    let low = crate::formula::percentile_of(numbers, icons.low)?;
    let number = value.as_number()?;
    if number >= high {
        Some(2)
    } else if number >= low {
        Some(1)
    } else {
        Some(0)
    }
}

/// Whether a cell's **number** is above the average of the range — or,
/// when the rule is about the numbers **below** it, below it.
fn average_matches(facts: &Facts, range: &str, average: &Average, value: &crate::formula::Value) -> bool {
    let Some(numbers) = facts.numbers_of(range) else { return false };
    let Some(average_of_the_range) = crate::formula::average_of(numbers) else { return false };
    let Some(number) = value.as_number() else { return false };
    if average.above { number > average_of_the_range } else { number < average_of_the_range }
}

/// Whether a cell's **number** is among the top of the range.
///
/// The **threshold** is the count-th highest number of the range, or the
/// count-th lowest, and a cell matches when its number is at the
/// threshold or above it. A cell that does not hold a number does not
/// match.
fn top_matches(facts: &Facts, range: &str, top: &Top, value: &crate::formula::Value) -> bool {
    let Some(numbers) = facts.numbers_of(range) else { return false };
    let Some(threshold) =
        crate::formula::top10_threshold_of_sorted(numbers, top.bottom, top.percent, top.count)
    else {
        return false;
    };
    let Some(number) = value.as_number() else { return false };
    // The **highest** values are the ones at the threshold or above it,
    // and the **lowest** the ones at the threshold or below it.
    if top.bottom { number <= threshold } else { number >= threshold }
}

/// What a **conditional format** says about a cell.
#[derive(Debug, Clone, Default)]
pub struct Looks {
    /// The **format code** the cell is written with.
    pub format: Option<String>,
    /// The **fill** the cell is filled with.
    pub fill: Option<String>,
    /// The **colour** the text of the cell is written in.
    pub colour: Option<String>,
    /// The **bar** behind the cell: the part of the cell's width it
    /// fills and the colour of it.
    pub bar: Option<Bar>,
    /// The **icon** in the cell: the tree's own numbers, and two is
    /// the high icon, one the middle one and zero the low one.
    pub icon: Option<u8>,
}

/// The **bar** behind a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    /// The part of the cell's width the bar fills, from zero to one.
    pub part: f64,
    /// The colour of the bar.
    pub colour: String,
}

/// The **look** the conditional formats of a sheet give a cell.
///
/// The rules are tried in **priority**, and for each **part** of the
/// look the rule that is seen first and says something about it wins:
/// the tree's own rule, where two rules set the same part of a cell
/// the one seen first wins.
///
/// And the tree's **stop if true**: a rule that matched and says stop
/// ends the walk, and the rules after it are not tried.
pub fn looks_for(
    book: &crate::formula::Book,
    node: &craftkit_sdk::Node,
    cell: CellRef,
    value: &crate::formula::Value,
    system: crate::dates::System,
    facts: &Facts,
) -> Looks {
    let mut looks = Looks::default();
    for conditional in ordered_conditionals(node) {
        let Some(area) = area_of(&conditional.range) else { continue };
        if !area.covers(cell) {
            continue;
        }
        // A rule of an **icon set** writes the icon of every cell of the
        // range: the icon is **derived** from the cell's number, and it
        // is not a match and not a not-match.
        if let Some(icons) = &conditional.icon_set {
            if let Some(icon) = icon_of(facts, &conditional.range, icons, value) {
                if looks.icon.is_none() {
                    looks.icon = Some(icon);
                }
                if conditional.stop.unwrap_or(false) {
                    break;
                }
            }
            continue;
        }
        // A rule of a **data bar** writes the bar behind every cell of
        // the range: the length of it is **derived** from the cell's
        // number, and it is not a match and not a not-match.
        if let Some(bar) = &conditional.data_bar {
            if let Some(part) = bar_part(facts, &conditional.range, value) {
                if looks.bar.is_none() {
                    looks.bar = Some(Bar { part, colour: bar.colour.clone() });
                }
                if conditional.stop.unwrap_or(false) {
                    break;
                }
            }
            continue;
        }
        // A rule of a **colour scale** writes the fill of every cell of
        // the range: the colour is **derived** from the cell's number,
        // and it is not a match and not a not-match.
        if let Some(scale) = &conditional.colour_scale {
            if let Some(fill) = scale_fill(facts, &conditional.range, scale, value) {
                if looks.fill.is_none() {
                    looks.fill = Some(fill);
                }
                if conditional.stop.unwrap_or(false) {
                    break;
                }
            }
            continue;
        }
        // A rule has a **criterion**, a **formula** or a **top**, and
        // the cells that match it are the cells the rule writes.
        let matched =
            match (&conditional.formula, &conditional.top, &conditional.duplicate, &conditional.average) {
                (Some(formula), _, _, _) => formula_matches(book, &node.name, formula, &area, cell),
                (None, Some(top), _, _) => top_matches(facts, &conditional.range, top, value),
                (None, None, Some(duplicate), _) => {
                    duplicate_matches(facts, &conditional.range, duplicate, value)
                }
                (None, None, None, Some(average)) => {
                    average_matches(facts, &conditional.range, average, value)
                }
                (None, None, None, None) => {
                    conditional.criterion.as_ref().is_some_and(|criterion| criterion.matches(value, system))
                }
            };
        if !matched {
            continue;
        }
        if looks.format.is_none() {
            looks.format = conditional.format.clone();
        }
        if looks.fill.is_none() {
            looks.fill = conditional.fill.clone();
        }
        if looks.colour.is_none() {
            looks.colour = conditional.colour.clone();
        }
        if conditional.stop.unwrap_or(false) {
            break;
        }
    }
    looks
}

/// The conditional format that **writes** a cell, if one does.
///
/// The first one that matches wins, and the rules are tried in
/// priority.
pub fn format_for(
    book: &crate::formula::Book,
    node: &craftkit_sdk::Node,
    cell: CellRef,
    value: &crate::formula::Value,
    system: crate::dates::System,
    facts: &Facts,
) -> Option<String> {
    looks_for(book, node, cell, value, system, facts).format
}

/// Whether the **formula** of a rule is **true** for a cell.
///
/// The formula is written as it is in the **top-left cell** of the
/// range, and it is moved with the cell it is judged for: the rule of
/// `A1:A10` and the formula `A1>0` is the rule *the cell is above
/// zero* for every cell of the range.
fn formula_matches(book: &crate::formula::Book, sheet: &str, text: &str, area: &Area, cell: CellRef) -> bool {
    let row = i64::from(cell.row) - i64::from(area.start.row);
    let column = i64::from(cell.col) - i64::from(area.start.col);
    let Some(shifted) = crate::formula::shift_in_formula(text, row, column) else { return false };
    let Ok(expression) = crate::formula::parse(&shifted) else { return false };
    let mut being = Vec::new();
    matches!(book.evaluate(sheet, &expression, &mut being), crate::formula::Value::Bool(true))
}

/// The **format** a rule names.
///
/// The tree's rule says the format of the cells, and a rule may say the
/// fill and no format. A format the tree cannot read is refused.
fn format_of_rule(given: Option<&Text>, id: &str) -> Result<Option<String>, EngineError> {
    let Some(given) = given else { return Ok(None) };
    let given = given.0.trim().to_string();
    if given.chars().count() > crate::format::MAX_FORMAT_CHARS {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a format is at most {} characters long", crate::format::MAX_FORMAT_CHARS),
        });
    }
    match crate::formatnames::resolve(&given) {
        Ok(code) => Ok(Some(code)),
        Err(refusal) => Err(EngineError::Failed {
            id: id.to_string(),
            message: crate::formatnames::message(&given, &refusal),
        }),
    }
}

/// The **fill** a rule names: a colour, and nothing else.
fn fill_of_rule(given: Option<&Text>, id: &str) -> Result<Option<String>, EngineError> {
    let Some(given) = given else { return Ok(None) };
    let Some(colour) = crate::colours::colour_of(&given.0) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{}` is not a colour: a colour is `#rrggbb`", given.0.trim()),
        });
    };
    match colour {
        Some(colour) => Ok(Some(colour)),
        None => Err(EngineError::Failed {
            id: id.to_string(),
            message: "a rule's fill is a colour, and `none` is not one".to_string(),
        }),
    }
}

/// The **colour** the text of the matching cells is written in.
///
/// A rule's colour is a colour: `none` is not one, because a rule that
/// says the text is in the colour it is in says nothing.
fn colour_of_rule(given: Option<&Text>, id: &str) -> Result<Option<String>, EngineError> {
    let Some(given) = given else { return Ok(None) };
    let Some(colour) = crate::colours::colour_of(&given.0) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{}` is not a colour: a colour is `#rrggbb`", given.0.trim()),
        });
    };
    match colour {
        Some(colour) => Ok(Some(colour)),
        None => Err(EngineError::Failed {
            id: id.to_string(),
            message: "a rule's colour is a colour, and `none` is not one".to_string(),
        }),
    }
}

/// The **priority** of a rule: a whole number, and not a negative one.
fn priority_of(given: Option<&Number>, id: &str) -> Result<Option<u32>, EngineError> {
    let Some(given) = given else { return Ok(None) };
    let number = given.0;
    if !number.is_finite() || number.fract() != 0.0 || number < 0.0 || number > u32::MAX as f64 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the priority of a rule is a whole number, and {number} is not one"),
        });
    }
    Ok(Some(number as u32))
}

/// The **entry** of a conditional format, as the sheet holds it.
fn entry_of(conditional: &Conditional) -> Json {
    json!({
        "range": conditional.range,
        "op": conditional.criterion.as_ref().map(|criterion| criterion.op.clone()),
        "value": conditional.criterion.as_ref().map(|criterion| criterion.value.clone()),
        "formula": conditional.formula,
        "top": conditional.top.as_ref().map(|top| json!({
            "count": top.count,
            "bottom": top.bottom,
            "percent": top.percent,
        })),
        "duplicate": conditional.duplicate.as_ref().map(|duplicate| json!({
            "unique": duplicate.unique,
        })),
        "average": conditional.average.as_ref().map(|average| json!({
            "above": average.above,
        })),
        "colour_scale": conditional.colour_scale.as_ref().map(|scale| json!({
            "colours": scale.colours,
        })),
        "data_bar": conditional.data_bar.as_ref().map(|bar| json!({
            "colour": bar.colour,
        })),
        "icon_set": conditional.icon_set.as_ref().map(|icons| json!({
            "high": icons.high,
            "low": icons.low,
        })),
        "format": conditional.format,
        "fill": conditional.fill,
        "colour": conditional.colour,
        "stop": conditional.stop,
        "priority": conditional.priority,
    })
}

commands! {
    /// Put a **conditional format** on a range of cells.
    ///
    /// The cells of the range whose value matches the criterion are
    /// written with the format.
    grid.conditional.set {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// The operation: `=`, `!=`, `<`, `<=`, `>` or `>=`, or one
            /// of the four on the **text** of a cell: `beginsWith`,
            /// `endsWith`, `contains` or `notContains`, or one of the
            /// four that name a cell by **what it holds**: `isEmpty`,
            /// `isNotEmpty`, `isError` or `isNotError`.
            op: Text,
            /// The value it compares with. An operation that names a cell
            /// by what it holds compares with nothing, and it takes no
            /// value.
            value: Option<Text>,
            /// The format the matching cells are written with. A rule may
            /// carry the format, the fill, or both.
            format: Option<Text>,
            /// The fill the matching cells are filled with: `#rrggbb`. A rule
            /// may carry the fill and no format.
            fill: Option<Text>,
            /// The **colour** the text of the matching cells is written
            /// in: `#rrggbb`. A rule may carry the colour and neither
            /// the format nor the fill.
            colour: Option<Text>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.set".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            let op = self.op.0.trim().to_string();
            if !Criterion::op_is_known(&op) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "there is no operation `{op}`: the operations are {}",
                        FILTER_OPS.join(", ")
                    ),
                });
            }
            let value = match Criterion::value_of(&op, self.value.as_ref().map(|value| value.0.as_str())) {
                Ok(value) => value,
                Err(message) => return Err(EngineError::Failed { id: id.clone(), message }),
            };
            // The **number** the operation compares with is read by the
            // tree's own rule of numbers — the rule `VALUE` reads a text
            // by — so a currency sign, thousands separators and a date
            // are numbers.
            let number = crate::textvalue::number_of(&value, crate::system_of(session, sheet));
            if (op == "<" || op == "<=" || op == ">" || op == ">=") && number.is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the operation `{op}` compares numbers, and `{value}` is not one"),
                });
            }
            let format = format_of_rule(self.format.as_ref(), &id)?;
            let fill = fill_of_rule(self.fill.as_ref(), &id)?;
            let colour = colour_of_rule(self.colour.as_ref(), &id)?;
            if format.is_none() && fill.is_none() && colour.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "name what the rule says: the format, the fill, the colour, or any two of \
                              them"
                        .to_string(),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "op": op,
                "value": value,
                "format": format,
                "fill": fill,
                "colour": colour,
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "op": op,
                    "value": value,
                    "format": format,
                    "fill": fill,
                    "colour": colour,
                    "stop": stop,
                }),
            ))

        }
    }

    /// Put a **conditional format** on a range of cells, and judge the
    /// cells by a **formula**.
    ///
    /// The formula is written as it is in the **top-left cell** of the
    /// range, and it is moved with the cell it is judged for: the rule
    /// of `A1:A10` and the formula `A1>0` is the rule *the cell is
    /// above zero* for every cell of the range.
    ///
    /// The cells whose formula is **true** are written with the look.
    grid.conditional.formula {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// The **formula** the cells are judged by.
            formula: Text,
            /// The format the matching cells are written with. A rule may
            /// carry the format, the fill, or both.
            format: Option<Text>,
            /// The fill the matching cells are filled with: `#rrggbb`. A rule
            /// may carry the fill and no format.
            fill: Option<Text>,
            /// The **colour** the text of the matching cells is written
            /// in: `#rrggbb`. A rule may carry the colour and neither
            /// the format nor the fill.
            colour: Option<Text>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.formula".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            // The **formula**: it is read, and a formula that cannot be
            // read is refused.
            let formula = self.formula.0.trim().to_string();
            if formula.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a rule's formula is a formula".to_string(),
                });
            }
            if crate::formula::parse(&formula).is_err() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{formula}` is not a formula"),
                });
            }
            let format = format_of_rule(self.format.as_ref(), &id)?;
            let fill = fill_of_rule(self.fill.as_ref(), &id)?;
            let colour = colour_of_rule(self.colour.as_ref(), &id)?;
            if format.is_none() && fill.is_none() && colour.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "name what the rule says: the format, the fill, the colour, or any two of \
                              them"
                        .to_string(),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "formula": formula,
                "format": format,
                "fill": fill,
                "colour": colour,
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "formula": formula,
                    "format": format,
                    "fill": fill,
                    "colour": colour,
                    "stop": stop,
                }),
            ))

        }
    }

    /// Take a **conditional format** away.
    grid.conditional.clear {
        area: "grid",
        layer: "grid",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range it was about: `A1:A10`.
            range: Text,
        },
        run: {
            let id = "grid.conditional.clear".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let conditionals = conditionals_of(node);
            if !conditionals.iter().any(|conditional| conditional.range == range) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no conditional format of {range} on the sheet `{sheet}`"),
                });
            }
            let list: Vec<_> = conditionals
                .into_iter()
                .filter(|conditional| conditional.range != range)
                .map(|conditional| entry_of(&conditional))
                .collect();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the conditional format of {range} is taken away"))
                .with_data(json!({ "sheet": sheet, "range": range })))

        }
    }

    /// Put a **conditional format** on a range of cells, and judge the
    /// cells by the **top** of the range.
    ///
    /// The **top** is the count highest numbers of the range, or the
    /// count lowest, and the cells whose number is at the threshold or
    /// above it are the cells the rule holds for. A cell that does not
    /// hold a number is not one of them.
    ///
    /// The count is a count of the numbers of the range, or a **percent**
    /// of them, and it is rounded **up**.
    grid.conditional.top10 {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// How many of the values: ten of them by default.
            count: Option<Number>,
            /// Whether the count is a **percent** of the numbers of the
            /// range.
            percent: Option<Bool>,
            /// Whether the **lowest** values are the ones and not the
            /// highest.
            bottom: Option<Bool>,
            /// The format the matching cells are written with. A rule may
            /// carry the format, the fill, or both.
            format: Option<Text>,
            /// The fill the matching cells are filled with: `#rrggbb`. A rule
            /// may carry the fill and no format.
            fill: Option<Text>,
            /// The **colour** the text of the matching cells is written
            /// in: `#rrggbb`. A rule may carry the colour and neither
            /// the format nor the fill.
            colour: Option<Text>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.top10".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            // The **count**: a whole number of values, and one at least.
            let count = self.count.map(|count| count.0).unwrap_or(10.0);
            if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the count of a top-ten rule is a whole number of values, and `{count}` is not one"
                    ),
                });
            }
            let count = count as u32;
            let percent = self.percent.map(|percent| percent.0).unwrap_or(false);
            if percent && count > 100 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the percent of a top-ten rule is a hundred at most, and this one is {count}"),
                });
            }
            let bottom = self.bottom.map(|bottom| bottom.0).unwrap_or(false);
            let format = format_of_rule(self.format.as_ref(), &id)?;
            let fill = fill_of_rule(self.fill.as_ref(), &id)?;
            let colour = colour_of_rule(self.colour.as_ref(), &id)?;
            if format.is_none() && fill.is_none() && colour.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "name what the rule says: the format, the fill, the colour, or any two of \
                              them"
                        .to_string(),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "top": {
                    "count": count,
                    "bottom": bottom,
                    "percent": percent,
                },
                "format": format,
                "fill": fill,
                "colour": colour,
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "count": count,
                    "percent": percent,
                    "bottom": bottom,
                    "format": format,
                    "fill": fill,
                    "colour": colour,
                    "stop": stop,
                }),
            ))
        }
    }

    /// Put a **conditional format** on a range of cells, and judge the
    /// cells by the **duplicate** of their value.
    ///
    /// The cells whose value is **repeated** in the range are the cells
    /// the rule holds for — and the cells whose value is held once and
    /// only once are the cells it holds for, when the rule is about the
    /// values that are **not** repeated.
    ///
    /// The **text** of a value is the text without case, and a number
    /// and the text of it are two values and not one. A cell that holds
    /// **nothing** is not a value and is not counted.
    grid.conditional.duplicate {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// Whether the rule is about the values that are **not**
            /// repeated — the ones held once and only once.
            unique: Option<Bool>,
            /// The format the matching cells are written with. A rule may
            /// carry the format, the fill, or both.
            format: Option<Text>,
            /// The fill the matching cells are filled with: `#rrggbb`. A rule
            /// may carry the fill and no format.
            fill: Option<Text>,
            /// The **colour** the text of the matching cells is written
            /// in: `#rrggbb`. A rule may carry the colour and neither
            /// the format nor the fill.
            colour: Option<Text>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.duplicate".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            let unique = self.unique.map(|unique| unique.0).unwrap_or(false);
            let format = format_of_rule(self.format.as_ref(), &id)?;
            let fill = fill_of_rule(self.fill.as_ref(), &id)?;
            let colour = colour_of_rule(self.colour.as_ref(), &id)?;
            if format.is_none() && fill.is_none() && colour.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "name what the rule says: the format, the fill, the colour, or any two of \
                              them"
                        .to_string(),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "duplicate": {
                    "unique": unique,
                },
                "format": format,
                "fill": fill,
                "colour": colour,
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "unique": unique,
                    "format": format,
                    "fill": fill,
                    "colour": colour,
                    "stop": stop,
                }),
            ))
        }
    }

    /// Put a **conditional format** on a range of cells, and judge the
    /// cells by the **average** of the range.
    ///
    /// The **average** is the average of the numbers of the range, and
    /// the cells whose number is **above** it are the cells the rule
    /// holds for — and the cells whose number is below it are the cells
    /// it holds for, when the rule is about the numbers below it.
    ///
    /// A cell that does not hold a **number** is not one of them.
    grid.conditional.average {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// Whether the cells **above** the average are the ones; the
            /// cells below it are the ones when they are not.
            above: Option<Bool>,
            /// The format the matching cells are written with. A rule may
            /// carry the format, the fill, or both.
            format: Option<Text>,
            /// The fill the matching cells are filled with: `#rrggbb`. A rule
            /// may carry the fill and no format.
            fill: Option<Text>,
            /// The **colour** the text of the matching cells is written
            /// in: `#rrggbb`. A rule may carry the colour and neither
            /// the format nor the fill.
            colour: Option<Text>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.average".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            let above = self.above.map(|above| above.0).unwrap_or(true);
            let format = format_of_rule(self.format.as_ref(), &id)?;
            let fill = fill_of_rule(self.fill.as_ref(), &id)?;
            let colour = colour_of_rule(self.colour.as_ref(), &id)?;
            if format.is_none() && fill.is_none() && colour.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "name what the rule says: the format, the fill, the colour, or any two of \
                              them"
                        .to_string(),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "average": {
                    "above": above,
                },
                "format": format,
                "fill": fill,
                "colour": colour,
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "above": above,
                    "format": format,
                    "fill": fill,
                    "colour": colour,
                    "stop": stop,
                }),
            ))
        }
    }

    /// Put a **conditional format** on a range of cells, and fill the
    /// cells by a **colour scale**.
    ///
    /// The scale has **two** colours — the one of the lowest number of
    /// the range and the one of the highest — or **three**, and then the
    /// middle one stands for the **middle** of the numbers. The fill of
    /// a cell is the **mix** of the two colours that hold it, in the
    /// proportion its number is between them.
    ///
    /// A cell that does not hold a **number** has no fill, and a range
    /// whose numbers are all the same has no span and is filled with
    /// nothing: a proportion between nothing and nothing is not a
    /// number.
    grid.conditional.colour.scale {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// The **colours** of the scale: the one of the lowest number
            /// of the range and the one of the highest, as `#rrggbb`.
            colours: craftkit_sdk::serde_json::Value,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.colour.scale".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            // The **colours**: two of them, and the tree refuses any
            // other number of them.
            let Some(colours) = self.colours.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the colours of a colour scale are a list of two".to_string(),
                });
            };
            if colours.len() != 2 && colours.len() != 3 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the colours of a colour scale are two or three, and this scale has {}",
                        colours.len()
                    ),
                });
            }
            let mut parsed: Vec<String> = Vec::new();
            for colour in colours {
                let Some(given) = colour.as_str() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the colours of a colour scale are colours".to_string(),
                    });
                };
                let Some(colour) = crate::colours::colour_of(given) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{given}` is not a colour: a colour is `#rrggbb`"),
                    });
                };
                let Some(colour) = colour else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a colour scale has two colours, and `none` is not one".to_string(),
                    });
                };
                parsed.push(colour);
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "colour_scale": {
                    "colours": parsed,
                },
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "colours": parsed,
                    "stop": stop,
                }),
            ))
        }
    }

    /// Put a **conditional format** on a range of cells, and draw a
    /// **bar** behind the cells of it.
    ///
    /// The bar is as long as the cell's number is between the lowest
    /// and the highest of the range: the cell with the lowest number
    /// has no bar, and the cell with the highest has one that fills the
    /// cell.
    ///
    /// A cell that does not hold a **number** has no bar, and a range
    /// whose numbers are all the same has no span and no bars.
    grid.conditional.data.bar {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// The colour of the bar: `#rrggbb`.
            colour: Text,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.data.bar".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            // The **colour** of the bar: a colour, and nothing else.
            let given = self.colour.0.trim().to_string();
            let Some(colour) = crate::colours::colour_of(&given) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{given}` is not a colour: a colour is `#rrggbb`"),
                });
            };
            let Some(colour) = colour else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a bar has a colour, and `none` is not one".to_string(),
                });
            };
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "data_bar": {
                    "colour": colour,
                },
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "colour": colour,
                    "stop": stop,
                }),
            ))
        }
    }

    /// Put a **conditional format** on a range of cells, and put an
    /// **icon** in the cells of it.
    ///
    /// The set has **three** icons: the high one, the middle one and
    /// the low one. The icons stand for the **percentiles** of the
    /// numbers of the range: the numbers at or above the high
    /// percentile carry the high icon, the numbers at or above the low
    /// percentile carry the middle one, and the rest carry the low one.
    ///
    /// The icons are the tree's own and they are drawn in code: a
    /// triangle pointing up, a square and a triangle pointing down.
    grid.conditional.icon.set {
        area: "grid",
        layer: "grid",
        icon: "brush",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
            /// The **percent** of the numbers that carry the high icon:
            /// sixty-seven of them by default.
            high: Option<Number>,
            /// The **percent** of the numbers that carry the middle icon:
            /// thirty-three of them by default.
            low: Option<Number>,
            /// The tree's **stop if true**: the walk ends at this rule when it
            /// matched, and the rules after it are not tried.
            stop: Option<Bool>,
            /// The **priority** of the rule: the lower the number, the earlier
            /// the rule is tried.
            priority: Option<Number>,
        },
        run: {
            let id = "grid.conditional.icon.set".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            // The **percents**: two of them, and the high one above the
            // low one.
            let high = self.high.map(|high| high.0).unwrap_or(67.0);
            let low = self.low.map(|low| low.0).unwrap_or(33.0);
            for (name, percent) in [("high", high), ("low", low)] {
                if !percent.is_finite() || percent < 0.0 || percent > 100.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the {name} percent of an icon set is a hundred at most, and this one is {percent}"),
                    });
                }
            }
            if high <= low {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the high percent of an icon set is above the low one, and {high} is not above {low}"),
                });
            }
            let stop = self.stop.map(|stop| stop.0);
            let range = format!("{}:{}", area.start.to_text(), area.end.to_text());
            let priority = priority_of(self.priority.as_ref(), &id)?;
            let mut list = conditionals_of(node)
                .into_iter()
                .map(|conditional| entry_of(&conditional))
                .collect::<Vec<_>>();
            // A range may have **one** conditional format: a second one for
            // the same range replaces the first.
            list.retain(|entry| entry["range"].as_str() != Some(range.as_str()));
            list.push(json!({
                "range": range,
                "icon_set": {
                    "high": high,
                    "low": low,
                },
                "stop": stop,
                "priority": priority,
            }));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["conditionals"] = json!(list);
            }
            Ok(Outcome::changed(format!("the range {range} of the sheet `{sheet}` is conditional")).with_data(
                json!({
                    "sheet": sheet,
                    "range": range,
                    "high": high,
                    "low": low,
                    "stop": stop,
                }),
            ))
        }
    }

    /// The **cells** of a range that a conditional format holds for.
    ///
    /// The cells are **derived**: they are worked out from the cells each
    /// time they are asked for.
    grid.conditional.check {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range it is about: `A1:A10`.
            range: Text,
        },
        run: {
            let id = "grid.conditional.check".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let range = self.range.0.trim().to_string();
            let Some(area) = area_of(&range) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{range}` is not a range of cells"),
                });
            };
            let conditionals = ordered_conditionals(node);
            let book = crate::book_of(session.document());
            let facts = Facts::of(&book, node);
            let name = node.name.clone();
            let mut cells: Vec<Json> = Vec::new();
            for row in area.start.row..=area.end.row {
                for col in area.start.col..=area.end.col {
                    let cell = CellRef { col, row };
                    let value = book.value_of(&name, cell);
                    let matched = conditionals.iter().find(|conditional| {
                        let Some(area) = area_of(&conditional.range) else { return false };
                        if !area.covers(cell) {
                            return false;
                        }
                        match (&conditional.formula, &conditional.top, &conditional.duplicate, &conditional.average) {
                            (Some(formula), _, _, _) => formula_matches(&book, &name, formula, &area, cell),
                            (None, Some(top), _, _) => top_matches(&facts, &conditional.range, top, &value),
                            (None, None, Some(duplicate), _) => {
                                duplicate_matches(&facts, &conditional.range, duplicate, &value)
                            }
                            (None, None, None, Some(average)) => {
                                average_matches(&facts, &conditional.range, average, &value)
                            }
                            (None, None, None, None) => conditional
                                .criterion
                                .as_ref()
                                .is_some_and(|criterion| criterion.matches(&value, book.system())),
                        }
                    });
                    if let Some(matched) = matched {
                        cells.push(json!({
                            "cell": cell.to_text(),
                            "format": matched.format,
                        }));
                    }
                }
            }
            let count = cells.len();
            Ok(Outcome::new(format!("{count} cell(s) match")).with_data(json!({
                "sheet": sheet,
                "range": range,
                "cells": cells,
            })))

        }
    }
}
