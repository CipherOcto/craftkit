//! The **world outside the workbook**.
//!
//! The tree's spreadsheet has a handful of functions that ask the
//! world something: what is the time (`NOW`, `TODAY`), give me a
//! random number (`RAND`, `RANDBETWEEN`), go and fetch a page
//! (`WEBSERVICE`), read an XML document (`FILTERXML`), and tell me
//! about the program I am running in (`INFO`).
//!
//! A formula here is a **value**, and a value that changes every time
//! it is read is not derived — so the tree's own reason for refusing
//! these is the tree's reason for refusing an **array** (§885): the
//! world is not part of the document.
//!
//! Each of them is refused with **its own** reason, because the
//! reasons are not the same: a clock is not a random number, and a
//! network is not an XML reader. A function that is refused with a
//! reason is not missing — it is answered.
//!
//! The table is **public** so that the tree's own test can walk it and
//! check that every name in it is refused and says why.

/// The **world-outside functions** of the tree, and the reason each
/// one is refused.
///
/// The name is the tree's own; the reason is this tree's.
pub const OUTSIDE_FUNCTIONS: &[(&str, &str)] = &[
    ("NOW", "not supported yet: the clock"),
    ("TODAY", "not supported yet: the clock"),
    ("RAND", "not supported yet: a random number"),
    ("RANDBETWEEN", "not supported yet: a random number"),
    ("WEBSERVICE", "not supported yet: the network"),
    ("FILTERXML", "not supported yet: an XML reader"),
    ("INFO", "not supported yet: the environment of the program"),
];

/// The reason a name is refused, when it is one of the **world-outside
/// functions**.
pub fn reason_for(name: &str) -> Option<&'static str> {
    OUTSIDE_FUNCTIONS.iter().find(|(function, _)| *function == name).map(|(_, reason)| *reason)
}
