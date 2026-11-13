//! The **signatures** of a file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-signatures-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A file with a **signed** field and one that is not signed yet.
fn a_file() -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n");
    let mut offsets: Vec<usize> = Vec::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Annots [4 0 R 5 0 R] >>"),
        (
            4,
            "<< /Type /Annot /Subtype /Widget /FT /Sig /T (the signature) /Rect [0 0 10 10] /P 3 0 R /V 6 0 R >>",
        ),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Sig /T (the other) /Rect [0 0 10 10] /P 3 0 R >>"),
        (
            6,
            "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 100 200 300] /Contents (00ff00) /Name (ann) /M (D:20240105093000Z) >>",
        ),
    ] {
        offsets.push(file.len());
        file.extend_from_slice(format!("{number} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let table_at = file.len();
    file.extend_from_slice(b"xref\n0 7\n");
    file.extend_from_slice(b"0000000000 65535 f\r\n");
    for offset in &offsets {
        file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
    }
    file.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    file
}

/// Load the file into the document.
fn load(session: &mut Session, name: &str) {
    let directory = directory(name);
    let path = format!("{directory}/file.pdf");
    std::fs::write(&path, a_file()).expect("the file is written");
    let into = format!("{directory}/streams");
    session.execute("pdf.file.load", &json!({ "path": path, "into": into })).expect("the file is loaded");
}

/// The **signatures** of a file, as the command reads them.
fn signatures_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("pdf.signatures.read", &json!({})).expect("the signatures are read");
    outcome.data["signatures"].as_array().expect("the signatures are a list").clone()
}

/// The signature of a file is **read**.
#[test]
fn the_signature_of_a_file_is_read() {
    let mut session = session();
    load(&mut session, "signed");
    let signatures = signatures_of(&mut session);
    assert_eq!(signatures.len(), 2, "there are two signature fields");
    let signed = signatures
        .iter()
        .find(|signature| signature["field"] == json!("the signature"))
        .expect("it is there");
    assert_eq!(signed["name"], json!("ann"), "the signer is named");
    assert_eq!(signed["time"], json!("D:20240105093000Z"), "and the time is given");
    assert_eq!(signed["sub_filter"], json!("adbe.pkcs7.detached"), "and the way it was made");
    assert_eq!(signed["byte_range"], json!([0, 100, 200, 300]), "and the range it covers");
    assert_eq!(signed["signed"], json!(true), "and it holds bytes");
}

/// A signature field with **no value** is not signed yet.
#[test]
fn a_signature_field_with_no_value_is_not_signed() {
    let mut session = session();
    load(&mut session, "unsigned");
    let signatures = signatures_of(&mut session);
    let other =
        signatures.iter().find(|signature| signature["field"] == json!("the other")).expect("it is there");
    assert_eq!(other["signed"], json!(false), "the field is not signed");
    assert_eq!(other["name"], Json::Null, "and there is no signer");
}

/// One signature may be read by the **name of its field**.
#[test]
fn one_signature_may_be_read_by_the_name_of_its_field() {
    let mut session = session();
    load(&mut session, "one");
    let outcome = session
        .execute("pdf.signatures.read", &json!({ "field": "the signature" }))
        .expect("the signature is read");
    assert_eq!(outcome.data["count"], json!(1), "one signature is read");
    assert_eq!(outcome.data["signatures"][0]["name"], json!("ann"), "and it is the one that was named");
}

/// The bytes of a signature are **not checked**.
#[test]
fn the_bytes_of_a_signature_are_not_checked() {
    let mut session = session();
    load(&mut session, "unchecked");
    let outcome = session.execute("pdf.signatures.read", &json!({})).expect("the signatures are read");
    assert_eq!(outcome.data["verified"], json!(false), "nothing was checked");
}

/// A name the file does not use is **refused**.
#[test]
fn a_name_the_file_does_not_use_is_refused() {
    let mut session = session();
    load(&mut session, "refused");
    let outcome = session.execute("pdf.signatures.read", &json!({ "field": "nothing" }));
    assert!(outcome.is_err(), "there is no such signature");
}
