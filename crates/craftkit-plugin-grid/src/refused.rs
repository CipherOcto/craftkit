//! The tree's functions that are **answered with a reason**.
//!
//! Two families of the tree's language are not here, and each of them
//! is refused with the reason it is not:
//!
//! - the **regular expressions** — `REGEXTEST`, `REGEXEXTRACT` and
//!   `REGEXREPLACE` — need an **engine** for patterns, and a formula
//!   language in this tree has no engine in it;
//! - the **cube** — `CUBEMEMBER`, `CUBESET`, `CUBEVALUE` and the
//!   rest of it — needs a **connection** to a cube of data, and a
//!   workbook in this tree is a document and not a connection.
//!
//! A function that is refused with a reason is not missing — it is
//! answered.
//!
//! The table is **public** so that the tree's own test can walk it and
//! check that every name in it is refused and says why.

/// The functions that are **answered with a reason**, and the reason
/// each one is.
pub const REFUSED_FUNCTIONS: &[(&str, &str)] = &[
    ("REGEXTEST", "not supported yet: a regular expression"),
    ("REGEXEXTRACT", "not supported yet: a regular expression"),
    ("REGEXREPLACE", "not supported yet: a regular expression"),
    ("CUBEMEMBER", "not supported yet: a cube"),
    ("CUBEMEMBERPROPERTY", "not supported yet: a cube"),
    ("CUBERANKEDMEMBER", "not supported yet: a cube"),
    ("CUBESET", "not supported yet: a cube"),
    ("CUBESETCOUNT", "not supported yet: a cube"),
    ("CUBEVALUE", "not supported yet: a cube"),
    ("CUBEKPIMEMBER", "not supported yet: a cube"),
];

/// The reason a name is refused, when it is one of the **refused**
/// functions.
pub fn reason_for(name: &str) -> Option<&'static str> {
    REFUSED_FUNCTIONS.iter().find(|(function, _)| *function == name).map(|(_, reason)| *reason)
}
