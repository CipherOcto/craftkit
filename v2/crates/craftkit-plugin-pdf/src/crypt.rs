//! The **standard security handler**: the way a PDF file is scrambled.
//!
//! A file whose trailer names an **encryption** is a file whose strings
//! and streams are scrambled. The file says how: the **encryption
//! dictionary** names the filter, the version and the revision of the
//! scheme, the length of the key, the permissions, and the two entries
//! that carry the scramble — the owner's entry and the user's entry.
//!
//! This module is the **handler** itself: the hash, the scramble, and
//! the keys that come out of them. The command `pdf.file.decrypt`
//! undoes the scramble with it (§577), and the command
//! `pdf.file.encrypt` makes one with it (§578).
//!
//! # The rules
//!
//! - **The revision of two and of revision three are read.** The key
//!   of the file is the hash of the padded password, of the owner's
//!   entry, of the permissions and of the file's identifier — and, from
//!   revision three, that hash is hashed again fifty times.
//! - **A password that does not open the file is refused.** The file
//!   keeps the user's entry, and the entry is worked out from the
//!   password.
//! - **The owner's password opens the file too.** The owner's entry
//!   holds the user's password, and a password that is the owner's is
//!   read as the owner's and the user's password is taken from it.
//! - **A file scrambled with AES is not read.** The standard handler
//!   of revision six and above uses AES, and this tree does not have
//!   it.
//!
//! # What it does not do
//!
//! - **The permissions are not enforced.** The bits the file holds say
//!   what a person may do with it, and this module reads them and does
//!   not act on them.
//! - **The metadata is not treated apart.** A file whose encryption
//!   says its metadata is not scrambled is not handled differently here.

use craftkit_sdk::serde_json::{Map, Value as Json};
use craftkit_sdk::{Document, EngineError};

/// The **padding** the specification adds to a password.
///
/// A password is at most thirty-two bytes long, and the bytes that are
/// missing are filled with these.
const PADDING: [u8; 32] = [
    0x28, 0xbf, 0x4e, 0x5e, 0x4e, 0x75, 0x8a, 0x41, 0x64, 0x00, 0x4e, 0x56, 0xff, 0xfa, 0x01, 0x08, 0x2e,
    0x2e, 0x00, 0xb6, 0xd0, 0x68, 0x3e, 0x80, 0x2f, 0x0c, 0xa9, 0xfe, 0x64, 0x53, 0x69, 0x7a,
];

/// The **length** of a password, in bytes.
const PASSWORD_LENGTH: usize = 32;

/// The **times** the key is hashed again, for a revision of three or
/// more.
const HASH_ROUNDS: usize = 50;

/// The **smallest** key a file may have, in bytes.
const SMALLEST_KEY: usize = 5;

/// The **largest** key an object may have, in bytes.
const LARGEST_OBJECT_KEY: usize = 16;

/// The **times** the user's entry is scrambled again.
const USER_ROUNDS: u8 = 19;

/// The **shifts** of the rounds of **MD5**.
const SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5,
    9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6,
    10, 15, 21, 6, 10, 15, 21,
];

/// The **constants** of the rounds of **MD5**.
const CONSTANTS: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

