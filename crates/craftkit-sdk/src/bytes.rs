//! **Binary** in a JSON payload.
//!
//! # The problem
//!
//! A node's payload is JSON, and the core does not read it. That covers
//! everything a document model usually wants to say — a number, a
//! string, a list, an object — and it does not cover **bytes**.
//!
//! And some documents carry bytes: a PDF stream is arbitrary binary and
//! must round-trip byte for byte; an embedded image is bytes; a font
//! blob is bytes. The port of a PDF application found this, and it is
//! not a PDF problem: any document that keeps a picture, a sound or a
//! font inside itself has it.
//!
//! # The answer
//!
//! The payload is JSON and stays JSON, and **bytes travel in it as
//! base64**. This is the honest small answer, and it is small on
//! purpose: a base64 string is a string, and a string is what a JSON
//! payload can carry.
//!
//! The cost is written down rather than hidden:
//!
//! - base64 is **a third bigger** than the bytes it carries;
//! - a big binary in a payload is a big JSON string, and a document
//!   that carries fifty megabytes of streams carries sixty-seven
//!   megabytes of text.
//!
//! The other answer is a **second payload field**: a node with a
//! dictionary *and* a stream, which is exactly the shape a PDF object
//! has. It would cost a field on the node, a version of the format, and
//! base64 in the file anyway — the file is JSON either way — and it is
//! written down in the design document as the next thing to build if a
//! document ever needs it.
//!
//! # The encoding
//!
//! **Base64**, as it is in every other file format: the standard
//! alphabet, with padding, and no line breaks. It is written out here
//! because a plugin should not have to depend on a crate to put a
//! picture in a payload, and because a base64 that is written out can be
//! **read** and not only trusted.

/// The standard base64 alphabet.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode some bytes as base64.
///
/// The standard alphabet, with padding, and no line breaks.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let bits = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        out.push(ALPHABET[(bits >> 18) as usize & 63] as char);
        out.push(ALPHABET[(bits >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(bits >> 6) as usize & 63] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[bits as usize & 63] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Decode some base64 into bytes.
///
/// The standard alphabet, with padding. Anything that is not base64 is
/// **refused**, with the reason: a payload that carries bytes that were
/// damaged is worse than a payload that says so.
pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(format!("base64 is in fours, and this is {} long", bytes.len()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let mut bits: u32 = 0;
        let mut padding = 0;
        for byte in chunk.iter() {
            if *byte == b'=' {
                // The padding is a character too: it shifts the bits like
                // any other and carries nothing.
                bits <<= 6;
                padding += 1;
                continue;
            }
            if padding > 0 {
                return Err("the padding is in the middle".to_string());
            }
            let value = match ALPHABET.iter().position(|known| known == byte) {
                Some(value) => value as u32,
                None => return Err(format!("`{}` is not in the base64 alphabet", *byte as char)),
            };
            bits = (bits << 6) | value;
        }
        out.push((bits >> 16) as u8);
        if padding < 2 {
            out.push((bits >> 8) as u8);
        }
        if padding < 1 {
            out.push(bits as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The examples every base64 has to agree with.
    #[test]
    fn the_examples_are_the_examples() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(b"f"), "Zg==");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(encode(b"foo"), "Zm9v");
        assert_eq!(encode(b"foob"), "Zm9vYg==");
        assert_eq!(encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
    }

    /// What is encoded comes back.
    #[test]
    fn what_goes_in_comes_out() {
        let bytes: Vec<u8> = (0..=255u8).collect();
        let text = encode(&bytes);
        assert_eq!(decode(&text).expect("it decodes"), bytes, "the bytes came back");
    }

    /// Every length round trips.
    #[test]
    fn every_length_round_trips() {
        for length in 0..40 {
            let bytes: Vec<u8> = (0..length).map(|index| index as u8).collect();
            let text = encode(&bytes);
            assert_eq!(decode(&text).expect("it decodes"), bytes, "length {length}");
        }
    }

    /// Something that is not base64 is refused.
    #[test]
    fn something_that_is_not_base64_is_refused() {
        let error = decode("not base64!!").expect_err("it is refused");
        assert!(error.contains("alphabet"), "{error}");
    }

    /// A length that is not in fours is refused.
    #[test]
    fn a_length_that_is_not_in_fours_is_refused() {
        let error = decode("Zm9").expect_err("it is refused");
        assert!(error.contains("in fours"), "{error}");
    }
}
