//! The **cipher** that opens an encrypted workbook.
//!
//! A workbook that is encrypted is a **compound file** with two
//! streams in it: the `EncryptionInfo`, which says how the workbook
//! is encrypted, and the `EncryptedPackage`, which is the workbook
//! itself — encrypted.
//!
//! This module reads the first and opens the second. The scheme it
//! reads is the **agile** one of [MS-OFFCRYPTO] §2.3.4.11 and the
//! following: the password is hashed and hashed again for every turn
//! of the spin, the hash is checked against the verifier of the file,
//! and the key of the cipher comes out of it.
//!
//! The code here is written from that specification and not from any
//! other program. The **three** parts of it are worth naming, because
//! they are the parts that are easy to get wrong:
//!
//! - the **verifier** says whether the password is the right one. Its
//!   input is hashed and the hash is compared with the verifier's
//!   hash, both opened with the password.
//! - the **package** is the workbook, in segments of **4096** bytes,
//!   each opened with an **iv** of its own.
//! - the **integrity** is the HMAC of the package, over the package
//!   as it is encrypted, and it is what says the package was not
//!   changed on the way.
//!
//! # The file is hostile
//!
//! Every number in the file is a number that could be anything: a
//! spin of a billion turns, a salt of a gigabyte, a key of the wrong
//! size. The reader refuses what it cannot make sense of, and it
//! refuses **within limits**: the spin may be **10 000 000** turns, a
//! salt may be **1024** bytes, and the package may be **512 MB**.

use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::NoPadding};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256, Sha512};

use crate::cfb::Compound;

/// The most turns of the **spin** the reader will do.
const MAX_SPIN: u32 = 10_000_000;

/// The most bytes of a **salt** the reader will read.
const MAX_SALT: usize = 1024;

/// The most bytes of the **package** the reader will open.
const MAX_PACKAGE: u64 = 512 * 1024 * 1024;

/// The **bytes** of one **segment** of the package.
const SEGMENT: usize = 4096;

/// The **bytes** of one **block** of the cipher.
const BLOCK: usize = 16;

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

/// Why an encrypted workbook could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// What went wrong.
    pub message: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

impl From<crate::cfb::Error> for Error {
    fn from(error: crate::cfb::Error) -> Error {
        Error { message: error.message }
    }
}

/// The **error** of an encrypted workbook.
fn error(message: &str) -> Error {
    Error { message: message.to_string() }
}

/// The **hash** the file uses: SHA-512 or SHA-256.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    /// **SHA-512**: 64 bytes of hash.
    Sha512,
    /// **SHA-256**: 32 bytes of hash.
    Sha256,
}

impl Hash {
    /// The **hash** of that name, if the reader knows it.
    pub fn of(name: &str) -> Option<Hash> {
        match name {
            "SHA512" | "SHA-512" => Some(Hash::Sha512),
            "SHA256" | "SHA-256" => Some(Hash::Sha256),
            _ => None,
        }
    }

    /// The **bytes** of the hash of those bytes.
    fn digest(self, bytes: &[u8]) -> Vec<u8> {
        match self {
            Hash::Sha512 => Sha512::digest(bytes).to_vec(),
            Hash::Sha256 => Sha256::digest(bytes).to_vec(),
        }
    }
}

/// The **parameters** of one half of the scheme: the salt, the size
/// of the key, and the hash.
struct Params {
    /// The **salt**.
    salt: Vec<u8>,
    /// The **bits of the key**.
    key_bits: u32,
    /// The **hash**.
    hash: Hash,
}

impl Params {
    /// The parameters of that element of the descriptor.
    ///
    /// The `hashAlgorithm` of the element is the hash of that half of
    /// the scheme; an element that does not say which hash it uses
    /// takes the one of the `keyData`, which is the hash of the file.
    fn read(element: &craftkit_xml::El, fallback: Option<Hash>) -> Result<Params, Error> {
        let hash = match element.attr("hashAlgorithm") {
            Some(name) => {
                Hash::of(name).ok_or_else(|| error("the hash of the file is not one this reader knows"))?
            }
            None => fallback.ok_or_else(|| error("the hash of the file is not one this reader knows"))?,
        };
        if element.attr("cipherAlgorithm").unwrap_or("AES") != "AES" {
            return Err(error("the cipher of the file is not one this reader knows"));
        }
        if element.attr("cipherChaining").unwrap_or("ChainingModeCBC") != "ChainingModeCBC" {
            return Err(error("the cipher of the file is not in the mode this reader knows"));
        }
        if element.attr_u32("blockSize").unwrap_or(16) != 16 {
            return Err(error("the cipher of the file does not use blocks of sixteen bytes"));
        }
        let key_bits = element.attr_u32("keyBits").unwrap_or(256);
        if key_bits != 128 && key_bits != 256 {
            return Err(error("the key of the file is not a size this reader knows"));
        }
        let salt =
            base64(element.attr("saltValue")).ok_or_else(|| error("the salt of the file is not there"))?;
        if salt.is_empty() || salt.len() > MAX_SALT {
            return Err(error("the salt of the file is not a size this reader reads"));
        }
        Ok(Params { salt, key_bits, hash })
    }