/// The **hash** of a message: sixteen bytes.
///
/// This is **MD5**, and it is here because the standard security
/// handler is written in it: the key of a file is the hash of the
/// password and of what the file says.
pub fn md5(message: &[u8]) -> [u8; 16] {
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];
    let mut padded: Vec<u8> = message.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((message.len() as u64).wrapping_mul(8)).to_le_bytes());
    for block in padded.chunks(64) {
        let mut words: [u32; 16] = [0; 16];
        for (index, word) in words.iter_mut().enumerate() {
            let at = index * 4;
            *word = u32::from_le_bytes([block[at], block[at + 1], block[at + 2], block[at + 3]]);
        }
        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);
        for index in 0..64 {
            let (mixed, word) = match index / 16 {
                0 => ((b & c) | (!b & d), index),
                1 => ((d & b) | (!d & c), (5 * index + 1) % 16),
                2 => (b ^ c ^ d, (3 * index + 5) % 16),
                _ => (c ^ (b | !d), (7 * index) % 16),
            };
            let sum = a.wrapping_add(mixed).wrapping_add(CONSTANTS[index]).wrapping_add(words[word]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(sum.rotate_left(SHIFTS[index]));
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
    let mut out: [u8; 16] = [0; 16];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

/// The **scramble** of bytes under a key.
///
/// This is **RC4**, the way the standard security handler of revision
/// two and three scrambles: a key, a stream of bytes, and a stream of
/// bytes back that is the same one with the key put over it.
pub fn rc4(key: &[u8], bytes: &[u8]) -> Vec<u8> {
    if key.is_empty() {
        return Vec::new();
    }
    let mut state: [u8; 256] = [0; 256];
    for (index, byte) in state.iter_mut().enumerate() {
        *byte = index as u8;
    }
    let mut j: u8 = 0;
    for index in 0..256 {
        j = j.wrapping_add(state[index]).wrapping_add(key[index % key.len()]);
        state.swap(index, usize::from(j));
    }
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    for byte in bytes {
        i = i.wrapping_add(1);
        j = j.wrapping_add(state[usize::from(i)]);
        state.swap(usize::from(i), usize::from(j));
        let at = usize::from(state[usize::from(i)].wrapping_add(state[usize::from(j)]));
        out.push(byte ^ state[at]);
    }
    out
}

/// The **password**, padded to the length the specification says.
pub fn padded(password: &[u8]) -> Vec<u8> {
    let password = &password[..password.len().min(PASSWORD_LENGTH)];
    let mut out: Vec<u8> = password.to_vec();
    out.extend_from_slice(&PADDING[..PASSWORD_LENGTH - password.len()]);
    out
}

/// The **password** with the padding taken off.
pub fn unpadded(bytes: &[u8]) -> Vec<u8> {
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == PADDING[0] {
            return bytes[..index].to_vec();
        }
    }
    bytes.to_vec()
}

/// The **length** of the key of a file, in bytes.
pub fn key_length(revision: i64, length_bits: i64) -> usize {
    if revision == 2 {
        return SMALLEST_KEY;
    }
    let bytes = length_bits / 8;
    if bytes < SMALLEST_KEY as i64 {
        return SMALLEST_KEY;
    }
    if bytes > LARGEST_OBJECT_KEY as i64 {
        return LARGEST_OBJECT_KEY;
    }
    bytes as usize
}

/// The **key of the file**: the key every object of the file is
/// scrambled under.
///
/// The key is the hash of the password, of the owner's entry, of the
/// permissions and of the file's own identifier — and, for a revision
/// of three or more, the hash of that hash, fifty times over.
pub fn file_key(password: &[u8], scheme: &Scheme) -> Vec<u8> {
    let mut message: Vec<u8> = padded(password);
    message.extend_from_slice(&scheme.owner);
    message.extend_from_slice(&(scheme.permissions as u32).to_le_bytes());
    message.extend_from_slice(&scheme.identifier);
    if scheme.revision >= 4 && !scheme.encrypt_metadata {
        message.extend_from_slice(&[0xff, 0xff, 0xff, 0xff]);
    }
    let mut digest = md5(&message);
    let length = key_length(scheme.revision, scheme.length_bits);
    if scheme.revision >= 3 {
        for _ in 0..HASH_ROUNDS {
            digest = md5(&digest[..length]);
        }
    }
    digest[..length].to_vec()
}

/// The **key of an object**: the key the bytes of one object are
/// scrambled under.
///
/// The key of an object is the hash of the key of the file and of the
/// number of the object — and of the generation it has.
pub fn object_key(key: &[u8], number: u64, generation: u64) -> Vec<u8> {
    let mut message: Vec<u8> = key.to_vec();
    message.push((number & 0xff) as u8);
    message.push(((number >> 8) & 0xff) as u8);
    message.push(((number >> 16) & 0xff) as u8);
    message.push((generation & 0xff) as u8);
    message.push(((generation >> 8) & 0xff) as u8);
    let digest = md5(&message);
    let length = (key.len() + 5).min(LARGEST_OBJECT_KEY);
    digest[..length].to_vec()
}

/// The **user's entry** of a file: what the file says the key of the
/// user password is.
///
/// For a revision of two it is the padding, scrambled with the key of
/// the file; for a revision of three or more it is the hash of the
/// padding and of the identifier, scrambled nineteen times over.
pub fn user_entry(key: &[u8], identifier: &[u8], revision: i64) -> Vec<u8> {
    if revision == 2 {
        return rc4(key, &PADDING);
    }
    let mut message: Vec<u8> = PADDING.to_vec();
    message.extend_from_slice(identifier);
    let mut digest: Vec<u8> = md5(&message).to_vec();
    digest = rc4(key, &digest);
    for round in 1..=USER_ROUNDS {
        let scrambled: Vec<u8> = key.iter().map(|byte| byte ^ round).collect();
        digest = rc4(&scrambled, &digest);
    }
    digest
}

/// The **user's password** of a file, when the password given is the
/// owner's.
///
/// The owner's entry holds the user's password, scrambled under a key
/// of the owner's own: the hash of the owner's password.
pub fn user_password_of(password: &[u8], scheme: &Scheme) -> Vec<u8> {
    let mut digest = md5(&padded(password));
    let length = key_length(scheme.revision, scheme.length_bits);
    if scheme.revision >= 3 {
        for _ in 0..HASH_ROUNDS {
            digest = md5(&digest[..length]);
        }
    }
    let key = digest[..length].to_vec();
    let mut user = rc4(&key, &scheme.owner);
    if scheme.revision >= 3 {
        for round in 1..=USER_ROUNDS {
            let scrambled: Vec<u8> = key.iter().map(|byte| byte ^ round).collect();
            user = rc4(&scrambled, &user);
        }
    }
    unpadded(&user)
}

/// Whether a **password** opens a file.
///
/// The file holds the user's entry, and the entry is worked out from
/// the password: a password whose entry is not the file's entry does
/// not open the file.
pub fn opens(password: &[u8], scheme: &Scheme) -> Option<Vec<u8>> {
    let key = file_key(password, scheme);
    if matches(&key, scheme) {
        return Some(key);
    }
    // The password may be the **owner's**: the owner's entry holds the
    // user's password, and the file is opened with that.
    let user_password = user_password_of(password, scheme);
    if user_password.is_empty() {
        return None;
    }
    let key = file_key(&user_password, scheme);
    if matches(&key, scheme) { Some(key) } else { None }
}

/// The **owner's entry** of a file that is made: the user's password,
/// scrambled under a key of the owner's own.
///
/// The owner's entry holds the user's password, and the key it is
/// scrambled under is the hash of the owner's password.
pub fn owner_entry(owner_password: &[u8], user_password: &[u8], revision: i64, length_bits: i64) -> Vec<u8> {
    let mut digest = md5(&padded(owner_password));
    let length = key_length(revision, length_bits);
    if revision >= 3 {
        for _ in 0..HASH_ROUNDS {
            digest = md5(&digest[..length]);
        }
    }
    let key = digest[..length].to_vec();
    let mut owner = rc4(&key, &padded(user_password));
    if revision >= 3 {
        for round in 1..=USER_ROUNDS {
            let scrambled: Vec<u8> = key.iter().map(|byte| byte ^ round).collect();
            owner = rc4(&scrambled, &owner);
        }
    }
    owner
}

/// Whether a **key** is the key the file's user's entry says.
pub fn matches(key: &[u8], scheme: &Scheme) -> bool {
    let expected = user_entry(key, &scheme.identifier, scheme.revision);
    let length = expected.len().min(scheme.user.len());
    expected[..length] == scheme.user[..length]
}

/// The **bytes** of a string of the file.
///
/// A string of a PDF file is a sequence of bytes, and the payload
/// holds it as a text of one character for each byte.
pub fn bytes_of(value: &Json) -> Option<Vec<u8>> {
    let text = value.get("string")?.as_str()?;
    Some(text.chars().map(|character| character as u32 as u8).collect())
}

/// The **value** with its strings scrambled.
///
/// The strings of an object are scrambled under the key of the object,
/// and the strings inside a dictionary or an array are scrambled too.
/// The scramble is its own inverse, so this both makes the scramble and
/// undoes it.
pub fn scrambled(value: &Json, key: &[u8]) -> Json {
    match value {
        Json::Object(map) => {
            if let Some(Json::String(text)) = map.get("string") {
                // A **text**: the bytes of the string are what the key
                // puts over.
                let bytes: Vec<u8> = text.chars().map(|character| character as u32 as u8).collect();
                let plain = rc4(key, &bytes);
                let text: String = plain.iter().map(|byte| *byte as char).collect();
                let mut out = Map::new();
                out.insert("string".to_string(), Json::String(text));
                return Json::Object(out);
            }
            if let Some(Json::Array(pairs)) = map.get("entries") {
                // A **dictionary**: the pairs in it are values too.
                let mut out = Map::new();
                let pairs: Vec<Json> = pairs
                    .iter()
                    .map(|pair| match pair.as_array() {
                        Some(pair) => Json::Array(pair.iter().map(|item| scrambled(item, key)).collect()),
                        None => pair.clone(),
                    })
                    .collect();
                out.insert("entries".to_string(), Json::Array(pairs));
                return Json::Object(out);
            }
            // Any other **object**: what it holds are values.
            let mut out = Map::new();
            for (name, item) in map {
                out.insert(name.clone(), scrambled(item, key));
            }
            Json::Object(out)
        }
        Json::Array(list) => Json::Array(list.iter().map(|item| scrambled(item, key)).collect()),
        other => other.clone(),
    }
}

/// The **encryption** of a file: what its dictionary says, and the
/// entries the handler needs.
pub struct Scheme {
    /// The number of the object that holds the encryption.
    pub number: u64,
    /// The revision of the scheme.
    pub revision: i64,
    /// The length of the key, in bits.
    pub length_bits: i64,
    /// The bits that say what a person may do with the file.
    pub permissions: i64,
    /// Whether the metadata is scrambled as well.
    pub encrypt_metadata: bool,
    /// The way the strings are scrambled.
    pub strings: String,
    /// The owner's entry: thirty-two bytes.
    pub owner: Vec<u8>,
    /// The user's entry: thirty-two bytes.
    pub user: Vec<u8>,
    /// The **identifier** of the file: the first half of the pair the
    /// trailer holds.
    pub identifier: Vec<u8>,
}

/// The **scheme** of a file: its encryption, read from the document.
pub fn scheme_of(document: &Document, command: &str) -> Result<Option<Scheme>, EngineError> {
    let Some(value) = crate::trailer::entry_of(document, "Encrypt") else { return Ok(None) };
    let Some(number) = crate::number_of(&value) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the file names its encryption, and it names it in no way".to_string(),
        });
    };
    let Some(node) =
        document.walk().find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
    else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!(
                "the file names the encryption of the object `{number}`, and that object is not in it"
            ),
        });
    };
    let entries = crate::entries_of(&node.data);
    let encryption = crate::encrypt::encryption_from(&entries);
    let owner =
        entries.iter().find(|(key, _)| key == "O").and_then(|(_, value)| bytes_of(value)).unwrap_or_default();
    let user =
        entries.iter().find(|(key, _)| key == "U").and_then(|(_, value)| bytes_of(value)).unwrap_or_default();
    Ok(Some(Scheme {
        number,
        identifier: identifier_of(document),
        revision: encryption.revision,
        length_bits: encryption.length_bits,
        permissions: encryption.permissions,
        encrypt_metadata: encryption.encrypt_metadata,
        strings: encryption.strings,
        owner,
        user,
    }))
}

/// The **identifier** of a file: the first half of the pair the
/// trailer holds.
pub fn identifier_of(document: &Document) -> Vec<u8> {
    let Some(value) = crate::trailer::entry_of(document, "ID") else { return Vec::new() };
    let Some(list) = value.as_array() else { return Vec::new() };
    let Some(first) = list.first() else { return Vec::new() };
    bytes_of(first).unwrap_or_default()
}

/// The **number** an entry of the trailer holds.
pub fn number_of(document: &Document, key: &str) -> Option<u64> {
    let value = crate::trailer::entry_of(document, key)?;
    crate::number_of(&value)
}
