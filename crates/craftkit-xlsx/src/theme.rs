//! The **theme** of a workbook: the colours and the fonts it names.
//!
//! A file names a colour in three ways, and a **theme** is one of
//! them: a cell may say "the fourth colour of the theme" instead of
//! saying what colour that is. The theme is the part of the file that
//! says what the colours are — and a file without a theme has no
//! themed colour to read.
//!
//! The **twelve colours** of a theme are named in the order the files
//! name them by **index**:
//!
//! ```text
//! 0 lt1   1 dk1   2 lt2   3 dk2   4 accent1 … 9 accent6   10 hlink   11 folHlink
//! ```
//!
//! The order of the file is another one — the file holds `dk1` before
//! `lt1` — and the index is what a cell names, so the index is the
//! order of this tree.
//!
//! The **fonts** are two: the **major** font is the face of the
//! headings, and the **minor** font is the face of the text. A cell
//! that names a font by its scheme takes the name from here.

use craftkit_xml::El;

/// The names of the twelve colours, in the order the files name them
/// by **index**.
pub const SLOTS: [&str; 12] = [
    "lt1", "dk1", "lt2", "dk2", "accent1", "accent2", "accent3", "accent4", "accent5", "accent6", "hlink",
    "folHlink",
];

/// The **theme** of a workbook: the colours and the fonts it names.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Theme {
    /// The **name** of the theme, when the file names it.
    pub name: Option<String>,
    /// The **colours**, in the order the files name them by index.
    pub colours: [Option<String>; 12],
    /// The **major font**: the face of the headings.
    pub major_font: Option<String>,
    /// The **minor font**: the face of the text.
    pub minor_font: Option<String>,
}

impl Theme {
    /// The **colour** of that index of the theme.
    ///
    /// An index the theme does not have is no colour: a cell that
    /// names a colour that is not there has no colour.
    pub fn colour(&self, index: u64) -> Option<&str> {
        self.colours.get(index as usize).and_then(|colour| colour.as_deref())
    }

    /// The **font** of that **scheme**: `major` or `minor`.
    ///
    /// A scheme the theme does not name is no font.
    pub fn font(&self, scheme: &str) -> Option<&str> {
        match scheme {
            "major" => self.major_font.as_deref(),
            "minor" => self.minor_font.as_deref(),
            _ => None,
        }
    }

    /// Whether the theme says **nothing**: a workbook with no theme
    /// part has no theme.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.colours.iter().all(|colour| colour.is_none())
            && self.major_font.is_none()
            && self.minor_font.is_none()
    }
}

/// Read the **theme** part of a workbook.
pub fn read_theme(root: &El) -> Theme {
    let name = root.attr("name").map(|name| name.to_string());
    let elements = root.child("themeElements");
    let scheme = elements.and_then(|elements| elements.child("clrScheme"));
    let colours = std::array::from_fn(|index| {
        let entry = scheme.and_then(|scheme| scheme.child(SLOTS[index]))?;
        colour_of(entry)
    });
    let fonts = elements.and_then(|elements| elements.child("fontScheme"));
    let major_font = fonts.and_then(|scheme| typeface(scheme, "majorFont"));
    let minor_font = fonts.and_then(|scheme| typeface(scheme, "minorFont"));
    Theme { name, colours, major_font, minor_font }
}

/// The **colour** a scheme entry holds.
///
/// A scheme entry names its colour in one of two ways: as **hex**, or
/// as a **system** colour — the colour of the window, or the colour of
/// the text in it — and a system colour says what it was last time it
/// was drawn.
fn colour_of(element: &El) -> Option<String> {
    if let Some(hex) = element.child("srgbClr").and_then(|child| child.attr("val")) {
        return crate::styles::colour_of_hex(hex);
    }
    if let Some(system) = element.child("sysClr") {
        if let Some(hex) = system.attr("lastClr") {
            return crate::styles::colour_of_hex(hex);
        }
        return match system.attr("val") {
            Some("windowText") => Some("#000000".to_string()),
            Some("window") => Some("#ffffff".to_string()),
            _ => None,
        };
    }
    None
}

/// The **typeface** of a font of the scheme.
///
/// An empty typeface is a typeface that is not set, and it is not the
/// theme's to give.
fn typeface(scheme: &El, font: &str) -> Option<String> {
    scheme
        .child(font)
        .and_then(|font| font.child("latin"))
        .and_then(|latin| latin.attr("typeface"))
        .filter(|face| !face.is_empty())
        .map(|face| face.to_string())
}

/// The **colour** with the **tint** of the file applied.
///
/// The tint is a **fraction** between -1 and 1: a negative tint
/// darkens the colour and a positive one lightens it. The rule is the
/// rule of **ECMA-376**: the tint moves the **luminance** of the
/// colour and leaves its hue and its saturation alone.
///
/// A colour that is not a colour here is not tinted: it is the colour
/// it was.
pub fn apply_tint(colour: &str, tint: f64) -> Option<String> {
    if tint == 0.0 {
        return Some(colour.to_string());
    }
    let rgb = rgb_of(colour)?;
    let (hue, saturation, luminance) = rgb_to_hsl(rgb);
    let luminance = if tint < 0.0 { luminance * (1.0 + tint) } else { luminance * (1.0 - tint) + tint };
    let [r, g, b] = hsl_to_rgb(hue, saturation, luminance.clamp(0.0, 1.0));
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// The **bytes** of a colour of `#rrggbb`.
fn rgb_of(colour: &str) -> Option<[u8; 3]> {
    let digits = colour.trim().trim_start_matches('#');
    if digits.len() != 6 || !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let number = u32::from_str_radix(digits, 16).ok()?;
    Some([(number >> 16) as u8, (number >> 8) as u8, number as u8])
}

/// A colour as a **hue**, a **saturation** and a **luminance**, each
/// between zero and one.
fn rgb_to_hsl([r, g, b]: [u8; 3]) -> (f64, f64, f64) {
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, lightness);
    }
    let delta = max - min;
    let saturation = if lightness > 0.5 { delta / (2.0 - max - min) } else { delta / (max + min) };
    let hue = if (max - r).abs() < f64::EPSILON {
        (g - b) / delta + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (hue / 6.0, saturation, lightness)
}

/// A **hue**, a **saturation** and a **luminance** as bytes.
///
/// The hue is a **turn** — from zero to one — and the three channels
/// are a third of a turn apart.
fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> [u8; 3] {
    if saturation == 0.0 {
        let value = (lightness * 255.0).round() as u8;
        return [value, value, value];
    }
    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;
    let channel = |t: f64| {
        // A third of a turn past the end is the beginning again.
        let t = t.rem_euclid(1.0);
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (value * 255.0).round() as u8
    };
    [channel(hue + 1.0 / 3.0), channel(hue), channel(hue - 1.0 / 3.0)]
}
