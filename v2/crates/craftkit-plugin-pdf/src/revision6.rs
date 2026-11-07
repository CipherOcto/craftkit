//! The **handler of revision six**: the standard security handler of
//! PDF 2.0.
//!
//! The handler of revision six is a different animal from the one of
//! revision four: the key of the file is not the hash of the password,
//! and the passwords are checked with a **hardened hash** that takes
//! many rounds to come out.
//!
//! # The rules
//!
//! - **The hardened hash is the specification's.** The hash of the
//!   input is taken, and then the bytes are scrambled with AES-128
//!   sixty-four times over, and the hash that comes out is hashed
//!   again — and the hash that is used is worked out from the bytes
//!   that were scrambled. The rounds go on until the last byte of the
//!   scrambled bytes says that they may stop.
//! - **The owner's key is the user's key with the user in it.** The
//!   hash of the owner's password is worked out with the user's entry
//!   beside it, and the key of the file comes out of the owner's
//!   encrypted entry.
//! - **The permissions are checked.** The file holds a block that is
//!   scrambled with the key of the file, and it says that the file is
//!   a file of this handler: the bytes nine to eleven spell `adb`.
//! - **A password is prepared before it is used.** The specification
//!   asks for the password to be prepared with **SASLprep**, and that
//!   is a set of rules about the letters of the world that this tree
//!   does not have. A password of plain ASCII is its own preparation,
//!   and a password with any other letter is **refused**.

use craftkit_sdk::EngineError;

use crate::crypt::Scheme;

/// The **length** of the user's entry of revision six: forty-eight
/// bytes.
const ENTRY: usize = 48;

/// The **length** of the key of the file: thirty-two bytes.
const KEY: usize = 32;

/// The **length** of the vector of the cipher: sixteen bytes.
const VECTOR: usize = 16;

/// The **length** of a password of revision six, in bytes.
const MOST_CHARACTERS: usize = 127;

/// The **hash** of the handler of revision six.
///
/// The input is hashed, and then sixty-four times over the bytes are
/// scrambled with the first sixteen bytes of the hash and chained from
/// the next sixteen, and the hash that comes out is hashed again — and
/// the hash that is used is the one the scrambled bytes name: the
/// first sixteen of them, taken as a number, divided by three.
///
/// The **user key** is the user's entry, and it is beside the
/// password in the bytes that are scrambled — and it is there only
/// when the owner's password is the one that is being worked out.
fn hash_of_revision_six(password: &[u8], input: &[u8], user_key: Option<&[u8]>) -> [u8; KEY] {
    let mut key: Vec<u8> = crate::sha2::sha256(input).to_vec();
    let mut round: u32 = 0;
    loop {
        // The **bytes that are scrambled**: sixty-four repetitions of
        // the password, the hash, and the user's entry.
        let mut scrambled_bytes: Vec<u8> = Vec::with_capacity(
            64 * (password.len() + key.len() + user_key.map(|user| user.len()).unwrap_or(0)),
        );
        for _ in 0..64 {
            scrambled_bytes.extend_from_slice(password);
            scrambled_bytes.extend_from_slice(&key);
            if let Some(user) = user_key {
                scrambled_bytes.extend_from_slice(user);
            }
        }
        // The **scrambling**: AES-128, chained, and with no padding.
        let scrambled = crate::aes::encrypt_raw(&key[0..VECTOR], &key[VECTOR..2 * VECTOR], &scrambled_bytes);
        // The **hash that is next**: the first sixteen bytes of the
        // scrambling, as a number, divided by three.
        let mut first = [0_u8; 16];
        first.copy_from_slice(&scrambled[0..16]);
        let number = u128::from_be_bytes(first);
        let next: Vec<u8> = match number % 3 {
            0 => crate::sha2::sha256(&scrambled).to_vec(),
            1 => crate::sha2::sha384(&scrambled).to_vec(),
            _ => crate::sha2::sha512(&scrambled).to_vec(),
        };
        // The **rounds**: sixty-four of them always, and after that
        // only while the last byte of the scrambling says so.
        let last = u32::from(scrambled[scrambled.len() - 1]);
        let stop = round >= 64 && last <= round - 32;
        key = next;
        round += 1;
        if stop {
            break;
        }
    }
    let mut out = [0_u8; KEY];
    out.copy_from_slice(&key[0..KEY]);
    out
}

