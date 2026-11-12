//! **SHA-2**: the hashes of the standard security handler of revision
//! six.
//!
//! The handler of revision six computes its hashes with **SHA-256**,
//! **SHA-384** or **SHA-512**, and the one it uses is worked out from
//! the bytes that are hashed: the first sixteen bytes of them, taken
//! as a number, divided by three, say which hash is next.
//!
//! # The rules
//!
//! - **A hash of nothing is the hash of nothing.** The empty message
//!   has a hash, and it is the one the standard says.
//! - **The bytes are the message**, and the message is put at the end
//!   of the block that is filled: the length of the message in bits,
//!   big-endian, and then a byte that says how many bytes of the
//!   length are there.

/// The **hash** of a message under SHA-256.
pub fn sha256(message: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] =
        [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    // The **blocks** of the message, and the rest of it.
    let (chunks, rest) = message.as_chunks::<64>();
    for chunk in chunks {
        compress256(&mut state, chunk);
    }
    // The **last** block: the rest of the message, the length, and the
    // byte that says how much of the length there is.
    let mut last = [0_u8; 64];
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] = 0x80;
    let bits = (message.len() as u64) * 8;
    if rest.len() + 1 + 8 > 64 {
        compress256(&mut state, &last);
        last = [0_u8; 64];
    }
    last[56..64].copy_from_slice(&bits.to_be_bytes());
    compress256(&mut state, &last);
    let mut out = [0_u8; 32];
    for (index, word) in state.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The **hash** of a message under SHA-512.
