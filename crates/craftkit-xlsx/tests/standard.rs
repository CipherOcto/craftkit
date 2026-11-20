//! A workbook of the **old encryption** is read: the Standard scheme
//! of [MS-OFFCRYPTO] §2.3.4, with AES and with RC4.
//!
//! The files of these tests are built here, and the **values** of the
//! two tests that use a real workbook are the ones the specification's
//! own worked example gives.

use aes::cipher::{BlockEncryptMut, KeyInit, generic_array::GenericArray};
use sha1::{Digest, Sha1};

use craftkit_xlsx::cfb::Compound;
use craftkit_xlsx::crypto;
use craftkit_xlsx::standard;

/// The **password** of the file the tests make.
const PASSWORD: &str = "the password";

/// The **password** of the real workbook the values below are of.
const REAL_PASSWORD: &str = "Password1234_";

/// The **signature** of a compound file.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The end of a **chain**.
const END: u32 = 0xFFFF_FFFF;

/// The size of a **sector**.
const SECTOR: usize = 512;

/// The size of a **mini sector**.
const MINI: usize = 64;

/// The **turns of the spin** the old scheme always does.
const SPIN: u32 = 50_000;

/// The **encryption info** of the file that another program wrote with AES128.
const INFO_AES128: &str = "AwACACQAAACMAAAAJAAAAAAAAAAOZgAABIAAAIAAAAAYAAAAAAAAAAAAAABNAGkAYwByAG8AcwBvAGYAdAAgAEUAbgBoAGEAbgBjAGUAZAAgAFIAUwBBACAAYQBuAGQAIABBAEUAUwAgAEMAcgB5AHAAdABvAGcAcgBhAHAAaABpAGMAIABQAHIAbwB2AGkAZABlAHIAAAAQAAAAAAECAwQFBgcICQoLDA0OD31WgEyM1C2OrNcoLt9RGvMUAAAAUQfivYUtjL0soO1IhGVnX7m+h6h6Sjs3AocEC44EHms=";

/// The **encrypted package** of that file.
const PACKAGE_AES128: &str = "JQAAAAAAAAAqtM88ollQ6bwgfh7P/igi4ZPtDLKxRpkiwFwyNo53UXvGuMKhB6x1b2/1Dw8Dc/I=";

/// The **encryption info** of the file that another program wrote with AES256.
const INFO_AES256: &str = "AwACACQAAACMAAAAJAAAAAAAAAAQZgAABIAAAAABAAAYAAAAAAAAAAAAAABNAGkAYwByAG8AcwBvAGYAdAAgAEUAbgBoAGEAbgBjAGUAZAAgAFIAUwBBACAAYQBuAGQAIABBAEUAUwAgAEMAcgB5AHAAdABvAGcAcgBhAHAAaABpAGMAIABQAHIAbwB2AGkAZABlAHIAAAAQAAAAAAECAwQFBgcICQoLDA0OD2KBwuaMZVsKkSD7j2PYfjQUAAAAlcAvSfiF7ShSphs+gJ3yneOzimGcZYduR1s96d3alsE=";

/// The **encrypted package** of that file.
const PACKAGE_AES256: &str = "JQAAAAAAAADkf6cREqzAvNglWqoyTVjxmd5XzuFF8Ohcl2seINTIEWIDRCO1B39yqoXiJBuUwVs=";

/// The **encryption info** of the file that another program wrote with RC4.
const INFO_RC4: &str = "AwACAAQAAACMAAAABAAAAAAAAAABaAAABIAAACgAAAAYAAAAAAAAAAAAAABNAGkAYwByAG8AcwBvAGYAdAAgAEUAbgBoAGEAbgBjAGUAZAAgAFIAUwBBACAAYQBuAGQAIABBAEUAUwAgAEMAcgB5AHAAdABvAGcAcgBhAHAAaABpAGMAIABQAHIAbwB2AGkAZABlAHIAAAAQAAAAAAECAwQFBgcICQoLDA0OD4vLctKAvLCRHbT9OS8EHVcUAAAAUc7txHzcGj7JSRoOH2sItcQRkfg=";

