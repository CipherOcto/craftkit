//! The **feature property bags** of a workbook: the cell controls a
//! file holds beside its cells.
//!
//! A **checkbox** is a cell control, and a file does not write it in
//! the cell: it writes it in a **bag**, and the cell's style names the
//! bag. The chain a file writes is four bags deep, and every step
//! points at an earlier one:
//!
//! ```text
//! bag type="Checkbox"
//! bag type="XFControls"   bagId k="CellControl"  -> 0
//! bag type="XFComplement"  bagId k="XFControls"   -> 1
//! bag type="XFComplements" a    k="MappedFeaturePropertyBags"
//!                                     -> 2, 3, ...
//! ```
//!
//! The style of a cell carries an `xfComplement` that names its place
//! in that list, and a place that resolves to a **Checkbox** bag is a
//! cell that is a checkbox.
//!
//! A chain that points at a bag that is not there, or at a bag of
//! another kind, or at itself, resolves to **nothing**: half a chain
//! is not a checkbox, and a broken file is not a file to guess at.
//!
//! The bags are read against a bound: a file may name **100 000**
//! complements, and the rest are not read — and the file is **told**
//! that they were not.

use craftkit_xml::El;

/// The **uri** of the extension on a cell format that names a
/// complement.
pub const XF_EXT_URI: &str = "{C7286773-470A-42A8-94C5-96B5CB345126}";

/// The most **bags** a file may name: the tree's own bound.
pub const MAX_BAGS: usize = 100_000;

/// The **checkboxes** of a workbook: for each complement a file names,
/// whether it resolves to a checkbox.
///
/// A file with no feature property bags has no complements, and a file
/// whose list of them is not there has none either.
///
/// The bags are read against a bound, and a file that names more than
/// the bound is **told** that the rest were not read.
pub fn checkbox_complements(root: &El, warnings: &mut Vec<String>) -> Vec<bool> {
    let mut bags: Vec<&El> = Vec::new();
    let mut more = false;
    for bag in root.kids("bag") {
        if bags.len() >= MAX_BAGS {
            more = true;
            break;
        }
        bags.push(bag);
    }
    if more {
        warnings.push(format!(
            "the file names more than {MAX_BAGS} feature property bags; the rest were not read"
        ));
    }
    // The bag of that number, when it is a bag of that kind.
    let bag = |number: Option<u32>, kind: &str| {
        number
            .and_then(|number| bags.get(number as usize).copied())
            .filter(|bag| bag.attr("type") == Some(kind))
    };
    // The number a bag names by a key: `bagId k="XFControls" -> 1`.
    let key = |bag: &El, name: &str| {
        bag.kids("bagId")
            .find(|id| id.attr("k") == Some(name))
            .and_then(|id| id.text.trim().parse::<u32>().ok())
    };
    let Some(map) = bags.iter().find(|bag| bag.attr("type") == Some("XFComplements")) else {
        return Vec::new();
    };
    let Some(list) = map.kids("a").find(|a| a.attr("k") == Some("MappedFeaturePropertyBags")) else {
        return Vec::new();
    };
    let mut ids: Vec<&El> = Vec::new();
    let mut more_ids = false;
    for id in list.kids("bagId") {
        if ids.len() >= MAX_BAGS {
            more_ids = true;
            break;
        }
        ids.push(id);
    }
    if more_ids {
        warnings.push(format!(
            "the file names more than {MAX_BAGS} feature property bag places; the rest were not read"
        ));
    }
    ids.into_iter()
        .map(|id| {
            // The complement this place names, then the controls it
            // names, then the control itself.
            let complement = bag(id.text.trim().parse::<u32>().ok(), "XFComplement");
            let controls = complement.and_then(|complement| bag(key(complement, "XFControls"), "XFControls"));
            controls.and_then(|controls| bag(key(controls, "CellControl"), "Checkbox")).is_some()
        })
        .collect()
}

/// Whether a cell format carries a complement that is a **checkbox**.
pub fn xf_is_checkbox(xf: &El, complements: &[bool]) -> bool {
    let Some(list) = xf.child("extLst") else { return false };
    list.kids("ext")
        .filter(|ext| ext.attr("uri").is_some_and(|uri| uri.eq_ignore_ascii_case(XF_EXT_URI)))
        .filter_map(|ext| ext.child("xfComplement"))
        .filter_map(|complement| complement.attr_u32("i"))
        .any(|number| complements.get(number as usize).copied().unwrap_or(false))
}
