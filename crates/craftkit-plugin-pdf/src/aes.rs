//! **AES**: the cipher of a file scrambled with AES.
//!
//! A file of revision four or above may be scrambled with **AES**
//! instead of RC4, and the scheme says so in its crypt filters: the
//! method `AESV2` is AES with a key of 128 bits.
//!
//! The cipher is **AES** in **CBC** mode: the bytes of a stream begin
//! with the sixteen bytes of the **vector**, and what follows are
//! blocks of sixteen bytes that are each one the block before it
//! scrambled. The last block carries the **padding**: a byte says how
//! many bytes of padding there are, and they are all that byte.
//!
//! # The rules
//!
//! - **A key that is not sixteen bytes is no key.** The cipher needs
//!   a key of 128 bits and a vector of the same length, and anything
//!   else is refused: the answer is empty.
//! - **Bytes that are not a whole block are not read.** A stream whose
//!   length is not a multiple of the block has a tail that is not a
//!   block, and the tail is left out.
//! - **The padding is only taken off when it is padding.** The last
//!   byte says how many bytes there are, and every one of them must be
//!   that byte; otherwise the padding is left where it is.

/// The **length** of a block of AES, in bytes.
const BLOCK: usize = 16;

/// The **length** of the key of AES-128, in bytes.
const KEY: usize = 16;

/// The **length** of the key of AES-256, in bytes.
const KEY_256: usize = 32;

/// Whether the bytes are a **key** of AES: one of 128 bits or of 256.
fn is_a_key(key: &[u8]) -> bool {
    key.len() == KEY || key.len() == KEY_256
}

/// The **product** of two bytes in the field of AES.
///
/// The field is the one of the specification: the bytes are eight
/// bits and the multiplication is the one that is not the one of the
/// numbers, with the reduction by the polynomial.
fn product(left: u8, right: u8) -> u8 {
    let mut result = 0_u8;
    let mut left = left;
    let mut right = right;
    for _ in 0..8 {
        if right & 1 == 1 {
            result ^= left;
        }
        let high = left & 0x80;
        left <<= 1;
        if high != 0 {
            left ^= 0x1b;
        }
        right >>= 1;
    }
    result
}

/// The **substitution box** of AES: the byte's own inverse in the
/// field, put through the affine transform.
fn box_of() -> [u8; 256] {
    let mut table = [0_u8; 256];
    for value in 1..=255_u16 {
        // The **inverse** of the byte: the power of two hundred and
        // fifty-four.
        let mut result = 1_u8;
        let mut base = value as u8;
        let mut power = 254_u32;
        while power > 0 {
            if power & 1 == 1 {
                result = product(result, base);
            }
            base = product(base, base);
            power >>= 1;
        }
        // The **affine** transform.
        let mut out = result;
        for shift in 1..=4 {
            out ^= result.rotate_left(shift);
        }
        out ^= 0x63;
        table[value as usize] = out;
    }
    table[0] = 0x63;
    table
}

/// The **inverted** substitution box: the box that undoes the one.
fn inverted_box() -> [u8; 256] {
    let table = box_of();
    let mut out = [0_u8; 256];
    for (value, byte) in table.iter().enumerate() {
        out[usize::from(*byte)] = value as u8;
    }
    out
}

