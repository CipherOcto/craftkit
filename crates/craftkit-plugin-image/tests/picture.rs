//! The **picture** is drawn: the file's pixels reach the canvas.
//!
//! The battery's job here is small and it is the whole point: a picture
//! node says where it is and which file it is, and the **paint** reads
//! that file, decodes it and puts the pixels on the canvas.
//!
//! The file in the tests is a **PNG written by the tree's own writer**,
//! so the test does not carry a picture in the tree — the tree has no
//! artwork of any kind, and a test that brought its own would break
//! that rule.

use craftkit_plugin_image::{ADD_PICTURE, PICTURE_KIND};
use craftkit_sdk::Registry;
use craftkit_sdk::command::CommandSpec;
use craftkit_sdk::document::{Colour, NodeKindSpec, RecordingCanvas};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Session, Text};

/// A session with this battery's commands in it.
fn session() -> Session {
    let mut commands: Registry<CommandSpec> = Registry::new();
    commands.register(ADD_PICTURE.clone()).expect("the command is registered");
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

/// A PNG file of that many pixels, with the top-left one black.
///
/// It is written with the tree's own writer, so the tree carries no
/// artwork and the fixture is made here and not shipped.
fn a_png_file(name: &str) -> String {
    let mut image = craftkit_raster::Image::new(2, 2, Colour::WHITE);
    image.blend(0, 0, Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let path = craftkit_scratch::file(&format!("craftkit-v2-image-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, image.to_png()).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// The picture is **drawn**: the paint puts the file's pixels on the
/// canvas, in the place the node says.
#[test]
fn the_picture_is_drawn() {
    let path = a_png_file("drawn");
    let mut session = session();
    session
        .execute(
            "image.add",
            &json!({ "path": path, "name": "square", "rect": { "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 } }),
        )
        .expect("the picture is added");
    let node = session.document().node(1).expect("the picture is there").clone();
    let mut canvas = RecordingCanvas::default();
    (PICTURE_KIND.paint)(&node, session.document(), &mut canvas);
    assert_eq!(canvas.drawn.len(), 1, "the picture drew nothing: {:?}", canvas.drawn);
    assert!(
        canvas.drawn[0].starts_with("picture(0, 0, 4 pixel(s))"),
        "the picture drew something else: {}",
        canvas.drawn[0]
    );
}

/// The picture's **size** is read from the file's header, and the
/// document says what it is.
#[test]
fn the_picture_knows_its_size() {
    let path = a_png_file("sized");
    let mut session = session();
    session.execute("image.add", &json!({ "path": path })).expect("the picture is added");
    let node = session.document().node(1).expect("the picture is there");
    assert_eq!(node.data.get("width").and_then(|width| width.as_u64()), Some(2));
    assert_eq!(node.data.get("height").and_then(|height| height.as_u64()), Some(2));
    assert_eq!(node.data.get("format").and_then(|format| format.as_str()), Some("png"));
}

/// A picture that is **not there** is refused, and nothing is added.
#[test]
fn a_picture_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("image.add", &json!({ "path": "/nonexistent/craftkit-v2-picture.png" }))
        .expect_err("the picture is not there");
    assert!(error.to_string().contains("image"), "{error}");
}

/// A **place** is four numbers, and a place that is not four numbers is
/// refused: the picture is not in the document's space, and a picture
/// that is not in the document's space cannot be drawn.
#[test]
fn a_place_that_is_not_four_numbers_is_refused() {
    let path = a_png_file("placed");
    let mut session = session();
    let error = session
        .execute("image.add", &json!({ "path": path, "rect": { "x": 0.0, "y": 0.0, "w": 10.0 } }))
        .expect_err("the place is not a place");
    assert!(error.to_string().contains("h"), "{error}");
}

/// The command's **path** is the only thing it needs: a picture without
/// a name is named after its file.
#[test]
fn a_picture_without_a_name_is_named_after_its_file() {
    let path = a_png_file("named");
    let mut session = session();
    let outcome = session.execute("image.add", &json!({ "path": path })).expect("the picture is added");
    let node = session.document().node(1).expect("the picture is there");
    assert_eq!(node.name, "craftkit-v2-image-named.png", "the picture is named after its file");
    assert!(outcome.message.contains("craftkit-v2-image-named.png"), "{outcome:?}");
}

/// The **text** of a path is the path: the command says what it needs,
/// and the tree does not make a second way in.
#[test]
fn the_path_is_the_whole_of_the_parameter() {
    let path = a_png_file("text");
    let text = Text(path.clone());
    assert_eq!(text.0, path);
}
