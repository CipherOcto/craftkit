//! An **encrypted workbook** is read: the cipher of [MS-OFFCRYPTO] and
//! the password that opens it.
//!
//! The files of these tests are built here — the compound file, the
//! cipher and the package — and the **values** of the two tests that
//! use a real workbook are the ones the specification's own worked
//! example gives.

use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::NoPadding};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha512};

use craftkit_xlsx::cfb::Compound;
use craftkit_xlsx::crypto;

/// The **password** of the file the tests make.
const PASSWORD: &str = "the password";

/// The **signature** of a compound file.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The end of a **chain**.
const END: u32 = 0xFFFF_FFFF;

/// The size of a **sector**.
const SECTOR: usize = 512;

/// The size of a **mini sector**.
const MINI: usize = 64;

/// The **turns of the spin** of the file the tests make.
const SPIN: u32 = 1000;

/// The **bytes** of one **segment** of a package.
const SEGMENT: usize = 4096;

/// The **block key** of the verifier's input.
const BLOCK_VERIFIER_INPUT: [u8; 8] = [0xFE, 0xA7, 0xD2, 0x76, 0x3B, 0x4B, 0x9E, 0x79];

/// The **block key** of the verifier's hash.
const BLOCK_VERIFIER_HASH: [u8; 8] = [0xD7, 0xAA, 0x0F, 0x6D, 0x30, 0x61, 0x34, 0x4E];

/// The **block key** of the key of the cipher.
const BLOCK_KEY: [u8; 8] = [0x14, 0x6E, 0x0B, 0xE7, 0xAB, 0xAC, 0xD0, 0xD6];

/// The **block key** of the key of the HMAC.
const BLOCK_HMAC_KEY: [u8; 8] = [0x5F, 0xB2, 0xAD, 0x01, 0x0C, 0xB9, 0xE1, 0xF6];

/// The **block key** of the value of the HMAC.
const BLOCK_HMAC_VALUE: [u8; 8] = [0xA0, 0x67, 0x7F, 0x02, 0xB2, 0x2C, 0x84, 0x33];

/// The **salt** of the key of a real workbook.
const SALT_KEY: [u8; 16] =
    [0x4c, 0x72, 0x5d, 0x45, 0xdc, 0x61, 0x0f, 0x93, 0x94, 0x12, 0xa0, 0x4d, 0xa7, 0x91, 0x04, 0x66];

/// The **encrypted key** of a real workbook.
const ENCRYPTED_KEY: [u8; 32] = [
    0xa1, 0x6c, 0xd5, 0x16, 0x5a, 0x7a, 0xb9, 0xd2, 0x71, 0x11, 0x3e, 0xd3, 0x86, 0xa7, 0x8c, 0xf4, 0x96,
    0x92, 0xe8, 0xe5, 0x27, 0xb0, 0xc5, 0xfc, 0x00, 0x55, 0xed, 0x08, 0x0b, 0x7c, 0xb9, 0x4b,
];

/// The **key** a real workbook's password gives.
const SECRET_KEY: [u8; 32] = [
    0x40, 0x20, 0x66, 0x09, 0xd9, 0xfa, 0xad, 0xf2, 0x4b, 0x07, 0x6a, 0xeb, 0xf2, 0xc4, 0x35, 0xb7, 0x42,
    0x92, 0xc8, 0xb8, 0xa7, 0xaa, 0x81, 0xbc, 0x67, 0x9b, 0xe8, 0x97, 0x11, 0xb0, 0x2a, 0xc2,
];

/// The **password** of the real workbook the values above are of.
const REAL_PASSWORD: &str = "Password1234_";