    /// The **bytes** of the key.
    fn key_len(&self) -> usize {
        (self.key_bits / 8) as usize
    }
}

/// The **integrity** of the package: the HMAC of the package, and the
/// key it is computed with, both encrypted.
struct Integrity {
    /// The **key of the HMAC**, encrypted.
    hmac_key: Vec<u8>,
    /// The **value of the HMAC**, encrypted.
    hmac_value: Vec<u8>,
}

/// The **descriptor** of an encrypted workbook: the XML that the
/// `EncryptionInfo` stream holds.
struct Descriptor {
    /// The parameters of the **package**.
    key_data: Params,
    /// The parameters of the **password**.
    key: Params,
    /// The **turns of the spin**.
    spin: u32,
    /// The **verifier's input**, encrypted.
    verifier_input: Vec<u8>,
    /// The **verifier's hash**, encrypted.
    verifier_hash: Vec<u8>,
    /// The **key of the cipher**, encrypted.
    key_value: Vec<u8>,
    /// The **integrity** of the package, if the file holds one.
    integrity: Option<Integrity>,
}

/// The **descriptor** of the file, from the `EncryptionInfo` stream.
fn descriptor(info: &[u8]) -> Result<Descriptor, Error> {
    // The stream begins with the **version**: the major and the minor
    // in two bytes each, and the flags in four. The agile scheme is
    // the version 4.4, and the descriptor is the XML that follows.
    if info.len() < 8 {
        return Err(error("the encryption info of the file is too short"));
    }
    let major = u16::from_le_bytes([info[0], info[1]]);
    let minor = u16::from_le_bytes([info[2], info[3]]);
    if (major, minor) != (4, 4) {
        return Err(error("the file is not encrypted in the way this reader knows"));
    }
    let root =
        craftkit_xml::parse(&info[8..]).map_err(|_| error("the encryption info of the file is not xml"))?;
    let key_data =
        root.child("keyData").ok_or_else(|| error("the encryption info of the file has no key"))?;
    let key_data = Params::read(key_data, None)?;
    // The **key encryptor**: the one of the password. A file may hold
    // others — a certificate, say — and the password is the one that
    // opens a workbook a person saved.
    let encryptor = root
        .child("keyEncryptors")
        .and_then(|encryptors| {
            encryptors
                .kids("keyEncryptor")
                .find(|encryptor| encryptor.attr("uri").is_some_and(|uri| uri.ends_with("/password")))
        })
        .and_then(|encryptor| encryptor.child("encryptedKey"))
        .ok_or_else(|| error("the file does not say how the key is held"))?;
    let key = Params::read(encryptor, Some(key_data.hash))?;
    let spin = encryptor.attr_u32("spinCount").unwrap_or(100_000);
    if spin > MAX_SPIN {
        return Err(error("the file asks for more turns of the spin than this reader does"));
    }
    let field = |name: &str| {
        base64(encryptor.attr(name)).ok_or_else(|| error("the encryption info of the file is not whole"))
    };
    let verifier_input = field("encryptedVerifierHashInput")?;
    let verifier_hash = field("encryptedVerifierHashValue")?;
    let key_value = field("encryptedKeyValue")?;
    // The **integrity** is what says the package was not changed on the
    // way. A file without it cannot prove its package, and the reader
    // says so rather than open a package it cannot vouch for.
    let integrity = match root.child("dataIntegrity") {
        Some(element) => Some(Integrity {
            hmac_key: base64(element.attr("encryptedHmacKey"))
                .ok_or_else(|| error("the integrity of the file is not there"))?,
            hmac_value: base64(element.attr("encryptedHmacValue"))
                .ok_or_else(|| error("the integrity of the file is not there"))?,
        }),
        None => None,
    };
    Ok(Descriptor { key_data, key, spin, verifier_input, verifier_hash, key_value, integrity })
}

