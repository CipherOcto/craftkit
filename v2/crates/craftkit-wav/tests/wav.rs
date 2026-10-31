//! The tests of the WAV reader.
//!
//! A file came from somewhere else, so the reader's job is to make
//! sense of it **within limits** and to refuse what it cannot. So the
//! tests are: the samples of every width the kit reads, and a refusal
//! for every way a file can lie.

use craftkit_wav::{Error, Limits, Sound, read};

/// A WAV file, made here: the header the reader is to make sense of.
fn a_wav(kind: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    // The format chunk: sixteen bytes, the plain kind.
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&kind.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(channels)).to_le_bytes());
    bytes.extend_from_slice(&(channels * bits / 8).to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    // The samples.
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    bytes
}

/// The limits every test uses: the defaults.
fn limits() -> Limits {
    Limits::default()
}

/// The sound of a file, or the refusal.
fn sound(bytes: &[u8]) -> Result<Sound, Error> {
    read(bytes, &limits())
}

/// A **sixteen bit** file is read: the samples, the rate, the channels
/// and the frames.
#[test]
fn a_sixteen_bit_file_is_read() {
    let data: Vec<u8> = [0_i16, 16384, -16384].iter().flat_map(|value| value.to_le_bytes()).collect();
    let sound = sound(&a_wav(1, 1, 48_000, 16, &data)).expect("the file is read");
    assert_eq!(sound.channels, 1);
    assert_eq!(sound.rate, 48_000);
    assert_eq!(sound.frames, 3);
    assert_eq!(sound.samples.len(), 3);
    assert!((sound.samples[0] - 0.0).abs() < 1e-6, "the first sample is silence");
    assert!((sound.samples[1] - 0.5).abs() < 1e-6, "the second is half");
    assert!((sound.samples[2] + 0.5).abs() < 1e-6, "the third is minus half");
}

/// An **eight bit** file is unsigned: the middle of the range is
/// silence.
#[test]
fn an_eight_bit_file_is_read() {
    let sound = sound(&a_wav(1, 1, 8_000, 8, &[128, 0, 255])).expect("the file is read");
    assert_eq!(sound.frames, 3);
    assert!((sound.samples[0] - 0.0).abs() < 1e-6, "the middle is silence");
    assert!((sound.samples[1] + 1.0).abs() < 1e-6, "the lowest is minus one");
    assert!((sound.samples[2] - 0.9921875).abs() < 1e-6, "the highest is nearly one");
}

/// A **twenty four bit** file is read.
#[test]
fn a_twenty_four_bit_file_is_read() {
    let sample: i32 = 8_388_608 / 2;
    let raw = (sample as u32).to_le_bytes();
    let data: Vec<u8> = raw[..3].to_vec();
    let sound = sound(&a_wav(1, 1, 96_000, 24, &data)).expect("the file is read");
    assert_eq!(sound.frames, 1);
    assert!((sound.samples[0] - 0.5).abs() < 1e-6, "the sample is half");
}

/// A **float** file is read as it is: there is no whole number to
/// divide.
#[test]
fn a_float_file_is_read() {
    let data: Vec<u8> = 0.25_f32.to_le_bytes().to_vec();
    let sound = sound(&a_wav(3, 1, 44_100, 32, &data)).expect("the file is read");
    assert!((sound.samples[0] - 0.25).abs() < 1e-6, "the sample is a quarter");
}

/// A **stereo** file's samples are **interleaved**.
#[test]
fn a_stereo_file_is_interleaved() {
    let left: Vec<u8> = 1.0_f32.to_le_bytes().to_vec();
    let right: Vec<u8> = (-1.0_f32).to_le_bytes().to_vec();
    let mut data = left.clone();
    data.extend_from_slice(&right);
    let sound = sound(&a_wav(3, 2, 44_100, 32, &data)).expect("the file is read");
    assert_eq!(sound.channels, 2);
    assert_eq!(sound.frames, 1);
    assert!((sound.samples[0] - 1.0).abs() < 1e-6, "the left channel is first");
    assert!((sound.samples[1] + 1.0).abs() < 1e-6, "and the right channel is second");
}