/// The **bytes** of a password: its UTF-8 form, as the specification
/// asks for it.
///
/// The specification asks for the password to be prepared with
/// **SASLprep**, and that is not here. A password of plain ASCII is
/// its own preparation, and a password with any other letter is
/// refused.
fn password_bytes(password: &str, command: &str) -> Result<Vec<u8>, EngineError> {
    for character in password.chars() {
        if !(' '..='~').contains(&character) {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!(
                    "the password holds `{character}`, and the preparation of the specification is not here"
                ),
            });
        }
    }
    let bytes = password.as_bytes().to_vec();
    if bytes.len() > MOST_CHARACTERS {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a password of revision six is at most {MOST_CHARACTERS} bytes"),
        });
    }
    Ok(bytes)
}

/// The **key of the file**, from the password that opens it.
///
/// The password may be the user's or the owner's, and the key comes
/// out of the user's encrypted entry or out of the owner's.
pub fn opens_of_revision_six(password: &str, scheme: &Scheme, command: &str) -> Result<Vec<u8>, EngineError> {
    let bytes = password_bytes(password, command)?;
    if scheme.user.len() < ENTRY || scheme.owner.len() < ENTRY {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the entries of the file are not forty-eight bytes".to_string(),
        });
    }
    let user = &scheme.user[0..ENTRY];
    let owner = &scheme.owner[0..ENTRY];
    // The **user's** password: the hash of the password and of the
    // validation salt, and it must be the first thirty-two bytes of
    // the user's entry.
    let mut input: Vec<u8> = bytes.clone();
    input.extend_from_slice(&user[32..40]);
    let hash = hash_of_revision_six(&bytes, &input, None);
    if hash.as_slice() == &user[0..32] {
        // The **key salt**: the hash of the password and of the salt is
        // the key the user's encrypted entry is opened with.
        let mut input: Vec<u8> = bytes.clone();
        input.extend_from_slice(&user[40..48]);
        let intermediate = hash_of_revision_six(&bytes, &input, None);
        let key = crate::aes::decrypt_raw(&intermediate, &[0_u8; VECTOR], &scheme.user_encrypted);
        return the_permissions_are_right(key, scheme, command);
    }
    // The **owner's** password: the user's entry is beside it.
    let mut input: Vec<u8> = bytes.clone();
    input.extend_from_slice(&owner[32..40]);
    input.extend_from_slice(user);
    let hash = hash_of_revision_six(&bytes, &input, Some(user));
    if hash.as_slice() == &owner[0..32] {
        let mut input: Vec<u8> = bytes.clone();
        input.extend_from_slice(&owner[40..48]);
        input.extend_from_slice(user);
        let intermediate = hash_of_revision_six(&bytes, &input, Some(user));
        let key = crate::aes::decrypt_raw(&intermediate, &[0_u8; VECTOR], &scheme.owner_encrypted);
        return the_permissions_are_right(key, scheme, command);
    }
    Err(EngineError::Failed {
        id: command.to_string(),
        message: "the password does not open the file".to_string(),
    })
}

/// The **permissions** of the file, checked against the key.
///
/// The block of the file is scrambled with the key of the file, and
/// the bytes nine to eleven spell `adb` — and the first four bytes
/// are the permissions the file says.
fn the_permissions_are_right(key: Vec<u8>, scheme: &Scheme, command: &str) -> Result<Vec<u8>, EngineError> {
    if scheme.permissions_block.len() < 16 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the file holds no permissions of its own".to_string(),
        });
    }
    let block = crate::aes::decrypt_ecb(&key, &scheme.permissions_block);
    if &block[9..12] != b"adb" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the permissions of the file do not say that it is a file of this handler".to_string(),
        });
    }
    let mut bits = [0_u8; 4];
    bits.copy_from_slice(&block[0..4]);
    let permissions = i32::from_le_bytes(bits);
    if permissions != scheme.permissions as i32 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the permissions of the file are not the permissions it says".to_string(),
        });
    }
    Ok(key)
}
