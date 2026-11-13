//! The **arrays** of the tree's formula language.
//!
//! The tree's spreadsheet has functions that give back a **table** of
//! values: the rows that meet a condition (`FILTER`), the rows in
//! order (`SORT`), the rows that are not like each other (`UNIQUE`),
//! a row of numbers (`SEQUENCE`), and the rest of that family.
//!
//! A formula here is a **value** and not a table, and so a function
//! that would give back a table is **refused** — with a message that
//! says so, the same message the tree's own `INDEX` gives when it is
//! asked for a whole row.
//!
//! # The list is the tree's
//!
//! The list is here so that the tree can say **which** of the tree's
//! functions are not here and **why**: a function that is refused with
//! a reason is not missing — it is answered.
//!
//! The module is **public** so that the tree's own test can walk the
//! list and check that every name in it is refused.

/// The **array functions** of the tree.
///
/// Every one of them gives back a **table** of values, and a formula
/// here is a value.
pub const ARRAY_FUNCTIONS: &[&str] = &[
    "ARRAYTOTEXT",
    "CHOOSECOLS",
    "CHOOSEROWS",
    "DROP",
    "EXPAND",
    "FILTER",
    "FREQUENCY",
    "GROUPBY",
    "GROWTH",
    "HSTACK",
    "LINEST",
    "LOGEST",
    "MODE.MULT",
    "PIVOTBY",
    "RANDARRAY",
    "SEQUENCE",
    "SORT",
    "SORTBY",
    "TAKE",
    "TEXTSPLIT",
    "TOCOL",
    "TOROW",
    "TRANSPOSE",
    "TREND",
    "UNIQUE",
    "VSTACK",
    "WRAPCOLS",
    "WRAPROWS",
];

/// Whether a name is one of the **array functions**.
pub fn is_array_function(name: &str) -> bool {
    ARRAY_FUNCTIONS.contains(&name)
}
