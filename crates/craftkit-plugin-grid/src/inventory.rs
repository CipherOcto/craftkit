//! The **registry** of the battery's functions.
//!
//! The tree has a registry: every function it knows is in it, with the
//! number of arguments it takes and the family it belongs to. This
//! tree had no registry of its own — the engine answered a call by
//! asking each family in turn — and so there was no one place that
//! said *what this battery can do*.
//!
//! So here is that place. It is a **value**, and not a list written by
//! hand: every family of the battery declares the names it carries,
//! and this registry is those declarations gathered up. A function
//! added to a family is in the registry the next time it is asked
//! for, and a family that is not in this table is a family the
//! compiler says is never used.
//!
//! The registry is **public**, so that a front end may ask what the
//! battery knows, and so that the tree's own test may walk it and
//! check that every name in it is answered by the engine.
//!
//! # The families
//!
//! A function belongs to the family that carries it, and the family is
//! the module that answers it. Two of the families are not families of
//! functions at all: the **arrays** (§885), which give back a table
//! and are refused with it, and the **world outside** (§888) and the
//! **refused** (§893), which are answered with the reason they are not
//! here. A function that is answered with a reason is in the registry
//! like any other: it is answered.

/// The **family** of a function: the module that answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// The functions the engine itself carries.
    Core,
    /// The **aggregate** functions.
    Aggregate,
    /// The **text** functions.
    Text,
    /// The **lookup** functions.
    Lookup,
    /// The **date** functions.
    Date,
    /// The **statistical** functions.
    Statistical,
    /// The **arithmetic** functions.
    Arithmetic,
    /// The **odds and ends**.
    Odds,
    /// The **trigonometric** functions.
    Trigonometric,
    /// The **financial** functions.
    Financial,
    /// The **depreciation** functions.
    Depreciation,
    /// The **security** functions.
    Security,
    /// The **bond** functions.
    Bond,
    /// The **combination** functions.
    Combination,
    /// The **engineering** functions.
    Engineering,
    /// The **Bessel** functions.
    Bessel,
    /// The **information** functions.
    Information,
    /// The **logical** functions.
    Logical,
    /// The **regression** functions.
    Regression,
    /// The **distribution** functions.
    Distribution,
    /// The **cash flow** functions.
    CashFlow,
    /// The **word** functions.
    Word,
    /// The **database** functions.
    Database,
    /// The **tests of a hypothesis**.
    Hypothesis,
    /// The functions that **read a text**.
    TextValue,
    /// The **complex** functions.
    Complex,
    /// The **ETS** functions: the model of a series in time.
    Ets,
    /// The **matrix** functions.
    Matrix,
    /// The tree's **arrays**: functions that give back a table.
    Array,
    /// The **world outside the workbook**.
    Outside,
    /// The functions that are **answered with a reason**.
    Refused,
    /// The **host's** functions: the ones the tree's program carries
    /// and its registry does not.
    Host,
}

impl Family {
    /// The **name** of the family, in prose.
    pub fn name(self) -> &'static str {
        match self {
            Family::Core => "core",
            Family::Aggregate => "aggregate",
            Family::Text => "text",
            Family::Lookup => "lookup",
            Family::Date => "date",
            Family::Statistical => "statistical",
            Family::Arithmetic => "arithmetic",
            Family::Odds => "odds and ends",
            Family::Trigonometric => "trigonometric",
            Family::Financial => "financial",
            Family::Depreciation => "depreciation",
            Family::Security => "security",
            Family::Bond => "bond",
            Family::Combination => "combination",
            Family::Engineering => "engineering",
            Family::Bessel => "Bessel",
            Family::Information => "information",
            Family::Logical => "logical",
            Family::Regression => "regression",
            Family::Distribution => "distribution",
            Family::CashFlow => "cash flow",
            Family::Word => "word",
            Family::Database => "database",
            Family::Hypothesis => "hypothesis",
            Family::TextValue => "text value",
            Family::Complex => "complex",
            Family::Ets => "ETS",
            Family::Matrix => "matrix",
            Family::Array => "array",
            Family::Outside => "outside",
            Family::Refused => "refused",
            Family::Host => "host",
        }
    }
}

/// The **names** of each family.
///
/// Every family of the battery is in this table, and a family that is
/// not in it is a family the compiler says is never used: the table is
/// what holds the tree's promise that a name is answered by exactly
/// one family.
const FAMILIES: &[(Family, &[&str])] = &[
    (Family::Core, crate::formula::NAMES),
    (Family::Aggregate, crate::aggregates::NAMES),
    (Family::Text, crate::texts::NAMES),
    (Family::Lookup, crate::lookups::NAMES),
    (Family::Date, crate::datetimes::NAMES),
    (Family::Statistical, crate::statistics::NAMES),
    (Family::Arithmetic, crate::arithmetic::NAMES),
    (Family::Odds, crate::odds::NAMES),
    (Family::Trigonometric, crate::trigonometry::NAMES),
    (Family::Financial, crate::finance::NAMES),
    (Family::Depreciation, crate::depreciation::NAMES),
    (Family::Security, crate::securities::NAMES),
    (Family::Bond, crate::bonds::NAMES),
    (Family::Combination, crate::combinations::NAMES),
    (Family::Engineering, crate::engineering::NAMES),
    (Family::Bessel, crate::bessels::NAMES),
    (Family::Information, crate::information::NAMES),
    (Family::Logical, crate::logic::NAMES),
    (Family::Regression, crate::regression::NAMES),
    (Family::Distribution, crate::distributions::NAMES),
    (Family::CashFlow, crate::cashflows::NAMES),
    (Family::Word, crate::words::NAMES),
    (Family::Database, crate::database::NAMES),
    (Family::Hypothesis, crate::hypotheses::NAMES),
    (Family::TextValue, crate::textvalue::NAMES),
    (Family::Complex, crate::complex::NAMES),
    (Family::Ets, crate::ets::NAMES),
    (Family::Matrix, crate::matrices::NAMES),
    (Family::Array, crate::arrays::ARRAY_FUNCTIONS),
    (Family::Host, crate::host::NAMES),
];

/// The **family** that carries a name, when one does.
///
/// The name is carried by exactly one family, and the tree's own test
/// says so: a name in two families is a name the order of the dispatch
/// would decide, and this tree does not decide things by order.
pub fn family_of(name: &str) -> Option<Family> {
    if let Some((family, _)) = FAMILIES.iter().find(|(_, names)| names.contains(&name)) {
        return Some(*family);
    }
    if crate::outside::reason_for(name).is_some() {
        return Some(Family::Outside);
    }
    if crate::refused::reason_for(name).is_some() {
        return Some(Family::Refused);
    }
    if crate::host::reason_for(name).is_some() {
        return Some(Family::Host);
    }
    None
}

/// Every function the battery knows, and the family that carries it.
///
/// The list is **derived** from the families, and it is gathered at
/// the moment it is asked for: a name that a family adds is in here
/// the next time this is called, and not before.
pub fn functions() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for (family, names) in FAMILIES {
        for name in *names {
            out.push((*name, family.name()));
        }
    }
    for (name, _) in crate::outside::OUTSIDE_FUNCTIONS {
        out.push((*name, Family::Outside.name()));
    }
    for (name, _) in crate::refused::REFUSED_FUNCTIONS {
        out.push((*name, Family::Refused.name()));
    }
    for (name, _) in crate::host::REFUSED {
        out.push((*name, Family::Host.name()));
    }
    out
}

/// Whether the battery knows a **name**.
pub fn knows(name: &str) -> bool {
    family_of(name).is_some()
}