/// An **extensible** file says what it is in its own sub-format.
#[test]
fn an_extensible_file_is_read() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&40_u32.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&44_100_u32.to_le_bytes());
    bytes.extend_from_slice(&176_400_u32.to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    // The extension: the size of the extension, the valid bits, the
    // channel mask and the sub-format's name.
    bytes.extend_from_slice(&22_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
    ]);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&1_000_i16.to_le_bytes());
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    let sound = sound(&bytes).expect("the file is read");
    assert_eq!(sound.channels, 1);
    assert_eq!(sound.rate, 44_100);
    assert!((sound.samples[0] - 1000.0 / 32768.0).abs() < 1e-6, "the sample is read");
}

/// A file that is **not** a RIFF file is refused.
#[test]
fn a_file_that_is_not_a_riff_is_refused() {
    let error = sound(b"OggS\x00\x00\x00\x00WAVE").expect_err("that is not a sound");
    assert_eq!(error, Error::NotRiff);
}

/// A RIFF file that is not a **sound** is refused.
#[test]
fn a_riff_file_that_is_not_a_sound_is_refused() {
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(b"AVI ");
    let error = sound(&bytes).expect_err("that is not a sound");
    assert_eq!(error, Error::NotWave);
}

/// A file with **no samples** is refused.
#[test]
fn a_file_with_no_samples_is_refused() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&96_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    let error = sound(&bytes).expect_err("there are no samples");
    assert_eq!(error, Error::NoData);
}

/// A format the kit does not read is refused, **with its name**.
#[test]
fn a_format_that_is_not_read_is_refused() {
    let error = sound(&a_wav(7, 1, 8_000, 8, &[0])).expect_err("that is not a format the kit reads");
    assert_eq!(error, Error::UnknownFormat { format: 7 });
}

/// A chunk that says it is **longer than the file** is refused.
#[test]
fn a_chunk_longer_than_the_file_is_refused() {
    let mut bytes = a_wav(1, 1, 8_000, 16, &[0, 0]);
    // The data chunk's size is made a lie.
    let at = bytes.len() - 4;
    bytes[at..at + 4].copy_from_slice(&1_000_000_u32.to_le_bytes());
    let error = sound(&bytes).expect_err("the chunk is longer than the file");
    assert!(matches!(error, Error::ChunkTooLong { .. }), "{error:?}");
}

/// A rate of **nothing** is refused.
#[test]
fn a_rate_of_nothing_is_refused() {
    let error = sound(&a_wav(1, 1, 0, 16, &[0, 0])).expect_err("a rate of nothing is not a rate");
    assert_eq!(error, Error::BadRate { rate: 0 });
}

/// More channels than the limits allow are refused.
#[test]
fn too_many_channels_are_refused() {
    let error = sound(&a_wav(1, 64, 48_000, 16, &[0, 0])).expect_err("that is too many channels");
    assert_eq!(error, Error::BadChannels { channels: 64 });
}

/// More samples than the limits allow are refused: the bound is the
/// point.
#[test]
fn more_samples_than_the_limits_allow_are_refused() {
    let limits = Limits { max_bytes: 4, ..Limits::default() };
    let bytes = a_wav(1, 1, 48_000, 16, &[0; 64]);
    let error = read(&bytes, &limits).expect_err("that is more than the limit");
    assert_eq!(error, Error::TooManySamples);
}

/// Samples that are not a whole number of frames are refused: the file
/// says one thing and holds another.
#[test]
fn samples_that_are_not_whole_frames_are_refused() {
    let error = sound(&a_wav(1, 1, 48_000, 16, &[0, 0, 0])).expect_err("that is half a frame");
    assert_eq!(error, Error::ShortData);
}