/// The **round keys** of AES: eleven keys for a key of 128 bits, and
/// fifteen for a key of 256 bits.
///
/// The words of the key are taken four bytes at a time, and each word
/// after the first is the word before it in the key, at the distance
/// of the key's own length in words, with the **rotated** word put
/// through the box every fourth word — and every fourth word again
/// when the key is a key of 256 bits.
fn round_keys(key: &[u8], table: &[u8; 256]) -> Vec<[u8; BLOCK]> {
    let words_of_key = key.len() / 4;
    let rounds = words_of_key + 6;
    let mut words: Vec<[u8; 4]> = key.chunks(4).map(|word| [word[0], word[1], word[2], word[3]]).collect();
    let mut constant = 1_u8;
    for index in words_of_key..4 * (rounds + 1) {
        let mut word = words[index - 1];
        if index % words_of_key == 0 {
            word = [
                table[usize::from(word[1])] ^ constant,
                table[usize::from(word[2])],
                table[usize::from(word[3])],
                table[usize::from(word[0])],
            ];
            constant = product(constant, 2);
        } else if words_of_key > 6 && index % words_of_key == 4 {
            word = [
                table[usize::from(word[0])],
                table[usize::from(word[1])],
                table[usize::from(word[2])],
                table[usize::from(word[3])],
            ];
        }
        let before = words[index - words_of_key];
        words.push([before[0] ^ word[0], before[1] ^ word[1], before[2] ^ word[2], before[3] ^ word[3]]);
    }
    let mut keys: Vec<[u8; BLOCK]> = Vec::with_capacity(rounds + 1);
    for round in 0..=rounds {
        let mut key = [0_u8; BLOCK];
        for word in 0..4 {
            key[word * 4..word * 4 + 4].copy_from_slice(&words[round * 4 + word]);
        }
        keys.push(key);
    }
    keys
}

/// The **state** of the cipher: sixteen bytes, in columns.
type State = [u8; BLOCK];

/// **Add** the round key to the state.
fn add_key(state: &mut State, key: &[u8; BLOCK]) {
    for index in 0..BLOCK {
        state[index] ^= key[index];
    }
}

/// **Shift** the rows of the state the other way.
fn unshift_rows(state: &mut State) {
    let original = *state;
    for row in 1..4 {
        for column in 0..4 {
            state[row + 4 * column] = original[row + 4 * ((column + 4 - row) % 4)];
        }
    }
}

/// **Substitute** the bytes of the state the other way.
fn unsubstitute(state: &mut State, table: &[u8; 256]) {
    for byte in state.iter_mut() {
        *byte = table[usize::from(*byte)];
    }
}

/// **Mix** the columns of the state the other way.
fn unmix_columns(state: &mut State) {
    for column in 0..4 {
        let at = column * 4;
        let a = [state[at], state[at + 1], state[at + 2], state[at + 3]];
        state[at] = product(a[0], 14) ^ product(a[1], 11) ^ product(a[2], 13) ^ product(a[3], 9);
        state[at + 1] = product(a[0], 9) ^ product(a[1], 14) ^ product(a[2], 11) ^ product(a[3], 13);
        state[at + 2] = product(a[0], 13) ^ product(a[1], 9) ^ product(a[2], 14) ^ product(a[3], 11);
        state[at + 3] = product(a[0], 11) ^ product(a[1], 13) ^ product(a[2], 9) ^ product(a[3], 14);
    }
}

/// The **decryption** of one block.
fn decrypt_block(keys: &[[u8; BLOCK]], block: &[u8; BLOCK], table: &[u8; 256]) -> State {
    let rounds = keys.len() - 1;
    let mut state = *block;
    add_key(&mut state, &keys[rounds]);
    for round in (1..rounds).rev() {
        unshift_rows(&mut state);
        unsubstitute(&mut state, table);
        add_key(&mut state, &keys[round]);
        unmix_columns(&mut state);
    }
    unshift_rows(&mut state);
    unsubstitute(&mut state, table);
    add_key(&mut state, &keys[0]);
    state
}

/// The **decryption** of bytes under AES-128 in CBC mode.
///
/// The vector is the sixteen bytes at the front of the bytes, and what
/// follows are the blocks. The padding is taken off the end when it is
/// padding.
pub fn decrypt(key: &[u8], vector: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut out = decrypt_raw(key, vector, bytes);
    // The **padding**: the last byte says how many bytes it holds.
    if let Some(last) = out.last().copied() {
        let count = usize::from(last);
        if (1..=BLOCK).contains(&count) && count <= out.len() {
            let start = out.len() - count;
            if out[start..].iter().all(|byte| usize::from(*byte) == count) {
                out.truncate(start);
            }
        }
    }
    out
}

