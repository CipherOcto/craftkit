//! The **relationships** of a part: which part is which.
//!
//! A package does not say what its parts are for. It says it in a
//! **relationship**: a part that holds the relationships of another
//! part is beside it, in a `_rels` directory, and named after it.
//!
//! - the relationships of the **package** itself are in `_rels/.rels`;
//! - the relationships of `xl/workbook.xml` are in
//!   `xl/_rels/workbook.xml.rels`.
//!
//! A relationship has an **id** — the name a part uses to point at it —
//! a **kind** — the last word of its type, which says what it is
//! (`worksheet`, `styles`, `sharedStrings`, `officeDocument`) — and a
//! **target**: the part it points at, or the outside thing it points at
//! when it is external.

use craftkit_zip::Package;

use crate::Error;

/// A **relationship**: a part, and what it is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rel {
    /// The **id** of the relationship: the name a part points at it by.
    pub id: String,
    /// The **kind**: the last word of the type, which says what it is.
    pub kind: String,
    /// The part it points at, resolved against the part that holds it.
    pub target: String,
    /// Whether it points at something **outside** the package.
    pub external: bool,
}

/// The relationships of a part.
///
/// A part that has no relationships — or whose relationships are
/// broken — has none, and not an error: a package may be written by a
/// program that leaves a part out.
pub fn rels_of(package: &mut Package<'_>, part: &str) -> Result<Vec<Rel>, Error> {
    let (dir, file) = split_dir(part);
    let name = if dir.is_empty() { format!("_rels/{file}.rels") } else { format!("{dir}/_rels/{file}.rels") };
    let Some(bytes) = package.read(&name).map_err(|error| Error::Package(error.to_string()))? else {
        return Ok(Vec::new());
    };
    let Ok(root) = craftkit_xml::parse(&bytes) else {
        return Ok(Vec::new());
    };
    Ok(root
        .kids("Relationship")
        .filter_map(|element| {
            let id = element.attr("Id")?.to_string();
            let kind = element.attr("Type").unwrap_or("").rsplit('/').next().unwrap_or("").to_string();
            let raw = element.attr("Target")?;
            let external =
                element.attr("TargetMode").is_some_and(|mode| mode.eq_ignore_ascii_case("external"));
            let target = if external { raw.to_string() } else { resolve(dir, raw) };
            Some(Rel { id, kind, target, external })
        })
        .collect())
}

/// The directory and the file of a part: `xl/worksheets/sheet1.xml`
/// gives `("xl/worksheets", "sheet1.xml")`.
pub fn split_dir(part: &str) -> (&str, &str) {
    let trimmed = part.trim_start_matches('/');
    match trimmed.rfind('/') {
        Some(at) => (trimmed.get(..at).unwrap_or(""), trimmed.get(at + 1..).unwrap_or("")),
        None => ("", trimmed),
    }
}

/// Resolve a **target** against the directory of the part that holds
/// it.
///
/// A target may be absolute — it starts with a slash — or relative, and
/// a relative target may walk up with `..`.
pub fn resolve(dir: &str, target: &str) -> String {
    let target = target.replace('\\', "/");
    let target = target.split('#').next().unwrap_or("").to_string();
    let mut segments: Vec<&str> = if target.starts_with('/') {
        Vec::new()
    } else {
        dir.split('/').filter(|part| !part.is_empty()).collect()
    };
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}