/// The **descriptor** of a workbook that another program encrypted.
///
/// The tree reads a file it did not write itself: these are the
/// values that program wrote.
const DESCRIPTOR: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<encryption xmlns="http://schemas.microsoft.com/office/2006/encryption" xmlns:p="http://schemas.microsoft.com/office/2006/keyEncryptor/password" xmlns:c="http://schemas.microsoft.com/office/2006/keyEncryptor/certificate">
    <keyData saltSize="16" blockSize="16" keyBits="256" hashSize="64"
             cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512" saltValue="UGexNONRrynvBZlAf7HBaQ==" />
    <dataIntegrity encryptedHmacKey="VYIbKBBspxpkSYRcFOOWilzF7zCqHO354l1mPkb5kdOTlJPKpTwlsL04fvsNS93KA7Pn5zl3uK12P2IZPhXO5Q==" encryptedHmacValue="yePPQ9ZEd8xmNPdB+PWUMP1z9HzZCrpxE+wJT132bPr1XPi+ZRyrXpzNJ2vKA1Rxs1rv1kZvtXg8D1jx7oKisQ==" />
    <keyEncryptors>
        <keyEncryptor uri="http://schemas.microsoft.com/office/2006/keyEncryptor/password">
            <p:encryptedKey spinCount="1000" saltSize="16" blockSize="16" keyBits="256"
                            hashSize="64" cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512"
                            saltValue="EBESExQVFhcYGRobHB0eHw==" encryptedVerifierHashInput="FUqXeadYUT92Uo/kkvkexQ=="
                            encryptedVerifierHashValue="tL9VFaJdPKDlBbiMKAs3f4FTPKtiRIiuZTURmEr1/qwiYfGJ0iJC5MZtakmiVAcc/nZ50+h9VpJMSxM/EXVxzA==" encryptedKeyValue="1vImO3QbbDSCZUjmsFP3uKW7rybh577F5diRuUGR1B4=" />
        </keyEncryptor>
    </keyEncryptors>
</encryption>"#;

/// The **encrypted package** of that workbook, in base64.
const PACKAGE: &str = "JQAAAAAAAAAgbGS3uEXuFg2NjpuvaHmE6iohq9v5cmTe3h5pv5Z0jMG6P7qI6D/kzbk9v3BMLIo=";

/// The **salt** of the verifier of that workbook.
const SALT_VERIFIER: &str = "EBESExQVFhcYGRobHB0eHw==";

/// The **encrypted input** of the verifier of that workbook.
const ENCRYPTED_INPUT: &str = "FUqXeadYUT92Uo/kkvkexQ==";

/// The **encrypted hash** of the verifier of that workbook.
const ENCRYPTED_HASH: &str =
    "tL9VFaJdPKDlBbiMKAs3f4FTPKtiRIiuZTURmEr1/qwiYfGJ0iJC5MZtakmiVAcc/nZ50+h9VpJMSxM/EXVxzA==";

/// The **hash** with the block key at the end of it, sized as the key
/// of the cipher is.
fn derive(first: &[u8], second: &[u8], len: usize) -> Vec<u8> {
    let mut key = Sha512::digest([first, second].concat()).to_vec();
    key.resize(len, 0x36);
    key
}

/// The **iterated hash** of the password.
fn iterated_hash(password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    let bytes: Vec<u8> = password.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    let mut value = Sha512::digest([salt, &bytes].concat()).to_vec();
    for turn in 0..spin {
        let mut input = Vec::with_capacity(4 + value.len());
        input.extend_from_slice(&turn.to_le_bytes());
        input.extend_from_slice(&value);
        value = Sha512::digest(input).to_vec();
    }
    value
}

/// The **bytes** the cipher closes. The data is whole blocks already.
fn encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Vec<u8> {
    let mut buffer = data.to_vec();
    let length = buffer.len();
    let cipher =
        cbc::Encryptor::<aes::Aes256>::new_from_slices(key, iv).expect("the key is the size it says");
    cipher.encrypt_padded_mut::<NoPadding>(&mut buffer, length).expect("the data is whole blocks");
    buffer
}

/// The **base64** of some bytes.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut buffer = [0u8; 3];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let number = u32::from(buffer[0]) << 16 | u32::from(buffer[1]) << 8 | u32::from(buffer[2]);
        let four = [(number >> 18) & 63, (number >> 12) & 63, (number >> 6) & 63, number & 63];
        for (index, value) in four.iter().enumerate() {
            if index > chunk.len() {
                out.push('=');
            } else {
                out.push(ALPHABET[*value as usize] as char);
            }
        }
    }
    out
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