/// The **decryption** of bytes under AES in CBC mode, and **no
/// padding**.
///
/// The bytes of a file of revision six are not padded: they are the
/// bytes there are, and the padding is taken off only where there is
/// one.
pub fn decrypt_raw(key: &[u8], vector: &[u8], bytes: &[u8]) -> Vec<u8> {
    if !is_a_key(key) || vector.len() != BLOCK {
        return Vec::new();
    }
    let table = inverted_box();
    let keys = round_keys(key, &box_of());
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut previous: State = [0; BLOCK];
    previous.copy_from_slice(vector);
    for chunk in bytes.chunks(BLOCK) {
        if chunk.len() < BLOCK {
            // A tail that is not a block is not read.
            break;
        }
        let mut block: State = [0; BLOCK];
        block.copy_from_slice(chunk);
        let plain = decrypt_block(&keys, &block, &table);
        for index in 0..BLOCK {
            out.push(plain[index] ^ previous[index]);
        }
        previous = block;
    }
    out
}

/// The **decryption** of bytes under AES in ECB mode, and **no
/// padding**: every block stands alone.
pub fn decrypt_ecb(key: &[u8], bytes: &[u8]) -> Vec<u8> {
    if !is_a_key(key) {
        return Vec::new();
    }
    let table = inverted_box();
    let keys = round_keys(key, &box_of());
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks(BLOCK) {
        if chunk.len() < BLOCK {
            break;
        }
        let mut block: State = [0; BLOCK];
        block.copy_from_slice(chunk);
        let plain = decrypt_block(&keys, &block, &table);
        out.extend_from_slice(&plain);
    }
    out
}

/// The **encryption** of bytes under AES in ECB mode, and **no
/// padding**: every block stands alone.
pub fn encrypt_ecb(key: &[u8], bytes: &[u8]) -> Vec<u8> {
    if !is_a_key(key) {
        return Vec::new();
    }
    let table = box_of();
    let keys = round_keys(key, &table);
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    for chunk in bytes.chunks(BLOCK) {
        if chunk.len() < BLOCK {
            break;
        }
        let mut block: State = [0; BLOCK];
        block.copy_from_slice(chunk);
        let cipher = encrypt_block(&keys, &block, &table);
        out.extend_from_slice(&cipher);
    }
    out
}

/// **Shift** the rows of the state.
fn shift_rows(state: &mut State) {
    let original = *state;
    for row in 1..4 {
        for column in 0..4 {
            state[row + 4 * column] = original[row + 4 * ((column + row) % 4)];
        }
    }
}

/// **Substitute** the bytes of the state.
fn substitute(state: &mut State, table: &[u8; 256]) {
    for byte in state.iter_mut() {
        *byte = table[usize::from(*byte)];
    }
}

/// **Mix** the columns of the state.
fn mix_columns(state: &mut State) {
    for column in 0..4 {
        let at = column * 4;
        let a = [state[at], state[at + 1], state[at + 2], state[at + 3]];
        state[at] = product(a[0], 2) ^ product(a[1], 3) ^ a[2] ^ a[3];
        state[at + 1] = a[0] ^ product(a[1], 2) ^ product(a[2], 3) ^ a[3];
        state[at + 2] = a[0] ^ a[1] ^ product(a[2], 2) ^ product(a[3], 3);
        state[at + 3] = product(a[0], 3) ^ a[1] ^ a[2] ^ product(a[3], 2);
    }
}

/// The **encryption** of one block.
fn encrypt_block(keys: &[[u8; BLOCK]], block: &[u8; BLOCK], table: &[u8; 256]) -> State {
    let rounds = keys.len() - 1;
    let mut state = *block;
    add_key(&mut state, &keys[0]);
    for key in keys.iter().take(rounds).skip(1) {
        substitute(&mut state, table);
        shift_rows(&mut state);
        mix_columns(&mut state);
        add_key(&mut state, key);
    }
    substitute(&mut state, table);
    shift_rows(&mut state);
    add_key(&mut state, &keys[rounds]);
    state
}