/// The **hash** of the password, with the block key at the end of it.
///
/// [MS-OFFCRYPTO] §2.3.4.11: the salt and the password are hashed,
/// and the hash is hashed again for every turn of the spin — the turn
/// in front of the hash — and the block key is put at the end of the
/// last hash.
///
/// The hash is then the size of the **key of the cipher**: a hash that
/// is longer is cut short, and a hash that is shorter is padded with
/// `0x36`.
fn derive(hash: Hash, first: &[u8], second: &[u8], len: usize) -> Vec<u8> {
    let mut key = hash.digest(&[first, second].concat());
    key.resize(len, 0x36);
    key
}

/// The **iterated hash** of the password: the salt and the password
/// hashed, and the hash hashed again for every turn of the spin.
fn iterated_hash(hash: Hash, password: &str, salt: &[u8], spin: u32) -> Vec<u8> {
    let mut value = hash.digest(&[salt, &password_of(password)].concat());
    for turn in 0..spin {
        let mut input = Vec::with_capacity(4 + value.len());
        input.extend_from_slice(&turn.to_le_bytes());
        input.extend_from_slice(&value);
        value = hash.digest(&input);
    }
    value
}

/// The **password**, as the bytes the hash takes: two bytes for every
/// character of it.
fn password_of(password: &str) -> Vec<u8> {
    password.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect()
}

/// The **iv** of the verifier and of the key: the salt of the
/// password, padded with `0x36` to one block.
fn iv_of(salt: &[u8]) -> Vec<u8> {
    let mut iv = salt.to_vec();
    iv.resize(BLOCK, 0x36);
    iv
}

/// The **bytes** the cipher opens.
fn aes_cbc(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut buffer = data.to_vec();
    let plain = match key.len() {
        16 => {
            let cipher = cbc::Decryptor::<aes::Aes128>::new_from_slices(key, iv)
                .map_err(|_| error("the key of the cipher is not the size it says"))?;
            cipher
                .decrypt_padded_mut::<NoPadding>(&mut buffer)
                .map_err(|_| error("the cipher could not read the data"))?
                .to_vec()
        }
        32 => {
            let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(key, iv)
                .map_err(|_| error("the key of the cipher is not the size it says"))?;
            cipher
                .decrypt_padded_mut::<NoPadding>(&mut buffer)
                .map_err(|_| error("the cipher could not read the data"))?
                .to_vec()
        }
        _ => return Err(error("the key of the cipher is not the size it says")),
    };
    Ok(plain)
}

