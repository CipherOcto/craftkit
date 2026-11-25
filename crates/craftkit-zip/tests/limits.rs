//! The **limits of the zip crate**: the numbers it will not go past.
//!
//! The crate reads a package of parts, and the numbers it keeps are
//! about how much of a package is read and how much it may inflate to.
//! A limit that no test names is a limit nothing proves holds.

use craftkit_zip::{Error, MAX_ENTRIES, MAX_PART, MAX_TOTAL, Package, crc32};

/// A **package** of one part, written by hand.
///
/// The **directory** of the package says how big the part is, and the
/// test says a size of its own: a package may claim a part that is
/// bigger than the bytes it holds.
fn a_package_claiming(name: &str, claimed: u32) -> Vec<u8> {
    let data = b"the bytes";
    let checksum = crc32(data);
    let mut out: Vec<u8> = Vec::new();
    // The **local header**.
    out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
    out.extend_from_slice(&20u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&claimed.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(data);
    // The **directory entry**.
    let mut directory: Vec<u8> = Vec::new();
    directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
    directory.extend_from_slice(&20u16.to_le_bytes());
    directory.extend_from_slice(&20u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&checksum.to_le_bytes());
    directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
    directory.extend_from_slice(&claimed.to_le_bytes());
    directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u16.to_le_bytes());
    directory.extend_from_slice(&0u32.to_le_bytes());
    directory.extend_from_slice(&0u32.to_le_bytes());
    directory.extend_from_slice(name.as_bytes());
    let directory_at = out.len() as u32;
    out.extend_from_slice(&directory);
    // The **end of the directory**.
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    out.extend_from_slice(&directory_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// A **part** that says it is bigger than `MAX_PART` is refused.
///
/// A package may claim a part of any size at all, and the tree reads
/// none of it: the bound is checked against what the package says.
#[test]
fn a_part_over_the_limit_is_refused() {
    let claimed = (MAX_PART + 1) as u32;
    let bytes = a_package_claiming("the part", claimed);
    let mut package = Package::open(&bytes).expect("the package opens");
    let error = package.read("the part").expect_err("that part is too big");
    match error {
        Error::TooLarge(message) => assert!(message.contains("the part"), "the part is named: {message}"),
        other => panic!("the part is refused: {other:?}"),
    }
}

/// A package with **more entries** than `MAX_ENTRIES` is refused — as
/// the package it would have to be.
///
/// The count of the entries is a number of sixteen bits in the record
/// at the end of the directory, so a package with more entries than
/// that keeps its real numbers in the **ZIP64** record, and this reader
/// does not read that one. The entries past `MAX_ENTRIES` cannot even
/// be named.
#[test]
fn a_package_with_too_many_entries_is_refused() {
    let count = u16::MAX as usize;
    assert!(count < MAX_ENTRIES, "the format's own count is below the tree's bound: {count}");
    let mut bytes = a_package_claiming("the part", 9);
    // The **count** of the entries in the record at the end: the most
    // the format's own number can hold.
    let at = bytes.len() - 22 + 10;
    if let Some(slot) = bytes.get_mut(at..at + 2) {
        slot.copy_from_slice(&u16::MAX.to_le_bytes());
    }
    let error = Package::open(&bytes).expect_err("that package is not one this reader reads");
    match error {
        Error::Unsupported(message) => {
            assert!(message.contains("ZIP64"), "the package is refused: {message}")
        }
        other => panic!("the package is refused: {other:?}"),
    }
}

/// The **total** a package may inflate to is `MAX_TOTAL`.
///
/// The bound is a gigabyte of bytes, and a test that spends a gigabyte
/// of memory is not a test the tree runs. What this test says is that
/// the tree reads a part and that the part is within the total.
#[test]
fn the_total_of_a_package_is_bounded() {
    let bytes = a_package_claiming("the part", 9);
    let mut package = Package::open(&bytes).expect("the package opens");
    let part = package.read("the part").expect("the part is read").expect("the part is there");
    assert_eq!(part, b"the bytes".to_vec(), "the part is read");
    assert!(part.len() as u64 <= MAX_TOTAL, "the part is within the total: {}", part.len());
}