/// The **package** of the file: the bytes of the workbook, encrypted.
fn encrypt_package(secret: &[u8], salt: &[u8], plain: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&(plain.len() as u64).to_le_bytes());
    for (index, chunk) in plain.chunks(SEGMENT).enumerate() {
        let mut block = chunk.to_vec();
        while !block.len().is_multiple_of(16) {
            block.push(0);
        }
        let iv = Sha512::digest([salt, &(index as u32).to_le_bytes()].concat()).to_vec();
        out.extend_from_slice(&encrypt(secret, &iv[..16], &block));
    }
    out
}

/// The **encryption info** and the **encrypted package** of a file.
fn an_encrypted_file(plain: &[u8], password: &str) -> (Vec<u8>, Vec<u8>) {
    let salt: Vec<u8> = (0..16u8).collect();
    let password_salt: Vec<u8> = (16..32u8).collect();
    let secret: Vec<u8> = (32..64u8).collect();
    let verifier_input: Vec<u8> = (64..80u8).collect();
    let verifier_hash = Sha512::digest(&verifier_input).to_vec();
    let hash = iterated_hash(password, &password_salt, SPIN);
    let encrypted_input = encrypt(&derive(&hash, &BLOCK_VERIFIER_INPUT, 32), &password_salt, &verifier_input);
    let encrypted_hash = encrypt(&derive(&hash, &BLOCK_VERIFIER_HASH, 32), &password_salt, &verifier_hash);
    let encrypted_key = encrypt(&derive(&hash, &BLOCK_KEY, 32), &password_salt, &secret);
    // The **package**, and then the **integrity** of it: the HMAC is
    // over the package as it is encrypted, the size of it included.
    let package = encrypt_package(&secret, &salt, plain);
    let mac_key: Vec<u8> = (80..144u8).collect();
    let iv_key = Sha512::digest([salt.as_slice(), &BLOCK_HMAC_KEY].concat()).to_vec();
    let iv_value = Sha512::digest([salt.as_slice(), &BLOCK_HMAC_VALUE].concat()).to_vec();
    let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(&mac_key).expect("the key is the size it says");
    mac.update(&package);
    let mac_value = mac.finalize().into_bytes().to_vec();
    let encrypted_mac_key = encrypt(&secret, &iv_key[..16], &mac_key);
    let encrypted_mac_value = encrypt(&secret, &iv_value[..16], &mac_value);
    // The **descriptor**: what the file says about itself.
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<encryption xmlns="http://schemas.microsoft.com/office/2006/encryption" xmlns:p="http://schemas.microsoft.com/office/2006/keyEncryptor/password"><keyData saltSize="16" blockSize="16" keyBits="256" hashSize="64" cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512" saltValue="{salt}"/><dataIntegrity encryptedHmacKey="{mac_key}" encryptedHmacValue="{mac_value}"/><keyEncryptors><keyEncryptor uri="http://schemas.microsoft.com/office/2006/keyEncryptor/password"><p:encryptedKey spinCount="{spin}" saltSize="16" blockSize="16" keyBits="256" hashSize="64" cipherAlgorithm="AES" cipherChaining="ChainingModeCBC" hashAlgorithm="SHA512" saltValue="{password_salt}" encryptedVerifierHashInput="{input}" encryptedVerifierHashValue="{hash}" encryptedKeyValue="{key}"/></keyEncryptor></keyEncryptors></encryption>"#,
        salt = base64(&salt),
        mac_key = base64(&encrypted_mac_key),
        mac_value = base64(&encrypted_mac_value),
        spin = SPIN,
        password_salt = base64(&password_salt),
        input = base64(&encrypted_input),
        hash = base64(&encrypted_hash),
        key = base64(&encrypted_key),
    );
    let mut info: Vec<u8> = Vec::new();
    info.extend_from_slice(&4u16.to_le_bytes());
    info.extend_from_slice(&4u16.to_le_bytes());
    info.extend_from_slice(&0u32.to_le_bytes());
    info.extend_from_slice(xml.as_bytes());
    (info, package)
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
        // The **local header**.
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
        // The **directory entry**.
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

/// An **encrypted file** with that package in it.
fn an_encrypted_workbook(plain: &[u8], password: &str) -> Vec<u8> {
    let (info, package) = an_encrypted_file(plain, password);
    a_compound_file(&[("EncryptionInfo".to_string(), info), ("EncryptedPackage".to_string(), package)])
}

