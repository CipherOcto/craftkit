//! The **old cipher**: the **Standard** scheme of [MS-OFFCRYPTO]
//! §2.3.4 — the encryption a workbook of Excel 2007 carries.
//!
//! The old scheme is a **stream of blocks** and not the agile scheme
//! of [`crate::crypto`]: the `EncryptionInfo` is not XML but a fixed
//! set of fields, the key comes from **SHA-1** and fifty thousand
//! turns of it, and the package is opened in one go.
//!
//! Two ciphers are read, and the file says which one it used:
//!
//! - **AES**, in the mode of no chaining: every block stands on its
//!   own.
//! - **RC4**, the stream cipher of [`crate::rc4`] — a key of forty
//!   bits, which is short enough to be searched, and which is why the
//!   scheme was replaced.
//!
//! The **verifier** of the file says whether the password is the right
//! one: the verifier is opened with the key, and the hash of it must
//! be the hash the file holds.
//!
//! # The file is hostile
//!
//! Every number in the file is a number that could be anything: a
//! header that says it is longer than the file, a key of a size no
//! cipher has. The reader refuses what it cannot make sense of, and it
//! refuses **within limits**: the header may be **64 KiB**, and the
//! package may be **512 MB**.

use aes::cipher::{BlockDecryptMut, KeyInit, generic_array::GenericArray};
use sha1::{Digest, Sha1};

use crate::crypto::Error;
use crate::rc4;

/// The most bytes of the **header** of the file.
const MAX_HEADER: usize = 64 * 1024;

/// The most bytes of the **package** the reader will open.
const MAX_PACKAGE: u64 = 512 * 1024 * 1024;

/// The **turns of the spin** the old scheme always does.
const SPIN: u32 = 50_000;

/// The **flags** of the file: the cipher is **AES**.
const F_AES: u32 = 0x20;

/// The **flags** of the file: the encryption is of the **crypto api**.
const F_CRYPTO_API: u32 = 0x04;

/// The **flags** of the file: the encryption is **external**.
const F_EXTERNAL: u32 = 0x10;

/// The **cipher** the file used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cipher {
    /// **AES**, in the mode of no chaining.
    Aes(u32),
    /// **RC4**, the stream cipher.
    Rc4,
}

impl Cipher {
    /// The **bytes** of the key of that cipher.
    fn key_len(self, key_bits: u32) -> usize {
        match self {
            Cipher::Aes(bits) => (bits / 8) as usize,
            Cipher::Rc4 => (key_bits / 8) as usize,
        }
    }
}

/// The **header** of the old scheme: the cipher, and the key of it.
struct Header {
    /// The **cipher**.
    cipher: Cipher,
    /// The **bits of the key**.
    key_bits: u32,
    /// The **salt** of the password.
    salt: Vec<u8>,
    /// The **verifier**, encrypted.
    verifier: Vec<u8>,
    /// The **hash of the verifier**, encrypted.
    verifier_hash: Vec<u8>,
}

/// The **error** of the old scheme.
fn error(message: &str) -> Error {
    Error { message: message.to_string() }
}

/// The **bytes** of a number of the file.
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let four = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([four[0], four[1], four[2], four[3]]))
}