/// The **encrypted package** of that file.
const PACKAGE_RC4: &str = "JQAAAAAAAADvsgXh5MjF7WTKggJcfyM8ghDEpn1vzCIy+9aPCdOynX4RS5/A";

/// The **salt** of the key of a real workbook.
const SALT_KEY: [u8; 16] =
    [0xe8, 0x82, 0x66, 0x49, 0x0c, 0x5b, 0xd1, 0xee, 0xbd, 0x2b, 0x43, 0x94, 0xe3, 0xf8, 0x30, 0xef];

/// The **key** a real workbook's password gives.
const EXPECTED_KEY: [u8; 16] =
    [0x40, 0xb1, 0x3a, 0x71, 0xf9, 0x0b, 0x96, 0x6e, 0x37, 0x54, 0x08, 0xf2, 0xd1, 0x81, 0xa1, 0xaa];

/// The **verifier** of a real workbook, encrypted.
const ENCRYPTED_VERIFIER: [u8; 16] =
    [0x51, 0x6f, 0x73, 0x2e, 0x96, 0x6f, 0xac, 0x17, 0xb1, 0xc5, 0xd7, 0xd8, 0xcc, 0x36, 0xc9, 0x28];

/// The **hash of the verifier** of a real workbook, encrypted.
const ENCRYPTED_VERIFIER_HASH: [u8; 32] = [
    0x2b, 0x61, 0x68, 0xda, 0xbe, 0x29, 0x11, 0xad, 0x2b, 0xd3, 0x7c, 0x17, 0x46, 0x74, 0x5c, 0x14, 0xd3,
    0xcf, 0x1b, 0xb1, 0x40, 0xa4, 0x8f, 0x4e, 0x6f, 0x3d, 0x23, 0x88, 0x08, 0x72, 0xb1, 0x6a,
];

