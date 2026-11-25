//! The **head** of a picture is read up to a bound, and the reader says
//! when it **stopped**.
//!
//! The size of a picture is in its header, and the header is at the front
//! of the file: only the head is read, and the head is bounded. A file
//! that is longer than the bound is **cut**, and the answer says so —
//! `head_truncated` — and a refusal names the bound.

use craftkit_plugin_image::{MAX_IMAGE_HEAD_BYTES, MEASURE_PICTURE, PICTURE_KIND};
use craftkit_sdk::Registry;
use craftkit_sdk::Session;
use craftkit_sdk::command::CommandSpec;
use craftkit_sdk::document::{Colour, NodeKindSpec};
use craftkit_sdk::serde_json::json;

/// A session with this battery's commands in it.
fn session() -> Session {
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands.register(MEASURE_PICTURE.clone()).expect("the command is registered");
    let mut kinds: Registry<NodeKindSpec> = Registry::new();
    kinds.register(PICTURE_KIND).expect("the kind is registered");
    Session::with_formats(
        commands,
        kinds,
        Registry::<craftkit_sdk::format::FormatSpec>::new(),
        Registry::<craftkit_sdk::format::MigrationSpec>::new(),
        Registry::<craftkit_sdk::evaluator::EvaluatorSpec>::new(),
    )
}

/// A **PNG file** of a picture, written with the tree's own writer so the
/// tree carries no artwork.
fn a_png_file(name: &str) -> String {
    let mut image = craftkit_raster::Image::new(2, 2, Colour::WHITE);
    image.blend(0, 0, Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-image-head-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, image.to_png()).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// A **PNG file** with a tail of bytes after it, long enough that the head
/// of it is cut at the bound.
///
/// The size is in the header, and the header is at the front: the picture
/// is still measured, and the answer says that the head was cut.
fn a_png_file_with_a_long_tail(name: &str) -> String {
    let mut image = craftkit_raster::Image::new(2, 2, Colour::WHITE);
    image.blend(0, 0, Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let mut bytes = image.to_png();
    bytes.resize(MAX_IMAGE_HEAD_BYTES as usize + 1, 0);
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-image-head-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, &bytes).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// A file of bytes that are **not** a picture, and that is longer than
/// the bound.
fn a_file_that_is_not_a_picture(name: &str) -> String {
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-image-head-{name}.bin")).expect("the scratch is made");
    std::fs::write(&path, vec![0u8; MAX_IMAGE_HEAD_BYTES as usize + 1]).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **size** of a picture, as it is measured.
fn measure(session: &mut Session, path: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("image.measure", &json!({ "path": path })).expect("the picture is measured");
    outcome.data.clone()
}

/// A picture that is **inside** the bound is measured, and its head is
/// not cut: there is nothing beyond the bound to have missed.
#[test]
fn a_picture_inside_the_bound_is_not_cut() {
    let path = a_png_file("whole");
    let mut session = session();
    let data = measure(&mut session, &path);
    assert_eq!(data.get("width").and_then(|width| width.as_u64()), Some(2));
    assert_eq!(
        data.get("head_truncated").and_then(|cut| cut.as_bool()),
        Some(false),
        "a picture inside the bound is not cut: {data:?}"
    );
}

/// A picture that is **beyond** the bound is measured, and the answer
/// says that its head was cut: the file was longer than the head that
/// was read.
#[test]
fn a_picture_beyond_the_bound_says_its_head_was_cut() {
    let path = a_png_file_with_a_long_tail("cut");
    let mut session = session();
    let data = measure(&mut session, &path);
    assert_eq!(
        data.get("width").and_then(|width| width.as_u64()),
        Some(2),
        "the size is still read: {data:?}"
    );
    assert_eq!(
        data.get("head_truncated").and_then(|cut| cut.as_bool()),
        Some(true),
        "a head that stopped at the bound says so: {data:?}"
    );
}

/// A file that is **not** a picture is refused, and a refusal of a head
/// that was cut **names the bound**: the caller is told where the read
/// stopped.
#[test]
fn a_refusal_of_a_cut_head_names_the_bound() {
    let path = a_file_that_is_not_a_picture("not-a-picture");
    let mut session = session();
    let error = session
        .execute("image.measure", &json!({ "path": path }))
        .expect_err("these bytes are not a picture");
    let message = error.to_string();
    assert!(message.contains(&MAX_IMAGE_HEAD_BYTES.to_string()), "the refusal names the bound: {message}");
}
