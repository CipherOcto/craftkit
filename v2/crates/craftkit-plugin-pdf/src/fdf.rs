//! The **FDF file** of the data of a form: the values of the fields, in
//! the syntax of PDF.
//!
//! The tree's own file: the version line of the FDF, one object, and a
//! trailer that names it as the root.
//!
//! # The rules
//!
//! - The file is a file of **PDF syntax**: the header, the object, the
//!   trailer and the end.
//! - The **fields** of the form are written in the `/Fields` array, and
//!   each one is a dictionary with its **name** (`/T`) and its **value**
//!   (`/V`).
//! - The **value** of a **button** is a **name** — the state it holds,
//!   or `Off` when it holds none — and the value of every other field
//!   is a **string**.
//! - A **push button** is left out: it holds nothing, and the tree
//!   leaves it out too.
//!
//! # What it does not settle
//!
//! - The **comments** of the tree's FDF are not here: this carries the
//!   values of the fields and not the annotations.
//! - The **appearances** of the fields are not carried: the file holds
//!   the values and nothing else.
//! - The file is **not read back** here: this tree writes the FDF and
//!   does not yet read one.

use crate::fields::{Field, FieldKind};

/// The **flag** of a **push button**: the sixteenth bit.
const PUSH_BUTTON: u32 = 1 << 16;

/// A **text** of a file, as the file writes it.
fn text(value: &str) -> String {
    format!("({})", crate::writer::escape_text(value))
}

/// The **value** of a field, as the file writes it.
///
/// The tree's own rule: a **button** holds a **state**, and a state is
/// a name; every other field holds a **string**.
fn value_of(field: &Field) -> String {
    match field.kind {
        FieldKind::Button => {
            let value = field.value.clone().unwrap_or_else(|| "Off".to_string());
            format!("/{}", crate::writer::escape_name(&value))
        }
        _ => text(field.value.as_deref().unwrap_or("")),
    }
}

/// Whether the field is a **push button**: it holds nothing.
fn is_push_button(field: &Field) -> bool {
    field.kind == FieldKind::Button && (field.flags as u32) & PUSH_BUTTON != 0
}

/// The **FDF file** of the data of the fields, and how many fields it
/// holds.
///
/// The tree's own file: the header of the FDF, one object with the
/// dictionary of the data in it, and the trailer that names the object
/// as the root.
pub fn fdf_of(fields: &[Field], file: &str) -> (Vec<u8>, usize) {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%FDF-1.2\n%\xE2\xE3\xCF\xD3\n1 0 obj\n");
    out.extend_from_slice(b"<< /FDF << /Fields [");
    let mut count = 0usize;
    for field in fields {
        if is_push_button(field) {
            continue;
        }
        out.extend_from_slice(b"<< /T ");
        out.extend_from_slice(text(&field.name).as_bytes());
        out.extend_from_slice(b" /V ");
        out.extend_from_slice(value_of(field).as_bytes());
        out.extend_from_slice(b" >> ");
        count += 1;
    }
    out.extend_from_slice(b"] ");
    out.extend_from_slice(b"/F ");
    out.extend_from_slice(text(file).as_bytes());
    out.extend_from_slice(b" >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n");
    (out, count)
}
