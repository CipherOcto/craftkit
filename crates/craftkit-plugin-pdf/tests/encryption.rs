//! The **encryption** of a PDF file.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the pdf battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-encryption-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A small **file**: a catalog, a page, an encryption and a trailer.
fn a_file(encrypt: Option<&str>) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    let mut bodies: Vec<(usize, String)> = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>".to_string()),
    ];
    if let Some(encrypt) = encrypt {
        bodies.push((4, encrypt.to_string()));
    }
    for (number, body) in &bodies {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(format!("xref\n0 {}\n", bodies.len() + 1).as_bytes());
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    let trailer = match encrypt {
        Some(_) => format!("<< /Size {} /Root 1 0 R /Encrypt 4 0 R >>", bodies.len() + 1),
        None => format!("<< /Size {} /Root 1 0 R >>", bodies.len() + 1),
    };
    file.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n").as_bytes());
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load a file into the document.
fn load(session: &mut Session, name: &str, encrypt: Option<&str>) {
    let directory = directory(name);
    let path = format!("{directory}/small.pdf");
    std::fs::write(&path, a_file(encrypt)).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **encryption** of the document, as the command reads it.
fn encryption_of(session: &mut Session) -> Json {
    let outcome = session.execute("pdf.encrypt.read", &json!({})).expect("the encryption is read");
    outcome.data.clone()
}

/// The **names** of the permissions.
fn permission_names(encryption: &Json) -> Vec<String> {
    encryption["permission_names"]
        .as_array()
        .expect("the names are a list")
        .iter()
        .filter_map(|name| name.as_str().map(|name| name.to_string()))
        .collect()
}

/// A file that names **no** encryption is not encrypted.
#[test]
fn a_file_that_names_no_encryption_is_not_encrypted() {
    let mut session = session();
    load(&mut session, "none", None);
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["encrypted"], json!(false), "the file is not encrypted");
}

/// The encryption says the **filter**, the **version** and the
/// **revision**.
#[test]
fn the_encryption_says_the_filter_the_version_and_the_revision() {
    let mut session = session();
    load(&mut session, "version", Some("<< /Filter /Standard /V 4 /R 4 /Length 128 /P -4 >>"));
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["encrypted"], json!(true), "the file is encrypted");
    assert_eq!(encryption["filter"], json!("Standard"), "the filter is the standard one");
    assert_eq!(encryption["version"], json!(4), "the version is four");
    assert_eq!(encryption["revision"], json!(4), "the revision is four");
    assert_eq!(encryption["length_bits"], json!(128), "the key is a hundred and twenty-eight bits");
}

/// The key length is **forty** when the file does not say.
#[test]
fn the_key_length_is_forty_when_the_file_does_not_say() {
    let mut session = session();
    load(&mut session, "length", Some("<< /Filter /Standard /V 2 /R 3 /P -4 >>"));
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["length_bits"], json!(40), "the key is forty bits");
}

/// The **permissions** are said in words.
#[test]
fn the_permissions_are_said_in_words() {
    let mut session = session();
    load(&mut session, "permissions", Some("<< /Filter /Standard /V 4 /R 4 /P -4 >>"));
    let encryption = encryption_of(&mut session);
    let names = permission_names(&encryption);
    for wanted in [
        "print",
        "modify",
        "copy",
        "annotate",
        "fill-forms",
        "extract-for-accessibility",
        "assemble",
        "print-high-quality",
    ] {
        assert!(names.contains(&wanted.to_string()), "the file may {wanted}: {names:?}");
    }
}

/// A file that may be **read** and not **written** says so.
#[test]
fn a_file_that_may_be_read_and_not_written_says_so() {
    let mut session = session();
    load(&mut session, "restricted", Some("<< /Filter /Standard /V 4 /R 4 /P 512 >>"));
    let encryption = encryption_of(&mut session);
    let names = permission_names(&encryption);
    assert!(!names.contains(&"print".to_string()), "the file may not print: {names:?}");
    assert!(!names.contains(&"modify".to_string()), "nor be modified: {names:?}");
    assert!(names.contains(&"extract-for-accessibility".to_string()), "but it may be read: {names:?}");
}

/// The **typical** value of the permissions allows nothing.
#[test]
fn the_typical_value_of_the_permissions_allows_nothing() {
    let mut session = session();
    load(&mut session, "typical", Some("<< /Filter /Standard /V 4 /R 4 /P -3904 >>"));
    let encryption = encryption_of(&mut session);
    let names = permission_names(&encryption);
    assert!(names.is_empty(), "the file allows nothing: {names:?}");
}

/// A file whose version is below four is scrambled with **RC4**.
#[test]
fn a_file_whose_version_is_below_four_is_scrambled_with_rc4() {
    let mut session = session();
    load(&mut session, "rc4", Some("<< /Filter /Standard /V 2 /R 3 /P -4 >>"));
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["strings"], json!("rc4"), "the strings are scrambled with RC4");
    assert_eq!(encryption["streams"], json!("rc4"), "and so are the streams");
}

/// The **crypt filters** say how the file is scrambled.
#[test]
fn the_crypt_filters_say_how_the_file_is_scrambled() {
    let mut session = session();
    load(
        &mut session,
        "aes",
        Some(
            "<< /Filter /Standard /V 4 /R 4 /Length 128 /P -4 /CF << /StdCF << /CFM /AESV2 /Length 16 >> >> /StmF /StdCF /StrF /StdCF >>",
        ),
    );
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["strings"], json!("aes-128"), "the strings are scrambled with AES");
    assert_eq!(encryption["streams"], json!("aes-128"), "and so are the streams");
}

/// The **metadata** is encrypted unless the file says otherwise.
#[test]
fn the_metadata_is_encrypted_unless_the_file_says_otherwise() {
    let mut other = session();
    let mut session = session();
    load(&mut session, "metadata", Some("<< /Filter /Standard /V 4 /R 4 /P -4 >>"));
    let encryption = encryption_of(&mut session);
    assert_eq!(encryption["encrypt_metadata"], json!(true), "the metadata is encrypted");
    load(
        &mut other,
        "plain-metadata",
        Some("<< /Filter /Standard /V 4 /R 4 /P -4 /EncryptMetadata false >>"),
    );
    let encryption = encryption_of(&mut other);
    assert_eq!(encryption["encrypt_metadata"], json!(false), "the metadata is not encrypted");
}

/// One **entry** of the encryption may be read.
#[test]
fn one_entry_of_the_encryption_may_be_read() {
    let mut session = session();
    load(&mut session, "entry", Some("<< /Filter /Standard /V 4 /R 4 /P -4 >>"));
    let outcome = session.execute("pdf.encrypt.read", &json!({ "key": "P" })).expect("the entry is read");
    let entries = outcome.data["entries"].as_array().expect("the entries are a list");
    assert_eq!(entries.len(), 1, "there is one entry");
    assert_eq!(entries[0][0], json!("P"), "and it is the one");
}

/// An entry that is **not there** is refused.
#[test]
fn an_entry_that_is_not_there_is_refused() {
    let mut session = session();
    load(&mut session, "missing", Some("<< /Filter /Standard /V 4 /R 4 /P -4 >>"));
    let refused =
        session.execute("pdf.encrypt.read", &json!({ "key": "O" })).expect_err("the entry is refused");
    assert!(refused.to_string().contains("no entry `O`"), "the refusal says why: {refused}");
}
