//! The **RC4** of the old scheme.
//!
//! RC4 is a **stream cipher**: a key is turned into a stream of bytes
//! as long as the data to be scrambled, and the data is scrambled by
//! a byte at a time with that stream. The same key and the same
//! stream scramble and unscramble, so the cipher is its own inverse.
//!
//! The cipher is written from its public description: a **permutation**
//! of the numbers from 0 to 255 is made from the key, and the stream
//! comes out of the permutation as it is walked.
//!
//! # The key is short
//!
//! The old scheme gives RC4 a key of **40 bits** — five bytes — and
//! the key is short enough to be broken by searching it. That is why
//! the scheme was replaced, and the tree says so where the file says
//! it: a workbook of the old encryption is a workbook of 2007.

/// The **permutation** RC4 walks: the numbers from 0 to 255.
const BYTES: usize = 256;

/// The **stream** of RC4, from a key.
///
/// The stream is as long as the data that is scrambled, and it is made
/// once: the permutation is made from the key, and then the stream is
/// the walk of it.
pub fn stream(key: &[u8], length: usize) -> Vec<u8> {
    if key.is_empty() {
        return Vec::new();
    }
    let mut permutation: [u8; BYTES] = [0; BYTES];
    for (index, slot) in permutation.iter_mut().enumerate() {
        *slot = index as u8;
    }
    // The **key schedule**: the permutation is shuffled by the key.
    let mut j: u8 = 0;
    for index in 0..BYTES {
        j = j.wrapping_add(permutation[index]).wrapping_add(key[index % key.len()]);
        permutation.swap(index, j as usize);
    }
    // The **walk**: the stream, a byte at a time.
    let mut out = Vec::with_capacity(length);
    let mut i: u8 = 0;
    let mut j: u8 = 0;
    for _ in 0..length {
        i = i.wrapping_add(1);
        j = j.wrapping_add(permutation[i as usize]);
        permutation.swap(i as usize, j as usize);
        let at = permutation[i as usize].wrapping_add(permutation[j as usize]);
        out.push(permutation[at as usize]);
    }
    out
}

/// The **bytes** the cipher opens: the data, with the stream of the key
/// laid over it.
pub fn open(key: &[u8], data: &[u8]) -> Vec<u8> {
    let stream = stream(key, data.len());
    data.iter().zip(stream.iter()).map(|(byte, mask)| byte ^ mask).collect()
}