pub fn sha512(message: &[u8]) -> [u8; 64] {
    let mut state: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    // The **blocks** of the message, and the rest of it.
    let (chunks, rest) = message.as_chunks::<128>();
    for chunk in chunks {
        compress512(&mut state, chunk);
    }
    let mut last = [0_u8; 128];
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] = 0x80;
    let bits = (message.len() as u128) * 8;
    if rest.len() + 1 + 16 > 128 {
        compress512(&mut state, &last);
        last = [0_u8; 128];
    }
    last[112..128].copy_from_slice(&bits.to_be_bytes());
    compress512(&mut state, &last);
    let mut out = [0_u8; 64];
    for (index, word) in state.iter().enumerate() {
        out[index * 8..index * 8 + 8].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The **hash** of a message under SHA-384: the hash of SHA-512 with
/// another beginning, cut short.
pub fn sha384(message: &[u8]) -> [u8; 48] {
    let mut state: [u64; 8] = [
        0xcbbb9d5dc1059ed8,
        0x629a292a367cd507,
        0x9159015a3070dd17,
        0x152fecd8f70e5939,
        0x67332667ffc00b31,
        0x8eb44a8768581511,
        0xdb0c2e0d64f98fa7,
        0x47b5481dbefa4fa4,
    ];
    // The **blocks** of the message, and the rest of it.
    let (chunks, rest) = message.as_chunks::<128>();
    for chunk in chunks {
        compress512(&mut state, chunk);
    }
    let mut last = [0_u8; 128];
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] = 0x80;
    let bits = (message.len() as u128) * 8;
    if rest.len() + 1 + 16 > 128 {
        compress512(&mut state, &last);
        last = [0_u8; 128];
    }
    last[112..128].copy_from_slice(&bits.to_be_bytes());
    compress512(&mut state, &last);
    let mut out = [0_u8; 48];
    for (index, word) in state.iter().take(6).enumerate() {
        out[index * 8..index * 8 + 8].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// The **constants** of SHA-256: the first bits of the fractional
/// parts of the cube roots of the first sixty-four primes.
const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// The **constants** of SHA-512: the first bits of the fractional
/// parts of the cube roots of the first eighty primes.
const K512: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

/// The **right** turn of a number.
fn rotr256(value: u32, places: u32) -> u32 {
    value.rotate_right(places)
}

/// The **right** turn of a long number.
fn rotr512(value: u64, places: u32) -> u64 {
    value.rotate_right(places)
}

/// One **block** of SHA-256.
fn compress256(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut words = [0_u32; 64];
    for index in 0..16 {
        words[index] = u32::from_be_bytes([
            block[index * 4],
            block[index * 4 + 1],
            block[index * 4 + 2],
            block[index * 4 + 3],
        ]);
    }
    for index in 16..64 {
        let first = words[index - 15];
        let second = words[index - 2];
        let small0 = rotr256(first, 7) ^ rotr256(first, 18) ^ (first >> 3);
        let small1 = rotr256(second, 17) ^ rotr256(second, 19) ^ (second >> 10);
        words[index] =
            words[index - 16].wrapping_add(small0).wrapping_add(words[index - 7]).wrapping_add(small1);
    }
    let mut work = *state;
    for index in 0..64 {
        let first = work[0];
        let second = work[1];
        let third = work[2];
        let fourth = work[3];
        let fifth = work[4];
        let sixth = work[5];
        let seventh = work[6];
        let eighth = work[7];
        let big1 = rotr256(fifth, 6) ^ rotr256(fifth, 11) ^ rotr256(fifth, 25);
        let choose = (fifth & sixth) ^ ((!fifth) & seventh);
        let big0 = rotr256(first, 2) ^ rotr256(first, 13) ^ rotr256(first, 22);
        let majority = (first & second) ^ (first & third) ^ (second & third);
        let one = eighth
            .wrapping_add(big1)
            .wrapping_add(choose)
            .wrapping_add(K256[index])
            .wrapping_add(words[index]);
        let two = big0.wrapping_add(majority);
        work[7] = seventh;
        work[6] = sixth;
        work[5] = fifth;
        work[4] = fourth.wrapping_add(one);
        work[3] = third;
        work[2] = second;
        work[1] = first;
        work[0] = one.wrapping_add(two);
    }
    for index in 0..8 {
        state[index] = state[index].wrapping_add(work[index]);
    }
}

/// One **block** of SHA-512.
fn compress512(state: &mut [u64; 8], block: &[u8; 128]) {
    let mut words = [0_u64; 80];
    for index in 0..16 {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&block[index * 8..index * 8 + 8]);
        words[index] = u64::from_be_bytes(bytes);
    }
    for index in 16..80 {
        let first = words[index - 15];
        let second = words[index - 2];
        let small0 = rotr512(first, 1) ^ rotr512(first, 8) ^ (first >> 7);
        let small1 = rotr512(second, 19) ^ rotr512(second, 61) ^ (second >> 6);
        words[index] =
            words[index - 16].wrapping_add(small0).wrapping_add(words[index - 7]).wrapping_add(small1);
    }
    let mut work = *state;
    for index in 0..80 {
        let first = work[0];
        let second = work[1];
        let third = work[2];
        let fourth = work[3];
        let fifth = work[4];
        let sixth = work[5];
        let seventh = work[6];
        let eighth = work[7];
        let big1 = rotr512(fifth, 14) ^ rotr512(fifth, 18) ^ rotr512(fifth, 41);
        let choose = (fifth & sixth) ^ ((!fifth) & seventh);
        let big0 = rotr512(first, 28) ^ rotr512(first, 34) ^ rotr512(first, 39);
        let majority = (first & second) ^ (first & third) ^ (second & third);
        let one = eighth
            .wrapping_add(big1)
            .wrapping_add(choose)
            .wrapping_add(K512[index])
            .wrapping_add(words[index]);
        let two = big0.wrapping_add(majority);
        work[7] = seventh;
        work[6] = sixth;
        work[5] = fifth;
        work[4] = fourth.wrapping_add(one);
        work[3] = third;
        work[2] = second;
        work[1] = first;
        work[0] = one.wrapping_add(two);
    }
    for index in 0..8 {
        state[index] = state[index].wrapping_add(work[index]);
    }
}

#[cfg(test)]
mod tests {
    use super::{sha256, sha384, sha512};

    #[test]
    fn the_hash_of_a_is_the_one_of_the_standard() {
        assert_eq!(
            sha256(b"abc").to_vec(),
            vec![
                0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
                0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
                0x15, 0xad
            ]
        );
        assert_eq!(
            sha384(b"abc").to_vec(),
            vec![
                0xcb, 0x00, 0x75, 0x3f, 0x45, 0xa3, 0x5e, 0x8b, 0xb5, 0xa0, 0x3d, 0x69, 0x9a, 0xc6, 0x50,
                0x07, 0x27, 0x2c, 0x32, 0xab, 0x0e, 0xde, 0xd1, 0x63, 0x1a, 0x8b, 0x60, 0x5a, 0x43, 0xff,
                0x5b, 0xed, 0x80, 0x86, 0x07, 0x2b, 0xa1, 0xe7, 0xcc, 0x23, 0x58, 0xba, 0xec, 0xa1, 0x34,
                0xc8, 0x25, 0xa7
            ]
        );
        assert_eq!(
            sha512(b"abc").to_vec(),
            vec![
                0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73, 0x49, 0xae, 0x20, 0x41,
                0x31, 0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9, 0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55,
                0xd3, 0x9a, 0x21, 0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23, 0xa3,
                0xfe, 0xeb, 0xbd, 0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8, 0x0e, 0x2a, 0x9a, 0xc9, 0x4f,
                0xa5, 0x4c, 0xa4, 0x9f
            ]
        );
    }

    #[test]
    fn the_hash_of_nothing_is_the_one_of_the_standard() {
        assert_eq!(
            sha256(b"").to_vec(),
            vec![
                0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
                0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
                0xb8, 0x55
            ]
        );
        assert_eq!(
            sha512(b"").to_vec(),
            vec![
                0xcf, 0x83, 0xe1, 0x35, 0x7e, 0xef, 0xb8, 0xbd, 0xf1, 0x54, 0x28, 0x50, 0xd6, 0x6d, 0x80,
                0x07, 0xd6, 0x20, 0xe4, 0x05, 0x0b, 0x57, 0x15, 0xdc, 0x83, 0xf4, 0xa9, 0x21, 0xd3, 0x6c,
                0xe9, 0xce, 0x47, 0xd0, 0xd1, 0x3c, 0x5d, 0x85, 0xf2, 0xb0, 0xff, 0x83, 0x18, 0xd2, 0x87,
                0x7e, 0xec, 0x2f, 0x63, 0xb9, 0x31, 0xbd, 0x47, 0x41, 0x7a, 0x81, 0xa5, 0x38, 0x32, 0x7a,
                0xf9, 0x27, 0xda, 0x3e
            ]
        );
    }

    #[test]
    fn a_message_of_many_blocks_is_hashed() {
        // A million times "a" is the message the standard gives.
        let message = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256(&message).to_vec(),
            vec![
                0xcd, 0xc7, 0x6e, 0x5c, 0x99, 0x14, 0xfb, 0x92, 0x81, 0xa1, 0xc7, 0xe2, 0x84, 0xd7, 0x3e,
                0x67, 0xf1, 0x80, 0x9a, 0x48, 0xa4, 0x97, 0x20, 0x0e, 0x04, 0x6d, 0x39, 0xcc, 0xc7, 0x11,
                0x2c, 0xd0
            ]
        );
    }
}