/// The **header** of the file: what it says about the cipher.
///
/// The stream begins with the **version**: the major and the minor in
/// two bytes each. The old scheme is the version 2.2, 3.2 and 4.2,
/// and the extensible one is the 3.3 and the 4.3.
fn header(info: &[u8]) -> Result<Header, Error> {
    if info.len() < 4 {
        return Err(error("the encryption info of the file is too short"));
    }
    let major = u16::from_le_bytes([info[0], info[1]]);
    let minor = u16::from_le_bytes([info[2], info[3]]);
    if (major == 3 || major == 4) && minor == 3 {
        return Err(error("the file is encrypted in the extensible way, and this tree does not read it"));
    }
    if minor != 2 || !(2..=4).contains(&major) {
        return Err(error("the file is not encrypted in the way this reader knows"));
    }
    // The **flags** of the file, and the size of the header that
    // follows them.
    let flags = u32_at(info, 4).ok_or_else(|| error("the encryption info of the file is too short"))?;
    if flags & F_EXTERNAL != 0 {
        return Err(error("the file is encrypted by rights management, and this tree does not read it"));
    }
    if flags & F_CRYPTO_API == 0 {
        return Err(error("the file is encrypted in a way this reader does not know"));
    }
    let size = u32_at(info, 8).ok_or_else(|| error("the encryption info of the file is too short"))?;
    let size = usize::try_from(size).map_err(|_| error("the header of the file is not a size"))?;
    if size > MAX_HEADER {
        return Err(error("the header of the file is longer than this reader reads"));
    }
    let block = info
        .get(12..12usize.checked_add(size).ok_or_else(|| error("the header of the file is out of range"))?)
        .ok_or_else(|| error("the header of the file is not in the file"))?;
    // The **cipher**: the id of it, and the size of the key.
    let alg_id = u32_at(block, 8).ok_or_else(|| error("the header of the file is too short"))?;
    let alg_hash = u32_at(block, 12).ok_or_else(|| error("the header of the file is too short"))?;
    let key_bits = u32_at(block, 16).ok_or_else(|| error("the header of the file is too short"))?;
    let cipher = match alg_id {
        0x660E => Cipher::Aes(128),
        0x660F => Cipher::Aes(192),
        0x6610 => Cipher::Aes(256),
        0x6801 => Cipher::Rc4,
        _ => return Err(error("the cipher of the file is not one this reader knows")),
    };
    // The **hash**: SHA-1, or nothing at all, which means SHA-1.
    if alg_hash != 0x8004 && alg_hash != 0 {
        return Err(error("the hash of the file is not one this reader knows"));
    }
    // The **flags** and the **cipher** must say the same thing: a
    // file that says two ciphers is a file that is not what it says.
    if (flags & F_AES != 0) != matches!(cipher, Cipher::Aes(_)) {
        return Err(error("the file says two different ciphers"));
    }
    let key_bits = match cipher {
        Cipher::Aes(bits) => bits,
        // RC4's key is forty bits, and the file may say nothing at
        // all about it.
        Cipher::Rc4 => {
            if key_bits == 0 {
                40
            } else {
                key_bits
            }
        }
    };
    if cipher.key_len(key_bits) == 0 || cipher.key_len(key_bits) > 32 {
        return Err(error("the key of the file is not a size this reader knows"));
    }
    // The **verifier**: the salt of the password, the verifier itself,
    // and the hash of it.
    let verifier_at = 12 + size;
    let rest = info.get(verifier_at..).ok_or_else(|| error("the verifier of the file is not in the file"))?;
    let salt_size = u32_at(rest, 0).ok_or_else(|| error("the verifier of the file is too short"))?;
    if salt_size != 16 {
        return Err(error("the salt of the file is not sixteen bytes"));
    }
    let salt = rest.get(4..20).ok_or_else(|| error("the verifier of the file is too short"))?.to_vec();
    let verifier = rest.get(20..36).ok_or_else(|| error("the verifier of the file is too short"))?.to_vec();
    let hash_size = u32_at(rest, 36).ok_or_else(|| error("the verifier of the file is too short"))?;
    if hash_size != 20 {
        return Err(error("the hash of the verifier of the file is not twenty bytes"));
    }
    // The hash of the verifier is a whole block of the cipher: thirty
    // two bytes for AES, and the twenty of SHA-1 for RC4.
    let hash_len = match cipher {
        Cipher::Aes(_) => 32,
        Cipher::Rc4 => 20,
    };
    let verifier_hash =
        rest.get(40..40 + hash_len).ok_or_else(|| error("the verifier of the file is too short"))?.to_vec();
    Ok(Header { cipher, key_bits, salt, verifier, verifier_hash })
}

/// The **key** of the cipher of the file, from the password.
///
/// [MS-OFFCRYPTO] §2.3.4.7: the salt and the password are hashed, and
/// the hash is hashed again for every turn of the spin — the turn in
/// front of the hash — and the block `0` is put at the end of the last
/// hash. The key is then the hash, expanded with `0x36` and `0x5C`,
/// and cut to the size of the key.
pub fn key_of(info: &[u8], password: &str) -> Result<Vec<u8>, Error> {
    let header = header(info)?;
    Ok(key_from(&header, password))
}

