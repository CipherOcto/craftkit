//! The **package**: a ZIP archive, and the parts in it.

use craftkit_zip::{Error, Package, crc32, normalize};

/// A **part** of a package, as it is written.
struct Part {
    /// The name of the part.
    name: String,
    /// The bytes of the part.
    data: Vec<u8>,
    /// Whether the part is compressed.
    compressed: bool,
}

/// A package of those parts, written by hand.
///
/// The tests of a reader are the packages that were written by somebody
/// else — and here that somebody is this file: the parts are laid out
/// as the format says, and the compressed ones are compressed by the
/// tree's own compressor.
fn a_package(parts: &[Part]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for part in parts {
        let name = part.name.as_bytes();
        let (method, body) =
            if part.compressed { (8u16, deflate(&part.data)) } else { (0u16, part.data.clone()) };
        let checksum = crc32(&part.data);
        let offset = out.len() as u32;
        // The **local header**.
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&checksum.to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&(part.data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&body);
        // The **directory entry**.
        directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&method.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&checksum.to_le_bytes());
        directory.extend_from_slice(&(body.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(part.data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }
    let directory_at = out.len() as u32;
    out.extend_from_slice(&directory);
    // The **end of the directory**.
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    out.extend_from_slice(&directory_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// A part of a package.
fn a_part(name: &str, data: &[u8]) -> Part {
    Part { name: name.to_string(), data: data.to_vec(), compressed: false }
}

/// A **compressed** part of a package.
fn a_deflated_part(name: &str, data: &[u8]) -> Part {
    Part { name: name.to_string(), data: data.to_vec(), compressed: true }
}

/// A DEFLATE stream of those bytes, by the tree's own compressor.
fn deflate(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("the bytes are written");
    encoder.finish().expect("the stream is finished")
}

/// A part that is **stored** comes back as it was.
#[test]
fn a_stored_part_comes_back() {
    let package = a_package(&[a_part("xl/workbook.xml", b"<workbook/>")]);
    let mut package = Package::open(&package).expect("the package opens");
    let part = package.read("xl/workbook.xml").expect("the part is read");
    assert_eq!(part, Some(b"<workbook/>".to_vec()));
}

/// A part that is **compressed** comes back as it was.
#[test]
fn a_compressed_part_comes_back() {
    let text = b"the quick brown fox jumps over the lazy dog";
    let package = a_package(&[a_deflated_part("xl/sharedStrings.xml", text)]);
    let mut package = Package::open(&package).expect("the package opens");
    let part = package.read("xl/sharedStrings.xml").expect("the part is read");
    assert_eq!(part, Some(text.to_vec()));
}

/// A part that is **not there** is nothing, and not an error.
#[test]
fn a_part_that_is_not_there_is_nothing() {
    let package = a_package(&[a_part("a.xml", b"a")]);
    let mut package = Package::open(&package).expect("the package opens");
    assert_eq!(package.read("b.xml").expect("the part is looked for"), None);
    assert!(!package.has("b.xml"), "the package does not have it");
    assert!(package.has("a.xml"), "and it does have the other one");
}

/// The **names** of the parts are the names of the package.
#[test]
fn the_names_of_the_parts_are_the_names_of_the_package() {
    let package = a_package(&[a_part("a.xml", b"a"), a_part("xl/b.xml", b"b")]);
    let package = Package::open(&package).expect("the package opens");
    assert_eq!(package.names(), vec!["a.xml", "xl/b.xml"]);
    assert!(package.has("a.xml"), "the package has the first one");
}

/// The names of a package are **normalised**: a part may be asked for
/// by a name that is not its own.
#[test]
fn the_names_are_normalised() {
    let package = a_package(&[a_part("xl/workbook.xml", b"a")]);
    let package = Package::open(&package).expect("the package opens");
    assert!(package.has("/xl/workbook.xml"), "a leading slash is nothing");
    assert!(package.has("XL/WORKBOOK.XML"), "and the case is not the case");
    assert_eq!(normalize("XL\\Workbook.XML"), "xl/workbook.xml", "the name is normalised");
}

/// Bytes that are **not a package** are refused.
#[test]
fn bytes_that_are_not_a_package_are_refused() {
    let error = Package::open(b"not a package at all").expect_err("that is not a package");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// A part whose **checksum** does not match is refused: the bytes
/// were damaged.
#[test]
fn a_part_with_a_wrong_checksum_is_refused() {
    let mut bytes = a_package(&[a_part("a.xml", b"the bytes")]);
    // The checksum of the part is in the local header and in the
    // directory; one of them is changed.
    let at = bytes.windows(4).position(|four| four == [0x50, 0x4b, 0x03, 0x04]).unwrap_or(0);
    bytes[at + 14] = bytes[at + 14].wrapping_add(1);
    let mut package = Package::open(&bytes).expect("the package opens");
    let error = package.read("a.xml").expect_err("the checksum does not match");
    assert!(matches!(error, Error::Malformed(_)), "{error}");
}

/// A part that says it is **too big** is refused, and the limit is
/// the caller's.
#[test]
fn a_part_that_is_too_big_is_refused() {
    let package = a_package(&[a_part("a.xml", b"the bytes")]);
    let mut package = Package::open(&package).expect("the package opens");
    let error = package.read_limited("a.xml", 2).expect_err("the part is too big");
    assert!(matches!(error, Error::TooLarge(_)), "{error}");
}

/// A package with **two** parts holds both.
#[test]
fn a_package_with_two_parts_holds_both() {
    let package = a_package(&[a_part("a.xml", b"a"), a_deflated_part("b.xml", b"b")]);
    let mut package = Package::open(&package).expect("the package opens");
    assert_eq!(package.read("a.xml").expect("the part is read"), Some(b"a".to_vec()));
    assert_eq!(package.read("b.xml").expect("the part is read"), Some(b"b".to_vec()));
}

/// The **checksum** of the bytes is the one the format says.
#[test]
fn the_checksum_of_the_bytes_is_the_one_the_format_says() {
    assert_eq!(crc32(b""), 0x0000_0000, "nothing");
    assert_eq!(crc32(b"a"), 0xE8B7_BE43, "one letter");
    assert_eq!(crc32(b"123456789"), 0xCBF4_3926, "the check value of the format");
}
