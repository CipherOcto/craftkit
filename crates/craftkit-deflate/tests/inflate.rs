//! The **inflate**: the zlib stream, and what comes out of it.

use craftkit_deflate::{deflate, zlib};

/// A **raw** DEFLATE stream of that text, compressed by somebody else.
fn a_deflate_stream(text: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text).expect("the text is written");
    encoder.finish().expect("the stream is finished")
}

/// A zlib stream of that text, compressed by somebody else.
fn a_zlib_stream(text: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(text).expect("the text is written");
    encoder.finish().expect("the stream is finished")
}

/// A stream that was compressed comes back as it was.
#[test]
fn a_compressed_stream_comes_back() {
    let text = b"the quick brown fox jumps over the lazy dog, and then it does it again";
    let stream = a_zlib_stream(text);
    let out = zlib(&stream, &Default::default()).expect("the stream is inflated");
    assert_eq!(out, text);
}

/// A stream of nothing is nothing.
#[test]
fn a_stream_of_nothing_is_nothing() {
    let stream = a_zlib_stream(b"");
    let out = zlib(&stream, &Default::default()).expect("the stream is inflated");
    assert!(out.is_empty(), "the stream is not empty");
}

/// A long stream comes back whole: the copies are the hard part.
#[test]
fn a_long_stream_comes_back_whole() {
    let mut text = Vec::new();
    for index in 0..5000u32 {
        text.extend_from_slice(format!("line {index}\n").as_bytes());
    }
    let stream = a_zlib_stream(&text);
    let out = zlib(&stream, &Default::default()).expect("the stream is inflated");
    assert_eq!(out.len(), text.len());
    assert_eq!(out, text);
}

/// A stream that is not a zlib stream is refused.
#[test]
fn a_stream_that_is_not_zlib_is_refused() {
    let error = zlib(b"not a stream", &Default::default()).expect_err("that is not a stream");
    assert!(error.to_string().contains("stream"), "{error}");
}

/// A **raw** DEFLATE stream comes back as it was: that is the thing a
/// package reader wants, because a package entry is raw.
#[test]
fn a_raw_deflate_stream_comes_back() {
    let text = b"the quick brown fox jumps over the lazy dog, and then it does it again";
    let stream = a_deflate_stream(text);
    let out = deflate(&stream, &Default::default()).expect("the stream is inflated");
    assert_eq!(out, text);
}

/// A **raw** stream of nothing is nothing.
#[test]
fn a_raw_stream_of_nothing_is_nothing() {
    let stream = a_deflate_stream(b"");
    let out = deflate(&stream, &Default::default()).expect("the stream is inflated");
    assert!(out.is_empty(), "the stream is not empty");
}

/// A raw stream that is **not** DEFLATE is refused.
#[test]
fn a_raw_stream_that_is_not_deflate_is_refused() {
    let error = deflate(b"not a stream", &Default::default()).expect_err("that is not a stream");
    assert!(error.to_string().contains("stream"), "{error}");
}

/// A stream that says it is **too big** is refused, and the limit is
/// the caller's.
#[test]
fn a_stream_that_is_too_big_is_refused() {
    let text = vec![b'x'; 100_000];
    let stream = a_deflate_stream(&text);
    let limits = craftkit_deflate::Limits { max_output: 1000 };
    let error = deflate(&stream, &limits).expect_err("the stream is too big");
    assert!(error.to_string().contains("limit"), "{error}");
}