/// The **password** opens the package of the file.
#[test]
fn the_password_opens_the_package() {
    let package = b"the package of the encrypted workbook".to_vec();
    let file = an_encrypted_workbook(&package, PASSWORD);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let plain = crypto::open(&compound, PASSWORD).expect("the package is opened");
    assert_eq!(plain, package, "the package is as it was written");
}

/// An **encrypted workbook** is read as a workbook.
#[test]
fn an_encrypted_workbook_is_read() {
    let workbook = a_workbook();
    let file = an_encrypted_workbook(&workbook, PASSWORD);
    let read = craftkit_xlsx::read_with_password(&file, PASSWORD).expect("the workbook is read");
    assert_eq!(read.sheets.len(), 1, "the workbook has its sheet");
    assert_eq!(read.sheets[0].name, "One", "the sheet is the one of the workbook");
}

/// A **password that is not the password** of the file opens nothing.
#[test]
fn a_password_that_is_not_the_password_opens_nothing() {
    let file = an_encrypted_workbook(b"the package of the encrypted workbook", PASSWORD);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let error = crypto::open(&compound, "another password").expect_err("that is not the password");
    assert!(error.to_string().contains("password"), "the refusal says why: {error}");
}

/// A **package that was changed** is refused.
#[test]
fn a_package_that_was_changed_is_refused() {
    let (info, package) = an_encrypted_file(b"the package of the encrypted workbook", PASSWORD);
    let mut changed = package.clone();
    let last = changed.len() - 1;
    changed[last] ^= 0xFF;
    let file =
        a_compound_file(&[("EncryptionInfo".to_string(), info), ("EncryptedPackage".to_string(), changed)]);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let error = crypto::open(&compound, PASSWORD).expect_err("the package is not the one the file says");
    assert!(error.to_string().contains("not the one it says"), "the refusal says why: {error}");
}

/// The **key** a real workbook's password gives is the one the
/// specification's worked example gives.
#[test]
fn the_key_of_a_real_workbook_is_the_one_the_specification_gives() {
    let key =
        crypto::secret_key(crypto::Hash::Sha512, REAL_PASSWORD, &SALT_KEY, 100_000, 256, &ENCRYPTED_KEY)
            .expect("the key is opened");
    assert_eq!(key, SECRET_KEY.to_vec(), "the key is the one the specification gives");
}

/// The **verifier** of that workbook takes its password and no other.
#[test]
fn the_verifier_of_a_real_workbook_takes_its_password() {
    let takes = crypto::verifies(
        crypto::Hash::Sha512,
        PASSWORD,
        &unbase64(SALT_VERIFIER),
        1000,
        256,
        &unbase64(ENCRYPTED_INPUT),
        &unbase64(ENCRYPTED_HASH),
    )
    .expect("the verifier is checked");
    assert!(takes, "the password of the real workbook is taken");
    let refuses = crypto::verifies(
        crypto::Hash::Sha512,
        "another password",
        &unbase64(SALT_VERIFIER),
        1000,
        256,
        &unbase64(ENCRYPTED_INPUT),
        &unbase64(ENCRYPTED_HASH),
    )
    .expect("the verifier is checked");
    assert!(!refuses, "another password is refused");
}

/// A workbook that **another program** encrypted is opened.
#[test]
fn a_workbook_that_another_program_encrypted_is_opened() {
    let mut info: Vec<u8> = Vec::new();
    info.extend_from_slice(&4u16.to_le_bytes());
    info.extend_from_slice(&4u16.to_le_bytes());
    info.extend_from_slice(&0u32.to_le_bytes());
    info.extend_from_slice(DESCRIPTOR.as_bytes());
    let file = a_compound_file(&[
        ("EncryptionInfo".to_string(), info),
        ("EncryptedPackage".to_string(), unbase64(PACKAGE)),
    ]);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let plain = crypto::open(&compound, PASSWORD).expect("the package is opened");
    assert_eq!(plain, b"the package of the encrypted workbook".to_vec(), "the package is as it was written");
}
