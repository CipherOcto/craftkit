//! A **compound file** is read: its directory, and the streams of it.

use craftkit_xlsx::cfb::Compound;

/// The **signature** of a compound file.
const SIGNATURE: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The end of a **chain**.
const END: u32 = 0xFFFF_FFFF;

/// The size of a **sector**.
const SECTOR: usize = 512;

/// The size of a **mini sector**.
const MINI: usize = 64;

/// One **stream** of the file that is written.
struct Stream {
    name: String,
    bytes: Vec<u8>,
}

/// A compound file with those streams in it.
///
/// The file is written the way the specification says a compound file
/// is written: a **header**, the table of allocation of the sectors,
/// the **directory**, the table of allocation of the mini sectors, the
/// **mini stream** and the streams that are too big for it.
fn a_compound_file(streams: &[Stream]) -> Vec<u8> {
    // The **mini stream**: the small streams, one after another, each
    // padded to a mini sector.
    let mut mini_stream: Vec<u8> = Vec::new();
    let mut mini_starts: Vec<u32> = Vec::new();
    for stream in streams {
        if stream.bytes.len() >= 4096 {
            continue;
        }
        let start = (mini_stream.len() / MINI) as u32;
        mini_starts.push(start);
        mini_stream.extend_from_slice(&stream.bytes);
        while !mini_stream.len().is_multiple_of(MINI) {
            mini_stream.push(0);
        }
    }
    // The **table of allocation of the mini sectors**.
    let mini_count = mini_stream.len() / MINI;
    let mut mini_fat: Vec<u32> = Vec::new();
    for index in 0..mini_count {
        mini_fat.push(if index + 1 == mini_count { END } else { index as u32 + 1 });
    }
    let mut mini_fat_bytes: Vec<u8> = Vec::new();
    for value in &mini_fat {
        mini_fat_bytes.extend_from_slice(&value.to_le_bytes());
    }
    while mini_fat_bytes.len() < SECTOR {
        mini_fat_bytes.extend_from_slice(&END.to_le_bytes());
    }

    // The **sectors** of the file, in their order: the directory, the
    // table of the mini sectors, the mini stream, and then the big
    // streams. The sector of the file is the place in this list plus
    // one, because the table of allocation is the first sector.
    let mut sectors: Vec<Vec<u8>> = Vec::new();
    let mut fat: Vec<u32> = vec![END, END, END];
    let mut big_starts: Vec<u32> = Vec::new();
    let mini_sectors = mini_stream.len().div_ceil(SECTOR).max(1);
    for index in 0..mini_sectors {
        let at = index * SECTOR;
        let mut part = mini_stream.get(at..).unwrap_or_default().to_vec();
        part.resize(SECTOR, 0);
        sectors.push(part);
        fat.push(if index + 1 == mini_sectors { END } else { 4 + index as u32 });
    }
    let mut next_sector = 3 + mini_sectors as u32;
    for stream in streams {
        if stream.bytes.len() < 4096 {
            big_starts.push(0);
            continue;
        }
        let start = next_sector;
        big_starts.push(start);
        let count = stream.bytes.len().div_ceil(SECTOR);
        for index in 0..count {
            let at = index * SECTOR;
            let mut part = stream.bytes.get(at..).unwrap_or_default().to_vec();
            part.resize(SECTOR, 0);
            sectors.push(part);
            fat.push(if index + 1 == count { END } else { start + index as u32 + 1 });
        }
        next_sector += count as u32;
    }
    let mini_start = 3u32;

    // The **directory**: the root, and then a stream for each.
    let mut directory: Vec<u8> = Vec::new();
    let mut entry = |name: &str, kind: u8, start: u32, size: u64| {
        let mut chunk = vec![0u8; 128];
        let mut units = name.encode_utf16().collect::<Vec<u16>>();
        units.push(0);
        for (index, unit) in units.iter().take(31).enumerate() {
            chunk[index * 2] = (unit & 0xFF) as u8;
            chunk[index * 2 + 1] = (unit >> 8) as u8;
        }
        chunk[64..66].copy_from_slice(&((units.len() * 2) as u16).to_le_bytes());
        chunk[66] = kind;
        chunk[67] = 1;
        chunk[68..72].copy_from_slice(&END.to_le_bytes());
        chunk[72..76].copy_from_slice(&END.to_le_bytes());
        chunk[76..80].copy_from_slice(&END.to_le_bytes());
        chunk[116..120].copy_from_slice(&start.to_le_bytes());
        chunk[120..128].copy_from_slice(&size.to_le_bytes());
        directory.extend_from_slice(&chunk);
    };
    entry("Root Entry", 5, mini_start, mini_stream.len() as u64);
    for (index, stream) in streams.iter().enumerate() {
        let small = stream.bytes.len() < 4096;
        let start = if small { mini_starts[index] } else { big_starts[index] };
        entry(&stream.name, 2, start, stream.bytes.len() as u64);
    }
    while !directory.len().is_multiple_of(SECTOR) {
        directory.push(0);
    }

    // The **file**: the header, the table of allocation, and then the
    // sectors.
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&SIGNATURE);
    bytes.extend_from_slice(&[0u8; 16]);
    bytes.extend_from_slice(&0x003Eu16.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&0xFFFEu16.to_le_bytes());
    bytes.extend_from_slice(&9u16.to_le_bytes());
    bytes.extend_from_slice(&6u16.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 6]);
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    bytes.extend_from_slice(&4096u32.to_le_bytes());
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&END.to_le_bytes());
    bytes.extend_from_slice(&0u32.to_le_bytes());
    for index in 0..109 {
        let value: u32 = if index == 0 { 0 } else { 0xFFFF_FFFF };
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    while bytes.len() < 512 {
        bytes.push(0);
    }
    let mut fat_bytes: Vec<u8> = Vec::new();
    for value in &fat {
        fat_bytes.extend_from_slice(&value.to_le_bytes());
    }
    while !fat_bytes.len().is_multiple_of(SECTOR) {
        fat_bytes.extend_from_slice(&END.to_le_bytes());
    }
    bytes.extend_from_slice(&fat_bytes);
    bytes.extend_from_slice(&directory);
    bytes.extend_from_slice(&mini_fat_bytes[..SECTOR]);
    for part in &sectors {
        bytes.extend_from_slice(part);
    }
    bytes
}

