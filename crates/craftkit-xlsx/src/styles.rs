//! The **styles** of a workbook: the format of each cell.
//!
//! A workbook has a part that says what its cells look like: the
//! **styles** part. It holds
//!
//! - the **number formats** the workbook names by their own number —
//!   the code of each, and the number it has;
//! - the **cell formats**, each of which says which number format its
//!   cell is written with.
//!
//! A cell names one of them by its **style** — the `s` attribute of
//! the cell — and so the format of a cell is found by two steps: the
//! style of the cell, and the number format of that style.
//!
//! # The built-in formats
//!
//! Some formats have no number of their own: they are **built in**, and
//! their number says which one they are. The table is the tree's own
//! table, and the numbers that are not in it — the East Asian ones —
//! read as the general format.
//!
//! # What this does not do
//!
//! - It does not read the **fonts**, the **fills** or the **borders**:
//!   this reader gives back the shape of a workbook, and the shape is
//!   the values and their formats.
//! - It does not decide what a format **means**: the battery that owns
//!   the spreadsheet model has its own format language, and it is the
//!   one that reads a code.

use craftkit_xml::El;

/// The **built-in** number formats, by the number they have.
///
/// The tree's own table, in the tree's own order. The numbers that are
/// not in it — 23 to 36, which are East Asian locale formats — read as
/// the general format.
const BUILTIN_FORMATS: [(u32, &str); 36] = [
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (5, "\"$\"#,##0_);\\(\"$\"#,##0\\)"),
    (6, "\"$\"#,##0_);[Red]\\(\"$\"#,##0\\)"),
    (7, "\"$\"#,##0.00_);\\(\"$\"#,##0.00\\)"),
    (8, "\"$\"#,##0.00_);[Red]\\(\"$\"#,##0.00\\)"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "m/d/yyyy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yyyy h:mm"),
    (37, "#,##0 ;(#,##0)"),
    (38, "#,##0 ;[Red](#,##0)"),
    (39, "#,##0.00;(#,##0.00)"),
    (40, "#,##0.00;[Red](#,##0.00)"),
    (41, "_(* #,##0_);_(* \\(#,##0\\);_(* \"-\"_);_(@_)"),
    (42, "_(\"$\"* #,##0_);_(\"$\"* \\(#,##0\\);_(\"$\"* \"-\"_);_(@_)"),
    (43, "_(* #,##0.00_);_(* \\(#,##0.00\\);_(* \"-\"??_);_(@_)"),
    (44, "_(\"$\"* #,##0.00_);_(\"$\"* \\(#,##0.00\\);_(\"$\"* \"-\"??_);_(@_)"),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mmss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
];

/// The format of the **general** format: the value as it is.
const GENERAL: &str = "General";

/// The **styles** of a workbook: the format of each cell, by the
/// number of the cell format.
#[derive(Debug, Clone, Default)]
pub struct Styles {
    /// The format of each cell format, in the order the file holds
    /// them. The first one is the normal format, and every cell with
    /// no style of its own has it.
    formats: Vec<String>,
}

impl Styles {
    /// The **format** of the style of that number.
    ///
    /// A style the file does not have has no format: the general
    /// format is the format of a cell that names nothing.
    pub fn format(&self, style: Option<u32>) -> Option<&str> {
        let index = style? as usize;
        let code = self.formats.get(index)?;
        if code.eq_ignore_ascii_case(GENERAL) { None } else { Some(code.as_str()) }
    }

    /// Whether there are **no** styles at all: a workbook with no
    /// styles part has no formats.
    pub fn is_empty(&self) -> bool {
        self.formats.is_empty()
    }
}

/// Read the **styles** part of a workbook.
pub fn read_styles(root: &El) -> Styles {
    // The **custom** number formats: the ones the workbook names by
    // their own number.
    let mut custom: Vec<(u32, String)> = Vec::new();
    if let Some(formats) = root.child("numFmts") {
        for format in formats.kids("numFmt") {
            let Some(id) = format.attr_u32("numFmtId") else { continue };
            let Some(code) = format.attr("formatCode") else { continue };
            custom.push((id, code.to_string()));
        }
    }
    // The **cell formats**: each says which number format its cell is
    // written with.
    let mut formats = Vec::new();
    if let Some(cells) = root.child("cellXfs") {
        for format in cells.kids("xf") {
            let id = format.attr_u32("numFmtId").unwrap_or(0);
            let code = custom
                .iter()
                .find(|(number, _)| *number == id)
                .map(|(_, code)| code.clone())
                .or_else(|| builtin_format(id).map(|code| code.to_string()))
                .unwrap_or_else(|| GENERAL.to_string());
            formats.push(code);
        }
    }
    Styles { formats }
}

/// The **built-in** format of that number, if it has one.
fn builtin_format(id: u32) -> Option<&'static str> {
    BUILTIN_FORMATS.iter().find(|(number, _)| *number == id).map(|(_, code)| *code)
}