/// The **encryption** of bytes under AES-128 in CBC mode.
///
/// The vector is the sixteen bytes the blocks are chained from, and
/// what comes back is the **ciphertext**: the bytes that are written
/// out after the vector.
///
/// The bytes are padded on the way in: the padding says how many bytes
/// there are, and they are all that byte.
pub fn encrypt(key: &[u8], vector: &[u8], bytes: &[u8]) -> Vec<u8> {
    // The **padding**: there is always a byte of it, and a whole block
    // of it when the bytes are already a whole block.
    let count = BLOCK - bytes.len() % BLOCK;
    let mut plain: Vec<u8> = bytes.to_vec();
    plain.extend(std::iter::repeat_n(count as u8, count));
    encrypt_raw(key, vector, &plain)
}

/// The **encryption** of bytes under AES in CBC mode, and **no
/// padding**.
pub fn encrypt_raw(key: &[u8], vector: &[u8], bytes: &[u8]) -> Vec<u8> {
    if !is_a_key(key) || vector.len() != BLOCK {
        return Vec::new();
    }
    let table = box_of();
    let keys = round_keys(key, &table);
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut previous: State = [0; BLOCK];
    previous.copy_from_slice(vector);
    for chunk in bytes.chunks(BLOCK) {
        if chunk.len() < BLOCK {
            break;
        }
        let mut block: State = [0; BLOCK];
        block.copy_from_slice(chunk);
        for index in 0..BLOCK {
            block[index] ^= previous[index];
        }
        let cipher = encrypt_block(&keys, &block, &table);
        out.extend_from_slice(&cipher);
        previous = cipher;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::decrypt;

    #[test]
    fn the_box_is_the_one_of_the_specification() {
        let table = super::box_of();
        let first: Vec<u8> = table.iter().take(16).copied().collect();
        assert_eq!(
            first,
            vec![
                0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab,
                0x76
            ]
        );
    }

    #[test]
    fn the_round_keys_are_the_ones_of_the_specification() {
        let key: Vec<u8> = (0..16).collect();
        let keys = super::round_keys(&key, &super::box_of());
        let first: Vec<u8> = keys[1].to_vec();
        assert_eq!(
            first,
            vec![
                0xd6, 0xaa, 0x74, 0xfd, 0xd2, 0xaf, 0x72, 0xfa, 0xda, 0xa6, 0x78, 0xf1, 0xd6, 0xab, 0x76,
                0xfe
            ]
        );
    }

    #[test]
    fn the_columns_are_mixed_the_other_way() {
        // The state of the specification, in columns.
        let mut state: [u8; 16] =
            [0x11, 0x7d, 0x77, 0x73, 0x23, 0x5e, 0x79, 0x11, 0x32, 0x19, 0x59, 0xd6, 0x15, 0x72, 0x15, 0x62];
        super::unmix_columns(&mut state);
        // The first column of the result.
        assert_eq!(&state[0..4], &[0x4f, 0x54, 0xf9, 0x8a]);
    }

    #[test]
    fn the_rows_are_shifted_the_other_way() {
        // The state of the specification, in columns: the first row is
        // not shifted, the second one is shifted by one byte.
        let mut state: [u8; 16] =
            [0x11, 0x7d, 0x77, 0x73, 0x23, 0x5e, 0x79, 0x11, 0x32, 0x19, 0x59, 0xd6, 0x15, 0x72, 0x15, 0x62];
        super::unshift_rows(&mut state);
        // The second row, shifted by one byte.
        assert_eq!(&state[4..8], &[0x23, 0x7d, 0x15, 0xd6]);
    }

    #[test]
    fn the_block_is_the_one_of_the_specification() {
        let key: Vec<u8> = (0..16).collect();
        let keys = super::round_keys(&key, &super::box_of());
        let cipher: [u8; 16] =
            [0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5, 0x5a];
        let plain = super::decrypt_block(&keys, &cipher, &super::inverted_box());
        assert_eq!(
            plain.to_vec(),
            vec![
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
                0xff
            ]
        );
    }

    #[test]
    fn the_cipher_undoes_what_openssl_did() {
        let key: Vec<u8> = (0..16).collect();
        let vector: Vec<u8> = (16..32).collect();
        let cipher: Vec<u8> = vec![
            0x32, 0xf6, 0xd6, 0xea, 0x1e, 0xc8, 0x87, 0x2d, 0xeb, 0xcc, 0xea, 0x85, 0x96, 0xd9, 0xc3, 0xc3,
        ];
        let plain = decrypt(&key, &vector, &cipher);
        assert_eq!(plain, b"hello".to_vec());
    }

    #[test]
    fn the_block_is_encrypted_the_way_the_specification_says() {
        let key: Vec<u8> = (0..16).collect();
        let keys = super::round_keys(&key, &super::box_of());
        let plain: [u8; 16] =
            [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let cipher = super::encrypt_block(&keys, &plain, &super::box_of());
        assert_eq!(
            cipher.to_vec(),
            vec![
                0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5,
                0x5a
            ]
        );
    }

    #[test]
    fn the_cipher_makes_what_openssl_made() {
        let key: Vec<u8> = (0..16).collect();
        let vector: Vec<u8> = (16..32).collect();
        let cipher: Vec<u8> = vec![
            0x32, 0xf6, 0xd6, 0xea, 0x1e, 0xc8, 0x87, 0x2d, 0xeb, 0xcc, 0xea, 0x85, 0x96, 0xd9, 0xc3, 0xc3,
        ];
        assert_eq!(super::encrypt(&key, &vector, b"hello"), cipher, "the bytes are the ones of openssl");
    }

    #[test]
    fn what_the_cipher_makes_the_cipher_undoes() {
        let key: Vec<u8> = (0..16).collect();
        let vector: Vec<u8> = (16..32).collect();
        let bytes: Vec<u8> = (0..40).collect();
        let cipher = super::encrypt(&key, &vector, &bytes);
        assert_eq!(decrypt(&key, &vector, &cipher), bytes, "the bytes come back as they were");
    }

    #[test]
    fn the_block_of_two_hundred_and_fifty_six_bits_is_the_one_of_the_specification() {
        let key: Vec<u8> = (0..32).collect();
        let keys = super::round_keys(&key, &super::box_of());
        let plain: [u8; 16] =
            [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let cipher = super::encrypt_block(&keys, &plain, &super::box_of());
        assert_eq!(
            cipher.to_vec(),
            vec![
                0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49, 0x60,
                0x89
            ]
        );
        let back = super::decrypt_block(&keys, &cipher, &super::inverted_box());
        assert_eq!(back.to_vec(), plain.to_vec(), "and the cipher undoes it");
    }

    #[test]
    fn the_cipher_of_two_hundred_and_fifty_six_bits_makes_what_it_undoes() {
        let key: Vec<u8> = (0..32).collect();
        let vector: Vec<u8> = (32..48).collect();
        let bytes: Vec<u8> = (0..40).collect();
        let cipher = super::encrypt(&key, &vector, &bytes);
        assert_eq!(super::decrypt(&key, &vector, &cipher), bytes, "the bytes come back as they were");
        // And the bytes without a padding are the bytes.
        let whole: Vec<u8> = (0..32).collect();
        let raw = super::encrypt_raw(&key, &vector, &whole);
        assert_eq!(super::decrypt_raw(&key, &vector, &raw), whole, "and the raw bytes come back too");
    }

    #[test]
    fn a_key_that_is_not_a_key_is_no_key() {
        let vector: Vec<u8> = (0..16).collect();
        assert!(super::encrypt(&[0_u8; 20], &vector, b"hello").is_empty(), "a key of 160 bits is no key");
        assert!(super::decrypt(&[0_u8; 20], &vector, b"hello").is_empty(), "a key of 160 bits is no key");
        assert!(super::decrypt_ecb(&[0_u8; 20], &[0_u8; 16]).is_empty(), "a key of 160 bits is no key");
    }
}