/// The bytes of a **stream** of the file.
fn read(file: &[u8], name: &str) -> Option<Vec<u8>> {
    let compound = Compound::open(file).expect("the compound file is opened");
    compound.read(name).expect("the stream is read")
}

/// A **big** stream is read: it lives in the sectors of the file.
#[test]
fn a_big_stream_is_read() {
    let bytes = vec![7u8; 5000];
    let file = a_compound_file(&[Stream { name: "EncryptedPackage".to_string(), bytes: bytes.clone() }]);
    assert_eq!(read(&file, "EncryptedPackage"), Some(bytes), "the stream is as it was written");
}

/// A **small** stream is read: it lives in the mini stream.
#[test]
fn a_small_stream_is_read() {
    let bytes = b"the encryption info".to_vec();
    let file = a_compound_file(&[Stream { name: "EncryptionInfo".to_string(), bytes: bytes.clone() }]);
    assert_eq!(read(&file, "EncryptionInfo"), Some(bytes), "the stream is as it was written");
}

/// **Both** kinds of stream are read, and the names of the file are
/// said.
#[test]
fn both_kinds_of_stream_are_read() {
    let small = b"the encryption info".to_vec();
    let big = vec![3u8; 9000];
    let file = a_compound_file(&[
        Stream { name: "EncryptionInfo".to_string(), bytes: small.clone() },
        Stream { name: "EncryptedPackage".to_string(), bytes: big.clone() },
    ]);
    let compound = Compound::open(&file).expect("the compound file is opened");
    let mut names = compound.names();
    names.sort();
    assert_eq!(names, vec!["EncryptedPackage".to_string(), "EncryptionInfo".to_string()], "the names");
    assert_eq!(compound.read("EncryptionInfo").expect("the stream is read"), Some(small));
    assert_eq!(compound.read("EncryptedPackage").expect("the stream is read"), Some(big));
}

/// A stream that is **not there** is nothing, and not an error.
#[test]
fn a_stream_that_is_not_there_is_nothing() {
    let file = a_compound_file(&[Stream { name: "EncryptionInfo".to_string(), bytes: b"x".to_vec() }]);
    let compound = Compound::open(&file).expect("the compound file is opened");
    assert_eq!(compound.read("Nothing").expect("the stream is read"), None, "no stream");
}

/// A file that is **not** a compound file is refused.
#[test]
fn a_file_that_is_not_a_compound_file_is_refused() {
    let error = Compound::open(b"not a compound file").expect_err("that is not a compound file");
    assert!(error.to_string().contains("compound file"), "{error}");
}