/// The **key** of the cipher, from the header and the password.
fn key_from(header: &Header, password: &str) -> Vec<u8> {
    let bytes: Vec<u8> = password.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    let mut value = Sha1::digest([header.salt.as_slice(), &bytes].concat()).to_vec();
    for turn in 0..SPIN {
        let mut input = Vec::with_capacity(4 + value.len());
        input.extend_from_slice(&turn.to_le_bytes());
        input.extend_from_slice(&value);
        value = Sha1::digest(input).to_vec();
    }
    let mut last = Vec::with_capacity(4 + value.len());
    last.extend_from_slice(&value);
    last.extend_from_slice(&0u32.to_le_bytes());
    let last = Sha1::digest(last).to_vec();
    // The **expansion**: the hash with `0x36` over it, and the hash
    // with `0x5C` over it, and the two of them together are the key.
    let mut key = Vec::with_capacity(40);
    for pad in [0x36u8, 0x5C] {
        let mut block = [pad; 64];
        for (byte, hash) in block.iter_mut().zip(last.iter()) {
            *byte ^= hash;
        }
        key.extend_from_slice(&Sha1::digest(block));
    }
    key.truncate(header.cipher.key_len(header.key_bits));
    key
}

/// The **bytes** the cipher opens, in the mode of no chaining.
///
/// Every block stands on its own: the mode of no chaining has no iv,
/// and the block before it has nothing to say to it.
fn aes_ecb(key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut buffer = data.to_vec();
    let (blocks, rest) = buffer.as_chunks_mut::<16>();
    if !rest.is_empty() {
        return Err(error("the data of the file is not whole blocks"));
    }
    let wrong = || error("the key of the cipher is not the size it says");
    match key.len() {
        16 => {
            let mut cipher = aes::Aes128::new_from_slice(key).map_err(|_| wrong())?;
            for block in blocks.iter_mut() {
                cipher.decrypt_block_mut(GenericArray::from_mut_slice(block));
            }
        }
        24 => {
            let mut cipher = aes::Aes192::new_from_slice(key).map_err(|_| wrong())?;
            for block in blocks.iter_mut() {
                cipher.decrypt_block_mut(GenericArray::from_mut_slice(block));
            }
        }
        32 => {
            let mut cipher = aes::Aes256::new_from_slice(key).map_err(|_| wrong())?;
            for block in blocks.iter_mut() {
                cipher.decrypt_block_mut(GenericArray::from_mut_slice(block));
            }
        }
        _ => return Err(wrong()),
    }
    Ok(buffer)
}

/// The **bytes** the cipher opens, whichever cipher the file used.
fn open_bytes(cipher: Cipher, key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    match cipher {
        Cipher::Aes(_) => aes_ecb(key, data),
        Cipher::Rc4 => Ok(rc4::open(key, data)),
    }
}

/// Whether the **password** is the password of the file.
///
/// The verifier says so: the verifier is opened with the key, and the
/// hash of it must be the hash the file holds.
pub fn verifies(info: &[u8], password: &str) -> Result<bool, Error> {
    let header = header(info)?;
    let key = key_from(&header, password);
    let verifier = open_bytes(header.cipher, &key, &header.verifier)?;
    let expected = open_bytes(header.cipher, &key, &header.verifier_hash)?;
    let hash = Sha1::digest(&verifier);
    Ok(expected.get(..hash.len()) == Some(hash.as_slice()))
}

/// The **package** of the file, opened with the password.
///
/// The package is the workbook, and the first eight bytes of the
/// stream say how long it is.
pub fn open(info: &[u8], package: &[u8], password: &str) -> Result<Vec<u8>, Error> {
    if package.len() as u64 > MAX_PACKAGE {
        return Err(error("the package of the file is larger than this reader reads"));
    }
    let header = header(info)?;
    let key = key_from(&header, password);
    // The **verifier** comes first: a password that is not the
    // password is said to be so, and not to be a broken file.
    let verifier = open_bytes(header.cipher, &key, &header.verifier)?;
    let expected = open_bytes(header.cipher, &key, &header.verifier_hash)?;
    let hash = Sha1::digest(&verifier);
    if expected.get(..hash.len()) != Some(hash.as_slice()) {
        return Err(error("the password is not the password of the file"));
    }
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
    let mut plain = open_bytes(header.cipher, &key, data)?;
    plain.truncate(size as usize);
    Ok(plain)
}