/// The **bytes** of the numbers, read from their base64 spelling.
fn base64(text: Option<&str>) -> Option<Vec<u8>> {
    let text = text?;
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
    Some(out)
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

/// Whether the **password** is the password of the file.
///
/// The verifier says so: the input of the verifier is hashed, and the
/// hash is the verifier's hash.
pub fn verifies(
    hash: Hash,
    password: &str,
    salt: &[u8],
    spin: u32,
    key_bits: u32,
    encrypted_input: &[u8],
    encrypted_hash: &[u8],
) -> Result<bool, Error> {
    let key = iterated_hash(hash, password, salt, spin);
    let iv = iv_of(salt);
    let input =
        aes_cbc(&derive(hash, &key, &BLOCK_VERIFIER_INPUT, (key_bits / 8) as usize), &iv, encrypted_input)?;
    let expected =
        aes_cbc(&derive(hash, &key, &BLOCK_VERIFIER_HASH, (key_bits / 8) as usize), &iv, encrypted_hash)?;
    Ok(hash.digest(&input) == expected)
}

/// The **key of the cipher** of the file, from the password.
///
/// The key is encrypted in the file, and the password is what opens
/// it.
pub fn secret_key(
    hash: Hash,
    password: &str,
    salt: &[u8],
    spin: u32,
    key_bits: u32,
    encrypted_value: &[u8],
) -> Result<Vec<u8>, Error> {
    let key = iterated_hash(hash, password, salt, spin);
    aes_cbc(&derive(hash, &key, &BLOCK_KEY, (key_bits / 8) as usize), &iv_of(salt), encrypted_value)
}

/// The **package** of an encrypted workbook: the bytes of the workbook
/// itself, opened with the password.
///
/// The file says which scheme it used: the **agile** one, whose
/// descriptor is XML, and the **old** one, whose descriptor is a set
/// of fields.
pub fn open(compound: &Compound, password: &str) -> Result<Vec<u8>, Error> {
    let Some(info) = compound.read("EncryptionInfo")? else {
        return Err(error("the file is not an encrypted workbook"));
    };
    if info.len() < 4 {
        return Err(error("the encryption info of the file is too short"));
    }
    let major = u16::from_le_bytes([info[0], info[1]]);
    let minor = u16::from_le_bytes([info[2], info[3]]);
    if major != 4 || minor != 4 {
        // The **old** scheme: the fields of the header, and the
        // verifier.
        let Some(package) = compound.read("EncryptedPackage")? else {
            return Err(error("the file holds no package"));
        };
        return crate::standard::open(&info, &package, password);
    }
    let descriptor = descriptor(&info)?;
    let Some(package) = compound.read("EncryptedPackage")? else {
        return Err(error("the file holds no package"));
    };
    if package.len() as u64 > MAX_PACKAGE {
        return Err(error("the package of the file is larger than this reader reads"));
    }
    // The **verifier** comes first: a password that is not the
    // password is said to be so, and not to be a broken file.
    if !verifies(
        descriptor.key.hash,
        password,
        &descriptor.key.salt,
        descriptor.spin,
        descriptor.key.key_bits,
        &descriptor.verifier_input,
        &descriptor.verifier_hash,
    )? {
        return Err(error("the password is not the password of the file"));
    }
    let secret = secret_key(
        descriptor.key.hash,
        password,
        &descriptor.key.salt,
        descriptor.spin,
        descriptor.key.key_bits,
        &descriptor.key_value,
    )?;
    let secret = secret.get(..descriptor.key_data.key_len()).unwrap_or(&[]).to_vec();
    if secret.len() != descriptor.key_data.key_len() {
        return Err(error("the key of the file is not the size it says"));
    }
    let plain = decrypt_package(&descriptor.key_data, &secret, &package)?;
    // The **integrity**: the HMAC of the package, over the package as
    // it is encrypted. A file without it cannot prove its package.
    let Some(integrity) = descriptor.integrity else {
        return Err(error("the file does not say how to prove its package"));
    };
    let iv_key = derive(descriptor.key_data.hash, &descriptor.key_data.salt, &BLOCK_HMAC_KEY, BLOCK);
    let iv_value = derive(descriptor.key_data.hash, &descriptor.key_data.salt, &BLOCK_HMAC_VALUE, BLOCK);
    let mac_key = aes_cbc(&secret, &iv_key, &integrity.hmac_key)?;
    let mac_value = aes_cbc(&secret, &iv_value, &integrity.hmac_value)?;
    let mac = hmac(descriptor.key_data.hash, &mac_key, &package)?;
    if mac.as_slice() != mac_value.as_slice() {
        return Err(error("the package of the file is not the one it says it is"));
    }
    Ok(plain)
}

/// The **hmac** of a message, with the hash the file uses.
fn hmac(hash: Hash, key: &[u8], message: &[u8]) -> Result<Vec<u8>, Error> {
    match hash {
        Hash::Sha512 => {
            let mut mac = <Hmac<Sha512> as Mac>::new_from_slice(key)
                .map_err(|_| error("the key of the HMAC is not the size it says"))?;
            mac.update(message);
            Ok(mac.finalize().into_bytes().to_vec())
        }
        Hash::Sha256 => {
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key)
                .map_err(|_| error("the key of the HMAC is not the size it says"))?;
            mac.update(message);
            Ok(mac.finalize().into_bytes().to_vec())
        }
    }
}

/// The **package** of the file, opened with the key of the cipher.
///
/// The package is in **segments** of 4096 bytes, each with an **iv**
/// of its own, and the first eight bytes of the stream say how long
/// the package is.
fn decrypt_package(params: &Params, key: &[u8], package: &[u8]) -> Result<Vec<u8>, Error> {
    if package.len() < 8 {
        return Err(error("the package of the file is too short"));
    }
    let size = u64::from_le_bytes([
        package[0], package[1], package[2], package[3], package[4], package[5], package[6], package[7],
    ]);
    let data = &package[8..];
    if size > data.len() as u64 {
        return Err(error("the package of the file is shorter than it says it is"));
    }
    let mut out: Vec<u8> = Vec::with_capacity(size as usize);
    for (index, segment) in data.chunks(SEGMENT).enumerate() {
        if out.len() as u64 >= size {
            break;
        }
        let iv = derive(params.hash, &params.salt, &(index as u32).to_le_bytes(), BLOCK);
        let plain = aes_cbc(key, &iv, segment)?;
        out.extend_from_slice(&plain);
    }
    out.truncate(size as usize);
    Ok(out)
}