/// The **value** of a character of base64.
fn base64_value(character: u8) -> Option<u8> {
    match character {
        b'A'..=b'Z' => Some(character - b'A'),
        b'a'..=b'z' => Some(character - b'a' + 26),
        b'0'..=b'9' => Some(character - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// The **bytes** of the numbers, read from their base64 spelling.
fn unbase64(text: &str) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for character in text.bytes() {
        let Some(value) = base64_value(character) else { continue };
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    out
}

/// The **key** of the cipher, from the password: the hash of the salt
/// and the password, hashed again for every turn of the spin, and then
/// expanded with `0x36` and `0x5C`.
fn a_key(password: &str, salt: &[u8], key_bits: u32) -> Vec<u8> {
    let bytes: Vec<u8> = password.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    let mut value = Sha1::digest([salt, &bytes].concat()).to_vec();
    for turn in 0..SPIN {
        let mut input = Vec::with_capacity(4 + value.len());
        input.extend_from_slice(&turn.to_le_bytes());
        input.extend_from_slice(&value);
        value = Sha1::digest(input).to_vec();
    }
    let mut last = value.clone();
    last.extend_from_slice(&0u32.to_le_bytes());
    let last = Sha1::digest(last).to_vec();
    let mut key = Vec::new();
    for pad in [0x36u8, 0x5C] {
        let mut block = [pad; 64];
        for (byte, hash) in block.iter_mut().zip(last.iter()) {
            *byte ^= hash;
        }
        key.extend_from_slice(&Sha1::digest(block));
    }
    key.truncate((key_bits / 8) as usize);
    key
}

/// The **bytes** the cipher closes, in the mode of no chaining.
fn aes_ecb(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buffer = data.to_vec();
    let (blocks, rest) = buffer.as_chunks_mut::<16>();
    assert!(rest.is_empty(), "the data is whole blocks");
    match key.len() {
        16 => {
            let mut cipher = aes::Aes128::new_from_slice(key).expect("the key is the size it says");
            for block in blocks.iter_mut() {
                cipher.encrypt_block_mut(GenericArray::from_mut_slice(block));
            }
        }
        32 => {
            let mut cipher = aes::Aes256::new_from_slice(key).expect("the key is the size it says");
            for block in blocks.iter_mut() {
                cipher.encrypt_block_mut(GenericArray::from_mut_slice(block));
            }
        }
        _ => panic!("the key is the size it says"),
    }
    buffer
}

/// The **encryption info** of a file of the old scheme.
fn an_encryption_info(
    salt: &[u8],
    verifier: &[u8],
    verifier_hash: &[u8],
    alg_id: u32,
    key_bits: u32,
) -> Vec<u8> {
    let mut header: Vec<u8> = Vec::new();
    header.extend_from_slice(&0x24u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    header.extend_from_slice(&alg_id.to_le_bytes());
    header.extend_from_slice(&0x8004u32.to_le_bytes());
    header.extend_from_slice(&key_bits.to_le_bytes());
    header.extend_from_slice(&0x18u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    let name = "Microsoft Enhanced RSA and AES Cryptographic Provider";
    for unit in name.encode_utf16() {
        header.extend_from_slice(&unit.to_le_bytes());
    }
    header.extend_from_slice(&0u16.to_le_bytes());
    let mut info: Vec<u8> = Vec::new();
    info.extend_from_slice(&3u16.to_le_bytes());
    info.extend_from_slice(&2u16.to_le_bytes());
    info.extend_from_slice(&0x24u32.to_le_bytes());
    info.extend_from_slice(&(header.len() as u32).to_le_bytes());
    info.extend_from_slice(&header);
    info.extend_from_slice(&16u32.to_le_bytes());
    info.extend_from_slice(salt);
    info.extend_from_slice(verifier);
    info.extend_from_slice(&20u32.to_le_bytes());
    info.extend_from_slice(verifier_hash);
    info
}

/// An **encrypted file** of the old scheme, with that package in it.
fn an_encrypted_file(plain: &[u8], password: &str, alg_id: u32, key_bits: u32) -> Vec<u8> {
    let salt: Vec<u8> = (0..16u8).collect();
    let verifier: Vec<u8> = (16..32u8).collect();
    let key = a_key(password, &salt, key_bits);
    let verifier_hash = Sha1::digest(&verifier).to_vec();
    let aes = alg_id & 0xFF00 == 0x6600;
    let (encrypted_verifier, encrypted_hash, body) = if aes {
        let mut hash = verifier_hash.clone();
        hash.resize(32, 0);
        let mut padded = plain.to_vec();
        while !padded.len().is_multiple_of(16) {
            padded.push(0);
        }
        (aes_ecb(&key, &verifier), aes_ecb(&key, &hash), aes_ecb(&key, &padded))
    } else {
        (
            craftkit_xlsx::rc4::open(&key, &verifier),
            craftkit_xlsx::rc4::open(&key, &verifier_hash),
            craftkit_xlsx::rc4::open(&key, plain),
        )
    };
    let info = an_encryption_info(&salt, &encrypted_verifier, &encrypted_hash, alg_id, key_bits);
    let mut package: Vec<u8> = Vec::new();
    package.extend_from_slice(&(plain.len() as u64).to_le_bytes());
    package.extend_from_slice(&body);
    a_compound_file(&[("EncryptionInfo".to_string(), info), ("EncryptedPackage".to_string(), package)])
}

/// A compound file with those streams in it.
fn a_compound_file(streams: &[(String, Vec<u8>)]) -> Vec<u8> {
    // The **mini stream**: the small streams, one after another.
    let mut mini_stream: Vec<u8> = Vec::new();
    let mut mini_starts: Vec<u32> = Vec::new();
    for (_, bytes) in streams {
        if bytes.len() >= 4096 {
            continue;
        }
        let start = (mini_stream.len() / MINI) as u32;
        mini_starts.push(start);
        mini_stream.extend_from_slice(bytes);
        while !mini_stream.len().is_multiple_of(MINI) {
            mini_stream.push(0);
        }
    }
    let mini_count = mini_stream.len() / MINI;
    let mut mini_fat: Vec<u8> = Vec::new();
    for index in 0..mini_count {
        let value = if index + 1 == mini_count { END } else { index as u32 + 1 };
        mini_fat.extend_from_slice(&value.to_le_bytes());
    }
    while mini_fat.len() < SECTOR {
        mini_fat.extend_from_slice(&END.to_le_bytes());
    }
    // The **sectors**: the directory, the table of the mini sectors, the
    // mini stream, and then the big streams.
    let mut sectors: Vec<Vec<u8>> = Vec::new();
    let mut fat: Vec<u32> = vec![END, END, END];
    let mut big_starts: Vec<u32> = Vec::new();
    let mini_sectors = mini_stream.len().div_ceil(SECTOR).max(1);
    for index in 0..mini_sectors {
        let at = index * SECTOR;
        let mut part = mini_stream.get(at..).unwrap_or_default().to_vec();
        part.resize(SECTOR, 0);
        sectors.push(part);
        fat.push(if index + 1 == mini_sectors { END } else { 4 + index as u32 });
    }
    let mut next_sector = 3 + mini_sectors as u32;
    for (_, bytes) in streams {
        if bytes.len() < 4096 {
            big_starts.push(0);
            continue;
        }
        let start = next_sector;
        big_starts.push(start);
        let count = bytes.len().div_ceil(SECTOR);
        for index in 0..count {
            let at = index * SECTOR;
            let mut part = bytes.get(at..).unwrap_or_default().to_vec();
            part.resize(SECTOR, 0);
            sectors.push(part);
            fat.push(if index + 1 == count { END } else { start + index as u32 + 1 });
        }
        next_sector += count as u32;
    }
    // The **directory**: the root, and then a stream for each.
    let mut directory: Vec<u8> = Vec::new();
    let mut entry = |name: &str, kind: u8, start: u32, size: u64| {
        let mut chunk = vec![0u8; 128];
        let mut units = name.encode_utf16().collect::<Vec<u16>>();
        units.push(0);
        for (index, unit) in units.iter().take(31).enumerate() {
            chunk[index * 2] = (unit & 0xFF) as u8;
            chunk[index * 2 + 1] = (unit >> 8) as u8;
        }
        chunk[64..66].copy_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        chunk[66] = kind;
        chunk[67] = 1;
        chunk[68..72].copy_from_slice(&END.to_le_bytes());
        chunk[72..76].copy_from_slice(&END.to_le_bytes());
        chunk[76..80].copy_from_slice(&END.to_le_bytes());
        chunk[116..120].copy_from_slice(&start.to_le_bytes());
        chunk[120..128].copy_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&chunk);
    };
    entry("Root Entry", 5, 3, mini_stream.len() as u64);
    for (index, (name, bytes)) in streams.iter().enumerate() {
        let small = bytes.len() < 4096;
        let start = if small { mini_starts[index] } else { big_starts[index] };
        entry(name, 2, start, bytes.len() as u64);
    }
    while !directory.len().is_multiple_of(SECTOR) {
        directory.push(0);
    }
    // The **file**: the header, the table of allocation, and the
    // sectors.
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&SIGNATURE);
    bytes.extend_from_slice(&[0u8; 16]);
    bytes.extend_from_slice(&0x003Eu16.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
    bytes.extend_from_slice(&9u16.to_le_bytes());
    bytes.extend_from_slice(&6u16.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 6]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&4096u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&END.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for index in 0..109 {
        let value: u32 = if index == 0 { 0 } else { 0xFFFF_FFFF };
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    while bytes.len() < 512 {
        bytes.push(0);
    }
    let mut fat_bytes: Vec<u8> = Vec::new();
    for value in &fat {
        fat_bytes.extend_from_slice(&value.to_le_bytes());
    }
    while !fat_bytes.len().is_multiple_of(SECTOR) {
        fat_bytes.extend_from_slice(&END.to_le_bytes());
    }
    bytes.extend_from_slice(&fat_bytes);
    bytes.extend_from_slice(&directory);
    bytes.extend_from_slice(&mini_fat[..SECTOR]);
    for part in &sectors {
        bytes.extend_from_slice(part);
    }
    bytes
}

/// A **package** of parts, written by hand.
fn a_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, text) in parts {
        let name = name.as_bytes();
        let data = text.as_bytes();
        let checksum = craftkit_zip::crc32(data);
        let offset = out.len() as u32;
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&checksum.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&checksum.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
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

/// A package with a workbook and a sheet.
fn a_workbook() -> Vec<u8> {
    a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>hello</t></is></c></row></sheetData></worksheet>"#,
        ),
    ])
}

/// The **key** a real workbook's password gives is the one the
/// specification's worked example gives.
#[test]
fn the_key_of_a_real_workbook_is_the_one_the_specification_gives() {
    let info = an_encryption_info(&SALT_KEY, &ENCRYPTED_VERIFIER, &ENCRYPTED_VERIFIER_HASH, 0x660E, 128);
    let key = standard::key_of(&info, REAL_PASSWORD).expect("the key is worked out");
    assert_eq!(key, EXPECTED_KEY.to_vec(), "the key is the one the specification gives");
}

/// The **verifier** of a real workbook takes its password and no other.
#[test]
fn the_verifier_of_a_real_workbook_takes_its_password() {
    let info = an_encryption_info(&SALT_KEY, &ENCRYPTED_VERIFIER, &ENCRYPTED_VERIFIER_HASH, 0x660E, 128);
    let takes = standard::verifies(&info, REAL_PASSWORD).expect("the verifier is checked");
    assert!(takes, "the password of the real workbook is taken");
    let refuses = standard::verifies(&info, "another password").expect("the verifier is checked");
    assert!(!refuses, "another password is refused");
}

/// An old workbook that **another program** encrypted is opened: with
/// AES of 128 bits, with AES of 256 bits, and with RC4.
#[test]
fn an_old_workbook_that_another_program_encrypted_is_opened() {
    for (info, package) in
        [(INFO_AES128, PACKAGE_AES128), (INFO_AES256, PACKAGE_AES256), (INFO_RC4, PACKAGE_RC4)]
    {
        let file = a_compound_file(&[
            ("EncryptionInfo".to_string(), unbase64(info)),
            ("EncryptedPackage".to_string(), unbase64(package)),
        ]);
        let compound = Compound::open(&file).expect("the compound file is opened");
        let plain = crypto::open(&compound, PASSWORD).expect("the package is opened");
        assert_eq!(
            plain,
            b"the package of the encrypted workbook".to_vec(),
            "the package is as it was written"
        );
    }
}

/// An old workbook is read as a workbook.
#[test]
fn an_old_workbook_is_read() {
    let workbook = a_workbook();
    let file = an_encrypted_file(&workbook, PASSWORD, 0x660E, 128);
    let read = craftkit_xlsx::read_with_password(&file, PASSWORD).expect("the workbook is read");
    assert_eq!(read.sheets.len(), 1, "the workbook has its sheet");
    assert_eq!(read.sheets[0].name, "One", "the sheet is the one of the workbook");
}

/// A **password that is not the password** of an old file opens nothing.
#[test]
fn a_password_that_is_not_the_password_opens_nothing() {
    let file = an_encrypted_file(b"the package of the encrypted workbook", PASSWORD, 0x660E, 128);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let error = crypto::open(&compound, "another password").expect_err("that is not the password");
    assert!(error.to_string().contains("password"), "the refusal says why: {error}");
}

/// A **file of the old encryption** that says two ciphers is refused.
#[test]
fn a_file_that_says_two_ciphers_is_refused() {
    let info = an_encryption_info(&SALT_KEY, &ENCRYPTED_VERIFIER, &ENCRYPTED_VERIFIER_HASH, 0x6801, 128);
    let error = standard::key_of(&info, REAL_PASSWORD).expect_err("that file says two ciphers");
    assert!(error.to_string().contains("cipher"), "the refusal says why: {error}");
}

/// The **stream** of RC4 is the one the cipher's own worked example
/// gives.
#[test]
fn the_stream_of_rc4_is_the_one_the_cipher_gives() {
    let stream = craftkit_xlsx::rc4::stream(b"Key", 9);
    let expected: Vec<u8> = vec![0xeb, 0x9f, 0x77, 0x81, 0xb7, 0x34, 0xca, 0x72, 0xa7];
    assert_eq!(stream, expected, "the stream is the one the cipher gives");
}
